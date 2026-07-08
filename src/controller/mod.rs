//! Application controller boundary.
//!
//! Commands flow from GTK widgets into `AppController`; events flow back to the
//! GTK main thread where `ui::window` applies them to state and widgets.

pub(crate) mod app_controller;
pub(crate) mod command;
pub(crate) mod dependencies;
pub(crate) mod event;
pub(crate) mod state;
