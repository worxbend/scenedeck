//! Scene role registry — the local metadata layer that augments the raw OBS
//! scene list with roles, tags, and protection flags.

use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::registry::{
    GraphDependencyPolicy, RoleDependency, SceneMetadata, SceneRegistrySnapshot,
};
use crate::domain::role::SceneRole;
use crate::storage::xdg;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SceneRegistry {
    #[serde(default)]
    pub scenes: HashMap<String, SceneEntry>,
    #[serde(default)]
    pub rules: RuleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntry {
    pub role: SceneRole,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub protected: bool,
}

/// Graph dependency rules evaluated by the Doctor.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    #[serde(default)]
    pub primary_can_depend_on: Vec<String>,
    #[serde(default)]
    pub module_can_depend_on: Vec<String>,
    /// Pairs `[from_role, to_role]` that are unconditionally forbidden.
    #[serde(default)]
    pub forbidden_edges: Vec<[String; 2]>,
}

#[derive(Debug, Error)]
pub enum RegistryStorageError {
    #[error("failed to read scene registry at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse scene registry at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl SceneRegistry {
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
}

impl RuleConfig {
    pub fn policy(&self) -> GraphDependencyPolicy {
        GraphDependencyPolicy {
            primary_can_depend_on: parse_role_list(&self.primary_can_depend_on),
            module_can_depend_on: parse_role_list(&self.module_can_depend_on),
            forbidden_edges: self
                .forbidden_edges
                .iter()
                .filter_map(|[from_key, to_key]| {
                    let from = SceneRole::from_rule_key(from_key);
                    let to = SceneRole::from_rule_key(to_key);
                    match (from, to) {
                        (Some(from), Some(to)) => Some(RoleDependency::new(from, to)),
                        _ => {
                            tracing::warn!(
                                from = from_key.as_str(),
                                to = to_key.as_str(),
                                "ignoring registry rule with unknown scene role"
                            );
                            None
                        }
                    }
                })
                .collect(),
        }
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

pub fn read_registry() -> SceneRegistry {
    let path = xdg::config_dir().join("registry.json");
    match read_registry_from_path(&path) {
        Ok(registry) => registry,
        Err(err) => {
            tracing::warn!(%err, "failed to load registry, using defaults");
            SceneRegistry::default()
        }
    }
}

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

pub fn write_registry(registry: &SceneRegistry) -> io::Result<()> {
    write_registry_to_path(&xdg::config_dir().join("registry.json"), registry)
}

pub fn write_registry_to_path(path: &Path, registry: &SceneRegistry) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }
    let raw = serde_json::to_string_pretty(registry).map_err(io::Error::other)?;
    write(path, raw)
}

pub fn read_registry_yaml_from_path(path: &Path) -> io::Result<SceneRegistry> {
    let raw = read_to_string(path)?;
    serde_yaml::from_str(&raw).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

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

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "scenedeck-{name}-{}-{nanos}.yaml",
            std::process::id()
        ))
    }

    #[test]
    fn yaml_registry_round_trips() {
        let path = unique_temp_path("registry-round-trip");
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
        let path = unique_temp_path("registry-invalid");
        std::fs::write(&path, "scenes: [").unwrap();

        let err = read_registry_yaml_from_path(&path).unwrap_err();
        let _ = std::fs::remove_file(path);

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn invalid_json_registry_returns_typed_parse_error() {
        let path = unique_temp_path("registry-invalid-json");
        std::fs::write(&path, "{").unwrap();

        let err = read_registry_from_path(&path).unwrap_err();
        let _ = std::fs::remove_file(path);

        assert!(matches!(err, RegistryStorageError::Parse { .. }));
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
        assert_eq!(
            snapshot.graph_policy().forbidden_edges,
            vec![RoleDependency::new(SceneRole::Primary, SceneRole::Debug)]
        );
    }
}
