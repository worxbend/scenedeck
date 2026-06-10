//! Scene architecture diagnostics.
//!
//! Pure functions over the scene inventory, the local registry, and the
//! dependency graph.  No OBS or GTK types.  Produces a list of `Diagnostic`s
//! sorted most-severe first.

use crate::domain::diagnostic::{Diagnostic, DiagnosticSeverity};
use crate::domain::graph::SceneGraph;
use crate::domain::registry::SceneRegistrySnapshot;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;

pub struct DoctorService;

impl DoctorService {
    pub fn run(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        graph: &SceneGraph,
    ) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        Self::check_role_coverage(inventory, registry, &mut diags);
        Self::check_stale_entries(inventory, registry, &mut diags);
        Self::check_protected_switchable(registry, &mut diags);
        Self::check_edges(registry, graph, &mut diags);
        Self::check_cycles(graph, &mut diags);

        // Most severe first; stable within a severity.
        diags.sort_by_key(|d| std::cmp::Reverse(d.severity));
        diags
    }

    fn role_of(registry: &SceneRegistrySnapshot, scene: &str) -> Option<SceneRole> {
        registry.scene_role(scene)
    }

    /// Every OBS scene should have a role assigned.
    fn check_role_coverage(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        diags: &mut Vec<Diagnostic>,
    ) {
        for scene in &inventory.scenes {
            if !registry.contains_scene(&scene.id) {
                diags.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    scene: Some(scene.id.clone()),
                    message: "Scene has no role assigned in the local registry.".to_string(),
                    suggestion: Some("Open Inventory and assign a role.".to_string()),
                });
            }
        }
    }

    /// Registry entries that no longer exist in OBS.
    fn check_stale_entries(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        diags: &mut Vec<Diagnostic>,
    ) {
        for (scene_name, _) in registry.scenes() {
            if !inventory.scenes.iter().any(|s| &s.id == scene_name) {
                diags.push(Diagnostic {
                    severity: DiagnosticSeverity::Info,
                    scene: Some(scene_name.clone()),
                    message: "Registry entry references a scene not found in OBS.".to_string(),
                    suggestion: Some("Remove the entry from Inventory.".to_string()),
                });
            }
        }
    }

    /// Protected scenes should not be in a live-switchable role.
    fn check_protected_switchable(registry: &SceneRegistrySnapshot, diags: &mut Vec<Diagnostic>) {
        for (name, metadata) in registry.scenes() {
            if metadata.protected && metadata.role.is_live_switchable() {
                diags.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    scene: Some(name.clone()),
                    message: format!(
                        "Protected scene is in the switchable '{}' role.",
                        metadata.role.label()
                    ),
                    suggestion: Some(
                        "Protected scenes are usually building blocks; consider Module or Raw."
                            .to_string(),
                    ),
                });
            }
        }
    }

    /// Structural rules between connected scenes, based on their roles.
    fn check_edges(
        registry: &SceneRegistrySnapshot,
        graph: &SceneGraph,
        diags: &mut Vec<Diagnostic>,
    ) {
        for (parent, children) in &graph.edges {
            let parent_role = Self::role_of(registry, parent);
            for child in children {
                let child_role = Self::role_of(registry, child);
                if let Some(diag) = edge_diagnostic(parent, parent_role, child, child_role) {
                    diags.push(diag);
                }
            }
        }
    }

    /// Detect circular scene references (A nests B … nests A).
    fn check_cycles(graph: &SceneGraph, diags: &mut Vec<Diagnostic>) {
        let mut reported: std::collections::HashSet<String> = std::collections::HashSet::new();

        for parent in graph.edges.keys() {
            for child in graph.children(parent) {
                // A cycle exists if `child` can reach back to `parent`.
                if graph.is_reachable(child, parent) {
                    // Report each cyclic scene once.
                    if reported.insert(parent.clone()) {
                        diags.push(Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            scene: Some(parent.clone()),
                            message: format!(
                                "Circular scene reference involving '{parent}' and '{child}'."
                            ),
                            suggestion: Some(
                                "Remove the nested-scene loop; OBS cannot render cycles."
                                    .to_string(),
                            ),
                        });
                    }
                }
            }
        }
    }
}

/// Classify a single parent→child role relationship into an optional diagnostic.
fn edge_diagnostic(
    parent: &str,
    parent_role: Option<SceneRole>,
    child: &str,
    child_role: Option<SceneRole>,
) -> Option<Diagnostic> {
    let (pr, cr) = (parent_role?, child_role?);

    let (severity, message, suggestion) = match (pr, cr) {
        (SceneRole::Primary, SceneRole::Debug) => (
            DiagnosticSeverity::Error,
            "Primary scene depends on a Debug scene.",
            "Remove the Debug scene from the live path before going live.",
        ),
        (SceneRole::Primary, SceneRole::Raw) => (
            DiagnosticSeverity::Warning,
            "Primary scene directly wraps a Raw source.",
            "Wrap the Raw source in a Module scene for reuse and clarity.",
        ),
        (SceneRole::Module, SceneRole::Primary) => (
            DiagnosticSeverity::Error,
            "Module depends on a Primary scene, inverting the hierarchy.",
            "Modules should be building blocks, not consumers of Primary scenes.",
        ),
        (SceneRole::Raw, _) => (
            DiagnosticSeverity::Error,
            "Raw scene nests another scene.",
            "Raw scenes should be leaf source wrappers with no nested scenes.",
        ),
        _ => return None,
    };

    Some(Diagnostic {
        severity,
        scene: Some(parent.to_string()),
        message: format!("{message} (→ '{child}')"),
        suggestion: Some(suggestion.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::domain::registry::{SceneMetadata, SceneRegistrySnapshot};
    use crate::domain::scene::{Scene, SceneInventory};

    fn scene(id: &str) -> Scene {
        Scene {
            id: id.to_string(),
            name: id.to_string(),
            role: None,
        }
    }

    fn metadata(role: SceneRole) -> SceneMetadata {
        SceneMetadata {
            role,
            tags: Vec::new(),
            protected: false,
        }
    }

    fn registry(
        entries: impl IntoIterator<Item = (&'static str, SceneRole)>,
    ) -> SceneRegistrySnapshot {
        SceneRegistrySnapshot::new(
            entries
                .into_iter()
                .map(|(scene, role)| (scene.to_string(), metadata(role)))
                .collect::<HashMap<_, _>>(),
            Default::default(),
        )
    }

    #[test]
    fn flags_module_depending_on_primary() {
        let inventory = SceneInventory {
            scenes: vec![scene("Mod"), scene("Main")],
            current_id: None,
        };
        let registry = registry([("Mod", SceneRole::Module), ("Main", SceneRole::Primary)]);

        let mut graph = SceneGraph::default();
        graph.edges.insert("Mod".into(), vec!["Main".into()]);

        let diags = DoctorService::run(&inventory, &registry, &graph);
        assert!(diags.iter().any(|d| d.severity == DiagnosticSeverity::Error
            && d.message.contains("inverting the hierarchy")));
    }

    #[test]
    fn detects_cycle() {
        let inventory = SceneInventory {
            scenes: vec![scene("A"), scene("B")],
            current_id: None,
        };
        let registry = registry([("A", SceneRole::Module), ("B", SceneRole::Module)]);

        let mut graph = SceneGraph::default();
        graph.edges.insert("A".into(), vec!["B".into()]);
        graph.edges.insert("B".into(), vec!["A".into()]);

        let diags = DoctorService::run(&inventory, &registry, &graph);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("Circular scene reference")));
    }
}
