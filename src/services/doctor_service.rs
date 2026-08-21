//! Scene architecture diagnostics.
//!
//! Pure functions over the scene inventory, the local registry, and the
//! dependency graph.  No OBS or GTK types.  Produces a list of `Diagnostic`s
//! sorted most-severe first.

use i18n_embed_fl::fl;

use crate::domain::diagnostic::{Diagnostic, DiagnosticSeverity};
use crate::domain::graph::{EdgeStatus, SceneGraph};
use crate::domain::registry::SceneRegistrySnapshot;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::graph_service::classify_edge;

/// Runs scene architecture diagnostics over domain state.
pub struct DoctorService;

impl DoctorService {
    /// Return diagnostics sorted most-severe first.
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
                    message: fl!(LANGUAGE_LOADER, "doctor-no-role"),
                    suggestion: Some(fl!(LANGUAGE_LOADER, "doctor-no-role-suggestion")),
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
                    message: fl!(LANGUAGE_LOADER, "doctor-stale-entry"),
                    suggestion: Some(fl!(LANGUAGE_LOADER, "doctor-stale-entry-suggestion")),
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
                    message: fl!(
                        LANGUAGE_LOADER,
                        "doctor-protected-switchable",
                        role = metadata.role.label()
                    ),
                    suggestion: Some(fl!(
                        LANGUAGE_LOADER,
                        "doctor-protected-switchable-suggestion"
                    )),
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
                let status = classify_edge(parent_role, child_role, registry.graph_policy());
                let diagnostic = edge_diagnostic(parent, parent_role, child, child_role, status);
                if let Some(diag) = diagnostic {
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
                            message: fl!(
                                LANGUAGE_LOADER,
                                "doctor-cycle",
                                parent = parent.as_str(),
                                child = child.as_str()
                            ),
                            suggestion: Some(fl!(LANGUAGE_LOADER, "doctor-cycle-suggestion")),
                        });
                    }
                }
            }
        }
    }
}

/// Severity a non-`Ok` edge contributes, stated once for every edge rule.
///
/// Every edge rule below used to repeat this alongside its message, which made
/// the two look independent when they are not: how bad an edge is depends only
/// on whether the policy forbids it, never on which roles are involved. The
/// variants are spelled out rather than caught by a `_` arm, so adding a
/// fourth `EdgeStatus` fails to compile here instead of silently landing on a
/// default.
const fn edge_severity(status: EdgeStatus) -> DiagnosticSeverity {
    match status {
        EdgeStatus::Forbidden => DiagnosticSeverity::Error,
        EdgeStatus::Ok | EdgeStatus::Warning => DiagnosticSeverity::Warning,
    }
}

/// Classify a single parent→child role relationship into an optional diagnostic.
fn edge_diagnostic(
    parent: &str,
    parent_role: Option<SceneRole>,
    child: &str,
    child_role: Option<SceneRole>,
    status: EdgeStatus,
) -> Option<Diagnostic> {
    let (pr, cr) = (parent_role?, child_role?);
    // An allowed edge has nothing to report. This was previously the first arm
    // of the match below, where a `return` hidden among the classification
    // rules read as if it were one of them.
    if status == EdgeStatus::Ok {
        return None;
    }

    // Matching on the status alongside the roles replaces five repeated
    // `if status == ...` guards, so each rule reads as the single triple it
    // applies to. Arm order is unchanged, so the same rule still wins.
    let (message, suggestion) = match (pr, cr, status) {
        (SceneRole::Primary, SceneRole::Debug, EdgeStatus::Forbidden) => (
            fl!(LANGUAGE_LOADER, "doctor-edge-primary-debug", child = child),
            fl!(LANGUAGE_LOADER, "doctor-edge-primary-debug-suggestion"),
        ),
        (SceneRole::Primary, SceneRole::Raw, EdgeStatus::Warning) => (
            fl!(LANGUAGE_LOADER, "doctor-edge-primary-raw", child = child),
            fl!(LANGUAGE_LOADER, "doctor-edge-primary-raw-suggestion"),
        ),
        (SceneRole::Module, SceneRole::Primary, EdgeStatus::Forbidden) => (
            fl!(LANGUAGE_LOADER, "doctor-edge-module-primary", child = child),
            fl!(LANGUAGE_LOADER, "doctor-edge-module-primary-suggestion"),
        ),
        (SceneRole::Raw, _, EdgeStatus::Forbidden) => (
            fl!(LANGUAGE_LOADER, "doctor-edge-raw-nests", child = child),
            fl!(LANGUAGE_LOADER, "doctor-edge-raw-nests-suggestion"),
        ),
        (_, _, EdgeStatus::Forbidden) => (
            fl!(LANGUAGE_LOADER, "doctor-edge-forbidden", child = child),
            fl!(LANGUAGE_LOADER, "doctor-edge-adjust-suggestion"),
        ),
        _ => (
            fl!(LANGUAGE_LOADER, "doctor-edge-outside-policy", child = child),
            fl!(LANGUAGE_LOADER, "doctor-edge-adjust-suggestion"),
        ),
    };

    Some(Diagnostic {
        severity: edge_severity(status),
        scene: Some(parent.to_string()),
        message,
        suggestion: Some(suggestion),
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
            previous_id: None,
        };
        let registry = registry([("Mod", SceneRole::Module), ("Main", SceneRole::Primary)]);

        let mut graph = SceneGraph::default();
        graph.edges.insert("Mod".into(), vec!["Main".into()]);

        let diags = DoctorService::run(&inventory, &registry, &graph);
        assert!(diags.iter().any(|d| d.severity == DiagnosticSeverity::Error
            && d.message.contains("inverting the hierarchy")));
    }

    #[test]
    fn flags_primary_depending_on_raw_from_shared_policy() {
        let inventory = SceneInventory {
            scenes: vec![scene("Main"), scene("Camera")],
            current_id: None,
            previous_id: None,
        };
        let registry = registry([("Main", SceneRole::Primary), ("Camera", SceneRole::Raw)]);

        let mut graph = SceneGraph::default();
        graph.edges.insert("Main".into(), vec!["Camera".into()]);

        let diags = DoctorService::run(&inventory, &registry, &graph);
        assert!(diags
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Warning
                && d.message.contains("directly wraps a Raw source")));
    }

    #[test]
    fn flags_raw_scene_with_nested_child_from_shared_policy() {
        let inventory = SceneInventory {
            scenes: vec![scene("Camera"), scene("Overlay")],
            current_id: None,
            previous_id: None,
        };
        let registry = registry([("Camera", SceneRole::Raw), ("Overlay", SceneRole::Module)]);

        let mut graph = SceneGraph::default();
        graph.edges.insert("Camera".into(), vec!["Overlay".into()]);

        let diags = DoctorService::run(&inventory, &registry, &graph);
        assert!(diags.iter().any(|d| d.severity == DiagnosticSeverity::Error
            && d.message.contains("Raw scene nests another scene")));
    }

    #[test]
    fn detects_cycle() {
        let inventory = SceneInventory {
            scenes: vec![scene("A"), scene("B")],
            current_id: None,
            previous_id: None,
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
