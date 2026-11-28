//! `AppState` and audio-input fixtures.

use crate::controller::event::OutputCommandFailureRecovery;
use crate::controller::state::AppState;
use crate::domain::audio::AudioInput;
use crate::storage::{config::AppConfig, registry::SceneRegistry};

/// A fresh state with default config, an empty registry, no stored password
/// and no startup notice.
pub(crate) fn app_state() -> AppState {
    AppState::new(
        AppConfig::default(),
        SceneRegistry::default(),
        None,
        Vec::new(),
    )
}

/// An unmuted input at unity gain (0 dB).
pub(crate) fn input(id: &str) -> AudioInput {
    AudioInput::new(id.to_string(), false, 1.0, 0.0)
}

/// Record a failed stream command through the real reducer, leaving the
/// stream status exactly as it is.
pub(crate) fn fail_stream_command(state: &mut AppState, message: &str) {
    let unchanged = state.stream_status.clone();
    state.set_stream_command_failure_with_recovery(
        OutputCommandFailureRecovery::for_test_unchecked(message.to_string(), unchanged),
    );
}

/// Record a failed record command through the real reducer, leaving the
/// record status exactly as it is.
pub(crate) fn fail_record_command(state: &mut AppState, message: &str) {
    let unchanged = state.record_status.clone();
    state.set_record_command_failure_with_recovery(
        OutputCommandFailureRecovery::for_test_unchecked(message.to_string(), unchanged),
    );
}
