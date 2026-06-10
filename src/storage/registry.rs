//! Scene role registry — the local metadata layer that augments the raw OBS
//! scene list with roles, tags, and protection flags.

use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

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

pub fn read_registry() -> SceneRegistry {
    let path = xdg::config_dir().join("registry.json");
    match read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => SceneRegistry::default(),
        Err(err) => {
            tracing::warn!(%err, "failed to read registry, using defaults");
            SceneRegistry::default()
        }
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
}
