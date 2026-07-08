//! Application identity constants shared by UI and desktop integration code.

/// Reverse-DNS application id used by desktop files and resources.
pub const APP_ID: &str = "io.scenedeck.app";
/// User-facing application name.
pub const APP_NAME: &str = "SceneDeck";
/// Cargo package version embedded at compile time.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
