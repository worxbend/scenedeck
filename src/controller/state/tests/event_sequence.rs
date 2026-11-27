//! Golden trace of one OBS session through the `AppState` reducers.
//!
//! Each step feeds the events `ui/window.rs` would apply, through the same
//! calls, and the trace records what the shell reads afterwards. The trace
//! must not change while the reducers are split into slices: a refactor
//! that moves a statement across the recording-path fence, drops an
//! activity entry or a workspace touch, or reorders a stop-ticket update
//! changes a step and fails here.

use std::time::{Duration, Instant};

use super::{app_state, obs_stats, stream_health};
use crate::controller::command::AppCommand;
use crate::controller::event::{AppEvent, ConnectionInfo};
use crate::controller::lifecycle::{OperationEvidence, OperationKind, SessionRetirement};
use crate::controller::state::{AppState, ObsStatus};
use crate::domain::health::AlertKind;
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
use crate::domain::output_result::OutputResultStatus;
use crate::domain::workflow::ActivityOrigin;

const FIRST_TAKE: &str = "/rec/take-1.mkv";
const SECOND_TAKE: &str = "/rec/take-2.mkv";
const READBACK_ERROR: &str = "stream status unavailable";
const LOST_STOP: &str = "stop lost";

/// Which `ObsStatus` variant is showing. Labels are localized, so the
/// trace keeps only the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Connection {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl Connection {
    fn of(status: &ObsStatus) -> Self {
        match status {
            ObsStatus::Disconnected => Self::Disconnected,
            ObsStatus::Connecting => Self::Connecting,
            ObsStatus::Connected { .. } => Self::Connected,
            ObsStatus::Error(_) => Self::Error,
        }
    }
}

/// One output as the status bar, the clocks and health alerts see it.
#[derive(Debug, PartialEq)]
struct Channel<'a> {
    active: bool,
    state: OutputRunState,
    clock_running: bool,
    observed_active: Option<bool>,
    error: Option<&'a str>,
}

/// One entry of the output results history: its status and file.
type ResultRow<'a> = (OutputResultStatus, Option<&'a str>);

/// The `AppState` fields the shell reads after an output, scene or session
/// event. Instants are reduced to whether they are set, so the trace is
/// independent of when the test runs.
#[derive(Debug, PartialEq)]
struct Snapshot<'a> {
    connection: Connection,
    program: Option<&'a str>,
    previous: Option<&'a str>,
    stream: Channel<'a>,
    record: Channel<'a>,
    recording_path: Option<&'a str>,
    stop_ticket_open: bool,
    output_results: Vec<ResultRow<'a>>,
    alerts: Vec<AlertKind>,
    activity: usize,
    last_activity: Option<ActivityOrigin>,
    workspace_revision: u64,
    stats_samples: usize,
    bitrate_kbps: Option<f64>,
    retired_sessions: usize,
}

impl<'a> Snapshot<'a> {
    fn of(state: &'a AppState) -> Self {
        Self {
            connection: Connection::of(&state.obs_status),
            program: state.scene_inventory.current_id.as_deref(),
            previous: state.scene_inventory.previous_id.as_deref(),
            stream: Channel::of(state, OutputKind::Stream),
            record: Channel::of(state, OutputKind::Record),
            recording_path: state.last_recording_path.as_deref(),
            stop_ticket_open: state.workspace.recording_stop_result.is_some(),
            output_results: state
                .workspace
                .output_results
                .entries()
                .map(|result| (result.status, result.path.as_deref()))
                .collect(),
            alerts: state
                .health
                .alerts(Instant::now())
                .iter()
                .map(|alert| alert.kind)
                .collect(),
            activity: state.workspace.activity.entries.len(),
            last_activity: state
                .workspace
                .activity
                .entries
                .back()
                .map(|entry| entry.origin),
            workspace_revision: state.workspace.revision(),
            stats_samples: state.stats_history.len(),
            bitrate_kbps: state.stream_bitrate_kbps,
            retired_sessions: state.session_retirements.len(),
        }
    }
}

impl<'a> Channel<'a> {
    fn of(state: &'a AppState, kind: OutputKind) -> Self {
        let (status, since, error) = match kind {
            OutputKind::Stream => (
                &state.stream_status,
                state.stream_active_since,
                &state.last_stream_command_error,
            ),
            OutputKind::Record => (
                &state.record_status,
                state.record_active_since,
                &state.last_record_command_error,
            ),
        };
        Self {
            active: status.active,
            state: status.state,
            clock_running: since.is_some(),
            observed_active: state.health.output_activity(kind),
            error: error.as_deref(),
        }
    }
}

/// The session, one step per entry of [`golden_trace`]. Event times are
/// offsets from a base instant two minutes in the past.
const SESSION: [fn(&mut AppState, Instant); 12] = [
    connect,
    read_inventory,
    change_program,
    start_stream,
    poll_stats,
    start_record,
    stop_record,
    restart_record,
    observe_late_file,
    fail_stream_readback,
    retire_session,
    disconnect,
];

fn at(base: Instant, seconds: u64) -> Instant {
    base + Duration::from_secs(seconds)
}

fn now_ms() -> u64 {
    crate::controller::workspace::unix_ms()
}

fn output(state: &mut AppState, event: AppEvent) {
    let kind = state.apply_output_event(&event, now_ms());
    assert!(kind.is_some(), "{event:?} is an output event");
}

fn connect(state: &mut AppState, _base: Instant) {
    state.note_command_intent(&AppCommand::Connect, now_ms());
    state.reset_obs_session(ObsStatus::Connecting);
    state.apply_connected(ConnectionInfo {
        connection_id: "studio".into(),
        endpoint: "ws://studio:4455".into(),
        obs_version: "32.0.0".into(),
        websocket_version: "5.6".into(),
        available_requests: vec!["GetVersion".into()],
        supported_image_formats: vec!["png".into()],
    });
}

fn read_inventory(state: &mut AppState, _base: Instant) {
    state.apply_scene_inventory(crate::domain::scene::SceneInventory {
        current_id: Some("Camera".into()),
        ..Default::default()
    });
}

/// The state half of the `CurrentSceneChanged` arm in `ui/window.rs`.
fn change_program(state: &mut AppState, _base: Instant) {
    state
        .workspace
        .log(ActivityOrigin::Observed, "Current scene: Slides".into());
    state.workspace.mark_dirty();
    state.set_current_program("Slides".into());
}

fn start_stream(state: &mut AppState, base: Instant) {
    let set = AppCommand::SetOutput {
        kind: OutputKind::Stream,
        active: true,
    };
    state.note_command_intent(&set, now_ms());
    let starting = OutputStatus::new(false, OutputRunState::Starting);
    output(
        state,
        AppEvent::OutputCommandPending(OutputKind::Stream, starting, at(base, 10)),
    );
    output(state, AppEvent::OutputCommandSucceeded(OutputKind::Stream));
    let active = OutputStatus::active();
    output(
        state,
        AppEvent::OutputStatusUpdated(OutputKind::Stream, active, at(base, 11)),
    );
}

fn poll_stats(state: &mut AppState, base: Instant) {
    let stats = obs_stats(60.0);
    state.set_obs_stats_at(stats, Some(6_000.0), Some(stream_health()), at(base, 12));
}

fn start_record(state: &mut AppState, base: Instant) {
    let active = OutputStatus::active();
    output(
        state,
        AppEvent::OutputStatusUpdated(OutputKind::Record, active, at(base, 20)),
    );
}

/// A live stop opens a ticket; the controller publishes the ack, then the
/// file the stop response named, then the inactive echo.
fn stop_record(state: &mut AppState, base: Instant) {
    let set = AppCommand::SetOutput {
        kind: OutputKind::Record,
        active: false,
    };
    state.note_command_intent(&set, now_ms());
    let stopping = OutputStatus::new(true, OutputRunState::Stopping);
    output(
        state,
        AppEvent::OutputCommandPending(OutputKind::Record, stopping, at(base, 30)),
    );
    output(state, AppEvent::OutputCommandSucceeded(OutputKind::Record));
    let path = FIRST_TAKE.into();
    let command_at = at(base, 30);
    output(state, AppEvent::RecordingStopObserved { path, command_at });
    let stopped = OutputStatus::inactive().with_detail(FIRST_TAKE);
    output(
        state,
        AppEvent::OutputStatusUpdated(OutputKind::Record, stopped, at(base, 31)),
    );
}

fn restart_record(state: &mut AppState, base: Instant) {
    let active = OutputStatus::active();
    output(
        state,
        AppEvent::OutputStatusUpdated(OutputKind::Record, active, at(base, 40)),
    );
}

/// An older stop echo with a file path lands after the restart. The file
/// is evidence; the stale inactive state is not.
fn observe_late_file(state: &mut AppState, base: Instant) {
    let stopped = OutputStatus::inactive().with_detail(SECOND_TAKE);
    output(
        state,
        AppEvent::OutputStatusUpdated(OutputKind::Record, stopped, at(base, 35)),
    );
}

/// The state half of the `OutputReadbackFailed` arm in `ui/window.rs`,
/// which has no `AppState` reducer yet. When that arm moves into state,
/// call the new reducer here; the trace must not change.
fn fail_stream_readback(state: &mut AppState, base: Instant) {
    if !state.accept_output_observation(OutputKind::Stream, at(base, 50)) {
        return;
    }
    let changed = state.last_stream_command_error.as_deref() != Some(READBACK_ERROR);
    state.health.output_unobserved(OutputKind::Stream);
    state.set_stream_status(OutputStatus::new(false, OutputRunState::Unknown));
    state.last_stream_command_error = Some(READBACK_ERROR.into());
    if changed {
        state
            .workspace
            .log(ActivityOrigin::Failed, READBACK_ERROR.into());
    }
}

/// `ui/window.rs` resets the current session before it records the
/// retirement evidence.
fn retire_session(state: &mut AppState, _base: Instant) {
    state.reset_obs_session(ObsStatus::Error("transport closed".into()));
    let lost_stop = OperationEvidence {
        id: 7,
        kind: OperationKind::Record,
        endpoint: "ws://studio:4455".into(),
        collection: 1,
        dispatched: true,
        acknowledged: false,
        uncertain: true,
        completed: false,
        failure: Some(LOST_STOP.into()),
    };
    let retirement = SessionRetirement {
        generation: 1,
        reason: "transport closed".into(),
        operations: vec![lost_stop],
        omitted_sessions: 0,
    };
    state.apply_terminal_evidence(retirement, true);
}

fn disconnect(state: &mut AppState, _base: Instant) {
    state.note_command_intent(&AppCommand::Disconnect, now_ms());
    state.reset_obs_session(ObsStatus::Disconnected);
}

#[test]
fn one_session_through_the_reducers_matches_the_golden_trace() {
    let base = Instant::now() - Duration::from_secs(120);
    let mut state = app_state();
    let golden = golden_trace();
    assert_eq!(golden.len(), SESSION.len(), "one golden snapshot per step");

    for (apply, (step, expected)) in SESSION.iter().zip(golden) {
        apply(&mut state, base);
        assert_eq!(Snapshot::of(&state), expected, "after step {step:?}");
    }
}

const UNKNOWN: Channel<'static> = Channel {
    active: false,
    state: OutputRunState::Unknown,
    clock_running: false,
    observed_active: None,
    error: None,
};

const LIVE: Channel<'static> = Channel {
    active: true,
    state: OutputRunState::Active,
    clock_running: true,
    observed_active: Some(true),
    error: None,
};

const IDLE: Channel<'static> = Channel {
    active: false,
    state: OutputRunState::Inactive,
    clock_running: false,
    observed_active: None,
    error: None,
};

const ACKED_STOP: ResultRow<'static> = (OutputResultStatus::Acknowledged, None);
const UNCONFIRMED_STOP: ResultRow<'static> = (OutputResultStatus::Unconfirmed, None);
const FIRST_FILE: ResultRow<'static> = (OutputResultStatus::Observed, Some(FIRST_TAKE));
const SECOND_FILE: ResultRow<'static> = (OutputResultStatus::Observed, Some(SECOND_TAKE));

/// What each step of [`SESSION`] leaves behind, recorded from the reducers
/// at the commit that added this test.
fn golden_trace() -> Vec<(&'static str, Snapshot<'static>)> {
    vec![
        (
            "connect",
            Snapshot {
                connection: Connection::Connected,
                program: None,
                previous: None,
                stream: UNKNOWN,
                record: UNKNOWN,
                recording_path: None,
                stop_ticket_open: false,
                output_results: vec![],
                alerts: vec![],
                activity: 1,
                last_activity: Some(ActivityOrigin::Connection),
                workspace_revision: 3,
                stats_samples: 0,
                bitrate_kbps: None,
                retired_sessions: 0,
            },
        ),
        (
            "scene inventory",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Camera"),
                previous: None,
                stream: UNKNOWN,
                record: UNKNOWN,
                recording_path: None,
                stop_ticket_open: false,
                output_results: vec![],
                alerts: vec![],
                activity: 1,
                last_activity: Some(ActivityOrigin::Connection),
                workspace_revision: 3,
                stats_samples: 0,
                bitrate_kbps: None,
                retired_sessions: 0,
            },
        ),
        (
            "program change",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Slides"),
                previous: Some("Camera"),
                stream: UNKNOWN,
                record: UNKNOWN,
                recording_path: None,
                stop_ticket_open: false,
                output_results: vec![],
                alerts: vec![],
                activity: 2,
                last_activity: Some(ActivityOrigin::Observed),
                workspace_revision: 5,
                stats_samples: 0,
                bitrate_kbps: None,
                retired_sessions: 0,
            },
        ),
        (
            "stream start",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Slides"),
                previous: Some("Camera"),
                stream: LIVE,
                record: UNKNOWN,
                recording_path: None,
                stop_ticket_open: false,
                output_results: vec![],
                alerts: vec![],
                activity: 4,
                last_activity: Some(ActivityOrigin::Observed),
                workspace_revision: 8,
                stats_samples: 0,
                bitrate_kbps: None,
                retired_sessions: 0,
            },
        ),
        (
            "stats poll",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Slides"),
                previous: Some("Camera"),
                stream: LIVE,
                record: UNKNOWN,
                recording_path: None,
                stop_ticket_open: false,
                output_results: vec![],
                alerts: vec![],
                activity: 4,
                last_activity: Some(ActivityOrigin::Observed),
                workspace_revision: 8,
                stats_samples: 1,
                bitrate_kbps: Some(6_000.0),
                retired_sessions: 0,
            },
        ),
        (
            "record start",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Slides"),
                previous: Some("Camera"),
                stream: LIVE,
                record: LIVE,
                recording_path: None,
                stop_ticket_open: false,
                output_results: vec![],
                alerts: vec![],
                activity: 5,
                last_activity: Some(ActivityOrigin::Observed),
                workspace_revision: 10,
                stats_samples: 1,
                bitrate_kbps: Some(6_000.0),
                retired_sessions: 0,
            },
        ),
        (
            // The ticket, the stop response's file and the inactive echo's
            // file are three results.
            "record stop with path",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Slides"),
                previous: Some("Camera"),
                stream: LIVE,
                record: Channel {
                    observed_active: Some(false),
                    ..IDLE
                },
                recording_path: Some(FIRST_TAKE),
                stop_ticket_open: false,
                output_results: vec![ACKED_STOP, FIRST_FILE, FIRST_FILE],
                alerts: vec![],
                activity: 7,
                last_activity: Some(ActivityOrigin::Observed),
                workspace_revision: 16,
                stats_samples: 1,
                bitrate_kbps: Some(6_000.0),
                retired_sessions: 0,
            },
        ),
        (
            "record restart",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Slides"),
                previous: Some("Camera"),
                stream: LIVE,
                record: LIVE,
                recording_path: Some(FIRST_TAKE),
                stop_ticket_open: false,
                output_results: vec![ACKED_STOP, FIRST_FILE, FIRST_FILE],
                alerts: vec![],
                activity: 8,
                last_activity: Some(ActivityOrigin::Observed),
                workspace_revision: 18,
                stats_samples: 1,
                bitrate_kbps: Some(6_000.0),
                retired_sessions: 0,
            },
        ),
        (
            // The file is recorded before the fence drops the stale state.
            "late file evidence",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Slides"),
                previous: Some("Camera"),
                stream: LIVE,
                record: LIVE,
                recording_path: Some(SECOND_TAKE),
                stop_ticket_open: false,
                output_results: vec![ACKED_STOP, FIRST_FILE, FIRST_FILE, SECOND_FILE],
                alerts: vec![],
                activity: 8,
                last_activity: Some(ActivityOrigin::Observed),
                workspace_revision: 19,
                stats_samples: 1,
                bitrate_kbps: Some(6_000.0),
                retired_sessions: 0,
            },
        ),
        (
            // The window arm leaves the stream clock running.
            "stream readback failure",
            Snapshot {
                connection: Connection::Connected,
                program: Some("Slides"),
                previous: Some("Camera"),
                stream: Channel {
                    clock_running: true,
                    error: Some(READBACK_ERROR),
                    ..UNKNOWN
                },
                record: LIVE,
                recording_path: Some(SECOND_TAKE),
                stop_ticket_open: false,
                output_results: vec![ACKED_STOP, FIRST_FILE, FIRST_FILE, SECOND_FILE],
                alerts: vec![],
                activity: 9,
                last_activity: Some(ActivityOrigin::Failed),
                workspace_revision: 20,
                stats_samples: 1,
                bitrate_kbps: Some(6_000.0),
                retired_sessions: 0,
            },
        ),
        (
            "terminal evidence",
            Snapshot {
                connection: Connection::Error,
                program: None,
                previous: None,
                stream: UNKNOWN,
                record: Channel {
                    error: Some(LOST_STOP),
                    ..UNKNOWN
                },
                recording_path: None,
                stop_ticket_open: false,
                output_results: vec![UNCONFIRMED_STOP, FIRST_FILE, FIRST_FILE, SECOND_FILE],
                alerts: vec![AlertKind::ConnectionLost],
                activity: 11,
                last_activity: Some(ActivityOrigin::Connection),
                workspace_revision: 23,
                stats_samples: 0,
                bitrate_kbps: None,
                retired_sessions: 1,
            },
        ),
        (
            "disconnect",
            Snapshot {
                connection: Connection::Disconnected,
                program: None,
                previous: None,
                stream: IDLE,
                record: IDLE,
                recording_path: None,
                stop_ticket_open: false,
                output_results: vec![UNCONFIRMED_STOP, FIRST_FILE, FIRST_FILE, SECOND_FILE],
                alerts: vec![AlertKind::ConnectionLost],
                activity: 12,
                last_activity: Some(ActivityOrigin::Connection),
                workspace_revision: 25,
                stats_samples: 0,
                bitrate_kbps: None,
                retired_sessions: 1,
            },
        ),
    ]
}
