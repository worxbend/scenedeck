//! Application configuration persistence.
//!
//! Stored at `$XDG_CONFIG_HOME/scenedeck/config.json`.
//! Scene role assignments live in `registry.json` (see `storage::registry`).
//! OBS passwords live in the Secret Service (see `storage::secret`).

use std::fs::{create_dir_all, read_to_string, write};
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::appearance::ThemeMode;
use crate::storage::xdg;

// ── Config structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub obs: ObsConfig,
    #[serde(default)]
    pub live: LiveConfig,
    #[serde(default)]
    pub theme_mode: ThemeMode,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            obs: ObsConfig::default(),
            live: LiveConfig::default(),
            theme_mode: ThemeMode::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsConfig {
    #[serde(default = "default_obs_host")]
    pub host: String,
    #[serde(default = "default_obs_port")]
    pub port: u16,
}

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            host: default_obs_host(),
            port: default_obs_port(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveConfig {
    /// Roles whose scenes appear on the Live page.
    #[serde(default = "default_show_roles")]
    pub show_roles: Vec<String>,
    /// OBS input names shown in the audio mixer.
    #[serde(default)]
    pub audio_inputs: Vec<String>,
    /// Roles whose scenes can be switched from Live.
    #[serde(default = "default_allow_switching")]
    pub allow_switching_only: Vec<String>,
}

// ── Defaults ──────────────────────────────────────────────────────────────────

/// Current on-disk config schema version.  Bump when adding a migration.
pub const CURRENT_VERSION: u32 = 1;

fn default_version() -> u32 {
    1
}
fn default_obs_host() -> String {
    "127.0.0.1".to_string()
}
fn default_obs_port() -> u16 {
    4455
}
fn default_show_roles() -> Vec<String> {
    vec!["primary".to_string()]
}
fn default_allow_switching() -> Vec<String> {
    vec!["primary".to_string()]
}

// ── Load / save ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct LoadedConfig {
    pub config: AppConfig,
    pub startup_notice: Option<ConfigStartupNotice>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConfigStartupNotice {
    FirstLaunch,
    ReadFailed(String),
    ParseFailed(String),
}

impl ConfigStartupNotice {
    pub fn user_message(&self) -> String {
        match self {
            Self::FirstLaunch => "No saved settings yet. Defaults are loaded.".to_string(),
            Self::ReadFailed(err) => format!("Settings could not be read: {err}"),
            Self::ParseFailed(err) => format!("Settings could not be parsed: {err}"),
        }
    }
}

pub fn read_config() -> LoadedConfig {
    read_config_from_path(&xdg::config_dir().join("config.json"))
}

pub fn read_config_from_path(path: &Path) -> LoadedConfig {
    match read_to_string(path) {
        Ok(raw) => match serde_json::from_str::<AppConfig>(&raw) {
            Ok(mut config) => {
                // Apply any schema migrations and persist if the version changed.
                if migrate(&mut config) {
                    let _ = write_config_to_path(path, &config);
                }
                LoadedConfig {
                    config,
                    startup_notice: None,
                }
            }
            Err(err) => LoadedConfig {
                config: AppConfig::default(),
                startup_notice: Some(ConfigStartupNotice::ParseFailed(err.to_string())),
            },
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => LoadedConfig {
            config: AppConfig::default(),
            startup_notice: Some(ConfigStartupNotice::FirstLaunch),
        },
        Err(err) => LoadedConfig {
            config: AppConfig::default(),
            startup_notice: Some(ConfigStartupNotice::ReadFailed(err.to_string())),
        },
    }
}

/// Upgrade an older config in place.  Returns `true` if anything changed
/// (so the caller can re-persist).  Each arm handles one version step.
fn migrate(config: &mut AppConfig) -> bool {
    let mut changed = false;
    // Example of the migration pattern for future schema bumps:
    //   while config.version < CURRENT_VERSION {
    //       match config.version {
    //           1 => { /* migrate v1 → v2 */ config.version = 2; }
    //           _ => break,
    //       }
    //       changed = true;
    //   }
    if config.version > CURRENT_VERSION {
        // Config written by a newer build; clamp so we don't loop, but keep
        // the user's data as-is.
        tracing::warn!(
            version = config.version,
            "config is newer than this build supports"
        );
    } else if config.version < CURRENT_VERSION {
        config.version = CURRENT_VERSION;
        changed = true;
    }
    changed
}

pub fn write_config(config: &AppConfig) -> io::Result<()> {
    write_config_to_path(&xdg::config_dir().join("config.json"), config)
}

pub fn write_config_to_path(path: &Path, config: &AppConfig) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }
    let raw = serde_json::to_string_pretty(config).map_err(io::Error::other)?;
    write(path, raw)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_uses_defaults() {
        let c: AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(c.version, 1);
        assert_eq!(c.obs.host, "127.0.0.1");
        assert_eq!(c.obs.port, 4455);
        assert_eq!(c.theme_mode, ThemeMode::System);
    }

    #[test]
    fn round_trips_through_json() {
        let config = AppConfig {
            version: 1,
            obs: ObsConfig {
                host: "192.168.1.10".to_string(),
                port: 4455,
            },
            live: LiveConfig::default(),
            theme_mode: ThemeMode::Dark,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.obs.host, "192.168.1.10");
        assert_eq!(parsed.theme_mode, ThemeMode::Dark);
    }

    #[test]
    fn unknown_theme_mode_uses_system_fallback() {
        let parsed: AppConfig = serde_json::from_str(r#"{"theme_mode":"future"}"#).unwrap();

        assert_eq!(parsed.theme_mode, ThemeMode::System);
    }
}
