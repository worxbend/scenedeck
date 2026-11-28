use super::*;
use crate::domain::{
    control::{ControlSnapshot, SourceDetail, SourceFilter, SourceItem, StudioState},
    role::SceneRole,
    scene::Scene,
};
use crate::storage::registry::SceneEntry;
use std::time::{Duration, Instant};

pub(crate) fn connected() -> AppState {
    let mut state = crate::controller::test_support::state::app_state();
    state.obs_status = ObsStatus::Connected {
        obs_version: "fixture".into(),
    };
    state.scene_collections.current = Some("Show".into());
    state.scene_inventory.scenes.push(Scene {
        id: "Camera <b>One</b>".into(),
        uuid: Some("scene-1".into()),
    });
    state.registry.scenes.insert(
        "Camera <b>One</b>".into(),
        SceneEntry {
            role: Some(SceneRole::Primary),
            tags: vec!["Interview".into()],
            ..Default::default()
        },
    );
    state.show_registry.status = RegistryStatus::Ready;
    state.workspace.snapshot = Some(ControlSnapshot {
        studio: StudioState {
            enabled: Some(true),
            ..Default::default()
        },
        replay_active: Some(true),
        items: vec![SourceItem {
            owner: "Camera <b>One</b>".into(),
            item_id: 1,
            input: "Microphone".into(),
            input_uuid: "input-1".into(),
            kind: None,
            path: vec![],
            enabled: true,
            ancestor_enabled: true,
            is_group: false,
            is_scene: false,
        }],
        ..Default::default()
    });
    state.workspace.observed_at = Some(Instant::now());
    state.workspace.detail = Some(SourceDetail {
        input: "Microphone".into(),
        muted: Some(false),
        filters: vec![SourceFilter {
            name: "Limiter".into(),
            kind: "limiter_filter".into(),
            enabled: false,
        }],
        ..Default::default()
    });
    state.workspace.input_catalog = Some(Ok(vec![crate::domain::operator_input::InputIdentity {
        target: ActionTarget {
            name: "Microphone".into(),
            uuid: Some("input-1".into()),
        },
        kind: "audio".into(),
    }]));
    state.workspace.audio =
        vec![
            crate::domain::audio::AudioInput::new("Microphone".into(), false, 1.0, 0.0)
                .with_identity(Some("input-1".into())),
        ];
    state.workspace.operator_inputs = vec![crate::domain::operator_input::InputObservation {
        target: ActionTarget {
            name: "Microphone".into(),
            uuid: Some("input-1".into()),
        },
        media: None,
        filters: Some(Ok(state.workspace.detail.as_ref().unwrap().filters.clone())),
    }];
    state.available_requests = [
        "SetCurrentPreviewScene",
        "SetCurrentProgramScene",
        "SetInputMute",
        "SaveReplayBuffer",
        "SetSourceFilterEnabled",
    ]
    .map(str::to_owned)
    .into();
    state
}

fn scene() -> OperatorAction {
    OperatorAction::Scene {
        target: ActionTarget {
            name: "Camera <b>One</b>".into(),
            uuid: Some("scene-1".into()),
        },
    }
}

#[test]
fn search_includes_tags_and_hidden_scenes_and_navigation_works_offline() {
    let mut state = connected();
    state.registry.scene_filter.query = "no match".into();
    let filtered: Vec<_> = entries(&state)
        .into_iter()
        .filter(|e| e.matches("INTERVIEW camera"))
        .collect();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].available);
    assert!(filtered[0].label.contains("<b>One</b>"));
    assert!(!filtered[0].matches("interview missing"));
    state.obs_status = ObsStatus::Disconnected;
    for entry in entries(&state) {
        assert_eq!(
            entry.available,
            matches!(
                entry.action,
                CatalogAction::Page(_) | CatalogAction::Bookmarks | CatalogAction::Reports
            )
        );
    }
}

#[test]
fn scene_resolution_retains_identity_and_requires_role_freshness_and_capability() {
    let mut state = connected();
    assert_eq!(
        resolve(&state, &scene()),
        Some(Resolved::Control(ControlAction::ActivateScene {
            scene: "Camera <b>One</b>".into(),
            studio_enabled: true
        }))
    );
    state.scene_inventory.scenes[0].id = "Renamed".into();
    let entry = state.registry.scenes.remove("Camera <b>One</b>").unwrap();
    state.registry.scenes.insert("Renamed".into(), entry);
    assert_eq!(
        resolve(&state, &scene()),
        Some(Resolved::Control(ControlAction::ActivateScene {
            scene: "Renamed".into(),
            studio_enabled: true
        }))
    );
    state.scene_inventory.scenes[0].uuid = Some("replacement".into());
    assert_eq!(resolve(&state, &scene()), None);
    state = connected();
    state
        .scene_inventory
        .scenes
        .push(state.scene_inventory.scenes[0].clone());
    assert_eq!(resolve(&state, &scene()), None);
    state = connected();
    state.workspace.observed_at = Some(Instant::now() - Duration::from_secs(31));
    assert_eq!(resolve(&state, &scene()), None);
    state = connected();
    state.available_requests.clear();
    assert_eq!(resolve(&state, &scene()), None);
    state = connected();
    state
        .registry
        .scenes
        .get_mut("Camera <b>One</b>")
        .unwrap()
        .role = Some(SceneRole::Module);
    assert_eq!(resolve(&state, &scene()), None);
    state = connected();
    state.show_registry.status = RegistryStatus::Loading;
    assert_eq!(resolve(&state, &scene()), None);
}

#[test]
fn source_intents_never_invert_or_target_a_replacement() {
    let mut state = connected();
    let target = ActionTarget {
        name: "Microphone".into(),
        uuid: Some("input-1".into()),
    };
    let mute = OperatorAction::SetMuted {
        target: target.clone(),
        muted: true,
    };
    assert!(resolve(&state, &mute).is_some());
    state.workspace.audio[0].muted = true;
    assert_eq!(
        resolve(&state, &mute),
        Some(Resolved::Control(ControlAction::SetInputMuted {
            input: "Microphone".into(),
            muted: true
        }))
    );
    let filter = OperatorAction::SetFilterEnabled {
        target: target.clone(),
        filter: "Limiter".into(),
        enabled: true,
    };
    assert!(resolve(&state, &filter).is_some());
    state.workspace.operator_inputs[0]
        .filters
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()[0]
        .enabled = true;
    assert_eq!(
        resolve(&state, &filter),
        Some(Resolved::Control(ControlAction::SetFilterEnabled {
            input: "Microphone".into(),
            filter: "Limiter".into(),
            enabled: true
        }))
    );
    let open = OperatorAction::OpenSource { target };
    assert!(resolve(&state, &open).is_some());
    let duplicate = state.workspace.snapshot.as_ref().unwrap().items[0].clone();
    state
        .workspace
        .snapshot
        .as_mut()
        .unwrap()
        .items
        .push(duplicate);
    assert!(
        resolve(&state, &open).is_some(),
        "repeated uses of one input are one target"
    );
    let mut conflicting = state
        .workspace
        .input_catalog
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()[0]
        .clone();
    conflicting.target.uuid = Some("different".into());
    state
        .workspace
        .input_catalog
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .push(conflicting);
    assert!(
        resolve(&state, &open).is_none(),
        "ambiguous names cannot be dispatched by name"
    );
    state = connected();
    state
        .workspace
        .input_catalog
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()[0]
        .target
        .uuid = Some("replacement".into());
    assert!(resolve(&state, &open).is_none());
}

#[test]
fn audio_actions_keep_identity_across_pages_and_renames() {
    use crate::domain::audio::AudioInput;
    let mut state = connected();
    state.current_page = Page::Settings;
    state.workspace.snapshot.as_mut().unwrap().items.clear();
    state.workspace.detail = None;
    state.workspace.audio =
        vec![AudioInput::new("Microphone".into(), false, 1.0, 0.0)
            .with_identity(Some("input-1".into()))];
    let action = OperatorAction::SetMuted {
        target: ActionTarget {
            name: "Microphone".into(),
            uuid: Some("input-1".into()),
        },
        muted: true,
    };
    assert!(entries(&state)
        .iter()
        .any(|entry| entry.action == CatalogAction::Operator(action.clone()) && entry.available));
    state.workspace.audio[0].id = "Renamed".into();
    state
        .workspace
        .input_catalog
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()[0]
        .target
        .name = "Renamed".into();
    assert_eq!(
        resolve(&state, &action),
        Some(Resolved::Control(ControlAction::SetInputMuted {
            input: "Renamed".into(),
            muted: true
        }))
    );
    state.workspace.audio[0].uuid = Some("replacement".into());
    assert_eq!(resolve(&state, &action), None);
    state.workspace.audio[0].uuid = Some("input-1".into());
    state.workspace.observed_at = None;
    assert_eq!(resolve(&state, &action), None);
}

#[test]
fn replay_and_source_controls_require_current_observations() {
    let mut state = connected();
    assert!(resolve(&state, &OperatorAction::SaveReplay).is_some());
    for mutate in [
        (|s: &mut AppState| s.workspace.pending = true) as fn(&mut AppState),
        |s| s.workspace.dirty = true,
        |s| s.workspace.error = Some("failed".into()),
        |s| s.workspace.snapshot.as_mut().unwrap().replay_active = Some(false),
        |s| s.available_requests.clear(),
        |s| s.scene_collections.current = None,
    ] {
        state = connected();
        mutate(&mut state);
        assert!(resolve(&state, &OperatorAction::SaveReplay).is_none());
    }
    let target = ActionTarget {
        name: "Old".into(),
        uuid: Some("id".into()),
    };
    assert!(target.matches("Renamed", Some("id")));
    assert!(!target.matches("Old", None));
    assert!(!ActionTarget {
        name: "Old".into(),
        uuid: None
    }
    .matches("Old", Some("new")));
}

#[test]
fn media_and_filters_use_targeted_reads_without_sources_selection() {
    use crate::domain::control::{MediaState, MediaStatus};
    let mut state = connected();
    state.current_page = Page::Mixer;
    state.workspace.selected_input = Some("Unrelated editor".into());
    state.workspace.detail = None;
    state.workspace.snapshot.as_mut().unwrap().items.clear();
    state
        .available_requests
        .push("TriggerMediaInputAction".into());
    let target = state.workspace.operator_inputs[0].target.clone();
    let media = OperatorAction::RestartMedia {
        target: target.clone(),
    };
    let filter = OperatorAction::SetFilterEnabled {
        target,
        filter: "Limiter".into(),
        enabled: true,
    };
    state.workspace.operator_inputs[0].media = Some(Ok(MediaStatus {
        state: MediaState::Playing,
        duration_ms: Some(5000),
        cursor_ms: Some(0),
    }));
    for action in [&media, &filter] {
        assert!(resolve(&state, action).is_some());
        assert!(entries(&state).iter().any(|entry| entry.action
            == CatalogAction::Operator(action.clone())
            && entry.available));
    }
    state.workspace.operator_inputs[0].filters = Some(Err("unavailable".into()));
    assert!(resolve(&state, &filter).is_none());
    assert!(resolve(&state, &media).is_some());
    state.workspace.input_catalog = Some(Err("inventory unavailable".into()));
    assert!(
        resolve(&state, &media).is_none(),
        "old targeted data cannot authorize a write without inventory"
    );
    state.workspace.invalidate();
    assert!(state.workspace.operator_inputs.is_empty());
    assert!(state.workspace.input_catalog.is_none());
}

#[test]
fn pin_state_reports_observations_instead_of_requested_values() {
    let mut state = connected();
    let target = state.workspace.operator_inputs[0].target.clone();
    let mute = OperatorAction::SetMuted {
        target: target.clone(),
        muted: true,
    };
    let filter = OperatorAction::SetFilterEnabled {
        target,
        filter: "Limiter".into(),
        enabled: true,
    };
    assert_eq!(
        observed_label(&state, &mute),
        Some(fl!(LANGUAGE_LOADER, "quick-unmuted"))
    );
    assert_eq!(
        observed_label(&state, &filter),
        Some(fl!(LANGUAGE_LOADER, "quick-filter-disabled"))
    );
    state.workspace.audio[0].muted = true;
    assert_eq!(
        observed_label(&state, &mute),
        Some(fl!(LANGUAGE_LOADER, "quick-muted"))
    );
    state.workspace.snapshot.as_mut().unwrap().replay_active = Some(false);
    assert_eq!(
        observed_label(&state, &OperatorAction::SaveReplay),
        Some(fl!(LANGUAGE_LOADER, "output-state-inactive"))
    );
    assert!(!binding_entry(&state, &OperatorAction::SaveReplay).available);
    state.workspace.snapshot.as_mut().unwrap().studio.program = Some("Camera <b>One</b>".into());
    assert_eq!(
        observed_label(&state, &scene()),
        Some(fl!(LANGUAGE_LOADER, "scene-card-program-label"))
    );
    for invalidate in [
        (|s: &mut AppState| s.workspace.dirty = true) as fn(&mut AppState),
        |s| s.workspace.pending = true,
        |s| s.workspace.observed_at = None,
        |s| s.scene_collections.current = None,
        |s| s.show_registry.status = RegistryStatus::Loading,
        |s| s.workspace.input_catalog = Some(Err("unavailable".into())),
        |s| {
            s.workspace
                .input_catalog
                .as_mut()
                .unwrap()
                .as_mut()
                .unwrap()[0]
                .target
                .uuid = Some("replacement".into())
        },
    ] {
        let mut state = connected();
        invalidate(&mut state);
        assert!(observed_label(&state, &mute).is_none());
    }
}

#[test]
fn toggles_resolve_to_explicit_values_only_from_current_observations() {
    let mut state = connected();
    let target = state.workspace.operator_inputs[0].target.clone();
    let mute = OperatorAction::ToggleMuted {
        target: target.clone(),
    };
    let filter = OperatorAction::ToggleFilter {
        target,
        filter: "Limiter".into(),
    };
    for value in [false, true] {
        state.workspace.audio[0].muted = value;
        state.workspace.operator_inputs[0]
            .filters
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap()[0]
            .enabled = value;
        assert_eq!(
            resolve(&state, &mute),
            Some(Resolved::Control(ControlAction::SetInputMuted {
                input: "Microphone".into(),
                muted: !value
            }))
        );
        assert_eq!(
            resolve(&state, &filter),
            Some(Resolved::Control(ControlAction::SetFilterEnabled {
                input: "Microphone".into(),
                filter: "Limiter".into(),
                enabled: !value
            }))
        );
        assert!(binding_entries(&state)
            .iter()
            .any(|entry| entry.action == CatalogAction::Operator(mute.clone())));
        assert!(binding_entries(&state)
            .iter()
            .any(|entry| entry.action == CatalogAction::Operator(filter.clone())));
    }
    state.workspace.operator_inputs[0].filters = Some(Err("unavailable".into()));
    assert!(resolve(&state, &filter).is_none());
    state.workspace.pending = true;
    assert!(resolve(&state, &mute).is_none());
}
