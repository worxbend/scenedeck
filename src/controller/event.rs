use crate::controller::state::OutputCommandFailure;
use crate::domain::audio::{AudioInput, InputId};
use crate::domain::diagnostic::Diagnostic;
use crate::domain::graph::SceneGraph;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::OutputStatus;
use crate::domain::scene::{SceneId, SceneInventory};
use crate::infra::error::AppError;

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub obs_version: String,
    pub websocket_version: String,
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
    StreamCommandFailed(OutputCommandFailure),
    RecordCommandFailed(OutputCommandFailure),

    // Graph & diagnostics
    GraphUpdated(SceneGraph),
    DiagnosticsUpdated(Vec<Diagnostic>),

    Error(AppError),
}
