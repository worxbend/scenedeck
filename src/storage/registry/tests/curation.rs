//! Editing curation in memory: roles, tags, accents, icons, order and renames.

use crate::storage::registry::*;

#[test]
fn tags_normalize_without_erasing_other_curation_and_favorites_survive_clearing() {
    let mut registry = SceneRegistry::default();
    assert!(!registry.set_scene_tags("Main", [" ".into()]));
    assert!(!registry.set_scene_favorite("Main", false));
    assert!(registry.scenes.is_empty());
    registry.set_scene_icon("Main", Some("camera"));
    assert!(registry.set_scene_tags(
        "Main",
        [" Podcast ", "", "podcast", "Київ"].map(str::to_owned)
    ));
    assert_eq!(registry.scenes["Main"].tags, ["Podcast", "Київ"]);
    assert!(!registry.set_scene_tags("Main", ["Podcast", "Київ"].map(str::to_owned)));
    assert!(registry.set_scene_favorite("Main", true));
    assert!(!registry.set_scene_favorite("Main", true));
    assert!(registry.set_scene_tags("Main", []));
    registry.set_scene_icon("Main", None);
    assert!(registry.scenes["Main"].favorite);
    let restored: SceneRegistry =
        serde_json::from_str(&serde_json::to_string(&registry).unwrap()).unwrap();
    assert_eq!(restored, registry);
    assert!(registry.set_scene_favorite("Main", false));
    assert!(registry.scenes.is_empty());
}

#[test]
fn scene_rename_preserves_all_curation_and_original_card_position() {
    let mut registry = SceneRegistry::default();
    let entry = SceneEntry {
        role: Some(SceneRole::Primary),
        tags: vec!["podcast".into()],
        favorite: true,
        protected: true,
        accent_color: Some("#112233".into()),
        icon: Some("camera".into()),
    };
    let expected = serde_json::to_value(&entry).unwrap();
    registry.scenes.insert("Camera".into(), entry);
    registry.scene_order = vec![
        "Intro".into(),
        "Camera".into(),
        "End".into(),
        "Guest".into(),
    ];
    assert_eq!(
        registry.rename_scene("Camera", "Guest", None),
        SceneRenameResult::Renamed
    );
    assert!(!registry.scenes.contains_key("Camera"));
    assert_eq!(
        serde_json::to_value(&registry.scenes["Guest"]).unwrap(),
        expected
    );
    assert_eq!(registry.scene_order, ["Intro", "Guest", "End"]);
    assert_eq!(
        registry.rename_scene("Camera", "Guest", None),
        SceneRenameResult::Unchanged
    );
}

#[test]
fn scene_rename_conflict_leaves_both_entries_and_order_untouched() {
    let mut registry = SceneRegistry::default();
    registry.set_scene_icon("Camera", Some("camera"));
    registry.set_scene_icon("Guest", Some("person"));
    registry.scene_order = vec!["Camera".into(), "Guest".into()];
    let before = serde_json::to_value(&registry).unwrap();
    assert_eq!(
        registry.rename_scene("Camera", "Guest", None),
        SceneRenameResult::Conflict
    );
    assert_eq!(serde_json::to_value(&registry).unwrap(), before);
}

#[test]
fn scene_rename_updates_order_without_creating_empty_metadata() {
    let mut registry = SceneRegistry {
        scene_order: vec!["Camera".into()],
        ..Default::default()
    };
    assert_eq!(
        registry.rename_scene("Camera", "Guest", None),
        SceneRenameResult::Renamed
    );
    assert_eq!(registry.scene_order, ["Guest"]);
    assert!(registry.scenes.is_empty());
    assert_eq!(
        registry.rename_scene("Guest", "Guest", None),
        SceneRenameResult::Unchanged
    );
    assert_eq!(
        registry.rename_scene("Guest", "", None),
        SceneRenameResult::Unchanged
    );
}

#[test]
fn clearing_an_accent_keeps_an_icon_the_scene_still_has() {
    let mut registry = SceneRegistry::default();
    registry.set_scene_icon("Camera", Some("nf-md-camera"));
    registry.set_scene_accent("Camera", Some("#ff0000".to_string()));

    registry.set_scene_accent("Camera", None);

    assert_eq!(registry.scene_icon("Camera"), Some("nf-md-camera"));
}

#[test]
fn clearing_the_last_piece_of_metadata_removes_the_entry() {
    // An untouched scene should leave no trace in the file, whichever of
    // the three fields was the last one set.
    for (set, clear) in [
        (
            Box::new(|r: &mut SceneRegistry| r.set_scene_icon("S", Some("nf-md-camera")))
                as Box<dyn Fn(&mut SceneRegistry) -> bool>,
            Box::new(|r: &mut SceneRegistry| r.set_scene_icon("S", None))
                as Box<dyn Fn(&mut SceneRegistry) -> bool>,
        ),
        (
            Box::new(|r: &mut SceneRegistry| r.set_scene_accent("S", Some("#abcdef".into()))),
            Box::new(|r: &mut SceneRegistry| r.set_scene_accent("S", None)),
        ),
        (
            Box::new(|r: &mut SceneRegistry| r.set_scene_role("S", Some(SceneRole::Primary))),
            Box::new(|r: &mut SceneRegistry| r.set_scene_role("S", None)),
        ),
    ] {
        let mut registry = SceneRegistry::default();
        assert!(set(&mut registry));
        assert!(registry.scenes.contains_key("S"));
        assert!(clear(&mut registry));
        assert!(
            !registry.scenes.contains_key("S"),
            "an entry with nothing left in it should be removed"
        );
    }
}

#[test]
fn clearing_an_accent_on_an_unknown_scene_changes_nothing() {
    // Must not report a change, or every colour-picker close would write
    // the registry; and must not create an empty entry.
    let mut registry = SceneRegistry::default();

    assert!(!registry.set_scene_accent("Never seen", None));
    assert!(registry.scenes.is_empty());
}

#[test]
fn setting_the_same_accent_twice_reports_no_change() {
    let mut registry = SceneRegistry::default();
    assert!(registry.set_scene_accent("Camera", Some("#123456".to_string())));
    assert!(!registry.set_scene_accent("Camera", Some("#123456".to_string())));
}

#[test]
fn clearing_a_role_keeps_an_icon_the_scene_still_has() {
    let mut registry = SceneRegistry::default();
    registry.set_scene_icon("Camera", Some("nf-md-camera"));
    registry.set_scene_role("Camera", Some(SceneRole::Primary));

    registry.set_scene_role("Camera", None);

    assert_eq!(
        registry.scene_icon("Camera"),
        Some("nf-md-camera"),
        "clearing the role threw away the scene's icon"
    );
}

#[test]
fn ordered_scene_ids_uses_saved_order_and_appends_new_scenes() {
    let registry = SceneRegistry {
        scene_order: vec!["Three".into(), "Missing".into(), "One".into()],
        ..SceneRegistry::default()
    };

    assert_eq!(
        registry.ordered_scene_ids(["One", "Two", "Three"]),
        ["Three", "One", "Two"]
    );
}

#[test]
fn set_scene_order_removes_duplicates_and_reports_changes() {
    let mut registry = SceneRegistry::default();

    assert!(registry.set_scene_order(["Two", "One", "Two"].into_iter().map(str::to_string)));
    assert_eq!(registry.scene_order, ["Two", "One"]);
    assert!(!registry.set_scene_order(["Two", "One"].into_iter().map(str::to_string)));
}

#[test]
fn scene_accent_normalizes_rgb_and_ignores_alpha() {
    assert_eq!(scene_accent_hex(0.2, 0.4, 0.6), "#336699");
    assert_eq!(scene_accent_hex(-1.0, 2.0, 0.5), "#00FF80");
}

#[test]
fn scene_accent_parser_accepts_rgb_hex_and_rejects_other_shapes() {
    assert_eq!(parse_scene_accent("#336699"), Some((51, 102, 153)));
    assert_eq!(parse_scene_accent("336699"), Some((51, 102, 153)));
    assert_eq!(parse_scene_accent("#33669980"), None);
    assert_eq!(parse_scene_accent("not-a-color"), None);
}

#[test]
fn scene_registry_set_role_adds_default_metadata_for_new_role() {
    let mut registry = SceneRegistry::default();

    assert!(registry.set_scene_role("Main", Some(SceneRole::Primary)));

    let entry = registry.scenes.get("Main").unwrap();
    assert_eq!(entry.role, Some(SceneRole::Primary));
    assert!(entry.tags.is_empty());
    assert!(!entry.protected);
}

#[test]
fn scene_registry_set_role_updates_role_without_losing_metadata() {
    let mut registry = SceneRegistry::default();
    registry.scenes.insert(
        "Main".to_string(),
        SceneEntry {
            favorite: false,
            role: Some(SceneRole::Module),
            tags: vec!["camera".to_string()],
            protected: true,
            accent_color: None,
            icon: None,
        },
    );

    assert!(registry.set_scene_role("Main", Some(SceneRole::Secondary)));

    let entry = registry.scenes.get("Main").unwrap();
    assert_eq!(entry.role, Some(SceneRole::Secondary));
    assert_eq!(entry.tags, ["camera"]);
    assert!(entry.protected);
}

#[test]
fn scene_registry_set_role_removes_entry_for_unassigned_role() {
    let mut registry = SceneRegistry::default();
    registry.scenes.insert(
        "Main".to_string(),
        SceneEntry {
            favorite: false,
            role: Some(SceneRole::Primary),
            tags: Vec::new(),
            protected: false,
            accent_color: None,
            icon: None,
        },
    );

    assert!(registry.set_scene_role("Main", None));

    assert!(!registry.scenes.contains_key("Main"));
}

#[test]
fn scene_registry_set_role_preserves_accent_for_unassigned_scene() {
    let mut registry = SceneRegistry::default();
    registry.scenes.insert(
        "Main".to_string(),
        SceneEntry {
            favorite: false,
            role: Some(SceneRole::Primary),
            tags: Vec::new(),
            protected: false,
            accent_color: Some("#336699".to_string()),
            icon: None,
        },
    );

    registry.set_scene_role("Main", None);

    let entry = registry.scenes.get("Main").unwrap();
    assert_eq!(entry.role, None);
    assert_eq!(entry.accent_color.as_deref(), Some("#336699"));
}

#[test]
fn scene_icons_are_set_cleared_and_ignored_when_unchanged() {
    let mut registry = SceneRegistry::default();

    assert!(registry.set_scene_icon("Main", Some("camera")));
    assert_eq!(registry.scene_icon("Main"), Some("camera"));
    assert!(!registry.set_scene_icon("Main", Some("camera")));

    assert!(registry.set_scene_icon("Main", Some("game")));
    assert_eq!(registry.scene_icon("Main"), Some("game"));

    assert!(registry.set_scene_icon("Main", None));
    assert_eq!(registry.scene_icon("Main"), None);
    assert!(
        !registry.scenes.contains_key("Main"),
        "an entry holding nothing but a cleared icon should not linger"
    );
    assert!(!registry.set_scene_icon("Main", None));
}

#[test]
fn clearing_a_scene_icon_keeps_the_rest_of_the_entry() {
    let mut registry = SceneRegistry::default();
    registry.set_scene_role("Main", Some(SceneRole::Primary));
    registry.set_scene_icon("Main", Some("camera"));

    registry.set_scene_icon("Main", None);

    let entry = registry
        .scenes
        .get("Main")
        .expect("entry kept for its role");
    assert_eq!(entry.role, Some(SceneRole::Primary));
    assert_eq!(entry.icon, None);
}

#[test]
fn input_icons_are_set_and_cleared_independently_of_scenes() {
    let mut registry = SceneRegistry::default();

    assert!(registry.set_input_icon("Mic/Aux", Some("microphone")));
    assert_eq!(registry.input_icon("Mic/Aux"), Some("microphone"));
    assert!(!registry.set_input_icon("Mic/Aux", Some("microphone")));
    assert!(registry.scenes.is_empty());

    assert!(registry.set_input_icon("Mic/Aux", None));
    assert!(registry.inputs.is_empty());
    assert!(!registry.set_input_icon("Mic/Aux", None));
}
