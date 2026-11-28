use super::*;
use crate::domain::operator_action::OperatorAction;
use crate::ui::action_catalog::CatalogAction;
use crate::ui::test_support::{navigation, pump};

fn binding(key: &str) -> ActionBinding {
    ActionBinding {
        shortcut: ActionShortcut {
            key: key.into(),
            modifiers: Modifiers::NONE,
        },
        action: OperatorAction::SaveReplay,
        enabled: true,
    }
}

#[test]
fn only_unique_enabled_nonconflicting_bindings_can_match() {
    let mut state = action_catalog::tests::connected();
    let original = binding("F8");
    state.registry.action_bindings.push(original.clone());
    assert_eq!(matching(&state, &original.shortcut), Some(original.clone()));
    state.registry.action_bindings.push(original.clone());
    assert!(matching(&state, &original.shortcut).is_none());
    assert_eq!(
        conflict(&state, &original.shortcut, Some(&original)),
        Some(ShortcutConflict::Duplicate)
    );
    state.registry.action_bindings.pop();
    state.registry.action_bindings[0].enabled = false;
    assert!(matching(&state, &original.shortcut).is_none());
    state.registry.action_bindings = vec![ActionBinding {
        shortcut: ActionShortcut {
            key: "1".into(),
            modifiers: Modifiers::new(true, false, false, false),
        },
        ..original
    }];
    assert!(matching(&state, &state.registry.action_bindings[0].shortcut).is_none());
}

fn find(widget: &gtk4::Widget, name: &str) -> gtk4::Widget {
    fn search(widget: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
        if widget.widget_name() == name {
            return Some(widget.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if let Some(found) = search(&widget, name) {
                return Some(found);
            }
        }
        None
    }
    search(widget, name).unwrap_or_else(|| panic!("missing {name}"))
}

fn dialog() -> adw::MessageDialog {
    gtk4::Window::list_toplevels()
        .into_iter()
        .find_map(|w| {
            w.downcast::<adw::MessageDialog>()
                .ok()
                .filter(|d| d.widget_name() == "binding-editor" && d.is_visible())
        })
        .unwrap()
}

fn key_controller(window: &impl IsA<gtk4::Widget>) -> gtk4::EventControllerKey {
    let controllers = window.observe_controllers();
    (0..controllers.n_items())
        .filter_map(|i| controllers.item(i))
        .find_map(|object| object.downcast().ok())
        .unwrap()
}

fn record(dialog: &adw::MessageDialog, key: gdk::Key, modifiers: gdk::ModifierType) {
    find(dialog.upcast_ref(), "binding-record")
        .downcast::<Button>()
        .unwrap()
        .emit_clicked();
    assert!(key_controller(dialog).emit_by_name::<bool>("key-pressed", &[&key, &0u32, &modifiers]));
}

#[test]
#[ignore = "requires a disposable GTK display"]
fn native_binding_editor_records_conflicts_persists_and_rejects_retired_scopes() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = navigation();
        *nav.state.borrow_mut() = action_catalog::tests::connected();
        let parent = adw::Window::new();
        parent.present();
        editor(parent.upcast_ref(), &nav, None);
        let first = dialog();
        record(&first, gdk::Key::k, gdk::ModifierType::CONTROL_MASK);
        assert!(!first.is_response_enabled("save"));
        record(&first, gdk::Key::F8, gdk::ModifierType::empty());
        assert!(first.is_response_enabled("save"));
        first.response("save");
        first.close();
        assert_eq!(nav.state.borrow().registry.action_bindings.len(), 1);
        let saved = nav.state.borrow().registry.action_bindings[0].clone();
        assert_eq!(saved.shortcut.key, "F8");
        assert!(saved.enabled);
        editor(parent.upcast_ref(), &nav, None);
        let second = dialog();
        record(&second, gdk::Key::F8, gdk::ModifierType::empty());
        assert!(!second.is_response_enabled("save"));
        second.response("save");
        second.close();
        assert_eq!(
            nav.state.borrow().registry.action_bindings.len(),
            1,
            "forced response must revalidate conflicts"
        );
        editor(parent.upcast_ref(), &nav, Some(saved.clone()));
        let third = dialog();
        record(&third, gdk::Key::F9, gdk::ModifierType::empty());
        assert!(third.is_response_enabled("save"));
        nav.state.borrow_mut().show_registry.generation += 1;
        third.response("save");
        third.close();
        assert_eq!(
            nav.state.borrow().registry.action_bindings.as_slice(),
            std::slice::from_ref(&saved)
        );
        let target = nav.state.borrow().show_registry.target.clone();
        assert!(crate::ui::test_support::pump_until(
            Duration::from_secs(2),
            || target
                .read()
                .is_ok_and(|r| r.action_bindings == [saved.clone()])
        ));
        let weak = third.downgrade();
        drop((first, second, third));
        pump(Duration::from_millis(280));
        assert!(weak.upgrade().is_none());
        parent.close();
    });
}

#[test]
#[ignore = "requires a disposable GTK display"]
fn native_binding_manager_edit_remove_and_large_text_layout() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = navigation();
        *nav.state.borrow_mut() = action_catalog::tests::connected();
        nav.state
            .borrow_mut()
            .registry
            .action_bindings
            .push(binding("F8"));
        gtk4::Settings::default()
            .unwrap()
            .set_gtk_font_name(Some("Sans 22"));
        let parent = adw::Window::new();
        parent.present();
        show(parent.upcast_ref(), &nav);
        let manager = gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|w| {
                w.downcast::<adw::Window>()
                    .ok()
                    .filter(|w| w.widget_name() == "binding-manager")
            })
            .unwrap();
        manager.set_default_size(360, 480);
        pump(Duration::from_millis(100));
        assert!(
            manager.width() <= 360,
            "manager too wide: {}",
            manager.width()
        );
        find(manager.upcast_ref(), "binding-edit-0")
            .downcast::<Button>()
            .unwrap()
            .emit_clicked();
        let edit = dialog();
        record(&edit, gdk::Key::F9, gdk::ModifierType::empty());
        edit.response("save");
        edit.close();
        pump(Duration::from_millis(280));
        assert_eq!(
            nav.state.borrow().registry.action_bindings[0].shortcut.key,
            "F9"
        );
        let stale_remove = find(manager.upcast_ref(), "binding-remove-0")
            .downcast::<Button>()
            .unwrap();
        stale_remove.emit_clicked();
        pump(Duration::from_millis(280));
        assert!(nav.state.borrow().registry.action_bindings.is_empty());
        nav.state
            .borrow_mut()
            .registry
            .action_bindings
            .push(binding("F10"));
        stale_remove.emit_clicked();
        assert_eq!(
            nav.state.borrow().registry.action_bindings[0].shortcut.key,
            "F10"
        );
        let weak = manager.downgrade();
        manager.close();
        drop((manager, edit, stale_remove));
        pump(Duration::from_millis(280));
        assert!(weak.upgrade().is_none());
        parent.close();
    });
}

#[test]
#[ignore = "requires a disposable GTK display"]
fn native_custom_keys_work_across_pages_suppress_typing_and_repeats() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = navigation();
        *nav.state.borrow_mut() = action_catalog::tests::connected();
        let scene = action_catalog::entries(&nav.state.borrow())
            .into_iter()
            .find_map(|entry| match entry.action {
                CatalogAction::Operator(action @ OperatorAction::Scene { .. }) => Some(action),
                _ => None,
            })
            .unwrap();
        nav.state
            .borrow_mut()
            .registry
            .action_bindings
            .push(ActionBinding {
                action: scene,
                ..binding("F8")
            });
        nav.state.borrow_mut().current_page = crate::controller::state::Page::Settings;
        nav.state.borrow_mut().registry.scene_filter.query = "hide everything".into();
        let window = adw::ApplicationWindow::builder().build();
        let root = GtkBox::new(Orientation::Vertical, 8);
        let button = Button::with_label("Focus");
        let entry = gtk4::Entry::new();
        root.append(&button);
        root.append(&entry);
        window.set_content(Some(&root));
        let live = Rc::new(crate::ui::pages::live::build(nav.clone()));
        let toast = adw::ToastOverlay::new();
        crate::ui::hotkeys::install(&window, &nav, live.clone(), &toast);
        window.present();
        pump(Duration::from_millis(70));
        button.grab_focus();
        let keys = key_controller(&window);
        let press = || {
            keys.emit_by_name::<bool>(
                "key-pressed",
                &[&gdk::Key::F8, &0u32, &gdk::ModifierType::empty()],
            )
        };
        let release = || {
            keys.emit_by_name::<()>(
                "key-released",
                &[&gdk::Key::F8, &0u32, &gdk::ModifierType::empty()],
            )
        };
        assert!(press());
        let sequence = nav.state.borrow().workspace.sequence;
        assert!(nav.state.borrow().workspace.active_action.is_some());
        assert!(press());
        assert_eq!(
            nav.state.borrow().workspace.sequence,
            sequence,
            "repeat cannot dispatch again"
        );
        release();
        entry.grab_focus();
        assert!(!press());
        assert_eq!(nav.state.borrow().workspace.sequence, sequence);
        button.grab_focus();
        release();
        nav.state.borrow_mut().registry.action_bindings[0].enabled = false;
        assert!(!press());
        let lower = chord(
            &window,
            gdk::Key::M,
            0,
            gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
        )
        .unwrap();
        assert_eq!(lower.key, "m");
        assert!(lower.modifiers.shift);
        assert!(chord(&window, gdk::Key::m, 0, gdk::ModifierType::META_MASK).is_none());
        window.close();
    });
}
