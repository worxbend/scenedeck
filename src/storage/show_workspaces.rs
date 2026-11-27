//! Collection-specific curation. All mutations run on the registry FIFO lane.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::registry::{read_registry_from_path, write_registry_to_path, SceneRegistry};
use crate::infra::sync::lock_or_recover;

/// Creating a workspace past this count drops the least recently used one.
pub(crate) const MAX_WORKSPACES: usize = 64;

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
                entry.last_used_unix_ms = now_unix_ms();
                commit(path, store, |_| ())
            }
        }
    }

    /// Adopt legacy data at most once, in the same atomic write as its owner.
    /// The source file is never modified or removed. A refused/corrupt source
    /// must fail the migration rather than adopt a defaults-based replacement.
    /// A hit writes nothing; its use is recorded by the next store write.
    pub(crate) fn load_or_create(&self, legacy: Option<&Path>) -> io::Result<LoadedWorkspace> {
        let Self::Workspace { path, key } = self else {
            return self.read().map(LoadedWorkspace::kept);
        };
        let mut store = read_store(path)?;
        if let Some(entry) = store.entries.iter().find(|entry| &entry.key == key) {
            remember_read(RecentRead {
                path: path.clone(),
                key: key.clone(),
                unix_ms: now_unix_ms(),
            });
            return Ok(LoadedWorkspace::kept(entry.registry.clone()));
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
            last_used_unix_ms: now_unix_ms(),
        });
        let evicted = commit(path, store, |store| store.evict_least_recently_used(key))?;
        Ok(LoadedWorkspace { registry, evicted })
    }
}

/// The selected workspace's registry, and any workspaces its creation dropped.
#[derive(Debug)]
pub(crate) struct LoadedWorkspace {
    pub registry: SceneRegistry,
    /// Dropped to stay within [`MAX_WORKSPACES`], least recently used first.
    pub evicted: Vec<WorkspaceKey>,
}

impl LoadedWorkspace {
    fn kept(registry: SceneRegistry) -> Self {
        Self {
            registry,
            evicted: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct StoredWorkspace {
    key: WorkspaceKey,
    registry: SceneRegistry,
    /// Zero for entries written before use was tracked: oldest first.
    #[serde(default)]
    last_used_unix_ms: u64,
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

impl WorkspaceStore {
    fn mark_used(&mut self, reads: &[RecentRead]) {
        for read in reads {
            if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key == read.key) {
                entry.last_used_unix_ms = entry.last_used_unix_ms.max(read.unix_ms);
            }
        }
    }

    /// Never drops `keep` or the legacy owner, so a clock that moved backwards
    /// cannot evict the workspace being opened. Equal stamps drop the earlier
    /// entry first.
    fn evict_least_recently_used(&mut self, keep: &WorkspaceKey) -> Vec<WorkspaceKey> {
        let mut evicted = Vec::new();
        while self.entries.len() > MAX_WORKSPACES {
            let oldest = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    &entry.key != keep && Some(&entry.key) != self.legacy_owner.as_ref()
                })
                .min_by_key(|(_, entry)| entry.last_used_unix_ms)
                .map(|(index, _)| index);
            let Some(index) = oldest else {
                break;
            };
            evicted.push(self.entries.remove(index).key);
        }
        evicted
    }
}

/// A read hit not yet written to its store.
#[derive(Clone, PartialEq)]
struct RecentRead {
    path: PathBuf,
    key: WorkspaceKey,
    unix_ms: u64,
}

/// Reading a workspace must not cost a write, so its use waits here for the
/// next write to the same store in this process.
static RECENT_READS: Mutex<Vec<RecentRead>> = Mutex::new(Vec::new());

fn remember_read(read: RecentRead) {
    let mut reads = lock_or_recover(&RECENT_READS);
    reads.retain(|known| known.path != read.path || known.key != read.key);
    reads.push(read);
}

/// Write `store` with the read hits recorded since its last write. `edit`
/// runs after those hits apply, so eviction sees every recent read.
fn commit<T>(
    path: &Path,
    mut store: WorkspaceStore,
    edit: impl FnOnce(&mut WorkspaceStore) -> T,
) -> io::Result<T> {
    let reads: Vec<RecentRead> = lock_or_recover(&RECENT_READS)
        .iter()
        .filter(|read| read.path == path)
        .cloned()
        .collect();
    store.mark_used(&reads);
    let outcome = edit(&mut store);
    write_store(path, &store)?;
    lock_or_recover(&RECENT_READS).retain(|read| !reads.contains(read));
    Ok(outcome)
}

/// Before the epoch counts as never used; the entry only becomes the first
/// eviction candidate.
fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

fn read_store(path: &Path) -> io::Result<WorkspaceStore> {
    let raw =
        match super::small_file::read_to_string(path, super::small_file::WORKSPACE_STORE_LIMIT) {
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
    super::small_file::check_size(raw.len() as u64, super::small_file::WORKSPACE_STORE_LIMIT)?;
    super::atomic::write(path, &raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decks_and_current_filters_remain_separate_after_reloading_workspaces() {
        use crate::storage::registry::SceneFilter;
        let (dir, first) = fixture();
        let mut registry = first.load_or_create(None).unwrap().registry;
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
        let mut other = second.load_or_create(None).unwrap().registry;
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
                .registry
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
            .registry
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
                .registry
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

    fn workspace(path: &Path, collection: &str) -> RegistryTarget {
        RegistryTarget::Workspace {
            path: path.to_path_buf(),
            key: WorkspaceKey {
                connection: "default".into(),
                endpoint: "obs:4455".into(),
                collection: collection.into(),
            },
        }
    }

    fn key_of(target: &RegistryTarget) -> WorkspaceKey {
        let RegistryTarget::Workspace { key, .. } = target else {
            unreachable!()
        };
        key.clone()
    }

    /// A full store whose entry `n` was last used at `stamp(n)`.
    fn full_store(path: &Path, stamp: impl Fn(u64) -> u64) -> WorkspaceStore {
        let entries = (0..MAX_WORKSPACES as u64)
            .map(|n| StoredWorkspace {
                key: key_of(&workspace(path, &format!("Show {n}"))),
                registry: SceneRegistry::default(),
                last_used_unix_ms: stamp(n),
            })
            .collect();
        WorkspaceStore {
            entries,
            ..WorkspaceStore::default()
        }
    }

    fn stored_keys(path: &Path) -> Vec<WorkspaceKey> {
        read_store(path)
            .unwrap()
            .entries
            .into_iter()
            .map(|entry| entry.key)
            .collect()
    }

    #[test]
    fn creating_one_workspace_past_the_cap_evicts_only_the_least_recently_used() {
        let (dir, _) = fixture();
        let path = dir.join("shows.json");
        write_store(&path, &full_store(&path, |n| 100 + n)).unwrap();
        let created = workspace(&path, "New");

        let loaded = created.load_or_create(None).unwrap();

        assert_eq!(loaded.evicted, [key_of(&workspace(&path, "Show 0"))]);
        let keys = stored_keys(&path);
        assert_eq!(keys.len(), MAX_WORKSPACES);
        assert!(keys.contains(&key_of(&created)));
        assert!(keys.contains(&key_of(&workspace(&path, "Show 1"))));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn eviction_skips_the_legacy_owner_and_a_target_stamped_behind_a_skewed_clock() {
        let (dir, _) = fixture();
        let path = dir.join("shows.json");
        // Every entry was used "after" now, as if the clock moved backwards.
        let mut store = full_store(&path, |n| u64::MAX - 100 + n);
        store.legacy_owner = Some(key_of(&workspace(&path, "Show 0")));
        write_store(&path, &store).unwrap();
        let created = workspace(&path, "New");

        let loaded = created.load_or_create(None).unwrap();

        assert_eq!(loaded.evicted, [key_of(&workspace(&path, "Show 1"))]);
        let keys = stored_keys(&path);
        assert!(keys.contains(&key_of(&created)));
        assert!(keys.contains(&key_of(&workspace(&path, "Show 0"))));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reading_or_writing_a_workspace_protects_it_from_the_next_eviction() {
        let (dir, _) = fixture();
        let path = dir.join("shows.json");
        write_store(&path, &full_store(&path, |n| 100 + n)).unwrap();
        let read = workspace(&path, "Show 0");
        let written = workspace(&path, "Show 1");
        let before_read = std::fs::read(&path).unwrap();

        read.load_or_create(None).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before_read);
        written.write(&SceneRegistry::default()).unwrap();
        let loaded = workspace(&path, "New").load_or_create(None).unwrap();

        assert_eq!(loaded.evicted, [key_of(&workspace(&path, "Show 2"))]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn entries_saved_before_use_tracking_are_evicted_first_in_file_order() {
        let (dir, _) = fixture();
        let path = dir.join("shows.json");
        let mut store = full_store(&path, |n| 100 + n);
        store.entries[10].last_used_unix_ms = 0;
        store.entries[20].last_used_unix_ms = 0;
        let mut raw: serde_json::Value = serde_json::to_value(&store).unwrap();
        for index in [10, 20] {
            raw["entries"][index]
                .as_object_mut()
                .unwrap()
                .remove("last_used_unix_ms");
        }
        std::fs::write(&path, raw.to_string()).unwrap();

        let loaded = workspace(&path, "New").load_or_create(None).unwrap();

        assert_eq!(loaded.evicted, [key_of(&workspace(&path, "Show 10"))]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn store_at_its_byte_limit_loads_and_one_byte_more_is_refused() {
        let (dir, target) = fixture();
        let path = dir.join("shows.json");
        let mut store = WorkspaceStore::default();
        store.entries.push(StoredWorkspace {
            key: key_of(&target),
            registry: SceneRegistry::default(),
            last_used_unix_ms: 1,
        });
        let mut raw = serde_json::to_vec(&store).unwrap();
        // JSON allows trailing whitespace: pad to exactly the store limit,
        // which is larger than one registry's limit.
        raw.resize(super::super::small_file::WORKSPACE_STORE_LIMIT, b' ');
        std::fs::write(&path, &raw).unwrap();

        assert_eq!(target.read().unwrap(), SceneRegistry::default());

        raw.push(b' ');
        std::fs::write(&path, &raw).unwrap();
        assert_eq!(
            target.read().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn show_workspace_fixture_round_trips() {
        let fixture = include_str!("fixtures/show-workspaces.json");
        crate::storage::golden::json_round_trip::<WorkspaceStore>(fixture);
        validate(&serde_json::from_str(fixture).unwrap()).unwrap();
    }
}
