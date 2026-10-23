//! Application controller boundary.
//!
//! Commands flow from GTK widgets into `AppController`; events flow back to the
//! GTK main thread where `ui::window` applies them to state and widgets.

pub(crate) mod app_controller;
pub(crate) mod command;
pub(crate) mod dependencies;
pub(crate) mod event;
pub(crate) mod event_sender;
pub(crate) mod lifecycle;
pub(crate) mod observations;
pub(crate) mod output_controller;
pub(crate) mod refresh_controller;
pub(crate) mod session_controller;
pub(crate) mod state;
pub(crate) mod workspace;

pub(crate) mod thumbnails;
