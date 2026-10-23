//! Native OBS operating controls. These values contain no protocol or GTK types.

use i18n_embed_fl::fl;

use crate::infra::i18n::LANGUAGE_LOADER;

/// Controls whose state is independent of the currently inspected scene/source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalToggle {
    StudioMode,
    VirtualCamera,
}

impl ControlAction {
    pub fn global_toggle(&self) -> Option<GlobalToggle> {
        match self {
            Self::SetStudioMode(_) => Some(GlobalToggle::StudioMode),
            Self::SetVirtualCamera(_) => Some(GlobalToggle::VirtualCamera),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ControlSnapshot {
    pub scene: Option<String>,
    pub items: Vec<SourceItem>,
    pub warnings: Vec<String>,
    pub studio: StudioState,
    pub replay_active: Option<bool>,
    pub last_replay_path: Option<String>,
    pub virtual_camera_active: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StudioState {
    pub enabled: Option<bool>,
    pub program: Option<String>,
    pub preview: Option<String>,
    pub transitions: Vec<String>,
    pub transition: Option<String>,
    pub transition_duration_ms: Option<i64>,
    pub transition_fixed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceItem {
    pub owner: String,
    pub item_id: i64,
    pub input: String,
    pub input_uuid: String,
    pub kind: Option<String>,
    pub path: Vec<String>,
    pub enabled: bool,
    pub ancestor_enabled: bool,
    pub is_group: bool,
    pub is_scene: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceDetail {
    pub input: String,
    pub kind: String,
    pub filters: Vec<SourceFilter>,
    pub media: Option<MediaStatus>,
    pub audio: Option<AudioSettings>,
    pub muted: Option<bool>,
    pub volume_db: Option<f64>,
    pub text: Option<String>,
    pub text_from_file: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFilter {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct MediaStatus {
    pub state: MediaState,
    pub duration_ms: Option<i64>,
    pub cursor_ms: Option<i64>,
}
/// OBS media-source playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaState {
    None,
    Playing,
    Opening,
    Buffering,
    Paused,
    Stopped,
    Ended,
    Error,
    Unknown,
}

impl MediaState {
    pub fn label(self) -> String {
        match self {
            Self::None => fl!(LANGUAGE_LOADER, "media-state-none"),
            Self::Playing => fl!(LANGUAGE_LOADER, "media-state-playing"),
            Self::Opening => fl!(LANGUAGE_LOADER, "media-state-opening"),
            Self::Buffering => fl!(LANGUAGE_LOADER, "media-state-buffering"),
            Self::Paused => fl!(LANGUAGE_LOADER, "media-state-paused"),
            Self::Stopped => fl!(LANGUAGE_LOADER, "media-state-stopped"),
            Self::Ended => fl!(LANGUAGE_LOADER, "media-state-ended"),
            Self::Error => fl!(LANGUAGE_LOADER, "media-state-error"),
            Self::Unknown => fl!(LANGUAGE_LOADER, "media-state-unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioSettings {
    pub balance: f32,
    pub sync_offset_ms: i64,
    pub monitor: AudioMonitor,
    pub tracks: [bool; 6],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioMonitor {
    Off,
    MonitorOnly,
    MonitorAndOutput,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MediaAction {
    Play,
    Pause,
    Stop,
    Restart,
    Next,
    Previous,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControlAction {
    SetItemEnabled {
        owner: String,
        item_id: i64,
        enabled: bool,
    },
    /// A click retains the observed mode as a precondition, never a cached target.
    ActivateScene {
        scene: String,
        studio_enabled: bool,
    },
    SetStudioMode(bool),
    SetProgram(String),
    Take,
    SetTransition(String),
    SetTransitionDuration(i64),
    SetRecordingPaused(bool),
    SplitRecording,
    RecordChapter(String),
    SetReplayActive(bool),
    SaveReplay,
    SetVirtualCamera(bool),
    Media {
        input: String,
        action: MediaAction,
    },
    SeekMedia {
        input: String,
        position_ms: i64,
    },
    SetFilterEnabled {
        input: String,
        filter: String,
        enabled: bool,
    },
    SetText {
        input: String,
        text: String,
    },
    SetInputMuted {
        input: String,
        muted: bool,
    },
    SetInputVolumeDb {
        input: String,
        db: f64,
    },
    SetAudioMonitor {
        input: String,
        monitor: AudioMonitor,
    },
    SetAudioSync {
        input: String,
        offset_ms: i64,
    },
    SetAudioBalance {
        input: String,
        balance: f32,
    },
    SetAudioTracks {
        input: String,
        tracks: [bool; 6],
    },
}

/// Only OBS's built-in text sources have a defined text-editing contract.
pub fn is_text_kind(kind: &str) -> bool {
    matches!(
        kind,
        "text_ft2_source"
            | "text_ft2_source_v2"
            | "text_gdiplus"
            | "text_gdiplus_v2"
            | "text_gdiplus_v3"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_allowlist_excludes_browser_and_arbitrary_plugins() {
        assert!(is_text_kind("text_ft2_source"));
        assert!(is_text_kind("text_gdiplus_v2"));
        assert!(!is_text_kind("browser_source"));
        assert!(!is_text_kind("my_text_plugin"));
    }
}
