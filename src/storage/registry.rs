//! Scene role registry — the local metadata layer that augments the raw OBS
//! scene list with roles, tags, and protection flags.

use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::registry::{GraphDependencyPolicy, SceneMetadata, SceneRegistrySnapshot};
use crate::domain::role::SceneRole;
use crate::storage::xdg;

/// Root `registry.json` payload.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SceneRegistry {
    /// Per-scene metadata keyed by OBS scene name.
    #[serde(default)]
    pub scenes: HashMap<String, SceneEntry>,
    /// User-defined display order for Inventory and Live scene cards.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_order: Vec<String>,
    /// Graph validation rules stored as role keys for human-editable files.
    #[serde(default)]
    pub rules: RuleConfig,
}

/// Serialized metadata for one OBS scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntry {
    /// Local classification used by Live, Graph, and Doctor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<SceneRole>,
    /// Free-form labels reserved for user workflows and future filters.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Protected scenes are expected to be building blocks, not live switches.
    #[serde(default)]
    pub protected: bool,
    /// Optional scene-card accent stored as an opaque RGB hex value. Alpha is
    /// intentionally not persisted; Live always renders it at 50%.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
}

/// Graph dependency rules evaluated by the Doctor.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

/// Typed failures while mutating the persisted registry.
#[derive(Debug, Error)]
pub enum RegistryMutationError {
    /// The existing registry could not be loaded safely.
    #[error(transparent)]
    Load(#[from] RegistryStorageError),
    /// The updated registry could not be written.
    #[error("failed to write scene registry: {source}")]
    Write {
        /// Underlying I/O failure.
        #[source]
        source: io::Error,
    },
}

impl SceneRegistry {
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
                            tags: Vec::new(),
                            protected: false,
                            accent_color: None,
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
                if entry.accent_color.is_none() && entry.tags.is_empty() && !entry.protected {
                    self.scenes.remove(scene_id);
                }
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
/// Missing or invalid registries fall back to defaults and log a warning.
pub fn read_registry() -> SceneRegistry {
    match try_read_registry() {
        Ok(registry) => registry,
        Err(err) => {
            tracing::warn!(%err, "failed to load registry, using defaults");
            SceneRegistry::default()
        }
    }
}

/// Read registry from the XDG config directory without falling back to defaults.
///
/// Use this for explicit user actions, such as export, where silently replacing
/// an invalid hand-edited registry with defaults would be misleading.
pub fn try_read_registry() -> Result<SceneRegistry, RegistryStorageError> {
    read_registry_from_path(&xdg::config_dir().join("registry.json"))
}

/// Read registry JSON from an explicit path.
pub fn read_registry_from_path(path: &Path) -> Result<SceneRegistry, RegistryStorageError> {
    match read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|source| RegistryStorageError::Parse {
            path: path.to_path_buf(),
            source,
        }),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(SceneRegistry::default()),
        Err(source) => Err(RegistryStorageError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Write registry JSON to the XDG config directory.
pub fn write_registry(registry: &SceneRegistry) -> io::Result<()> {
    write_registry_to_path(&xdg::config_dir().join("registry.json"), registry)
}

/// Set or clear one scene role in the registry stored in the XDG config
/// directory.
///
/// Returns `true` when the registry changed. Load failures are returned instead
/// of being coerced to an empty registry, which avoids overwriting a hand-edited
/// but temporarily invalid file during UI role assignment.
pub fn set_scene_role(
    scene_id: &str,
    role: Option<SceneRole>,
) -> Result<bool, RegistryMutationError> {
    set_scene_role_from_path(&xdg::config_dir().join("registry.json"), scene_id, role)
}

fn set_scene_role_from_path(
    path: &Path,
    scene_id: &str,
    role: Option<SceneRole>,
) -> Result<bool, RegistryMutationError> {
    let mut registry = read_registry_from_path(path)?;
    let changed = registry.set_scene_role(scene_id, role);
    if changed {
        write_registry_to_path(path, &registry)
            .map_err(|source| RegistryMutationError::Write { source })?;
    }
    Ok(changed)
}

/// Remove one scene entry from the registry stored in the XDG config directory.
///
/// Returns `true` when an entry existed and was removed. Load failures are
/// returned instead of being coerced to an empty registry, which avoids
/// overwriting a hand-edited but temporarily invalid file during UI cleanup.
pub fn remove_scene_entry(scene_id: &str) -> Result<bool, RegistryMutationError> {
    remove_scene_entry_from_path(&xdg::config_dir().join("registry.json"), scene_id)
}

fn remove_scene_entry_from_path(
    path: &Path,
    scene_id: &str,
) -> Result<bool, RegistryMutationError> {
    let mut registry = read_registry_from_path(path)?;
    let removed = registry.scenes.remove(scene_id).is_some();
    if removed {
        write_registry_to_path(path, &registry)
            .map_err(|source| RegistryMutationError::Write { source })?;
    }
    Ok(removed)
}

/// Pretty-print registry JSON to an explicit path, creating parents as needed.
pub fn write_registry_to_path(path: &Path, registry: &SceneRegistry) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }
    let raw = serde_json::to_string_pretty(registry).map_err(io::Error::other)?;
    write(path, raw)
}

/// Read a YAML registry export from an explicit path.
pub fn read_registry_yaml_from_path(path: &Path) -> io::Result<SceneRegistry> {
    let raw = read_to_string(path)?;
    serde_yaml::from_str(&raw).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

/// Write a YAML registry export to an explicit path, creating parents as needed.
pub fn write_registry_yaml_to_path(path: &Path, registry: &SceneRegistry) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }
    let raw = serde_yaml::to_string(registry)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    write(path, raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::registry::RoleDependency;

    fn unique_temp_path(name: &str, extension: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "scenedeck-{name}-{}-{nanos}.{extension}",
            std::process::id(),
        ))
    }

    #[test]
    fn yaml_registry_round_trips() {
        let path = unique_temp_path("registry-round-trip", "yaml");
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".into(),
            SceneEntry {
                role: Some(SceneRole::Primary),
                tags: vec!["live".into()],
                protected: true,
                accent_color: Some("#336699".into()),
            },
        );
        registry.scene_order = vec!["Main".into(), "Holding".into()];
        registry
            .rules
            .forbidden_edges
            .push(["primary".into(), "debug".into()]);

        write_registry_yaml_to_path(&path, &registry).unwrap();
        let parsed = read_registry_yaml_from_path(&path).unwrap();
        assert_eq!(
            parsed
                .scenes
                .get("Main")
                .and_then(|entry| entry.accent_color.as_deref()),
            Some("#336699")
        );
        let _ = std::fs::remove_file(path);

        let main = parsed.scenes.get("Main").unwrap();
        assert_eq!(main.role, Some(SceneRole::Primary));
        assert_eq!(main.tags, vec!["live"]);
        assert!(main.protected);
        assert_eq!(
            parsed.rules.forbidden_edges,
            vec![["primary".to_string(), "debug".to_string()]]
        );
        assert_eq!(parsed.scene_order, ["Main", "Holding"]);
    }

    #[test]
    fn ordered_scene_ids_uses_saved_order_and_appends_new_scenes() {
        let registry = SceneRegistry {
            scene_order: vec!["Three".into(), "Missing".into(), "One".into()],
            ..SceneRegistry::default()
        };

        assert_eq!(
            registry.ordered_scene_ids(["One", "Two", "Three"]),
            ["Three", "One", "Two"]
        );
    }

    #[test]
    fn set_scene_order_removes_duplicates_and_reports_changes() {
        let mut registry = SceneRegistry::default();

        assert!(registry.set_scene_order(["Two", "One", "Two"].into_iter().map(str::to_string)));
        assert_eq!(registry.scene_order, ["Two", "One"]);
        assert!(!registry.set_scene_order(["Two", "One"].into_iter().map(str::to_string)));
    }

    #[test]
    fn scene_accent_normalizes_rgb_and_ignores_alpha() {
        assert_eq!(scene_accent_hex(0.2, 0.4, 0.6), "#336699");
        assert_eq!(scene_accent_hex(-1.0, 2.0, 0.5), "#00FF80");
    }

    #[test]
    fn scene_accent_parser_accepts_rgb_hex_and_rejects_other_shapes() {
        assert_eq!(parse_scene_accent("#336699"), Some((51, 102, 153)));
        assert_eq!(parse_scene_accent("336699"), Some((51, 102, 153)));
        assert_eq!(parse_scene_accent("#33669980"), None);
        assert_eq!(parse_scene_accent("not-a-color"), None);
    }

    #[test]
    fn invalid_yaml_returns_invalid_data() {
        let path = unique_temp_path("registry-invalid", "yaml");
        std::fs::write(&path, "scenes: [").unwrap();

        let err = read_registry_yaml_from_path(&path).unwrap_err();
        let _ = std::fs::remove_file(path);

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn invalid_json_registry_returns_typed_parse_error() {
        let path = unique_temp_path("registry-invalid-json", "json");
        std::fs::write(&path, "{").unwrap();

        let err = read_registry_from_path(&path).unwrap_err();
        let _ = std::fs::remove_file(path);

        assert!(matches!(err, RegistryStorageError::Parse { .. }));
    }

    #[test]
    fn scene_registry_set_role_adds_default_metadata_for_new_role() {
        let mut registry = SceneRegistry::default();

        assert!(registry.set_scene_role("Main", Some(SceneRole::Primary)));

        let entry = registry.scenes.get("Main").unwrap();
        assert_eq!(entry.role, Some(SceneRole::Primary));
        assert!(entry.tags.is_empty());
        assert!(!entry.protected);
    }

    #[test]
    fn scene_registry_set_role_updates_role_without_losing_metadata() {
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".to_string(),
            SceneEntry {
                role: Some(SceneRole::Module),
                tags: vec!["camera".to_string()],
                protected: true,
                accent_color: None,
            },
        );

        assert!(registry.set_scene_role("Main", Some(SceneRole::Secondary)));

        let entry = registry.scenes.get("Main").unwrap();
        assert_eq!(entry.role, Some(SceneRole::Secondary));
        assert_eq!(entry.tags, ["camera"]);
        assert!(entry.protected);
    }

    #[test]
    fn scene_registry_set_role_removes_entry_for_unassigned_role() {
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".to_string(),
            SceneEntry {
                role: Some(SceneRole::Primary),
                tags: Vec::new(),
                protected: false,
                accent_color: None,
            },
        );

        assert!(registry.set_scene_role("Main", None));

        assert!(!registry.scenes.contains_key("Main"));
    }

    #[test]
    fn scene_registry_set_role_preserves_accent_for_unassigned_scene() {
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".to_string(),
            SceneEntry {
                role: Some(SceneRole::Primary),
                tags: Vec::new(),
                protected: false,
                accent_color: Some("#336699".to_string()),
            },
        );

        registry.set_scene_role("Main", None);

        let entry = registry.scenes.get("Main").unwrap();
        assert_eq!(entry.role, None);
        assert_eq!(entry.accent_color.as_deref(), Some("#336699"));
    }

    #[test]
    fn set_scene_role_from_path_preserves_existing_metadata() {
        let path = unique_temp_path("registry-set-role", "json");
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".into(),
            SceneEntry {
                role: Some(SceneRole::Module),
                tags: vec!["camera".into()],
                protected: true,
                accent_color: None,
            },
        );
        write_registry_to_path(&path, &registry).unwrap();

        let changed = set_scene_role_from_path(&path, "Main", Some(SceneRole::Secondary)).unwrap();
        let parsed = read_registry_from_path(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert!(changed);
        let entry = &parsed.scenes["Main"];
        assert_eq!(entry.role, Some(SceneRole::Secondary));
        assert_eq!(entry.tags, ["camera"]);
        assert!(entry.protected);
    }

    #[test]
    fn set_scene_role_from_path_does_not_overwrite_invalid_registry() {
        let path = unique_temp_path("registry-set-role-invalid", "json");
        std::fs::write(&path, "{").unwrap();

        let err = set_scene_role_from_path(&path, "Main", Some(SceneRole::Primary)).unwrap_err();
        let raw = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert!(matches!(
            err,
            RegistryMutationError::Load(RegistryStorageError::Parse { .. })
        ));
        assert_eq!(raw, "{");
    }

    #[test]
    fn remove_scene_entry_removes_only_requested_scene() {
        let path = unique_temp_path("registry-remove-entry", "json");
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".into(),
            SceneEntry {
                role: Some(SceneRole::Primary),
                tags: vec!["live".into()],
                protected: false,
                accent_color: None,
            },
        );
        registry.scenes.insert(
            "Camera".into(),
            SceneEntry {
                role: Some(SceneRole::Raw),
                tags: vec!["source".into()],
                protected: true,
                accent_color: None,
            },
        );
        write_registry_to_path(&path, &registry).unwrap();

        let removed = remove_scene_entry_from_path(&path, "Main").unwrap();
        let parsed = read_registry_from_path(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert!(removed);
        assert!(!parsed.scenes.contains_key("Main"));
        assert_eq!(parsed.scenes["Camera"].role, Some(SceneRole::Raw));
        assert!(parsed.scenes["Camera"].protected);
    }

    #[test]
    fn remove_scene_entry_does_not_overwrite_invalid_registry() {
        let path = unique_temp_path("registry-remove-invalid", "json");
        std::fs::write(&path, "{").unwrap();

        let err = remove_scene_entry_from_path(&path, "Main").unwrap_err();
        let raw = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert!(matches!(
            err,
            RegistryMutationError::Load(RegistryStorageError::Parse { .. })
        ));
        assert_eq!(raw, "{");
    }

    #[test]
    fn registry_snapshot_converts_string_rules_to_typed_policy() {
        let mut registry = SceneRegistry::default();
        registry
            .rules
            .primary_can_depend_on
            .push("module".to_string());
        registry
            .rules
            .forbidden_edges
            .push(["primary".to_string(), "debug".to_string()]);

        let snapshot = registry.snapshot();

        assert_eq!(
            snapshot.graph_policy().primary_can_depend_on,
            vec![SceneRole::Module]
        );
        assert!(snapshot
            .graph_policy()
            .forbidden_edges
            .contains(&RoleDependency::new(SceneRole::Primary, SceneRole::Debug)));
        assert!(snapshot
            .graph_policy()
            .forbidden_edges
            .contains(&RoleDependency::new(SceneRole::Raw, SceneRole::Module)));
    }
}
