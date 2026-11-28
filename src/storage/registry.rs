//! Scene role registry — the local metadata layer that augments the raw OBS
//! scene list with roles, tags, and protection flags.

mod decks;
mod identity;
mod import;

pub(crate) use decks::SceneFilter;
pub(crate) use identity::RetainedScene;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::registry::{GraphDependencyPolicy, SceneMetadata, SceneRegistrySnapshot};
use crate::domain::role::SceneRole;
use crate::storage::{schema_version, xdg};

/// Schema version of `registry.json`. A file from before the field existed
/// has no `version` and reads as this version.
const REGISTRY_FILE_VERSION: u32 = 1;

/// Root `registry.json` payload.
///
/// The file adds `version` (see `RegistryFile`); show-workspace entries and
/// YAML exports serialize this type as it is.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRegistry {
    /// Per-scene metadata keyed by OBS scene name.
    #[serde(default)]
    pub scenes: HashMap<String, SceneEntry>,
    /// Last confirmed UUID for each named scene in this workspace.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) scene_uuids: HashMap<String, String>,
    /// Curation withheld after name reuse or an ambiguous rename.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) retained_scenes: Vec<RetainedScene>,
    /// User-defined display order for Inventory and Live scene cards.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_order: Vec<String>,
    /// Current view shared by Inventory and Live in this workspace.
    #[serde(default)]
    pub(crate) scene_filter: SceneFilter,
    /// Named views kept separately for every workspace.
    #[serde(
        default,
        deserialize_with = "decks::deserialize_decks",
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    pub(crate) decks: std::collections::BTreeMap<String, SceneFilter>,
    /// Explicit actions keep their bindings when scene cards are reordered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) action_bindings: Vec<crate::domain::action_shortcut::ActionBinding>,
    /// Ordered Live controls, independent of scene order and keyboard bindings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) quick_actions: Vec<crate::domain::operator_action::OperatorPin>,
    /// Audio expectations are opt-in and scoped to this production's inputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) audio_alerts: Vec<crate::domain::health::AudioRule>,
    /// Named gain/mute snapshots belonging only to this connection/collection.
    #[serde(
        default,
        deserialize_with = "crate::domain::audio_preset::deserialize_presets",
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    pub(crate) audio_presets:
        std::collections::BTreeMap<String, crate::domain::audio_preset::AudioPreset>,
    /// Per-audio-input metadata keyed by OBS input name.
    ///
    /// Audio inputs have no roles or ordering, only presentation, so they get
    /// their own map rather than being squeezed into `scenes`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub inputs: HashMap<String, InputEntry>,
    /// Graph validation rules stored as role keys for human-editable files.
    #[serde(default)]
    pub rules: RuleConfig,
}

/// `registry.json` as written: the schema version, then the registry.
#[derive(Serialize)]
struct RegistryFile<'a> {
    version: u32,
    #[serde(flatten)]
    registry: &'a SceneRegistry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SceneRenameResult {
    Unchanged,
    Renamed,
    Conflict,
}

/// Serialized metadata for one OBS scene.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneEntry {
    /// Local classification used by Live, Graph, and Doctor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<SceneRole>,
    /// Free-form labels used to organize and find scenes.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether the operator has marked this scene as a favorite.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub favorite: bool,
    /// Protected scenes are expected to be building blocks, not live switches.
    #[serde(default)]
    pub protected: bool,
    /// Optional scene-card accent stored as an opaque RGB hex value. Alpha is
    /// intentionally not persisted; Live always renders it at 50%.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    /// Optional icon key from `domain::icon`. Unknown keys are ignored when
    /// rendering, so a registry from a newer build stays readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Serialized metadata for one OBS audio input.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputEntry {
    /// Optional icon key from `domain::icon`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Graph dependency rules evaluated by the Doctor.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleConfig {
    /// Role keys that Primary scenes may depend on without warning.
    #[serde(default)]
    pub primary_can_depend_on: Vec<String>,
    /// Role keys that Module scenes may depend on without warning.
    #[serde(default)]
    pub module_can_depend_on: Vec<String>,
    /// Pairs `[from_role, to_role]` that are unconditionally forbidden.
    #[serde(default)]
    pub forbidden_edges: Vec<[String; 2]>,
}

/// Typed registry load failures.
#[derive(Debug, Error)]
pub enum RegistryStorageError {
    /// The registry file could not be read.
    #[error("failed to read scene registry at {path}: {source}")]
    Read {
        /// Path that failed to load.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: io::Error,
    },
    /// The registry file was readable but invalid JSON.
    #[error("failed to parse scene registry at {path}: {source}")]
    Parse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying JSON parse failure.
        #[source]
        source: serde_json::Error,
    },
}

impl SceneRegistry {
    /// Follow an observed OBS rename without replacing another scene's metadata.
    /// Repeated delivery is harmless; the old card's position survives even if
    /// the destination name has a stale order entry.
    pub(crate) fn rename_scene(
        &mut self,
        old: &str,
        new: &str,
        uuid: Option<&str>,
    ) -> SceneRenameResult {
        if old == new || old.is_empty() || new.is_empty() {
            return SceneRenameResult::Unchanged;
        }
        if uuid.is_some_and(|uuid| self.scene_uuids.get(old).is_some_and(|known| known != uuid)) {
            return SceneRenameResult::Unchanged;
        }
        if self.scenes.contains_key(old) && self.scenes.contains_key(new) {
            return SceneRenameResult::Conflict;
        }
        let identity = self.scene_uuids.remove(old);
        let metadata = self.scenes.remove(old);
        let ordered = self.scene_order.iter().any(|name| name == old);
        let changed = metadata.is_some() || ordered || identity.is_some();
        if changed {
            if let Some(uuid) = uuid.map(str::to_owned).or(identity) {
                self.scene_uuids.insert(new.to_owned(), uuid);
            }
        }
        if let Some(metadata) = metadata {
            self.scenes.insert(new.to_owned(), metadata);
        }
        if ordered {
            self.scene_order.retain(|name| name != new);
            for name in &mut self.scene_order {
                if name == old {
                    *name = new.to_owned();
                }
            }
            self.set_scene_order(self.scene_order.clone());
        }
        if changed {
            SceneRenameResult::Renamed
        } else {
            SceneRenameResult::Unchanged
        }
    }

    /// Return known scene IDs in persisted order, appending IDs not yet stored.
    pub fn ordered_scene_ids<'a>(
        &self,
        scene_ids: impl IntoIterator<Item = &'a str>,
    ) -> Vec<String> {
        let available: Vec<&str> = scene_ids.into_iter().collect();
        let available_set: std::collections::HashSet<&str> = available.iter().copied().collect();
        let mut seen = std::collections::HashSet::new();
        let mut ordered = Vec::with_capacity(available.len());

        for scene_id in &self.scene_order {
            if available_set.contains(scene_id.as_str()) && seen.insert(scene_id.as_str()) {
                ordered.push(scene_id.clone());
            }
        }
        for scene_id in available {
            if seen.insert(scene_id) {
                ordered.push(scene_id.to_string());
            }
        }
        ordered
    }

    /// Replace the persisted display order after removing duplicate IDs.
    pub fn set_scene_order(&mut self, scene_ids: impl IntoIterator<Item = String>) -> bool {
        let mut seen = std::collections::HashSet::new();
        let order: Vec<String> = scene_ids
            .into_iter()
            .filter(|scene_id| seen.insert(scene_id.clone()))
            .collect();
        if self.scene_order == order {
            return false;
        }
        self.scene_order = order;
        true
    }

    /// Convert serialized registry data into a domain-facing immutable snapshot.
    pub fn snapshot(&self) -> SceneRegistrySnapshot {
        let scenes = self
            .scenes
            .iter()
            .filter_map(|(scene_id, entry)| {
                let role = entry.role?;
                Some((
                    scene_id.clone(),
                    SceneMetadata {
                        role,
                        tags: entry.tags.clone(),
                        protected: entry.protected,
                    },
                ))
            })
            .collect();

        SceneRegistrySnapshot::new(scenes, self.rules.policy())
    }

    /// Set or clear the role for one scene while preserving existing metadata.
    ///
    /// Assigning a role creates default metadata when the scene is not already
    /// registered. Passing `None` removes the scene entry entirely. Returns
    /// `true` when the registry changed.
    pub fn set_scene_role(&mut self, scene_id: &str, role: Option<SceneRole>) -> bool {
        match role {
            Some(role) => match self.scenes.get_mut(scene_id) {
                Some(entry) if entry.role == Some(role) => false,
                Some(entry) => {
                    entry.role = Some(role);
                    true
                }
                None => {
                    self.scenes.insert(
                        scene_id.to_string(),
                        SceneEntry {
                            role: Some(role),
                            ..SceneEntry::default()
                        },
                    );
                    true
                }
            },
            None => {
                let Some(entry) = self.scenes.get_mut(scene_id) else {
                    return false;
                };
                if entry.role.is_none() {
                    return false;
                }
                entry.role = None;
                if entry.is_empty() {
                    self.scenes.remove(scene_id);
                }
                true
            }
        }
    }
}

impl SceneRegistry {
    /// Set or clear one scene's icon, preserving the rest of its metadata.
    ///
    /// Returns `true` when the registry changed. Clearing the last piece of
    /// metadata removes the entry, so an untouched scene leaves no trace.
    pub fn set_scene_icon(&mut self, scene_id: &str, icon: Option<&str>) -> bool {
        let icon = icon.map(str::to_string);
        match self.scenes.get_mut(scene_id) {
            Some(entry) if entry.icon == icon => false,
            Some(entry) => {
                entry.icon = icon;
                if entry.is_empty() {
                    self.scenes.remove(scene_id);
                }
                true
            }
            None => {
                let Some(icon) = icon else {
                    return false;
                };
                self.scenes.insert(
                    scene_id.to_string(),
                    SceneEntry {
                        icon: Some(icon),
                        ..SceneEntry::default()
                    },
                );
                true
            }
        }
    }

    /// Set or clear one audio input's icon.
    pub fn set_input_icon(&mut self, input_id: &str, icon: Option<&str>) -> bool {
        let icon = icon.map(str::to_string);
        match self.inputs.get_mut(input_id) {
            Some(entry) if entry.icon == icon => false,
            Some(entry) => {
                entry.icon = icon;
                if entry.icon.is_none() {
                    self.inputs.remove(input_id);
                }
                true
            }
            None => {
                let Some(icon) = icon else {
                    return false;
                };
                self.inputs
                    .insert(input_id.to_string(), InputEntry { icon: Some(icon) });
                true
            }
        }
    }

    /// Icon key assigned to a scene, if it has one.
    pub fn scene_icon(&self, scene_id: &str) -> Option<&str> {
        self.scenes.get(scene_id)?.icon.as_deref()
    }

    /// Icon key assigned to an audio input, if it has one.
    pub fn input_icon(&self, input_id: &str) -> Option<&str> {
        self.inputs.get(input_id)?.icon.as_deref()
    }
}

impl SceneEntry {
    /// Whether the entry carries no metadata worth keeping.
    ///
    /// This is the only place the rule is written down. Two call sites used to
    /// re-derive it and both forgot `icon`, which silently dropped a scene's
    /// icon whenever its role or accent was cleared.
    pub(crate) fn is_empty(&self) -> bool {
        self.role.is_none()
            && self.tags.is_empty()
            && !self.favorite
            && !self.protected
            && self.accent_color.is_none()
            && self.icon.is_none()
    }
}

impl SceneRegistry {
    /// Trim tags and keep the first spelling of each case-insensitive label.
    /// Clearing tags preserves every other scene field.
    pub(crate) fn set_scene_tags(
        &mut self,
        scene: &str,
        tags: impl IntoIterator<Item = String>,
    ) -> bool {
        let mut seen = std::collections::HashSet::new();
        let tags: Vec<_> = tags
            .into_iter()
            .map(|tag| tag.trim().to_owned())
            .filter(|tag| !tag.is_empty() && seen.insert(tag.to_lowercase()))
            .collect();
        if self
            .scenes
            .get(scene)
            .map(|entry| entry.tags.as_slice())
            .unwrap_or_default()
            == tags
        {
            return false;
        }
        let entry = self.scenes.entry(scene.to_owned()).or_default();
        entry.tags = tags;
        if entry.is_empty() {
            self.scenes.remove(scene);
        }
        true
    }

    /// Mark or unmark a scene without removing its other curation.
    pub(crate) fn set_scene_favorite(&mut self, scene: &str, favorite: bool) -> bool {
        if self.scenes.get(scene).is_some_and(|entry| entry.favorite) == favorite {
            return false;
        }
        let entry = self.scenes.entry(scene.to_owned()).or_default();
        entry.favorite = favorite;
        if entry.is_empty() {
            self.scenes.remove(scene);
        }
        true
    }
}

impl SceneRegistry {
    /// Set or clear one scene's accent colour, preserving the rest of its
    /// metadata.
    ///
    /// Shaped exactly like [`SceneRegistry::set_scene_icon`], and for the same
    /// reason: which fields make an entry worth keeping is a registry rule, not
    /// something a page should decide while handling a colour-picker signal.
    ///
    /// Returns `true` when the registry changed.
    pub fn set_scene_accent(&mut self, scene_id: &str, accent_color: Option<String>) -> bool {
        match self.scenes.get_mut(scene_id) {
            Some(entry) if entry.accent_color == accent_color => false,
            Some(entry) => {
                entry.accent_color = accent_color;
                if entry.is_empty() {
                    self.scenes.remove(scene_id);
                }
                true
            }
            None => {
                // Clearing an accent on a scene with no entry changes nothing,
                // and must not create one.
                let Some(accent_color) = accent_color else {
                    return false;
                };
                self.scenes.insert(
                    scene_id.to_string(),
                    SceneEntry {
                        accent_color: Some(accent_color),
                        ..SceneEntry::default()
                    },
                );
                true
            }
        }
    }
}

/// Normalize a GTK color selection to the persisted `#RRGGBB` form.
pub fn scene_accent_hex(red: f32, green: f32, blue: f32) -> String {
    fn channel(value: f32) -> u8 {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(red),
        channel(green),
        channel(blue)
    )
}

/// Parse persisted scene accents. Invalid hand-edited values are ignored.
pub fn parse_scene_accent(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some((
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}

impl RuleConfig {
    /// Convert string role keys into a typed graph policy.
    ///
    /// Empty allow-lists keep SceneDeck's built-in architecture defaults.
    /// Non-empty allow-lists replace the matching default list. Forbidden edges
    /// are added to the built-in forbidden pairs. Unknown role keys are ignored
    /// and logged so hand-edited registries remain recoverable.
    pub fn policy(&self) -> GraphDependencyPolicy {
        let mut policy = GraphDependencyPolicy::default();

        let primary_can_depend_on = parse_role_list(&self.primary_can_depend_on);
        if !primary_can_depend_on.is_empty() {
            policy = policy.primary_can_depend_on(primary_can_depend_on);
        }

        let module_can_depend_on = parse_role_list(&self.module_can_depend_on);
        if !module_can_depend_on.is_empty() {
            policy = policy.module_can_depend_on(module_can_depend_on);
        }

        for [from_key, to_key] in &self.forbidden_edges {
            let from = SceneRole::from_rule_key(from_key);
            let to = SceneRole::from_rule_key(to_key);
            match (from, to) {
                (Some(from), Some(to)) => {
                    policy = policy.forbid(from, to);
                }
                _ => {
                    tracing::warn!(
                        from = from_key.as_str(),
                        to = to_key.as_str(),
                        "ignoring registry rule with unknown scene role"
                    );
                }
            }
        }

        policy
    }
}

fn parse_role_list(keys: &[String]) -> Vec<SceneRole> {
    keys.iter()
        .filter_map(|key| match SceneRole::from_rule_key(key) {
            Some(role) => Some(role),
            None => {
                tracing::warn!(key, "ignoring registry rule with unknown scene role");
                None
            }
        })
        .collect()
}

/// Read registry from the XDG config directory.
///
/// A missing file reads as defaults. An unreadable one is an error: silently
/// replacing hand-edited curation with defaults would look like lost work.
pub fn read_registry() -> Result<SceneRegistry, RegistryStorageError> {
    read_registry_from_path(&xdg::registry_path())
}

/// Read registry JSON from an explicit path.
///
/// A file from a newer build reads as far as this build understands it;
/// `write_registry_to_path` refuses to replace it.
pub fn read_registry_from_path(path: &Path) -> Result<SceneRegistry, RegistryStorageError> {
    match read_registry_text(path)? {
        Some(raw) => parse_registry(path, &raw),
        None => Ok(SceneRegistry::default()),
    }
}

/// The text of the registry file, or `None` when there is no file yet.
fn read_registry_text(path: &Path) -> Result<Option<String>, RegistryStorageError> {
    match crate::storage::small_file::read_to_string(
        path,
        crate::storage::small_file::REGISTRY_LIMIT,
    ) {
        Ok(raw) => Ok(Some(raw)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(RegistryStorageError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn parse_registry(path: &Path, raw: &str) -> Result<SceneRegistry, RegistryStorageError> {
    serde_json::from_str(raw).map_err(|source| RegistryStorageError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

/// Pretty-print registry JSON to an explicit path, creating parents as needed.
///
/// Refused with `io::ErrorKind::Unsupported` over a file from a newer build:
/// this build would write back only the fields it knows, and with
/// `io::ErrorKind::ResourceBusy` while another process saves it.
pub fn write_registry_to_path(path: &Path, registry: &SceneRegistry) -> io::Result<()> {
    let _lock = crate::storage::lock::lock_store(path)?;
    ensure_replaceable(path)?;
    let file = RegistryFile {
        version: REGISTRY_FILE_VERSION,
        registry,
    };
    let raw = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
    crate::storage::small_file::check_size(
        raw.len() as u64,
        crate::storage::small_file::REGISTRY_LIMIT,
    )?;
    // Whole-file replacement: a half-written file would be unparsable, and the
    // loaders treat unparsable as absent.
    crate::storage::atomic::write(path, raw.as_bytes())
}

/// Fail unless the file at `path` is absent or a registry this build can
/// rewrite without loss.
///
/// Every JSON write, including imports and whole-cache snapshots, refuses to
/// destroy an unreadable original. Missing files still initialize normally.
fn ensure_replaceable(path: &Path) -> io::Result<()> {
    let Some(raw) = read_registry_text(path).map_err(io::Error::other)? else {
        return Ok(());
    };
    if let Some(version) = schema_version::newer_version(&raw, REGISTRY_FILE_VERSION) {
        return Err(schema_version::refuse_newer(
            path,
            version,
            REGISTRY_FILE_VERSION,
        ));
    }
    parse_registry(path, &raw).map_err(io::Error::other)?;
    Ok(())
}

/// Read a YAML registry export from an explicit path.
pub fn read_registry_yaml_from_path(path: &Path) -> io::Result<SceneRegistry> {
    let raw =
        crate::storage::small_file::read_to_string(path, crate::storage::small_file::YAML_LIMIT)?;
    let mut registry = import::parse_import(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    // YAML is portable named intent. Identities from another connection must
    // never redirect it or revive detached conflicts in this workspace.
    registry.scene_uuids.clear();
    for retained in &mut registry.retained_scenes {
        retained.uuid = None;
    }
    // Keep stable identities as evidence, but never activate imported key
    // bindings automatically in another production. Users review/rebind them.
    for binding in &mut registry.action_bindings {
        binding.enabled = false;
    }
    for pin in &mut registry.quick_actions {
        pin.enabled = false;
    }
    for rule in &mut registry.audio_alerts {
        rule.enabled = false;
    }
    Ok(registry)
}

/// Write a YAML registry export to an explicit path, creating parents as needed.
pub fn write_registry_yaml_to_path(path: &Path, registry: &SceneRegistry) -> io::Result<()> {
    let raw = serde_saphyr::to_string(registry)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    crate::storage::small_file::check_size(
        raw.len() as u64,
        crate::storage::small_file::YAML_LIMIT,
    )?;
    // An export is usually someone's backup; a half-written one is worse than
    // no backup, because it looks like a backup.
    crate::storage::atomic::write(path, raw.as_bytes())
}

#[cfg(test)]
mod tests;
