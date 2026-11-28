//! Fixtures shared by the `AppState` tests here and the tests of the other
//! `state` child modules.

use std::time::Instant;

use crate::controller::state::{
    AppState, MixerAudioRefreshState, MixerAudioRefreshTransition, MixerAudioRequest,
    MixerInspectionInput, MixerVisibleAudioStatus, ObsStatus,
};
use crate::domain::audio::AudioInput;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::domain::stats::{ObsStats, StreamHealth};

pub(in crate::controller::state) use crate::controller::test_support::state::{app_state, input};

pub(in crate::controller::state) fn visible_loaded_input<'a>(
    state: &'a AppState,
    scene: &str,
    id: &str,
) -> &'a AudioInput {
    let MixerVisibleAudioStatus::Loaded(inputs) = state.visible_mixer_audio_status(scene) else {
        panic!("expected visible mixer inputs for {scene}");
    };

    inputs
        .iter()
        .find(|input| input.id == id)
        .expect("visible mixer input")
}

pub(in crate::controller::state) fn hidden_loaded_input<'a>(
    state: &'a AppState,
    scene: &str,
    id: &str,
) -> &'a AudioInput {
    let snapshot = state
        .mixer_audio_refresh
        .loaded
        .as_ref()
        .expect("loaded mixer snapshot");

    assert_eq!(snapshot.scene, scene);
    snapshot
        .inputs
        .iter()
        .find(|input| input.id == id)
        .expect("loaded mixer input")
}

pub(in crate::controller::state) fn loud_muted_input(id: &str) -> AudioInput {
    let mut input = AudioInput::new(id.to_string(), true, 0.5, -6.24);
    input.display_name = format!("{id} Display");
    input
}

pub(in crate::controller::state) fn assert_snapshot_input(
    input: &MixerInspectionInput<'_>,
    name: &str,
    muted: bool,
) {
    assert_eq!(input.name, name);
    assert_eq!(input.display_name, format!("{name} Display"));
    assert_eq!(input.muted, muted);
    assert_eq!(input.volume_mul, 0.5);
    assert_eq!(input.volume_db, -6.24);
}

pub(in crate::controller::state) fn complete_mixer_request(
    state: &mut MixerAudioRefreshState,
    request: MixerAudioRequest,
    succeeds: bool,
) -> MixerAudioRefreshTransition {
    if succeeds {
        state.success(request, vec![input("Newest Mic")])
    } else {
        state.failure(request, "newest failure".into())
    }
}

pub(in crate::controller::state) fn output_status(
    active: bool,
    state: OutputRunState,
) -> OutputStatus {
    OutputStatus {
        duration_ms: None,
        active,
        state,
        detail: None,
    }
}

pub(in crate::controller::state) fn obs_stats(active_fps: f64) -> ObsStats {
    ObsStats {
        disk_space_mb: None,
        configured_fps: None,
        cpu_usage_percent: 10.0,
        memory_usage_mb: 400.0,
        active_fps,
        average_frame_render_time_ms: 3.0,
        render_skipped_frames: 0,
        render_total_frames: 100,
        output_skipped_frames: 0,
        output_total_frames: 100,
    }
}

pub(in crate::controller::state) fn stream_health() -> StreamHealth {
    StreamHealth {
        active: true,
        reconnecting: false,
        congestion: 0.1,
        skipped_frames: 2,
        total_frames: 200,
        bytes: 2_048,
    }
}

pub(in crate::controller::state) fn seed_session_observations(state: &mut AppState) {
    state.obs_status = ObsStatus::Connected {
        obs_version: "fixture".into(),
    };
    state.connected_endpoint = Some("fixture:4455".into());
    state.available_requests = vec!["GetStats".into()];
    state.supported_image_formats = vec!["png".into()];
    state.profiles = ObsNamedList {
        items: vec!["Profile".into()],
        current: Some("Profile".into()),
    };
    state.scene_collections = ObsNamedList {
        items: vec!["Collection".into()],
        current: Some("Collection".into()),
    };
    state.set_stream_status(output_status(true, OutputRunState::Active));
    state.set_record_status(output_status(true, OutputRunState::Active));
    state.stream_active_since = Some(Instant::now());
    state.record_active_since = Some(Instant::now());
    state.last_recording_path = Some("old-recording.mkv".into());
    state.set_stream_command_failure("old stream failure".into());
    state.set_record_command_failure("old record failure".into());
    state.set_obs_stats(obs_stats(60.0), Some(6000.0), Some(stream_health()));
}
