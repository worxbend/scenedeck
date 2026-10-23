//! Scene architecture diagnostics over domain snapshots.
//!
//! `evaluate` produces pure rule results; `run` preserves the localized Doctor
//! page contract by formatting those results only after their stable sort.

use crate::domain::diagnostic::{Diagnostic, DiagnosticSeverity, DoctorFinding, DoctorFindingKind};
use crate::domain::graph::{EdgeStatus, SceneGraph};
use crate::domain::registry::SceneRegistrySnapshot;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use crate::services::doctor_format::format_finding;
use crate::services::graph_service::classify_edge;

/// Runs scene architecture diagnostics over domain state.
pub struct DoctorService;

impl DoctorService {
    /// Return localized diagnostics sorted most-severe first.
    pub fn run(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        graph: &SceneGraph,
    ) -> Vec<Diagnostic> {
        Self::evaluate(inventory, registry, graph)
            .into_iter()
            .map(format_finding)
            .collect()
    }

    /// Evaluate all rules without consulting localization or display labels.
    ///
    /// Most severe first; ties retain rule traversal order (including the
    /// existing registry/graph iteration order, not a new alphabetical order).
    pub fn evaluate(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        graph: &SceneGraph,
    ) -> Vec<DoctorFinding> {
        let mut findings = Vec::new();
        Self::check_role_coverage(inventory, registry, &mut findings);
        Self::check_stale_entries(inventory, registry, &mut findings);
        Self::check_protected_switchable(registry, &mut findings);
        Self::check_edges(registry, graph, &mut findings);
        Self::check_cycles(graph, &mut findings);
        findings.sort_by_key(|finding| std::cmp::Reverse(finding.severity));
        findings
    }

    /// Every OBS scene should have a role assigned.
    fn check_role_coverage(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        findings: &mut Vec<DoctorFinding>,
    ) {
        for scene in &inventory.scenes {
            if !registry.contains_scene(&scene.id) {
                findings.push(DoctorFinding {
                    severity: DiagnosticSeverity::Warning,
                    scene: scene.id.clone(),
                    kind: DoctorFindingKind::NoRole,
                });
            }
        }
    }

    /// Registry entries that no longer exist in OBS.
    fn check_stale_entries(
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
        findings: &mut Vec<DoctorFinding>,
    ) {
        for (scene_name, _) in registry.scenes() {
            if !inventory.scenes.iter().any(|scene| &scene.id == scene_name) {
                findings.push(DoctorFinding {
                    severity: DiagnosticSeverity::Info,
                    scene: scene_name.clone(),
                    kind: DoctorFindingKind::StaleEntry,
                });
            }
        }
    }

    /// Protected scenes should not be in a live-switchable role.
    fn check_protected_switchable(
        registry: &SceneRegistrySnapshot,
        findings: &mut Vec<DoctorFinding>,
    ) {
        for (name, metadata) in registry.scenes() {
            if metadata.protected && metadata.role.is_live_switchable() {
                findings.push(DoctorFinding {
                    severity: DiagnosticSeverity::Warning,
                    scene: name.clone(),
                    kind: DoctorFindingKind::ProtectedSwitchable {
                        role: metadata.role,
                    },
                });
            }
        }
    }

    /// Structural rules between connected scenes, based on their roles.
    fn check_edges(
        registry: &SceneRegistrySnapshot,
        graph: &SceneGraph,
        findings: &mut Vec<DoctorFinding>,
    ) {
        for (parent, children) in &graph.edges {
            let parent_role = registry.scene_role(parent);
            for child in children {
                let child_role = registry.scene_role(child);
                let status = classify_edge(parent_role, child_role, registry.graph_policy());
                if let Some(finding) = edge_finding(parent, parent_role, child, child_role, status)
                {
                    findings.push(finding);
                }
            }
        }
    }

    /// Detect circular scene references (A nests B … nests A).
    fn check_cycles(graph: &SceneGraph, findings: &mut Vec<DoctorFinding>) {
        for (parent, child) in graph.cycle_edges() {
            findings.push(DoctorFinding {
                severity: DiagnosticSeverity::Error,
                scene: parent.to_owned(),
                kind: DoctorFindingKind::Cycle {
                    child: child.to_owned(),
                },
            });
        }
    }
}

/// Severity depends only on the policy status, not the endpoint roles.
const fn edge_severity(status: EdgeStatus) -> DiagnosticSeverity {
    match status {
        EdgeStatus::Forbidden => DiagnosticSeverity::Error,
        EdgeStatus::Ok | EdgeStatus::Warning => DiagnosticSeverity::Warning,
    }
}

/// Retain the existing role/status precedence, including custom-policy fallbacks.
fn edge_finding(
    parent: &str,
    parent_role: Option<SceneRole>,
    child: &str,
    child_role: Option<SceneRole>,
    status: EdgeStatus,
) -> Option<DoctorFinding> {
    if status == EdgeStatus::Ok {
        return None;
    }
    // `classify_edge` returns `Ok` for an unassigned endpoint.
    let (parent_role, child_role) = (parent_role?, child_role?);
    let child = child.to_owned();
    let kind = match (parent_role, child_role, status) {
        (SceneRole::Primary, SceneRole::Debug, EdgeStatus::Forbidden) => {
            DoctorFindingKind::PrimaryDebug { child }
        }
        (SceneRole::Primary, SceneRole::Raw, EdgeStatus::Warning) => {
            DoctorFindingKind::PrimaryRaw { child }
        }
        (SceneRole::Module, SceneRole::Primary, EdgeStatus::Forbidden) => {
            DoctorFindingKind::ModulePrimary { child }
        }
        (SceneRole::Raw, _, EdgeStatus::Forbidden) => DoctorFindingKind::RawNests { child },
        (_, _, EdgeStatus::Forbidden) => DoctorFindingKind::ForbiddenEdge { child },
        _ => DoctorFindingKind::OutsidePolicy { child },
    };
    Some(DoctorFinding {
        severity: edge_severity(status),
        scene: parent.to_owned(),
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::domain::registry::{GraphDependencyPolicy, RoleDependency, SceneMetadata};
    use crate::domain::scene::Scene;

    fn scene(id: &str) -> Scene {
        Scene { id: id.to_owned() }
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
                .map(|(name, role)| (name.to_owned(), metadata(role)))
                .collect(),
            Default::default(),
        )
    }

    fn finding(
        severity: DiagnosticSeverity,
        scene: &str,
        kind: DoctorFindingKind,
    ) -> DoctorFinding {
        DoctorFinding {
            severity,
            scene: scene.to_owned(),
            kind,
        }
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

    #[test]
    fn pure_rule_result_retains_scene_and_typed_role_without_localizing() {
        let inventory = SceneInventory {
            scenes: vec![scene("Protected")],
            ..Default::default()
        };
        let registry = SceneRegistrySnapshot::new(
            HashMap::from([(
                "Protected".into(),
                SceneMetadata {
                    protected: true,
                    ..metadata(SceneRole::Primary)
                },
            )]),
            Default::default(),
        );
        assert_eq!(
            DoctorService::evaluate(&inventory, &registry, &SceneGraph::default()),
            vec![DoctorFinding {
                severity: DiagnosticSeverity::Warning,
                scene: "Protected".into(),
                kind: DoctorFindingKind::ProtectedSwitchable {
                    role: SceneRole::Primary
                },
            }]
        );
    }

    #[test]
    fn coverage_stale_and_protected_keep_rule_order() {
        let inventory = SceneInventory {
            scenes: ["Missing Z", "Missing A", "Protected"].map(scene).into(),
            ..Default::default()
        };
        let registry = SceneRegistrySnapshot::new(
            HashMap::from([
                (
                    "Protected".into(),
                    SceneMetadata {
                        protected: true,
                        ..metadata(SceneRole::Primary)
                    },
                ),
                ("Stale".into(), metadata(SceneRole::Raw)),
            ]),
            Default::default(),
        );
        assert_eq!(
            DoctorService::evaluate(&inventory, &registry, &SceneGraph::default()),
            vec![
                finding(
                    DiagnosticSeverity::Warning,
                    "Missing Z",
                    DoctorFindingKind::NoRole
                ),
                finding(
                    DiagnosticSeverity::Warning,
                    "Missing A",
                    DoctorFindingKind::NoRole
                ),
                finding(
                    DiagnosticSeverity::Warning,
                    "Protected",
                    DoctorFindingKind::ProtectedSwitchable {
                        role: SceneRole::Primary
                    }
                ),
                finding(
                    DiagnosticSeverity::Info,
                    "Stale",
                    DoctorFindingKind::StaleEntry
                ),
            ]
        );
    }

    #[test]
    fn edge_rules_preserve_precedence_severity_and_literal_arguments() {
        use DiagnosticSeverity::{Error, Warning};
        use EdgeStatus::Forbidden;
        use SceneRole::{Debug, Module, Primary, Raw, Secondary};
        let child = "Child <&> 日本語";
        for (parent_role, child_role, status, severity, kind) in [
            (
                Primary,
                Debug,
                Forbidden,
                Error,
                DoctorFindingKind::PrimaryDebug {
                    child: child.into(),
                },
            ),
            (
                Primary,
                Raw,
                EdgeStatus::Warning,
                Warning,
                DoctorFindingKind::PrimaryRaw {
                    child: child.into(),
                },
            ),
            (
                Module,
                Primary,
                Forbidden,
                Error,
                DoctorFindingKind::ModulePrimary {
                    child: child.into(),
                },
            ),
            (
                Raw,
                Module,
                Forbidden,
                Error,
                DoctorFindingKind::RawNests {
                    child: child.into(),
                },
            ),
            (
                Secondary,
                Debug,
                Forbidden,
                Error,
                DoctorFindingKind::ForbiddenEdge {
                    child: child.into(),
                },
            ),
            (
                Primary,
                Raw,
                Forbidden,
                Error,
                DoctorFindingKind::ForbiddenEdge {
                    child: child.into(),
                },
            ),
            (
                Primary,
                Debug,
                EdgeStatus::Warning,
                Warning,
                DoctorFindingKind::OutsidePolicy {
                    child: child.into(),
                },
            ),
            (
                Module,
                Raw,
                EdgeStatus::Warning,
                Warning,
                DoctorFindingKind::OutsidePolicy {
                    child: child.into(),
                },
            ),
        ] {
            assert_eq!(
                edge_finding(
                    "Parent <&>",
                    Some(parent_role),
                    child,
                    Some(child_role),
                    status
                ),
                Some(finding(severity, "Parent <&>", kind))
            );
        }
        assert!(edge_finding("P", Some(Primary), "C", Some(Module), EdgeStatus::Ok).is_none());
        assert!(edge_finding("P", None, "C", Some(Module), Forbidden).is_none());
        assert!(edge_finding("P", Some(Primary), "C", None, EdgeStatus::Warning).is_none());
    }

    #[test]
    fn custom_policy_controls_fallbacks_and_stable_severity_sort() {
        let inventory = SceneInventory {
            scenes: ["P", "D", "R", "M"].map(scene).into(),
            ..Default::default()
        };
        let registry = SceneRegistrySnapshot::new(
            [
                ("P", SceneRole::Primary),
                ("D", SceneRole::Debug),
                ("R", SceneRole::Raw),
                ("M", SceneRole::Module),
            ]
            .into_iter()
            .map(|(name, role)| (name.into(), metadata(role)))
            .collect(),
            GraphDependencyPolicy {
                primary_can_depend_on: vec![SceneRole::Module],
                module_can_depend_on: vec![],
                forbidden_edges: vec![RoleDependency::new(SceneRole::Primary, SceneRole::Raw)],
            },
        );
        let mut graph = SceneGraph::default();
        graph.edges.insert(
            "P".into(),
            vec!["D".into(), "R".into(), "M".into(), "Unknown".into()],
        );
        assert_eq!(
            DoctorService::evaluate(&inventory, &registry, &graph),
            vec![
                finding(
                    DiagnosticSeverity::Error,
                    "P",
                    DoctorFindingKind::ForbiddenEdge { child: "R".into() }
                ),
                finding(
                    DiagnosticSeverity::Warning,
                    "P",
                    DoctorFindingKind::OutsidePolicy { child: "D".into() }
                ),
            ]
        );
    }

    #[test]
    fn self_cycle_and_disconnected_cycles_keep_representative_order() {
        let mut graph = SceneGraph::default();
        graph.edges.insert("Self".into(), vec!["Self".into()]);
        graph.edges.insert("A".into(), vec!["B".into(), "B".into()]);
        graph.edges.insert("B".into(), vec!["A".into()]);
        graph.edges.insert("C".into(), vec!["D".into()]);
        graph.edges.insert("D".into(), vec!["C".into()]);
        let registry = registry(["A", "B", "C", "D", "Self"].map(|name| (name, SceneRole::Module)));
        let inventory = SceneInventory {
            scenes: ["A", "B", "C", "D", "Self"].map(scene).into(),
            ..Default::default()
        };
        let expected = [
            ("A", "B"),
            ("B", "A"),
            ("C", "D"),
            ("D", "C"),
            ("Self", "Self"),
        ]
        .map(|(parent, child)| {
            finding(
                DiagnosticSeverity::Error,
                parent,
                DoctorFindingKind::Cycle {
                    child: child.into(),
                },
            )
        });
        assert_eq!(
            DoctorService::evaluate(&inventory, &registry, &graph),
            expected
        );
    }

    #[test]
    fn empty_and_assigned_non_switchable_protected_scenes_have_no_findings() {
        assert!(DoctorService::evaluate(
            &SceneInventory::default(),
            &SceneRegistrySnapshot::default(),
            &SceneGraph::default()
        )
        .is_empty());
        for role in SceneRole::ALL {
            for protected in [false, true] {
                let inventory = SceneInventory {
                    scenes: vec![scene("Scene")],
                    ..Default::default()
                };
                let registry = SceneRegistrySnapshot::new(
                    HashMap::from([(
                        "Scene".into(),
                        SceneMetadata {
                            protected,
                            ..metadata(role)
                        },
                    )]),
                    Default::default(),
                );
                let expected = if protected && role == SceneRole::Primary {
                    vec![finding(
                        DiagnosticSeverity::Warning,
                        "Scene",
                        DoctorFindingKind::ProtectedSwitchable { role },
                    )]
                } else {
                    vec![]
                };
                assert_eq!(
                    DoctorService::evaluate(&inventory, &registry, &SceneGraph::default()),
                    expected
                );
            }
        }
    }

    #[test]
    fn registry_and_graph_ties_retain_existing_iteration_order() {
        let registry = SceneRegistrySnapshot::new(
            ["Z", "A", "M"]
                .into_iter()
                .map(|name| {
                    (
                        name.into(),
                        SceneMetadata {
                            protected: true,
                            ..metadata(SceneRole::Primary)
                        },
                    )
                })
                .collect(),
            Default::default(),
        );
        let expected: Vec<_> = registry
            .scenes()
            .map(|(name, _)| {
                finding(
                    DiagnosticSeverity::Warning,
                    name,
                    DoctorFindingKind::ProtectedSwitchable {
                        role: SceneRole::Primary,
                    },
                )
            })
            .chain(registry.scenes().map(|(name, _)| {
                finding(
                    DiagnosticSeverity::Info,
                    name,
                    DoctorFindingKind::StaleEntry,
                )
            }))
            .collect();
        assert_eq!(
            DoctorService::evaluate(
                &SceneInventory::default(),
                &registry,
                &SceneGraph::default()
            ),
            expected
        );

        let registry = registry_for_edges();
        let inventory = SceneInventory {
            scenes: ["P1", "P2", "D1", "D2"].map(scene).into(),
            ..Default::default()
        };
        let graph = SceneGraph {
            edges: HashMap::from([
                ("P1".into(), vec!["D2".into(), "D1".into(), "D2".into()]),
                ("P2".into(), vec!["D1".into(), "D2".into()]),
            ]),
        };
        let expected: Vec<_> = graph
            .edges
            .iter()
            .flat_map(|(parent, children)| {
                children.iter().map(|child| {
                    finding(
                        DiagnosticSeverity::Error,
                        parent,
                        DoctorFindingKind::PrimaryDebug {
                            child: child.clone(),
                        },
                    )
                })
            })
            .collect();
        assert_eq!(
            DoctorService::evaluate(&inventory, &registry, &graph),
            expected
        );
    }

    fn registry_for_edges() -> SceneRegistrySnapshot {
        registry([
            ("P1", SceneRole::Primary),
            ("P2", SceneRole::Primary),
            ("D1", SceneRole::Debug),
            ("D2", SceneRole::Debug),
        ])
    }

    #[test]
    fn default_policy_flags_each_original_edge_example() {
        for (parent, child, severity, kind) in [
            (
                SceneRole::Module,
                SceneRole::Primary,
                DiagnosticSeverity::Error,
                DoctorFindingKind::ModulePrimary { child: "C".into() },
            ),
            (
                SceneRole::Primary,
                SceneRole::Raw,
                DiagnosticSeverity::Warning,
                DoctorFindingKind::PrimaryRaw { child: "C".into() },
            ),
            (
                SceneRole::Raw,
                SceneRole::Module,
                DiagnosticSeverity::Error,
                DoctorFindingKind::RawNests { child: "C".into() },
            ),
            (
                SceneRole::Primary,
                SceneRole::Debug,
                DiagnosticSeverity::Error,
                DoctorFindingKind::PrimaryDebug { child: "C".into() },
            ),
        ] {
            let registry = registry([("P", parent), ("C", child)]);
            let inventory = SceneInventory {
                scenes: ["P", "C"].map(scene).into(),
                ..Default::default()
            };
            let graph = SceneGraph {
                edges: HashMap::from([("P".into(), vec!["C".into()])]),
            };
            assert_eq!(
                DoctorService::evaluate(&inventory, &registry, &graph),
                vec![finding(severity, "P", kind)]
            );
        }
    }
}
