//! Runtime application state — the single source of truth held inside an
//! `Rc<RefCell<AppState>>` on the GTK main thread.

use crate::controller::event::OutputCommandFailureRecovery;
use crate::domain::appearance::ThemeMode;
use crate::domain::audio::AudioInput;
use crate::domain::diagnostic::Diagnostic;
use crate::domain::graph::SceneGraph;
use crate::domain::mixer::{MixerMode, MixerSelection};
use crate::domain::obs::ObsNamedList;
use crate::domain::output::OutputStatus;
use crate::domain::scene::{SceneId, SceneInventory};
use crate::domain::stats::ObsStats;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::config::OutputConfig;
use i18n_embed_fl::fl;
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

    pub fn title(self) -> String {
        match self {
            Self::Live => fl!(LANGUAGE_LOADER, "page-live"),
            Self::Mixer => fl!(LANGUAGE_LOADER, "page-mixer"),
            Self::Graph => fl!(LANGUAGE_LOADER, "page-graph"),
            Self::Inventory => fl!(LANGUAGE_LOADER, "page-inventory"),
            Self::Doctor => fl!(LANGUAGE_LOADER, "page-doctor"),
            Self::Settings => fl!(LANGUAGE_LOADER, "page-settings"),
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
    pub fn label(&self) -> String {
        match self {
            Self::Disconnected => fl!(LANGUAGE_LOADER, "obs-status-disconnected"),
            Self::Connecting => fl!(LANGUAGE_LOADER, "obs-status-connecting"),
            Self::Connected { .. } => fl!(LANGUAGE_LOADER, "obs-status-connected"),
            Self::Error(_) => fl!(LANGUAGE_LOADER, "obs-status-error"),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerSceneRefreshTargetReason {
    DirectSelectedScene,
    DirectPinnedScene,
    SelectedModeCurrentSceneFallback,
    PinnedModeSelectedSceneFallback,
    PinnedModeCurrentSceneFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixerSceneRefreshTarget<'a> {
    pub scene: &'a str,
    pub reason: MixerSceneRefreshTargetReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerInspectionRenderSourceKind {
    ActiveScene,
    Scene,
    MissingScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerInspectionStatus<'a> {
    LoadingPlaceholderShown,
    ErrorPlaceholderShown(&'a str),
    MissingNoTarget,
    LoadedWithVisibleInputCards,
    LoadedNoAudioSources,
    LoadedNoMatchingAudioSourcesAfterFiltering,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerInspectionInput<'a> {
    pub name: &'a str,
    pub display_name: &'a str,
    pub muted: bool,
    pub volume_mul: f64,
    pub volume_db: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixerInspectionSnapshot<'a> {
    pub mode: MixerMode,
    pub selected_scene: Option<&'a str>,
    pub pinned_scene: Option<&'a str>,
    pub refresh_target: Option<MixerSceneRefreshTarget<'a>>,
    pub render_source_kind: MixerInspectionRenderSourceKind,
    pub scene: Option<&'a str>,
    pub status: MixerInspectionStatus<'a>,
    pub inputs: Vec<MixerInspectionInput<'a>>,
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
    pub last_stream_command_error: Option<String>,
    pub last_record_command_error: Option<String>,
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
    /// Latest OBS `GetStats` snapshot for the status bar. `None` until the
    /// first poll completes after connecting.
    pub obs_stats: Option<ObsStats>,
    /// Rolling stream bitrate derived from consecutive stats polls.
    pub stream_bitrate_kbps: Option<f64>,
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
            last_stream_command_error: None,
            last_record_command_error: None,
            stream_active_since: None,
            record_active_since: None,
            last_recording_path: None,
            output_confirmations,
            audio_inputs: Vec::new(),
            mixer_audio_refresh: MixerAudioRefreshState::default(),
            mixer,
            diagnostics: Vec::new(),
            startup_notice,
            obs_stats: None,
            stream_bitrate_kbps: None,
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

    pub fn set_stream_status(&mut self, status: OutputStatus) {
        self.stream_status = status;
    }

    pub fn set_record_status(&mut self, status: OutputStatus) {
        self.record_status = status;
    }

    pub fn set_stream_command_pending(&mut self, status: OutputStatus) {
        self.last_stream_command_error = None;
        self.stream_status = status;
    }

    pub fn set_record_command_pending(&mut self, status: OutputStatus) {
        self.last_record_command_error = None;
        self.record_status = status;
    }

    pub fn set_stream_command_success(&mut self) {
        self.last_stream_command_error = None;
    }

    pub fn set_record_command_success(&mut self) {
        self.last_record_command_error = None;
    }

    pub fn set_stream_command_failure(&mut self, message: String) {
        self.last_stream_command_error = Some(message);
    }

    pub fn set_record_command_failure(&mut self, message: String) {
        self.last_record_command_error = Some(message);
    }

    pub fn set_stream_command_failure_with_recovery(
        &mut self,
        failure: OutputCommandFailureRecovery,
    ) {
        self.last_stream_command_error = Some(failure.message().to_string());
        self.stream_status = failure.fallback_status().clone();
    }

    pub fn set_record_command_failure_with_recovery(
        &mut self,
        failure: OutputCommandFailureRecovery,
    ) {
        self.last_record_command_error = Some(failure.message().to_string());
        self.record_status = failure.fallback_status().clone();
    }

    pub fn clear_output_command_errors(&mut self) {
        self.last_stream_command_error = None;
        self.last_record_command_error = None;
    }

    pub fn set_obs_stats(&mut self, stats: ObsStats, bitrate_kbps: Option<f64>) {
        self.obs_stats = Some(stats);
        self.stream_bitrate_kbps = if self.stream_status.active {
            bitrate_kbps
        } else {
            None
        };
    }

    pub fn clear_obs_stats(&mut self) {
        self.obs_stats = None;
        self.stream_bitrate_kbps = None;
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
                let Some(scene) = self.mixer_scene_refresh_target() else {
                    return MixerVisibleRenderSource::MissingScene;
                };

                MixerVisibleRenderSource::Scene {
                    scene,
                    status: self.visible_mixer_audio_status(scene),
                }
            }
        }
    }

    pub fn mixer_inspection_snapshot(&self) -> MixerInspectionSnapshot<'_> {
        let refresh_target = self.mixer_scene_refresh_target_details();
        let (render_source_kind, scene, status, inputs) = match self.visible_mixer_render_source() {
            MixerVisibleRenderSource::ActiveScene(inputs) => (
                MixerInspectionRenderSourceKind::ActiveScene,
                self.scene_inventory.current_id.as_deref(),
                mixer_inspection_loaded_status(inputs),
                mixer_inspection_inputs(inputs),
            ),
            MixerVisibleRenderSource::Scene { scene, status } => {
                let (status, inputs) = mixer_inspection_scene_status(status);
                (
                    MixerInspectionRenderSourceKind::Scene,
                    Some(scene),
                    status,
                    inputs,
                )
            }
            MixerVisibleRenderSource::MissingScene => (
                MixerInspectionRenderSourceKind::MissingScene,
                None,
                MixerInspectionStatus::MissingNoTarget,
                Vec::new(),
            ),
        };

        MixerInspectionSnapshot {
            mode: self.mixer.mode,
            selected_scene: self.mixer.selected_scene.as_deref(),
            pinned_scene: self.mixer.pinned_scene.as_deref(),
            refresh_target,
            render_source_kind,
            scene,
            status,
            inputs,
        }
    }

    /// Scene target for OBS scene-specific Mixer refresh requests.
    ///
    /// Active mode renders live active-scene audio and must not dispatch a
    /// scene-specific Mixer refresh, so it intentionally has no target here.
    pub fn mixer_scene_refresh_target(&self) -> Option<&str> {
        self.mixer_scene_refresh_target_details()
            .map(|target| target.scene)
    }

    /// Scene target for OBS scene-specific Mixer refresh requests, annotated
    /// with the selected/pinned/fallback rule that produced it.
    pub fn mixer_scene_refresh_target_details(&self) -> Option<MixerSceneRefreshTarget<'_>> {
        match self.mixer.mode {
            MixerMode::ActiveScene => None,
            MixerMode::SelectedScene => {
                if let Some(scene) = self.mixer.selected_scene.as_deref() {
                    Some(MixerSceneRefreshTarget {
                        scene,
                        reason: MixerSceneRefreshTargetReason::DirectSelectedScene,
                    })
                } else {
                    self.scene_inventory.current_id.as_deref().map(|scene| {
                        MixerSceneRefreshTarget {
                            scene,
                            reason: MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback,
                        }
                    })
                }
            }
            MixerMode::PinnedScene => {
                if let Some(scene) = self.mixer.pinned_scene.as_deref() {
                    Some(MixerSceneRefreshTarget {
                        scene,
                        reason: MixerSceneRefreshTargetReason::DirectPinnedScene,
                    })
                } else if let Some(scene) = self.mixer.selected_scene.as_deref() {
                    Some(MixerSceneRefreshTarget {
                        scene,
                        reason: MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback,
                    })
                } else {
                    self.scene_inventory.current_id.as_deref().map(|scene| {
                        MixerSceneRefreshTarget {
                            scene,
                            reason: MixerSceneRefreshTargetReason::PinnedModeCurrentSceneFallback,
                        }
                    })
                }
            }
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

fn mixer_inspection_scene_status(
    status: MixerVisibleAudioStatus<'_>,
) -> (MixerInspectionStatus<'_>, Vec<MixerInspectionInput<'_>>) {
    match status {
        MixerVisibleAudioStatus::Loading => {
            (MixerInspectionStatus::LoadingPlaceholderShown, Vec::new())
        }
        MixerVisibleAudioStatus::Error(error) => (
            MixerInspectionStatus::ErrorPlaceholderShown(error.message.as_str()),
            Vec::new(),
        ),
        MixerVisibleAudioStatus::Loaded(inputs) => (
            mixer_inspection_loaded_status(inputs),
            mixer_inspection_inputs(inputs),
        ),
        MixerVisibleAudioStatus::Missing => (MixerInspectionStatus::MissingNoTarget, Vec::new()),
    }
}

fn mixer_inspection_loaded_status(inputs: &[AudioInput]) -> MixerInspectionStatus<'static> {
    if inputs.is_empty() {
        MixerInspectionStatus::LoadedNoAudioSources
    } else {
        MixerInspectionStatus::LoadedWithVisibleInputCards
    }
}

fn mixer_inspection_inputs(inputs: &[AudioInput]) -> Vec<MixerInspectionInput<'_>> {
    inputs
        .iter()
        .map(|input| MixerInspectionInput {
            name: input.name.as_str(),
            display_name: input.display_name.as_str(),
            muted: input.muted,
            volume_mul: input.volume_mul,
            volume_db: input.volume_db,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::output::OutputRunState;

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

    fn hidden_loaded_input<'a>(state: &'a AppState, scene: &str, id: &str) -> &'a AudioInput {
        let snapshot = state
            .mixer_audio_refresh
            .loaded
            .as_ref()
            .expect("loaded mixer snapshot");

        assert_eq!(snapshot.scene, scene);
        snapshot
            .inputs
            .iter()
            .find(|input| input.id == id)
            .expect("loaded mixer input")
    }

    fn loud_muted_input(id: &str) -> AudioInput {
        let mut input = AudioInput::new(id.to_string(), true, 0.5, -6.24);
        input.display_name = format!("{id} Display");
        input
    }

    fn assert_snapshot_input(input: &MixerInspectionInput<'_>, name: &str, muted: bool) {
        assert_eq!(input.name, name);
        assert_eq!(input.display_name, format!("{name} Display"));
        assert_eq!(input.muted, muted);
        assert_eq!(input.volume_mul, 0.5);
        assert_eq!(input.volume_db, -6.24);
    }

    fn output_status(active: bool, state: OutputRunState) -> OutputStatus {
        OutputStatus {
            active,
            state,
            detail: None,
        }
    }

    fn command_failure_recovery(
        message: &str,
        fallback_status: OutputStatus,
    ) -> OutputCommandFailureRecovery {
        OutputCommandFailureRecovery::with_failed_command_fallback_status(
            message.to_string(),
            fallback_status,
        )
    }

    fn unchecked_command_failure_recovery(
        message: &str,
        fallback_status: OutputStatus,
    ) -> OutputCommandFailureRecovery {
        OutputCommandFailureRecovery::for_test_unchecked(message.to_string(), fallback_status)
    }

    fn assert_non_transitioning(status: &OutputStatus) {
        assert!(
            !status.state.is_transitioning(),
            "expected non-transitioning status, got {:?}",
            status
        );
    }

    #[test]
    fn stream_command_failure_sets_stream_error_only() {
        let mut state = app_state();

        state.set_stream_command_failure("stream failed".to_string());

        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream failed")
        );
        assert_eq!(state.last_record_command_error, None);
    }

    #[test]
    fn record_command_failure_sets_record_error_only() {
        let mut state = app_state();

        state.set_record_command_failure("record failed".to_string());

        assert_eq!(state.last_stream_command_error, None);
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record failed")
        );
    }

    #[test]
    fn stream_command_start_failure_applies_inactive_recovery_payload() {
        let mut state = app_state();
        state.set_record_command_failure("existing record failure".to_string());
        state.set_stream_command_pending(output_status(false, OutputRunState::Starting));

        state.set_stream_command_failure_with_recovery(command_failure_recovery(
            "stream start failed",
            output_status(false, OutputRunState::Inactive),
        ));

        assert_eq!(
            state.stream_status,
            output_status(false, OutputRunState::Inactive)
        );
        assert_non_transitioning(&state.stream_status);
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream start failed")
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("existing record failure")
        );
    }

    #[test]
    fn stream_command_stop_failure_applies_active_recovery_payload() {
        let mut state = app_state();
        state.set_record_command_failure("existing record failure".to_string());
        state.set_stream_command_pending(output_status(true, OutputRunState::Stopping));

        state.set_stream_command_failure_with_recovery(command_failure_recovery(
            "stream stop failed",
            output_status(true, OutputRunState::Active),
        ));

        assert_eq!(
            state.stream_status,
            output_status(true, OutputRunState::Active)
        );
        assert_non_transitioning(&state.stream_status);
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream stop failed")
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("existing record failure")
        );
    }

    #[test]
    fn record_command_start_failure_applies_inactive_recovery_payload() {
        let mut state = app_state();
        state.set_stream_command_failure("existing stream failure".to_string());
        state.set_record_command_pending(output_status(false, OutputRunState::Starting));

        state.set_record_command_failure_with_recovery(command_failure_recovery(
            "record start failed",
            output_status(false, OutputRunState::Inactive),
        ));

        assert_eq!(
            state.record_status,
            output_status(false, OutputRunState::Inactive)
        );
        assert_non_transitioning(&state.record_status);
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("existing stream failure")
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record start failed")
        );
    }

    #[test]
    fn record_command_stop_failure_applies_active_recovery_payload() {
        let mut state = app_state();
        state.set_stream_command_failure("existing stream failure".to_string());
        state.set_record_command_pending(output_status(true, OutputRunState::Stopping));

        state.set_record_command_failure_with_recovery(command_failure_recovery(
            "record stop failed",
            output_status(true, OutputRunState::Active),
        ));

        assert_eq!(
            state.record_status,
            output_status(true, OutputRunState::Active)
        );
        assert_non_transitioning(&state.record_status);
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("existing stream failure")
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record stop failed")
        );
    }

    #[test]
    fn stream_failure_recovery_applies_exact_payload_status() {
        let mut state = app_state();
        state.set_stream_command_pending(output_status(false, OutputRunState::Starting));

        state.set_stream_command_failure_with_recovery(unchecked_command_failure_recovery(
            "stream failed",
            output_status(false, OutputRunState::Reconnecting),
        ));

        assert_eq!(
            state.stream_status,
            output_status(false, OutputRunState::Reconnecting)
        );
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream failed")
        );
    }

    #[test]
    fn record_failure_recovery_applies_exact_payload_status() {
        let mut state = app_state();
        state.set_record_command_pending(output_status(true, OutputRunState::Stopping));

        state.set_record_command_failure_with_recovery(unchecked_command_failure_recovery(
            "record failed",
            output_status(true, OutputRunState::Reconnecting),
        ));

        assert_eq!(
            state.record_status,
            output_status(true, OutputRunState::Reconnecting)
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record failed")
        );
    }

    #[test]
    fn stream_failure_recovery_applies_payload_instead_of_current_state() {
        let mut state = app_state();
        state.set_stream_command_pending(output_status(false, OutputRunState::Starting));

        state.set_stream_command_failure_with_recovery(command_failure_recovery(
            "stream failed",
            output_status(true, OutputRunState::Active),
        ));

        assert_eq!(
            state.stream_status,
            output_status(true, OutputRunState::Active)
        );
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream failed")
        );
    }

    #[test]
    fn record_failure_recovery_applies_payload_instead_of_current_state() {
        let mut state = app_state();
        state.set_record_command_pending(output_status(true, OutputRunState::Stopping));

        state.set_record_command_failure_with_recovery(command_failure_recovery(
            "record failed",
            output_status(false, OutputRunState::Inactive),
        ));

        assert_eq!(
            state.record_status,
            output_status(false, OutputRunState::Inactive)
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record failed")
        );
    }

    #[test]
    fn separate_status_event_can_reconnect_after_stream_failure_recovery() {
        let mut state = app_state();
        state.set_stream_command_pending(output_status(false, OutputRunState::Starting));
        state.set_stream_command_failure_with_recovery(command_failure_recovery(
            "stream failed",
            output_status(false, OutputRunState::Inactive),
        ));
        assert_non_transitioning(&state.stream_status);

        state.set_stream_status(output_status(false, OutputRunState::Reconnecting));

        assert_eq!(
            state.stream_status,
            output_status(false, OutputRunState::Reconnecting)
        );
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream failed")
        );
    }

    #[test]
    fn separate_status_event_can_reconnect_after_record_failure_recovery() {
        let mut state = app_state();
        state.set_record_command_pending(output_status(false, OutputRunState::Starting));
        state.set_record_command_failure_with_recovery(command_failure_recovery(
            "record failed",
            output_status(false, OutputRunState::Inactive),
        ));
        assert_non_transitioning(&state.record_status);

        state.set_record_status(output_status(false, OutputRunState::Reconnecting));

        assert_eq!(
            state.record_status,
            output_status(false, OutputRunState::Reconnecting)
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record failed")
        );
    }

    #[test]
    fn stream_pending_and_success_clear_only_stream_error() {
        let mut state = app_state();
        state.set_stream_command_failure("old stream failure".to_string());
        state.set_record_command_failure("record failure".to_string());

        state.set_stream_command_pending(output_status(false, OutputRunState::Starting));

        assert_eq!(state.last_stream_command_error, None);
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record failure")
        );
        assert_eq!(state.stream_status.state, OutputRunState::Starting);

        state.set_stream_command_failure("new stream failure".to_string());
        state.set_stream_command_success();

        assert_eq!(state.last_stream_command_error, None);
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record failure")
        );
    }

    #[test]
    fn record_pending_and_success_clear_only_record_error() {
        let mut state = app_state();
        state.set_stream_command_failure("stream failure".to_string());
        state.set_record_command_failure("old record failure".to_string());

        state.set_record_command_pending(output_status(true, OutputRunState::Stopping));

        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream failure")
        );
        assert_eq!(state.last_record_command_error, None);
        assert_eq!(state.record_status.state, OutputRunState::Stopping);

        state.set_record_command_failure("new record failure".to_string());
        state.set_record_command_success();

        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream failure")
        );
        assert_eq!(state.last_record_command_error, None);
    }

    #[test]
    fn output_command_errors_clear_together_on_session_reset() {
        let mut state = app_state();
        state.set_stream_command_failure("stream failure".to_string());
        state.set_record_command_failure("record failure".to_string());

        state.clear_output_command_errors();

        assert_eq!(state.last_stream_command_error, None);
        assert_eq!(state.last_record_command_error, None);
    }

    #[test]
    fn stream_command_failure_sequence_keeps_obs_connected_and_record_error() {
        let mut state = app_state();
        state.current_page = Page::Live;
        state.set_obs_status(ObsStatus::Connected {
            obs_version: "32.1.2".to_string(),
        });
        state.set_record_command_failure("existing record failure".to_string());

        state.set_stream_command_pending(output_status(false, OutputRunState::Starting));
        assert_eq!(state.stream_status.state, OutputRunState::Starting);
        assert_eq!(state.last_stream_command_error, None);

        let failure = command_failure_recovery(
            "stream command failed",
            output_status(false, OutputRunState::Inactive),
        );
        state.set_stream_command_failure_with_recovery(failure);

        assert_eq!(
            state.obs_status,
            ObsStatus::Connected {
                obs_version: "32.1.2".to_string()
            }
        );
        assert_eq!(state.current_page, Page::Live);
        assert!(!state.stream_status.state.is_transitioning());
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("stream command failed")
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("existing record failure")
        );

        state.set_stream_command_pending(output_status(false, OutputRunState::Starting));
        assert_eq!(state.last_stream_command_error, None);
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("existing record failure")
        );

        state.set_stream_status(output_status(true, OutputRunState::Active));
        state.set_stream_command_success();

        assert_eq!(
            state.obs_status,
            ObsStatus::Connected {
                obs_version: "32.1.2".to_string()
            }
        );
        assert_eq!(state.current_page, Page::Live);
        assert!(!state.stream_status.state.is_transitioning());
        assert_eq!(state.last_stream_command_error, None);
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("existing record failure")
        );
    }

    #[test]
    fn record_command_failure_sequence_keeps_obs_connected_and_stream_error() {
        let mut state = app_state();
        state.current_page = Page::Live;
        state.set_obs_status(ObsStatus::Connected {
            obs_version: "32.1.2".to_string(),
        });
        state.set_stream_command_failure("existing stream failure".to_string());

        state.set_record_command_pending(output_status(true, OutputRunState::Stopping));
        assert_eq!(state.record_status.state, OutputRunState::Stopping);
        assert_eq!(state.last_record_command_error, None);

        let failure = command_failure_recovery(
            "record command failed",
            output_status(true, OutputRunState::Active),
        );
        state.set_record_command_failure_with_recovery(failure);

        assert_eq!(
            state.obs_status,
            ObsStatus::Connected {
                obs_version: "32.1.2".to_string()
            }
        );
        assert_eq!(state.current_page, Page::Live);
        assert!(!state.record_status.state.is_transitioning());
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("existing stream failure")
        );
        assert_eq!(
            state.last_record_command_error.as_deref(),
            Some("record command failed")
        );

        state.set_record_command_pending(output_status(true, OutputRunState::Stopping));
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("existing stream failure")
        );
        assert_eq!(state.last_record_command_error, None);

        state.set_record_status(output_status(false, OutputRunState::Inactive));
        state.set_record_command_success();

        assert_eq!(
            state.obs_status,
            ObsStatus::Connected {
                obs_version: "32.1.2".to_string()
            }
        );
        assert_eq!(state.current_page, Page::Live);
        assert!(!state.record_status.state.is_transitioning());
        assert_eq!(
            state.last_stream_command_error.as_deref(),
            Some("existing stream failure")
        );
        assert_eq!(state.last_record_command_error, None);
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
    fn mixer_scene_refresh_target_active_mode_has_no_scene_specific_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), None);
    }

    #[test]
    fn mixer_scene_refresh_target_selected_mode_uses_selected_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Selected"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Selected",
                reason: MixerSceneRefreshTargetReason::DirectSelectedScene,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_selected_mode_falls_back_to_current_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Active"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Active",
                reason: MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_selected_mode_reports_none_without_selected_or_current_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;

        assert_eq!(state.mixer_scene_refresh_target(), None);
        assert_eq!(state.mixer_scene_refresh_target_details(), None);
    }

    #[test]
    fn mixer_scene_refresh_target_pinned_mode_uses_pinned_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Pinned"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Pinned",
                reason: MixerSceneRefreshTargetReason::DirectPinnedScene,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_pinned_mode_falls_back_to_selected_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Selected"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Selected",
                reason: MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_pinned_mode_falls_back_to_current_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());

        assert_eq!(state.mixer_scene_refresh_target(), Some("Active"));
        assert_eq!(
            state.mixer_scene_refresh_target_details(),
            Some(MixerSceneRefreshTarget {
                scene: "Active",
                reason: MixerSceneRefreshTargetReason::PinnedModeCurrentSceneFallback,
            })
        );
    }

    #[test]
    fn mixer_scene_refresh_target_pinned_mode_reports_none_without_any_fallback_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;

        assert_eq!(state.mixer_scene_refresh_target(), None);
        assert_eq!(state.mixer_scene_refresh_target_details(), None);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_active_render_source() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());
        state.audio_inputs = vec![loud_muted_input("Live Mic")];
        state.set_mixer_audio_loading("Selected".to_string());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.mode, MixerMode::ActiveScene);
        assert_eq!(snapshot.selected_scene, Some("Selected"));
        assert_eq!(snapshot.pinned_scene, Some("Pinned"));
        assert_eq!(snapshot.refresh_target, None);
        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::ActiveScene
        );
        assert_eq!(snapshot.scene, Some("Active"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Live Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_selected_direct_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success("Selected".to_string(), vec![loud_muted_input("Scene Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.mode, MixerMode::SelectedScene);
        assert_eq!(
            snapshot.refresh_target,
            Some(MixerSceneRefreshTarget {
                scene: "Selected",
                reason: MixerSceneRefreshTargetReason::DirectSelectedScene,
            })
        );
        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::Scene
        );
        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Scene Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_selected_fallback_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.set_mixer_audio_loading("Active".to_string());
        state.set_mixer_audio_success("Active".to_string(), vec![loud_muted_input("Active Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.selected_scene, None);
        assert_eq!(
            snapshot.refresh_target,
            Some(MixerSceneRefreshTarget {
                scene: "Active",
                reason: MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback,
            })
        );
        assert_eq!(snapshot.scene, Some("Active"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Active Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_pinned_direct_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());
        state.set_mixer_audio_loading("Pinned".to_string());
        state.set_mixer_audio_success("Pinned".to_string(), vec![loud_muted_input("Pinned Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.mode, MixerMode::PinnedScene);
        assert_eq!(snapshot.selected_scene, Some("Selected"));
        assert_eq!(snapshot.pinned_scene, Some("Pinned"));
        assert_eq!(
            snapshot.refresh_target,
            Some(MixerSceneRefreshTarget {
                scene: "Pinned",
                reason: MixerSceneRefreshTargetReason::DirectPinnedScene,
            })
        );
        assert_eq!(snapshot.scene, Some("Pinned"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Pinned Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_pinned_fallback_target() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Active".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success(
            "Selected".to_string(),
            vec![loud_muted_input("Selected Mic")],
        );

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.pinned_scene, None);
        assert_eq!(
            snapshot.refresh_target,
            Some(MixerSceneRefreshTarget {
                scene: "Selected",
                reason: MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback,
            })
        );
        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );
        assert_eq!(snapshot.inputs.len(), 1);
        assert_snapshot_input(&snapshot.inputs[0], "Selected Mic", true);
    }

    #[test]
    fn mixer_inspection_snapshot_reports_loading_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        state.set_mixer_audio_loading("Selected".to_string());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadingPlaceholderShown
        );
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_snapshot_reports_error_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.mixer.pinned_scene = Some("Pinned".to_string());
        state.set_mixer_audio_loading("Pinned".to_string());
        state.set_mixer_audio_failure("Pinned".to_string(), "OBS failed".to_string());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(snapshot.scene, Some("Pinned"));
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::ErrorPlaceholderShown("OBS failed")
        );
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_snapshot_reports_missing_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        state.set_mixer_audio_loading("Other".to_string());
        state.set_mixer_audio_success("Other".to_string(), vec![loud_muted_input("Other Mic")]);

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::Scene
        );
        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(snapshot.status, MixerInspectionStatus::MissingNoTarget);
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_snapshot_reports_active_loaded_empty_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Active".to_string());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::ActiveScene
        );
        assert_eq!(snapshot.scene, Some("Active"));
        assert_eq!(snapshot.status, MixerInspectionStatus::LoadedNoAudioSources);
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_snapshot_reports_scene_loaded_empty_status() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success("Selected".to_string(), Vec::new());

        let snapshot = state.mixer_inspection_snapshot();

        assert_eq!(
            snapshot.render_source_kind,
            MixerInspectionRenderSourceKind::Scene
        );
        assert_eq!(snapshot.scene, Some("Selected"));
        assert_eq!(snapshot.status, MixerInspectionStatus::LoadedNoAudioSources);
        assert!(snapshot.inputs.is_empty());
    }

    #[test]
    fn mixer_inspection_status_can_represent_filtered_loaded_empty_status() {
        assert_ne!(
            MixerInspectionStatus::LoadedNoAudioSources,
            MixerInspectionStatus::LoadedNoMatchingAudioSourcesAfterFiltering
        );
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
    fn mixer_input_mute_update_preserves_hidden_loaded_snapshot_invariant() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_mute("Mic", true));

        let transition = state.set_mixer_audio_loading("Scene A".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Loading);
        assert!(hidden_loaded_input(&state, "Scene A", "Mic").muted);
        assert!(!hidden_loaded_input(&state, "Scene A", "Music").muted);

        let transition =
            state.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        assert!(hidden_loaded_input(&state, "Scene A", "Mic").muted);
        assert!(!hidden_loaded_input(&state, "Scene A", "Music").muted);
    }

    #[test]
    fn mixer_input_volume_update_preserves_hidden_loaded_snapshot_invariant() {
        let mut state = app_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);

        assert!(state.update_mixer_input_volume("Music", 0.42, -7.5));

        let transition = state.set_mixer_audio_loading("Scene A".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Loading);
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Music").volume_mul,
            0.42
        );
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Music").volume_db,
            -7.5
        );
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Mic").volume_mul,
            1.0
        );

        let transition =
            state.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());
        assert_eq!(transition, MixerAudioRefreshTransition::Failure);
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Music").volume_mul,
            0.42
        );
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Music").volume_db,
            -7.5
        );
        assert_eq!(
            hidden_loaded_input(&state, "Scene A", "Mic").volume_mul,
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
