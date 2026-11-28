//! Stream and record output commands, status events and the recording stop
//! ticket.

use std::time::Instant;

use super::fixtures::{app_state, output_status};
use crate::controller::event::OutputCommandFailureRecovery;
use crate::controller::state::{ObsStatus, Page};
use crate::controller::test_support::state::{fail_record_command, fail_stream_command};
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};

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

    let event = AppEvent::OutputStatusUpdated(
        OutputKind::Stream,
        active.clone(),
        std::time::Instant::now(),
    );
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
        &AppEvent::OutputStatusUpdated(
            OutputKind::Stream,
            OutputStatus::inactive(),
            std::time::Instant::now(),
        ),
        3,
    );
    assert!(state.stream_active_since.is_none());

    // A settled stop opens a ticket; success acknowledges it.
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            active.clone(),
            std::time::Instant::now(),
        ),
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
        &AppEvent::OutputStatusUpdated(OutputKind::Record, stopped, std::time::Instant::now()),
        7,
    );
    assert_eq!(state.last_recording_path.as_deref(), Some("/tmp/take.mkv"));
    assert!(state.record_active_since.is_none());
    let empty = OutputStatus {
        detail: Some(String::new()),
        ..OutputStatus::new(true, OutputRunState::Active)
    };
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(OutputKind::Record, empty, std::time::Instant::now()),
        8,
    );
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
fn late_recording_output_observation_keeps_file_evidence_without_undoing_a_restart() {
    use crate::controller::event::AppEvent;
    let start = Instant::now() - std::time::Duration::from_secs(60);
    let mut state = app_state();
    state.health.connected();
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::active(),
            start + std::time::Duration::from_secs(20),
        ),
        20,
    );
    let revision = state.workspace.revision();
    let stopped = |path: &str| OutputStatus {
        duration_ms: None,
        active: false,
        state: OutputRunState::Inactive,
        detail: Some(path.into()),
    };
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            stopped("first.mkv"),
            start + std::time::Duration::from_secs(10),
        ),
        21,
    );
    assert!(state.record_status.active);
    assert_eq!(state.health.output_activity(OutputKind::Record), Some(true));
    assert!(state.health.alerts(Instant::now()).is_empty());
    assert_eq!(state.last_recording_path.as_deref(), Some("first.mkv"));
    assert!(
        state.workspace.revision() > revision,
        "late file evidence still refreshes result consumers"
    );
    assert!(state
        .workspace
        .output_results
        .entries()
        .any(|result| result.path.as_deref() == Some("first.mkv")));
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::active_with_detail("new.mkv"),
            start + std::time::Duration::from_secs(30),
        ),
        30,
    );
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            stopped("older.mkv"),
            start + std::time::Duration::from_secs(15),
        ),
        31,
    );
    assert_eq!(state.last_recording_path.as_deref(), Some("new.mkv"));
    assert!(state.record_status.active);
    state.invalidate_collection_runtime();
    assert!(
        !state.accept_output_observation(OutputKind::Record, start),
        "collection retirement must retain connection-owned output ordering"
    );
    state.reset_obs_session(ObsStatus::Disconnected);
    assert!(
        state.accept_output_observation(OutputKind::Record, start),
        "a new connection begins a new observation context"
    );
}

#[test]
fn stream_command_failure_sets_stream_error_only() {
    let mut state = app_state();

    fail_stream_command(&mut state, "stream failed");

    assert_eq!(
        state.last_stream_command_error.as_deref(),
        Some("stream failed")
    );
    assert_eq!(state.last_record_command_error, None);
}

#[test]
fn record_command_failure_sets_record_error_only() {
    let mut state = app_state();

    fail_record_command(&mut state, "record failed");

    assert_eq!(state.last_stream_command_error, None);
    assert_eq!(
        state.last_record_command_error.as_deref(),
        Some("record failed")
    );
}

#[test]
fn stream_command_start_failure_applies_inactive_recovery_payload() {
    let mut state = app_state();
    fail_record_command(&mut state, "existing record failure");
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
    fail_record_command(&mut state, "existing record failure");
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
    fail_stream_command(&mut state, "existing stream failure");
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
    fail_stream_command(&mut state, "existing stream failure");
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
    fail_stream_command(&mut state, "old stream failure");
    fail_record_command(&mut state, "record failure");

    state.set_stream_command_pending(output_status(false, OutputRunState::Starting));

    assert_eq!(state.last_stream_command_error, None);
    assert_eq!(
        state.last_record_command_error.as_deref(),
        Some("record failure")
    );
    assert_eq!(state.stream_status.state, OutputRunState::Starting);

    fail_stream_command(&mut state, "new stream failure");
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
    fail_stream_command(&mut state, "stream failure");
    fail_record_command(&mut state, "old record failure");

    state.set_record_command_pending(output_status(true, OutputRunState::Stopping));

    assert_eq!(
        state.last_stream_command_error.as_deref(),
        Some("stream failure")
    );
    assert_eq!(state.last_record_command_error, None);
    assert_eq!(state.record_status.state, OutputRunState::Stopping);

    fail_record_command(&mut state, "new record failure");
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
    fail_stream_command(&mut state, "stream failure");
    fail_record_command(&mut state, "record failure");

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
    fail_record_command(&mut state, "existing record failure");

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
    fail_stream_command(&mut state, "existing stream failure");

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
