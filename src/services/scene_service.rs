//! Scene switching and role enforcement.

use crate::domain::registry::SceneRegistrySnapshot;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use thiserror::Error;

#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum SceneSwitchError {
    #[error("scene '{scene_id}' was not found in OBS")]
    SceneNotFound { scene_id: String },

    #[error("scene '{scene_id}' has role {role:?}, only Primary scenes are live-switchable")]
    NotLiveSwitchable { scene_id: String, role: SceneRole },
}

pub struct SceneService;

impl SceneService {
    /// Returns `Err` if `scene_id` does not have the `Primary` role.
    pub fn validate_switch(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        scene_id: &str,
    ) -> Result<(), SceneSwitchError> {
        if !inventory.scenes.iter().any(|scene| scene.id == scene_id) {
            return Err(SceneSwitchError::SceneNotFound {
                scene_id: scene_id.to_string(),
            });
        }

        let role = registry.scene_role(scene_id).unwrap_or_default();

        if role != SceneRole::Primary {
            return Err(SceneSwitchError::NotLiveSwitchable {
                scene_id: scene_id.to_string(),
                role,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::domain::registry::SceneMetadata;
    use crate::domain::scene::Scene;

    fn inventory(scene_id: &str) -> SceneInventory {
        SceneInventory {
            scenes: vec![Scene {
                id: scene_id.to_string(),
                name: scene_id.to_string(),
                role: None,
            }],
            current_id: None,
        }
    }

    fn registry(scene_id: &str, role: SceneRole) -> SceneRegistrySnapshot {
        SceneRegistrySnapshot::new(
            HashMap::from([(
                scene_id.to_string(),
                SceneMetadata {
                    role,
                    tags: Vec::new(),
                    protected: false,
                },
            )]),
            Default::default(),
        )
    }

    #[test]
    fn primary_scene_is_switchable() {
        let inventory = inventory("Main");
        let registry = registry("Main", SceneRole::Primary);

        assert_eq!(
            SceneService::validate_switch(&inventory, &registry, "Main"),
            Ok(())
        );
    }

    #[test]
    fn missing_scene_is_not_switchable() {
        let inventory = inventory("Main");
        let registry = registry("Main", SceneRole::Primary);

        assert_eq!(
            SceneService::validate_switch(&inventory, &registry, "Missing"),
            Err(SceneSwitchError::SceneNotFound {
                scene_id: "Missing".to_string()
            })
        );
    }

    #[test]
    fn non_primary_scene_is_not_switchable() {
        let inventory = inventory("Module");
        let registry = registry("Module", SceneRole::Module);

        assert_eq!(
            SceneService::validate_switch(&inventory, &registry, "Module"),
            Err(SceneSwitchError::NotLiveSwitchable {
                scene_id: "Module".to_string(),
                role: SceneRole::Module,
            })
        );
    }
}
