//! Shared handle passed to every page builder.
//!
//! Pages use `nav.switch_to_page()` for navigation and `nav.dispatch()` to
//! send commands to the AppController.  The stack and controller are private;
//! pages only interact through these two methods.

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
    content_stack: glib::WeakRef<Stack>,
    controller: Rc<RefCell<AppController>>,
    /// Sidebar list and the page order it was built from, once the window has
    /// built them.  Held so that navigation triggered from anywhere else — a
    /// Help topic button, a keyboard action, the first-run dialog — moves the
    /// sidebar highlight too, instead of leaving it pointing at the page you
    /// just left.
    sidebar: Rc<RefCell<Option<Sidebar>>>,
    binding: Option<NavigationBinding>,
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

/// The sidebar list plus the page each of its rows stands for, in row order.
struct Sidebar {
    list: glib::WeakRef<ListBox>,
    pages: &'static [Page],
}

impl NavigationContext {
    pub(crate) fn new(
        state: Rc<RefCell<AppState>>,
        content_stack: Stack,
        controller: Rc<RefCell<AppController>>,
    ) -> Self {
        Self {
            state,
            content_stack: content_stack.downgrade(),
            controller,
            sidebar: Rc::new(RefCell::new(None)),
            binding: None,
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
        let requested = match &cmd {
            AppCommand::SwitchPrimaryScene(scene) => Some(i18n_embed_fl::fl!(
                crate::infra::i18n::LANGUAGE_LOADER,
                "control-program",
                scene = scene.clone()
            )),
            AppCommand::StartStreaming => Some(i18n_embed_fl::fl!(
                crate::infra::i18n::LANGUAGE_LOADER,
                "window-sidebar-start-stream"
            )),
            AppCommand::StopStreaming => Some(i18n_embed_fl::fl!(
                crate::infra::i18n::LANGUAGE_LOADER,
                "window-sidebar-stop-stream"
            )),
            AppCommand::StartRecording => Some(i18n_embed_fl::fl!(
                crate::infra::i18n::LANGUAGE_LOADER,
                "window-sidebar-start-recording"
            )),
            AppCommand::StopRecording => Some(i18n_embed_fl::fl!(
                crate::infra::i18n::LANGUAGE_LOADER,
                "window-sidebar-stop-recording"
            )),
            _ => None,
        };
        if let Some(description) = requested {
            let mut state = self.state.borrow_mut();
            state.workspace.log(
                crate::domain::workflow::ActivityOrigin::Requested,
                description,
            );
            state.workspace.revision = state.workspace.revision.wrapping_add(1);
        }
        if matches!(cmd, AppCommand::StopRecording) {
            let mut state = self.state.borrow_mut();
            if state.record_status.active
                && !state.record_status.state.is_transitioning()
                && state.workspace.recording_stop_result.is_none()
            {
                let scope = state.workflow_scope();
                let id = state.workspace.output_results.request(
                    crate::domain::output_result::OutputResultKind::RecordingStop,
                    scope,
                    crate::controller::workspace::unix_ms(),
                );
                state.workspace.recording_stop_result = Some(id);
            }
        }
        self.controller.borrow_mut().handle(cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::NavigationBinding;
    use crate::domain::workflow::ScopeBinding;

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_navigation_does_not_retain_its_stack_or_sidebar() {
        crate::ui::test_support::native_test(
            "ui::navigation::tests::native_navigation_does_not_retain_its_stack_or_sidebar",
            || {
                use super::*;
                let runtime = tokio::runtime::Runtime::new().unwrap();
                let (tx, _rx) = std::sync::mpsc::sync_channel(1);
                let controller = Rc::new(RefCell::new(AppController::new(
                    runtime.handle().clone(),
                    tx,
                )));
                let state = Rc::new(RefCell::new(AppState::new(
                    Default::default(),
                    Default::default(),
                    None,
                    None,
                )));
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
            },
        );
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
