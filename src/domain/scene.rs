//! Scene inventory state normalized from OBS.

/// Stable identifier for an OBS scene.  Matches `sceneName`.
pub type SceneId = String;

/// OBS scene identity and user-visible name.
#[derive(Debug, Clone)]
pub struct Scene {
    /// Stable OBS scene identifier.
    pub id: SceneId,
    /// User-visible scene name.  Currently the same value as `id`.
    pub name: String,
}

/// Full scene list plus the currently active scene.
#[derive(Debug, Default, Clone)]
pub struct SceneInventory {
    /// OBS scenes in switcher order.
    pub scenes: Vec<Scene>,
    /// Currently active program scene id, if OBS has reported one.
    pub current_id: Option<SceneId>,
    pub previous_id: Option<SceneId>,
}

impl SceneInventory {
    /// Scene matching `current_id`, if it is present in the inventory.
    pub fn current_scene(&self) -> Option<&Scene> {
        self.current_id
            .as_deref()
            .and_then(|id| self.scenes.iter().find(|s| s.id == id))
    }

    pub fn set_current_scene(&mut self, scene_id: SceneId) {
        if self.current_id.as_deref() != Some(scene_id.as_str()) {
            self.previous_id = self.current_id.replace(scene_id);
        }
    }
}
