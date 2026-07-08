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
    /// Graph validation rules stored as role keys for human-editable files.
    #[serde(default)]
    pub rules: RuleConfig,
}

/// Serialized metadata for one OBS scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntry {
    /// Local classification used by Live, Graph, and Doctor.
    pub role: SceneRole,
    /// Free-form labels reserved for user workflows and future filters.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Protected scenes are expected to be building blocks, not live switches.
    #[serde(default)]
    pub protected: bool,
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
    /// Convert serialized registry data into a domain-facing immutable snapshot.
    pub fn snapshot(&self) -> SceneRegistrySnapshot {
        let scenes = self
            .scenes
            .iter()
            .map(|(scene_id, entry)| {
                (
                    scene_id.clone(),
                    SceneMetadata {
                        role: entry.role,
                        tags: entry.tags.clone(),
                        protected: entry.protected,
                    },
                )
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
                Some(entry) if entry.role == role => false,
                Some(entry) => {
                    entry.role = role;
                    true
                }
                None => {
                    self.scenes.insert(
                        scene_id.to_string(),
                        SceneEntry {
                            role,
                            tags: Vec::new(),
                            protected: false,
                        },
                    );
                    true
                }
            },
            None => self.scenes.remove(scene_id).is_some(),
        }
    }
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
                role: SceneRole::Primary,
                tags: vec!["live".into()],
                protected: true,
            },
        );
        registry
            .rules
            .forbidden_edges
            .push(["primary".into(), "debug".into()]);

        write_registry_yaml_to_path(&path, &registry).unwrap();
        let parsed = read_registry_yaml_from_path(&path).unwrap();
        let _ = std::fs::remove_file(path);

        let main = parsed.scenes.get("Main").unwrap();
        assert_eq!(main.role, SceneRole::Primary);
        assert_eq!(main.tags, vec!["live"]);
        assert!(main.protected);
        assert_eq!(
            parsed.rules.forbidden_edges,
            vec![["primary".to_string(), "debug".to_string()]]
        );
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
        assert_eq!(entry.role, SceneRole::Primary);
        assert!(entry.tags.is_empty());
        assert!(!entry.protected);
    }

    #[test]
    fn scene_registry_set_role_updates_role_without_losing_metadata() {
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".to_string(),
            SceneEntry {
                role: SceneRole::Module,
                tags: vec!["camera".to_string()],
                protected: true,
            },
        );

        assert!(registry.set_scene_role("Main", Some(SceneRole::Secondary)));

        let entry = registry.scenes.get("Main").unwrap();
        assert_eq!(entry.role, SceneRole::Secondary);
        assert_eq!(entry.tags, ["camera"]);
        assert!(entry.protected);
    }

    #[test]
    fn scene_registry_set_role_removes_entry_for_unassigned_role() {
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".to_string(),
            SceneEntry {
                role: SceneRole::Primary,
                tags: Vec::new(),
                protected: false,
            },
        );

        assert!(registry.set_scene_role("Main", None));

        assert!(!registry.scenes.contains_key("Main"));
    }

    #[test]
    fn set_scene_role_from_path_preserves_existing_metadata() {
        let path = unique_temp_path("registry-set-role", "json");
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Main".into(),
            SceneEntry {
                role: SceneRole::Module,
                tags: vec!["camera".into()],
                protected: true,
            },
        );
        write_registry_to_path(&path, &registry).unwrap();

        let changed = set_scene_role_from_path(&path, "Main", Some(SceneRole::Secondary)).unwrap();
        let parsed = read_registry_from_path(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert!(changed);
        let entry = &parsed.scenes["Main"];
        assert_eq!(entry.role, SceneRole::Secondary);
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
                role: SceneRole::Primary,
                tags: vec!["live".into()],
                protected: false,
            },
        );
        registry.scenes.insert(
            "Camera".into(),
            SceneEntry {
                role: SceneRole::Raw,
                tags: vec!["source".into()],
                protected: true,
            },
        );
        write_registry_to_path(&path, &registry).unwrap();

        let removed = remove_scene_entry_from_path(&path, "Main").unwrap();
        let parsed = read_registry_from_path(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert!(removed);
        assert!(!parsed.scenes.contains_key("Main"));
        assert_eq!(parsed.scenes["Camera"].role, SceneRole::Raw);
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
