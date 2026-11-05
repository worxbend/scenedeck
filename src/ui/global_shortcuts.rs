//! Explicit, window/session-scoped desktop shortcuts. No background registration.

mod portal;

use super::{action_catalog, action_shortcuts, navigation::NavigationContext};
use crate::{
    controller::state::AppState, domain::action_shortcut::ActionBinding,
    infra::i18n::LANGUAGE_LOADER,
};
use adw::prelude::*;
use futures_util::future::{AbortHandle, Abortable};
use i18n_embed_fl::fl;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

#[derive(Default)]
pub(super) struct Manager {
    generation: u64,
    abort: Option<AbortHandle>,
    active: Vec<ActionBinding>,
    details: Vec<String>,
    status: String,
}

impl Manager {
    pub(super) fn stop(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
        self.active.clear();
        self.details.clear();
        self.status = fl!(LANGUAGE_LOADER, "global-off");
    }

    pub(super) fn owns(&self, binding: &ActionBinding) -> bool {
        self.active.contains(binding)
    }
}

#[derive(Clone)]
struct Spec {
    id: String,
    label: String,
    binding: ActionBinding,
}

fn eligible(state: &AppState) -> Vec<ActionBinding> {
    state
        .registry
        .action_bindings
        .iter()
        .filter(|b| action_shortcuts::matching(state, &b.shortcut).as_ref() == Some(*b))
        .cloned()
        .collect()
}

fn specs(state: &AppState) -> Vec<Spec> {
    eligible(state)
        .into_iter()
        .map(|binding| {
            // IDs depend on workspace + intention, never display order or key choice.
            // Drop a known UUID's renameable display name from its stable identity.
            let mut identity = serde_json::to_value(&binding.action).expect("serializable action");
            if let Some(target) = identity.get_mut("target") {
                if target.get("uuid").is_some_and(|v| v.is_string()) {
                    target["name"] = "".into();
                }
            }
            let key = serde_json::to_vec(&(
                state.connected_connection_id.as_deref(),
                state.workflow_scope(),
                identity,
            ))
            .expect("serializable identity");
            let id = glib::compute_checksum_for_data(glib::ChecksumType::Sha256, &key)
                .expect("SHA256 supported")
                .to_string();
            Spec {
                id,
                label: action_catalog::binding_entry(state, &binding.action).label,
                binding,
            }
        })
        .collect()
}

struct Context {
    nav: NavigationContext,
    parent: glib::WeakRef<gtk4::Window>,
    bindings: Vec<ActionBinding>,
    generation: u64,
}

impl Context {
    fn can_activate(&self) -> bool {
        let Some(parent) = self.parent.upgrade() else {
            return false;
        };
        let editing = parent.is_active()
            && gtk4::prelude::RootExt::focus(&parent).is_some_and(|widget| {
                widget.is::<gtk4::Editable>() || widget.is::<gtk4::TextView>()
            });
        let modal = gtk4::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk4::Window>().ok())
            .any(|w| {
                if !w.is_visible() || !w.is_modal() {
                    return false;
                }
                let mut ancestor = w.transient_for();
                while let Some(window) = ancestor {
                    if window == parent {
                        return true;
                    }
                    ancestor = window.transient_for();
                }
                false
            });
        !editing && !modal
    }

    fn current(&self) -> bool {
        self.nav.global_shortcuts.borrow().generation == self.generation
            && self.parent.upgrade().is_some_and(|w| w.is_visible())
            && self.nav.is_current()
            && self.nav.registry_is_current()
            && eligible(&self.nav.state.borrow()) == self.bindings
    }
}

fn update_bound(nav: &NavigationContext, specs: &[Spec], shortcuts: portal::Shortcuts) {
    let bound: HashMap<_, _> = shortcuts.into_iter().collect();
    let mut manager = nav.global_shortcuts.borrow_mut();
    manager.active = specs
        .iter()
        .filter(|spec| bound.contains_key(&spec.id))
        .map(|spec| spec.binding.clone())
        .collect();
    let mut seen = HashSet::new();
    manager.details = specs
        .iter()
        .filter(|spec| seen.insert(&spec.id))
        .filter_map(|spec| {
            let props = bound.get(&spec.id)?;
            let trigger = props
                .get("trigger_description")
                .and_then(|v| v.str())
                .unwrap_or("—");
            Some(format!("{} · {}", spec.label, trigger))
        })
        .collect();
    manager.status = if manager.active.is_empty() {
        fl!(LANGUAGE_LOADER, "global-none")
    } else {
        fl!(LANGUAGE_LOADER, "global-active")
    };
}

fn start(nav: &NavigationContext) {
    let nav = nav.scoped().registry_scoped();
    let Some(parent) = nav.parent_window() else {
        return;
    };
    let entries = specs(&nav.state.borrow());
    if !nav.is_current()
        || !nav.registry_is_current()
        || entries.is_empty()
        || entries.len() > 64
        || nav.global_shortcuts.borrow().abort.is_some()
    {
        return;
    }
    let (abort, registration) = AbortHandle::new_pair();
    let generation = {
        let mut manager = nav.global_shortcuts.borrow_mut();
        manager.stop();
        manager.abort = Some(abort);
        manager.status = fl!(LANGUAGE_LOADER, "global-pending");
        manager.generation
    };
    let context = Rc::new(Context {
        bindings: eligible(&nav.state.borrow()),
        nav: nav.clone(),
        parent: parent.downgrade(),
        generation,
    });
    glib::MainContext::default().spawn_local(async move {
        let operation = async {
            let portal =
                glib::future_with_timeout(Duration::from_secs(10), portal::Portal::connect())
                    .await
                    .map_err(|_| "Portal connection timed out".to_string())??;
            let session = portal.create().await?;
            let closed = Rc::new(Cell::new(false));
            let ready = Rc::new(Cell::new(false));
            let held = RefCell::new(HashSet::new());
            let changed = Rc::new(RefCell::new(None));
            let _signals = session.listen({
                let context = context.clone();
                let entries = entries.clone();
                let closed = closed.clone();
                let ready = ready.clone();
                let changed = changed.clone();
                move |event| {
                    if !context.current() || closed.get() {
                        return;
                    }
                    match event {
                        portal::Event::Closed => {
                            closed.set(true);
                            context.nav.global_shortcuts.borrow_mut().active.clear();
                        }
                        portal::Event::Changed(shortcuts) => {
                            if !ready.get() {
                                *changed.borrow_mut() = Some(shortcuts);
                                return;
                            }
                            update_bound(&context.nav, &entries, shortcuts);
                        }
                        portal::Event::Key { id, pressed } => {
                            if !pressed {
                                held.borrow_mut().remove(&id);
                                return;
                            }
                            if !ready.get()
                                || !entries.iter().any(|s| s.id == id)
                                || !held.borrow_mut().insert(id.clone())
                                || !context.can_activate()
                            {
                                return;
                            }
                            let binding = entries.iter().find(|s| s.id == id).map(|s| &s.binding);
                            if let Some(binding) =
                                binding.filter(|b| context.nav.global_shortcuts.borrow().owns(b))
                            {
                                if !action_catalog::execute(
                                    &context.nav,
                                    &action_catalog::CatalogAction::Operator(
                                        binding.action.clone(),
                                    ),
                                ) {
                                    context.nav.global_shortcuts.borrow_mut().status =
                                        fl!(LANGUAGE_LOADER, "binding-unavailable");
                                    super::workspace::refresh(&context.nav);
                                }
                            }
                        }
                    }
                }
            });
            let mut seen = HashSet::new();
            let shortcuts = entries
                .iter()
                .filter(|s| seen.insert(&s.id))
                .map(|s| {
                    (
                        s.id.clone(),
                        HashMap::from([("description".into(), s.label.to_variant())]),
                    )
                })
                .collect();
            let bound = session.bind(shortcuts).await?;
            if !context.current() || closed.get() {
                return Ok::<(), String>(());
            }
            update_bound(&nav, &entries, changed.borrow_mut().take().unwrap_or(bound));
            ready.set(true);
            while context.current() && session.alive() && !closed.get() {
                glib::timeout_future(Duration::from_millis(250)).await;
            }
            Ok(())
        };
        // Scope/watchdog races even an outstanding permission dialog, so switching
        // collections or closing the main window promptly drops request guards.
        let watchdog = async {
            loop {
                glib::timeout_future(Duration::from_millis(100)).await;
                if !context.current() {
                    return;
                }
            }
        };
        let result = Abortable::new(
            async {
                futures_util::pin_mut!(operation, watchdog);
                match futures_util::future::select(operation, watchdog).await {
                    futures_util::future::Either::Left((result, _)) => result,
                    futures_util::future::Either::Right(_) => Ok(()),
                }
            },
            registration,
        )
        .await;
        let mut manager = nav.global_shortcuts.borrow_mut();
        if manager.generation == generation {
            manager.stop();
            if let Ok(Err(error)) = result {
                manager.status = format!("{}\n{error}", fl!(LANGUAGE_LOADER, "global-unavailable"));
            }
        }
    });
}

fn button(text: &str) -> gtk4::Button {
    let label = gtk4::Label::builder()
        .label(text)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .build();
    let button = gtk4::Button::builder().child(&label).build();
    button.update_property(&[gtk4::accessible::Property::Label(text)]);
    button
}

pub(super) fn controls(nav: &NavigationContext) -> gtk4::Box {
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    let title = gtk4::Label::builder()
        .label(fl!(LANGUAGE_LOADER, "global-title"))
        .wrap(true)
        .width_chars(1)
        .xalign(0.0)
        .build();
    title.add_css_class("heading");
    root.append(&title);
    let description = gtk4::Label::builder()
        .label(fl!(LANGUAGE_LOADER, "global-description"))
        .wrap(true)
        .width_chars(1)
        .xalign(0.0)
        .build();
    root.append(&description);
    let start_button = button(&fl!(LANGUAGE_LOADER, "global-enable"));
    start_button.set_widget_name("global-enable");
    start_button.connect_clicked({
        let nav = nav.clone();
        move |_| start(&nav)
    });
    root.append(&start_button);
    let stop = button(&fl!(LANGUAGE_LOADER, "global-stop"));
    stop.set_widget_name("global-stop");
    stop.connect_clicked({
        let manager = nav.global_shortcuts.clone();
        move |_| manager.borrow_mut().stop()
    });
    root.append(&stop);
    let status = gtk4::Label::builder()
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .xalign(0.0)
        .build();
    status.set_widget_name("global-status");
    root.append(&status);
    let refresh = Rc::new({
        let nav = nav.clone();
        let root = root.downgrade();
        let start_button = start_button.downgrade();
        let stop = stop.downgrade();
        let status = status.downgrade();
        move || {
            let (Some(root), Some(start_button), Some(stop), Some(status)) = (
                root.upgrade(),
                start_button.upgrade(),
                stop.upgrade(),
                status.upgrade(),
            ) else {
                return false;
            };
            let count = eligible(&nav.state.borrow()).len();
            let manager = nav.global_shortcuts.borrow();
            start_button.set_sensitive(
                manager.abort.is_none()
                    && nav.scoped().is_current()
                    && nav.registry_is_current()
                    && (1..=64).contains(&count),
            );
            stop.set_sensitive(manager.abort.is_some());
            let message = if manager.status.is_empty() {
                fl!(LANGUAGE_LOADER, "global-off")
            } else {
                manager.status.clone()
            };
            status.set_label(
                &std::iter::once(message)
                    .chain(manager.details.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            root.is_visible()
        }
    });
    refresh();
    glib::timeout_add_local(Duration::from_millis(250), move || {
        if refresh() {
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        action_shortcut::ActionShortcut,
        hotkey::Modifiers,
        operator_action::{ActionTarget, OperatorAction},
    };

    #[test]
    fn desktop_ids_follow_uuid_renames_but_never_cross_connections_or_collections() {
        let mut state = action_catalog::tests::connected();
        let binding = ActionBinding {
            action: OperatorAction::OpenSource {
                target: ActionTarget {
                    name: "Microphone".into(),
                    uuid: Some("input-1".into()),
                },
            },
            enabled: true,
            shortcut: ActionShortcut {
                key: "F8".into(),
                modifiers: Modifiers::NONE,
            },
        };
        state.registry.action_bindings.push(binding.clone());
        let original = specs(&state)[0].id.clone();
        state.registry.action_bindings[0].shortcut.key = "F9".into();
        let OperatorAction::OpenSource { target } = &mut state.registry.action_bindings[0].action
        else {
            unreachable!()
        };
        target.name = "Renamed microphone".into();
        assert_eq!(specs(&state)[0].id, original);
        state.connected_connection_id = Some("Other computer".into());
        assert_ne!(specs(&state)[0].id, original);
        state.connected_connection_id = None;
        state.scene_collections.current = Some("Other show".into());
        assert_ne!(specs(&state)[0].id, original);
        state.registry.action_bindings[0].enabled = false;
        assert!(specs(&state).is_empty());
        state.registry.action_bindings = vec![binding.clone(), binding];
        assert!(
            specs(&state).is_empty(),
            "conflicted keys are never registered"
        );
    }
}
