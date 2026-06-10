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
    pub role: SceneRole,
    pub tags: Vec<String>,
    pub protected: bool,
}

/// Typed role dependency rule.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RoleDependency {
    pub from: SceneRole,
    pub to: SceneRole,
}

impl RoleDependency {
    pub const fn new(from: SceneRole, to: SceneRole) -> Self {
        Self { from, to }
    }
}

/// Graph validation policy used by Doctor and the Graph page.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct GraphDependencyPolicy {
    pub primary_can_depend_on: Vec<SceneRole>,
    pub module_can_depend_on: Vec<SceneRole>,
    pub forbidden_edges: Vec<RoleDependency>,
}

impl GraphDependencyPolicy {
    pub fn primary_can_depend_on(mut self, roles: impl IntoIterator<Item = SceneRole>) -> Self {
        self.primary_can_depend_on = roles.into_iter().collect();
        self
    }

    pub fn module_can_depend_on(mut self, roles: impl IntoIterator<Item = SceneRole>) -> Self {
        self.module_can_depend_on = roles.into_iter().collect();
        self
    }

    pub fn forbid(mut self, from: SceneRole, to: SceneRole) -> Self {
        self.forbidden_edges.push(RoleDependency::new(from, to));
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
    pub fn new(
        scenes: HashMap<SceneId, SceneMetadata>,
        graph_policy: GraphDependencyPolicy,
    ) -> Self {
        Self {
            scenes,
            graph_policy,
        }
    }

    pub fn scene_metadata(&self, scene: &str) -> Option<&SceneMetadata> {
        self.scenes.get(scene)
    }

    pub fn scene_role(&self, scene: &str) -> Option<SceneRole> {
        self.scene_metadata(scene).map(|metadata| metadata.role)
    }

    pub fn contains_scene(&self, scene: &str) -> bool {
        self.scenes.contains_key(scene)
    }

    pub fn scenes(&self) -> impl Iterator<Item = (&SceneId, &SceneMetadata)> {
        self.scenes.iter()
    }

    pub const fn graph_policy(&self) -> &GraphDependencyPolicy {
        &self.graph_policy
    }
}
