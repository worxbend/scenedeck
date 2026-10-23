use crate::domain::audio::InputId;
use crate::domain::output::OutputKind;
use crate::domain::scene::SceneId;

#[derive(Debug, Clone)]
pub enum AppCommand {
    CaptureThumbnail(crate::controller::thumbnails::ThumbnailRequest),
    CancelThumbnails,
    /// New workspace work is scoped and never replayed after reconnecting.
    ReadWorkspace(crate::controller::workspace::WorkspaceRequest),
    WorkspaceAction {
        request: crate::controller::workspace::WorkspaceRequest,
        action: crate::domain::control::ControlAction,
        expected_text: Option<String>,
    },
    CancelWorkspace,
    RefreshWorkspaceMedia(crate::controller::workspace::WorkspaceRequest),
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
        scope: crate::domain::workflow::ScopeBinding,
        input: InputId,
        muted: bool,
    },
    SetInputVolume {
        scope: crate::domain::workflow::ScopeBinding,
        input: InputId,
        volume_mul: f64,
    },
    RefreshMixerSceneAudio(crate::controller::state::MixerAudioRequest),

    // Outputs
    SetOutput {
        kind: OutputKind,
        active: bool,
    },
    /// Poll `GetStats` and stream byte counters for the status bar.
    RefreshStats,
}
