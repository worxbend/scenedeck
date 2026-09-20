use crate::domain::audio::InputId;
use crate::domain::scene::SceneId;

#[derive(Debug, Clone)]
pub enum AppCommand {
    // Connection lifecycle
    Connect,
    Disconnect,
    /// Re-fetch inventory + audio + graph from OBS without reconnecting.
    RefreshData,

    // Scene control
    SwitchPrimaryScene(SceneId),
    SetCurrentProfile(String),
    SetCurrentSceneCollection(String),

    // Audio
    SetInputMute {
        input: InputId,
        muted: bool,
    },
    SetInputVolume {
        input: InputId,
        volume_mul: f64,
    },
    RefreshMixerSceneAudio(SceneId),

    // Outputs
    StartStreaming,
    StopStreaming,
    StartRecording,
    StopRecording,
    /// Poll `GetStats` and stream byte counters for the status bar.
    RefreshStats,
}
