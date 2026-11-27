//! XDG base-directory helpers.
//!
//! No path resolves against the process working directory: an `XDG_*_HOME`
//! value counts only when it is absolute, as the XDG Base Directory spec
//! requires, and so does the home directory it falls back to.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const APP_DIR: &str = "scenedeck";
const CONFIG_FILE: &str = "config.json";
const REGISTRY_FILE: &str = "registry.json";
const SHOW_WORKSPACES_FILE: &str = "show-workspaces.json";
const BOOKMARKS_FILE: &str = "bookmarks.json";
const REPORT_HISTORY_FILE: &str = "session-history.json";

/// Path to the application settings file, `config.json`.
pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

/// Path to the scene registry file, `registry.json`.
pub fn registry_path() -> PathBuf {
    config_dir().join(REGISTRY_FILE)
}

pub(crate) fn show_workspaces_path() -> PathBuf {
    config_dir().join(SHOW_WORKSPACES_FILE)
}

pub(crate) fn bookmarks_path() -> PathBuf {
    config_dir().join(BOOKMARKS_FILE)
}

pub(crate) fn report_history_path() -> PathBuf {
    app_dir("XDG_STATE_HOME", ".local/state").join(REPORT_HISTORY_FILE)
}

fn config_dir() -> PathBuf {
    app_dir("XDG_CONFIG_HOME", ".config")
}

/// The application's directory under the base directory named by `var`.
fn app_dir(var: &str, home_suffix: &str) -> PathBuf {
    base_dir(env::var_os(var), env::home_dir(), home_suffix).join(APP_DIR)
}

/// Resolve one XDG base directory: the `XDG_*_HOME` value when it is
/// absolute, else `home_suffix` under an absolute home directory.
///
/// With neither, the base sits under the filesystem root rather than the
/// working directory. Only root can write there, so a save fails visibly and
/// a read finds no file and keeps the defaults, instead of picking up or
/// planting files wherever the process happened to start.
fn base_dir(xdg_value: Option<OsString>, home: Option<PathBuf>, home_suffix: &str) -> PathBuf {
    if let Some(dir) = xdg_value.map(PathBuf::from).filter(|dir| dir.is_absolute()) {
        return dir;
    }
    if let Some(home) = home.filter(|home| home.is_absolute()) {
        return home.join(home_suffix);
    }
    tracing::warn!(
        home_suffix,
        "no absolute XDG base directory or home directory; resolving under the filesystem root"
    );
    Path::new("/").join(home_suffix)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::base_dir;

    fn home() -> Option<PathBuf> {
        Some(PathBuf::from("/home/operator"))
    }

    #[test]
    fn absolute_xdg_value_is_the_base_dir() {
        let resolved = base_dir(Some(OsString::from("/srv/xdg")), home(), ".config");

        assert_eq!(resolved, PathBuf::from("/srv/xdg"));
    }

    #[test]
    fn relative_xdg_value_is_ignored_in_favour_of_home() {
        let resolved = base_dir(Some(OsString::from("xdg")), home(), ".config");

        assert_eq!(resolved, PathBuf::from("/home/operator/.config"));
    }

    #[test]
    fn empty_xdg_value_is_ignored_in_favour_of_home() {
        let resolved = base_dir(Some(OsString::new()), home(), ".local/state");

        assert_eq!(resolved, PathBuf::from("/home/operator/.local/state"));
    }

    #[test]
    fn relative_home_never_resolves_against_the_working_directory() {
        let resolved = base_dir(None, Some(PathBuf::from("operator")), ".config");

        assert_eq!(resolved, PathBuf::from("/.config"));
    }

    #[test]
    fn unset_home_and_xdg_never_resolve_against_the_working_directory() {
        let resolved = base_dir(None, None, ".config");

        assert_eq!(resolved, PathBuf::from("/.config"));
    }
}
