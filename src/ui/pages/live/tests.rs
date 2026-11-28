use super::*;

#[test]
#[ignore = "requires isolated GTK; no OBS connection"]
fn native_preflight_continuation_honors_current_output_confirmation_once() {
    crate::ui::native_test!(|| {
        use crate::ui::preflight::tests::{connected, save_plan, window};
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        connected(&nav);
        save_plan(
            &nav,
            crate::domain::preflight::PreflightPlan {
                review_before_output: true,
                ..Default::default()
            },
        );
        let button = Button::with_label("Record");
        let parent = adw::Window::new();
        parent.set_content(Some(&button));
        parent.present();
        handle_output_toggle(&button, &nav, OutputKind::Record);
        let review = window("preflight-review")
            .downcast::<adw::MessageDialog>()
            .unwrap();
        nav.state
            .borrow_mut()
            .config
            .outputs
            .confirm_start_recording = true;
        review.response("continue");
        let confirmation = gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|widget| {
                widget
                    .downcast::<adw::MessageDialog>()
                    .ok()
                    .filter(|dialog| {
                        dialog.is_visible() && dialog.widget_name() != "preflight-review"
                    })
            })
            .expect("ordinary output confirmation after review");
        assert!(nav.state.borrow().workspace.activity.entries.is_empty());
        confirmation.response("cancel");
        assert!(nav.state.borrow().workspace.activity.entries.is_empty());
        handle_output_toggle(&button, &nav, OutputKind::Record);
        let review = window("preflight-review")
            .downcast::<adw::MessageDialog>()
            .unwrap();
        nav.state
            .borrow_mut()
            .config
            .outputs
            .confirm_start_recording = false;
        review.response("continue");
        review.response("continue");
        assert_eq!(nav.state.borrow().workspace.activity.entries.len(), 1);
        {
            let mut state = nav.state.borrow_mut();
            state.record_status = OutputStatus::active();
            state.config.outputs.confirm_stop_recording = false;
        }
        handle_output_toggle(&button, &nav, OutputKind::Record);
        assert_eq!(
            nav.state.borrow().workspace.activity.entries.len(),
            2,
            "stop retains its existing flow"
        );
        assert!(!gtk4::Window::list_toplevels().iter().any(|window| {
            window
                .clone()
                .downcast::<gtk4::Window>()
                .is_ok_and(|window| {
                    window.is_visible() && window.widget_name() == "preflight-review"
                })
        }));
        parent.close();
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_live_filters_keep_search_widget_and_original_shortcut_badges() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let (inventory, mut registry) = live_fixture(
            &[
                ("Camera", SceneRole::Primary),
                ("Slides", SceneRole::Primary),
            ],
            &[],
        );
        registry.set_scene_favorite("Slides", true);
        nav.state.borrow_mut().registry = registry;
        nav.state.borrow_mut().scene_inventory = inventory.clone();
        let live = build(nav.clone());
        fn find(
            root: &gtk4::Widget,
            matches: &impl Fn(&gtk4::Widget) -> bool,
        ) -> Option<gtk4::Widget> {
            if matches(root) {
                return Some(root.clone());
            }
            let mut child = root.first_child();
            while let Some(widget) = child {
                if let Some(found) = find(&widget, matches) {
                    return Some(found);
                }
                child = widget.next_sibling();
            }
            None
        }
        let badge = |card: &scene_card::SceneCardHandle| {
            find(card.root.upcast_ref(), &|widget| {
                widget.has_css_class("scene-card-hotkey")
            })
            .unwrap()
            .downcast::<Label>()
            .unwrap()
            .text()
            .to_string()
        };
        rebuild_scene_cards(&live, &inventory, &nav);
        assert_eq!(live.scene_cards.borrow()[0].scene_id, "Slides");
        assert_eq!(badge(&live.scene_cards.borrow()[0]), "2");
        assert_eq!(badge(&live.scene_cards.borrow()[1]), "1");
        let search = find(live.deck_controls.upcast_ref(), &|widget| {
            widget.widget_name() == "deck-search"
        })
        .unwrap()
        .downcast::<gtk4::SearchEntry>()
        .unwrap();
        search.set_text("Slides");
        search.emit_by_name::<()>("search-changed", &[]);
        rebuild_scene_cards(&live, &inventory, &nav);
        assert_eq!(live.scene_cards.borrow().len(), 1);
        assert_eq!(badge(&live.scene_cards.borrow()[0]), "2");
        search.set_text("missing");
        search.emit_by_name::<()>("search-changed", &[]);
        rebuild_scene_cards(&live, &inventory, &nav);
        assert!(live.scene_cards.borrow().is_empty());
        assert_eq!(
            find(live.deck_controls.upcast_ref(), &|widget| widget
                .widget_name()
                == "deck-search")
            .unwrap(),
            search.clone().upcast::<gtk4::Widget>()
        );
        assert_eq!(
            scene_for_slot(&inventory, &nav.state.borrow().registry, 1).as_deref(),
            Some("Slides")
        );
        nav.state.borrow_mut().show_registry.generation += 1;
        nav.state.borrow_mut().registry.scene_filter.query = "Camera".into();
        rebuild_scene_cards(&live, &inventory, &nav);
        search.set_text("Slides");
        search.emit_by_name::<()>("search-changed", &[]);
        assert_eq!(nav.state.borrow().registry.scene_filter.query, "Camera");
        assert!(crate::ui::test_support::pump_until(
            std::time::Duration::from_secs(5),
            || crate::ui::persist::registry_save_status()
                == crate::ui::persist::RegistrySaveStatus::Saved
        ));
    });
}

#[test]
fn favorite_sorting_and_deck_filters_keep_unfiltered_shortcut_slots() {
    let (inventory, mut registry) = live_fixture(
        &[
            ("Camera", SceneRole::Primary),
            ("Overlay", SceneRole::Module),
            ("Slides", SceneRole::Primary),
        ],
        &[],
    );
    registry.set_scene_favorite("Slides", true);
    assert_eq!(
        visible_scene_slots(&inventory, &registry),
        [(1, "Slides".into()), (0, "Camera".into())]
    );
    registry.scene_filter.favorites_only = true;
    assert_eq!(
        visible_scene_slots(&inventory, &registry),
        [(1, "Slides".into())]
    );
    assert_eq!(
        scene_for_slot(&inventory, &registry, 0).as_deref(),
        Some("Camera")
    );
    registry.scene_filter.query = "nothing matches".into();
    assert!(visible_scene_slots(&inventory, &registry).is_empty());
    assert_eq!(
        scene_for_slot(&inventory, &registry, 1).as_deref(),
        Some("Slides")
    );
    assert_eq!(
        registry.ordered_scene_ids(["Camera", "Slides"]),
        ["Camera", "Slides"]
    );
}

#[test]
fn short_windows_collapse_audio_until_expanded() {
    assert!(audio_collapsed(true, false));
    assert!(!audio_collapsed(true, true));
    assert!(!audio_collapsed(false, false));
}
use crate::domain::hotkey::SceneHotkeyStyle;
use crate::domain::output::OutputRunState;
use crate::domain::role::SceneRole;
use crate::domain::scene::Scene;
use crate::storage::registry::SceneEntry;

#[test]
#[ignore = "requires a disposable GTK display; no OBS or keyring access"]
fn native_audio_refresh_retains_controls_until_their_owner_changes() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let input = crate::domain::audio::AudioInput::new("Mic".into(), false, 0.5, -6.0);
        nav.state.borrow_mut().set_current_program("Main".into());
        nav.state
            .borrow_mut()
            .apply_active_audio(Some("Main".into()), Ok(vec![input.clone()]));
        let live = build(nav.clone());
        rebuild_audio_cards(&live, std::slice::from_ref(&input), &nav);
        let root = live.audio_cards.borrow()[0].root.clone();
        rebuild_audio_cards(&live, std::slice::from_ref(&input), &nav);
        assert_eq!(
            live.audio_cards.borrow()[0].root,
            root,
            "identical audio readback replaced its controls"
        );
        nav.state.borrow_mut().invalidate_active_audio();
        rebuild_audio_cards(&live, &[], &nav);
        assert_eq!(
            live.audio_cards.borrow()[0].root,
            root,
            "same-scene loading replaced controls"
        );
        assert!(root.is_sensitive());
        nav.state
            .borrow_mut()
            .apply_active_audio(Some("Main".into()), Ok(vec![input.clone()]));
        rebuild_audio_cards(&live, std::slice::from_ref(&input), &nav);
        assert_eq!(live.audio_cards.borrow()[0].root, root);
        assert!(root.is_sensitive());
        nav.state.borrow_mut().set_current_program("Other".into());
        rebuild_audio_cards(&live, &[], &nav);
        assert_eq!(
            live.audio_cards.borrow()[0].root,
            root,
            "Program changed the layout before membership arrived"
        );
        let other = crate::domain::audio::AudioInput::new("Other input".into(), false, 1.0, 0.0);
        let inputs = vec![input.clone(), other];
        nav.state
            .borrow_mut()
            .apply_active_audio(Some("Other".into()), Ok(inputs.clone()));
        rebuild_audio_cards(&live, &inputs, &nav);
        assert_eq!(
            live.audio_cards.borrow()[0].root,
            root,
            "adding an input replaced an unrelated fader"
        );
        nav.state.borrow_mut().workspace.invalidate();
        rebuild_audio_cards(&live, &[], &nav);
        assert!(
            live.audio_cards.borrow().is_empty(),
            "collection/session changes must retire old controls"
        );
    });
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_active_audio_readiness_retains_loading_but_retires_failed_controls() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.set_current_program("A".into());
            state.apply_active_audio(
                Some("A".into()),
                Ok(vec![crate::domain::audio::AudioInput::new(
                    "MicA".into(),
                    false,
                    0.5,
                    -6.0,
                )]),
            );
        }
        let live = build(nav.clone());
        let inputs = nav.state.borrow().audio_inputs.clone();
        rebuild_audio_cards(&live, &inputs, &nav);
        assert_eq!(live.audio_cards.borrow().len(), 1);
        nav.state.borrow_mut().set_current_program("B".into());
        rebuild_audio_cards(&live, &[], &nav);
        assert_eq!(live.audio_cards.borrow().len(), 1);
        assert!(live.audio_refresh_notice.is_spinning());
        let rendered_hint = || {
            live.audio_box
                .first_child()
                .unwrap()
                .first_child()
                .unwrap()
                .downcast::<Label>()
                .unwrap()
                .text()
                .to_string()
        };
        nav.state.borrow_mut().apply_active_audio(
            Some("B".into()),
            Err("fixture observation unavailable".into()),
        );
        rebuild_audio_cards(&live, &[], &nav);
        assert!(live.audio_cards.borrow().is_empty());
        assert!(rendered_hint().contains("fixture observation unavailable"));
        nav.state
            .borrow_mut()
            .apply_active_audio(Some("B".into()), Ok(Vec::new()));
        rebuild_audio_cards(&live, &[], &nav);
        assert_eq!(
            rendered_hint(),
            fl!(LANGUAGE_LOADER, "live-audio-empty-hint")
        );
    });
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_observed_studio_changes_update_hints_without_replacing_cards() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.scene_collections.current = Some("Fixture".into());
            state.scene_inventory.scenes = vec![Scene {
                uuid: None,
                id: "Main".into(),
            }];
            state.scene_inventory.current_id = Some("Main".into());
            state.registry.scenes.insert(
                "Main".into(),
                SceneEntry {
                    role: Some(SceneRole::Primary),
                    ..Default::default()
                },
            );
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.snapshot = Some(Default::default());
            state.workspace.snapshot.as_mut().unwrap().studio.enabled = Some(false);
        }
        let live = build(nav.clone());
        let inventory = nav.state.borrow().scene_inventory.clone();
        rebuild_scene_cards(&live, &inventory, &nav);
        let root = live.scene_cards.borrow()[0].root.clone();
        for studio in [false, true, false] {
            {
                let mut state = nav.state.borrow_mut();
                state.workspace.snapshot.as_mut().unwrap().studio.enabled = Some(studio);
                state.workspace.pending = false;
                state.workspace.active_action = None;
                state.workspace.loading = false;
                state.workspace.observed_at = Some(std::time::Instant::now());
                state.workspace.observe_global_controls();
            }
            rebuild_scene_cards(&live, &inventory, &nav);
            assert_eq!(live.scene_cards.borrow()[0].root, root);
            let hint = if studio {
                fl!(LANGUAGE_LOADER, "scene-card-stage-hint")
            } else {
                fl!(LANGUAGE_LOADER, "scene-card-tooltip-ready")
            };
            assert!(root.tooltip_text().unwrap().contains(&hint));
            assert_eq!(
                live.hotkey_hint.tooltip_text().as_deref(),
                crate::ui::hotkeys::scene_hint(&nav, &nav.state.borrow().config.hotkeys).as_deref()
            );
            root.emit_clicked();
            let expected = if studio {
                fl!(LANGUAGE_LOADER, "control-preview", scene = "Main")
            } else {
                fl!(LANGUAGE_LOADER, "control-program", scene = "Main")
            };
            assert_eq!(
                nav.state
                    .borrow()
                    .workspace
                    .activity
                    .entries
                    .back()
                    .unwrap()
                    .description,
                expected
            );
        }
    });
}

/// Cards take their order and roles from curation, so they wait until it
/// is Ready instead of falling back to OBS order.
#[test]
#[ignore = "requires a native GTK display"]
fn native_scene_cards_wait_for_ready_curation() {
    crate::ui::native_test!(|| {
        use crate::controller::state::RegistryStatus;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        nav.update(|state| {
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.scene_collections.current = Some("Fixture".into());
            state.scene_inventory.scenes = vec![Scene {
                uuid: None,
                id: "Main".into(),
            }];
            state.registry.scenes.insert(
                "Main".into(),
                SceneEntry {
                    role: Some(SceneRole::Primary),
                    ..Default::default()
                },
            );
            let mut snapshot = crate::domain::control::ControlSnapshot::default();
            snapshot.studio.enabled = Some(false);
            state.workspace.snapshot = Some(snapshot);
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.observe_scene_controls();
        })
        .expect("AppState is not borrowed");
        let live = build(nav.clone());
        let inventory = nav.read(|state| state.scene_inventory.clone());
        rebuild_scene_cards(&live, &inventory, &nav);
        assert_eq!(live.scene_cards.borrow().len(), 1);
        assert!(live.scene_cards.borrow()[0].root.is_sensitive());

        for status in [
            RegistryStatus::Loading,
            RegistryStatus::Unavailable,
            RegistryStatus::Failed("corrupt show-workspaces.json".into()),
        ] {
            nav.update(|state| state.show_registry.status = status.clone())
                .expect("AppState is not borrowed");
            rebuild_scene_cards(&live, &inventory, &nav);
            assert!(
                live.scene_cards.borrow().is_empty(),
                "{status:?} curation rendered scene cards"
            );
            let notice = crate::ui::persist::registry_availability_message(&nav)
                .unwrap_or_else(|| panic!("{status:?} has no availability notice"));
            assert!(
                crate::ui::test_support::find_where(live.scenes_box.upcast_ref(), |widget| {
                    widget
                        .downcast_ref::<Label>()
                        .is_some_and(|label| label.label() == notice)
                })
                .is_some(),
                "{status:?} notice is not shown in place of the cards"
            );
        }
    });
}

#[test]
fn disconnect_confirmation_respects_stops_and_unknown_states() {
    let mut config = OutputConfig {
        confirm_stop_stream: false,
        confirm_stop_recording: false,
        ..Default::default()
    };
    assert!(!requires_disconnect_confirmation(
        &OutputStatus::inactive(),
        &OutputStatus::inactive(),
        &config
    ));
    assert!(!requires_disconnect_confirmation(
        &OutputStatus::active(),
        &OutputStatus::active(),
        &config
    ));
    config.confirm_stop_stream = true;
    assert!(requires_disconnect_confirmation(
        &OutputStatus::active(),
        &OutputStatus::inactive(),
        &config
    ));
    assert!(!requires_disconnect_confirmation(
        &OutputStatus::inactive(),
        &OutputStatus::active(),
        &config
    ));
    config.confirm_stop_recording = true;
    assert!(requires_disconnect_confirmation(
        &OutputStatus::inactive(),
        &OutputStatus::active(),
        &config
    ));
    for state in [
        OutputRunState::Unknown,
        OutputRunState::Starting,
        OutputRunState::Stopping,
        OutputRunState::Reconnecting,
    ] {
        assert!(requires_disconnect_confirmation(
            &OutputStatus::new(false, state),
            &OutputStatus::inactive(),
            &config
        ));
    }
}

fn scene_entry(role: SceneRole) -> SceneEntry {
    SceneEntry {
        favorite: false,
        role: Some(role),
        tags: Vec::new(),
        protected: false,
        accent_color: None,
        icon: None,
    }
}

/// Inventory in OBS order, with every scene registered under `roles`.
fn live_fixture(roles: &[(&str, SceneRole)], order: &[&str]) -> (SceneInventory, SceneRegistry) {
    let inventory = SceneInventory {
        scenes: roles
            .iter()
            .map(|(id, _)| Scene {
                uuid: None,
                id: (*id).to_string(),
            })
            .collect(),
        current_id: None,
        previous_id: None,
    };
    let mut registry = SceneRegistry {
        scene_order: order.iter().map(|id| (*id).to_string()).collect(),
        ..SceneRegistry::default()
    };
    for (id, role) in roles {
        registry
            .scenes
            .insert((*id).to_string(), scene_entry(*role));
    }
    (inventory, registry)
}

#[test]
fn live_scene_order_follows_the_registry_order() {
    let (inventory, registry) = live_fixture(
        &[
            ("Camera", SceneRole::Primary),
            ("Slides", SceneRole::Primary),
            ("Break", SceneRole::Primary),
        ],
        &["Break", "Camera"],
    );

    assert_eq!(
        live_scene_order(&inventory, &registry),
        ["Break", "Camera", "Slides"]
    );
}

#[test]
fn live_scene_order_skips_scenes_that_are_not_switchable() {
    let (inventory, registry) = live_fixture(
        &[
            ("Camera", SceneRole::Primary),
            ("Overlay", SceneRole::Module),
            ("Slides", SceneRole::Primary),
        ],
        &[],
    );

    assert_eq!(
        live_scene_order(&inventory, &registry),
        ["Camera", "Slides"]
    );
}

#[test]
fn slots_index_the_rendered_card_order() {
    let (inventory, registry) = live_fixture(
        &[
            ("Camera", SceneRole::Primary),
            ("Overlay", SceneRole::Module),
            ("Slides", SceneRole::Primary),
        ],
        &["Slides"],
    );

    assert_eq!(
        scene_for_slot(&inventory, &registry, 0).as_deref(),
        Some("Slides")
    );
    assert_eq!(
        scene_for_slot(&inventory, &registry, 1).as_deref(),
        Some("Camera")
    );
    assert_eq!(scene_for_slot(&inventory, &registry, 2), None);
}

#[test]
fn card_shortcuts_show_the_digit_and_name_the_full_binding() {
    let hotkeys = SceneHotkeyConfig::default();

    assert_eq!(
        shortcut_for_slot(&hotkeys, 0),
        Some(SceneShortcut {
            badge: "1".to_string(),
            label: "Ctrl+1".to_string(),
        })
    );
    assert_eq!(
        shortcut_for_slot(&hotkeys, 9),
        Some(SceneShortcut {
            badge: "0".to_string(),
            label: "Ctrl+0".to_string(),
        })
    );
    assert_eq!(shortcut_for_slot(&hotkeys, 10), None);
}

#[test]
fn cards_past_the_last_slot_and_disabled_hotkeys_get_no_badge() {
    let disabled = SceneHotkeyConfig {
        enabled: false,
        style: SceneHotkeyStyle::Plain,
        ..SceneHotkeyConfig::default()
    };

    assert_eq!(shortcut_for_slot(&disabled, 0), None);
}

#[test]
fn output_confirmation_defaults_match_output_actions() {
    let config = OutputConfig::default();

    assert!(!requires_output_confirmation(
        OutputKind::Stream,
        false,
        &config
    ));
    assert!(requires_output_confirmation(
        OutputKind::Stream,
        true,
        &config
    ));
    assert!(!requires_output_confirmation(
        OutputKind::Record,
        false,
        &config
    ));
    assert!(requires_output_confirmation(
        OutputKind::Record,
        true,
        &config
    ));
}

#[test]
fn output_confirmation_honors_disabled_stop_preferences() {
    let config = OutputConfig {
        confirm_stop_stream: false,
        confirm_stop_recording: false,
        ..OutputConfig::default()
    };

    assert!(!requires_output_confirmation(
        OutputKind::Stream,
        true,
        &config
    ));
    assert!(!requires_output_confirmation(
        OutputKind::Record,
        true,
        &config
    ));
}

#[test]
fn output_confirmation_honors_enabled_start_preferences() {
    let config = OutputConfig {
        confirm_start_stream: true,
        confirm_start_recording: true,
        ..OutputConfig::default()
    };

    assert!(requires_output_confirmation(
        OutputKind::Stream,
        false,
        &config
    ));
    assert!(requires_output_confirmation(
        OutputKind::Record,
        false,
        &config
    ));
}

#[test]
fn stream_start_confirmation_metadata_is_suggested() {
    assert_eq!(
        output_confirmation_dialog(OutputKind::Stream, OutputAction::Start),
        OutputConfirmationDialog {
            heading: "Start Stream?".to_string(),
            body: "OBS will start sending the live stream.".to_string(),
            confirm_label: "Start Stream".to_string(),
            appearance: OutputConfirmationAppearance::Suggested,
        }
    );
}

#[test]
fn stream_stop_confirmation_metadata_is_destructive() {
    assert_eq!(
        output_confirmation_dialog(OutputKind::Stream, OutputAction::Stop),
        OutputConfirmationDialog {
            heading: "Stop Stream?".to_string(),
            body: "OBS will stop sending the live stream.".to_string(),
            confirm_label: "Stop Stream".to_string(),
            appearance: OutputConfirmationAppearance::Destructive,
        }
    );
}

#[test]
fn recording_start_confirmation_metadata_is_suggested() {
    assert_eq!(
        output_confirmation_dialog(OutputKind::Record, OutputAction::Start),
        OutputConfirmationDialog {
            heading: "Start Recording?".to_string(),
            body: "OBS will start a new recording.".to_string(),
            confirm_label: "Start Recording".to_string(),
            appearance: OutputConfirmationAppearance::Suggested,
        }
    );
}

#[test]
fn recording_stop_confirmation_metadata_is_destructive() {
    assert_eq!(
        output_confirmation_dialog(OutputKind::Record, OutputAction::Stop),
        OutputConfirmationDialog {
            heading: "Stop Recording?".to_string(),
            body: "OBS will stop the current recording.".to_string(),
            confirm_label: "Stop Recording".to_string(),
            appearance: OutputConfirmationAppearance::Destructive,
        }
    );
}

#[test]
fn active_state_maps_to_output_action() {
    assert_eq!(output_action_for_active_state(false), OutputAction::Start);
    assert_eq!(output_action_for_active_state(true), OutputAction::Stop);
}
