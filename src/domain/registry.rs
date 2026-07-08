//! Domain-facing scene registry metadata.
//!
//! Storage modules own the serialized registry format. Application services
//! consume this snapshot so diagnostics and validation do not depend on JSON or
//! YAML adapter structs.

use std::collections::HashMap;

use crate::domain::role::SceneRole;
use crate::domain::scene::SceneId;

/// Local metadata assigned to an OBS scene.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SceneMetadata {
    /// Scene role used by Live, Doctor, and graph validation.
    pub role: SceneRole,
    /// Free-form labels reserved for user workflows and future filters.
    pub tags: Vec<String>,
    /// Protected scenes are expected to be building blocks, not live switches.
    pub protected: bool,
}

/// Typed role dependency rule.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RoleDependency {
    /// Parent scene role.
    pub from: SceneRole,
    /// Nested child scene role.
    pub to: SceneRole,
}

impl RoleDependency {
    /// Build a typed role dependency rule.
    pub const fn new(from: SceneRole, to: SceneRole) -> Self {
        Self { from, to }
    }
}

/// Graph validation policy used by Doctor and the Graph page.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphDependencyPolicy {
    /// Roles that Primary scenes may depend on without warning.
    pub primary_can_depend_on: Vec<SceneRole>,
    /// Roles that Module scenes may depend on without warning.
    pub module_can_depend_on: Vec<SceneRole>,
    /// Role pairs that are always invalid.
    pub forbidden_edges: Vec<RoleDependency>,
}

impl Default for GraphDependencyPolicy {
    fn default() -> Self {
        Self::scene_architecture_defaults()
    }
}

impl GraphDependencyPolicy {
    /// Built-in SceneDeck architecture rules.
    ///
    /// These mirror the Doctor findings users already expect: Primary scenes
    /// should not depend on Debug or Raw scenes, Modules should not depend on
    /// Primary scenes, and Raw scenes should stay leaf nodes.
    pub fn scene_architecture_defaults() -> Self {
        Self {
            primary_can_depend_on: vec![
                SceneRole::Primary,
                SceneRole::Secondary,
                SceneRole::Module,
                SceneRole::Archive,
            ],
            module_can_depend_on: vec![
                SceneRole::Secondary,
                SceneRole::Module,
                SceneRole::Raw,
                SceneRole::Debug,
                SceneRole::Archive,
            ],
            forbidden_edges: vec![
                RoleDependency::new(SceneRole::Primary, SceneRole::Debug),
                RoleDependency::new(SceneRole::Module, SceneRole::Primary),
                RoleDependency::new(SceneRole::Raw, SceneRole::Primary),
                RoleDependency::new(SceneRole::Raw, SceneRole::Secondary),
                RoleDependency::new(SceneRole::Raw, SceneRole::Module),
                RoleDependency::new(SceneRole::Raw, SceneRole::Raw),
                RoleDependency::new(SceneRole::Raw, SceneRole::Debug),
                RoleDependency::new(SceneRole::Raw, SceneRole::Archive),
            ],
        }
    }

    /// Set the Primary-scene allow-list.
    pub fn primary_can_depend_on(mut self, roles: impl IntoIterator<Item = SceneRole>) -> Self {
        self.primary_can_depend_on = roles.into_iter().collect();
        self
    }

    /// Set the Module-scene allow-list.
    pub fn module_can_depend_on(mut self, roles: impl IntoIterator<Item = SceneRole>) -> Self {
        self.module_can_depend_on = roles.into_iter().collect();
        self
    }

    /// Add one forbidden role pair to the policy.
    pub fn forbid(mut self, from: SceneRole, to: SceneRole) -> Self {
        let edge = RoleDependency::new(from, to);
        if !self.forbidden_edges.contains(&edge) {
            self.forbidden_edges.push(edge);
        }
        self
    }
}

/// Immutable registry snapshot used by application services.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct SceneRegistrySnapshot {
    scenes: HashMap<SceneId, SceneMetadata>,
    graph_policy: GraphDependencyPolicy,
}

impl SceneRegistrySnapshot {
    /// Create an immutable registry snapshot from typed metadata and policy.
    pub fn new(
        scenes: HashMap<SceneId, SceneMetadata>,
        graph_policy: GraphDependencyPolicy,
    ) -> Self {
        Self {
            scenes,
            graph_policy,
        }
    }

    /// Metadata for a scene id, if it has a registry entry.
    pub fn scene_metadata(&self, scene: &str) -> Option<&SceneMetadata> {
        self.scenes.get(scene)
    }

    /// Role for a scene id, if it has a registry entry.
    pub fn scene_role(&self, scene: &str) -> Option<SceneRole> {
        self.scene_metadata(scene).map(|metadata| metadata.role)
    }

    /// Whether the registry contains metadata for `scene`.
    pub fn contains_scene(&self, scene: &str) -> bool {
        self.scenes.contains_key(scene)
    }

    /// Iterate all registered scenes and their metadata.
    pub fn scenes(&self) -> impl Iterator<Item = (&SceneId, &SceneMetadata)> {
        self.scenes.iter()
    }

    /// Graph dependency policy associated with this snapshot.
    pub const fn graph_policy(&self) -> &GraphDependencyPolicy {
        &self.graph_policy
    }
}
