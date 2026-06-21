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
    MixerAudioInputsUpdated {
        scene: SceneId,
        inputs: Vec<AudioInput>,
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

    // Graph & diagnostics
    GraphUpdated(SceneGraph),
    DiagnosticsUpdated(Vec<Diagnostic>),

    Error(AppError),
}
