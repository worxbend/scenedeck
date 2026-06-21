//! Runtime application state — the single source of truth held inside an
//! `Rc<RefCell<AppState>>` on the GTK main thread.

use crate::domain::appearance::ThemeMode;
use crate::domain::audio::AudioInput;
use crate::domain::diagnostic::Diagnostic;
use crate::domain::graph::SceneGraph;
use crate::domain::mixer::MixerSelection;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::OutputStatus;
use crate::domain::scene::{SceneId, SceneInventory};
use crate::storage::config::OutputConfig;
use std::time::Instant;

// ── Navigation ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Live,
    Mixer,
    Graph,
    Inventory,
    Doctor,
    Settings,
}

impl Page {
    /// Stable `gtk4::Stack` child name.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Mixer => "mixer",
            Self::Graph => "graph",
            Self::Inventory => "inventory",
            Self::Doctor => "doctor",
            Self::Settings => "settings",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Live => "Live",
            Self::Mixer => "Mixer",
            Self::Graph => "Graph",
            Self::Inventory => "Inventory",
            Self::Doctor => "Doctor",
            Self::Settings => "Settings",
        }
    }

    /// Symbolic icon name for the sidebar row.
    pub const fn icon_name(self) -> &'static str {
        match self {
            Self::Live => "media-record-symbolic",
            Self::Mixer => "audio-volume-high-symbolic",
            Self::Graph => "view-grid-symbolic",
            Self::Inventory => "view-list-symbolic",
            Self::Doctor => "emblem-default-symbolic",
            Self::Settings => "preferences-system-symbolic",
        }
    }
}

// ── OBS connection ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ObsStatus {
    Disconnected,
    Connecting,
    Connected { obs_version: String },
    Error(String),
}

impl ObsStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Disconnected => "Disconnected",
            Self::Connecting => "Connecting…",
            Self::Connected { .. } => "Connected",
            Self::Error(_) => "Error",
        }
    }

    pub fn css_class(&self) -> &str {
        match self {
            Self::Disconnected => "obs-disconnected",
            Self::Connecting => "obs-connecting",
            Self::Connected { .. } => "obs-connected",
            Self::Error(_) => "obs-error",
        }
    }
}

// ── Mixer audio ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixerAudioError {
    pub scene: SceneId,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct MixerAudioSnapshot {
    pub scene: SceneId,
    pub inputs: Vec<AudioInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerAudioRefreshTransition {
    Loading,
    Success,
    Failure,
    StaleSuccess,
    StaleFailure,
}

#[derive(Debug, Default, Clone)]
pub struct MixerAudioRefreshState {
    pub requested_scene: Option<SceneId>,
    pub loaded: Option<MixerAudioSnapshot>,
    pub error: Option<MixerAudioError>,
}

impl MixerAudioRefreshState {
    pub fn loading(&mut self, scene: SceneId) -> MixerAudioRefreshTransition {
        self.requested_scene = Some(scene.clone());
        if self.error.as_ref().map(|err| err.scene.as_str()) == Some(scene.as_str()) {
            self.error = None;
        }
        MixerAudioRefreshTransition::Loading
    }

    pub fn success(
        &mut self,
        scene: SceneId,
        inputs: Vec<AudioInput>,
    ) -> MixerAudioRefreshTransition {
        if self.requested_scene.as_deref() != Some(scene.as_str()) {
            return MixerAudioRefreshTransition::StaleSuccess;
        }

        self.requested_scene = None;
        self.error = None;
        self.loaded = Some(MixerAudioSnapshot { scene, inputs });
        MixerAudioRefreshTransition::Success
    }

    pub fn failure(&mut self, scene: SceneId, message: String) -> MixerAudioRefreshTransition {
        if self.requested_scene.as_deref() != Some(scene.as_str()) {
            return MixerAudioRefreshTransition::StaleFailure;
        }

        self.requested_scene = None;
        self.error = Some(MixerAudioError { scene, message });
        MixerAudioRefreshTransition::Failure
    }

    pub fn clear_pending(&mut self) {
        self.requested_scene = None;
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_page: Page,
    pub theme_mode: ThemeMode,
    pub obs_status: ObsStatus,
    pub scene_inventory: SceneInventory,
    pub scene_graph: SceneGraph,
    pub profiles: ObsNamedList,
    pub scene_collections: ObsNamedList,
    pub stream_status: OutputStatus,
    pub record_status: OutputStatus,
    pub stream_active_since: Option<Instant>,
    pub record_active_since: Option<Instant>,
    pub last_recording_path: Option<String>,
    pub output_confirmations: OutputConfig,
    pub audio_inputs: Vec<AudioInput>,
    pub mixer_audio_scene: Option<String>,
    pub mixer_audio_inputs: Vec<AudioInput>,
    pub mixer_audio_loading_scene: Option<SceneId>,
    pub mixer_audio_error: Option<MixerAudioError>,
    pub mixer_audio_refresh: MixerAudioRefreshState,
    pub mixer: MixerSelection,
    pub diagnostics: Vec<Diagnostic>,
    /// Human-readable config-load notice shown once on the Settings page.
    pub startup_notice: Option<String>,
}

impl AppState {
    pub fn new(
        theme_mode: ThemeMode,
        mixer: MixerSelection,
        output_confirmations: OutputConfig,
        startup_notice: Option<String>,
    ) -> Self {
        Self {
            current_page: Page::Live,
            theme_mode,
            obs_status: ObsStatus::Disconnected,
            scene_inventory: SceneInventory::default(),
            scene_graph: SceneGraph::default(),
            profiles: ObsNamedList::default(),
            scene_collections: ObsNamedList::default(),
            stream_status: OutputStatus::default(),
            record_status: OutputStatus::default(),
            stream_active_since: None,
            record_active_since: None,
            last_recording_path: None,
            output_confirmations,
            audio_inputs: Vec::new(),
            mixer_audio_scene: None,
            mixer_audio_inputs: Vec::new(),
            mixer_audio_loading_scene: None,
            mixer_audio_error: None,
            mixer_audio_refresh: MixerAudioRefreshState::default(),
            mixer,
            diagnostics: Vec::new(),
            startup_notice,
        }
    }

    pub fn set_page(&mut self, page: Page) {
        self.current_page = page;
    }
    pub fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
    }
    pub fn set_obs_status(&mut self, status: ObsStatus) {
        self.obs_status = status;
    }

    pub fn set_mixer_audio_loading(&mut self, scene: SceneId) -> MixerAudioRefreshTransition {
        let transition = self.mixer_audio_refresh.loading(scene);
        self.sync_mixer_audio_fields();
        transition
    }

    pub fn set_mixer_audio_success(
        &mut self,
        scene: SceneId,
        inputs: Vec<AudioInput>,
    ) -> MixerAudioRefreshTransition {
        let transition = self.mixer_audio_refresh.success(scene, inputs);
        self.sync_mixer_audio_fields();
        transition
    }

    pub fn set_mixer_audio_failure(
        &mut self,
        scene: SceneId,
        message: String,
    ) -> MixerAudioRefreshTransition {
        let transition = self.mixer_audio_refresh.failure(scene, message);
        self.sync_mixer_audio_fields();
        transition
    }

    pub fn clear_pending_mixer_audio_refresh(&mut self) {
        self.mixer_audio_refresh.clear_pending();
        self.sync_mixer_audio_fields();
    }

    fn sync_mixer_audio_fields(&mut self) {
        self.mixer_audio_loading_scene = self.mixer_audio_refresh.requested_scene.clone();
        self.mixer_audio_error = self.mixer_audio_refresh.error.clone();
        if let Some(snapshot) = self.mixer_audio_refresh.loaded.as_ref() {
            self.mixer_audio_scene = Some(snapshot.scene.clone());
            self.mixer_audio_inputs = snapshot.inputs.clone();
        } else {
            self.mixer_audio_scene = None;
            self.mixer_audio_inputs.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(id: &str) -> AudioInput {
        AudioInput::new(id.to_string(), false, 1.0, 0.0)
    }

    #[test]
    fn mixer_refresh_loading_tracks_requested_scene_and_preserves_snapshot() {
        let mut state = MixerAudioRefreshState {
            requested_scene: None,
            loaded: Some(MixerAudioSnapshot {
                scene: "Loaded".to_string(),
                inputs: vec![input("Mic")],
            }),
            error: Some(MixerAudioError {
                scene: "Target".to_string(),
                message: "previous failure".to_string(),
            }),
        };

        let transition = state.loading("Target".to_string());

        assert_eq!(transition, MixerAudioRefreshTransition::Loading);
        assert_eq!(state.requested_scene.as_deref(), Some("Target"));
        assert_eq!(
            state
                .loaded
                .as_ref()
                .map(|snapshot| snapshot.scene.as_str()),
            Some("Loaded")
        );
        assert!(state.error.is_none());
    }

    #[test]
    fn mixer_refresh_success_replaces_loaded_snapshot_for_current_request() {
        let mut state = MixerAudioRefreshState::default();
        state.loading("Scene A".to_string());

        let transition = state.success("Scene A".to_string(), vec![input("Mic A")]);

        assert_eq!(transition, MixerAudioRefreshTransition::Success);
        assert_eq!(state.requested_scene, None);
        let loaded = state.loaded.as_ref().expect("loaded mixer snapshot");
        assert_eq!(loaded.scene, "Scene A");
        assert_eq!(loaded.inputs.len(), 1);
        assert_eq!(loaded.inputs[0].id, "Mic A");
        assert!(state.error.is_none());
    }

    #[test]
    fn mixer_refresh_failure_sets_visible_error_for_current_request() {
        let mut state = MixerAudioRefreshState {
            requested_scene: None,
            loaded: Some(MixerAudioSnapshot {
                scene: "Previous".to_string(),
                inputs: vec![input("Previous Mic")],
            }),
            error: None,
        };
        state.loading("Scene A".to_string());

        let transition = state.failure("Scene A".to_string(), "OBS failed".to_string());

        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        assert_eq!(state.requested_scene, None);
        assert_eq!(
            state
                .loaded
                .as_ref()
                .map(|snapshot| snapshot.scene.as_str()),
            Some("Previous")
        );
        assert_eq!(
            state.error,
            Some(MixerAudioError {
                scene: "Scene A".to_string(),
                message: "OBS failed".to_string(),
            })
        );
    }

    #[test]
    fn mixer_refresh_stale_success_does_not_overwrite_current_state() {
        let mut state = MixerAudioRefreshState::default();
        state.loading("Scene A".to_string());
        state.loading("Scene B".to_string());

        let transition = state.success("Scene A".to_string(), vec![input("Stale Mic")]);

        assert_eq!(transition, MixerAudioRefreshTransition::StaleSuccess);
        assert_eq!(state.requested_scene.as_deref(), Some("Scene B"));
        assert!(state.loaded.is_none());
        assert!(state.error.is_none());

        let transition = state.success("Scene B".to_string(), vec![input("Current Mic")]);

        assert_eq!(transition, MixerAudioRefreshTransition::Success);
        let loaded = state.loaded.as_ref().expect("loaded mixer snapshot");
        assert_eq!(loaded.scene, "Scene B");
        assert_eq!(loaded.inputs[0].id, "Current Mic");
    }

    #[test]
    fn mixer_refresh_stale_failure_does_not_overwrite_loaded_snapshot_or_visible_error() {
        let mut state = MixerAudioRefreshState {
            requested_scene: Some("Scene B".to_string()),
            loaded: Some(MixerAudioSnapshot {
                scene: "Loaded".to_string(),
                inputs: vec![input("Loaded Mic")],
            }),
            error: Some(MixerAudioError {
                scene: "Scene B".to_string(),
                message: "current failure".to_string(),
            }),
        };

        let transition = state.failure("Scene A".to_string(), "stale failure".to_string());

        assert_eq!(transition, MixerAudioRefreshTransition::StaleFailure);
        assert_eq!(state.requested_scene.as_deref(), Some("Scene B"));
        assert_eq!(
            state
                .loaded
                .as_ref()
                .map(|snapshot| snapshot.scene.as_str()),
            Some("Loaded")
        );
        assert_eq!(
            state.error,
            Some(MixerAudioError {
                scene: "Scene B".to_string(),
                message: "current failure".to_string(),
            })
        );
    }
}
