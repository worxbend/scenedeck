use crate::domain::audio::InputId;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneId;

#[derive(Debug, Clone)]
pub enum AppCommand {
    // Connection lifecycle
    Connect,
    Disconnect,
    RefreshAll,
    /// Re-fetch inventory + audio + graph from OBS without reconnecting.
    RefreshData,

    // Scene control
    SwitchPrimaryScene(SceneId),
    SetCurrentProfile(String),
    CreateProfile(String),
    RemoveProfile(String),
    SetCurrentSceneCollection(String),
    CreateSceneCollection(String),

    // Role management
    SetSceneRole {
        scene: SceneId,
        role: SceneRole,
    },

    // Audio
    SetInputMute {
        input: InputId,
        muted: bool,
    },
    ToggleInputMute {
        input: InputId,
    },
    SetInputVolume {
        input: InputId,
        volume_mul: f64,
    },
    RefreshMixerSceneAudio(SceneId),

    // Outputs
    StartStreaming,
    StopStreaming,
    ToggleStreaming,
    SetStreaming(bool),
    StartRecording,
    StopRecording,
    ToggleRecording,
    SetRecording(bool),
    RefreshOutputStatus,

    // Diagnostics
    RunDoctor,
}
