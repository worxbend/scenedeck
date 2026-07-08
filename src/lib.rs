#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! SceneDeck application crate.
//!
//! The crate is organized by application boundary: domain types, storage
//! adapters, OBS adapters, controller orchestration, and GTK UI modules. The
//! binary entry point calls [`run`]; the internal modules stay crate-private so
//! the application does not accidentally expose its implementation layers as a
//! library API.

pub(crate) mod app;
pub(crate) mod app_info;
pub(crate) mod controller;
pub(crate) mod domain;
pub(crate) mod infra;
pub(crate) mod obs;
pub(crate) mod services;
pub(crate) mod storage;
pub(crate) mod ui;

/// Start the SceneDeck GTK application.
///
/// This initializes logging, creates the Tokio runtime used for OBS WebSocket
/// work, and then hands control to the GTK main loop.
pub fn run() {
    infra::logging::init();
    app::run();
}
