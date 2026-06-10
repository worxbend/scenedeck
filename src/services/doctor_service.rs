//! Scene architecture diagnostics.
//!
//! Pure functions over the scene inventory, the local registry, and the
//! dependency graph.  No OBS or GTK types.  Produces a list of `Diagnostic`s
//! sorted most-severe first.

use crate::domain::diagnostic::{Diagnostic, DiagnosticSeverity};
use crate::domain::graph::SceneGraph;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use crate::storage::registry::SceneRegistry;

pub struct DoctorService;

impl DoctorService {
    pub fn run(
        inventory: &SceneInventory,
        registry: &SceneRegistry,
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

    fn role_of(registry: &SceneRegistry, scene: &str) -> Option<SceneRole> {
        registry.scenes.get(scene).map(|e| e.role)
    }

    /// Every OBS scene should have a role assigned.
    fn check_role_coverage(
        inventory: &SceneInventory,
        registry: &SceneRegistry,
        diags: &mut Vec<Diagnostic>,
    ) {
        for scene in &inventory.scenes {
            if !registry.scenes.contains_key(&scene.id) {
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
        registry: &SceneRegistry,
        diags: &mut Vec<Diagnostic>,
    ) {
        for scene_name in registry.scenes.keys() {
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
    fn check_protected_switchable(registry: &SceneRegistry, diags: &mut Vec<Diagnostic>) {
        for (name, entry) in &registry.scenes {
            if entry.protected && entry.role.is_live_switchable() {
                diags.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    scene: Some(name.clone()),
                    message: format!(
                        "Protected scene is in the switchable '{}' role.",
                        entry.role.label()
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
    fn check_edges(registry: &SceneRegistry, graph: &SceneGraph, diags: &mut Vec<Diagnostic>) {
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
    use crate::domain::scene::{Scene, SceneInventory};
    use crate::storage::registry::SceneEntry;

    fn scene(id: &str) -> Scene {
        Scene {
            id: id.to_string(),
            name: id.to_string(),
            role: None,
        }
    }

    fn entry(role: SceneRole) -> SceneEntry {
        SceneEntry {
            role,
            tags: Vec::new(),
            protected: false,
        }
    }

    #[test]
    fn flags_module_depending_on_primary() {
        let inventory = SceneInventory {
            scenes: vec![scene("Mod"), scene("Main")],
            current_id: None,
        };
        let mut registry = SceneRegistry::default();
        registry
            .scenes
            .insert("Mod".into(), entry(SceneRole::Module));
        registry
            .scenes
            .insert("Main".into(), entry(SceneRole::Primary));

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
        let mut registry = SceneRegistry::default();
        registry.scenes.insert("A".into(), entry(SceneRole::Module));
        registry.scenes.insert("B".into(), entry(SceneRole::Module));

        let mut graph = SceneGraph::default();
        graph.edges.insert("A".into(), vec!["B".into()]);
        graph.edges.insert("B".into(), vec!["A".into()]);

        let diags = DoctorService::run(&inventory, &registry, &graph);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("Circular scene reference")));
    }
}
