use crate::domain::role::SceneRole;

/// Stable identifier for an OBS scene.  Matches `sceneName`.
pub type SceneId = String;

#[derive(Debug, Clone)]
pub struct Scene {
    pub id: SceneId,
    pub name: String,
    /// Role assigned by the local registry; `None` if unclassified.
    pub role: Option<SceneRole>,
}

/// Full scene list plus the currently active scene.
#[derive(Debug, Default, Clone)]
pub struct SceneInventory {
    pub scenes: Vec<Scene>,
    pub current_id: Option<SceneId>,
}

impl SceneInventory {
    /// Scenes that may be switched from the Live page.
    pub fn live_scenes(&self) -> impl Iterator<Item = &Scene> {
        self.scenes
            .iter()
            .filter(|s| s.role.map(SceneRole::is_live_switchable).unwrap_or(false))
    }

    pub fn current_scene(&self) -> Option<&Scene> {
        self.current_id
            .as_deref()
            .and_then(|id| self.scenes.iter().find(|s| s.id == id))
    }
}
