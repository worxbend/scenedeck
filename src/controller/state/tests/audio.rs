//! The Live audio list and the Mixer refresh snapshot kept in step.

use super::fixtures::{app_state, input, visible_loaded_input};
use crate::controller::state::{MixerAudioRefreshState, MixerAudioRefreshTransition};

#[test]
fn repeated_same_scene_requests_reject_old_success_and_failure_without_ending_loading() {
    let mut state = MixerAudioRefreshState::default();
    let first = state.loading("A".into());
    let latest = state.loading("A".into());
    assert!(first.id < latest.id);
    assert!(state.success(first.clone(), vec![input("Old")]).is_stale());
    assert!(state.failure(first, "old failure".into()).is_stale());
    assert_eq!(state.requested.as_ref(), Some(&latest));
    assert_eq!(
        state.success(latest, vec![input("New")]),
        MixerAudioRefreshTransition::Success
    );
    assert_eq!(state.loaded.as_ref().unwrap().inputs[0].id, "New");
}

#[test]
fn applying_an_input_change_updates_both_places_the_input_is_held() {
    let mut state = app_state();
    state.audio_inputs = vec![input("Mic"), input("Music")];
    let request = state.set_mixer_audio_loading("Scene A".to_string());
    state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

    state.apply_input_mute("Mic", true);
    state.apply_input_volume("Mic", 0.5, -6.0);

    // The Live page's flat list...
    let live = state
        .audio_inputs
        .iter()
        .find(|candidate| candidate.id == "Mic")
        .expect("live input");
    assert!(live.muted);
    assert_eq!(live.volume_mul, 0.5);
    assert_eq!(live.volume_db, -6.0);

    // ...and the mixer's snapshot of the scene must agree.
    let mixed = visible_loaded_input(&state, "Scene A", "Mic");
    assert!(mixed.muted);
    assert_eq!(mixed.volume_mul, 0.5);
    assert_eq!(mixed.volume_db, -6.0);

    // An input the change did not name is untouched in both.
    assert!(!visible_loaded_input(&state, "Scene A", "Music").muted);
    assert!(!state
        .audio_inputs
        .iter()
        .any(|candidate| candidate.id == "Music" && candidate.muted));
}
