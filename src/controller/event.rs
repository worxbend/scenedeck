use crate::domain::audio::{AudioInput, InputId};
use crate::domain::graph::SceneGraph;
use crate::domain::meter::InputLevels;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::domain::scene::{SceneId, SceneInventory};
use crate::domain::stats::{ObsStats, StreamHealth};
use crate::infra::error::AppError;

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub endpoint: String,
    pub obs_version: String,
    pub websocket_version: String,
    pub available_requests: Vec<String>,
    pub supported_image_formats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputCommandFailureRecovery {
    message: String,
    fallback_status: OutputStatus,
    uncertain: bool,
}

impl OutputCommandFailureRecovery {
    pub fn with_failed_command_fallback_status(
        message: String,
        fallback_status: OutputStatus,
    ) -> Self {
        Self {
            message,
            fallback_status: fallback_status_after_failed_output_command(&fallback_status),
            uncertain: false,
        }
    }

    pub(crate) fn outcome_unknown(message: String) -> Self {
        Self {
            message: format!("{message}; OBS command outcome unknown; command was not retried"),
            fallback_status: OutputStatus::new(false, OutputRunState::Unknown),
            uncertain: true,
        }
    }

    pub(crate) fn uncertain(&self) -> bool {
        self.uncertain
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn fallback_status(&self) -> &OutputStatus {
        &self.fallback_status
    }

    #[cfg(test)]
    pub(crate) fn for_test_unchecked(message: String, fallback_status: OutputStatus) -> Self {
        Self {
            message,
            fallback_status,
            uncertain: false,
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

    #[test]
    fn failed_output_command_recovery_normalizes_transition_fallback_before_storing() {
        for active in [false, true] {
            let expected_state = if active {
                OutputRunState::Active
            } else {
                OutputRunState::Inactive
            };

            for transition_state in [
                OutputRunState::Starting,
                OutputRunState::Stopping,
                OutputRunState::Reconnecting,
            ] {
                let recovery = OutputCommandFailureRecovery::with_failed_command_fallback_status(
                    "stream failed".to_string(),
                    output_status(active, transition_state),
                );

                assert_eq!(recovery.message(), "stream failed");
                assert_eq!(
                    recovery.fallback_status(),
                    &output_status(active, expected_state)
                );
                assert_non_transitioning(recovery.fallback_status());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Audit identity survives retirement/reconnect; only the same generation
    /// may apply its terminal state to current controls.
    SessionRetired(super::lifecycle::SessionRetirement),
    /// Every connection-owned publication is checked before any UI mutation.
    SessionEvent {
        generation: u64,
        collection: Option<u64>,
        event: Box<AppEvent>,
    },
    /// The mutation was accepted; subsequent readback is independent evidence.
    WorkspaceAcknowledged {
        epoch: u64,
        sequence: u64,
        result_id: Option<u64>,
    },
    /// A rejected command or failed readback does not imply a lost connection.
    CommandFailed {
        operation: &'static str,
        error: AppError,
    },
    GraphInvalidated,
    GraphFailed(AppError),
    /// Command completion, not a reusable topology observation.
    GraphRefreshRejected(AppError),
    ThumbnailLoaded {
        request: crate::controller::thumbnails::ThumbnailRequest,
        result: Result<String, String>,
    },
    WorkspaceMediaLoaded {
        epoch: u64,
        sequence: u64,
        input: String,
        result: Result<crate::domain::control::SourceDetail, String>,
    },
    WorkspaceLoaded {
        epoch: u64,
        sequence: u64,
        result_id: Option<u64>,
        acknowledged: bool,
        uncertain: bool,
        result: Result<Box<crate::controller::workspace::WorkspaceObservation>, String>,
        action: bool,
    },
    WorkspaceInvalidated,
    WorkspaceScopeChanging,
    WorkspaceAudioFailed(String),
    ReplaySaved(String),
    // Connection
    Connecting,
    Connected(ConnectionInfo),
    Disconnected,

    // Scenes
    SceneInventoryUpdated(SceneInventory),
    CurrentSceneChanged(SceneId),
    ProfilesUpdated(ObsNamedList),
    SceneCollectionsUpdated(ObsNamedList),

    /// The revision is issued before reads, or when a transport echo arrives.
    /// Graph invalidation/completion share scan ownership here; reconcile before
    /// applying GraphUpdated/GraphFailed so a superseded scan cannot become Ready.
    Observation {
        revision: u64,
        event: Box<AppEvent>,
    },
    ActiveAudioLoading {
        scene: Option<String>,
    },
    ActiveAudioObserved {
        scene: Option<String>,
        result: Result<Vec<AudioInput>, String>,
    },
    // Audio
    #[cfg(test)]
    AudioInputsUpdated(Vec<AudioInput>),
    /// Completes only the exact GTK-issued current-scope Mixer request.
    MixerAudioInputsUpdated {
        request: crate::controller::state::MixerAudioRequest,
        inputs: Vec<AudioInput>,
    },
    MixerAudioInputsFailed {
        request: crate::controller::state::MixerAudioRequest,
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
    /// Volume-meter levels for every input OBS is currently metering.
    ///
    /// High volume: OBS sends this about twenty times a second while connected.
    /// It is dropped rather than queued when the UI falls behind, so it must
    /// never be the only carrier of a state change.
    InputLevelsUpdated(Vec<InputLevels>),

    // Outputs
    OutputReadbackFailed {
        kind: crate::domain::output::OutputKind,
        error: AppError,
    },
    StreamStatusUpdated(OutputStatus),
    RecordStatusUpdated(OutputStatus),
    StreamCommandPending(OutputStatus),
    RecordCommandPending(OutputStatus),
    StreamCommandSucceeded,
    RecordCommandSucceeded,
    StreamCommandFailed(OutputCommandFailureRecovery),
    RecordCommandFailed(OutputCommandFailureRecovery),

    // Graph
    GraphUpdated(SceneGraph),

    // Performance
    /// OBS performance snapshot plus a derived stream bitrate, if a prior
    /// sample exists to diff against, and stream output health when the
    /// `GetStreamStatus` read succeeded alongside it.
    StatsUpdated {
        stats: ObsStats,
        bitrate_kbps: Option<f64>,
        stream: Option<StreamHealth>,
    },

    Error(AppError),
}
