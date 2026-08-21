//! Application configuration persistence.
//!
//! Stored at `$XDG_CONFIG_HOME/scenedeck/config.json`.
//! Scene role assignments live in `registry.json` (see `storage::registry`).
//! OBS passwords live in the Secret Service (see `storage::secret`).

use std::fs::read_to_string;
use std::io;
use std::path::{Path, PathBuf};

use i18n_embed_fl::fl;
use serde::{Deserialize, Serialize};

use crate::domain::appearance::{Language, ThemeMode, ThemePreference};
use crate::domain::hotkey::SceneHotkeyConfig;
use crate::domain::mixer::MixerSelection;
use crate::infra::i18n::LANGUAGE_LOADER;
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
    pub language: Language,
    #[serde(default)]
    pub mixer: MixerSelection,
    /// Live-page scene-switch shortcuts.
    #[serde(default)]
    pub hotkeys: SceneHotkeyConfig,
    /// What the first-run onboarding has already shown this user.
    #[serde(default)]
    pub onboarding: OnboardingConfig,
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
            language: Language::default(),
            mixer: MixerSelection::default(),
            hotkeys: SceneHotkeyConfig::default(),
            onboarding: OnboardingConfig::default(),
            legacy_theme_mode: None,
        }
    }
}

/// First-run onboarding state.
///
/// SceneDeck greets a brand-new user with a welcome dialog that points at the
/// Help page. `welcome_shown` records that the greeting has happened, so it
/// only ever interrupts once. It defaults to `false`, which means a config file
/// written before this field existed also gets the greeting exactly once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingConfig {
    #[serde(default)]
    pub welcome_shown: bool,
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
pub const CURRENT_VERSION: u32 = 3;

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
    /// The file exists but is not valid JSON. `backup` is where the original
    /// was moved to, when it could be moved at all.
    ParseFailed {
        detail: String,
        backup: Option<PathBuf>,
    },
}

impl ConfigStartupNotice {
    pub fn user_message(&self) -> String {
        match self {
            Self::FirstLaunch => fl!(LANGUAGE_LOADER, "config-first-launch"),
            Self::ReadFailed(err) => {
                fl!(LANGUAGE_LOADER, "config-read-failed", detail = err.as_str())
            }
            Self::ParseFailed { detail, backup } => match backup {
                Some(backup) => fl!(
                    LANGUAGE_LOADER,
                    "config-parse-failed-backed-up",
                    detail = detail.as_str(),
                    path = backup.display().to_string()
                ),
                None => fl!(
                    LANGUAGE_LOADER,
                    "config-parse-failed",
                    detail = detail.as_str()
                ),
            },
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
                // Apply any schema migrations and persist if anything changed.
                if apply_migrations(&mut config) {
                    let _ = write_config_to_path(path, &config);
                }
                LoadedConfig {
                    config,
                    startup_notice: None,
                }
            }
            Err(err) => LoadedConfig {
                config: AppConfig::default(),
                startup_notice: Some(ConfigStartupNotice::ParseFailed {
                    detail: err.to_string(),
                    backup: preserve_unparsable_config(path),
                }),
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

/// Move an unparsable config aside so the next save does not destroy it.
///
/// A config that will not parse is replaced in memory by defaults, and the
/// next write — any switch on the Settings page, a mixer change, or the
/// first-run dialog recording that it has been shown — persists those defaults
/// straight over the file. One stray comma in a hand edit would otherwise cost
/// the user every setting they had.
///
/// The registry loader already takes this position for its own file, refusing
/// to coerce a bad load into an empty registry; this brings the config in
/// line, by keeping the original rather than by refusing to save.
///
/// Returns where the original went, or `None` if it could not be moved — in
/// which case the app still starts on defaults, because refusing to start
/// helps nobody.
///
/// A second bad launch overwrites the previous backup. That is deliberate:
/// the interesting file is the one the user last edited, and a directory
/// slowly filling with timestamped copies of a broken config is its own
/// problem.
fn preserve_unparsable_config(path: &Path) -> Option<PathBuf> {
    let backup = path.with_extension("json.invalid");
    match std::fs::rename(path, &backup) {
        Ok(()) => {
            tracing::warn!(
                original = %path.display(),
                backup = %backup.display(),
                "config could not be parsed; kept the original and started on defaults"
            );
            Some(backup)
        }
        Err(error) => {
            tracing::warn!(%error, "could not preserve the unparsable config");
            None
        }
    }
}

/// Run every migration step over a freshly parsed config.
///
/// Returns `true` if any step changed something, so the caller re-persists.
///
/// Each step is bound to its own local and the results are combined
/// afterwards, rather than being chained with `||`. `||` stops evaluating at
/// the first `true`, so chaining lets a step that fired suppress every step
/// after it — which is exactly what happened here: `migrate` returns `true`
/// for any config older than `CURRENT_VERSION`, and every config still
/// carrying the legacy `custom_css.path` key is older than that, so the custom
/// stylesheet migration never ran. The rewrite that followed then dropped the
/// key for good, because `legacy_path` is `skip_serializing`.
fn apply_migrations(config: &mut AppConfig) -> bool {
    let version_changed = migrate(config);
    let custom_css_changed = config.appearance.migrate_legacy_custom_css_path();
    version_changed || custom_css_changed
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
                2 => {
                    // The first-run welcome dialog is for people meeting
                    // SceneDeck for the first time, not for people who have
                    // been using it and just upgraded.  A config file that
                    // already exists means the user is neither new nor in
                    // need of being interrupted, so the greeting is marked
                    // as already given.
                    config.onboarding.welcome_shown = true;
                    config.version = 3;
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
    let raw = serde_json::to_string_pretty(config).map_err(io::Error::other)?;
    // Whole-file replacement: a half-written file would be unparsable, and the
    // loaders treat unparsable as absent.
    crate::storage::atomic::write(path, raw.as_bytes())
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
        assert_eq!(c.hotkeys, SceneHotkeyConfig::default());
        assert!(!c.onboarding.welcome_shown);
    }

    /// Writes `contents` to a uniquely named temp file, runs `read_config_from_path`
    /// over it, and hands back both the loaded config and whatever ended up on
    /// disk afterwards.  The file is removed before the assertions run so a
    /// failing test does not leave litter behind.
    fn load_from_temp_file(contents: &str) -> (AppConfig, String) {
        let path = std::env::temp_dir().join(format!(
            "scenedeck-config-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, contents).unwrap();
        let loaded = read_config_from_path(&path);
        let persisted = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(path);
        (loaded.config, persisted)
    }

    #[test]
    fn an_unparsable_config_is_kept_rather_than_overwritten() {
        // The scenario: a hand edit leaves a stray comma. The app starts on
        // defaults, and the very next save would land on this path — so the
        // original has to be somewhere else by then.
        let dir = std::env::temp_dir().join(format!(
            "scenedeck-config-invalid-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let original = br#"{ "obs": { "host": "192.168.1.42",, } }"#;
        std::fs::write(&path, original).unwrap();

        let loaded = read_config_from_path(&path);

        let backup = dir.join("config.json.invalid");
        assert!(backup.exists(), "the unparsable file was not preserved");
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        assert!(
            !path.exists(),
            "the bad file was left where a save would hit it"
        );

        match loaded.startup_notice {
            Some(ConfigStartupNotice::ParseFailed {
                backup: Some(at), ..
            }) => {
                assert_eq!(at, backup);
            }
            other => panic!("expected a ParseFailed notice naming the backup, got {other:?}"),
        }
        // The app still starts, on defaults.
        assert_eq!(loaded.config.obs.host, "127.0.0.1");

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn existing_configs_are_not_greeted_on_upgrade() {
        // Anyone whose config predates the Help page has already been using
        // SceneDeck, so the welcome dialog must not interrupt their upgrade.
        let (config, persisted) = load_from_temp_file(r#"{ "version": 2 }"#);

        assert_eq!(config.version, CURRENT_VERSION);
        assert!(config.onboarding.welcome_shown);
        assert!(persisted.contains(r#""welcome_shown": true"#));
    }

    #[test]
    fn a_fresh_install_is_greeted() {
        // No config file at all is the one case that means "new user".
        let missing = std::env::temp_dir().join(format!(
            "scenedeck-config-absent-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let loaded = read_config_from_path(&missing);

        assert_eq!(
            loaded.startup_notice,
            Some(ConfigStartupNotice::FirstLaunch)
        );
        assert!(!loaded.config.onboarding.welcome_shown);
    }

    #[test]
    fn onboarding_flag_round_trips() {
        let parsed: AppConfig =
            serde_json::from_str(r#"{"onboarding":{"welcome_shown":true}}"#).unwrap();
        assert!(parsed.onboarding.welcome_shown);

        let json = serde_json::to_string(&parsed).unwrap();
        assert!(
            serde_json::from_str::<AppConfig>(&json)
                .unwrap()
                .onboarding
                .welcome_shown
        );
    }

    #[test]
    fn hotkey_section_round_trips_and_tolerates_partial_objects() {
        use crate::domain::hotkey::{LeaderKey, SceneHotkeyStyle};

        let parsed: AppConfig =
            serde_json::from_str(r#"{"hotkeys":{"style":"leader","leader":"comma"}}"#).unwrap();

        assert!(parsed.hotkeys.enabled);
        assert_eq!(parsed.hotkeys.style, SceneHotkeyStyle::Leader);
        assert_eq!(parsed.hotkeys.leader, LeaderKey::Comma);
        assert_eq!(
            parsed.hotkeys.leader_timeout_ms,
            crate::domain::hotkey::DEFAULT_LEADER_TIMEOUT_MS
        );

        let json = serde_json::to_string(&parsed).unwrap();
        assert_eq!(
            serde_json::from_str::<AppConfig>(&json).unwrap().hotkeys,
            parsed.hotkeys
        );
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
    fn a_legacy_custom_css_path_survives_a_version_migration() {
        // Both migration steps have to run on the same load. The version bump
        // fires for every config this old, and it used to short-circuit the
        // custom-stylesheet step, which cost the user the path entirely: the
        // legacy `path` key is `skip_serializing`, so the rewrite that follows
        // a migration is what deleted it.
        let (config, persisted) = load_from_temp_file(
            r#"{
              "version": 2,
              "appearance": { "custom_css": { "enabled": true, "path": "/tmp/scenedeck.css" } }
            }"#,
        );

        assert_eq!(config.version, CURRENT_VERSION);
        // Assert the value, not the key: `light_path` is `#[serde(default)]`
        // with no `skip_serializing_if`, so it is written as `null` even when
        // the migration did not run. Checking only for the key name would
        // pass against the very bug this test exists to catch.
        assert!(
            persisted.contains(r#""light_path": "/tmp/scenedeck.css""#),
            "{persisted}"
        );
        assert!(
            persisted.contains(r#""dark_path": "/tmp/scenedeck.css""#),
            "{persisted}"
        );
    }

    #[test]
    fn migrates_v1_theme_mode_to_v2_appearance() {
        let (config, persisted) = load_from_temp_file(
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
        );

        // A v1 config runs through every later step in one pass, so it lands on
        // the current version rather than stopping at 2.
        assert_eq!(config.version, CURRENT_VERSION);
        assert_eq!(config.appearance.mode, ThemeMode::Dark);
        assert!(persisted.contains(r#""appearance""#));
        assert!(!persisted.contains(r#""theme_mode""#));
        // It is an existing config, so it is not a first run.
        assert!(config.onboarding.welcome_shown);
    }
}
