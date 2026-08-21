//! XDG base-directory helpers.

use std::env;
use std::path::PathBuf;

const APP_DIR: &str = "scenedeck";
const CONFIG_FILE: &str = "config.json";
const REGISTRY_FILE: &str = "registry.json";

/// Return the application config directory.
///
/// Prefers `XDG_CONFIG_HOME`, falls back to `$HOME/.config`, then to the
/// current directory when neither environment variable is available.
/// Path to the application settings file, `config.json`.
pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

/// Path to the scene registry file, `registry.json`.
pub fn registry_path() -> PathBuf {
    config_dir().join(REGISTRY_FILE)
}

fn config_dir() -> PathBuf {
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
