//! `AppState` and audio-input fixtures.

use crate::controller::state::AppState;
use crate::domain::audio::AudioInput;
use crate::storage::{config::AppConfig, registry::SceneRegistry};

/// A fresh state with default config, an empty registry, no stored password
/// and no startup notice.
pub(crate) fn app_state() -> AppState {
    AppState::new(AppConfig::default(), SceneRegistry::default(), None, None)
}

/// An unmuted input at unity gain (0 dB).
pub(crate) fn input(id: &str) -> AudioInput {
    AudioInput::new(id.to_string(), false, 1.0, 0.0)
}
