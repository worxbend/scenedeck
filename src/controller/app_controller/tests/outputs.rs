//! Stream and record commands routed through `AppController`: missing
//! client, command and status-readback failures, recording stop, and
//! disconnect stopping active outputs.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use futures_util::future::BoxFuture;
use tokio::runtime::Runtime;

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::{AppEvent, OutputCommandFailureRecovery};
use crate::controller::output_controller::{FakeOutputCommandClient, OutputCommandClient};
use crate::controller::test_support::events::{wait_until, TEST_DEADLINE};
use crate::controller::test_support::output;
use crate::controller::test_support::state::app_state;
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
use crate::infra::error::AppError;

impl AppController {
    fn set_output_client_override(&self, client: OutputCommandClient) {
        self.outputs.set_client_override(client);
    }
}

fn controller_with_receiver() -> (Runtime, AppController, mpsc::Receiver<AppEvent>) {
    let runtime = Runtime::new().expect("test runtime");
    let handle = runtime.handle().clone();
    let (tx, rx) = mpsc::sync_channel(16);
    (runtime, AppController::new(handle, tx), rx)
}

#[derive(Debug)]
struct FakeOutputClient {
    stream_result: Result<(), AppError>,
    record_result: Result<Option<String>, AppError>,
    stream_status: Result<OutputStatus, AppError>,
    record_status: Result<OutputStatus, AppError>,
    stream_commands: Arc<Mutex<Vec<bool>>>,
    record_commands: Arc<Mutex<Vec<bool>>>,
    stream_status_calls: Arc<Mutex<usize>>,
    record_status_calls: Arc<Mutex<usize>>,
}

impl FakeOutputClient {
    fn with_failures() -> Arc<Self> {
        Arc::new(Self {
            stream_result: Err(refused(OutputKind::Stream)),
            record_result: Err(refused(OutputKind::Record)),
            stream_status: Ok(OutputStatus {
                duration_ms: None,
                active: false,
                state: OutputRunState::Inactive,
                detail: None,
            }),
            record_status: Ok(OutputStatus {
                duration_ms: None,
                active: true,
                state: OutputRunState::Active,
                detail: Some("/tmp/current-recording.mkv".to_string()),
            }),
            stream_commands: Arc::new(Mutex::new(Vec::new())),
            record_commands: Arc::new(Mutex::new(Vec::new())),
            stream_status_calls: Arc::new(Mutex::new(0)),
            record_status_calls: Arc::new(Mutex::new(0)),
        })
    }

    fn with_status_results(
        stream_status: Result<OutputStatus, AppError>,
        record_status: Result<OutputStatus, AppError>,
    ) -> Arc<Self> {
        Arc::new(Self {
            stream_result: Err(refused(OutputKind::Stream)),
            record_result: Err(refused(OutputKind::Record)),
            stream_status,
            record_status,
            stream_commands: Arc::new(Mutex::new(Vec::new())),
            record_commands: Arc::new(Mutex::new(Vec::new())),
            stream_status_calls: Arc::new(Mutex::new(0)),
            record_status_calls: Arc::new(Mutex::new(0)),
        })
    }

    fn with_successful_commands(
        stream_status: OutputStatus,
        record_status: OutputStatus,
    ) -> Arc<Self> {
        Arc::new(Self {
            stream_result: Ok(()),
            record_result: Ok(None),
            stream_status: Ok(stream_status),
            record_status: Ok(record_status),
            stream_commands: Arc::new(Mutex::new(Vec::new())),
            record_commands: Arc::new(Mutex::new(Vec::new())),
            stream_status_calls: Arc::new(Mutex::new(0)),
            record_status_calls: Arc::new(Mutex::new(0)),
        })
    }

    fn stream_status_call_count(&self) -> usize {
        *self
            .stream_status_calls
            .lock()
            .expect("stream status calls")
    }

    fn record_status_call_count(&self) -> usize {
        *self
            .record_status_calls
            .lock()
            .expect("record status calls")
    }

    fn stream_commands(&self) -> Vec<bool> {
        self.stream_commands
            .lock()
            .expect("stream commands")
            .clone()
    }

    fn record_commands(&self) -> Vec<bool> {
        self.record_commands
            .lock()
            .expect("record commands")
            .clone()
    }
}

/// What the fake backend answers to a start or stop command on `kind`.
fn backend_refusal(kind: OutputKind) -> &'static str {
    match kind {
        OutputKind::Stream => "stream backend refused",
        OutputKind::Record => "record backend refused",
    }
}

fn refused(kind: OutputKind) -> AppError {
    AppError::Request(backend_refusal(kind).to_string())
}

impl FakeOutputCommandClient for FakeOutputClient {
    fn set_streaming(&self, active: bool) -> BoxFuture<'_, Result<(), AppError>> {
        let result = self.stream_result.clone();
        let commands = Arc::clone(&self.stream_commands);
        Box::pin(async move {
            commands.lock().expect("stream commands").push(active);
            result
        })
    }

    fn set_recording(&self, active: bool) -> BoxFuture<'_, Result<Option<String>, AppError>> {
        let result = self.record_result.clone();
        let commands = Arc::clone(&self.record_commands);
        Box::pin(async move {
            commands.lock().expect("record commands").push(active);
            result
        })
    }

    fn get_stream_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        let status = self.stream_status.clone();
        let calls = Arc::clone(&self.stream_status_calls);
        Box::pin(async move {
            *calls.lock().expect("stream status calls") += 1;
            status
        })
    }

    fn get_record_status(&self) -> BoxFuture<'_, Result<OutputStatus, AppError>> {
        let status = self.record_status.clone();
        let calls = Arc::clone(&self.record_status_calls);
        Box::pin(async move {
            *calls.lock().expect("record status calls") += 1;
            status
        })
    }
}

fn receive_events(rx: &mpsc::Receiver<AppEvent>, count: usize, context: &str) -> Vec<AppEvent> {
    (0..count)
        .map(|_| {
            rx.recv_timeout(TEST_DEADLINE)
                .unwrap_or_else(|_| panic!("timed out waiting for {context} event"))
        })
        .collect()
}

fn inactive_status() -> OutputStatus {
    OutputStatus {
        duration_ms: None,
        active: false,
        state: OutputRunState::Inactive,
        detail: None,
    }
}

fn active_status() -> OutputStatus {
    OutputStatus {
        duration_ms: None,
        active: true,
        state: OutputRunState::Active,
        detail: Some("/tmp/current-recording.mkv".to_string()),
    }
}

fn mark_obs_connected(state: &mut crate::controller::state::AppState) {
    state.set_obs_status(crate::controller::state::ObsStatus::Connected {
        obs_version: "32.1.2".to_string(),
    });
}

fn assert_obs_connected(state: &crate::controller::state::AppState) {
    assert_eq!(
        state.obs_status,
        crate::controller::state::ObsStatus::Connected {
            obs_version: "32.1.2".to_string()
        }
    );
}

fn assert_non_transitioning(status: &OutputStatus) {
    assert!(
        !status.state.is_transitioning(),
        "expected non-transitioning output status, got {status:?}"
    );
}

fn wait_for_status_calls(fake: &FakeOutputClient, stream_calls: usize, record_calls: usize) {
    let reached = wait_until(|| {
        fake.stream_status_call_count() >= stream_calls
            && fake.record_status_call_count() >= record_calls
    });

    assert!(
        reached,
        "expected {stream_calls} stream and {record_calls} record status calls, saw {} and {}",
        fake.stream_status_call_count(),
        fake.record_status_call_count()
    );
}

#[test]
fn disconnect_stops_active_outputs_before_disconnected_event() {
    let (_runtime, mut controller, rx) = controller_with_receiver();
    let fake = FakeOutputClient::with_successful_commands(active_status(), active_status());
    controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

    controller.handle(AppCommand::Disconnect);

    let event = rx.recv_timeout(TEST_DEADLINE).expect("disconnected event");
    assert!(
        matches!(event, AppEvent::SessionEvent { event, .. } if matches!(*event, AppEvent::Disconnected))
    );
    assert_eq!(fake.stream_status_call_count(), 1);
    assert_eq!(fake.record_status_call_count(), 1);
    assert_eq!(fake.stream_commands(), vec![false]);
    assert_eq!(fake.record_commands(), vec![false]);
}

#[test]
fn recording_stop_reports_the_written_file_before_status_readback() {
    let (_runtime, mut controller, rx) = controller_with_receiver();
    let fake = Arc::new(FakeOutputClient {
        record_result: Ok(Some("/tmp/take-1.mkv".to_string())),
        ..Arc::into_inner(FakeOutputClient::with_successful_commands(
            inactive_status(),
            inactive_status(),
        ))
        .expect("fresh fake")
    });
    controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Record,
        active: false,
    });

    let events = receive_events(&rx, 3, "record stop with path");
    assert!(matches!(
        events[0],
        AppEvent::OutputCommandPending(OutputKind::Record, _, _)
    ));
    assert!(matches!(
        events[1],
        AppEvent::OutputCommandSucceeded(OutputKind::Record)
    ));
    assert!(matches!(
        &events[2],
        AppEvent::RecordingStopObserved { path, .. }
            if path == "/tmp/take-1.mkv"
    ));
    assert_eq!(fake.record_commands(), vec![false]);
}

/// A case run once per output. `label` is the name the case had as its own
/// test, so a failure still points at it.
struct OutputCase {
    label: &'static str,
    kind: OutputKind,
}

fn assert_pending(event: &AppEvent, case: &OutputCase, expected: &OutputStatus) {
    let AppEvent::OutputCommandPending(kind, status, _) = event else {
        panic!("{}: expected a pending event, got {event:?}", case.label);
    };
    assert_eq!(*kind, case.kind, "{}", case.label);
    assert_eq!(status, expected, "{}", case.label);
}

fn command_failure<'a>(event: &'a AppEvent, case: &OutputCase) -> &'a OutputCommandFailureRecovery {
    let AppEvent::OutputCommandFailed(kind, failure) = event else {
        panic!("{}: expected a command failure, got {event:?}", case.label);
    };
    assert_eq!(*kind, case.kind, "{}", case.label);
    failure
}

fn assert_no_generic_error(events: &[AppEvent], case: &OutputCase) {
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AppEvent::Error(_))),
        "{}: an output command failure must not emit a generic AppEvent::Error",
        case.label
    );
}

const COMMAND_WITHOUT_CLIENT: [OutputCase; 2] = [
    OutputCase {
        label: "start_streaming_without_client_emits_stream_command_failure",
        kind: OutputKind::Stream,
    },
    OutputCase {
        label: "start_recording_without_client_emits_record_command_failure",
        kind: OutputKind::Record,
    },
];

#[test]
fn starting_an_output_without_client_emits_its_command_failure() {
    for case in COMMAND_WITHOUT_CLIENT {
        let (_runtime, mut controller, rx) = controller_with_receiver();

        controller.handle(AppCommand::SetOutput {
            kind: case.kind,
            active: true,
        });

        let event = rx
            .recv_timeout(TEST_DEADLINE)
            .unwrap_or_else(|_| panic!("{}: no failure event", case.label));
        let failure = command_failure(&event, &case);
        assert_eq!(failure.message(), "Not connected to OBS", "{}", case.label);
        assert_eq!(
            failure.fallback_status(),
            &inactive_status(),
            "{}",
            case.label
        );
        assert!(rx.try_recv().is_err(), "{}", case.label);
    }
}

const ASYNC_COMMAND_FAILURE: [OutputCase; 2] = [
    OutputCase {
        label: "async_stream_command_failure_emits_only_stream_failure_and_refreshes_status",
        kind: OutputKind::Stream,
    },
    OutputCase {
        label: "async_record_command_failure_emits_only_record_failure_and_refreshes_status",
        kind: OutputKind::Record,
    },
];

#[test]
fn async_command_failure_emits_only_its_output_failure_and_refreshes_both_statuses() {
    for case in ASYNC_COMMAND_FAILURE {
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_failures();
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::SetOutput {
            kind: case.kind,
            active: true,
        });

        let events = receive_events(&rx, 4, case.label);
        assert_pending(
            &events[0],
            &case,
            &OutputStatus::new(false, OutputRunState::Starting),
        );
        let failure = command_failure(&events[1], &case);
        assert!(
            failure.message().contains(backend_refusal(case.kind)),
            "{}",
            case.label
        );
        assert_eq!(
            failure.fallback_status(),
            &inactive_status(),
            "{}",
            case.label
        );
        assert!(
            matches!(
                &events[2],
                AppEvent::OutputStatusUpdated(OutputKind::Stream, status, _)
                    if *status == inactive_status()
            ),
            "{}",
            case.label
        );
        assert!(
            matches!(
                &events[3],
                AppEvent::OutputStatusUpdated(OutputKind::Record, status, _)
                    if *status == active_status()
            ),
            "{}",
            case.label
        );
        assert_no_generic_error(&events, &case);
        assert_eq!(fake.stream_status_call_count(), 1, "{}", case.label);
        assert_eq!(fake.record_status_call_count(), 1, "{}", case.label);
        assert!(rx.try_recv().is_err(), "{}", case.label);
    }
}

/// Status readbacks where only `kind`'s fails; the other output reports its
/// usual status.
fn readbacks_failing_for(
    kind: OutputKind,
) -> (
    Result<OutputStatus, AppError>,
    Result<OutputStatus, AppError>,
) {
    match kind {
        OutputKind::Stream => (
            Err(AppError::Request("stream status unavailable".to_string())),
            Ok(active_status()),
        ),
        OutputKind::Record => (
            Ok(inactive_status()),
            Err(AppError::Request("record status unavailable".to_string())),
        ),
    }
}

const OWN_READBACK_FAILURE: [OutputCase; 2] = [
    OutputCase {
        label: "stream_command_failure_with_stream_status_refresh_failure_unblocks_stream",
        kind: OutputKind::Stream,
    },
    OutputCase {
        label: "record_command_failure_with_record_status_refresh_failure_unblocks_record",
        kind: OutputKind::Record,
    },
];

#[test]
fn command_failure_with_its_own_status_readback_failing_unblocks_the_output() {
    for case in OWN_READBACK_FAILURE {
        let other = output::other(case.kind);
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let (stream_status, record_status) = readbacks_failing_for(case.kind);
        let fake = FakeOutputClient::with_status_results(stream_status, record_status);
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::SetOutput {
            kind: case.kind,
            active: true,
        });

        let events = receive_events(&rx, 3, case.label);
        wait_for_status_calls(&fake, 1, 1);

        assert_pending(
            &events[0],
            &case,
            &OutputStatus::new(false, OutputRunState::Starting),
        );
        let failure = command_failure(&events[1], &case);
        assert!(
            failure.message().contains(backend_refusal(case.kind)),
            "{}",
            case.label
        );
        assert_eq!(
            failure.fallback_status(),
            &inactive_status(),
            "{}",
            case.label
        );
        assert!(
            matches!(&events[2], AppEvent::OutputStatusUpdated(kind, _, _) if *kind == other),
            "{}",
            case.label
        );
        assert_no_generic_error(&events, &case);

        let mut state = app_state();
        mark_obs_connected(&mut state);
        output::fail_command(&mut state, other, "existing error");
        for event in &events {
            state.apply_output_event(event, 0);
        }
        assert_obs_connected(&state);
        assert_eq!(
            output::status(&state, case.kind),
            &inactive_status(),
            "{}",
            case.label
        );
        assert_non_transitioning(output::status(&state, case.kind));
        assert!(
            output::command_error(&state, case.kind)
                .is_some_and(|message| message.contains(backend_refusal(case.kind))),
            "{}",
            case.label
        );
        assert_eq!(
            output::command_error(&state, other),
            Some("existing error"),
            "{}",
            case.label
        );
        assert_eq!(fake.stream_status_call_count(), 1, "{}", case.label);
        assert_eq!(fake.record_status_call_count(), 1, "{}", case.label);
        assert!(rx.try_recv().is_err(), "{}", case.label);
    }
}

const BOTH_READBACKS_FAILING: [OutputCase; 2] = [
    OutputCase {
        label: "stream_command_failure_with_both_status_refreshes_failing_unblocks_stream_stop",
        kind: OutputKind::Stream,
    },
    OutputCase {
        label: "record_command_failure_with_both_status_refreshes_failing_unblocks_record_stop",
        kind: OutputKind::Record,
    },
];

#[test]
fn stop_failure_with_both_status_readbacks_failing_unblocks_the_output() {
    let still_active = OutputStatus::new(true, OutputRunState::Active);
    for case in BOTH_READBACKS_FAILING {
        let other = output::other(case.kind);
        let (_runtime, mut controller, rx) = controller_with_receiver();
        let fake = FakeOutputClient::with_status_results(
            Err(AppError::Request("stream status unavailable".to_string())),
            Err(AppError::Request("record status unavailable".to_string())),
        );
        controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

        controller.handle(AppCommand::SetOutput {
            kind: case.kind,
            active: false,
        });

        let events = receive_events(&rx, 2, case.label);
        wait_for_status_calls(&fake, 1, 1);

        assert_pending(
            &events[0],
            &case,
            &OutputStatus::new(true, OutputRunState::Stopping),
        );
        let failure = command_failure(&events[1], &case);
        assert!(
            failure.message().contains(backend_refusal(case.kind)),
            "{}",
            case.label
        );
        assert_eq!(failure.fallback_status(), &still_active, "{}", case.label);
        assert_no_generic_error(&events, &case);

        let mut state = app_state();
        mark_obs_connected(&mut state);
        output::fail_command(&mut state, other, "existing error");
        for event in &events {
            state.apply_output_event(event, 0);
        }
        assert_obs_connected(&state);
        assert_eq!(
            output::status(&state, case.kind),
            &still_active,
            "{}",
            case.label
        );
        assert_non_transitioning(output::status(&state, case.kind));
        assert_eq!(
            output::command_error(&state, other),
            Some("existing error"),
            "{}",
            case.label
        );
        assert_eq!(fake.stream_status_call_count(), 1, "{}", case.label);
        assert_eq!(fake.record_status_call_count(), 1, "{}", case.label);
        assert!(rx.try_recv().is_err(), "{}", case.label);
    }
}
