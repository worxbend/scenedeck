//! XDG base-directory helpers.

use std::env;
use std::path::PathBuf;

const APP_DIR: &str = "scenedeck";

/// Return the application config directory.
///
/// Prefers `XDG_CONFIG_HOME`, falls back to `$HOME/.config`, then to the
/// current directory when neither environment variable is available.
pub fn config_dir() -> PathBuf {
    env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
}
