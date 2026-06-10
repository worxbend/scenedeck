//! Scene switching and role enforcement.

use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use crate::infra::error::AppError;
use crate::storage::registry::SceneRegistry;

pub struct SceneService;

impl SceneService {
    /// Returns `Err` if `scene_id` does not have the `Primary` role.
    pub fn validate_switch(
        inventory: &SceneInventory,
        registry: &SceneRegistry,
        scene_id: &str,
    ) -> Result<(), AppError> {
        let role = registry
            .scenes
            .get(scene_id)
            .map(|e| e.role)
            .unwrap_or_default();

        if role != SceneRole::Primary {
            return Err(AppError::Request(format!(
                "scene '{scene_id}' has role {:?}, only Primary scenes are live-switchable",
                role
            )));
        }
        let _ = inventory;
        Ok(())
    }
}
