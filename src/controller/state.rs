//! Runtime application state — the single source of truth held inside an
//! `Rc<RefCell<AppState>>` on the GTK main thread.

use crate::domain::appearance::ThemeMode;
use crate::domain::audio::AudioInput;
use crate::domain::diagnostic::Diagnostic;
use crate::domain::graph::SceneGraph;
use crate::domain::mixer::{MixerMode, MixerSelection};
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

#[derive(Debug, Clone, Copy)]
pub enum MixerVisibleAudioStatus<'a> {
    Loading,
    Error(&'a MixerAudioError),
    Loaded(&'a [AudioInput]),
    Missing,
}

#[derive(Debug, Clone, Copy)]
pub enum MixerVisibleRenderSource<'a> {
    ActiveScene(&'a [AudioInput]),
    Scene {
        scene: &'a str,
        status: MixerVisibleAudioStatus<'a>,
    },
    MissingScene,
}

#[derive(Debug, Default, Clone)]
pub struct MixerAudioRefreshState {
    /// Scene-level freshness marker for the currently pending mixer snapshot.
    ///
    /// This deliberately tracks the requested scene, not a per-request token:
    /// when multiple refreshes for the same scene are started, a success for
    /// that scene is still current and must be accepted. Responses for any
    /// other scene are stale and ignored by the reducer.
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
        self.mixer_audio_refresh.loading(scene)
    }

    pub fn set_mixer_audio_success(
        &mut self,
        scene: SceneId,
        inputs: Vec<AudioInput>,
    ) -> MixerAudioRefreshTransition {
        self.mixer_audio_refresh.success(scene, inputs)
    }

    pub fn set_mixer_audio_failure(
        &mut self,
        scene: SceneId,
        message: String,
    ) -> MixerAudioRefreshTransition {
        self.mixer_audio_refresh.failure(scene, message)
    }

    pub fn clear_pending_mixer_audio_refresh(&mut self) {
        self.mixer_audio_refresh.clear_pending();
    }

    pub fn visible_mixer_audio_status(&self, scene: &str) -> MixerVisibleAudioStatus<'_> {
        if self.mixer_audio_refresh.requested_scene.as_deref() == Some(scene) {
            return MixerVisibleAudioStatus::Loading;
        }

        if let Some(error) = self.mixer_audio_refresh.error.as_ref() {
            if error.scene == scene {
                return MixerVisibleAudioStatus::Error(error);
            }
        }

        if let Some(snapshot) = self.mixer_audio_refresh.loaded.as_ref() {
            if snapshot.scene == scene {
                return MixerVisibleAudioStatus::Loaded(&snapshot.inputs);
            }
        }

        MixerVisibleAudioStatus::Missing
    }

    pub fn visible_mixer_render_source(&self) -> MixerVisibleRenderSource<'_> {
        match self.mixer.mode {
            MixerMode::ActiveScene => MixerVisibleRenderSource::ActiveScene(&self.audio_inputs),
            MixerMode::SelectedScene | MixerMode::PinnedScene => {
                let Some(scene) = self.visible_mixer_target_scene() else {
                    return MixerVisibleRenderSource::MissingScene;
                };

                MixerVisibleRenderSource::Scene {
                    scene,
                    status: self.visible_mixer_audio_status(scene),
                }
            }
        }
    }

    fn visible_mixer_target_scene(&self) -> Option<&str> {
        match self.mixer.mode {
            MixerMode::ActiveScene => self.scene_inventory.current_id.as_deref(),
            MixerMode::SelectedScene => self
                .mixer
                .selected_scene
                .as_deref()
                .or(self.scene_inventory.current_id.as_deref()),
            MixerMode::PinnedScene => self
                .mixer
                .pinned_scene
                .as_deref()
                .or(self.mixer.selected_scene.as_deref())
                .or(self.scene_inventory.current_id.as_deref()),
        }
    }

    pub fn update_mixer_input_mute(&mut self, input_id: &str, muted: bool) -> bool {
        let updated = self
            .mixer_audio_refresh
            .loaded
            .as_mut()
            .and_then(|snapshot| {
                snapshot
                    .inputs
                    .iter_mut()
                    .find(|input| input.id == input_id)
            })
            .map(|input| {
                input.muted = muted;
            })
            .is_some();

        updated
    }

    pub fn update_mixer_input_volume(
        &mut self,
        input_id: &str,
        volume_mul: f64,
        volume_db: f64,
    ) -> bool {
        let updated = self
            .mixer_audio_refresh
            .loaded
            .as_mut()
            .and_then(|snapshot| {
                snapshot
                    .inputs
                    .iter_mut()
                    .find(|input| input.id == input_id)
            })
            .map(|input| {
                input.volume_mul = volume_mul;
                input.volume_db = volume_db;
            })
            .is_some();

        updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(id: &str) -> AudioInput {
        AudioInput::new(id.to_string(), false, 1.0, 0.0)
    }

    fn app_state() -> AppState {
        AppState::new(
            ThemeMode::default(),
            MixerSelection::default(),
            OutputConfig::default(),
            None,
        )
    }

    fn visible_loaded_input<'a>(state: &'a AppState, scene: &str, id: &str) -> &'a AudioInput {
        let MixerVisibleAudioStatus::Loaded(inputs) = state.visible_mixer_audio_status(scene)
        else {
            panic!("expected visible mixer inputs for {scene}");
        };

        inputs
            .iter()
            .find(|input| input.id == id)
            .expect("visible mixer input")
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
    fn mixer_refresh_repeated_same_scene_loading_accepts_same_scene_success() {
        let mut state = MixerAudioRefreshState::default();
        state.loading("Scene A".to_string());
        state.loading("Scene A".to_string());

        let transition = state.success("Scene A".to_string(), vec![input("Latest Mic")]);

        assert_eq!(transition, MixerAudioRefreshTransition::Success);
        assert_eq!(state.requested_scene, None);
        let loaded = state.loaded.as_ref().expect("loaded mixer snapshot");
        assert_eq!(loaded.scene, "Scene A");
        assert_eq!(loaded.inputs[0].id, "Latest Mic");
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

    #[test]
    fn visible_mixer_audio_status_reports_loading_for_target_scene() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());

        assert!(matches!(
            state.visible_mixer_audio_status("Scene A"),
            MixerVisibleAudioStatus::Loading
        ));
    }

    #[test]
    fn visible_mixer_audio_status_reports_error_for_target_scene() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());

        let MixerVisibleAudioStatus::Error(error) = state.visible_mixer_audio_status("Scene A")
        else {
            panic!("expected visible mixer error");
        };

        assert_eq!(error.scene, "Scene A");
        assert_eq!(error.message, "OBS failed");
    }

    #[test]
    fn visible_mixer_audio_status_reports_loaded_inputs_for_target_scene() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);

        let MixerVisibleAudioStatus::Loaded(inputs) = state.visible_mixer_audio_status("Scene A")
        else {
            panic!("expected visible mixer inputs");
        };

        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].id, "Mic");
        assert_eq!(inputs[1].id, "Music");
    }

    #[test]
    fn visible_mixer_audio_status_reports_missing_without_target_state() {
        let state = app_state();

        assert!(matches!(
            state.visible_mixer_audio_status("Scene A"),
            MixerVisibleAudioStatus::Missing
        ));
    }

    #[test]
    fn visible_mixer_audio_status_treats_other_scene_state_as_missing() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        assert!(matches!(
            state.visible_mixer_audio_status("Scene B"),
            MixerVisibleAudioStatus::Missing
        ));

        state.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());
        assert!(matches!(
            state.visible_mixer_audio_status("Scene B"),
            MixerVisibleAudioStatus::Missing
        ));

        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic")]);
        assert!(matches!(
            state.visible_mixer_audio_status("Scene B"),
            MixerVisibleAudioStatus::Missing
        ));
    }

    #[test]
    fn visible_mixer_render_source_active_reads_live_audio_inputs() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.audio_inputs = vec![input("Live Mic"), input("Live Music")];
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());

        let MixerVisibleRenderSource::ActiveScene(inputs) = state.visible_mixer_render_source()
        else {
            panic!("expected active mixer render source");
        };

        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].id, "Live Mic");
        assert_eq!(inputs[1].id, "Live Music");
    }

    #[test]
    fn visible_mixer_render_source_selected_uses_selected_scene_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success("Selected".to_string(), vec![input("Selected Mic")]);

        let MixerVisibleRenderSource::Scene { scene, status } = state.visible_mixer_render_source()
        else {
            panic!("expected scene mixer render source");
        };

        assert_eq!(scene, "Selected");
        let MixerVisibleAudioStatus::Loaded(inputs) = status else {
            panic!("expected selected mixer inputs");
        };
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].id, "Selected Mic");
    }

    #[test]
    fn visible_mixer_render_source_pinned_uses_pinned_scene_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());
        state.set_mixer_audio_loading("Pinned".to_string());

        let MixerVisibleRenderSource::Scene { scene, status } = state.visible_mixer_render_source()
        else {
            panic!("expected scene mixer render source");
        };

        assert_eq!(scene, "Pinned");
        assert!(matches!(status, MixerVisibleAudioStatus::Loading));
    }

    #[test]
    fn visible_mixer_render_source_selected_reports_missing_for_other_loaded_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        state.set_mixer_audio_loading("Loaded".to_string());
        state.set_mixer_audio_success("Loaded".to_string(), vec![input("Other Mic")]);

        let MixerVisibleRenderSource::Scene { scene, status } = state.visible_mixer_render_source()
        else {
            panic!("expected scene mixer render source");
        };

        assert_eq!(scene, "Selected");
        assert!(matches!(status, MixerVisibleAudioStatus::Missing));
    }

    #[test]
    fn visible_mixer_render_source_pinned_reports_target_error_over_other_loaded_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.mixer.pinned_scene = Some("Pinned".to_string());
        state.set_mixer_audio_loading("Loaded".to_string());
        state.set_mixer_audio_success("Loaded".to_string(), vec![input("Other Mic")]);
        state.set_mixer_audio_loading("Pinned".to_string());
        state.set_mixer_audio_failure("Pinned".to_string(), "OBS failed".to_string());

        let MixerVisibleRenderSource::Scene { scene, status } = state.visible_mixer_render_source()
        else {
            panic!("expected scene mixer render source");
        };

        assert_eq!(scene, "Pinned");
        let MixerVisibleAudioStatus::Error(error) = status else {
            panic!("expected pinned mixer error");
        };
        assert_eq!(error.scene, "Pinned");
        assert_eq!(error.message, "OBS failed");
    }

    #[test]
    fn visible_mixer_render_source_scene_modes_report_missing_without_target_scene() {
        let mut state = app_state();

        state.mixer.mode = MixerMode::SelectedScene;
        assert!(matches!(
            state.visible_mixer_render_source(),
            MixerVisibleRenderSource::MissingScene
        ));

        state.mixer.mode = MixerMode::PinnedScene;
        assert!(matches!(
            state.visible_mixer_render_source(),
            MixerVisibleRenderSource::MissingScene
        ));
    }

    #[test]
    fn mixer_input_mute_update_changes_visible_loaded_snapshot() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_mute("Mic", true));

        assert!(visible_loaded_input(&state, "Scene A", "Mic").muted);
        assert!(!visible_loaded_input(&state, "Scene A", "Music").muted);
    }

    #[test]
    fn mixer_input_volume_update_changes_visible_loaded_snapshot() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_volume("Music", 0.42, -7.5));

        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Music").volume_mul,
            0.42
        );
        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Music").volume_db,
            -7.5
        );
        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Mic").volume_mul,
            1.0
        );
    }

    #[test]
    fn mixer_input_mute_update_is_hidden_by_same_scene_loading_and_failure_status() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_mute("Mic", true));

        let transition = state.set_mixer_audio_loading("Scene A".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Loading);
        assert!(matches!(
            state.visible_mixer_audio_status("Scene A"),
            MixerVisibleAudioStatus::Loading
        ));

        let transition =
            state.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        let MixerVisibleAudioStatus::Error(error) = state.visible_mixer_audio_status("Scene A")
        else {
            panic!("expected visible mixer error");
        };
        assert_eq!(error.message, "OBS failed");

        state.clear_pending_mixer_audio_refresh();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);
        assert!(!visible_loaded_input(&state, "Scene A", "Mic").muted);
    }

    #[test]
    fn mixer_input_volume_update_is_hidden_by_same_scene_loading_and_failure_status() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_volume("Music", 0.42, -7.5));

        let transition = state.set_mixer_audio_loading("Scene A".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Loading);
        assert!(matches!(
            state.visible_mixer_audio_status("Scene A"),
            MixerVisibleAudioStatus::Loading
        ));

        let transition =
            state.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        let MixerVisibleAudioStatus::Error(error) = state.visible_mixer_audio_status("Scene A")
        else {
            panic!("expected visible mixer error");
        };
        assert_eq!(error.message, "OBS failed");

        state.clear_pending_mixer_audio_refresh();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);
        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Music").volume_mul,
            1.0
        );
        assert_eq!(
            visible_loaded_input(&state, "Scene A", "Music").volume_db,
            0.0
        );
    }
}
