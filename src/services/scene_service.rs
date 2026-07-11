//! Scene switching and role enforcement.

use crate::domain::registry::SceneRegistrySnapshot;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use thiserror::Error;

/// Reason a scene switch request cannot be executed safely.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum SceneSwitchError {
    /// The requested scene id is not present in the current OBS inventory.
    #[error("scene '{scene_id}' was not found in OBS")]
    SceneNotFound {
        /// Requested OBS scene id.
        scene_id: String,
    },

    /// The scene exists but does not have a live-switchable role.
    #[error("scene '{scene_id}' has role {role:?}, which is not configured as live-switchable")]
    NotLiveSwitchable {
        /// Requested OBS scene id.
        scene_id: String,
        /// Current local role that blocked the switch.
        role: SceneRole,
    },
}

/// Validates scene switching rules before OBS commands are sent.
pub struct SceneService;

impl SceneService {
    /// Returns `Err` unless `scene_id` exists and has one of `allowed_roles`.
    pub fn validate_switch_with_roles(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        scene_id: &str,
        allowed_roles: &[SceneRole],
    ) -> Result<(), SceneSwitchError> {
        if !inventory.scenes.iter().any(|scene| scene.id == scene_id) {
            return Err(SceneSwitchError::SceneNotFound {
                scene_id: scene_id.to_string(),
            });
        }

        let role = registry.scene_role(scene_id).unwrap_or_default();

        if !allowed_roles.contains(&role) {
            return Err(SceneSwitchError::NotLiveSwitchable {
                scene_id: scene_id.to_string(),
                role,
            });
        }
        Ok(())
    }

    /// Returns `true` when a scene exists and is both shown and switchable.
    pub fn can_show_live_card(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        scene_id: &str,
        shown_roles: &[SceneRole],
        switchable_roles: &[SceneRole],
    ) -> bool {
        let Some(role) = registry.scene_role(scene_id) else {
            return false;
        };
        shown_roles.contains(&role)
            && Self::validate_switch_with_roles(inventory, registry, scene_id, switchable_roles)
                .is_ok()
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
            }],
            current_id: None,
            previous_id: None,
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
            SceneService::validate_switch_with_roles(
                &inventory,
                &registry,
                "Main",
                &[SceneRole::Primary],
            ),
            Ok(())
        );
    }

    #[test]
    fn missing_scene_is_not_switchable() {
        let inventory = inventory("Main");
        let registry = registry("Main", SceneRole::Primary);

        assert_eq!(
            SceneService::validate_switch_with_roles(
                &inventory,
                &registry,
                "Missing",
                &[SceneRole::Primary],
            ),
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
            SceneService::validate_switch_with_roles(
                &inventory,
                &registry,
                "Module",
                &[SceneRole::Primary],
            ),
            Err(SceneSwitchError::NotLiveSwitchable {
                scene_id: "Module".to_string(),
                role: SceneRole::Module,
            })
        );
    }

    #[test]
    fn configured_role_can_be_switchable() {
        let inventory = inventory("Secondary");
        let registry = registry("Secondary", SceneRole::Secondary);

        assert_eq!(
            SceneService::validate_switch_with_roles(
                &inventory,
                &registry,
                "Secondary",
                &[SceneRole::Primary, SceneRole::Secondary],
            ),
            Ok(())
        );
    }

    #[test]
    fn live_card_requires_visible_and_switchable_role() {
        let inventory = inventory("Secondary");
        let registry = registry("Secondary", SceneRole::Secondary);

        assert!(SceneService::can_show_live_card(
            &inventory,
            &registry,
            "Secondary",
            &[SceneRole::Primary, SceneRole::Secondary],
            &[SceneRole::Secondary],
        ));
        assert!(!SceneService::can_show_live_card(
            &inventory,
            &registry,
            "Secondary",
            &[SceneRole::Primary],
            &[SceneRole::Secondary],
        ));
        assert!(!SceneService::can_show_live_card(
            &inventory,
            &registry,
            "Secondary",
            &[SceneRole::Secondary],
            &[SceneRole::Primary],
        ));
    }
}
