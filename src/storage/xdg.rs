//! XDG base-directory helpers.

use std::env;
use std::path::PathBuf;

const APP_DIR: &str = "scenedeck";
const CONFIG_FILE: &str = "config.json";
const REGISTRY_FILE: &str = "registry.json";

/// Path to the application settings file, `config.json`.
pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

/// Path to the scene registry file, `registry.json`.
pub fn registry_path() -> PathBuf {
    config_dir().join(REGISTRY_FILE)
}

/// Return the config directory, using XDG_CONFIG_HOME, HOME, then the current directory.
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
