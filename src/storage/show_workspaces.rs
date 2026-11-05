//! Collection-specific curation. All mutations run on the registry FIFO lane.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::registry::{read_registry_from_path, write_registry_to_path, SceneRegistry};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct WorkspaceKey {
    pub connection: String,
    pub endpoint: String,
    pub collection: String,
}

/// Capture this destination at edit admission, never at worker execution time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum RegistryTarget {
    File(PathBuf),
    Workspace { path: PathBuf, key: WorkspaceKey },
}

impl RegistryTarget {
    pub(crate) fn legacy() -> Self {
        Self::File(super::xdg::registry_path())
    }

    pub(crate) fn workspace(key: WorkspaceKey) -> Self {
        Self::Workspace {
            path: super::xdg::show_workspaces_path(),
            key,
        }
    }

    pub(crate) fn read(&self) -> io::Result<SceneRegistry> {
        match self {
            Self::File(path) => read_registry_from_path(path).map_err(io::Error::other),
            Self::Workspace { path, key } => read_store(path)?
                .entries
                .into_iter()
                .find(|entry| &entry.key == key)
                .map(|entry| entry.registry)
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "show workspace is missing")
                }),
        }
    }

    pub(crate) fn write(&self, registry: &SceneRegistry) -> io::Result<()> {
        match self {
            Self::File(path) => write_registry_to_path(path, registry),
            Self::Workspace { path, key } => {
                let mut store = read_store(path)?;
                let entry = store
                    .entries
                    .iter_mut()
                    .find(|entry| &entry.key == key)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::NotFound, "show workspace is missing")
                    })?;
                entry.registry = registry.clone();
                write_store(path, &store)
            }
        }
    }

    /// Adopt legacy data at most once, in the same atomic write as its owner.
    /// The source file is never modified or removed. A refused/corrupt source
    /// must fail the migration rather than adopt a defaults-based replacement.
    pub(crate) fn load_or_create(&self, legacy: Option<&Path>) -> io::Result<SceneRegistry> {
        let Self::Workspace { path, key } = self else {
            return self.read();
        };
        let mut store = read_store(path)?;
        if let Some(entry) = store.entries.iter().find(|entry| &entry.key == key) {
            return Ok(entry.registry.clone());
        }
        let registry = match legacy.filter(|_| store.legacy_owner.is_none()) {
            Some(source) => {
                let registry = read_registry_from_path(source).map_err(io::Error::other)?;
                store.legacy_owner = Some(key.clone());
                registry
            }
            None => SceneRegistry::default(),
        };
        store.entries.push(StoredWorkspace {
            key: key.clone(),
            registry: registry.clone(),
        });
        write_store(path, &store)?;
        Ok(registry)
    }
}

#[derive(Serialize, Deserialize)]
struct StoredWorkspace {
    key: WorkspaceKey,
    registry: SceneRegistry,
}

#[derive(Serialize, Deserialize)]
struct WorkspaceStore {
    version: u32,
    #[serde(default)]
    legacy_owner: Option<WorkspaceKey>,
    #[serde(default)]
    entries: Vec<StoredWorkspace>,
}

impl Default for WorkspaceStore {
    fn default() -> Self {
        Self {
            version: 1,
            legacy_owner: None,
            entries: Vec::new(),
        }
    }
}

fn read_store(path: &Path) -> io::Result<WorkspaceStore> {
    let raw = match super::small_file::read_to_string(path, super::small_file::REGISTRY_LIMIT) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(WorkspaceStore::default())
        }
        Err(error) => return Err(error),
    };
    let store: WorkspaceStore = serde_json::from_str(&raw).map_err(io::Error::other)?;
    validate(&store)?;
    Ok(store)
}

fn validate(store: &WorkspaceStore) -> io::Result<()> {
    let mut keys = std::collections::HashSet::new();
    if store.version != 1
        || store.entries.iter().any(|entry| {
            entry.key.connection.is_empty()
                || entry.key.endpoint.is_empty()
                || entry.key.collection.is_empty()
                || !keys.insert(&entry.key)
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported or invalid show workspace store",
        ));
    }
    Ok(())
}

fn write_store(path: &Path, store: &WorkspaceStore) -> io::Result<()> {
    validate(store)?;
    let raw = serde_json::to_vec_pretty(store).map_err(io::Error::other)?;
    super::small_file::check_size(raw.len() as u64, super::small_file::REGISTRY_LIMIT)?;
    super::atomic::write(path, &raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decks_and_current_filters_remain_separate_after_reloading_workspaces() {
        use crate::storage::registry::SceneFilter;
        let (dir, first) = fixture();
        let mut registry = first.load_or_create(None).unwrap();
        registry.scene_filter = SceneFilter {
            query: "podcast".into(),
            favorites_only: true,
        };
        registry.save_deck("Show", registry.scene_filter.clone());
        registry
            .action_bindings
            .push(crate::domain::action_shortcut::ActionBinding {
                shortcut: crate::domain::action_shortcut::ActionShortcut {
                    key: "F8".into(),
                    modifiers: crate::domain::hotkey::Modifiers::NONE,
                },
                action: crate::domain::operator_action::OperatorAction::SaveReplay,
                enabled: true,
            });
        registry
            .quick_actions
            .push(crate::domain::operator_action::OperatorPin {
                action: crate::domain::operator_action::OperatorAction::SaveReplay,
                enabled: true,
            });
        registry
            .audio_alerts
            .push(crate::domain::health::AudioRule {
                target: crate::domain::operator_action::ActionTarget {
                    name: "Mic".into(),
                    uuid: Some("mic-one".into()),
                },
                enabled: true,
                silence: true,
                clipping: false,
            });
        first.write(&registry).unwrap();
        let RegistryTarget::Workspace { path, mut key } = first.clone() else {
            unreachable!()
        };
        key.connection = "another OBS".into();
        let second = RegistryTarget::Workspace { path, key };
        let mut other = second.load_or_create(None).unwrap();
        assert!(other.decks.is_empty());
        assert!(other.action_bindings.is_empty());
        assert!(other.quick_actions.is_empty());
        assert!(other.audio_alerts.is_empty());
        other.scene_filter.query = "game".into();
        other.save_deck("Show", other.scene_filter.clone());
        second.write(&other).unwrap();
        assert_eq!(first.read().unwrap(), registry);
        assert_eq!(second.read().unwrap(), other);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn fixture() -> (PathBuf, RegistryTarget) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "scenedeck-workspaces-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut target = RegistryTarget::workspace(WorkspaceKey {
            connection: "default".into(),
            endpoint: "obs:4455".into(),
            collection: "Podcast".into(),
        });
        if let RegistryTarget::Workspace { path, .. } = &mut target {
            *path = dir.join("shows.json");
        }
        (dir, target)
    }

    #[test]
    fn migration_is_once_and_collection_writes_preserve_other_workspaces() {
        let (dir, podcast) = fixture();
        let legacy = dir.join("registry.json");
        let mut registry = SceneRegistry::default();
        registry.set_scene_icon("Camera", Some("podcast"));
        let mut source = RegistryTarget::legacy();
        if let RegistryTarget::File(path) = &mut source {
            *path = legacy.clone();
        }
        source.write(&registry).unwrap();
        assert_eq!(
            podcast
                .load_or_create(Some(&legacy))
                .unwrap()
                .scene_icon("Camera"),
            Some("podcast")
        );
        let RegistryTarget::Workspace { path, mut key } = podcast.clone() else {
            unreachable!()
        };
        key.collection = "Gaming".into();
        let gaming = RegistryTarget::Workspace { path, key };
        assert!(gaming
            .load_or_create(Some(&legacy))
            .unwrap()
            .scenes
            .is_empty());
        registry.set_scene_icon("Camera", Some("gaming"));
        gaming.write(&registry).unwrap();
        assert_eq!(
            podcast.read().unwrap().scene_icon("Camera"),
            Some("podcast")
        );
        assert_eq!(gaming.read().unwrap().scene_icon("Camera"), Some("gaming"));
        assert_eq!(
            read_registry_from_path(&legacy)
                .unwrap()
                .scene_icon("Camera"),
            Some("podcast")
        );
        assert_eq!(
            podcast
                .load_or_create(Some(&legacy))
                .unwrap()
                .scene_icon("Camera"),
            Some("podcast")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_legacy_or_future_store_never_becomes_an_empty_migration() {
        let (dir, target) = fixture();
        let legacy = dir.join("registry.json");
        std::fs::write(&legacy, "invalid legacy").unwrap();
        assert!(target.load_or_create(Some(&legacy)).is_err());
        assert!(!dir.join("shows.json").exists());
        let future = r#"{"version":99,"entries":[]}"#;
        std::fs::write(dir.join("shows.json"), future).unwrap();
        assert!(target.load_or_create(None).is_err());
        assert!(target.write(&SceneRegistry::default()).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.join("shows.json")).unwrap(),
            future
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
