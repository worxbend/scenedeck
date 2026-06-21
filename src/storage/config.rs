//! Application configuration persistence.
//!
//! Stored at `$XDG_CONFIG_HOME/scenedeck/config.json`.
//! Scene role assignments live in `registry.json` (see `storage::registry`).
//! OBS passwords live in the Secret Service (see `storage::secret`).

use std::fs::{create_dir_all, read_to_string, write};
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::appearance::{ThemeMode, ThemePreference};
use crate::domain::mixer::MixerSelection;
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
    pub outputs: OutputConfig,
    #[serde(default)]
    pub appearance: ThemePreference,
    #[serde(default)]
    pub mixer: MixerSelection,
    #[serde(default, rename = "theme_mode", skip_serializing)]
    pub(crate) legacy_theme_mode: Option<ThemeMode>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            obs: ObsConfig::default(),
            live: LiveConfig::default(),
            outputs: OutputConfig::default(),
            appearance: ThemePreference::default(),
            mixer: MixerSelection::default(),
            legacy_theme_mode: None,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default)]
    pub confirm_start_stream: bool,
    #[serde(default = "default_confirm_stop_stream")]
    pub confirm_stop_stream: bool,
    #[serde(default)]
    pub confirm_start_recording: bool,
    #[serde(default = "default_confirm_stop_recording")]
    pub confirm_stop_recording: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            confirm_start_stream: false,
            confirm_stop_stream: true,
            confirm_start_recording: false,
            confirm_stop_recording: true,
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
pub const CURRENT_VERSION: u32 = 2;

fn default_version() -> u32 {
    CURRENT_VERSION
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
fn default_confirm_stop_stream() -> bool {
    true
}
fn default_confirm_stop_recording() -> bool {
    true
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
                if migrate(&mut config) || config.appearance.migrate_legacy_custom_css_path() {
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
    if config.version > CURRENT_VERSION {
        // Config written by a newer build; clamp so we don't loop, but keep
        // the user's data as-is.
        tracing::warn!(
            version = config.version,
            "config is newer than this build supports"
        );
    } else {
        while config.version < CURRENT_VERSION {
            match config.version {
                0 | 1 => {
                    if let Some(mode) = config.legacy_theme_mode.take() {
                        config.appearance.mode = mode;
                    }
                    config.version = 2;
                    changed = true;
                }
                _ => {
                    config.version = CURRENT_VERSION;
                    changed = true;
                }
            }
        }
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
        assert_eq!(c.version, CURRENT_VERSION);
        assert_eq!(c.obs.host, "127.0.0.1");
        assert_eq!(c.obs.port, 4455);
        assert!(!c.outputs.confirm_start_stream);
        assert!(c.outputs.confirm_stop_stream);
        assert!(!c.outputs.confirm_start_recording);
        assert!(c.outputs.confirm_stop_recording);
        assert_eq!(c.appearance.mode, ThemeMode::System);
        assert_eq!(c.appearance.selected_theme_id(), "adwaita-default");
        assert_eq!(c.mixer, MixerSelection::default());
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
            appearance: ThemePreference {
                mode: ThemeMode::Dark,
                ..ThemePreference::default()
            },
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.obs.host, "192.168.1.10");
        assert_eq!(parsed.appearance.mode, ThemeMode::Dark);
    }

    #[test]
    fn unknown_theme_mode_uses_system_fallback() {
        let parsed: AppConfig =
            serde_json::from_str(r#"{"appearance":{"mode":"future"}}"#).unwrap();

        assert_eq!(parsed.appearance.mode, ThemeMode::System);
    }

    #[test]
    fn migrates_v1_theme_mode_to_v2_appearance() {
        let path = std::env::temp_dir().join(format!(
            "scenedeck-config-migration-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"{
              "version": 1,
              "obs": { "host": "127.0.0.1", "port": 4455 },
              "live": {
                "show_roles": ["primary"],
                "audio_inputs": [],
                "allow_switching_only": ["primary"]
              },
              "theme_mode": "dark"
            }"#,
        )
        .unwrap();

        let loaded = read_config_from_path(&path);
        let persisted = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert_eq!(loaded.config.version, CURRENT_VERSION);
        assert_eq!(loaded.config.appearance.mode, ThemeMode::Dark);
        assert!(persisted.contains(r#""version": 2"#));
        assert!(persisted.contains(r#""appearance""#));
        assert!(!persisted.contains(r#""theme_mode""#));
    }
}
