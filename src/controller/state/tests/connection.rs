//! Connecting, session retirement and collection invalidation.

use std::time::Instant;

use super::fixtures::{app_state, input, seed_session_observations};
use crate::controller::state::{
    AppState, MixerAudioRefreshTransition, MixerAudioRequest, MixerVisibleAudioStatus, ObsStatus,
    Page, TopologyStatus, WorkspaceState,
};
use crate::domain::appearance::ThemeMode;
use crate::domain::graph::SceneGraph;
use crate::domain::meter::InputLevels;
use crate::domain::mixer::MixerMode;
use crate::domain::output::OutputStatus;

fn seed_collection_observations(state: &mut AppState) -> MixerAudioRequest {
    state
        .scene_inventory
        .scenes
        .push(crate::domain::scene::Scene {
            uuid: None,
            id: "A".into(),
        });
    state.scene_inventory.current_id = Some("A".into());
    state.scene_inventory.previous_id = Some("Previous".into());
    state.set_topology_ready(SceneGraph {
        edges: std::collections::BTreeMap::from([("A".into(), vec!["Nested".into()])]),
    });
    state.audio_inputs = vec![input("Mic")];
    state.set_audio_levels(vec![InputLevels::from_mul("Mic".into(), &[[0.5; 3]])]);
    state.mixer.locked_inputs.insert("Mic".into());
    let loaded = state.set_mixer_audio_loading("A".into());
    state.set_mixer_audio_success(loaded, vec![input("Old Mic")]);
    let failed = state.set_mixer_audio_loading("A".into());
    state.set_mixer_audio_failure(failed, "old error".into());
    state.workspace = WorkspaceState {
        dirty: true,
        epoch: 7,
        sequence: 12,
        revision: 20,
        loading: true,
        pending: true,
        error: Some("old workspace error".into()),
        selected_scene: Some("A".into()),
        selected_input: Some("Mic".into()),
        snapshot: Some(crate::domain::control::ControlSnapshot::default()),
        detail: Some(crate::domain::control::SourceDetail::default()),
        usage: vec![crate::domain::control::SourceItem {
            owner: "A".into(),
            item_id: 1,
            input: "Mic".into(),
            input_uuid: "fixture".into(),
            kind: Some("audio".into()),
            path: vec!["A".into()],
            enabled: true,
            ancestor_enabled: true,
            is_group: false,
            is_scene: false,
        }],
        warnings: vec![crate::domain::control::WorkspaceWarning::InventoryTruncated],
        audio: vec![input("Mic")],
        observed_at: Some(Instant::now()),
        screenshot: Some("old image".into()),
        screenshot_scene: Some("A".into()),
        screenshot_observed_at: Some(Instant::now()),
        result: "old result".into(),
        recording_stop_result: Some(1),
        ..Default::default()
    };
    let scope = state.workflow_scope();
    state.workspace.output_results.request(
        crate::domain::output_result::OutputResultKind::RecordingStop,
        scope.clone(),
        1,
    );
    state.workspace.output_results.observe(
        crate::domain::output_result::OutputResultKind::ReplaySave,
        scope,
        Some("old-replay.mkv".into()),
        2,
    );
    state.workspace.log(
        crate::domain::workflow::ActivityOrigin::Observed,
        "old observation".into(),
    );
    state.set_mixer_audio_loading("B".into())
}

fn assert_collection_observations_cleared(state: &AppState, revision: u64) {
    assert!(state.scene_inventory.scenes.is_empty());
    assert_eq!(state.scene_inventory.current_id, None);
    assert_eq!(state.scene_inventory.previous_id, None);
    assert!(state.scene_graph.edges.is_empty());
    assert_eq!(state.topology_status, TopologyStatus::Missing);
    assert!(state.audio_inputs.is_empty() && state.audio_levels.is_empty());
    assert!(state.mixer_audio_refresh.loaded.is_none());
    assert!(state.mixer_audio_refresh.error.is_none());
    assert_eq!(state.mixer_audio_refresh.requested_scene(), None);
    assert!(state.mixer.locked_inputs.is_empty());
    assert_eq!(state.scene_collections.current, None);
    assert!(matches!(
        state.visible_mixer_audio_status("A"),
        MixerVisibleAudioStatus::Missing
    ));
    let workspace = &state.workspace;
    assert_eq!(
        (workspace.epoch, workspace.sequence, workspace.revision),
        (8, 13, revision)
    );
    assert!(!workspace.loading && !workspace.pending && !workspace.dirty);
    assert!(workspace.snapshot.is_none() && workspace.detail.is_none());
    assert!(workspace.usage.is_empty() && workspace.audio.is_empty());
    assert!(workspace.selected_scene.is_none() && workspace.selected_input.is_none());
    assert!(workspace.observed_at.is_none() && workspace.screenshot.is_none());
    assert!(workspace.screenshot_scene.is_none() && workspace.screenshot_observed_at.is_none());
    assert!(workspace.warnings.is_empty() && workspace.error.is_none());
    assert!(workspace.result.is_empty() && workspace.recording_stop_result.is_none());
    let history: Vec<_> = workspace.output_results.entries().collect();
    assert_eq!(history.len(), 2);
    assert_eq!(
        history[0].status,
        crate::domain::output_result::OutputResultStatus::Unconfirmed
    );
    assert_eq!(
        history[1].status,
        crate::domain::output_result::OutputResultStatus::Observed
    );
    assert_eq!(history[1].path.as_deref(), Some("old-replay.mkv"));
    assert_eq!(workspace.activity.entries[0].description, "old observation");
}

#[test]
fn connecting_resets_outputs_to_unknown_and_records_the_session() {
    use crate::domain::output::OutputRunState;
    let mut state = app_state();
    state.stream_status = OutputStatus::new(true, OutputRunState::Active);
    let status = state.apply_connected(crate::controller::event::ConnectionInfo {
        connection_id: "default".into(),
        endpoint: "ws://obs:4455".into(),
        obs_version: "32.2.2".into(),
        websocket_version: "5.6".into(),
        available_requests: vec!["GetVersion".into()],
        supported_image_formats: vec!["png".into()],
    });
    assert_eq!(status, state.obs_status);
    assert!(matches!(status, ObsStatus::Connected { obs_version } if obs_version == "32.2.2"));
    assert_eq!(state.stream_status.state, OutputRunState::Unknown);
    assert_eq!(state.record_status.state, OutputRunState::Unknown);
    assert_eq!(state.connected_endpoint.as_deref(), Some("ws://obs:4455"));
    assert_eq!(state.available_requests, ["GetVersion"]);
    assert!(state.workspace.dirty);
}

#[test]
fn resetting_the_session_forgets_all_runtime_caches_for_every_retirement_status() {
    for status in [
        ObsStatus::Disconnected,
        ObsStatus::Connecting,
        ObsStatus::Error("fixture".into()),
    ] {
        let mut state = app_state();
        seed_session_observations(&mut state);
        let old = seed_collection_observations(&mut state);
        assert!(
            state.mixer_audio_refresh.loaded.is_some() && state.mixer_audio_refresh.error.is_some()
        );

        state.reset_obs_session(status.clone());

        // Invalidation and the retirement notice each log a render.
        assert_collection_observations_cleared(&state, 23);
        assert_eq!(state.obs_status, status);
        assert!(state.available_requests.is_empty() && state.supported_image_formats.is_empty());
        assert!(state.connected_endpoint.is_none());
        assert!(state.profiles.items.is_empty() && state.profiles.current.is_none());
        assert!(state.scene_collections.items.is_empty());
        assert_eq!(state.stream_status, OutputStatus::default());
        assert_eq!(state.record_status, OutputStatus::default());
        assert!(state.stream_active_since.is_none() && state.record_active_since.is_none());
        assert!(
            state.last_stream_command_error.is_none() && state.last_record_command_error.is_none()
        );
        assert!(state.last_recording_path.is_none());
        assert!(state.obs_stats.is_none() && state.stream_health.is_none());
        assert!(state.stream_bitrate_kbps.is_none() && state.stats_history.is_empty());
        let new = state.set_mixer_audio_loading("B".into());
        assert!(new.id > old.id && new.scope_epoch > old.scope_epoch);
        assert!(state
            .set_mixer_audio_success(old.clone(), vec![input("Old")])
            .is_stale());
        assert!(state
            .set_mixer_audio_failure(old, "old session".into())
            .is_stale());
        assert!(matches!(
            state.visible_mixer_audio_status("B"),
            MixerVisibleAudioStatus::Loading
        ));
        assert_eq!(
            state.set_mixer_audio_success(new, vec![input("New")]),
            MixerAudioRefreshTransition::Success
        );
    }
}

#[test]
fn collection_invalidation_clears_observations_but_preserves_session_state() {
    let mut state = app_state();
    seed_session_observations(&mut state);
    let old = seed_collection_observations(&mut state);
    let before = state.clone();

    state.invalidate_collection_runtime();

    assert_collection_observations_cleared(&state, 22);
    assert_eq!(state.obs_status, before.obs_status);
    assert_eq!(state.connected_endpoint, before.connected_endpoint);
    assert_eq!(state.available_requests, before.available_requests);
    assert_eq!(
        state.supported_image_formats,
        before.supported_image_formats
    );
    assert_eq!(state.profiles.items, before.profiles.items);
    assert_eq!(state.profiles.current, before.profiles.current);
    assert_eq!(
        state.scene_collections.items,
        before.scene_collections.items
    );
    assert_eq!(state.stream_status, before.stream_status);
    assert_eq!(state.record_status, before.record_status);
    assert_eq!(state.stream_active_since, before.stream_active_since);
    assert_eq!(state.record_active_since, before.record_active_since);
    assert_eq!(state.last_recording_path, before.last_recording_path);
    assert_eq!(
        state.last_stream_command_error,
        before.last_stream_command_error
    );
    assert_eq!(
        state.last_record_command_error,
        before.last_record_command_error
    );
    assert_eq!(state.obs_stats, before.obs_stats);
    assert_eq!(state.stream_health, before.stream_health);
    assert_eq!(state.stream_bitrate_kbps, before.stream_bitrate_kbps);
    assert_eq!(state.stats_history.len(), before.stats_history.len());
    let new = state.set_mixer_audio_loading("B".into());
    assert!(new.id > old.id && new.scope_epoch > old.scope_epoch);
    assert!(state
        .set_mixer_audio_success(old, vec![input("Old collection")])
        .is_stale());
    assert!(matches!(
        state.visible_mixer_audio_status("B"),
        MixerVisibleAudioStatus::Loading
    ));
    assert_eq!(
        state.set_mixer_audio_success(new, vec![input("New collection")]),
        MixerAudioRefreshTransition::Success
    );
}

#[test]
fn session_and_collection_invalidation_preserve_local_preferences() {
    for retire_session in [false, true] {
        let mut state = app_state();
        state.config.obs.host = "configured-host".into();
        state.config.appearance.mode = ThemeMode::Dark;
        state.registry.scene_order = vec!["Preferred".into()];
        state.obs_password = Some("in-memory-fixture".into());
        state.startup_notices = vec!["local notice".into()];
        state.current_page = Page::Mixer;
        state.mixer.mode = MixerMode::PinnedScene;
        state.mixer.selected_scene = Some("Preferred".into());
        state.mixer.pinned_scene = Some("Pinned".into());
        state.mixer.search = "microphone".into();
        state.mixer.grouping = crate::domain::mixer::MixerGrouping::ScenePath;
        let before = state.clone();
        seed_session_observations(&mut state);
        seed_collection_observations(&mut state);

        if retire_session {
            state.reset_obs_session(ObsStatus::Disconnected);
        } else {
            state.invalidate_collection_runtime();
        }

        assert_eq!(
            format!("{:?}", state.config),
            format!("{:?}", before.config)
        );
        assert_eq!(
            format!("{:?}", state.registry),
            format!("{:?}", before.registry)
        );
        assert_eq!(
            state
                .obs_password
                .as_ref()
                .map(|password| password.expose()),
            before
                .obs_password
                .as_ref()
                .map(|password| password.expose())
        );
        assert_eq!(state.startup_notices, before.startup_notices);
        assert_eq!(state.current_page, before.current_page);
        assert_eq!(state.mixer, before.mixer);
    }
}

#[test]
fn debug_output_of_the_state_never_contains_the_obs_password() {
    let mut state = app_state();
    state.obs_password = Some("hunter2-fixture".into());

    assert!(!format!("{state:?}").contains("hunter2-fixture"));
}
