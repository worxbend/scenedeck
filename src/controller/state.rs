//! Runtime application state — the single source of truth held inside an
//! `Rc<RefCell<AppState>>` on the GTK main thread.

use crate::controller::event::OutputCommandFailureRecovery;
use crate::domain::appearance::ThemeMode;
use crate::domain::audio::AudioInput;
use crate::domain::graph::SceneGraph;
use crate::domain::meter::{InputLevelSnapshot, InputLevels};
use crate::domain::mixer::{MixerMode, MixerSelection};
use crate::domain::obs::ObsNamedList;
use crate::domain::output::{OutputKind, OutputStatus};
use crate::domain::scene::{SceneId, SceneInventory};
use crate::domain::stats::{ObsStats, StatsHistory, StreamHealth};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::config::AppConfig;
use crate::storage::registry::SceneRegistry;
use i18n_embed_fl::fl;
use std::time::Instant;

mod mixer;
pub use mixer::*;

// ── Navigation ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Live,
    Sources,
    Mixer,
    Graph,
    Inventory,
    Doctor,
    Settings,
    /// Onboarding guide and feature reference.
    Help,
    /// Live streaming telemetry, kept last in the sidebar.
    Stats,
}

impl Page {
    /// Stable `gtk4::Stack` child name.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Sources => "sources",
            Self::Mixer => "mixer",
            Self::Graph => "graph",
            Self::Inventory => "inventory",
            Self::Doctor => "doctor",
            Self::Settings => "settings",
            Self::Help => "help",
            Self::Stats => "stats",
        }
    }

    pub fn title(self) -> String {
        match self {
            Self::Live => fl!(LANGUAGE_LOADER, "page-live"),
            Self::Sources => fl!(LANGUAGE_LOADER, "sources-title"),
            Self::Mixer => fl!(LANGUAGE_LOADER, "page-mixer"),
            Self::Graph => fl!(LANGUAGE_LOADER, "page-graph"),
            Self::Inventory => fl!(LANGUAGE_LOADER, "page-inventory"),
            Self::Doctor => fl!(LANGUAGE_LOADER, "page-doctor"),
            Self::Settings => fl!(LANGUAGE_LOADER, "page-settings"),
            Self::Help => fl!(LANGUAGE_LOADER, "page-help"),
            Self::Stats => fl!(LANGUAGE_LOADER, "page-stats"),
        }
    }

    /// Symbolic icon name for the sidebar row.
    pub const fn icon_name(self) -> &'static str {
        match self {
            Self::Live => "media-record-symbolic",
            Self::Sources => "document-properties-symbolic",
            Self::Mixer => "audio-volume-high-symbolic",
            Self::Graph => "view-grid-symbolic",
            Self::Inventory => "view-list-symbolic",
            Self::Doctor => "emblem-default-symbolic",
            Self::Settings => "preferences-system-symbolic",
            Self::Help => "help-browser-symbolic",
            Self::Stats => "power-profile-performance-symbolic",
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

    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Disconnected => "obs-disconnected",
            Self::Connecting => "obs-connecting",
            Self::Connected { .. } => "obs-connected",
            Self::Error(_) => "obs-error",
        }
    }
}

/// Completeness of the current collection's topology, independent of edge count.
/// The OBS adapter fails a scan if any scene/group cannot be read completely.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TopologyStatus {
    #[default]
    Missing,
    Loading,
    Ready,
    Failed(String),
}

impl TopologyStatus {
    /// No diagnostic or empty-success presentation is valid until Ready.
    pub fn notice(&self) -> Option<(String, String)> {
        match self {
            Self::Missing => Some((
                fl!(LANGUAGE_LOADER, "topology-missing-title"),
                fl!(LANGUAGE_LOADER, "topology-missing-description"),
            )),
            Self::Loading => Some((
                fl!(LANGUAGE_LOADER, "topology-loading-title"),
                fl!(LANGUAGE_LOADER, "topology-loading-description"),
            )),
            Self::Failed(error) => Some((
                fl!(LANGUAGE_LOADER, "topology-failed-title"),
                fl!(
                    LANGUAGE_LOADER,
                    "topology-failed-description",
                    error = error.clone()
                ),
            )),
            Self::Ready => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ActiveAudioStatus {
    #[default]
    Missing,
    Loading,
    Ready,
    Failed(MixerAudioError),
}

fn previous_scene_for_inventory_update(
    old_current: Option<&str>,
    old_previous: Option<&str>,
    new_current: Option<&str>,
) -> Option<String> {
    match (old_current, new_current) {
        (Some(old), Some(new)) if old != new => Some(old.to_string()),
        _ => old_previous.map(str::to_string),
    }
}

/// Start the output clock on the first active observation; clear it on stop.
pub(crate) fn update_active_since(active: bool, active_since: &mut Option<Instant>) {
    match (active, active_since.is_some()) {
        (true, false) => *active_since = Some(Instant::now()),
        (false, true) => *active_since = None,
        _ => {}
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct WorkspaceState {
    pub dirty: bool,
    pub epoch: u64,
    pub sequence: u64,
    /// Render key: bump via `touch`/`mark_dirty` whenever visible state changes.
    revision: u64,
    pub loading: bool,
    pub pending: bool,
    /// Scene activation is independent of Sources/output readback. The
    /// controller validates the observed Studio mode again before dispatch.
    pub scene_control_observation: Option<(bool, Instant)>,
    pub pending_scene: Option<String>,
    /// Keep target-specific busy state through acknowledgement and readback.
    pub active_action: Option<crate::domain::control::ControlAction>,
    pub queued_toggles: std::collections::VecDeque<crate::domain::control::ControlAction>,
    pub virtual_camera_observation: Option<(Option<bool>, Instant)>,
    pub error: Option<String>,
    pub selected_scene: Option<String>,
    pub selected_input: Option<String>,
    pub snapshot: Option<crate::domain::control::ControlSnapshot>,
    pub detail: Option<crate::domain::control::SourceDetail>,
    pub usage: Vec<crate::domain::control::SourceItem>,
    pub warnings: Vec<crate::domain::control::WorkspaceWarning>,
    pub audio: Vec<AudioInput>,
    pub observed_at: Option<Instant>,
    pub screenshot: Option<String>,
    pub activity: crate::domain::workflow::ActivityLog,
    pub result: String,
    pub output_results: crate::domain::output_result::OutputResults,
    pub recording_stop_result: Option<u64>,
    pub screenshot_scene: Option<String>,
    pub screenshot_observed_at: Option<Instant>,
}

impl WorkspaceState {
    pub fn toggle_value(&self, toggle: crate::domain::control::GlobalToggle) -> Option<bool> {
        use crate::domain::control::GlobalToggle;
        let observed = match toggle {
            GlobalToggle::StudioMode => self
                .scene_control_observation
                .map(|(value, at)| (Some(value), at)),
            GlobalToggle::VirtualCamera => self.virtual_camera_observation,
        };
        if let Some((value, at)) = observed {
            return (at.elapsed().as_secs() <= 30).then_some(value).flatten();
        }
        // Initial snapshots also support callers before the first GTK refresh.
        if !self.can_control() {
            return None;
        }
        match toggle {
            GlobalToggle::StudioMode => self.snapshot.as_ref()?.studio.enabled,
            GlobalToggle::VirtualCamera => self.snapshot.as_ref()?.virtual_camera_active,
        }
    }

    pub fn toggle_pending(&self, toggle: crate::domain::control::GlobalToggle) -> bool {
        self.toggle_target(toggle).is_some()
    }

    pub fn toggle_target(&self, toggle: crate::domain::control::GlobalToggle) -> Option<bool> {
        self.active_action
            .iter()
            .chain(self.queued_toggles.iter())
            .find(|action| action.global_toggle() == Some(toggle))
            .and_then(|action| match action {
                crate::domain::control::ControlAction::SetStudioMode(value)
                | crate::domain::control::ControlAction::SetVirtualCamera(value) => Some(*value),
                _ => None,
            })
    }

    pub fn observe_global_controls(&mut self) {
        let Some(at) = self.observed_at else {
            return;
        };
        if self
            .scene_control_observation
            .is_none_or(|(_, previous)| previous <= at)
        {
            self.observe_scene_controls();
        }
        if self
            .virtual_camera_observation
            .is_none_or(|(_, previous)| previous <= at)
        {
            self.virtual_camera_observation = self
                .snapshot
                .as_ref()
                .map(|s| (s.virtual_camera_active, at));
        }
    }

    pub fn select_input(&mut self, input: Option<String>) {
        // Explicit source inspection remains attached to its scene when Program changes.
        if input.is_some() && self.selected_scene.is_none() {
            self.selected_scene = self
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.scene.clone());
        }
        self.selected_input = input;
        self.detail = None;
        self.usage.clear();
        self.touch();
    }

    pub fn can_control(&self) -> bool {
        // A background read does not invalidate a recent observation. A mutation
        // may supersede that read; epoch/sequence fencing rejects its late reply.
        !self.pending
            && !self.dirty
            && self.error.is_none()
            && self
                .observed_at
                .is_some_and(|time| time.elapsed().as_secs() <= 30)
    }
    pub fn scene_action(&self, scene: &str) -> Option<crate::domain::control::ControlAction> {
        let studio_enabled = if let Some((enabled, observed_at)) = self.scene_control_observation {
            if observed_at.elapsed().as_secs() > 30
                || self.error.is_some()
                || (self.pending && self.pending_scene.is_none())
            {
                return None;
            }
            enabled
        } else {
            if !self.can_control() {
                return None;
            }
            self.snapshot.as_ref()?.studio.enabled?
        };
        Some(crate::domain::control::ControlAction::ActivateScene {
            scene: scene.to_owned(),
            studio_enabled,
        })
    }
    pub fn observe_scene_controls(&mut self) {
        self.scene_control_observation = self
            .snapshot
            .as_ref()
            .and_then(|s| s.studio.enabled)
            .zip(self.observed_at);
    }
    pub fn invalidate_controls(&mut self) {
        self.scene_control_observation = None;
        self.mark_dirty();
    }
    pub fn fail_read(&mut self, error: String) {
        self.error = Some(error);
        self.observed_at = None;
        self.scene_control_observation = None;
        self.virtual_camera_observation = None;
        self.queued_toggles.clear();
    }
    pub fn invalidate(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.sequence = self.sequence.wrapping_add(1);
        self.touch();
        self.snapshot = None;
        self.detail = None;
        self.usage.clear();
        self.audio.clear();
        self.observed_at = None;
        self.screenshot = None;
        self.screenshot_scene = None;
        self.screenshot_observed_at = None;
        self.output_results.invalidate(super::workspace::unix_ms());
        self.loading = false;
        self.pending = false;
        self.pending_scene = None;
        self.active_action = None;
        self.queued_toggles.clear();
        self.virtual_camera_observation = None;
        self.scene_control_observation = None;
        self.recording_stop_result = None;
        self.selected_scene = None;
        self.selected_input = None;
        self.warnings.clear();
        self.error = None;
        self.result.clear();
        self.dirty = false;
    }
    pub fn log(&mut self, origin: crate::domain::workflow::ActivityOrigin, description: String) {
        self.activity.push(crate::domain::workflow::ActivityRecord {
            timestamp_unix_ms: super::workspace::unix_ms(),
            origin,
            description,
        });
        self.touch();
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
    /// Observed values are stale and need readback.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.touch();
    }
    /// A transport echo outranks the last readback until the next read lands.
    pub fn observe_studio_mode(&mut self, enabled: bool, at: Instant) {
        self.scene_control_observation = Some((enabled, at));
        if let Some(snapshot) = &mut self.snapshot {
            snapshot.studio.enabled = Some(enabled);
        }
        self.mark_dirty();
    }
    pub fn observe_virtual_camera(&mut self, active: Option<bool>, at: Instant) {
        self.virtual_camera_observation = Some((active, at));
        if let Some(snapshot) = &mut self.snapshot {
            snapshot.virtual_camera_active = active;
        }
        self.mark_dirty();
    }
    pub fn accepts(&self, epoch: u64, sequence: u64) -> bool {
        self.epoch == epoch && self.sequence == sequence
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub workspace: WorkspaceState,
    pub session_retirements: std::collections::VecDeque<super::lifecycle::SessionRetirement>,
    pub available_requests: Vec<String>,
    pub supported_image_formats: Vec<String>,
    pub connected_endpoint: Option<String>,
    /// Cached local persistence snapshots. GTK reads these values only from
    /// memory; blocking adapters update them through the background-I/O bridge.
    pub config: AppConfig,
    pub registry: SceneRegistry,
    pub obs_password: Option<String>,
    pub current_page: Page,
    pub obs_status: ObsStatus,
    pub scene_inventory: SceneInventory,
    pub scene_graph: SceneGraph,
    pub topology_status: TopologyStatus,
    topology_revision: u64,
    pub profiles: ObsNamedList,
    pub scene_collections: ObsNamedList,
    pub stream_status: OutputStatus,
    pub record_status: OutputStatus,
    pub last_stream_command_error: Option<String>,
    pub last_record_command_error: Option<String>,
    pub stream_active_since: Option<Instant>,
    pub record_active_since: Option<Instant>,
    pub last_recording_path: Option<String>,
    pub audio_inputs: Vec<AudioInput>,
    pub active_audio_status: ActiveAudioStatus,
    active_audio_revision: u64,
    observations: super::observations::Observations,
    /// Latest volume-meter levels, keyed by input. Meter widgets poll this on
    /// the frames they draw rather than being pushed to, because OBS reports
    /// every active input together about twenty times a second.
    pub audio_levels: InputLevelSnapshot,
    pub mixer_audio_refresh: MixerAudioRefreshState,
    pub mixer: MixerSelection,
    /// Human-readable config-load notice shown once on the Settings page.
    pub startup_notice: Option<String>,
    /// Latest OBS `GetStats` snapshot for the status bar. `None` until the
    /// first poll completes after connecting.
    pub obs_stats: Option<ObsStats>,
    /// Rolling stream bitrate derived from consecutive stats polls.
    pub stream_bitrate_kbps: Option<f64>,
    /// Latest `GetStreamStatus` health read, paired with `obs_stats`.
    pub stream_health: Option<StreamHealth>,
    /// Recent samples backing the Stats page charts. Filled by the session's
    /// poll loop regardless of which page is open, so opening Stats shows
    /// history rather than starting from an empty graph.
    pub stats_history: StatsHistory,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        registry: SceneRegistry,
        obs_password: Option<String>,
        startup_notice: Option<String>,
    ) -> Self {
        let mixer = config.mixer.clone();
        Self {
            workspace: WorkspaceState::default(),
            session_retirements: std::collections::VecDeque::new(),
            available_requests: Vec::new(),
            supported_image_formats: Vec::new(),
            connected_endpoint: None,
            config,
            registry,
            obs_password,
            current_page: Page::Live,
            obs_status: ObsStatus::Disconnected,
            scene_inventory: SceneInventory::default(),
            scene_graph: SceneGraph::default(),
            topology_status: TopologyStatus::Missing,
            topology_revision: 0,
            profiles: ObsNamedList::default(),
            scene_collections: ObsNamedList::default(),
            stream_status: OutputStatus::default(),
            record_status: OutputStatus::default(),
            last_stream_command_error: None,
            last_record_command_error: None,
            stream_active_since: None,
            record_active_since: None,
            last_recording_path: None,
            audio_inputs: Vec::new(),
            active_audio_status: ActiveAudioStatus::Missing,
            active_audio_revision: 0,
            observations: super::observations::Observations::default(),
            audio_levels: InputLevelSnapshot::default(),
            mixer_audio_refresh: MixerAudioRefreshState::default(),
            mixer,
            startup_notice,
            obs_stats: None,
            stream_bitrate_kbps: None,
            stream_health: None,
            stats_history: StatsHistory::default(),
        }
    }

    pub fn set_page(&mut self, page: Page) {
        self.current_page = page;
    }
    /// Record what the user asked for before the controller acts: output
    /// commands get an activity entry, and a live recording stop opens a
    /// result ticket so its file path can be reported later.
    pub fn note_command_intent(
        &mut self,
        cmd: &crate::controller::command::AppCommand,
        now_ms: u64,
    ) {
        use crate::controller::command::AppCommand;
        let AppCommand::SetOutput { kind, active } = *cmd else {
            return;
        };
        let requested = match (kind, active) {
            (OutputKind::Stream, true) => fl!(LANGUAGE_LOADER, "window-sidebar-start-stream"),
            (OutputKind::Stream, false) => fl!(LANGUAGE_LOADER, "window-sidebar-stop-stream"),
            (OutputKind::Record, true) => fl!(LANGUAGE_LOADER, "window-sidebar-start-recording"),
            (OutputKind::Record, false) => fl!(LANGUAGE_LOADER, "window-sidebar-stop-recording"),
        };
        self.workspace.log(
            crate::domain::workflow::ActivityOrigin::Requested,
            requested,
        );
        if (kind, active) == (OutputKind::Record, false)
            && self.record_status.active
            && !self.record_status.state.is_transitioning()
            && self.workspace.recording_stop_result.is_none()
        {
            let scope = self.workflow_scope();
            let id = self.workspace.output_results.request(
                crate::domain::output_result::OutputResultKind::RecordingStop,
                scope,
                now_ms,
            );
            self.workspace.recording_stop_result = Some(id);
        }
    }

    /// Apply one output event. Returns its kind, or `None` for any other event.
    pub fn apply_output_event(
        &mut self,
        event: &crate::controller::event::AppEvent,
        now_ms: u64,
    ) -> Option<crate::domain::output::OutputKind> {
        use crate::controller::event::AppEvent;
        use crate::domain::output::OutputKind;
        use crate::domain::output_result::OutputResultKind;
        use crate::domain::workflow::ActivityOrigin;

        if let AppEvent::OutputStatusUpdated(kind, status) = event {
            let kind = *kind;
            let previous = match kind {
                OutputKind::Stream => &self.stream_status,
                OutputKind::Record => &self.record_status,
            };
            if previous != status {
                if kind == OutputKind::Record && !status.active {
                    if let Some(path) = status.detail.as_ref().filter(|path| !path.is_empty()) {
                        let scope = self.workflow_scope();
                        self.workspace.output_results.observe(
                            OutputResultKind::RecordingStop,
                            scope,
                            Some(path.clone()),
                            now_ms,
                        );
                    }
                }
                self.workspace.log(
                    ActivityOrigin::Observed,
                    format!("{}: {}", kind.label(), status.state.label()),
                );
                self.workspace.mark_dirty();
            }
        }

        let kind = match event {
            AppEvent::OutputStatusUpdated(OutputKind::Stream, status) => {
                update_active_since(status.active, &mut self.stream_active_since);
                self.set_stream_status(status.clone());
                OutputKind::Stream
            }
            AppEvent::OutputStatusUpdated(OutputKind::Record, status) => {
                update_active_since(status.active, &mut self.record_active_since);
                // OBS sends an empty path when there is no new file, not "no recording".
                if let Some(path) = status.detail.as_ref().filter(|path| !path.is_empty()) {
                    self.last_recording_path = Some(path.clone());
                }
                self.set_record_status(status.clone());
                OutputKind::Record
            }
            AppEvent::OutputCommandPending(OutputKind::Stream, status) => {
                update_active_since(status.active, &mut self.stream_active_since);
                self.set_stream_command_pending(status.clone());
                OutputKind::Stream
            }
            AppEvent::OutputCommandPending(OutputKind::Record, status) => {
                update_active_since(status.active, &mut self.record_active_since);
                self.set_record_command_pending(status.clone());
                OutputKind::Record
            }
            AppEvent::OutputCommandSucceeded(OutputKind::Stream) => {
                self.set_stream_command_success();
                OutputKind::Stream
            }
            AppEvent::OutputCommandSucceeded(OutputKind::Record) => {
                if let Some(id) = self.workspace.recording_stop_result.take() {
                    self.workspace.output_results.acknowledge(id, now_ms);
                }
                self.workspace.touch();
                self.set_record_command_success();
                OutputKind::Record
            }
            AppEvent::OutputCommandFailed(OutputKind::Stream, failure) => {
                self.workspace
                    .log(ActivityOrigin::Failed, failure.message().to_owned());
                self.set_stream_command_failure_with_recovery(failure.clone());
                OutputKind::Stream
            }
            AppEvent::OutputCommandFailed(OutputKind::Record, failure) => {
                if let Some(id) = self.workspace.recording_stop_result.take() {
                    if failure.uncertain() {
                        self.workspace.output_results.invalidate(now_ms);
                    } else {
                        self.workspace.output_results.fail(
                            id,
                            failure.message().to_string(),
                            now_ms,
                        );
                    }
                }
                self.workspace
                    .log(ActivityOrigin::Failed, failure.message().to_string());
                self.set_record_command_failure_with_recovery(failure.clone());
                OutputKind::Record
            }
            _ => return None,
        };
        Some(kind)
    }

    /// A new session starts with unknown outputs until readback arrives.
    pub fn apply_connected(&mut self, info: crate::controller::event::ConnectionInfo) -> ObsStatus {
        use crate::domain::output::OutputRunState;
        self.available_requests = info.available_requests;
        self.supported_image_formats = info.supported_image_formats;
        self.connected_endpoint = Some(info.endpoint);
        self.workspace.mark_dirty();
        self.obs_status = ObsStatus::Connected {
            obs_version: info.obs_version,
        };
        self.stream_status = OutputStatus::new(false, OutputRunState::Unknown);
        self.record_status = OutputStatus::new(false, OutputRunState::Unknown);
        self.obs_status.clone()
    }

    /// Keep Previous across same-scene readbacks; a Program change also
    /// invalidates the active-scene audio list.
    pub fn apply_scene_inventory(&mut self, mut inventory: SceneInventory) {
        inventory.previous_id = previous_scene_for_inventory_update(
            self.scene_inventory.current_id.as_deref(),
            self.scene_inventory.previous_id.as_deref(),
            inventory.current_id.as_deref(),
        );
        if self.scene_inventory.current_id != inventory.current_id {
            self.invalidate_active_audio();
        }
        self.scene_inventory = inventory;
    }

    pub fn workflow_scope(&self) -> crate::domain::workflow::ScopeBinding {
        crate::domain::workflow::ScopeBinding {
            endpoint: self
                .connected_endpoint
                .clone()
                .unwrap_or_else(|| format!("{}:{}", self.config.obs.host, self.config.obs.port)),
            collection: self.scene_collections.current.clone().unwrap_or_default(),
        }
    }
    /// The colour scheme the user has chosen.
    ///
    /// This reads straight out of the cached config rather than from a
    /// separate field. There used to be both, seeded from the config at
    /// startup and then updated alongside it on every change — two copies of
    /// one setting, correct only for as long as every writer remembered to
    /// update both.
    pub const fn theme_mode(&self) -> ThemeMode {
        self.config.appearance.mode
    }

    pub fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.config.appearance.mode = mode;
    }
    pub fn set_obs_status(&mut self, status: ObsStatus) {
        self.obs_status = status;
    }

    pub fn set_topology_loading(&mut self) {
        self.scene_graph = SceneGraph::default();
        self.topology_status = TopologyStatus::Loading;
    }

    pub fn set_topology_ready(&mut self, graph: SceneGraph) {
        self.scene_graph = graph;
        self.topology_status = TopologyStatus::Ready;
    }

    pub fn set_topology_failed(&mut self, message: String) {
        self.scene_graph = SceneGraph::default();
        self.topology_status = TopologyStatus::Failed(message);
    }

    /// Retire observations whose scene/input names belong to one collection.
    /// Local preferences and scope-bound audit history are not observations of
    /// the new collection; keep them separate from the caches cleared here.
    pub fn invalidate_collection_runtime(&mut self) {
        self.workspace.invalidate();
        self.scene_inventory = SceneInventory::default();
        self.scene_graph = SceneGraph::default();
        self.topology_status = TopologyStatus::Missing;
        self.topology_revision = 0;
        self.scene_collections.current = None;
        self.invalidate_active_audio();
        self.observations = super::observations::Observations::default();
        self.active_audio_revision = 0;
        self.clear_audio_levels();
        self.mixer_audio_refresh.invalidate();
        self.mixer.locked_inputs.clear();
    }

    /// Forget all observations from the retired connection, but retain local
    /// preferences and explicitly scope-bound activity/output evidence.
    pub fn reset_obs_session(&mut self, status: ObsStatus) {
        self.invalidate_collection_runtime();
        self.workspace.log(
            crate::domain::workflow::ActivityOrigin::Connection,
            status.label(),
        );
        self.available_requests.clear();
        self.supported_image_formats.clear();
        self.connected_endpoint = None;
        self.profiles = ObsNamedList::default();
        self.scene_collections = ObsNamedList::default();
        self.set_obs_status(status);
        self.set_stream_status(OutputStatus::default());
        self.set_record_status(OutputStatus::default());
        self.stream_active_since = None;
        self.record_active_since = None;
        self.last_recording_path = None;
        self.clear_output_command_errors();
        self.clear_obs_stats();
    }

    pub fn accepts_session_publication(
        &self,
        generation: u64,
        event: &super::event::AppEvent,
    ) -> bool {
        matches!(event, super::event::AppEvent::GraphRefreshRejected(_))
            || !self
                .session_retirements
                .iter()
                .any(|terminal| terminal.generation == generation)
    }

    /// Called after the connection reset, or audit-only for a replaced session.
    /// No old-generation evidence is allowed to mutate replacement controls.
    pub fn apply_terminal_evidence(
        &mut self,
        terminal: super::lifecycle::SessionRetirement,
        current: bool,
    ) {
        if current {
            let unknown = OutputStatus::new(false, crate::domain::output::OutputRunState::Unknown);
            self.set_stream_status(unknown.clone());
            self.set_record_status(unknown);
            for operation in &terminal.operations {
                match operation.kind {
                    super::lifecycle::OperationKind::Stream => {
                        self.last_stream_command_error = operation.failure.clone()
                    }
                    super::lifecycle::OperationKind::Record => {
                        self.last_record_command_error = operation.failure.clone()
                    }
                    _ => {}
                }
            }
        }
        let operations: Vec<_> = terminal.operations.iter().map(|operation| format!(
            "#{} {:?} endpoint={} collection={} dispatched={} acknowledged={} unknown={} failure={:?}",
            operation.id, operation.kind, operation.endpoint, operation.collection,
            operation.dispatched, operation.acknowledged, operation.uncertain, operation.failure,
        )).collect();
        let audit = format!(
            "Session {}: {} (omitted sessions: {}; {})",
            terminal.generation,
            terminal.reason,
            terminal.omitted_sessions,
            operations.join("; ")
        );
        self.workspace
            .log(crate::domain::workflow::ActivityOrigin::Connection, audit);
        self.session_retirements.push_back(terminal);
        while self.session_retirements.len() > 32 {
            self.session_retirements.pop_front();
        }
    }

    /// Store the newest volume-meter readings, replacing the previous set.
    pub fn set_audio_levels(&mut self, levels: Vec<InputLevels>) {
        self.audio_levels.update(levels);
    }

    /// Forget every metered level, so meters fall silent instead of freezing.
    pub fn clear_audio_levels(&mut self) {
        self.audio_levels.clear();
    }

    /// This is the production GTK observation reducer, also used by wire tests.
    pub fn reconcile_observation(
        &mut self,
        revision: u64,
        event: super::event::AppEvent,
    ) -> Option<super::event::AppEvent> {
        use super::event::AppEvent;
        match &event {
            AppEvent::GraphInvalidated => {
                if revision < self.topology_revision {
                    return None;
                }
                self.topology_revision = revision;
            }
            AppEvent::GraphUpdated(_) | AppEvent::GraphFailed(_)
                if revision != self.topology_revision =>
            {
                return None;
            }
            AppEvent::CurrentSceneChanged(_) => {
                self.active_audio_revision = self.active_audio_revision.max(revision);
            }
            AppEvent::ActiveAudioLoading { scene } => {
                if *scene != self.scene_inventory.current_id
                    || revision < self.active_audio_revision
                {
                    return None;
                }
                self.active_audio_revision = revision;
            }
            AppEvent::ActiveAudioObserved { scene, .. }
                if *scene != self.scene_inventory.current_id
                    || revision != self.active_audio_revision =>
            {
                return None;
            }
            _ => {}
        }
        self.observations.reconcile(revision, event)
    }

    pub fn set_current_program(&mut self, scene: String) {
        if self.scene_inventory.current_id.as_ref() != Some(&scene) {
            self.invalidate_active_audio();
            self.active_audio_revision = self
                .active_audio_revision
                .checked_add(1)
                .expect("active audio revision exhausted");
        }
        self.scene_inventory.set_current_scene(scene);
    }

    pub fn invalidate_active_audio(&mut self) {
        self.audio_inputs.clear();
        self.active_audio_status = ActiveAudioStatus::Loading;
    }

    pub fn apply_active_audio(
        &mut self,
        scene: Option<String>,
        result: Result<Vec<AudioInput>, String>,
    ) {
        if scene != self.scene_inventory.current_id {
            return;
        }
        self.audio_inputs.clear();
        match result {
            Ok(inputs) => {
                self.audio_inputs = inputs;
                self.active_audio_status = ActiveAudioStatus::Ready;
            }
            Err(message) => {
                self.active_audio_status = ActiveAudioStatus::Failed(MixerAudioError {
                    scene: scene.unwrap_or_default(),
                    message,
                })
            }
        }
    }

    pub fn active_audio_notice(&self) -> Option<String> {
        match &self.active_audio_status {
            ActiveAudioStatus::Ready => None,
            ActiveAudioStatus::Failed(error) => Some(format!(
                "{}: {}",
                fl!(LANGUAGE_LOADER, "mixer-error-title"),
                error.message
            )),
            _ => Some(fl!(LANGUAGE_LOADER, "mixer-loading-title")),
        }
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

    #[cfg(test)]
    pub fn set_stream_command_failure(&mut self, message: String) {
        self.last_stream_command_error = Some(message);
    }

    #[cfg(test)]
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

    pub fn set_obs_stats(
        &mut self,
        stats: ObsStats,
        bitrate_kbps: Option<f64>,
        stream: Option<StreamHealth>,
    ) {
        self.obs_stats = Some(stats);
        self.stream_bitrate_kbps = if self.stream_status.active {
            bitrate_kbps
        } else {
            None
        };
        self.stream_health = stream;
        self.stats_history
            .push(stats, self.stream_bitrate_kbps, stream);
    }

    pub fn clear_obs_stats(&mut self) {
        self.obs_stats = None;
        self.stream_bitrate_kbps = None;
        self.stream_health = None;
        self.stats_history.clear();
    }

    /// Record a mute change OBS reported for `input_id`.
    ///
    /// The same input can be held in two places: the Live page's flat list,
    /// and the mixer's snapshot of one scene. Both have to move together or
    /// the two pages start disagreeing about whether a source is muted, so
    /// both are updated here rather than at the call site.
    pub fn apply_input_mute(&mut self, input_id: &str, muted: bool) {
        if let Some(input) = self.audio_input_mut(input_id) {
            input.muted = muted;
        }
        self.update_mixer_input_mute(input_id, muted);
        for input in &mut self.workspace.audio {
            if input.id == input_id {
                input.muted = muted;
            }
        }
        if let Some(detail) = self
            .workspace
            .detail
            .as_mut()
            .filter(|d| d.input == input_id)
        {
            detail.muted = Some(muted);
        }
    }

    /// Record a volume change OBS reported for `input_id`, in both places the
    /// input may be held. See [`AppState::apply_input_mute`].
    pub fn apply_input_volume(&mut self, input_id: &str, volume_mul: f64, volume_db: f64) {
        if let Some(input) = self.audio_input_mut(input_id) {
            input.volume_mul = volume_mul;
            input.volume_db = volume_db;
        }
        self.update_mixer_input_volume(input_id, volume_mul, volume_db);
        for input in &mut self.workspace.audio {
            if input.id == input_id {
                input.volume_mul = volume_mul;
                input.volume_db = volume_db;
            }
        }
        if let Some(detail) = self
            .workspace
            .detail
            .as_mut()
            .filter(|d| d.input == input_id)
        {
            detail.volume_db = Some(volume_db);
        }
    }

    fn audio_input_mut(&mut self, input_id: &str) -> Option<&mut AudioInput> {
        self.audio_inputs
            .iter_mut()
            .find(|input| input.id == input_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::output::OutputRunState;
    use crate::domain::stats::StatsMetric;

    pub(super) fn input(id: &str) -> AudioInput {
        AudioInput::new(id.to_string(), false, 1.0, 0.0)
    }

    pub(super) fn app_state() -> AppState {
        AppState::new(AppConfig::default(), SceneRegistry::default(), None, None)
    }

    #[test]
    fn output_commands_log_intent_and_only_a_settled_live_recording_opens_a_stop_ticket() {
        use crate::controller::command::AppCommand;
        let mut state = app_state();
        for cmd in [
            AppCommand::SetOutput {
                kind: OutputKind::Stream,
                active: true,
            },
            AppCommand::SetOutput {
                kind: OutputKind::Stream,
                active: false,
            },
            AppCommand::SetOutput {
                kind: OutputKind::Record,
                active: true,
            },
        ] {
            state.note_command_intent(&cmd, 1);
        }
        state.note_command_intent(&AppCommand::Disconnect, 1);
        assert_eq!(
            state.workspace.activity.entries.len(),
            3,
            "non-output commands are not logged"
        );

        state.note_command_intent(
            &AppCommand::SetOutput {
                kind: OutputKind::Record,
                active: false,
            },
            1,
        );
        assert_eq!(
            state.workspace.recording_stop_result, None,
            "recording was not active"
        );

        state.record_status = OutputStatus::new(true, OutputRunState::Stopping);
        state.note_command_intent(
            &AppCommand::SetOutput {
                kind: OutputKind::Record,
                active: false,
            },
            2,
        );
        assert_eq!(
            state.workspace.recording_stop_result, None,
            "already transitioning"
        );

        state.record_status = OutputStatus::new(true, OutputRunState::Active);
        state.note_command_intent(
            &AppCommand::SetOutput {
                kind: OutputKind::Record,
                active: false,
            },
            3,
        );
        let ticket = state
            .workspace
            .recording_stop_result
            .expect("ticket opened");
        state.note_command_intent(
            &AppCommand::SetOutput {
                kind: OutputKind::Record,
                active: false,
            },
            4,
        );
        assert_eq!(
            state.workspace.recording_stop_result,
            Some(ticket),
            "one ticket per stop"
        );
        assert_eq!(state.workspace.activity.entries.len(), 7);
    }

    #[test]
    fn output_events_track_clocks_paths_and_the_recording_stop_ticket() {
        use crate::controller::command::AppCommand;
        use crate::controller::event::{AppEvent, OutputCommandFailureRecovery};
        use crate::domain::output::OutputKind;
        use crate::domain::output_result::OutputResultStatus;
        let mut state = app_state();
        let active = OutputStatus::new(true, OutputRunState::Active);

        let event = AppEvent::OutputStatusUpdated(OutputKind::Stream, active.clone());
        assert_eq!(
            state.apply_output_event(&event, 1),
            Some(OutputKind::Stream)
        );
        assert!(state.stream_active_since.is_some());
        assert_eq!(
            state.workspace.activity.entries.len(),
            1,
            "changes are logged"
        );
        assert!(state.workspace.dirty);
        state.apply_output_event(&event, 2);
        assert_eq!(state.workspace.activity.entries.len(), 1, "repeats are not");
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(OutputKind::Stream, OutputStatus::inactive()),
            3,
        );
        assert!(state.stream_active_since.is_none());

        // A settled stop opens a ticket; success acknowledges it.
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(OutputKind::Record, active.clone()),
            4,
        );
        state.note_command_intent(
            &AppCommand::SetOutput {
                kind: OutputKind::Record,
                active: false,
            },
            5,
        );
        assert!(state.workspace.recording_stop_result.is_some());
        state.apply_output_event(&AppEvent::OutputCommandSucceeded(OutputKind::Record), 6);
        assert_eq!(state.workspace.recording_stop_result, None);

        let stopped = OutputStatus {
            detail: Some("/tmp/take.mkv".into()),
            ..OutputStatus::inactive()
        };
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(OutputKind::Record, stopped),
            7,
        );
        assert_eq!(state.last_recording_path.as_deref(), Some("/tmp/take.mkv"));
        assert!(state.record_active_since.is_none());
        let empty = OutputStatus {
            detail: Some(String::new()),
            ..OutputStatus::new(true, OutputRunState::Active)
        };
        state.apply_output_event(&AppEvent::OutputStatusUpdated(OutputKind::Record, empty), 8);
        assert_eq!(state.last_recording_path.as_deref(), Some("/tmp/take.mkv"));

        // A rejected stop fails its ticket instead.
        state.note_command_intent(
            &AppCommand::SetOutput {
                kind: OutputKind::Record,
                active: false,
            },
            9,
        );
        let ticket = state.workspace.recording_stop_result.expect("ticket");
        let failure = OutputCommandFailureRecovery::with_failed_command_fallback_status(
            "refused".into(),
            OutputStatus::inactive(),
        );
        state.apply_output_event(
            &AppEvent::OutputCommandFailed(OutputKind::Record, failure),
            10,
        );
        assert_eq!(state.workspace.recording_stop_result, None);
        assert!(state
            .workspace
            .output_results
            .entries()
            .any(|entry| entry.id == ticket && entry.status == OutputResultStatus::Failed));

        assert_eq!(state.apply_output_event(&AppEvent::Disconnected, 11), None);
    }

    #[test]
    fn inventory_refresh_tracks_previous_scene_only_when_current_changes() {
        assert_eq!(
            previous_scene_for_inventory_update(Some("A"), None, Some("B")),
            Some("A".to_string())
        );
        assert_eq!(
            previous_scene_for_inventory_update(Some("A"), Some("Prev"), Some("A")),
            Some("Prev".to_string())
        );
        assert_eq!(
            previous_scene_for_inventory_update(None, Some("Prev"), Some("A")),
            Some("Prev".to_string())
        );
    }

    #[test]
    fn connecting_resets_outputs_to_unknown_and_records_the_session() {
        use crate::domain::output::OutputRunState;
        let mut state = app_state();
        state.stream_status = OutputStatus::new(true, OutputRunState::Active);
        let status = state.apply_connected(crate::controller::event::ConnectionInfo {
            endpoint: "ws://obs:4455".into(),
            obs_version: "32.2.2".into(),
            websocket_version: "5.6".into(),
            available_requests: vec!["GetVersion".into()],
            supported_image_formats: vec!["png".into()],
        });
        assert_eq!(status, state.obs_status);
        assert!(matches!(status, ObsStatus::Connected { obs_version } if obs_version == "32.2.2"));
        assert_eq!(state.stream_status.state, OutputRunState::Unknown);
        assert_eq!(state.record_status.state, OutputRunState::Unknown);
        assert_eq!(state.connected_endpoint.as_deref(), Some("ws://obs:4455"));
        assert_eq!(state.available_requests, ["GetVersion"]);
        assert!(state.workspace.dirty);
    }

    #[test]
    fn program_change_moves_previous_and_invalidates_scene_audio() {
        let mut state = app_state();
        let inventory = |current: &str| SceneInventory {
            current_id: Some(current.into()),
            ..SceneInventory::default()
        };
        state.apply_scene_inventory(inventory("A"));
        state.audio_inputs = vec![input("Mic")];
        state.apply_scene_inventory(inventory("A"));
        assert_eq!(state.audio_inputs.len(), 1, "same scene keeps audio");
        state.apply_scene_inventory(inventory("B"));
        assert_eq!(state.scene_inventory.previous_id.as_deref(), Some("A"));
        assert!(state.audio_inputs.is_empty());
        assert!(matches!(
            state.active_audio_status,
            ActiveAudioStatus::Loading
        ));
    }

    #[test]
    fn studio_and_virtual_camera_echoes_patch_the_snapshot_and_mark_dirty() {
        let mut workspace = WorkspaceState {
            snapshot: Some(Default::default()),
            ..Default::default()
        };
        let at = Instant::now();
        workspace.observe_studio_mode(true, at);
        assert_eq!(workspace.scene_control_observation, Some((true, at)));
        assert_eq!(
            workspace.snapshot.as_ref().unwrap().studio.enabled,
            Some(true)
        );
        workspace.dirty = false;
        workspace.observe_virtual_camera(Some(false), at);
        assert_eq!(
            workspace.virtual_camera_observation,
            Some((Some(false), at))
        );
        assert_eq!(
            workspace.snapshot.as_ref().unwrap().virtual_camera_active,
            Some(false)
        );
        assert!(workspace.dirty);
    }

    #[test]
    fn every_visible_workspace_mutation_advances_the_revision() {
        let mut workspace = WorkspaceState::default();
        workspace.log(
            crate::domain::workflow::ActivityOrigin::Failed,
            "stream failed".into(),
        );
        assert_eq!(workspace.revision(), 1, "log entries are rendered");

        workspace.mark_dirty();
        assert!(workspace.dirty);
        assert_eq!(workspace.revision(), 2);

        workspace.dirty = false;
        workspace.invalidate_controls();
        assert!(workspace.dirty);
        assert_eq!(workspace.revision(), 3);
    }

    #[test]
    fn topology_missing_is_distinct_from_a_successful_empty_observation() {
        let mut state = app_state();
        assert_eq!(state.topology_status, TopologyStatus::Missing);
        assert!(state.scene_graph.edges.is_empty());
        assert!(state.topology_status.notice().is_some());

        state.set_topology_ready(SceneGraph::default());
        assert_eq!(state.topology_status, TopologyStatus::Ready);
        assert!(state.scene_graph.edges.is_empty());
        assert!(state.topology_status.notice().is_none());
    }

    #[test]
    fn topology_failure_after_ready_retires_old_graph_evidence() {
        let mut state = app_state();
        state.set_topology_ready(SceneGraph {
            edges: std::collections::HashMap::from([("Main".into(), vec!["Child".into()])]),
        });
        state.set_topology_failed("Group could not be read".into());

        assert!(state.scene_graph.edges.is_empty());
        assert_eq!(
            state.topology_status,
            TopologyStatus::Failed("Group could not be read".into())
        );
        let (title, description) = state.topology_status.notice().unwrap();
        assert_eq!(title, "Scene topology could not be read");
        assert!(description.contains("Group could not be read"));
    }

    #[test]
    fn topology_rerun_is_loading_until_a_complete_success() {
        let mut state = app_state();
        state.set_topology_ready(SceneGraph::default());
        for _ in 0..2 {
            state.set_topology_loading();
            assert_eq!(state.topology_status, TopologyStatus::Loading);
            assert!(state.topology_status.notice().is_some());
            state.set_topology_failed("Incomplete scan".into());
            assert!(state.topology_status.notice().is_some());
        }
        state.set_topology_loading();
        state.set_topology_ready(SceneGraph::default());
        assert_eq!(state.topology_status, TopologyStatus::Ready);
        assert!(state.topology_status.notice().is_none());
    }

    #[test]
    fn topology_session_and_collection_resets_clear_every_observation_status() {
        for retire_session in [false, true] {
            for status in [
                TopologyStatus::Missing,
                TopologyStatus::Loading,
                TopologyStatus::Ready,
                TopologyStatus::Failed("old failure".into()),
            ] {
                let mut state = app_state();
                state.topology_status = status;
                state
                    .scene_graph
                    .edges
                    .insert("Main".into(), vec!["Child".into()]);
                if retire_session {
                    state.reset_obs_session(ObsStatus::Disconnected);
                } else {
                    state.invalidate_collection_runtime();
                }
                assert_eq!(state.topology_status, TopologyStatus::Missing);
                assert!(state.scene_graph.edges.is_empty());
            }
        }
    }

    pub(super) fn visible_loaded_input<'a>(
        state: &'a AppState,
        scene: &str,
        id: &str,
    ) -> &'a AudioInput {
        let MixerVisibleAudioStatus::Loaded(inputs) = state.visible_mixer_audio_status(scene)
        else {
            panic!("expected visible mixer inputs for {scene}");
        };

        inputs
            .iter()
            .find(|input| input.id == id)
            .expect("visible mixer input")
    }

    pub(super) fn hidden_loaded_input<'a>(
        state: &'a AppState,
        scene: &str,
        id: &str,
    ) -> &'a AudioInput {
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

    pub(super) fn loud_muted_input(id: &str) -> AudioInput {
        let mut input = AudioInput::new(id.to_string(), true, 0.5, -6.24);
        input.display_name = format!("{id} Display");
        input
    }

    pub(super) fn assert_snapshot_input(input: &MixerInspectionInput<'_>, name: &str, muted: bool) {
        assert_eq!(input.name, name);
        assert_eq!(input.display_name, format!("{name} Display"));
        assert_eq!(input.muted, muted);
        assert_eq!(input.volume_mul, 0.5);
        assert_eq!(input.volume_db, -6.24);
    }

    pub(super) fn output_status(active: bool, state: OutputRunState) -> OutputStatus {
        OutputStatus {
            active,
            state,
            detail: None,
        }
    }

    pub(super) fn command_failure_recovery(
        message: &str,
        fallback_status: OutputStatus,
    ) -> OutputCommandFailureRecovery {
        OutputCommandFailureRecovery::with_failed_command_fallback_status(
            message.to_string(),
            fallback_status,
        )
    }

    pub(super) fn unchecked_command_failure_recovery(
        message: &str,
        fallback_status: OutputStatus,
    ) -> OutputCommandFailureRecovery {
        OutputCommandFailureRecovery::for_test_unchecked(message.to_string(), fallback_status)
    }

    pub(super) fn assert_non_transitioning(status: &OutputStatus) {
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

    pub(super) fn complete_mixer_request(
        state: &mut MixerAudioRefreshState,
        request: MixerAudioRequest,
        succeeds: bool,
    ) -> MixerAudioRefreshTransition {
        if succeeds {
            state.success(request, vec![input("Newest Mic")])
        } else {
            state.failure(request, "newest failure".into())
        }
    }

    #[test]
    fn repeated_same_scene_requests_reject_old_success_and_failure_without_ending_loading() {
        let mut state = MixerAudioRefreshState::default();
        let first = state.loading("A".into());
        let latest = state.loading("A".into());
        assert!(first.id < latest.id);
        assert!(state.success(first.clone(), vec![input("Old")]).is_stale());
        assert!(state.failure(first, "old failure".into()).is_stale());
        assert_eq!(state.requested.as_ref(), Some(&latest));
        assert_eq!(
            state.success(latest, vec![input("New")]),
            MixerAudioRefreshTransition::Success
        );
        assert_eq!(state.loaded.as_ref().unwrap().inputs[0].id, "New");
    }

    #[test]
    fn applying_an_input_change_updates_both_places_the_input_is_held() {
        let mut state = app_state();
        state.audio_inputs = vec![input("Mic"), input("Music")];
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request.clone(), vec![input("Mic"), input("Music")]);

        state.apply_input_mute("Mic", true);
        state.apply_input_volume("Mic", 0.5, -6.0);

        // The Live page's flat list...
        let live = state
            .audio_inputs
            .iter()
            .find(|candidate| candidate.id == "Mic")
            .expect("live input");
        assert!(live.muted);
        assert_eq!(live.volume_mul, 0.5);
        assert_eq!(live.volume_db, -6.0);

        // ...and the mixer's snapshot of the scene must agree.
        let mixed = visible_loaded_input(&state, "Scene A", "Mic");
        assert!(mixed.muted);
        assert_eq!(mixed.volume_mul, 0.5);
        assert_eq!(mixed.volume_db, -6.0);

        // An input the change did not name is untouched in both.
        assert!(!visible_loaded_input(&state, "Scene A", "Music").muted);
        assert!(!state
            .audio_inputs
            .iter()
            .any(|candidate| candidate.id == "Music" && candidate.muted));
    }

    pub(super) fn obs_stats(active_fps: f64) -> ObsStats {
        ObsStats {
            disk_space_mb: None,
            configured_fps: None,
            cpu_usage_percent: 10.0,
            memory_usage_mb: 400.0,
            active_fps,
            average_frame_render_time_ms: 3.0,
            render_skipped_frames: 0,
            render_total_frames: 100,
            output_skipped_frames: 0,
            output_total_frames: 100,
        }
    }

    pub(super) fn stream_health() -> StreamHealth {
        StreamHealth {
            active: true,
            reconnecting: false,
            congestion: 0.1,
            skipped_frames: 2,
            total_frames: 200,
            bytes: 2_048,
        }
    }

    #[test]
    fn stats_updates_accumulate_history_for_the_charts() {
        let mut state = app_state();

        state.set_obs_stats(obs_stats(60.0), None, Some(stream_health()));
        state.set_obs_stats(obs_stats(59.0), None, Some(stream_health()));

        assert_eq!(state.stats_history.len(), 2);
        assert_eq!(
            state.stats_history.series(StatsMetric::Fps),
            vec![60.0, 59.0]
        );
        assert_eq!(state.stream_health, Some(stream_health()));
    }

    #[test]
    fn bitrate_is_dropped_from_state_and_history_while_not_streaming() {
        let mut state = app_state();

        state.set_obs_stats(obs_stats(60.0), Some(6_000.0), Some(stream_health()));

        assert_eq!(state.stream_bitrate_kbps, None);
        assert_eq!(
            state
                .stats_history
                .latest()
                .and_then(|sample| sample.bitrate_kbps),
            None
        );
    }

    #[test]
    fn bitrate_is_recorded_once_the_stream_output_is_active() {
        let mut state = app_state();
        state.set_stream_status(OutputStatus::active());

        state.set_obs_stats(obs_stats(60.0), Some(6_000.0), Some(stream_health()));

        assert_eq!(state.stream_bitrate_kbps, Some(6_000.0));
        assert_eq!(
            state
                .stats_history
                .latest()
                .and_then(|sample| sample.bitrate_kbps),
            Some(6_000.0)
        );
    }

    #[test]
    fn clearing_stats_empties_the_history_and_stream_health() {
        let mut state = app_state();
        state.set_obs_stats(obs_stats(60.0), None, Some(stream_health()));

        state.clear_obs_stats();

        assert!(state.stats_history.is_empty());
        assert_eq!(state.stream_health, None);
        assert_eq!(state.obs_stats, None);
    }

    pub(super) fn seed_collection_observations(state: &mut AppState) -> MixerAudioRequest {
        state
            .scene_inventory
            .scenes
            .push(crate::domain::scene::Scene { id: "A".into() });
        state.scene_inventory.current_id = Some("A".into());
        state.scene_inventory.previous_id = Some("Previous".into());
        state.set_topology_ready(SceneGraph {
            edges: std::collections::HashMap::from([("A".into(), vec!["Nested".into()])]),
        });
        state.audio_inputs = vec![input("Mic")];
        state.set_audio_levels(vec![InputLevels::from_mul("Mic".into(), &[[0.5; 3]])]);
        state.mixer.locked_inputs.insert("Mic".into());
        let loaded = state.set_mixer_audio_loading("A".into());
        state.set_mixer_audio_success(loaded, vec![input("Old Mic")]);
        let failed = state.set_mixer_audio_loading("A".into());
        state.set_mixer_audio_failure(failed, "old error".into());
        state.workspace = WorkspaceState {
            dirty: true,
            epoch: 7,
            sequence: 12,
            revision: 20,
            loading: true,
            pending: true,
            error: Some("old workspace error".into()),
            selected_scene: Some("A".into()),
            selected_input: Some("Mic".into()),
            snapshot: Some(crate::domain::control::ControlSnapshot::default()),
            detail: Some(crate::domain::control::SourceDetail::default()),
            usage: vec![crate::domain::control::SourceItem {
                owner: "A".into(),
                item_id: 1,
                input: "Mic".into(),
                input_uuid: "fixture".into(),
                kind: Some("audio".into()),
                path: vec!["A".into()],
                enabled: true,
                ancestor_enabled: true,
                is_group: false,
                is_scene: false,
            }],
            warnings: vec![crate::domain::control::WorkspaceWarning::InventoryTruncated],
            audio: vec![input("Mic")],
            observed_at: Some(Instant::now()),
            screenshot: Some("old image".into()),
            screenshot_scene: Some("A".into()),
            screenshot_observed_at: Some(Instant::now()),
            result: "old result".into(),
            recording_stop_result: Some(1),
            ..Default::default()
        };
        let scope = state.workflow_scope();
        state.workspace.output_results.request(
            crate::domain::output_result::OutputResultKind::RecordingStop,
            scope.clone(),
            1,
        );
        state.workspace.output_results.observe(
            crate::domain::output_result::OutputResultKind::ReplaySave,
            scope,
            Some("old-replay.mkv".into()),
            2,
        );
        state.workspace.log(
            crate::domain::workflow::ActivityOrigin::Observed,
            "old observation".into(),
        );
        state.set_mixer_audio_loading("B".into())
    }

    pub(super) fn seed_session_observations(state: &mut AppState) {
        state.obs_status = ObsStatus::Connected {
            obs_version: "fixture".into(),
        };
        state.connected_endpoint = Some("fixture:4455".into());
        state.available_requests = vec!["GetStats".into()];
        state.supported_image_formats = vec!["png".into()];
        state.profiles = ObsNamedList {
            items: vec!["Profile".into()],
            current: Some("Profile".into()),
        };
        state.scene_collections = ObsNamedList {
            items: vec!["Collection".into()],
            current: Some("Collection".into()),
        };
        state.set_stream_status(output_status(true, OutputRunState::Active));
        state.set_record_status(output_status(true, OutputRunState::Active));
        state.stream_active_since = Some(Instant::now());
        state.record_active_since = Some(Instant::now());
        state.last_recording_path = Some("old-recording.mkv".into());
        state.set_stream_command_failure("old stream failure".into());
        state.set_record_command_failure("old record failure".into());
        state.set_obs_stats(obs_stats(60.0), Some(6000.0), Some(stream_health()));
    }

    pub(super) fn assert_collection_observations_cleared(state: &AppState, revision: u64) {
        assert!(state.scene_inventory.scenes.is_empty());
        assert_eq!(state.scene_inventory.current_id, None);
        assert_eq!(state.scene_inventory.previous_id, None);
        assert!(state.scene_graph.edges.is_empty());
        assert_eq!(state.topology_status, TopologyStatus::Missing);
        assert!(state.audio_inputs.is_empty() && state.audio_levels.is_empty());
        assert!(state.mixer_audio_refresh.loaded.is_none());
        assert!(state.mixer_audio_refresh.error.is_none());
        assert_eq!(state.mixer_audio_refresh.requested_scene(), None);
        assert!(state.mixer.locked_inputs.is_empty());
        assert_eq!(state.scene_collections.current, None);
        assert!(matches!(
            state.visible_mixer_audio_status("A"),
            MixerVisibleAudioStatus::Missing
        ));
        let workspace = &state.workspace;
        assert_eq!(
            (workspace.epoch, workspace.sequence, workspace.revision),
            (8, 13, revision)
        );
        assert!(!workspace.loading && !workspace.pending && !workspace.dirty);
        assert!(workspace.snapshot.is_none() && workspace.detail.is_none());
        assert!(workspace.usage.is_empty() && workspace.audio.is_empty());
        assert!(workspace.selected_scene.is_none() && workspace.selected_input.is_none());
        assert!(workspace.observed_at.is_none() && workspace.screenshot.is_none());
        assert!(workspace.screenshot_scene.is_none() && workspace.screenshot_observed_at.is_none());
        assert!(workspace.warnings.is_empty() && workspace.error.is_none());
        assert!(workspace.result.is_empty() && workspace.recording_stop_result.is_none());
        let history: Vec<_> = workspace.output_results.entries().collect();
        assert_eq!(history.len(), 2);
        assert_eq!(
            history[0].status,
            crate::domain::output_result::OutputResultStatus::Unconfirmed
        );
        assert_eq!(
            history[1].status,
            crate::domain::output_result::OutputResultStatus::Observed
        );
        assert_eq!(history[1].path.as_deref(), Some("old-replay.mkv"));
        assert_eq!(workspace.activity.entries[0].description, "old observation");
    }

    #[test]
    fn resetting_the_session_forgets_all_runtime_caches_for_every_retirement_status() {
        for status in [
            ObsStatus::Disconnected,
            ObsStatus::Connecting,
            ObsStatus::Error("fixture".into()),
        ] {
            let mut state = app_state();
            seed_session_observations(&mut state);
            let old = seed_collection_observations(&mut state);
            assert!(
                state.mixer_audio_refresh.loaded.is_some()
                    && state.mixer_audio_refresh.error.is_some()
            );

            state.reset_obs_session(status.clone());

            // Invalidation and the retirement notice each log a render.
            assert_collection_observations_cleared(&state, 23);
            assert_eq!(state.obs_status, status);
            assert!(
                state.available_requests.is_empty() && state.supported_image_formats.is_empty()
            );
            assert!(state.connected_endpoint.is_none());
            assert!(state.profiles.items.is_empty() && state.profiles.current.is_none());
            assert!(state.scene_collections.items.is_empty());
            assert_eq!(state.stream_status, OutputStatus::default());
            assert_eq!(state.record_status, OutputStatus::default());
            assert!(state.stream_active_since.is_none() && state.record_active_since.is_none());
            assert!(
                state.last_stream_command_error.is_none()
                    && state.last_record_command_error.is_none()
            );
            assert!(state.last_recording_path.is_none());
            assert!(state.obs_stats.is_none() && state.stream_health.is_none());
            assert!(state.stream_bitrate_kbps.is_none() && state.stats_history.is_empty());
            let new = state.set_mixer_audio_loading("B".into());
            assert!(new.id > old.id && new.scope_epoch > old.scope_epoch);
            assert!(state
                .set_mixer_audio_success(old.clone(), vec![input("Old")])
                .is_stale());
            assert!(state
                .set_mixer_audio_failure(old, "old session".into())
                .is_stale());
            assert!(matches!(
                state.visible_mixer_audio_status("B"),
                MixerVisibleAudioStatus::Loading
            ));
            assert_eq!(
                state.set_mixer_audio_success(new, vec![input("New")]),
                MixerAudioRefreshTransition::Success
            );
        }
    }

    #[test]
    fn collection_invalidation_clears_observations_but_preserves_session_state() {
        let mut state = app_state();
        seed_session_observations(&mut state);
        let old = seed_collection_observations(&mut state);
        let before = state.clone();

        state.invalidate_collection_runtime();

        assert_collection_observations_cleared(&state, 22);
        assert_eq!(state.obs_status, before.obs_status);
        assert_eq!(state.connected_endpoint, before.connected_endpoint);
        assert_eq!(state.available_requests, before.available_requests);
        assert_eq!(
            state.supported_image_formats,
            before.supported_image_formats
        );
        assert_eq!(state.profiles.items, before.profiles.items);
        assert_eq!(state.profiles.current, before.profiles.current);
        assert_eq!(
            state.scene_collections.items,
            before.scene_collections.items
        );
        assert_eq!(state.stream_status, before.stream_status);
        assert_eq!(state.record_status, before.record_status);
        assert_eq!(state.stream_active_since, before.stream_active_since);
        assert_eq!(state.record_active_since, before.record_active_since);
        assert_eq!(state.last_recording_path, before.last_recording_path);
        assert_eq!(
            state.last_stream_command_error,
            before.last_stream_command_error
        );
        assert_eq!(
            state.last_record_command_error,
            before.last_record_command_error
        );
        assert_eq!(state.obs_stats, before.obs_stats);
        assert_eq!(state.stream_health, before.stream_health);
        assert_eq!(state.stream_bitrate_kbps, before.stream_bitrate_kbps);
        assert_eq!(state.stats_history.len(), before.stats_history.len());
        let new = state.set_mixer_audio_loading("B".into());
        assert!(new.id > old.id && new.scope_epoch > old.scope_epoch);
        assert!(state
            .set_mixer_audio_success(old, vec![input("Old collection")])
            .is_stale());
        assert!(matches!(
            state.visible_mixer_audio_status("B"),
            MixerVisibleAudioStatus::Loading
        ));
        assert_eq!(
            state.set_mixer_audio_success(new, vec![input("New collection")]),
            MixerAudioRefreshTransition::Success
        );
    }

    #[test]
    fn session_and_collection_invalidation_preserve_local_preferences() {
        for retire_session in [false, true] {
            let mut state = app_state();
            state.config.obs.host = "configured-host".into();
            state.config.appearance.mode = ThemeMode::Dark;
            state.registry.scene_order = vec!["Preferred".into()];
            state.obs_password = Some("in-memory-fixture".into());
            state.startup_notice = Some("local notice".into());
            state.current_page = Page::Mixer;
            state.mixer.mode = MixerMode::PinnedScene;
            state.mixer.selected_scene = Some("Preferred".into());
            state.mixer.pinned_scene = Some("Pinned".into());
            state.mixer.search = "microphone".into();
            state.mixer.grouping = crate::domain::mixer::MixerGrouping::ScenePath;
            let before = state.clone();
            seed_session_observations(&mut state);
            seed_collection_observations(&mut state);

            if retire_session {
                state.reset_obs_session(ObsStatus::Disconnected);
            } else {
                state.invalidate_collection_runtime();
            }

            assert_eq!(
                format!("{:?}", state.config),
                format!("{:?}", before.config)
            );
            assert_eq!(
                format!("{:?}", state.registry),
                format!("{:?}", before.registry)
            );
            assert_eq!(state.obs_password, before.obs_password);
            assert_eq!(state.startup_notice, before.startup_notice);
            assert_eq!(state.current_page, before.current_page);
            assert_eq!(state.mixer, before.mixer);
        }
    }
    #[test]
    fn workspace_generation_rejects_old_and_out_of_order_results() {
        let mut workspace = super::WorkspaceState {
            epoch: 7,
            sequence: 12,
            loading: true,
            pending: true,
            selected_scene: Some("Old scene".into()),
            selected_input: Some("Old input".into()),
            snapshot: Some(crate::domain::control::ControlSnapshot::default()),
            observed_at: Some(std::time::Instant::now()),
            screenshot: Some("old image".into()),
            ..Default::default()
        };
        assert!(workspace.accepts(7, 12));
        assert!(!workspace.accepts(7, 11));
        assert!(!workspace.accepts(6, 12));
        workspace.invalidate();
        assert!(!workspace.accepts(7, 12));
        assert!(workspace.accepts(8, 13));
        assert!(!workspace.loading);
        assert!(!workspace.pending);
        assert!(workspace.snapshot.is_none());
        assert!(workspace.observed_at.is_none());
        assert!(workspace.screenshot.is_none());
        assert!(workspace.selected_scene.is_none());
        assert!(workspace.selected_input.is_none());
    }

    pub(super) fn controllable_workspace(studio: Option<bool>) -> super::WorkspaceState {
        super::WorkspaceState {
            snapshot: Some(crate::domain::control::ControlSnapshot {
                scene: Some("Inspected scene".into()),
                studio: crate::domain::control::StudioState {
                    enabled: studio,
                    ..Default::default()
                },
                ..Default::default()
            }),
            observed_at: Some(Instant::now()),
            ..Default::default()
        }
    }

    #[test]
    fn source_inspection_retains_its_scene_when_program_changes() {
        let mut workspace = controllable_workspace(Some(false));
        workspace.select_input(Some("Caption".into()));
        assert_eq!(workspace.selected_scene.as_deref(), Some("Inspected scene"));
        workspace.snapshot.as_mut().unwrap().studio.program = Some("New Program".into());
        workspace.select_input(Some("Camera".into()));
        assert_eq!(workspace.selected_scene.as_deref(), Some("Inspected scene"));
        assert_eq!(workspace.selected_input.as_deref(), Some("Camera"));
        workspace.invalidate();
        assert!(workspace.selected_scene.is_none());
        assert!(workspace.selected_input.is_none());
    }

    #[test]
    fn workspace_failed_read_disables_recent_cached_controls() {
        let mut workspace = controllable_workspace(Some(false));
        assert!(workspace.can_control());
        assert!(workspace.scene_action("Target").is_some());
        workspace.fail_read("Target read failed".into());
        assert!(
            workspace.snapshot.is_some(),
            "last-known display data may be retained"
        );
        assert!(!workspace.can_control());
        assert!(workspace.scene_action("Target").is_none());
        assert!(workspace.observed_at.is_none());
        assert_eq!(workspace.error.as_deref(), Some("Target read failed"));
        workspace.error = None;
        assert!(
            !workspace.can_control(),
            "clearing an error is not a fresh observation"
        );
    }

    #[test]
    fn global_toggle_availability_is_independent_and_retires_with_its_scope() {
        use crate::domain::control::{ControlAction, GlobalToggle};
        let mut workspace = controllable_workspace(Some(false));
        workspace.snapshot.as_mut().unwrap().virtual_camera_active = Some(false);
        workspace.observe_global_controls();
        workspace.pending = true;
        workspace.loading = true;
        workspace.dirty = true;
        workspace.observed_at = None;
        workspace.active_action = Some(ControlAction::SetText {
            input: "Caption".into(),
            text: "Edit".into(),
        });
        for toggle in [GlobalToggle::StudioMode, GlobalToggle::VirtualCamera] {
            assert_eq!(workspace.toggle_value(toggle), Some(false));
            assert!(!workspace.toggle_pending(toggle));
        }
        workspace
            .queued_toggles
            .push_back(ControlAction::SetVirtualCamera(true));
        assert!(workspace.toggle_pending(GlobalToggle::VirtualCamera));
        assert!(!workspace.toggle_pending(GlobalToggle::StudioMode));
        workspace.virtual_camera_observation = Some((
            Some(false),
            Instant::now() - std::time::Duration::from_secs(31),
        ));
        assert_eq!(workspace.toggle_value(GlobalToggle::VirtualCamera), None);
        assert_eq!(
            workspace.toggle_value(GlobalToggle::StudioMode),
            Some(false)
        );
        workspace.invalidate();
        assert!(workspace.active_action.is_none() && workspace.queued_toggles.is_empty());
        assert_eq!(workspace.toggle_value(GlobalToggle::StudioMode), None);
        assert_eq!(workspace.toggle_value(GlobalToggle::VirtualCamera), None);
    }

    #[test]
    fn workspace_scene_action_requires_known_studio_mode() {
        use crate::domain::control::ControlAction;
        let unknown = controllable_workspace(None);
        assert!(unknown.scene_action("Target").is_none());
        assert!(super::WorkspaceState::default()
            .scene_action("Target")
            .is_none());
        assert_eq!(
            controllable_workspace(Some(false)).scene_action("Target"),
            Some(ControlAction::ActivateScene {
                scene: "Target".into(),
                studio_enabled: false
            })
        );
        assert_eq!(
            controllable_workspace(Some(true)).scene_action("Target"),
            Some(ControlAction::ActivateScene {
                scene: "Target".into(),
                studio_enabled: true
            })
        );
    }

    #[test]
    fn scene_controls_survive_scene_readback_but_reject_mode_changes_failures_and_other_actions() {
        for studio in [false, true] {
            let mut workspace = controllable_workspace(Some(studio));
            workspace.observe_scene_controls();
            workspace.loading = true;
            workspace.pending = true;
            workspace.pending_scene = Some("First".into());
            workspace.observed_at = None;
            workspace.dirty = true; // Program echo requires source membership readback.
            assert_eq!(
                workspace.scene_action("Next"),
                Some(crate::domain::control::ControlAction::ActivateScene {
                    scene: "Next".into(),
                    studio_enabled: studio,
                })
            );
            workspace.pending_scene = None;
            assert!(
                workspace.scene_action("Next").is_none(),
                "another mutation owns the workspace"
            );
            workspace.pending = false;
            workspace.invalidate_controls();
            assert!(
                workspace.scene_action("Next").is_none(),
                "an OBS mode change must invalidate cached scene controls"
            );
            workspace.dirty = false;
            workspace.observed_at = Some(Instant::now());
            workspace.observe_scene_controls();
            workspace.fail_read("read failed".into());
            assert!(workspace.scene_action("Next").is_none());
            workspace.error = None;
            workspace.scene_control_observation =
                Some((studio, Instant::now() - std::time::Duration::from_secs(31)));
            assert!(workspace.scene_action("Next").is_none());
            workspace.invalidate();
            assert!(workspace.scene_control_observation.is_none());
        }
    }

    #[test]
    fn background_read_keeps_recent_controls_available_but_never_revives_invalid_data() {
        let mut workspace = controllable_workspace(Some(false));
        workspace.loading = true;
        assert!(workspace.can_control());
        assert!(workspace.scene_action("Target").is_some());
        workspace.dirty = true;
        assert!(!workspace.can_control());
        workspace.dirty = false;
        workspace.observed_at = None;
        assert!(!workspace.can_control());
        workspace.observed_at = Some(Instant::now());
        workspace.pending = true;
        assert!(!workspace.can_control());
    }

    #[test]
    fn workspace_scene_actions_reject_busy_stale_and_failed_observations_in_both_modes() {
        type Invalidate = fn(&mut super::WorkspaceState);
        let cases: [(&str, Invalidate); 5] = [
            ("dirty", |state| state.dirty = true),
            ("pending", |state| state.pending = true),
            ("failed", |state| state.error = Some("Read failed".into())),
            ("expired", |state| {
                state.observed_at = Some(Instant::now() - std::time::Duration::from_secs(31))
            }),
            ("unobserved", |state| state.observed_at = None),
        ];
        for studio in [false, true] {
            for (name, invalidate) in cases {
                let mut state = controllable_workspace(Some(studio));
                assert!(state.scene_action("Target").is_some());
                invalidate(&mut state);
                assert!(!state.can_control(), "{name} state must disable controls");
                assert!(
                    state.scene_action("Target").is_none(),
                    "{name} state dispatched in studio={studio}"
                );
            }
        }
    }
}
