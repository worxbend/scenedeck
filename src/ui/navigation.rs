//! Shared handle passed to every page builder.
//!
//! Pages use `nav.switch_to_page()` for navigation and `nav.dispatch()` to
//! send commands to the AppController.  The stack and controller are private;
//! pages only interact through these two methods.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::Stack;

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::state::{AppState, Page};

#[derive(Clone)]
pub(crate) struct NavigationContext {
    pub(crate) state: Rc<RefCell<AppState>>,
    content_stack: Stack,
    controller: Rc<RefCell<AppController>>,
}

impl NavigationContext {
    pub(crate) fn new(
        state: Rc<RefCell<AppState>>,
        content_stack: Stack,
        controller: Rc<RefCell<AppController>>,
    ) -> Self {
        Self {
            state,
            content_stack,
            controller,
        }
    }

    /// Switch the visible content page and update the model.
    pub(crate) fn switch_to_page(&self, page: Page) {
        self.state.borrow_mut().set_page(page);
        self.content_stack.set_visible_child_name(page.id());
    }

    /// Send a command to the AppController.
    pub(crate) fn dispatch(&self, cmd: AppCommand) {
        self.controller.borrow_mut().handle(cmd);
    }
}
