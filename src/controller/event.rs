use crate::domain::audio::{AudioInput, InputId};
use crate::domain::diagnostic::Diagnostic;
use crate::domain::graph::SceneGraph;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::domain::scene::{SceneId, SceneInventory};
use crate::infra::error::AppError;

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub obs_version: String,
    pub websocket_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputCommandFailureRecovery {
    pub message: String,
    pub fallback_status: OutputStatus,
}

impl OutputCommandFailureRecovery {
    pub fn with_fallback_status(message: String, fallback_status: OutputStatus) -> Self {
        Self {
            message,
            fallback_status: fallback_status_after_failed_output_command(&fallback_status),
        }
    }
}

pub(crate) fn fallback_status_after_failed_output_command(status: &OutputStatus) -> OutputStatus {
    let fallback_state = match (status.active, status.state) {
        (
            true,
            OutputRunState::Starting | OutputRunState::Stopping | OutputRunState::Reconnecting,
        ) => OutputRunState::Active,
        (
            false,
            OutputRunState::Starting | OutputRunState::Stopping | OutputRunState::Reconnecting,
        ) => OutputRunState::Inactive,
        (_, state) => state,
    };

    OutputStatus {
        active: status.active,
        state: fallback_state,
        detail: status.detail.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output_status(active: bool, state: OutputRunState) -> OutputStatus {
        OutputStatus {
            active,
            state,
            detail: None,
        }
    }

    fn assert_non_transitioning(status: &OutputStatus) {
        assert!(
            !status.state.is_transitioning(),
            "expected non-transitioning status, got {:?}",
            status
        );
    }

    #[test]
    fn failed_output_command_fallback_status_maps_all_output_run_states() {
        let cases = [
            (false, OutputRunState::Inactive, OutputRunState::Inactive),
            (false, OutputRunState::Starting, OutputRunState::Inactive),
            (false, OutputRunState::Active, OutputRunState::Active),
            (false, OutputRunState::Stopping, OutputRunState::Inactive),
            (
                false,
                OutputRunState::Reconnecting,
                OutputRunState::Inactive,
            ),
            (false, OutputRunState::Paused, OutputRunState::Paused),
            (false, OutputRunState::Unknown, OutputRunState::Unknown),
            (true, OutputRunState::Inactive, OutputRunState::Inactive),
            (true, OutputRunState::Starting, OutputRunState::Active),
            (true, OutputRunState::Active, OutputRunState::Active),
            (true, OutputRunState::Stopping, OutputRunState::Active),
            (true, OutputRunState::Reconnecting, OutputRunState::Active),
            (true, OutputRunState::Paused, OutputRunState::Paused),
            (true, OutputRunState::Unknown, OutputRunState::Unknown),
        ];

        for (active, state, expected_state) in cases {
            let status = OutputStatus {
                active,
                state,
                detail: Some("detail".to_string()),
            };

            let fallback = fallback_status_after_failed_output_command(&status);

            assert_eq!(fallback.active, active);
            assert_eq!(fallback.state, expected_state);
            assert_eq!(fallback.detail.as_deref(), Some("detail"));
            assert_non_transitioning(&fallback);
        }
    }

    #[test]
    fn failed_output_command_fallback_normalizes_transitioning_states_only() {
        for active in [false, true] {
            let expected_state = if active {
                OutputRunState::Active
            } else {
                OutputRunState::Inactive
            };

            for state in [
                OutputRunState::Starting,
                OutputRunState::Stopping,
                OutputRunState::Reconnecting,
            ] {
                let fallback =
                    fallback_status_after_failed_output_command(&output_status(active, state));

                assert_eq!(fallback.state, expected_state);
                assert_ne!(fallback.state, state);
                assert_non_transitioning(&fallback);
            }
        }
    }

    #[test]
    fn failed_output_command_fallback_preserves_unknown_and_paused_states() {
        for active in [false, true] {
            for state in [OutputRunState::Unknown, OutputRunState::Paused] {
                let fallback =
                    fallback_status_after_failed_output_command(&output_status(active, state));

                assert_eq!(fallback, output_status(active, state));
                assert_non_transitioning(&fallback);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    // Connection
    Connecting,
    Connected(ConnectionInfo),
    Disconnected,

    // Scenes
    SceneInventoryUpdated(SceneInventory),
    CurrentSceneChanged(SceneId),
    ProfilesUpdated(ObsNamedList),
    SceneCollectionsUpdated(ObsNamedList),

    // Audio
    AudioInputsUpdated(Vec<AudioInput>),
    /// Starts a mixer scene audio refresh for the requested target scene.
    MixerAudioInputsLoading {
        scene: SceneId,
    },
    /// Completes a mixer scene audio refresh; state decides whether it is stale.
    MixerAudioInputsUpdated {
        scene: SceneId,
        inputs: Vec<AudioInput>,
    },
    /// Fails a mixer scene audio refresh; state decides whether it is stale.
    MixerAudioInputsFailed {
        scene: SceneId,
        message: String,
    },
    InputMuteChanged {
        input: InputId,
        muted: bool,
    },
    InputVolumeChanged {
        input: InputId,
        volume_mul: f64,
        volume_db: f64,
    },

    // Outputs
    StreamStatusUpdated(OutputStatus),
    RecordStatusUpdated(OutputStatus),
    StreamCommandPending(OutputStatus),
    RecordCommandPending(OutputStatus),
    StreamCommandSucceeded,
    RecordCommandSucceeded,
    StreamCommandFailed(OutputCommandFailureRecovery),
    RecordCommandFailed(OutputCommandFailureRecovery),

    // Graph & diagnostics
    GraphUpdated(SceneGraph),
    DiagnosticsUpdated(Vec<Diagnostic>),

    Error(AppError),
}
