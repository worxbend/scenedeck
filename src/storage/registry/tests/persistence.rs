//! Reading and writing registry JSON and YAML files.

use std::io;
use std::path::Path;

use super::scratch_file;
use crate::storage::registry::*;
use crate::storage::show_workspaces::RegistryTarget;

/// Apply `change` to the registry file the way a field save does: read it,
/// edit it, and write it back only when `change` reports a change.
fn edit_registry_file(
    path: &Path,
    change: impl FnOnce(&mut SceneRegistry) -> bool,
) -> io::Result<bool> {
    let target = RegistryTarget::File(path.to_path_buf());
    let mut registry = target.read()?;
    let changed = change(&mut registry);
    if changed {
        target.write(&registry)?;
    }
    Ok(changed)
}

fn is_parse_failure(error: &io::Error) -> bool {
    matches!(
        error
            .get_ref()
            .and_then(|source| source.downcast_ref::<RegistryStorageError>()),
        Some(RegistryStorageError::Parse { .. })
    )
}

#[test]
fn oversized_registry_and_yaml_writes_preserve_existing_targets() {
    let (_dir, path) = scratch_file("oversized-output", "json");
    std::fs::write(&path, b"{}").unwrap();
    let mut registry = SceneRegistry::default();
    registry.scenes.insert(
        "Huge".into(),
        SceneEntry {
            tags: vec!["x".repeat(crate::storage::small_file::REGISTRY_LIMIT + 1)],
            ..SceneEntry::default()
        },
    );
    assert_eq!(
        write_registry_to_path(&path, &registry).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"{}");
    assert_eq!(
        write_registry_yaml_to_path(&path, &registry)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"{}");
}

#[test]
fn every_registry_json_write_refuses_a_corrupt_original() {
    let (_dir, path) = scratch_file("corrupt-all-writes", "json");
    let original = br#"{"scenes":{"Camera":{"role":"primary"}}, oops"#;
    std::fs::write(&path, original).unwrap();
    for edit in 0..5 {
        let mut registry = SceneRegistry::default();
        match edit {
            0 => {
                registry.set_scene_role("New", Some(SceneRole::Primary));
            }
            1 => {
                registry.set_scene_accent("New", Some("#ABCDEF".into()));
            }
            2 => {
                registry.set_scene_order(["New".into()]);
            }
            3 => {
                registry.set_scene_icon("New", Some("nf-md-camera"));
            }
            _ => {
                registry.set_input_icon("Mic", Some("nf-md-microphone"));
            }
        }
        assert!(write_registry_to_path(&path, &registry).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
    std::fs::remove_file(&path).unwrap();
    let mut first = SceneRegistry::default();
    first.set_scene_role("First", Some(SceneRole::Primary));
    write_registry_to_path(&path, &first).unwrap();
    assert_eq!(
        read_registry_from_path(&path).unwrap().scenes["First"].role,
        Some(SceneRole::Primary)
    );
}

#[test]
fn a_registry_write_while_another_process_holds_its_lock_is_busy_and_writes_nothing() {
    let (_dir, path) = scratch_file("registry-busy", "json");
    write_registry_to_path(&path, &SceneRegistry::default()).unwrap();
    let before = std::fs::read(&path).unwrap();
    let other_process = crate::storage::lock::lock_store(&path).unwrap();
    let mut changed = SceneRegistry::default();
    changed.set_scene_role("Camera", Some(SceneRole::Primary));

    let refused = write_registry_to_path(&path, &changed).unwrap_err();

    assert!(crate::storage::lock::is_busy(&refused));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    drop(other_process);
    write_registry_to_path(&path, &changed).unwrap();
}

#[test]
fn yaml_export_writes_only_the_explicitly_selected_target() {
    for extension in ["txt", "", "yml", "YAML"] {
        let (_dir, path) = scratch_file("exact-export-target", extension);
        let sibling = path.with_extension("yaml");
        assert_ne!(path, sibling);
        std::fs::write(&sibling, b"unselected backup").unwrap();
        std::fs::write(&path, b"confirmed replacement").unwrap();
        write_registry_yaml_to_path(&path, &SceneRegistry::default()).unwrap();
        assert!(read_registry_yaml_from_path(&path).is_ok());
        assert_eq!(std::fs::read(&sibling).unwrap(), b"unselected backup");
    }
}

#[test]
fn explicit_bindings_round_trip_locally_but_import_disabled_with_identity_intact() {
    use crate::domain::{
        action_shortcut::{ActionBinding, ActionShortcut},
        hotkey::Modifiers,
        operator_action::{ActionTarget, OperatorAction},
    };
    let mut registry = SceneRegistry::default();
    registry.action_bindings.push(ActionBinding {
        shortcut: ActionShortcut {
            key: "F8".into(),
            modifiers: Modifiers::NONE,
        },
        action: OperatorAction::Scene {
            target: ActionTarget {
                name: "Camera".into(),
                uuid: Some("scene-1".into()),
            },
        },
        enabled: true,
    });
    let json = serde_json::to_string(&registry).unwrap();
    assert_eq!(
        serde_json::from_str::<SceneRegistry>(&json).unwrap(),
        registry
    );
    assert!(serde_json::from_str::<SceneRegistry>("{}")
        .unwrap()
        .action_bindings
        .is_empty());
    let (_dir, path) = scratch_file("action-bindings", "yaml");
    write_registry_yaml_to_path(&path, &registry).unwrap();
    let imported = read_registry_yaml_from_path(&path).unwrap();
    assert!(!imported.action_bindings[0].enabled);
    assert_eq!(
        imported.action_bindings[0].action,
        registry.action_bindings[0].action
    );
}

#[test]
fn audio_expectations_round_trip_locally_and_require_review_after_import() {
    use crate::domain::{health::AudioRule, operator_action::ActionTarget};
    assert!(serde_json::from_str::<SceneRegistry>("{}")
        .unwrap()
        .audio_alerts
        .is_empty());
    let registry = SceneRegistry {
        audio_alerts: vec![AudioRule {
            target: ActionTarget {
                name: "Mic".into(),
                uuid: Some("mic-one".into()),
            },
            enabled: true,
            silence: true,
            clipping: false,
        }],
        ..Default::default()
    };
    let json = serde_json::to_string(&registry).unwrap();
    assert_eq!(
        serde_json::from_str::<SceneRegistry>(&json).unwrap(),
        registry
    );
    let (_dir, path) = scratch_file("audio-alerts", "yaml");
    write_registry_yaml_to_path(&path, &registry).unwrap();
    let imported = read_registry_yaml_from_path(&path).unwrap();
    assert!(!imported.audio_alerts[0].enabled);
    assert_eq!(
        imported.audio_alerts[0].target,
        registry.audio_alerts[0].target
    );
    assert!(imported.audio_alerts[0].silence);
    assert!(!imported.audio_alerts[0].clipping);
}

#[test]
fn live_pins_keep_order_and_identity_locally_but_import_disabled() {
    use crate::domain::operator_action::{ActionTarget, OperatorAction, OperatorPin};
    assert!(serde_json::from_str::<SceneRegistry>("{}")
        .unwrap()
        .quick_actions
        .is_empty());
    let registry = SceneRegistry {
        quick_actions: vec![
            OperatorPin {
                action: OperatorAction::SetMuted {
                    target: ActionTarget {
                        name: "Old name".into(),
                        uuid: Some("mic-1".into()),
                    },
                    muted: true,
                },
                enabled: true,
            },
            OperatorPin {
                action: OperatorAction::SaveReplay,
                enabled: false,
            },
        ],
        ..Default::default()
    };
    let encoded = serde_json::to_string(&registry).unwrap();
    assert_eq!(
        serde_json::from_str::<SceneRegistry>(&encoded).unwrap(),
        registry
    );
    let (_dir, path) = scratch_file("quick-actions", "yaml");
    write_registry_yaml_to_path(&path, &registry).unwrap();
    let imported = read_registry_yaml_from_path(&path).unwrap();
    assert!(imported.quick_actions.iter().all(|pin| !pin.enabled));
    assert_eq!(
        imported
            .quick_actions
            .iter()
            .map(|pin| &pin.action)
            .collect::<Vec<_>>(),
        registry
            .quick_actions
            .iter()
            .map(|pin| &pin.action)
            .collect::<Vec<_>>()
    );
}

#[test]
fn yaml_registry_round_trips() {
    let (_dir, path) = scratch_file("registry-round-trip", "yaml");
    let mut registry = SceneRegistry::default();
    registry.scenes.insert(
        "Main".into(),
        SceneEntry {
            favorite: false,
            role: Some(SceneRole::Primary),
            tags: vec!["live".into()],
            protected: true,
            accent_color: Some("#336699".into()),
            icon: None,
        },
    );
    registry.scene_order = vec!["Main".into(), "Holding".into()];
    registry
        .rules
        .forbidden_edges
        .push(["primary".into(), "debug".into()]);

    write_registry_yaml_to_path(&path, &registry).unwrap();
    let parsed = read_registry_yaml_from_path(&path).unwrap();
    assert_eq!(
        parsed
            .scenes
            .get("Main")
            .and_then(|entry| entry.accent_color.as_deref()),
        Some("#336699")
    );

    let main = parsed.scenes.get("Main").unwrap();
    assert_eq!(main.role, Some(SceneRole::Primary));
    assert_eq!(main.tags, vec!["live"]);
    assert!(main.protected);
    assert_eq!(
        parsed.rules.forbidden_edges,
        vec![["primary".to_string(), "debug".to_string()]]
    );
    assert_eq!(parsed.scene_order, ["Main", "Holding"]);
}

#[test]
fn legacy_yaml_export_preserves_roles_order_source_metadata_and_rules() {
    let (_dir, path) = scratch_file("legacy-registry", "yaml");
    let exported = r##"---
scenes:
  'Intro: Привіт':
    role: primary
    tags:
    - live
    - 'true'
    protected: true
    accent_color: '#336699'
    icon: nf-md-camera
  Holding:
    role: secondary
    tags: []
    protected: false
  Module:
    role: module
    tags: []
    protected: false
  Raw:
    role: raw
    tags: []
    protected: false
  Debug:
    role: debug
    tags: []
    protected: false
  Archive:
    role: archive
    tags: []
    protected: false
scene_order:
- 'Intro: Привіт'
- Holding
inputs:
  'Mic / Left':
    icon: nf-md-microphone
rules:
  primary_can_depend_on:
  - module
  module_can_depend_on:
  - raw
  forbidden_edges:
  - - primary
    - debug
"##;
    std::fs::write(&path, exported).unwrap();
    let imported = read_registry_yaml_from_path(&path).unwrap();
    assert_eq!(imported.scenes.len(), 6);
    for (name, role) in [
        ("Holding", SceneRole::Secondary),
        ("Module", SceneRole::Module),
        ("Raw", SceneRole::Raw),
        ("Debug", SceneRole::Debug),
        ("Archive", SceneRole::Archive),
    ] {
        assert_eq!(imported.scenes[name].role, Some(role));
    }
    let intro = &imported.scenes["Intro: Привіт"];
    assert_eq!(intro.role, Some(SceneRole::Primary));
    assert!(intro.protected);
    assert_eq!(intro.tags, ["live", "true"]);
    assert_eq!(intro.accent_color.as_deref(), Some("#336699"));
    assert_eq!(intro.icon.as_deref(), Some("nf-md-camera"));
    assert_eq!(
        imported.inputs["Mic / Left"].icon.as_deref(),
        Some("nf-md-microphone")
    );
    assert_eq!(imported.scene_order, ["Intro: Привіт", "Holding"]);
    assert_eq!(imported.rules.forbidden_edges, [["primary", "debug"]]);
    let expected = serde_json::to_value(&imported).unwrap();
    write_registry_yaml_to_path(&path, &imported).unwrap();
    let actual = read_registry_yaml_from_path(&path).unwrap();
    assert_eq!(serde_json::to_value(actual).unwrap(), expected);
}

#[test]
fn invalid_yaml_returns_invalid_data() {
    let (_dir, path) = scratch_file("registry-invalid", "yaml");
    std::fs::write(&path, "scenes: [").unwrap();

    let err = read_registry_yaml_from_path(&path).unwrap_err();

    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn invalid_json_registry_returns_typed_parse_error() {
    let (_dir, path) = scratch_file("registry-invalid-json", "json");
    std::fs::write(&path, "{").unwrap();

    let err = read_registry_from_path(&path).unwrap_err();

    assert!(matches!(err, RegistryStorageError::Parse { .. }));
}

#[test]
fn a_role_edit_of_the_registry_file_preserves_existing_metadata() {
    let (_dir, path) = scratch_file("registry-set-role", "json");
    let mut registry = SceneRegistry::default();
    registry.scenes.insert(
        "Main".into(),
        SceneEntry {
            favorite: false,
            role: Some(SceneRole::Module),
            tags: vec!["camera".into()],
            protected: true,
            accent_color: None,
            icon: None,
        },
    );
    write_registry_to_path(&path, &registry).unwrap();

    let changed = edit_registry_file(&path, |registry| {
        registry.set_scene_role("Main", Some(SceneRole::Secondary))
    })
    .unwrap();
    let parsed = read_registry_from_path(&path).unwrap();

    assert!(changed);
    let entry = &parsed.scenes["Main"];
    assert_eq!(entry.role, Some(SceneRole::Secondary));
    assert_eq!(entry.tags, ["camera"]);
    assert!(entry.protected);
}

#[test]
fn a_role_edit_does_not_overwrite_an_invalid_registry_file() {
    let (_dir, path) = scratch_file("registry-set-role-invalid", "json");
    std::fs::write(&path, "{").unwrap();

    let err = edit_registry_file(&path, |registry| {
        registry.set_scene_role("Main", Some(SceneRole::Primary))
    })
    .unwrap_err();
    let raw = std::fs::read_to_string(&path).unwrap();

    assert!(is_parse_failure(&err), "{err}");
    assert_eq!(raw, "{");
}

#[test]
fn removing_a_scene_from_the_registry_file_keeps_the_others() {
    let (_dir, path) = scratch_file("registry-remove-entry", "json");
    let mut registry = SceneRegistry::default();
    registry.scenes.insert(
        "Main".into(),
        SceneEntry {
            favorite: false,
            role: Some(SceneRole::Primary),
            tags: vec!["live".into()],
            protected: false,
            accent_color: None,
            icon: None,
        },
    );
    registry.scenes.insert(
        "Camera".into(),
        SceneEntry {
            favorite: false,
            role: Some(SceneRole::Raw),
            tags: vec!["source".into()],
            protected: true,
            accent_color: None,
            icon: None,
        },
    );
    write_registry_to_path(&path, &registry).unwrap();

    let removed =
        edit_registry_file(&path, |registry| registry.scenes.remove("Main").is_some()).unwrap();
    let parsed = read_registry_from_path(&path).unwrap();

    assert!(removed);
    assert!(!parsed.scenes.contains_key("Main"));
    assert_eq!(parsed.scenes["Camera"].role, Some(SceneRole::Raw));
    assert!(parsed.scenes["Camera"].protected);
}

#[test]
fn a_scene_removal_does_not_overwrite_an_invalid_registry_file() {
    let (_dir, path) = scratch_file("registry-remove-invalid", "json");
    std::fs::write(&path, "{").unwrap();

    let err =
        edit_registry_file(&path, |registry| registry.scenes.remove("Main").is_some()).unwrap_err();
    let raw = std::fs::read_to_string(&path).unwrap();

    assert!(is_parse_failure(&err), "{err}");
    assert_eq!(raw, "{");
}

#[test]
fn icons_round_trip_through_json_and_stay_out_of_it_when_unset() {
    let (_dir, path) = scratch_file("registry-icons", "json");
    let mut registry = SceneRegistry::default();
    registry.set_scene_icon("Main", Some("camera"));
    registry.set_input_icon("Mic/Aux", Some("microphone"));
    write_registry_to_path(&path, &registry).unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    let parsed = read_registry_from_path(&path).unwrap();

    registry.set_scene_icon("Main", None);
    registry.set_input_icon("Mic/Aux", None);
    write_registry_to_path(&path, &registry).unwrap();
    let empty_raw = std::fs::read_to_string(&path).unwrap();

    assert_eq!(parsed.scene_icon("Main"), Some("camera"));
    assert_eq!(parsed.input_icon("Mic/Aux"), Some("microphone"));
    assert!(raw.contains(r#""inputs""#));
    assert!(!empty_raw.contains(r#""inputs""#));
    assert!(!empty_raw.contains(r#""icon""#));
}

#[test]
fn a_registry_without_an_inputs_section_still_loads() {
    let registry: SceneRegistry = serde_json::from_str(r#"{"scenes":{}}"#).unwrap();

    assert!(registry.inputs.is_empty());
    assert_eq!(registry.input_icon("Mic/Aux"), None);
}

#[test]
fn an_unversioned_registry_json_is_rewritten_at_the_current_version() {
    let (_dir, path) = scratch_file("registry-unversioned", "json");
    std::fs::write(&path, r#"{"scenes":{"Intro":{"tags":["live"]}}}"#).unwrap();
    let mut registry = read_registry_from_path(&path).unwrap();
    assert!(registry.set_scene_favorite("Intro", true));

    write_registry_to_path(&path, &registry).unwrap();

    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(written["version"], REGISTRY_FILE_VERSION);
    assert_eq!(written["scenes"]["Intro"]["favorite"], true);
}

#[test]
fn a_newer_registry_json_is_read_but_never_overwritten() {
    let (_dir, path) = scratch_file("registry-newer", "json");
    let original = r#"{"version":999,"scenes":{"Intro":{"tags":["live"]}},"future":true}"#;
    std::fs::write(&path, original).unwrap();
    let mut registry = read_registry_from_path(&path).unwrap();
    assert_eq!(registry.scenes["Intro"].tags, ["live"]);
    assert!(registry.set_scene_favorite("Intro", true));

    let refused = write_registry_to_path(&path, &registry).unwrap_err();

    assert_eq!(refused.kind(), io::ErrorKind::Unsupported);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn workspace_entries_and_yaml_exports_carry_no_registry_file_version() {
    let registry = SceneRegistry::default();

    let entry = serde_json::to_value(&registry).unwrap();
    let yaml = serde_saphyr::to_string(&registry).unwrap();

    assert!(entry.get("version").is_none());
    assert!(!yaml.contains("version"), "{yaml}");
}
