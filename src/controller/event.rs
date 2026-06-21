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

    pub fn from_current_status(message: String, current_status: &OutputStatus) -> Self {
        Self::with_fallback_status(
            message,
            fallback_status_after_failed_output_command(current_status),
        )
    }
}

pub fn fallback_status_after_failed_output_command(status: &OutputStatus) -> OutputStatus {
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
