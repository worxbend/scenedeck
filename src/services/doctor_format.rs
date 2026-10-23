//! Presentation phase for unlocalized Doctor rule results.
//!
//! Keep the shared loader (including its English fallback) and localized role
//! labels here; rule evaluation must not consult either.

use i18n_embed_fl::fl;

use crate::domain::diagnostic::{Diagnostic, DoctorFinding, DoctorFindingKind};
use crate::infra::i18n::LANGUAGE_LOADER;

/// Localize one finding without reclassifying or reordering it.
pub(super) fn format_finding(finding: DoctorFinding) -> Diagnostic {
    let (message, suggestion) = match finding.kind {
        DoctorFindingKind::NoRole => (
            fl!(LANGUAGE_LOADER, "doctor-no-role"),
            fl!(LANGUAGE_LOADER, "doctor-no-role-suggestion"),
        ),
        DoctorFindingKind::StaleEntry => (
            fl!(LANGUAGE_LOADER, "doctor-stale-entry"),
            fl!(LANGUAGE_LOADER, "doctor-stale-entry-suggestion"),
        ),
        DoctorFindingKind::ProtectedSwitchable { role } => (
            fl!(
                LANGUAGE_LOADER,
                "doctor-protected-switchable",
                role = role.label()
            ),
            fl!(LANGUAGE_LOADER, "doctor-protected-switchable-suggestion"),
        ),
        DoctorFindingKind::PrimaryDebug { child } => (
            fl!(
                LANGUAGE_LOADER,
                "doctor-edge-primary-debug",
                child = child.as_str()
            ),
            fl!(LANGUAGE_LOADER, "doctor-edge-primary-debug-suggestion"),
        ),
        DoctorFindingKind::PrimaryRaw { child } => (
            fl!(
                LANGUAGE_LOADER,
                "doctor-edge-primary-raw",
                child = child.as_str()
            ),
            fl!(LANGUAGE_LOADER, "doctor-edge-primary-raw-suggestion"),
        ),
        DoctorFindingKind::ModulePrimary { child } => (
            fl!(
                LANGUAGE_LOADER,
                "doctor-edge-module-primary",
                child = child.as_str()
            ),
            fl!(LANGUAGE_LOADER, "doctor-edge-module-primary-suggestion"),
        ),
        DoctorFindingKind::RawNests { child } => (
            fl!(
                LANGUAGE_LOADER,
                "doctor-edge-raw-nests",
                child = child.as_str()
            ),
            fl!(LANGUAGE_LOADER, "doctor-edge-raw-nests-suggestion"),
        ),
        DoctorFindingKind::ForbiddenEdge { child } => (
            fl!(
                LANGUAGE_LOADER,
                "doctor-edge-forbidden",
                child = child.as_str()
            ),
            fl!(LANGUAGE_LOADER, "doctor-edge-adjust-suggestion"),
        ),
        DoctorFindingKind::OutsidePolicy { child } => (
            fl!(
                LANGUAGE_LOADER,
                "doctor-edge-outside-policy",
                child = child.as_str()
            ),
            fl!(LANGUAGE_LOADER, "doctor-edge-adjust-suggestion"),
        ),
        DoctorFindingKind::Cycle { child } => (
            fl!(
                LANGUAGE_LOADER,
                "doctor-cycle",
                parent = finding.scene.as_str(),
                child = child.as_str()
            ),
            fl!(LANGUAGE_LOADER, "doctor-cycle-suggestion"),
        ),
    };
    Diagnostic {
        severity: finding.severity,
        scene: Some(finding.scene),
        message,
        suggestion: Some(suggestion),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::domain::diagnostic::DiagnosticSeverity;
    use crate::domain::role::SceneRole;

    fn assert_localized(
        kind: DoctorFindingKind,
        message: &str,
        arguments: &[(&str, &str)],
        suggestion: &str,
    ) {
        let diagnostic = format_finding(DoctorFinding {
            severity: DiagnosticSeverity::Warning,
            scene: "Parent <&> 日本語".into(),
            kind,
        });
        assert_eq!(diagnostic.severity, DiagnosticSeverity::Warning);
        assert_eq!(diagnostic.scene.as_deref(), Some("Parent <&> 日本語"));
        assert_eq!(
            diagnostic.message,
            LANGUAGE_LOADER.get_args(
                message,
                arguments.iter().copied().collect::<HashMap<_, _>>()
            )
        );
        assert_eq!(diagnostic.suggestion, Some(LANGUAGE_LOADER.get(suggestion)));
        assert_ne!(
            diagnostic.message, message,
            "message must resolve through the loader"
        );
        assert_ne!(diagnostic.suggestion.as_deref(), Some(suggestion));
    }

    #[test]
    fn coverage_and_stale_findings_keep_message_and_suggestion_keys() {
        assert_localized(
            DoctorFindingKind::NoRole,
            "doctor-no-role",
            &[],
            "doctor-no-role-suggestion",
        );
        assert_localized(
            DoctorFindingKind::StaleEntry,
            "doctor-stale-entry",
            &[],
            "doctor-stale-entry-suggestion",
        );
    }

    #[test]
    fn protected_finding_localizes_the_typed_role_only_at_presentation() {
        assert_localized(
            DoctorFindingKind::ProtectedSwitchable {
                role: SceneRole::Primary,
            },
            "doctor-protected-switchable",
            &[("role", &SceneRole::Primary.label())],
            "doctor-protected-switchable-suggestion",
        );
    }

    #[test]
    fn every_edge_finding_keeps_message_arguments_and_suggestion_keys() {
        let child = "Child <&> 日本語";
        for (kind, key, suggestion) in [
            (
                DoctorFindingKind::PrimaryDebug {
                    child: child.into(),
                },
                "doctor-edge-primary-debug",
                "doctor-edge-primary-debug-suggestion",
            ),
            (
                DoctorFindingKind::PrimaryRaw {
                    child: child.into(),
                },
                "doctor-edge-primary-raw",
                "doctor-edge-primary-raw-suggestion",
            ),
            (
                DoctorFindingKind::ModulePrimary {
                    child: child.into(),
                },
                "doctor-edge-module-primary",
                "doctor-edge-module-primary-suggestion",
            ),
            (
                DoctorFindingKind::RawNests {
                    child: child.into(),
                },
                "doctor-edge-raw-nests",
                "doctor-edge-raw-nests-suggestion",
            ),
            (
                DoctorFindingKind::ForbiddenEdge {
                    child: child.into(),
                },
                "doctor-edge-forbidden",
                "doctor-edge-adjust-suggestion",
            ),
            (
                DoctorFindingKind::OutsidePolicy {
                    child: child.into(),
                },
                "doctor-edge-outside-policy",
                "doctor-edge-adjust-suggestion",
            ),
        ] {
            assert_localized(kind, key, &[("child", child)], suggestion);
        }
    }

    fn run_contract_fixture() -> Vec<Diagnostic> {
        use crate::domain::graph::SceneGraph;
        use crate::domain::registry::{SceneMetadata, SceneRegistrySnapshot};
        use crate::domain::scene::{Scene, SceneInventory};
        use crate::services::doctor_service::DoctorService;

        let inventory = SceneInventory {
            scenes: ["Missing Z", "Missing A", "P", "D", "M", "R"]
                .map(|id| Scene { id: id.into() })
                .into(),
            ..Default::default()
        };
        let registry = SceneRegistrySnapshot::new(
            [
                ("P", SceneRole::Primary),
                ("D", SceneRole::Debug),
                ("M", SceneRole::Module),
                ("R", SceneRole::Raw),
                ("Stale", SceneRole::Raw),
            ]
            .into_iter()
            .map(|(name, role)| {
                (
                    name.into(),
                    SceneMetadata {
                        role,
                        tags: vec![],
                        protected: name == "P",
                    },
                )
            })
            .collect(),
            Default::default(),
        );
        let graph = SceneGraph {
            edges: HashMap::from([
                ("P".into(), vec!["R".into(), "D".into()]),
                ("M".into(), vec!["M".into()]),
            ]),
        };
        let diagnostics = DoctorService::run(&inventory, &registry, &graph);
        use DiagnosticSeverity::{Error, Info, Warning};
        let expected = [
            (
                Error,
                "P",
                "doctor-edge-primary-debug",
                vec![("child", "D".to_owned())],
                "doctor-edge-primary-debug-suggestion",
            ),
            (
                Error,
                "M",
                "doctor-cycle",
                vec![("parent", "M".to_owned()), ("child", "M".to_owned())],
                "doctor-cycle-suggestion",
            ),
            (
                Warning,
                "Missing Z",
                "doctor-no-role",
                vec![],
                "doctor-no-role-suggestion",
            ),
            (
                Warning,
                "Missing A",
                "doctor-no-role",
                vec![],
                "doctor-no-role-suggestion",
            ),
            (
                Warning,
                "P",
                "doctor-protected-switchable",
                vec![("role", SceneRole::Primary.label())],
                "doctor-protected-switchable-suggestion",
            ),
            (
                Warning,
                "P",
                "doctor-edge-primary-raw",
                vec![("child", "R".to_owned())],
                "doctor-edge-primary-raw-suggestion",
            ),
            (
                Info,
                "Stale",
                "doctor-stale-entry",
                vec![],
                "doctor-stale-entry-suggestion",
            ),
        ];
        assert_eq!(diagnostics.len(), expected.len());
        for (diagnostic, (severity, scene, key, arguments, suggestion)) in
            diagnostics.iter().zip(expected)
        {
            assert_eq!(diagnostic.severity, severity);
            assert_eq!(diagnostic.scene.as_deref(), Some(scene));
            assert_eq!(
                diagnostic.message,
                LANGUAGE_LOADER.get_args(key, arguments.into_iter().collect::<HashMap<_, _>>())
            );
            assert_eq!(diagnostic.suggestion, Some(LANGUAGE_LOADER.get(suggestion)));
        }
        diagnostics
    }

    #[test]
    fn run_keeps_localized_contract_and_english_fallback_in_isolated_process() {
        const CHILD: &str = "SCENEDECK_DOCTOR_FORMAT_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            // Locale selection mutates the global loader; never share that
            // mutation with other tests in the parent process.
            let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
                .args(["--exact", "services::doctor_format::tests::run_keeps_localized_contract_and_english_fallback_in_isolated_process", "--nocapture", "--test-threads=1"])
                .env(CHILD, "1")
                .output().expect("run isolated Doctor formatting test");
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("1 passed"),
                "child must execute the selected test"
            );
            return;
        }

        use crate::domain::appearance::Language;
        use crate::infra::i18n::init;
        let english = run_contract_fixture();
        init(Language::De);
        let german = run_contract_fixture();
        assert_ne!(german[4].message, english[4].message);
        // en-GB is a selectable locale with no shipped bundle: retain the
        // existing loader's English fallback, including interpolated roles.
        init(Language::EnGb);
        let fallback = run_contract_fixture();
        assert_eq!(fallback.len(), english.len());
        for (actual, expected) in fallback.iter().zip(english) {
            assert_eq!(actual.severity, expected.severity);
            assert_eq!(actual.scene, expected.scene);
            assert_eq!(actual.message, expected.message);
            assert_eq!(actual.suggestion, expected.suggestion);
        }
    }

    #[test]
    fn cycle_finding_uses_scene_as_parent_and_preserves_child_argument() {
        assert_localized(
            DoctorFindingKind::Cycle {
                child: "Child <&> 日本語".into(),
            },
            "doctor-cycle",
            &[
                ("parent", "Parent <&> 日本語"),
                ("child", "Child <&> 日本語"),
            ],
            "doctor-cycle-suggestion",
        );
    }
}
