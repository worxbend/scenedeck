//! Top-level GTK pages mounted into the main application stack.
//!
//! Each module owns one primary page surface and exposes a `build` entry point
//! that returns the page widget plus any refresh hook needed by navigation.

pub(crate) mod doctor;
pub(crate) mod graph;
pub(crate) mod inventory;
pub(crate) mod live;
pub(crate) mod mixer;
pub(crate) mod settings;
