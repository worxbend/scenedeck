//! Shared handle passed to every page builder.
//!
//! Pages use `nav.switch_to_page()` for navigation and `nav.dispatch()` to
//! send commands to the AppController.  The stack and controller are private;
//! pages only interact through these two methods.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::{ListBox, Stack};

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::state::{AppState, Page};

#[derive(Clone)]
pub(crate) struct NavigationContext {
    pub(crate) state: Rc<RefCell<AppState>>,
    content_stack: Stack,
    controller: Rc<RefCell<AppController>>,
    /// Sidebar list and the page order it was built from, once the window has
    /// built them.  Held so that navigation triggered from anywhere else — a
    /// Help topic button, a keyboard action, the first-run dialog — moves the
    /// sidebar highlight too, instead of leaving it pointing at the page you
    /// just left.
    sidebar: Rc<RefCell<Option<Sidebar>>>,
}

/// The sidebar list plus the page each of its rows stands for, in row order.
struct Sidebar {
    list: ListBox,
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
            content_stack,
            controller,
            sidebar: Rc::new(RefCell::new(None)),
        }
    }

    /// Hand the sidebar list to every clone of this context.
    ///
    /// Called once, after `build_sidebar`.  The `Rc<RefCell<…>>` is what makes
    /// it reach clones that pages already hold.
    pub(crate) fn attach_sidebar(&self, list: &ListBox, pages: &'static [Page]) {
        *self.sidebar.borrow_mut() = Some(Sidebar {
            list: list.clone(),
            pages,
        });
    }

    /// Switch the visible content page and update the model.
    pub(crate) fn switch_to_page(&self, page: Page) {
        self.state.borrow_mut().set_page(page);
        self.content_stack.set_visible_child_name(page.id());
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
            let Some(index) = sidebar
                .pages
                .iter()
                .position(|candidate| *candidate == page)
            else {
                return;
            };
            sidebar
                .list
                .row_at_index(index as i32)
                .map(|row| (sidebar.list.clone(), row))
        };
        if let Some((list, row)) = target {
            list.select_row(Some(&row));
        }
    }

    /// Send a command to the AppController.
    pub(crate) fn dispatch(&self, cmd: AppCommand) {
        self.controller.borrow_mut().handle(cmd);
    }
}
