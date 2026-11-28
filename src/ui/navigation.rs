//! Shared handle passed to every page builder.
//!
//! Pages use `nav.switch_to_page()` for navigation and `nav.dispatch()` to
//! send commands to the AppController.  The stack and controller are private.
//! `read` and `update` are the later borrow boundary. Existing `state.borrow`
//! call sites stay until a follow-up moves them.
//!
//! A closure passed to `read` or `update` must not call `dispatch`, persist,
//! or refresh another page. Those paths borrow this same `RefCell` again.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::ObjectExt;
use gtk4::{ListBox, Stack};

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::state::{AppState, ObsStatus, Page};
use crate::domain::workflow::ScopeBinding;

#[derive(Clone)]
pub(crate) struct NavigationContext {
    pub(crate) state: Rc<RefCell<AppState>>,
    pub(super) global_shortcuts: Rc<RefCell<super::global_shortcuts::Manager>>,
    content_stack: glib::WeakRef<Stack>,
    controller: Rc<RefCell<AppController>>,
    /// Sidebar list and the page order it was built from, once the window has
    /// built them.  Held so that navigation triggered from anywhere else — a
    /// Help topic button, a keyboard action, the first-run dialog — moves the
    /// sidebar highlight too, instead of leaving it pointing at the page you
    /// just left.
    sidebar: Rc<RefCell<Option<Sidebar>>>,
    binding: Option<NavigationBinding>,
    registry_binding: Option<(crate::storage::show_workspaces::RegistryTarget, u64)>,
}

#[derive(Clone)]
struct NavigationBinding {
    epoch: u64,
    scope: ScopeBinding,
}

impl NavigationBinding {
    fn accepts(&self, epoch: u64, scope: &ScopeBinding, connected: bool) -> bool {
        connected && self.epoch == epoch && &self.scope == scope
    }
}

/// `update` found `AppState` already borrowed, so the write was not applied.
///
/// shortcut: no production caller until pages leave `state.borrow`; delete the allow then.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReentrantUpdate;

/// The sidebar list plus the page each of its rows stands for, in row order.
struct Sidebar {
    list: glib::WeakRef<ListBox>,
    pages: &'static [Page],
}

impl NavigationContext {
    pub(super) fn parent_window(&self) -> Option<gtk4::Window> {
        use gtk4::prelude::*;
        self.content_stack.upgrade()?.root()?.downcast().ok()
    }

    pub(crate) fn new(
        state: Rc<RefCell<AppState>>,
        content_stack: Stack,
        controller: Rc<RefCell<AppController>>,
    ) -> Self {
        Self {
            state,
            global_shortcuts: Rc::new(RefCell::new(Default::default())),
            content_stack: content_stack.downgrade(),
            controller,
            sidebar: Rc::new(RefCell::new(None)),
            binding: None,
            registry_binding: None,
        }
    }

    /// Borrow `AppState` for `f`.
    ///
    /// Panics when `update` already holds the cell. Shared `read` calls nest.
    /// `f` must not `dispatch`, persist, or refresh another page.
    pub(crate) fn read<R>(&self, f: impl FnOnce(&AppState) -> R) -> R {
        f(&self.state.borrow())
    }

    /// Borrow `AppState` mutably for `f`.
    ///
    /// Returns [`ReentrantUpdate`] when the cell is already borrowed, including
    /// a nested `update`. The closure does not run, so the write is not dropped
    /// on the floor. `f` must not `dispatch`, persist, or refresh another page.
    ///
    /// shortcut: no production caller until pages leave `state.borrow`; delete the allow then.
    #[cfg_attr(not(test), allow(dead_code))]
    #[track_caller]
    pub(crate) fn update<R>(
        &self,
        f: impl FnOnce(&mut AppState) -> R,
    ) -> Result<R, ReentrantUpdate> {
        let caller = std::panic::Location::caller();
        let Ok(mut state) = self.state.try_borrow_mut() else {
            tracing::error!(%caller, "reentrant NavigationContext::update");
            // `cargo test` is a debug build. An unconditional `debug_assert`
            // would abort before the caller can observe `Err`.
            #[cfg(not(test))]
            debug_assert!(false, "reentrant NavigationContext::update at {caller}");
            return Err(ReentrantUpdate);
        };
        Ok(f(&mut state))
    }

    /// State-only context. `new` needs a GTK `Stack`, which unit tests have no display for.
    #[cfg(test)]
    fn for_borrow_tests(
        state: Rc<RefCell<AppState>>,
        controller: Rc<RefCell<AppController>>,
    ) -> Self {
        Self {
            state,
            global_shortcuts: Rc::new(RefCell::new(Default::default())),
            content_stack: glib::WeakRef::new(),
            controller,
            sidebar: Rc::new(RefCell::new(None)),
            binding: None,
            registry_binding: None,
        }
    }

    /// Bind dynamic target callbacks to their displayed OBS session and collection.
    pub(crate) fn scoped(&self) -> Self {
        let mut nav = self.clone();
        if nav.binding.is_none() {
            let state = self.state.borrow();
            nav.binding = Some(NavigationBinding {
                epoch: state.workspace.epoch,
                scope: state.workflow_scope(),
            });
        }
        nav
    }

    pub(crate) fn is_current(&self) -> bool {
        self.binding.as_ref().is_none_or(|binding| {
            let state = self.state.borrow();
            binding.accepts(
                state.workspace.epoch,
                &state.workflow_scope(),
                matches!(state.obs_status, ObsStatus::Connected { .. }),
            )
        })
    }

    /// Bind local edits to the curation that was displayed when a callback formed.
    pub(crate) fn registry_scoped(&self) -> Self {
        let mut nav = self.clone();
        if nav.registry_binding.is_none() {
            let state = self.state.borrow();
            nav.registry_binding = Some((
                state.show_registry.target.clone(),
                state.show_registry.generation,
            ));
        }
        nav
    }

    pub(crate) fn registry_is_current(&self) -> bool {
        let state = self.state.borrow();
        state.show_registry.status == crate::controller::state::RegistryStatus::Ready
            && !crate::ui::persist::registry_import_busy()
            && self
                .registry_binding
                .as_ref()
                .is_none_or(|(target, generation)| {
                    target == &state.show_registry.target
                        && *generation == state.show_registry.generation
                })
    }

    /// Hand the sidebar list to every clone of this context.
    ///
    /// Called once, after `build_sidebar`.  The `Rc<RefCell<…>>` is what makes
    /// it reach clones that pages already hold.
    pub(crate) fn attach_sidebar(&self, list: &ListBox, pages: &'static [Page]) {
        *self.sidebar.borrow_mut() = Some(Sidebar {
            list: list.downgrade(),
            pages,
        });
    }

    /// Switch the visible content page and update the model.
    pub(crate) fn switch_to_page(&self, page: Page) {
        self.state.borrow_mut().set_page(page);
        if let Some(stack) = self.content_stack.upgrade() {
            stack.set_visible_child_name(page.id());
        }
        self.select_sidebar_row(page);
    }

    /// Move the sidebar highlight onto `page`.
    ///
    /// GTK does not re-emit `row-selected` when the row is already selected,
    /// so the sidebar's own handler calling back into `switch_to_page` settles
    /// after one hop rather than looping.
    fn select_sidebar_row(&self, page: Page) {
        // Read the sidebar out of the cell and drop the borrow before touching
        // GTK: selecting a row runs the sidebar's own handler, which comes
        // straight back into this type.
        let target = {
            let sidebar = self.sidebar.borrow();
            let Some(sidebar) = sidebar.as_ref() else {
                return;
            };
            let Some(list) = sidebar.list.upgrade() else {
                return;
            };
            let row = sidebar
                .pages
                .iter()
                .position(|candidate| *candidate == page)
                .and_then(|index| list.row_at_index(index as i32));
            Some((list, row))
        };
        if let Some((list, row)) = target {
            list.select_row(row.as_ref());
        }
    }

    pub(crate) fn accepts_session_event(&self, generation: u64, collection: Option<u64>) -> bool {
        self.controller
            .borrow()
            .accepts_scope(generation, collection)
    }

    /// Send a command to the AppController.
    pub(crate) fn dispatch(&self, cmd: AppCommand) {
        if !self.is_current() {
            return;
        }
        if let AppCommand::SwitchPrimaryScene(ref scene) = cmd {
            // Card order and hotkey slots come from curation, so a switch
            // waits until it is Ready rather than acting on a guessed scene.
            if !self.registry_is_current() {
                return;
            }
            let (action, refresh) = {
                let state = self.state.borrow();
                (
                    state.workspace.scene_action(scene),
                    !state.workspace.loading && !state.workspace.pending,
                )
            };
            if let Some(action) = action {
                crate::ui::workspace::act(self, action);
            } else if refresh {
                crate::ui::workspace::refresh(self);
            }
            return;
        }
        if matches!(
            &cmd,
            AppCommand::Connect | AppCommand::Disconnect | AppCommand::SetCurrentSceneCollection(_)
        ) {
            self.global_shortcuts.borrow_mut().stop();
        }
        self.state
            .borrow_mut()
            .note_command_intent(&cmd, crate::infra::clock::unix_ms());
        self.controller.borrow_mut().handle(cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::{NavigationBinding, NavigationContext, ReentrantUpdate};
    use crate::controller::app_controller::AppController;
    use crate::controller::{state::Page, test_support::state::app_state};
    use crate::domain::workflow::ScopeBinding;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tracing::field::{Field, Visit};
    use tracing::Subscriber;
    use tracing_subscriber::layer::{Context, SubscriberExt};
    use tracing_subscriber::Layer;

    struct ReentrantLog(Arc<AtomicBool>);

    struct ReentrantVisit {
        message: bool,
        caller: bool,
    }

    impl ReentrantVisit {
        fn note(&mut self, name: &str, value: &str) {
            if name == "message" && value.contains("reentrant") {
                self.message = true;
            }
            if name == "caller" && !value.is_empty() {
                self.caller = true;
            }
        }
    }

    impl Visit for ReentrantVisit {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.note(field.name(), &format!("{value:?}"));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.note(field.name(), value);
        }
    }

    impl<S: Subscriber> Layer<S> for ReentrantLog {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let mut visit = ReentrantVisit {
                message: false,
                caller: false,
            };
            event.record(&mut visit);
            if visit.message && visit.caller {
                self.0.store(true, Ordering::Relaxed);
            }
        }
    }

    fn borrow_nav() -> (tokio::runtime::Runtime, NavigationContext) {
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let (tx, _rx) = std::sync::mpsc::sync_channel(1);
        let controller = Rc::new(RefCell::new(AppController::new(
            runtime.handle().clone(),
            tx,
        )));
        let state = Rc::new(RefCell::new(app_state()));
        let nav = NavigationContext::for_borrow_tests(state, controller);
        (runtime, nav)
    }

    #[test]
    fn nested_update_returns_err_and_read_inside_read_works() {
        let (_runtime, nav) = borrow_nav();
        assert!(nav.read(|outer| nav.read(|inner| inner.current_page == outer.current_page)));

        let logged = Arc::new(AtomicBool::new(false));
        let subscriber = tracing_subscriber::registry().with(ReentrantLog(Arc::clone(&logged)));
        let updated = tracing::subscriber::with_default(subscriber, || {
            nav.update(|state| {
                state.set_page(Page::Stats);
                let nested = nav.update(|inner| inner.set_page(Page::Mixer));
                assert_eq!(nested, Err(ReentrantUpdate));
                state.current_page
            })
        });

        assert_eq!(updated, Ok(Page::Stats));
        assert_eq!(nav.read(|state| state.current_page), Page::Stats);
        assert!(
            logged.load(Ordering::Relaxed),
            "reentrant update was not logged"
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_navigation_does_not_retain_its_stack_or_sidebar() {
        crate::ui::native_test!(|| {
            use super::*;
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let (tx, _rx) = std::sync::mpsc::sync_channel(1);
            let controller = Rc::new(RefCell::new(AppController::new(
                runtime.handle().clone(),
                tx,
            )));
            let state = Rc::new(RefCell::new(app_state()));
            let stack = Stack::new();
            let list = ListBox::new();
            let weak_stack = stack.downgrade();
            let weak_list = list.downgrade();
            let nav = NavigationContext::new(state, stack.clone(), controller);
            nav.attach_sidebar(&list, &[Page::Live]);
            drop(stack);
            drop(list);
            assert!(weak_stack.upgrade().is_none());
            assert!(weak_list.upgrade().is_none());
            nav.switch_to_page(Page::Live);
        });
    }

    #[test]
    fn scene_switch_dispatch_waits_for_ready_curation() {
        use crate::controller::command::AppCommand;
        use crate::controller::state::{ObsStatus, RegistryStatus};
        let (_runtime, nav) = borrow_nav();
        nav.update(|state| {
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.scene_collections.current = Some("Show".into());
            let mut snapshot = crate::domain::control::ControlSnapshot::default();
            snapshot.studio.enabled = Some(false);
            state.workspace.snapshot = Some(snapshot);
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.observe_scene_controls();
        })
        .expect("AppState is not borrowed");
        let pending_scene = || nav.read(|state| state.workspace.pending_scene.clone());

        for status in [
            RegistryStatus::Loading,
            RegistryStatus::Unavailable,
            RegistryStatus::Failed("corrupt show-workspaces.json".into()),
        ] {
            nav.update(|state| state.show_registry.status = status.clone())
                .expect("AppState is not borrowed");
            nav.dispatch(AppCommand::SwitchPrimaryScene("Main".into()));
            assert_eq!(
                pending_scene(),
                None,
                "{status:?} curation admitted a switch"
            );
        }

        nav.update(|state| state.show_registry.status = RegistryStatus::Ready)
            .expect("AppState is not borrowed");
        nav.dispatch(AppCommand::SwitchPrimaryScene("Main".into()));
        assert_eq!(pending_scene().as_deref(), Some("Main"));
    }

    #[test]
    fn dynamic_navigation_rejects_reconnect_and_another_target_scope() {
        let scope = ScopeBinding {
            endpoint: "obs:4455".into(),
            collection: "Show".into(),
        };
        let binding = NavigationBinding {
            epoch: 3,
            scope: scope.clone(),
        };
        assert!(binding.accepts(3, &scope, true));
        assert!(!binding.accepts(4, &scope, true));
        assert!(!binding.accepts(3, &scope, false));
        assert!(!binding.accepts(
            3,
            &ScopeBinding {
                endpoint: "other:4455".into(),
                ..scope.clone()
            },
            true
        ));
        assert!(!binding.accepts(
            3,
            &ScopeBinding {
                collection: "Other".into(),
                ..scope
            },
            true
        ));
    }
}
