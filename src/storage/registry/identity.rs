//! Reconcile names only within a confirmed connection and scene collection.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{SceneEntry, SceneRegistry};
use crate::domain::scene::SceneInventory;

/// Retained curation is not applied to any scene until its identity is unambiguous.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RetainedScene {
    pub name: String,
    pub uuid: Option<String>,
    pub entry: Option<SceneEntry>,
    pub order: Option<usize>,
}

impl SceneRegistry {
    /// Stage all moves together, so swaps cannot overwrite one another. Missing
    /// UUIDs preserve name-based behavior; duplicate identities prove nothing.
    pub(crate) fn reconcile_scenes(&mut self, inventory: &SceneInventory) -> bool {
        let mut live = HashMap::new();
        let mut by_uuid = HashMap::new();
        let mut names = HashSet::new();
        for scene in &inventory.scenes {
            if scene.id.is_empty() || !names.insert(scene.id.as_str()) {
                return false;
            }
            if let Some(uuid) = scene.uuid.as_deref().filter(|uuid| !uuid.is_empty()) {
                if by_uuid.insert(uuid, scene.id.as_str()).is_some() {
                    return false;
                }
                live.insert(scene.id.clone(), uuid.to_owned());
            }
        }
        if live.is_empty() {
            return false;
        }
        let before = self.clone();
        let mut candidates: BTreeMap<String, Vec<RetainedScene>> = BTreeMap::new();
        let mut source_names: Vec<_> = self
            .scenes
            .keys()
            .chain(&self.scene_order)
            .cloned()
            .collect();
        source_names.sort();
        source_names.dedup();
        for name in source_names {
            let record = RetainedScene {
                uuid: self.scene_uuids.get(&name).cloned(),
                entry: self.scenes.remove(&name),
                order: self.scene_order.iter().position(|item| item == &name),
                name,
            };
            let destination = record.uuid.as_deref().and_then(|uuid| by_uuid.get(uuid));
            if destination.is_none()
                && record.uuid.as_ref().is_some_and(|uuid| {
                    live.get(&record.name)
                        .is_some_and(|current| current != uuid)
                })
            {
                self.retained_scenes.push(record);
            } else {
                candidates
                    .entry(destination.copied().unwrap_or(&record.name).to_owned())
                    .or_default()
                    .push(record);
            }
        }
        self.scene_uuids = live;
        let mut ordered = Vec::new();
        for (name, mut records) in candidates {
            // An explicit destination entry wins over a colliding rename. Both
            // are kept; only the chosen entry participates in controls/Doctor.
            let winner = if records.len() == 1 {
                Some(0)
            } else {
                records.iter().position(|record| record.name == name)
            };
            if let Some(winner) = winner {
                let record = records.remove(winner);
                self.install_identity_record(&name, record, &mut ordered);
            }
            self.retained_scenes.extend(records);
        }
        // A deleted scene may reappear with its old UUID under another name.
        // Multiple retained versions or existing curation require manual review.
        let retained = std::mem::take(&mut self.retained_scenes);
        for record in &retained {
            let destination = record.uuid.as_deref().and_then(|uuid| by_uuid.get(uuid));
            let unique = retained
                .iter()
                .filter(|other| other.uuid == record.uuid)
                .count()
                == 1;
            if let Some(name) = destination.filter(|name| {
                unique
                    && !self.scenes.contains_key(**name)
                    && !ordered
                        .iter()
                        .any(|(_, ordered_name)| ordered_name == *name)
            }) {
                self.install_identity_record(name, record.clone(), &mut ordered);
            } else {
                self.retained_scenes.push(record.clone());
            }
        }
        ordered.sort();
        self.scene_order = ordered.into_iter().map(|(_, name)| name).collect();
        *self != before
    }

    fn install_identity_record(
        &mut self,
        name: &str,
        record: RetainedScene,
        ordered: &mut Vec<(usize, String)>,
    ) {
        if let Some(entry) = record.entry {
            self.scenes.insert(name.to_owned(), entry);
        }
        if let Some(order) = record.order {
            ordered.push((order, name.to_owned()));
        }
        if let Some(uuid) = record.uuid {
            self.scene_uuids.entry(name.to_owned()).or_insert(uuid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{role::SceneRole, scene::Scene};

    fn inventory(scenes: &[(&str, &str)]) -> SceneInventory {
        SceneInventory {
            scenes: scenes
                .iter()
                .map(|(name, uuid)| Scene {
                    id: (*name).into(),
                    uuid: Some((*uuid).into()),
                })
                .collect(),
            ..Default::default()
        }
    }

    fn curated() -> SceneRegistry {
        let mut registry = SceneRegistry::default();
        registry.scenes.insert(
            "Camera".into(),
            SceneEntry {
                role: Some(SceneRole::Primary),
                tags: vec!["podcast".into()],
                favorite: true,
                protected: true,
                accent_color: Some("#112233".into()),
                icon: Some("camera".into()),
            },
        );
        registry.set_scene_icon("Break", Some("pause"));
        registry.scene_order = vec!["Break".into(), "Camera".into()];
        assert!(registry.reconcile_scenes(&inventory(&[("Camera", "a"), ("Break", "b")])));
        registry
    }

    #[test]
    fn disconnected_rename_and_simultaneous_swaps_follow_uuid_and_order() {
        let mut registry = curated();
        let original = registry.scenes["Camera"].clone();
        let renamed = inventory(&[("Guest", "a"), ("Break", "b")]);
        assert!(registry.reconcile_scenes(&renamed));
        assert_eq!(registry.scenes["Guest"], original);
        assert_eq!(registry.scene_order, ["Break", "Guest"]);
        assert!(!registry.reconcile_scenes(&renamed));
        let swapped = inventory(&[("Break", "a"), ("Guest", "b")]);
        assert!(registry.reconcile_scenes(&swapped));
        assert_eq!(registry.scenes["Break"], original);
        assert_eq!(registry.scene_icon("Guest"), Some("pause"));
        assert_eq!(registry.scene_order, ["Guest", "Break"]);
        assert!(!registry.reconcile_scenes(&swapped));
        assert!(registry.retained_scenes.is_empty());
    }

    #[test]
    fn reused_name_retains_metadata_and_restores_only_the_original_uuid() {
        let mut registry = curated();
        let original = registry.scenes["Camera"].clone();
        let replacement = inventory(&[("Camera", "replacement"), ("Break", "b")]);
        assert!(registry.reconcile_scenes(&replacement));
        assert!(!registry.scenes.contains_key("Camera"));
        assert_eq!(registry.scene_order, ["Break"]);
        assert_eq!(registry.retained_scenes[0].entry, Some(original.clone()));
        assert!(!registry.reconcile_scenes(&replacement));
        let returned = inventory(&[("Camera", "replacement"), ("Guest", "a"), ("Break", "b")]);
        assert!(registry.reconcile_scenes(&returned));
        assert_eq!(registry.scenes["Guest"], original);
        assert_eq!(registry.scene_order, ["Break", "Guest"]);
        assert!(registry.retained_scenes.is_empty());
        assert!(!registry.reconcile_scenes(&returned));
    }

    #[test]
    fn explicit_destination_conflict_preserves_both_versions_without_repeat_changes() {
        let mut registry = curated();
        registry.set_scene_icon("Guest", Some("explicit destination"));
        let original = registry.scenes["Camera"].clone();
        let renamed = inventory(&[("Guest", "a"), ("Break", "b")]);
        assert!(registry.reconcile_scenes(&renamed));
        assert_eq!(registry.scene_icon("Guest"), Some("explicit destination"));
        assert_eq!(registry.retained_scenes[0].entry, Some(original));
        assert!(!registry.reconcile_scenes(&renamed));
    }

    #[test]
    fn unknown_and_duplicate_identities_cannot_move_curation() {
        let mut registry = curated();
        let original = registry.clone();
        assert!(!registry.reconcile_scenes(&inventory(&[("Guest", "a"), ("Other", "a")])));
        assert!(!registry.reconcile_scenes(&SceneInventory {
            scenes: vec![Scene {
                id: "Guest".into(),
                uuid: None
            }],
            ..Default::default()
        }));
        assert_eq!(registry, original);
        assert_eq!(
            registry.rename_scene("Camera", "Guest", Some("different")),
            super::super::SceneRenameResult::Unchanged
        );
        assert_eq!(registry, original);
    }

    #[test]
    fn portable_yaml_rebinds_named_intent_and_keeps_retained_entries_inert() {
        let mut registry = curated();
        registry.reconcile_scenes(&inventory(&[("Camera", "replacement"), ("Break", "b")]));
        let path =
            std::env::temp_dir().join(format!("scenedeck-identity-{}.yaml", std::process::id()));
        super::super::write_registry_yaml_to_path(&path, &registry).unwrap();
        let mut imported = super::super::read_registry_yaml_from_path(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(imported.scene_uuids.is_empty());
        assert_eq!(imported.retained_scenes[0].uuid, None);
        let other = inventory(&[("Break", "other collection"), ("Unexpected", "a")]);
        assert!(imported.reconcile_scenes(&other));
        assert_eq!(imported.scene_icon("Break"), Some("pause"));
        assert_eq!(imported.scene_icon("Unexpected"), None);
        assert_eq!(
            imported.retained_scenes[0].entry,
            registry.retained_scenes[0].entry
        );
        assert!(!imported.reconcile_scenes(&other));
    }
}
