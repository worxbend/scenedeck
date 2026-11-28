//! Application configuration persistence.
//!
//! Stored at `$XDG_CONFIG_HOME/scenedeck/config.json`.
//! Scene curation lives in `show-workspaces.json` (see `storage::show_workspaces`).
//! OBS passwords live in the Secret Service (see `storage::secret`).

use std::io;
use std::path::{Path, PathBuf};

use i18n_embed_fl::fl;
use serde::{Deserialize, Serialize};

use crate::domain::appearance::{Language, ThemeMode, ThemePreference};
use crate::domain::hotkey::SceneHotkeyConfig;
use crate::domain::mixer::MixerSelection;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::{schema_version, xdg};

mod connections;
mod preflight;
pub(crate) use connections::SavedConnection;

// ── Config structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub(crate) session_history: crate::domain::session_report::HistoryPolicy,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub obs: ObsConfig,
    /// Stable saved-connection identity; display names may change independently.
    #[serde(default = "default_connection_id")]
    pub connection_id: String,
    /// Saved endpoints; passwords live only in the keyring.
    #[serde(default, deserialize_with = "connections::deserialize_connections")]
    pub(crate) connections: Vec<SavedConnection>,
    /// Readiness expectations keyed by saved connection and actual endpoint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) preflight_plans: Vec<crate::domain::preflight::SavedPreflight>,
    /// Endpoint eligible to adopt the pre-workspace registry once.
    #[serde(default)]
    pub legacy_registry_endpoint: Option<String>,
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
            session_history: Default::default(),
            version: default_version(),
            obs: ObsConfig::default(),
            connection_id: default_connection_id(),
            connections: Vec::new(),
            preflight_plans: Vec::new(),
            legacy_registry_endpoint: None,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Opt-in scene thumbnails; disabled until the local performance budget passes.
    #[serde(default)]
    pub scene_previews: bool,
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
pub const CURRENT_VERSION: u32 = 5;

fn default_connection_id() -> String {
    "default".into()
}

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
    /// Disk work the read found necessary but left undone, so a read never
    /// writes or renames the file.
    pub pending_repair: Option<PendingRepair>,
}

/// A change to the config file that only startup performs.
///
/// The controller reads the config again on every connect, off the ordered
/// config lane. A write or rename from there could race a Settings save, or
/// move away a hand edit made while the app runs.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PendingRepair {
    /// Migrations changed the loaded config; saving it records them.
    WriteMigrated,
    /// The file does not parse; it has to move aside before any save of the
    /// defaults lands on it.
    PreserveUnparsable,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConfigStartupNotice {
    FirstLaunch,
    ReadFailed(String),
    /// The file is a symbolic link. Config files are opened with O_NOFOLLOW,
    /// so the link is never read through and every save over it is refused.
    Symlinked,
    /// The file exists but is not valid JSON. `backup` is where startup moved
    /// the original, when it could be moved at all.
    ParseFailed {
        detail: String,
        backup: Option<PathBuf>,
    },
    /// The file was written by a newer build. It is loaded as far as this
    /// build understands it, and never written back.
    NewerVersion {
        version: u32,
    },
}

impl ConfigStartupNotice {
    pub fn user_message(&self) -> String {
        match self {
            Self::FirstLaunch => fl!(LANGUAGE_LOADER, "config-first-launch"),
            Self::ReadFailed(err) => {
                fl!(LANGUAGE_LOADER, "config-read-failed", detail = err.as_str())
            }
            Self::Symlinked => fl!(LANGUAGE_LOADER, "config-symlinked"),
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
            Self::NewerVersion { version } => fl!(
                LANGUAGE_LOADER,
                "config-newer-version",
                version = version.to_string()
            ),
        }
    }
}

pub fn read_config() -> LoadedConfig {
    read_config_from_path(&xdg::config_path())
}

/// Read the config without touching the file; see `PendingRepair` for the
/// disk work this leaves to startup.
pub fn read_config_from_path(path: &Path) -> LoadedConfig {
    match crate::storage::small_file::read_to_string(path, crate::storage::small_file::CONFIG_LIMIT)
    {
        Ok(raw) => match schema_version::newer_version(&raw, CURRENT_VERSION) {
            Some(version) => load_newer_config(&raw, version),
            None => load_supported_config(&raw),
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => LoadedConfig {
            config: AppConfig::default(),
            startup_notice: Some(ConfigStartupNotice::FirstLaunch),
            pending_repair: None,
        },
        Err(err) if crate::storage::small_file::is_symlink_refusal(path, &err) => LoadedConfig {
            config: AppConfig::default(),
            startup_notice: Some(ConfigStartupNotice::Symlinked),
            pending_repair: None,
        },
        Err(err) => LoadedConfig {
            config: AppConfig::default(),
            startup_notice: Some(ConfigStartupNotice::ReadFailed(err.to_string())),
            pending_repair: None,
        },
    }
}

pub fn read_startup_config() -> LoadedConfig {
    read_startup_config_from_path(&xdg::config_path())
}

/// Read the config once at startup and move an unparsable file aside.
///
/// This runs before the GTK loop exists, so no config save can be queued yet
/// and the rename cannot race one. The migration write stays pending: the GTK
/// side performs it through the ordered config lane, where a failure blocks
/// Close instead of vanishing.
fn read_startup_config_from_path(path: &Path) -> LoadedConfig {
    let mut loaded = read_config_from_path(path);
    if loaded.pending_repair != Some(PendingRepair::PreserveUnparsable) {
        return loaded;
    }
    loaded.pending_repair = None;
    if let Some(ConfigStartupNotice::ParseFailed { backup, .. }) = &mut loaded.startup_notice {
        *backup = preserve_unparsable_config(path);
    }
    loaded
}

fn load_supported_config(raw: &str) -> LoadedConfig {
    match serde_json::from_str::<AppConfig>(raw) {
        Ok(mut config) => {
            let migrated = apply_migrations(&mut config);
            LoadedConfig {
                config,
                startup_notice: None,
                pending_repair: migrated.then_some(PendingRepair::WriteMigrated),
            }
        }
        Err(err) => LoadedConfig {
            config: AppConfig::default(),
            startup_notice: Some(ConfigStartupNotice::ParseFailed {
                detail: err.to_string(),
                backup: None,
            }),
            pending_repair: Some(PendingRepair::PreserveUnparsable),
        },
    }
}

/// Use a config from a newer build as far as it parses, and leave the file
/// alone.
///
/// Migrating it, or moving it aside when it does not parse, would cost the
/// fields this build cannot see; `write_config_to_path` refuses it for the
/// same reason. An unparsable newer file still loads defaults, so the
/// controller refuses it like any other unparsable config.
fn load_newer_config(raw: &str, version: u32) -> LoadedConfig {
    tracing::warn!(
        version,
        supported = CURRENT_VERSION,
        "config is newer than this build supports; reading it without saving"
    );
    match serde_json::from_str::<AppConfig>(raw) {
        Ok(config) => LoadedConfig {
            config,
            startup_notice: Some(ConfigStartupNotice::NewerVersion { version }),
            pending_repair: None,
        },
        Err(err) => LoadedConfig {
            config: AppConfig::default(),
            startup_notice: Some(ConfigStartupNotice::ParseFailed {
                detail: err.to_string(),
                backup: None,
            }),
            pending_repair: None,
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
///
/// Only configs at or below `CURRENT_VERSION` reach this: a newer one is
/// read by `load_newer_config`, which never migrates.
fn apply_migrations(config: &mut AppConfig) -> bool {
    let version_changed = migrate(config);
    let custom_css_changed = config.appearance.migrate_legacy_custom_css_path();
    let connections_changed = config.normalize_connections();
    version_changed || custom_css_changed || connections_changed
}

/// Upgrade an older config in place.  Returns `true` if anything changed
/// (so the caller can re-persist).  Each arm handles one version step.
fn migrate(config: &mut AppConfig) -> bool {
    let mut changed = false;
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
            3 => {
                config.legacy_registry_endpoint =
                    Some(format!("{}:{}", config.obs.host, config.obs.port));
                config.version = 4;
                changed = true;
            }
            4 => {
                config.version = 5;
                changed = true;
            }
            _ => {
                config.version = CURRENT_VERSION;
                changed = true;
            }
        }
    }
    changed
}

pub fn write_config(config: &AppConfig) -> io::Result<()> {
    write_config_to_path(&xdg::config_path(), config)
}

/// Replace the config at `path` with `config`.
///
/// Refused with `io::ErrorKind::Unsupported` when either side comes from a
/// newer build: this build would write back only the fields it knows, and
/// with `io::ErrorKind::ResourceBusy` while another process saves it.
pub fn write_config_to_path(path: &Path, config: &AppConfig) -> io::Result<()> {
    if config.version > CURRENT_VERSION {
        return Err(schema_version::refuse_newer(
            path,
            config.version,
            CURRENT_VERSION,
        ));
    }
    let _lock = crate::storage::lock::lock_store(path)?;
    ensure_replaceable(path)?;
    // Materialize profiles at the ordered write boundary. Reconnecting must not
    // discover an empty list and trigger a migration write outside the UI lane.
    let mut persisted = config.clone();
    persisted.normalize_connections();
    let raw = serde_json::to_string_pretty(&persisted).map_err(io::Error::other)?;
    crate::storage::small_file::check_size(
        raw.len() as u64,
        crate::storage::small_file::CONFIG_LIMIT,
    )?;
    // Whole-file replacement: a half-written file would be unparsable, and the
    // loaders treat unparsable as absent.
    crate::storage::atomic::write(path, raw.as_bytes())
}

/// Fail unless the file at `path` is absent or a config this build can
/// rewrite without loss.
///
/// If preserving an invalid startup file failed (or reading was refused), a
/// defaults-based save must not silently destroy the original either.
fn ensure_replaceable(path: &Path) -> io::Result<()> {
    match crate::storage::small_file::read_to_string(path, crate::storage::small_file::CONFIG_LIMIT)
    {
        Ok(raw) => {
            if let Some(version) = schema_version::newer_version(&raw, CURRENT_VERSION) {
                return Err(schema_version::refuse_newer(path, version, CURRENT_VERSION));
            }
            serde_json::from_str::<AppConfig>(&raw).map_err(io::Error::other)?;
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
