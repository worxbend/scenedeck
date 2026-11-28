use super::{
    format_mixer_inspection_line, prepare_mixer_scene_audio_request,
    should_request_mixer_scene_audio, source_summary, MixerRefreshRequestIntent,
    MixerRetryInspection,
};
use crate::controller::state::{
    AppState, MixerAudioError, MixerInspectionStatus, MixerSceneRefreshTargetReason,
    MixerVisibleAudioStatus,
};
use crate::controller::test_support::state::app_state;
use crate::domain::audio::AudioInput;
use crate::domain::mixer::MixerMode;
use crate::services::audio_service::AudioService;

#[test]
fn mixer_structure_ignores_audio_echoes_but_tracks_membership_and_scope() {
    let mut state = app_state();
    state.apply_active_audio(
        None,
        Ok(vec![AudioInput::new("Mic".into(), false, 1.0, 0.0)]),
    );
    let initial = super::mixer_structure(&state);
    state.audio_inputs[0].muted = true;
    state.audio_inputs[0].volume_mul = 0.5;
    state.audio_inputs[0].volume_db = -6.0;
    state.mixer.locked_inputs.insert("Mic".into());
    assert_eq!(initial, super::mixer_structure(&state));
    state
        .audio_inputs
        .push(AudioInput::new("Other".into(), false, 1.0, 0.0));
    assert_ne!(initial, super::mixer_structure(&state));
    state.audio_inputs.pop();
    state.workspace.epoch += 1;
    assert_ne!(initial, super::mixer_structure(&state));
}

fn summary_target_details(state: &AppState) -> Option<(String, MixerSceneRefreshTargetReason)> {
    state
        .mixer_scene_refresh_target_details()
        .map(|target| (target.scene.to_string(), target.reason))
}

fn mixer_summary(state: &AppState) -> String {
    source_summary(
        state.mixer.mode,
        state.scene_inventory.current_id.as_deref(),
        summary_target_details(state),
    )
}

fn mixer_error() -> MixerAudioError {
    MixerAudioError {
        scene: "scene-a".to_string(),
        message: "failed".to_string(),
    }
}

fn loaded_status() -> MixerVisibleAudioStatus<'static> {
    MixerVisibleAudioStatus::Loaded(&[])
}

fn input(id: &str, muted: bool, volume_mul: f64, volume_db: f64) -> AudioInput {
    let mut input = AudioInput::new(id.to_string(), muted, volume_mul, volume_db);
    input.display_name = format!("{id} Display");
    input
}

fn inspection_json(line: &str) -> serde_json::Value {
    let payload = line
        .strip_prefix("scenedeck_mixer_inspect ")
        .expect("inspection line prefix");
    serde_json::from_str(payload).expect("valid inspection json")
}

fn inspection_status_kind(json: &serde_json::Value) -> &str {
    json["status"]["kind"]
        .as_str()
        .expect("inspection status kind")
}

#[test]
fn active_mode_summary_follows_active_obs_scene() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::ActiveScene;
    state.scene_inventory.current_id = Some("Program".to_string());
    state.mixer.selected_scene = Some("Selected".to_string());
    state.mixer.pinned_scene = Some("Pinned".to_string());

    assert_eq!(mixer_summary(&state), "Following active OBS scene: Program");
}

#[test]
fn selected_mode_summary_names_explicit_selected_scene() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::SelectedScene;
    state.scene_inventory.current_id = Some("Program".to_string());
    state.mixer.selected_scene = Some("Selected".to_string());

    assert_eq!(mixer_summary(&state), "Selected scene: Selected");
}

#[test]
fn pinned_mode_summary_names_explicit_pinned_scene() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::PinnedScene;
    state.scene_inventory.current_id = Some("Program".to_string());
    state.mixer.selected_scene = Some("Selected".to_string());
    state.mixer.pinned_scene = Some("Pinned".to_string());

    assert_eq!(mixer_summary(&state), "Pinned scene: Pinned");
}

#[test]
fn selected_mode_summary_describes_current_scene_fallback() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::SelectedScene;
    state.scene_inventory.current_id = Some("Program".to_string());

    assert_eq!(
        mixer_summary(&state),
        "Selected scene not set; using active OBS scene: Program"
    );
}

#[test]
fn pinned_mode_summary_describes_selected_scene_fallback() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::PinnedScene;
    state.scene_inventory.current_id = Some("Program".to_string());
    state.mixer.selected_scene = Some("Selected".to_string());

    assert_eq!(
        mixer_summary(&state),
        "Pinned scene not set; using selected scene: Selected"
    );
}

#[test]
fn pinned_mode_summary_describes_current_scene_fallback() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::PinnedScene;
    state.scene_inventory.current_id = Some("Program".to_string());

    assert_eq!(
        mixer_summary(&state),
        "Pinned and selected scenes not set; using active OBS scene: Program"
    );
}

#[test]
fn scene_specific_mode_summary_reports_no_scene_without_target_or_fallback() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::SelectedScene;

    assert_eq!(mixer_summary(&state), "No scene selected");
}

#[test]
fn mixer_inspection_line_reports_visible_cards() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::ActiveScene;
    state.scene_inventory.current_id = Some("Program".to_string());
    state.audio_inputs = vec![
        input("Music", true, 0.5, -6.24),
        input("Mic", false, 1.0, 0.0),
    ];
    let visible_cards = vec![state.audio_inputs[0].clone()];
    let snapshot = state.mixer_inspection_snapshot();

    let json = inspection_json(&format_mixer_inspection_line(
        &snapshot,
        MixerInspectionStatus::LoadedWithVisibleInputCards,
        &visible_cards,
        MixerRetryInspection::HIDDEN,
    ));

    assert_eq!(json["event"], "mixer_inspect");
    assert_eq!(json["mode"], "active");
    assert_eq!(json["render_source"], "active_scene");
    assert_eq!(json["render_scene"], "Program");
    assert_eq!(json["status"]["kind"], "loaded_with_visible_input_cards");
    assert_eq!(json["retry"]["visible"], false);
    let cards = json["visible_cards"].as_array().unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0]["name"], "Music");
    assert_eq!(cards[0]["display_name"], "Music Display");
    assert_eq!(cards[0]["muted"], true);
    assert_eq!(cards[0]["volume_mul"], 0.5);
    assert_eq!(cards[0]["volume_db"], -6.24);
    assert_eq!(cards[0]["volume_label"], "-6.2 dB");
}

#[test]
fn mixer_inspection_line_volume_labels_match_audio_card_formatter() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::ActiveScene;
    state.scene_inventory.current_id = Some("Program".to_string());
    let volume_cases = [
        ("NegInf", f64::NEG_INFINITY),
        ("BelowFloor", -120.0),
        ("NearZeroPositive", 0.01),
        ("NearZeroNegative", -0.01),
        ("Zero", 0.0),
        ("AboveObsMax", 6.0),
        ("Normal", -6.24),
    ];
    state.audio_inputs = volume_cases
        .iter()
        .map(|(name, volume_db)| input(name, false, 1.0, *volume_db))
        .collect();
    let visible_cards = state.audio_inputs.clone();
    let snapshot = state.mixer_inspection_snapshot();

    let json = inspection_json(&format_mixer_inspection_line(
        &snapshot,
        MixerInspectionStatus::LoadedWithVisibleInputCards,
        &visible_cards,
        MixerRetryInspection::HIDDEN,
    ));

    let cards = json["visible_cards"].as_array().unwrap();
    assert_eq!(cards.len(), volume_cases.len());
    for ((name, volume_db), card) in volume_cases.iter().zip(cards) {
        assert_eq!(card["name"], *name);
        assert_eq!(
            card["volume_label"],
            AudioService::format_db(AudioService::sanitize_volume_db(*volume_db)),
            "inspection label should match rendered audio-card formatter for {name}"
        );
    }
}

#[test]
fn mixer_inspection_line_volume_label_uses_visible_card_volume() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::ActiveScene;
    state.scene_inventory.current_id = Some("Program".to_string());
    state.audio_inputs = vec![input("Mic", false, 1.0, -120.0)];
    let snapshot = state.mixer_inspection_snapshot();
    let visible_cards = vec![input("Mic", false, 0.5, -6.24)];

    let json = inspection_json(&format_mixer_inspection_line(
        &snapshot,
        MixerInspectionStatus::LoadedWithVisibleInputCards,
        &visible_cards,
        MixerRetryInspection::HIDDEN,
    ));

    let cards = json["visible_cards"].as_array().unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0]["name"], "Mic");
    assert_eq!(cards[0]["volume_db"], -6.24);
    assert_eq!(cards[0]["volume_label"], "-6.2 dB");
}

#[test]
fn mixer_inspection_line_reports_error_and_retry_state() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::SelectedScene;
    state.mixer.selected_scene = Some("Scene A".to_string());
    let request = state.set_mixer_audio_loading("Scene A".to_string());
    state.set_mixer_audio_failure(request, "OBS failed".to_string());
    let snapshot = state.mixer_inspection_snapshot();

    let json = inspection_json(&format_mixer_inspection_line(
        &snapshot,
        MixerInspectionStatus::ErrorPlaceholderShown("OBS failed"),
        &[],
        MixerRetryInspection::VISIBLE_ENABLED,
    ));

    assert_eq!(json["mode"], "selected");
    assert_eq!(json["refresh_target"], "Scene A");
    assert_eq!(json["refresh_reason"], "direct_selected_scene");
    assert_eq!(json["render_source"], "scene");
    assert_eq!(json["render_scene"], "Scene A");
    assert_eq!(json["status"]["kind"], "error");
    assert_eq!(json["status"]["message"], "OBS failed");
    assert_eq!(json["visible_cards"].as_array().unwrap().len(), 0);
    assert_eq!(json["retry"]["visible"], true);
    assert_eq!(json["retry"]["enabled"], true);
}

#[test]
fn mixer_inspection_line_reports_loading_placeholder_after_missing_automatic_request() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::SelectedScene;
    state.mixer.selected_scene = Some("Scene A".to_string());
    let snapshot = state.mixer_inspection_snapshot();
    assert_eq!(snapshot.status, MixerInspectionStatus::MissingNoTarget);

    let mut tracked_scene = None;
    let command = prepare_mixer_scene_audio_request(
        MixerRefreshRequestIntent::Automatic,
        "Scene A",
        MixerVisibleAudioStatus::Missing,
        &mut tracked_scene,
    );
    assert_eq!(command.as_deref(), Some("Scene A"));
    assert_eq!(tracked_scene.as_deref(), Some("Scene A"));

    let json = inspection_json(&format_mixer_inspection_line(
        &snapshot,
        MixerInspectionStatus::LoadingPlaceholderShown,
        &[],
        MixerRetryInspection::HIDDEN,
    ));

    assert_eq!(json["mode"], "selected");
    assert_eq!(json["refresh_target"], "Scene A");
    assert_eq!(json["refresh_reason"], "direct_selected_scene");
    assert_eq!(json["render_source"], "scene");
    assert_eq!(json["render_scene"], "Scene A");
    assert_eq!(inspection_status_kind(&json), "loading_placeholder_shown");
    assert_ne!(inspection_status_kind(&json), "missing_no_target");
    assert_eq!(json["visible_cards"].as_array().unwrap().len(), 0);
    assert_eq!(json["retry"]["visible"], false);
}

#[test]
fn mixer_inspection_line_reports_loaded_empty_audio_sources() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::SelectedScene;
    state.mixer.selected_scene = Some("Scene A".to_string());
    let request = state.set_mixer_audio_loading("Scene A".to_string());
    state.set_mixer_audio_success(request, Vec::new());
    let snapshot = state.mixer_inspection_snapshot();
    assert_eq!(snapshot.status, MixerInspectionStatus::LoadedNoAudioSources);

    let json = inspection_json(&format_mixer_inspection_line(
        &snapshot,
        MixerInspectionStatus::LoadedNoAudioSources,
        &[],
        MixerRetryInspection::HIDDEN,
    ));

    assert_eq!(json["mode"], "selected");
    assert_eq!(json["render_source"], "scene");
    assert_eq!(json["render_scene"], "Scene A");
    assert_eq!(inspection_status_kind(&json), "loaded_no_audio_sources");
    assert_ne!(inspection_status_kind(&json), "loading_placeholder_shown");
    assert_ne!(inspection_status_kind(&json), "missing_no_target");
    assert_ne!(inspection_status_kind(&json), "error");
    assert_ne!(
        inspection_status_kind(&json),
        "loaded_no_matching_audio_sources_after_filtering"
    );
    assert_eq!(json["visible_cards"].as_array().unwrap().len(), 0);
    assert_eq!(json["retry"]["visible"], false);
}

#[test]
fn mixer_inspection_line_reports_loaded_filtered_empty_audio_sources() {
    let mut state = app_state();
    state.mixer.mode = MixerMode::SelectedScene;
    state.mixer.selected_scene = Some("Scene A".to_string());
    state.mixer.search = "does-not-match".to_string();
    let request = state.set_mixer_audio_loading("Scene A".to_string());
    state.set_mixer_audio_success(request, vec![input("Mic", false, 1.0, 0.0)]);
    let snapshot = state.mixer_inspection_snapshot();
    assert_eq!(
        snapshot.status,
        MixerInspectionStatus::LoadedWithVisibleInputCards
    );

    let json = inspection_json(&format_mixer_inspection_line(
        &snapshot,
        MixerInspectionStatus::LoadedNoMatchingAudioSourcesAfterFiltering,
        &[],
        MixerRetryInspection::HIDDEN,
    ));

    assert_eq!(json["mode"], "selected");
    assert_eq!(json["render_source"], "scene");
    assert_eq!(json["render_scene"], "Scene A");
    assert_eq!(
        inspection_status_kind(&json),
        "loaded_no_matching_audio_sources_after_filtering"
    );
    assert_ne!(inspection_status_kind(&json), "loaded_no_audio_sources");
    assert_ne!(inspection_status_kind(&json), "loading_placeholder_shown");
    assert_ne!(inspection_status_kind(&json), "missing_no_target");
    assert_ne!(inspection_status_kind(&json), "error");
    assert_eq!(json["visible_cards"].as_array().unwrap().len(), 0);
    assert_eq!(json["retry"]["visible"], false);
}

#[test]
fn automatic_request_dedupes_matching_failure() {
    let error = mixer_error();

    assert!(!should_request_mixer_scene_audio(
        MixerRefreshRequestIntent::Automatic,
        MixerVisibleAudioStatus::Error(&error),
        "scene-a",
        None,
    ));
}

#[test]
fn explicit_request_retries_matching_failure() {
    let error = mixer_error();

    assert!(should_request_mixer_scene_audio(
        MixerRefreshRequestIntent::Explicit,
        MixerVisibleAudioStatus::Error(&error),
        "scene-a",
        None,
    ));
}

#[test]
fn request_dedupes_loaded_scene() {
    for intent in [
        MixerRefreshRequestIntent::Automatic,
        MixerRefreshRequestIntent::Explicit,
    ] {
        assert!(!should_request_mixer_scene_audio(
            intent,
            loaded_status(),
            "scene-a",
            None,
        ));
    }
}

#[test]
fn request_dedupes_in_flight_scene() {
    for intent in [
        MixerRefreshRequestIntent::Automatic,
        MixerRefreshRequestIntent::Explicit,
    ] {
        assert!(!should_request_mixer_scene_audio(
            intent,
            MixerVisibleAudioStatus::Loading,
            "scene-a",
            None,
        ));
    }
}

#[test]
fn request_dedupes_tracked_scene() {
    for intent in [
        MixerRefreshRequestIntent::Automatic,
        MixerRefreshRequestIntent::Explicit,
    ] {
        assert!(!should_request_mixer_scene_audio(
            intent,
            MixerVisibleAudioStatus::Missing,
            "scene-a",
            Some("scene-a"),
        ));
    }
}

#[test]
fn adapter_tracks_and_dispatches_missing_request_once() {
    let mut tracked_scene = None;

    let command = prepare_mixer_scene_audio_request(
        MixerRefreshRequestIntent::Automatic,
        "scene-a",
        MixerVisibleAudioStatus::Missing,
        &mut tracked_scene,
    );

    assert_eq!(command.as_deref(), Some("scene-a"));
    assert_eq!(tracked_scene.as_deref(), Some("scene-a"));

    let command = prepare_mixer_scene_audio_request(
        MixerRefreshRequestIntent::Automatic,
        "scene-a",
        MixerVisibleAudioStatus::Missing,
        &mut tracked_scene,
    );

    assert_eq!(command, None);
    assert_eq!(tracked_scene.as_deref(), Some("scene-a"));
}

#[test]
fn adapter_does_not_loop_automatic_rebuild_after_failure() {
    let mut tracked_scene = None;
    let error = mixer_error();

    let command = prepare_mixer_scene_audio_request(
        MixerRefreshRequestIntent::Automatic,
        "scene-a",
        MixerVisibleAudioStatus::Error(&error),
        &mut tracked_scene,
    );

    assert_eq!(command, None);
    assert_eq!(tracked_scene, None);
}

#[test]
fn adapter_allows_one_explicit_retry_after_failure() {
    let mut tracked_scene = None;
    let error = mixer_error();

    let command = prepare_mixer_scene_audio_request(
        MixerRefreshRequestIntent::Explicit,
        "scene-a",
        MixerVisibleAudioStatus::Error(&error),
        &mut tracked_scene,
    );

    assert_eq!(command.as_deref(), Some("scene-a"));
    assert_eq!(tracked_scene.as_deref(), Some("scene-a"));

    let command = prepare_mixer_scene_audio_request(
        MixerRefreshRequestIntent::Explicit,
        "scene-a",
        MixerVisibleAudioStatus::Error(&error),
        &mut tracked_scene,
    );

    assert_eq!(command, None);
}

#[test]
fn adapter_dedupes_explicit_retry_while_loading() {
    let mut tracked_scene = None;

    let command = prepare_mixer_scene_audio_request(
        MixerRefreshRequestIntent::Explicit,
        "scene-a",
        MixerVisibleAudioStatus::Loading,
        &mut tracked_scene,
    );

    assert_eq!(command, None);
    assert_eq!(tracked_scene, None);
}

#[test]
fn adapter_dedupes_explicit_retry_while_tracked() {
    let mut tracked_scene = Some("scene-a".to_string());
    let error = mixer_error();

    let command = prepare_mixer_scene_audio_request(
        MixerRefreshRequestIntent::Explicit,
        "scene-a",
        MixerVisibleAudioStatus::Error(&error),
        &mut tracked_scene,
    );

    assert_eq!(command, None);
    assert_eq!(tracked_scene.as_deref(), Some("scene-a"));
}
