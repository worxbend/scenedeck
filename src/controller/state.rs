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

mod audio_presets;
mod bookmarks;
mod report_history;
mod session_reports;
pub(crate) use report_history::ReportHistoryState;
pub(crate) use session_reports::{ReportExit, ReportsState};
mod mixer;
pub(crate) use bookmarks::{BookmarkError, BookmarksState};
mod show_registry;
pub use mixer::*;
pub(crate) use show_registry::{RegistryStatus, ShowRegistryState};

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
    pub operator_inspections: Vec<(u64, crate::domain::operator_action::ActionTarget)>,
    next_operator_inspection: u64,
    pub input_catalog: Option<Result<Vec<crate::domain::operator_input::InputIdentity>, String>>,
    pub operator_inputs: Vec<crate::domain::operator_input::InputObservation>,
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
    pub fn inspect_operator_input(
        &mut self,
        target: crate::domain::operator_action::ActionTarget,
    ) -> u64 {
        self.next_operator_inspection = self.next_operator_inspection.wrapping_add(1);
        let token = self.next_operator_inspection;
        self.operator_inspections.push((token, target));
        self.mark_dirty();
        token
    }

    pub fn release_operator_inspection(&mut self, token: u64) {
        self.operator_inspections.retain(|(id, _)| *id != token);
    }

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
        self.input_catalog = None;
        self.operator_inputs.clear();
        self.operator_inspections.clear();
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
    pub(crate) preset_run: Option<crate::domain::audio_preset::PresetRun>,
    preset_serial: u64,
    pub report_history: ReportHistoryState,
    pub reports: ReportsState,
    pub bookmarks: BookmarksState,
    pub health: crate::domain::health::HealthMonitor,
    pub workspace: WorkspaceState,
    pub session_retirements: std::collections::VecDeque<super::lifecycle::SessionRetirement>,
    pub available_requests: Vec<String>,
    pub supported_image_formats: Vec<String>,
    pub connected_endpoint: Option<String>,
    pub connected_connection_id: Option<String>,
    /// Cached local persistence snapshots. GTK reads these values only from
    /// memory; blocking adapters update them through the background-I/O bridge.
    pub config: AppConfig,
    pub registry: SceneRegistry,
    pub(crate) show_registry: ShowRegistryState,
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
    stream_observed_at: Option<Instant>,
    record_observed_at: Option<Instant>,
    record_path_observed_at: Option<Instant>,
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
    /// Config and registry load notices, shown once on the Settings page.
    pub startup_notices: Vec<String>,
    /// Latest OBS `GetStats` snapshot for the status bar. `None` until the
    /// first poll completes after connecting.
    pub obs_stats: Option<ObsStats>,
    pub obs_stats_observed_at: Option<Instant>,
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
    pub fn new(config: AppConfig, registry: SceneRegistry, startup_notices: Vec<String>) -> Self {
        let mixer = config.mixer.clone();
        Self {
            preset_run: None,
            preset_serial: 0,
            report_history: ReportHistoryState::default(),
            reports: ReportsState::default(),
            bookmarks: BookmarksState::default(),
            health: crate::domain::health::HealthMonitor::default(),
            workspace: WorkspaceState::default(),
            session_retirements: std::collections::VecDeque::new(),
            available_requests: Vec::new(),
            supported_image_formats: Vec::new(),
            connected_endpoint: None,
            connected_connection_id: None,
            config,
            registry,
            show_registry: ShowRegistryState::default(),
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
            stream_observed_at: None,
            record_observed_at: None,
            record_path_observed_at: None,
            audio_inputs: Vec::new(),
            active_audio_status: ActiveAudioStatus::Missing,
            active_audio_revision: 0,
            observations: super::observations::Observations::default(),
            audio_levels: InputLevelSnapshot::default(),
            mixer_audio_refresh: MixerAudioRefreshState::default(),
            mixer,
            startup_notices,
            obs_stats: None,
            obs_stats_observed_at: None,
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
        if matches!(cmd, AppCommand::Connect | AppCommand::Disconnect) {
            self.health.connection_requested();
        }
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

    /// Request-start/ingress time fences both transport events and readback results.
    /// Output identity belongs to the connection, so collection changes keep this fence.
    pub(crate) fn accept_output_observation(&mut self, kind: OutputKind, at: Instant) -> bool {
        let latest = match kind {
            OutputKind::Stream => &mut self.stream_observed_at,
            OutputKind::Record => &mut self.record_observed_at,
        };
        if latest.is_some_and(|latest| at < latest) {
            return false;
        }
        *latest = Some(at);
        true
    }

    fn observe_recording_path(&mut self, path: &str, at: Instant, stopped: bool, now_ms: u64) {
        if path.is_empty() {
            return;
        }
        self.workspace.touch();
        if stopped {
            let scope = self.workflow_scope();
            self.workspace.output_results.observe(
                crate::domain::output_result::OutputResultKind::RecordingStop,
                scope,
                Some(path.to_owned()),
                now_ms,
            );
        }
        if self
            .record_path_observed_at
            .is_none_or(|latest| at >= latest)
        {
            self.record_path_observed_at = Some(at);
            self.last_recording_path = Some(path.to_owned());
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
        use crate::domain::workflow::ActivityOrigin;

        if let AppEvent::OutputStatusUpdated(kind, status, at) = event {
            let previous = match kind {
                OutputKind::Stream => &self.stream_status,
                OutputKind::Record => &self.record_status,
            };
            let changed = !previous.same_run_state(status);
            // File evidence can arrive after a newer running-state echo. Preserve
            // that fact without rolling the current output or latest path backward.
            if *kind == OutputKind::Record {
                if let Some(path) = &status.detail {
                    self.observe_recording_path(path, *at, !status.active && changed, now_ms);
                }
            }
            if !self.accept_output_observation(*kind, *at) {
                return Some(*kind);
            }
            self.health.observe_output(*kind, status, *at);
            if changed {
                self.workspace.log(
                    ActivityOrigin::Observed,
                    format!("{}: {}", kind.label(), status.state.label()),
                );
                self.workspace.mark_dirty();
            }
        }
        match event {
            AppEvent::OutputCommandPending(kind, status, at) => {
                self.health.output_pending(*kind, status, *at)
            }
            AppEvent::OutputCommandFailed(kind, failure) => {
                self.health.output_failed(*kind, failure.uncertain())
            }
            _ => {}
        }

        let kind = match event {
            AppEvent::RecordingStopObserved { path, command_at } => {
                self.observe_recording_path(path, *command_at, true, now_ms);
                OutputKind::Record
            }
            AppEvent::OutputStatusUpdated(OutputKind::Stream, status, _) => {
                update_active_since(status.active, &mut self.stream_active_since);
                self.set_stream_status(status.clone());
                OutputKind::Stream
            }
            AppEvent::OutputStatusUpdated(OutputKind::Record, status, _) => {
                update_active_since(status.active, &mut self.record_active_since);
                self.set_record_status(status.clone());
                OutputKind::Record
            }
            AppEvent::OutputCommandPending(OutputKind::Stream, status, _) => {
                update_active_since(status.active, &mut self.stream_active_since);
                self.set_stream_command_pending(status.clone());
                OutputKind::Stream
            }
            AppEvent::OutputCommandPending(OutputKind::Record, status, _) => {
                update_active_since(status.active, &mut self.record_active_since);
                self.set_record_command_pending(status.clone());
                OutputKind::Record
            }
            AppEvent::OutputCommandSucceeded(OutputKind::Stream) => {
                self.report_output_ack(OutputKind::Stream, now_ms);
                self.set_stream_command_success();
                OutputKind::Stream
            }
            AppEvent::OutputCommandSucceeded(OutputKind::Record) => {
                self.report_output_ack(OutputKind::Record, now_ms);
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
    pub fn apply_connected(&mut self, info: crate::domain::obs::ConnectionInfo) -> ObsStatus {
        use crate::domain::output::OutputRunState;
        self.finish_report(&ObsStatus::Connecting.label());
        self.health.connected();
        self.bookmarks
            .begin_session(Instant::now(), crate::controller::workspace::unix_ms());
        self.invalidate_registry_selection();
        self.available_requests = info.available_requests;
        self.supported_image_formats = info.supported_image_formats;
        self.connected_endpoint = Some(info.endpoint);
        self.connected_connection_id = Some(info.connection_id);
        self.workspace.mark_dirty();
        self.obs_status = ObsStatus::Connected {
            obs_version: info.obs_version,
        };
        self.stream_status = OutputStatus::new(false, OutputRunState::Unknown);
        self.record_status = OutputStatus::new(false, OutputRunState::Unknown);
        self.begin_report();
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
        self.retire_preset();
        self.collect_report(Instant::now(), crate::controller::workspace::unix_ms());
        self.invalidate_registry_selection();
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
        self.collect_report(Instant::now(), crate::controller::workspace::unix_ms());
        self.health.disconnected();
        self.invalidate_collection_runtime();
        self.workspace.log(
            crate::domain::workflow::ActivityOrigin::Connection,
            status.label(),
        );
        self.finish_report(&status.label());
        self.bookmarks.end_session();
        self.available_requests.clear();
        self.supported_image_formats.clear();
        self.connected_endpoint = None;
        self.connected_connection_id = None;
        self.profiles = ObsNamedList::default();
        self.scene_collections = ObsNamedList::default();
        self.set_obs_status(status);
        self.set_stream_status(OutputStatus::default());
        self.set_record_status(OutputStatus::default());
        self.stream_active_since = None;
        self.record_active_since = None;
        self.last_recording_path = None;
        self.stream_observed_at = None;
        self.record_observed_at = None;
        self.record_path_observed_at = None;
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
        for operation in &terminal.operations {
            self.preset_terminal(operation);
        }
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
        self.collect_report(Instant::now(), crate::controller::workspace::unix_ms());
        self.workspace
            .log(crate::domain::workflow::ActivityOrigin::Connection, audit);
        self.report_terminal_audit(terminal.generation);
        self.session_retirements.push_back(terminal);
        while self.session_retirements.len() > super::lifecycle::TERMINAL_HISTORY {
            self.session_retirements.pop_front();
        }
    }

    /// Store the newest volume-meter readings, replacing the previous set.
    pub fn set_audio_levels_at(&mut self, levels: Vec<InputLevels>, at: Instant) {
        self.audio_levels.update_at(levels, at);
    }

    /// Forget every metered level, so meters fall silent instead of freezing.
    pub fn clear_audio_levels(&mut self) {
        self.audio_levels.clear();
    }

    /// Evaluate only the actual, ready workspace, independent of the visible page.
    pub(crate) fn observe_audio_health(&mut self, now: Instant) -> bool {
        let scope = (self.show_registry.status == RegistryStatus::Ready
            && self.desired_registry_target().as_ref() == Some(&self.show_registry.target))
        .then_some((self.workspace.epoch, self.show_registry.generation));
        let inputs = self
            .workspace
            .can_control()
            .then_some(())
            .and(self.workspace.input_catalog.as_ref())
            .and_then(|inputs| inputs.as_ref().ok())
            .map(Vec::as_slice);
        self.health.audio.observe(
            scope,
            &self.registry.audio_alerts,
            crate::domain::health::audio::AudioSnapshot {
                inputs,
                audio: &self.workspace.audio,
                levels: &self.audio_levels,
            },
            now,
        );
        scope.is_some()
            && self
                .registry
                .audio_alerts
                .iter()
                .any(|rule| rule.enabled && (rule.silence || rule.clipping))
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

    pub(crate) fn merge_workspace_audio(
        &self,
        observed: &mut super::workspace::WorkspaceObservation,
    ) {
        self.observations.merge_workspace_audio(observed);
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

    pub fn set_obs_stats_at(
        &mut self,
        stats: ObsStats,
        bitrate_kbps: Option<f64>,
        stream: Option<StreamHealth>,
        at: Instant,
    ) {
        let recording = self.health.output_activity(OutputKind::Record);
        let streaming = self.health.output_activity(OutputKind::Stream);
        let encoding = match (recording, streaming) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        };
        self.health
            .observe_stats(stats, stream, recording, encoding, at, Instant::now());
        self.obs_stats_observed_at = Some(at);
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
        self.health.clear_samples();
        self.obs_stats_observed_at = None;
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
mod tests;
