//! Isolated fake portal: no real desktop registration or production bus.
use super::*;
use crate::ui::test_support::{native_test, navigation, pump, pump_until};
use adw::prelude::*;
use std::{cell::Cell, rc::Rc};

const XML: &str = r#"<node><interface name='org.freedesktop.portal.GlobalShortcuts'>
<property name='version' type='u' access='read'/>
<method name='CreateSession'><arg type='a{sv}' direction='in'/><arg type='o' direction='out'/></method>
<method name='BindShortcuts'><arg type='o' direction='in'/><arg type='a(sa{sv})' direction='in'/><arg type='s' direction='in'/><arg type='a{sv}' direction='in'/><arg type='o' direction='out'/></method>
<signal name='Activated'><arg type='o'/><arg type='s'/><arg type='t'/><arg type='a{sv}'/></signal>
<signal name='Deactivated'><arg type='o'/><arg type='s'/><arg type='t'/><arg type='a{sv}'/></signal>
<signal name='ShortcutsChanged'><arg type='o'/><arg type='a(sa{sv})'/></signal>
</interface></node>"#;

#[derive(Default)]
struct State {
    mode: Cell<u32>, // 0 accepted, 1 cancelled, 2 held dialog, 3 empty subset.
    version: Cell<u32>,
    registry_mode: Cell<u32>,
    registered: RefCell<std::collections::HashSet<String>>,
    sessions: RefCell<Vec<String>>,
    bindings: RefCell<Shortcuts>,
    closed: RefCell<Vec<String>>,
    registrations: RefCell<Vec<gio::RegistrationId>>,
}
struct Fixture {
    connection: gio::DBusConnection,
    state: Rc<State>,
}

impl Fixture {
    async fn new() -> Self {
        let connection = gio::DBusConnection::for_address_future(
            &std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap(),
            gio::DBusConnectionFlags::AUTHENTICATION_CLIENT
                | gio::DBusConnectionFlags::MESSAGE_BUS_CONNECTION,
            None,
        )
        .await
        .unwrap();
        let state = Rc::new(State::default());
        state.version.set(1);
        let registry_xml = "<node><interface name='org.freedesktop.host.portal.Registry'><method name='Register'><arg type='s' direction='in'/><arg type='a{sv}' direction='in'/></method></interface></node>";
        let registry_info = gio::DBusNodeInfo::for_xml(registry_xml)
            .unwrap()
            .lookup_interface("org.freedesktop.host.portal.Registry")
            .unwrap();
        let registration = connection
            .register_object(PATH, &registry_info)
            .method_call({
                let state = state.clone();
                move |_, sender, _, _, _, parameters, invocation| {
                    assert_eq!(parameters.child_get::<String>(0), crate::app_info::APP_ID);
                    assert!(parameters.child_get::<Dict>(1).is_empty());
                    match state.registry_mode.get() {
                        1 => invocation.return_dbus_error(
                            "org.freedesktop.DBus.Error.UnknownMethod",
                            "Old portal without host registry",
                        ),
                        2 => invocation.return_dbus_error(
                            "org.freedesktop.portal.Error.NotAllowed",
                            "Sandbox identity already provided",
                        ),
                        _ => {
                            assert!(
                                state.registered.borrow_mut().insert(sender.unwrap().into()),
                                "dedicated peer registers at most once"
                            );
                            invocation.return_value(None);
                        }
                    }
                }
            })
            .build()
            .unwrap();
        state.registrations.borrow_mut().push(registration);
        let info = gio::DBusNodeInfo::for_xml(XML)
            .unwrap()
            .lookup_interface(INTERFACE)
            .unwrap();
        let registration = connection
            .register_object(PATH, &info)
            .property({
                let state = state.clone();
                move |_, sender, _, _, _| {
                    assert!(
                        state.registry_mode.get() != 0
                            || state.registered.borrow().contains(sender.unwrap()),
                        "Register must precede even the proxy property read"
                    );
                    state.version.get().to_variant()
                }
            })
            .method_call({
                let state = state.clone();
                move |connection, sender, _, _, method, parameters, invocation| {
                    let sender = sender.unwrap();
                    assert!(
                        state.registry_mode.get() != 0
                            || state.registered.borrow().contains(sender),
                        "host identity must precede CreateSession"
                    );
                    let prefix = sender.trim_start_matches(':').replace('.', "_");
                    let options = if method == "CreateSession" {
                        parameters.child_get::<Dict>(0)
                    } else {
                        parameters.child_get::<Dict>(3)
                    };
                    let token = options["handle_token"].str().unwrap();
                    let request = format!("{PATH}/request/{prefix}/{token}");
                    register_close(
                        &connection,
                        &state,
                        &request,
                        "org.freedesktop.portal.Request",
                    );
                    let result: Dict = if method == "CreateSession" {
                        let token = options["session_handle_token"].str().unwrap();
                        let session = format!("{PATH}/session/{prefix}/{token}");
                        register_close(
                            &connection,
                            &state,
                            &session,
                            "org.freedesktop.portal.Session",
                        );
                        state.sessions.borrow_mut().push(session.clone());
                        HashMap::from([("session_handle".into(), session.to_variant())])
                    } else {
                        assert_eq!(parameters.child_get::<String>(2), "");
                        let mut shortcuts = parameters.child_get::<Shortcuts>(1);
                        // Return a subset and a desktop-selected key, never the hint.
                        shortcuts.truncate(1);
                        for (_, props) in &mut shortcuts {
                            props.insert("trigger_description".into(), "Ctrl+Alt+F8".to_variant());
                        }
                        if state.mode.get() == 3 {
                            shortcuts.clear();
                        }
                        *state.bindings.borrow_mut() = shortcuts.clone();
                        HashMap::from([("shortcuts".into(), shortcuts.to_variant())])
                    };
                    if method == "CreateSession" || state.mode.get() != 2 {
                        let code = if method == "BindShortcuts" && state.mode.get() == 1 {
                            1u32
                        } else {
                            0
                        };
                        // Deliberately emit BEFORE the method returns to prove the
                        // pre-call subscription doesn't lose a fast Response.
                        connection
                            .emit_signal(
                                Some(sender),
                                &request,
                                "org.freedesktop.portal.Request",
                                "Response",
                                Some(&(code, result).to_variant()),
                            )
                            .unwrap();
                    }
                    invocation.return_value(Some(
                        &(ObjectPath::try_from(request).unwrap(),).to_variant(),
                    ));
                }
            })
            .build()
            .unwrap();
        state.registrations.borrow_mut().push(registration);
        connection
            .call_future(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "RequestName",
                Some(&(BUS, 0u32).to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                3000,
            )
            .await
            .unwrap();
        Self { connection, state }
    }
    fn session(&self) -> ObjectPath {
        ObjectPath::try_from(self.state.sessions.borrow().last().unwrap().clone()).unwrap()
    }
    fn key(&self, session: ObjectPath, id: &str, pressed: bool) {
        self.connection
            .emit_signal(
                None,
                PATH,
                INTERFACE,
                if pressed { "Activated" } else { "Deactivated" },
                Some(&(session, id, 1u64, Dict::new()).to_variant()),
            )
            .unwrap();
    }
    fn changed(&self, shortcuts: Shortcuts) {
        self.connection
            .emit_signal(
                None,
                PATH,
                INTERFACE,
                "ShortcutsChanged",
                Some(&(self.session(), shortcuts).to_variant()),
            )
            .unwrap();
    }
}
fn register_close(
    connection: &gio::DBusConnection,
    state: &Rc<State>,
    path: &str,
    interface: &str,
) {
    let xml =
        format!("<node><interface name='{interface}'><method name='Close'/></interface></node>");
    let info = gio::DBusNodeInfo::for_xml(&xml)
        .unwrap()
        .lookup_interface(interface)
        .unwrap();
    let registration = connection
        .register_object(path, &info)
        .method_call({
            let state = state.clone();
            move |_, _, path, _, _, _, invocation| {
                state.closed.borrow_mut().push(path.into());
                invocation.return_value(None);
            }
        })
        .build()
        .unwrap();
    state.registrations.borrow_mut().push(registration);
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for id in self.state.registrations.borrow_mut().drain(..) {
            self.connection.unregister_object(id).unwrap();
        }
        self.connection.close(gio::Cancellable::NONE, |_| {});
    }
}

#[test]
#[ignore = "requires isolated GTK and disposable session bus"]
fn native_portal_fast_response_subset_cancellation_and_capability() {
    native_test("ui::global_shortcuts::portal::tests::native_portal_fast_response_subset_cancellation_and_capability", || {
        let context = glib::MainContext::default();
        let fixture = context.block_on(Fixture::new());
        let session = context.block_on(async { Portal::connect().await.unwrap().create().await.unwrap() });
        let spec = |id: &str| (id.into(), HashMap::from([("description".into(), "Action".to_variant())]));
        let bound = context.block_on(session.bind(vec![spec("one"), spec("two")])).unwrap();
        assert_eq!(bound.len(), 1); assert_eq!(bound[0].0, "one");
        assert_eq!(bound[0].1["trigger_description"].str(), Some("Ctrl+Alt+F8"));
        let received = Rc::new(Cell::new(0));
        let subscriptions = session.listen({ let received = received.clone(); move |event| { if matches!(event, Event::Key { .. }) { received.set(received.get()+1); } } });
        fixture.key(ObjectPath::try_from("/wrong").unwrap(), "one", true);
        pump(Duration::from_millis(40)); assert_eq!(received.get(), 0);
        fixture.key(fixture.session(), "one", true);
        assert!(pump_until(Duration::from_secs(2), || received.get() == 1));
        drop(subscriptions); drop(session);
        assert!(pump_until(Duration::from_secs(2), || fixture.state.closed.borrow().contains(&fixture.session().to_string())));
        fixture.state.mode.set(1);
        let session = context.block_on(async { Portal::connect().await.unwrap().create().await.unwrap() });
        assert!(context.block_on(session.bind(vec![spec("cancel")])).unwrap_err().contains("cancelled"));
        drop(session);
        fixture.state.version.set(0);
        assert!(context.block_on(Portal::connect()).is_err());
        assert_eq!(fixture.state.sessions.borrow().len(), 2, "unsupported portal never creates a session");
        fixture.state.version.set(1); fixture.state.mode.set(0);
        for mode in [1, 2] {
            fixture.state.registry_mode.set(mode);
            let session = context.block_on(async { Portal::connect().await.unwrap().create().await.unwrap() });
            assert_eq!(context.block_on(session.bind(vec![spec("compatible")])).unwrap().len(), 1);
            drop(session);
        }

    });
}

#[test]
#[ignore = "requires isolated GTK and disposable session bus"]
fn native_global_shortcuts_dispatch_once_revoke_and_retire_with_workspace() {
    native_test("ui::global_shortcuts::portal::tests::native_global_shortcuts_dispatch_once_revoke_and_retire_with_workspace", || {
        use crate::{domain::{action_shortcut::{ActionBinding, ActionShortcut}, hotkey::Modifiers, operator_action::{ActionTarget, OperatorAction}}, controller::state::Page};
        use super::super::{start, controls};
        let fixture = glib::MainContext::default().block_on(Fixture::new());
        let (nav, stack, _runtime, _events) = navigation();
        *nav.state.borrow_mut() = crate::ui::action_catalog::tests::connected();
        let action = OperatorAction::OpenSource { target: ActionTarget { name: "Microphone".into(), uuid: Some("input-1".into()) } };
        nav.state.borrow_mut().registry.action_bindings = vec![ActionBinding { action, enabled: true, shortcut: ActionShortcut { key: "F8".into(), modifiers: Modifiers::NONE } }];
        let parent = adw::ApplicationWindow::builder().build();
        let body = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        body.append(&stack); body.append(&controls(&nav)); parent.set_content(Some(&body)); parent.present();
        let live = Rc::new(crate::ui::pages::live::build(nav.clone()));
        crate::ui::hotkeys::install(&parent, &nav, live.clone(), &adw::ToastOverlay::new());
        start(&nav);
        assert!(pump_until(Duration::from_secs(3), || nav.global_shortcuts.borrow().active.len() == 1));
        assert!(nav.global_shortcuts.borrow().details[0].contains("Ctrl+Alt+F8"));
        let session = fixture.session(); let id = fixture.state.bindings.borrow()[0].0.clone();
        let controllers = parent.observe_controllers();
        let keys = (0..controllers.n_items()).filter_map(|i| controllers.item(i)).find_map(|o| o.downcast::<gtk4::EventControllerKey>().ok()).unwrap();
        nav.state.borrow_mut().current_page = Page::Live;
        assert!(keys.emit_by_name::<bool>("key-pressed", &[&gtk4::gdk::Key::F8, &0u32, &gtk4::gdk::ModifierType::empty()]));
        assert_eq!(nav.state.borrow().current_page, Page::Live, "portal-owned action cannot also fire locally");

        nav.state.borrow_mut().current_page = Page::Live;
        fixture.key(session.clone(), &id, true);
        assert!(pump_until(Duration::from_secs(2), || nav.state.borrow().current_page == Page::Sources));
        nav.state.borrow_mut().current_page = Page::Live;
        fixture.key(session.clone(), &id, true); pump(Duration::from_millis(60));
        assert_eq!(nav.state.borrow().current_page, Page::Live, "held key never repeats");
        fixture.key(session.clone(), &id, false);
        let entry = gtk4::Entry::new(); body.append(&entry); parent.present(); entry.grab_focus();
        assert!(pump_until(Duration::from_secs(2), || parent.is_active()));
        fixture.key(session.clone(), &id, true); pump(Duration::from_millis(60));
        assert_eq!(nav.state.borrow().current_page, Page::Live, "typing in the active app suppresses global actions");
        let other = adw::Window::new(); other.set_content(Some(&gtk4::Button::with_label("Other window"))); other.present();
        assert!(pump_until(Duration::from_secs(2), || !parent.is_active() && other.is_active()));
        fixture.key(session.clone(), &id, false); fixture.key(session.clone(), &id, true);
        assert!(pump_until(Duration::from_secs(2), || nav.state.borrow().current_page == Page::Sources), "a remembered text field in the inactive app must not suppress global actions");
        other.close(); entry.set_sensitive(false); parent.present(); nav.state.borrow_mut().current_page = Page::Live;
        fixture.key(session.clone(), &id, false);
        // A modal editor prevents action dispatch even from the desktop.
        let modal = adw::Window::builder().transient_for(&parent).modal(true).build(); modal.present();
        fixture.key(session.clone(), &id, true); pump(Duration::from_millis(60));
        assert_eq!(nav.state.borrow().current_page, Page::Live); modal.close();
        fixture.key(session.clone(), &id, false);
        fixture.changed(vec![]);
        assert!(pump_until(Duration::from_secs(2), || nav.global_shortcuts.borrow().active.is_empty()));
        fixture.key(session.clone(), &id, true); pump(Duration::from_millis(60));
        assert_eq!(nav.state.borrow().current_page, Page::Live, "revoked key cannot dispatch");
        fixture.changed(fixture.state.bindings.borrow().clone());
        assert!(pump_until(Duration::from_secs(2), || nav.global_shortcuts.borrow().active.len() == 1));
        nav.state.borrow_mut().workspace.epoch += 1;
        fixture.key(session.clone(), &id, false); fixture.key(session.clone(), &id, true);
        assert!(pump_until(Duration::from_secs(2), || nav.global_shortcuts.borrow().abort.is_none()));
        assert_eq!(nav.state.borrow().current_page, Page::Live, "retired scope cannot dispatch");
        assert!(pump_until(Duration::from_secs(2), || fixture.state.closed.borrow().contains(&session.to_string())));
        // Cancel while the permission dialog is pending, then retry explicitly.
        fixture.state.mode.set(2); start(&nav);
        assert!(pump_until(Duration::from_secs(2), || fixture.state.sessions.borrow().len() == 2));
        pump(Duration::from_millis(60)); nav.global_shortcuts.borrow_mut().stop();
        assert!(pump_until(Duration::from_secs(2), || fixture.state.closed.borrow().iter().any(|p| p.contains("/request/"))));
        assert!(nav.global_shortcuts.borrow().active.is_empty());
        fixture.state.mode.set(3); start(&nav);
        assert!(pump_until(Duration::from_secs(2), || fixture.state.sessions.borrow().len() == 3 && nav.global_shortcuts.borrow().status == crate::infra::i18n::LANGUAGE_LOADER.get("global-none")));
        fixture.connection.emit_signal(None, fixture.session().as_str(), "org.freedesktop.portal.Session", "Closed", Some(&(Dict::new(),).to_variant())).unwrap();
        assert!(pump_until(Duration::from_secs(2), || nav.global_shortcuts.borrow().abort.is_none()));
        fixture.state.mode.set(0); start(&nav);
        assert!(pump_until(Duration::from_secs(2), || nav.global_shortcuts.borrow().active.len() == 1));
        nav.dispatch(crate::controller::command::AppCommand::SetCurrentSceneCollection("Next".into()));
        assert!(nav.global_shortcuts.borrow().active.is_empty(), "switch intent stops keys before OBS responds");
        assert!(pump_until(Duration::from_secs(2), || fixture.state.closed.borrow().contains(&fixture.session().to_string())));
        start(&nav);
        assert!(pump_until(Duration::from_secs(2), || nav.global_shortcuts.borrow().active.len() == 1));
        fixture.connection.close(gio::Cancellable::NONE, |_| {});
        assert!(pump_until(Duration::from_secs(2), || nav.global_shortcuts.borrow().abort.is_none()));
        parent.close();
    });
}
