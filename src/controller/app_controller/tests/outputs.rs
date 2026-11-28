//! Stream and record commands routed through `AppController`: missing
//! client, command and status-readback failures, recording stop, and
//! disconnect stopping active outputs.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use futures_util::future::BoxFuture;
use tokio::runtime::Runtime;

use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::AppEvent;
use crate::controller::output_controller::{FakeOutputCommandClient, OutputCommandClient};
use crate::controller::test_support::events::{wait_until, TEST_DEADLINE};
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
            stream_result: Err(AppError::Request("stream backend refused".to_string())),
            record_result: Err(AppError::Request("record backend refused".to_string())),
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
            stream_result: Err(AppError::Request("stream backend refused".to_string())),
            record_result: Err(AppError::Request("record backend refused".to_string())),
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
fn start_streaming_without_client_emits_stream_command_failure() {
    let (_runtime, mut controller, rx) = controller_with_receiver();

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Stream,
        active: true,
    });

    let event = rx
        .recv_timeout(TEST_DEADLINE)
        .expect("stream failure event");
    let AppEvent::OutputCommandFailed(OutputKind::Stream, failure) = event else {
        panic!("expected stream command failure event");
    };
    assert_eq!(failure.message(), "Not connected to OBS");
    assert_eq!(failure.fallback_status(), &inactive_status());
    assert!(rx.try_recv().is_err());
}

#[test]
fn start_recording_without_client_emits_record_command_failure() {
    let (_runtime, mut controller, rx) = controller_with_receiver();

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Record,
        active: true,
    });

    let event = rx
        .recv_timeout(TEST_DEADLINE)
        .expect("record failure event");
    let AppEvent::OutputCommandFailed(OutputKind::Record, failure) = event else {
        panic!("expected record command failure event");
    };
    assert_eq!(failure.message(), "Not connected to OBS");
    assert_eq!(failure.fallback_status(), &inactive_status());
    assert!(rx.try_recv().is_err());
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

#[test]
fn async_stream_command_failure_emits_only_stream_failure_and_refreshes_status() {
    let (_runtime, mut controller, rx) = controller_with_receiver();
    let fake = FakeOutputClient::with_failures();
    controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Stream,
        active: true,
    });

    let events = receive_events(&rx, 4, "stream failure");
    assert!(matches!(
        events[0],
        AppEvent::OutputCommandPending(
            OutputKind::Stream,
            OutputStatus {
                duration_ms: None,
                active: false,
                state: OutputRunState::Starting,
                detail: None
            },
            _
        )
    ));

    let AppEvent::OutputCommandFailed(OutputKind::Stream, failure) = &events[1] else {
        panic!("expected stream command failure after pending event");
    };
    assert!(failure.message().contains("stream backend refused"));
    assert_eq!(failure.fallback_status(), &inactive_status());

    assert!(matches!(
        events[2],
        AppEvent::OutputStatusUpdated(
            OutputKind::Stream,
            OutputStatus {
                duration_ms: None,
                active: false,
                state: OutputRunState::Inactive,
                detail: None
            },
            _
        )
    ));
    assert!(matches!(
        events[3],
        AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus {
                duration_ms: None,
                active: true,
                state: OutputRunState::Active,
                detail: Some(_)
            },
            _
        )
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AppEvent::Error(_))),
        "output command failure should not emit generic AppEvent::Error"
    );
    assert_eq!(fake.stream_status_call_count(), 1);
    assert_eq!(fake.record_status_call_count(), 1);
    assert!(rx.try_recv().is_err());
}

#[test]
fn async_record_command_failure_emits_only_record_failure_and_refreshes_status() {
    let (_runtime, mut controller, rx) = controller_with_receiver();
    let fake = FakeOutputClient::with_failures();
    controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Record,
        active: true,
    });

    let events = receive_events(&rx, 4, "record failure");
    assert!(matches!(
        events[0],
        AppEvent::OutputCommandPending(
            OutputKind::Record,
            OutputStatus {
                duration_ms: None,
                active: false,
                state: OutputRunState::Starting,
                detail: None
            },
            _
        )
    ));

    let AppEvent::OutputCommandFailed(OutputKind::Record, failure) = &events[1] else {
        panic!("expected record command failure after pending event");
    };
    assert!(failure.message().contains("record backend refused"));
    assert_eq!(failure.fallback_status(), &inactive_status());

    assert!(matches!(
        events[2],
        AppEvent::OutputStatusUpdated(
            OutputKind::Stream,
            OutputStatus {
                duration_ms: None,
                active: false,
                state: OutputRunState::Inactive,
                detail: None
            },
            _
        )
    ));
    assert!(matches!(
        events[3],
        AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus {
                duration_ms: None,
                active: true,
                state: OutputRunState::Active,
                detail: Some(_)
            },
            _
        )
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AppEvent::Error(_))),
        "output command failure should not emit generic AppEvent::Error"
    );
    assert_eq!(fake.stream_status_call_count(), 1);
    assert_eq!(fake.record_status_call_count(), 1);
    assert!(rx.try_recv().is_err());
}

#[test]
fn stream_command_failure_with_stream_status_refresh_failure_unblocks_stream() {
    let (_runtime, mut controller, rx) = controller_with_receiver();
    let fake = FakeOutputClient::with_status_results(
        Err(AppError::Request("stream status unavailable".to_string())),
        Ok(active_status()),
    );
    controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Stream,
        active: true,
    });

    let events = receive_events(&rx, 3, "stream command failure with stream refresh failure");
    wait_for_status_calls(&fake, 1, 1);

    assert!(matches!(
        events[0],
        AppEvent::OutputCommandPending(
            OutputKind::Stream,
            OutputStatus {
                duration_ms: None,
                active: false,
                state: OutputRunState::Starting,
                detail: None
            },
            _
        )
    ));
    let AppEvent::OutputCommandFailed(OutputKind::Stream, failure) = &events[1] else {
        panic!("expected stream command failure after pending event");
    };
    assert!(failure.message().contains("stream backend refused"));
    assert_eq!(failure.fallback_status(), &inactive_status());
    assert!(matches!(
        events[2],
        AppEvent::OutputStatusUpdated(OutputKind::Record, _, _)
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AppEvent::Error(_))),
        "localized stream/status failure should not emit generic AppEvent::Error"
    );

    let mut state = app_state();
    mark_obs_connected(&mut state);
    state.set_record_command_failure("existing record error".to_string());
    for event in &events {
        state.apply_output_event(event, 0);
    }
    assert_obs_connected(&state);
    assert_eq!(state.stream_status, inactive_status());
    assert_non_transitioning(&state.stream_status);
    assert!(state
        .last_stream_command_error
        .as_deref()
        .is_some_and(|message| message.contains("stream backend refused")));
    assert_eq!(
        state.last_record_command_error.as_deref(),
        Some("existing record error")
    );
    assert_eq!(fake.stream_status_call_count(), 1);
    assert_eq!(fake.record_status_call_count(), 1);
    assert!(rx.try_recv().is_err());
}

#[test]
fn record_command_failure_with_record_status_refresh_failure_unblocks_record() {
    let (_runtime, mut controller, rx) = controller_with_receiver();
    let fake = FakeOutputClient::with_status_results(
        Ok(inactive_status()),
        Err(AppError::Request("record status unavailable".to_string())),
    );
    controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Record,
        active: true,
    });

    let events = receive_events(&rx, 3, "record command failure with record refresh failure");
    wait_for_status_calls(&fake, 1, 1);

    assert!(matches!(
        events[0],
        AppEvent::OutputCommandPending(
            OutputKind::Record,
            OutputStatus {
                duration_ms: None,
                active: false,
                state: OutputRunState::Starting,
                detail: None
            },
            _
        )
    ));
    let AppEvent::OutputCommandFailed(OutputKind::Record, failure) = &events[1] else {
        panic!("expected record command failure after pending event");
    };
    assert!(failure.message().contains("record backend refused"));
    assert_eq!(failure.fallback_status(), &inactive_status());
    assert!(matches!(
        events[2],
        AppEvent::OutputStatusUpdated(OutputKind::Stream, _, _)
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AppEvent::Error(_))),
        "localized record/status failure should not emit generic AppEvent::Error"
    );

    let mut state = app_state();
    mark_obs_connected(&mut state);
    state.set_stream_command_failure("existing stream error".to_string());
    for event in &events {
        state.apply_output_event(event, 0);
    }
    assert_obs_connected(&state);
    assert_eq!(state.record_status, inactive_status());
    assert_non_transitioning(&state.record_status);
    assert_eq!(
        state.last_stream_command_error.as_deref(),
        Some("existing stream error")
    );
    assert!(state
        .last_record_command_error
        .as_deref()
        .is_some_and(|message| message.contains("record backend refused")));
    assert_eq!(fake.stream_status_call_count(), 1);
    assert_eq!(fake.record_status_call_count(), 1);
    assert!(rx.try_recv().is_err());
}

#[test]
fn stream_command_failure_with_both_status_refreshes_failing_unblocks_stream_stop() {
    let (_runtime, mut controller, rx) = controller_with_receiver();
    let fake = FakeOutputClient::with_status_results(
        Err(AppError::Request("stream status unavailable".to_string())),
        Err(AppError::Request("record status unavailable".to_string())),
    );
    controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Stream,
        active: false,
    });

    let events = receive_events(&rx, 2, "stream command failure with both refreshes failing");
    wait_for_status_calls(&fake, 1, 1);

    assert!(matches!(
        events[0],
        AppEvent::OutputCommandPending(
            OutputKind::Stream,
            OutputStatus {
                duration_ms: None,
                active: true,
                state: OutputRunState::Stopping,
                detail: None
            },
            _
        )
    ));
    let AppEvent::OutputCommandFailed(OutputKind::Stream, failure) = &events[1] else {
        panic!("expected stream command failure after pending event");
    };
    assert!(failure.message().contains("stream backend refused"));
    assert_eq!(
        failure.fallback_status(),
        &OutputStatus {
            duration_ms: None,
            active: true,
            state: OutputRunState::Active,
            detail: None
        }
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AppEvent::Error(_))),
        "localized stream/status failure should not emit generic AppEvent::Error"
    );

    let mut state = app_state();
    mark_obs_connected(&mut state);
    state.set_record_command_failure("existing record error".to_string());
    for event in &events {
        state.apply_output_event(event, 0);
    }
    assert_obs_connected(&state);
    assert_eq!(
        state.stream_status,
        OutputStatus {
            duration_ms: None,
            active: true,
            state: OutputRunState::Active,
            detail: None
        }
    );
    assert_non_transitioning(&state.stream_status);
    assert_eq!(
        state.last_record_command_error.as_deref(),
        Some("existing record error")
    );
    assert_eq!(fake.stream_status_call_count(), 1);
    assert_eq!(fake.record_status_call_count(), 1);
    assert!(rx.try_recv().is_err());
}

#[test]
fn record_command_failure_with_both_status_refreshes_failing_unblocks_record_stop() {
    let (_runtime, mut controller, rx) = controller_with_receiver();
    let fake = FakeOutputClient::with_status_results(
        Err(AppError::Request("stream status unavailable".to_string())),
        Err(AppError::Request("record status unavailable".to_string())),
    );
    controller.set_output_client_override(OutputCommandClient::Fake(fake.clone()));

    controller.handle(AppCommand::SetOutput {
        kind: OutputKind::Record,
        active: false,
    });

    let events = receive_events(&rx, 2, "record command failure with both refreshes failing");
    wait_for_status_calls(&fake, 1, 1);

    assert!(matches!(
        events[0],
        AppEvent::OutputCommandPending(
            OutputKind::Record,
            OutputStatus {
                duration_ms: None,
                active: true,
                state: OutputRunState::Stopping,
                detail: None
            },
            _
        )
    ));
    let AppEvent::OutputCommandFailed(OutputKind::Record, failure) = &events[1] else {
        panic!("expected record command failure after pending event");
    };
    assert!(failure.message().contains("record backend refused"));
    assert_eq!(
        failure.fallback_status(),
        &OutputStatus {
            duration_ms: None,
            active: true,
            state: OutputRunState::Active,
            detail: None
        }
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AppEvent::Error(_))),
        "localized record/status failure should not emit generic AppEvent::Error"
    );

    let mut state = app_state();
    mark_obs_connected(&mut state);
    state.set_stream_command_failure("existing stream error".to_string());
    for event in &events {
        state.apply_output_event(event, 0);
    }
    assert_obs_connected(&state);
    assert_eq!(
        state.record_status,
        OutputStatus {
            duration_ms: None,
            active: true,
            state: OutputRunState::Active,
            detail: None
        }
    );
    assert_non_transitioning(&state.record_status);
    assert_eq!(
        state.last_stream_command_error.as_deref(),
        Some("existing stream error")
    );
    assert_eq!(fake.stream_status_call_count(), 1);
    assert_eq!(fake.record_status_call_count(), 1);
    assert!(rx.try_recv().is_err());
}
