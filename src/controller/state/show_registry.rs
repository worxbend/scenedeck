//! Local curation follows the observed connection and collection.

use std::collections::HashMap;

use crate::storage::{
    registry::SceneRegistry,
    show_workspaces::{RegistryTarget, WorkspaceKey},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RegistryStatus {
    Ready,
    Loading,
    Failed(String),
    Unavailable,
}

#[derive(Debug, Clone)]
pub(crate) struct ShowRegistryState {
    pub target: RegistryTarget,
    pub generation: u64,
    pub revision: u64,
    pub status: RegistryStatus,
    cache: HashMap<RegistryTarget, SceneRegistry>,
}

impl Default for ShowRegistryState {
    fn default() -> Self {
        Self {
            target: RegistryTarget::legacy(),
            generation: 0,
            revision: 0,
            status: RegistryStatus::Ready,
            cache: HashMap::new(),
        }
    }
}

impl super::AppState {
    fn remember_registry(&mut self) {
        if self.show_registry.status == RegistryStatus::Ready {
            self.show_registry
                .cache
                .insert(self.show_registry.target.clone(), self.registry.clone());
        }
    }

    pub(crate) fn invalidate_registry_selection(&mut self) {
        self.remember_registry();
        self.show_registry.generation += 1;
        self.show_registry.status = RegistryStatus::Unavailable;
    }

    pub(crate) fn desired_registry_target(&self) -> Option<RegistryTarget> {
        if !matches!(self.obs_status, super::ObsStatus::Connected { .. }) {
            return None;
        }
        let collection = self
            .scene_collections
            .current
            .as_ref()
            .filter(|name| !name.is_empty())?;
        Some(RegistryTarget::workspace(WorkspaceKey {
            connection: self
                .connected_connection_id
                .clone()
                .unwrap_or_else(|| self.config.connection_id.clone()),
            endpoint: self.connected_endpoint.clone()?,
            collection: collection.clone(),
        }))
    }

    /// Returns a generation ticket only when disk loading is necessary.
    pub(crate) fn select_registry(&mut self, target: RegistryTarget) -> Option<u64> {
        if self.show_registry.target == target
            && matches!(
                self.show_registry.status,
                RegistryStatus::Ready | RegistryStatus::Loading
            )
        {
            return None;
        }
        self.remember_registry();
        self.show_registry.generation += 1;
        self.show_registry.target = target;
        if let Some(registry) = self.show_registry.cache.get(&self.show_registry.target) {
            self.registry = registry.clone();
            self.show_registry.status = RegistryStatus::Ready;
            None
        } else {
            self.registry = SceneRegistry::default();
            self.show_registry.status = RegistryStatus::Loading;
            Some(self.show_registry.generation)
        }
    }

    pub(crate) fn accepts_registry_load(&self, generation: u64, target: &RegistryTarget) -> bool {
        self.show_registry.generation == generation
            && &self.show_registry.target == target
            && self.show_registry.status == RegistryStatus::Loading
    }

    pub(crate) fn publish_registry_import(
        &mut self,
        target: RegistryTarget,
        registry: SceneRegistry,
    ) {
        // The exclusive import gate has refused edits since admission. If the
        // user returned to this cached workspace meanwhile, its accepted import
        // is still authoritative; a generation mismatch must not resurrect the
        // pre-import snapshot on the next edit. A pending load reads it from disk.
        if self.show_registry.target == target && self.show_registry.status == RegistryStatus::Ready
        {
            self.registry = registry.clone();
            self.show_registry.generation += 1;
        }
        self.show_registry.cache.insert(target, registry);
    }

    pub(crate) fn finish_registry_load(
        &mut self,
        generation: u64,
        target: &RegistryTarget,
        result: std::io::Result<SceneRegistry>,
    ) {
        if !self.accepts_registry_load(generation, target) {
            return;
        }
        match result {
            Ok(registry) => {
                self.registry = registry;
                self.show_registry.status = RegistryStatus::Ready;
            }
            Err(error) => self.show_registry.status = RegistryStatus::Failed(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_load_is_rejected_and_unsaved_workspace_intent_survives_switching() {
        let mut state =
            super::super::AppState::new(Default::default(), Default::default(), None, None);
        let a = RegistryTarget::File("a.json".into());
        let b = RegistryTarget::File("b.json".into());
        let first = state.select_registry(a.clone()).unwrap();
        let second = state.select_registry(b.clone()).unwrap();
        state.finish_registry_load(first, &a, Ok(SceneRegistry::default()));
        assert_eq!(state.show_registry.status, RegistryStatus::Loading);
        state.finish_registry_load(second, &b, Ok(SceneRegistry::default()));
        state
            .registry
            .set_scene_icon("Camera", Some("accepted but unsaved"));
        state.invalidate_registry_selection();
        state.select_registry(a.clone());
        state.invalidate_registry_selection();
        assert!(state.select_registry(b).is_none());
        assert_eq!(
            state.registry.scene_icon("Camera"),
            Some("accepted but unsaved")
        );
        assert_eq!(state.show_registry.status, RegistryStatus::Ready);
    }
}
