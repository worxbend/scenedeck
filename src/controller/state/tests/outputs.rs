//! Stream and record output commands, status events and the recording stop
//! ticket.

use std::time::Instant;

use super::fixtures::{app_state, output_status};
use crate::controller::event::OutputCommandFailureRecovery;
use crate::controller::state::{ObsStatus, Page};
use crate::controller::test_support::output;
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

/// A case run once per output. `label` is the name the case had as its own
/// test, so a failure still points at it.
struct OutputCase {
    label: &'static str,
    kind: OutputKind,
}

const COMMAND_FAILURE_ISOLATION: [OutputCase; 2] = [
    OutputCase {
        label: "stream_command_failure_sets_stream_error_only",
        kind: OutputKind::Stream,
    },
    OutputCase {
        label: "record_command_failure_sets_record_error_only",
        kind: OutputKind::Record,
    },
];

#[test]
fn command_failure_sets_only_its_own_output_error() {
    for OutputCase { label, kind } in COMMAND_FAILURE_ISOLATION {
        let mut state = app_state();

        output::fail_command(&mut state, kind, "command failed");

        assert_eq!(
            output::command_error(&state, kind),
            Some("command failed"),
            "{label}"
        );
        assert_eq!(
            output::command_error(&state, output::other(kind)),
            None,
            "{label}"
        );
    }
}

/// A command that went pending and then failed with a recovery payload.
struct RecoveryCase {
    label: &'static str,
    kind: OutputKind,
    pending: OutputStatus,
    fallback: OutputStatus,
}

const START_AND_STOP_FAILURES: [RecoveryCase; 4] = [
    RecoveryCase {
        label: "stream_command_start_failure_applies_inactive_recovery_payload",
        kind: OutputKind::Stream,
        pending: OutputStatus::new(false, OutputRunState::Starting),
        fallback: OutputStatus::new(false, OutputRunState::Inactive),
    },
    RecoveryCase {
        label: "stream_command_stop_failure_applies_active_recovery_payload",
        kind: OutputKind::Stream,
        pending: OutputStatus::new(true, OutputRunState::Stopping),
        fallback: OutputStatus::new(true, OutputRunState::Active),
    },
    RecoveryCase {
        label: "record_command_start_failure_applies_inactive_recovery_payload",
        kind: OutputKind::Record,
        pending: OutputStatus::new(false, OutputRunState::Starting),
        fallback: OutputStatus::new(false, OutputRunState::Inactive),
    },
    RecoveryCase {
        label: "record_command_stop_failure_applies_active_recovery_payload",
        kind: OutputKind::Record,
        pending: OutputStatus::new(true, OutputRunState::Stopping),
        fallback: OutputStatus::new(true, OutputRunState::Active),
    },
];

#[test]
fn start_and_stop_failures_apply_the_recovery_payload_and_keep_the_other_error() {
    for case in START_AND_STOP_FAILURES {
        let label = case.label;
        let other = output::other(case.kind);
        let mut state = app_state();
        output::fail_command(&mut state, other, "existing failure");
        output::set_pending(&mut state, case.kind, case.pending);

        output::fail_with_recovery(
            &mut state,
            case.kind,
            command_failure_recovery("command failed", case.fallback.clone()),
        );

        let status = output::status(&state, case.kind);
        assert_eq!(status, &case.fallback, "{label}");
        assert_non_transitioning(status);
        assert_eq!(
            output::command_error(&state, case.kind),
            Some("command failed"),
            "{label}"
        );
        assert_eq!(
            output::command_error(&state, other),
            Some("existing failure"),
            "{label}"
        );
    }
}

type RecoveryConstructor = fn(&str, OutputStatus) -> OutputCommandFailureRecovery;

/// A recovery payload the reducer must apply as given, even when it is not
/// what the pending status would suggest.
struct PayloadCase {
    label: &'static str,
    kind: OutputKind,
    pending: OutputStatus,
    recovery: RecoveryConstructor,
    payload: OutputStatus,
}

const EXACT_PAYLOADS: [PayloadCase; 4] = [
    PayloadCase {
        label: "stream_failure_recovery_applies_exact_payload_status",
        kind: OutputKind::Stream,
        pending: OutputStatus::new(false, OutputRunState::Starting),
        recovery: unchecked_command_failure_recovery,
        payload: OutputStatus::new(false, OutputRunState::Reconnecting),
    },
    PayloadCase {
        label: "record_failure_recovery_applies_exact_payload_status",
        kind: OutputKind::Record,
        pending: OutputStatus::new(true, OutputRunState::Stopping),
        recovery: unchecked_command_failure_recovery,
        payload: OutputStatus::new(true, OutputRunState::Reconnecting),
    },
    PayloadCase {
        label: "stream_failure_recovery_applies_payload_instead_of_current_state",
        kind: OutputKind::Stream,
        pending: OutputStatus::new(false, OutputRunState::Starting),
        recovery: command_failure_recovery,
        payload: OutputStatus::new(true, OutputRunState::Active),
    },
    PayloadCase {
        label: "record_failure_recovery_applies_payload_instead_of_current_state",
        kind: OutputKind::Record,
        pending: OutputStatus::new(true, OutputRunState::Stopping),
        recovery: command_failure_recovery,
        payload: OutputStatus::new(false, OutputRunState::Inactive),
    },
];

#[test]
fn failure_recovery_applies_the_payload_status_exactly() {
    for case in EXACT_PAYLOADS {
        let label = case.label;
        let mut state = app_state();
        output::set_pending(&mut state, case.kind, case.pending);

        output::fail_with_recovery(
            &mut state,
            case.kind,
            (case.recovery)("command failed", case.payload.clone()),
        );

        assert_eq!(output::status(&state, case.kind), &case.payload, "{label}");
        assert_eq!(
            output::command_error(&state, case.kind),
            Some("command failed"),
            "{label}"
        );
    }
}

const RECONNECT_AFTER_RECOVERY: [OutputCase; 2] = [
    OutputCase {
        label: "separate_status_event_can_reconnect_after_stream_failure_recovery",
        kind: OutputKind::Stream,
    },
    OutputCase {
        label: "separate_status_event_can_reconnect_after_record_failure_recovery",
        kind: OutputKind::Record,
    },
];

#[test]
fn separate_status_event_can_reconnect_after_failure_recovery() {
    for OutputCase { label, kind } in RECONNECT_AFTER_RECOVERY {
        let mut state = app_state();
        output::set_pending(
            &mut state,
            kind,
            output_status(false, OutputRunState::Starting),
        );
        output::fail_with_recovery(
            &mut state,
            kind,
            command_failure_recovery(
                "command failed",
                output_status(false, OutputRunState::Inactive),
            ),
        );
        assert_non_transitioning(output::status(&state, kind));

        output::set_status(
            &mut state,
            kind,
            output_status(false, OutputRunState::Reconnecting),
        );

        assert_eq!(
            output::status(&state, kind),
            &output_status(false, OutputRunState::Reconnecting),
            "{label}"
        );
        assert_eq!(
            output::command_error(&state, kind),
            Some("command failed"),
            "{label}"
        );
    }
}

/// A command that goes pending, fails, and is then confirmed by OBS.
struct PendingCase {
    label: &'static str,
    kind: OutputKind,
    pending: OutputStatus,
}

const PENDING_AND_SUCCESS: [PendingCase; 2] = [
    PendingCase {
        label: "stream_pending_and_success_clear_only_stream_error",
        kind: OutputKind::Stream,
        pending: OutputStatus::new(false, OutputRunState::Starting),
    },
    PendingCase {
        label: "record_pending_and_success_clear_only_record_error",
        kind: OutputKind::Record,
        pending: OutputStatus::new(true, OutputRunState::Stopping),
    },
];

#[test]
fn pending_and_success_clear_only_their_own_output_error() {
    for case in PENDING_AND_SUCCESS {
        let label = case.label;
        let other = output::other(case.kind);
        let mut state = app_state();
        output::fail_command(&mut state, case.kind, "old failure");
        output::fail_command(&mut state, other, "other failure");

        output::set_pending(&mut state, case.kind, case.pending.clone());

        assert_eq!(output::command_error(&state, case.kind), None, "{label}");
        assert_eq!(
            output::command_error(&state, other),
            Some("other failure"),
            "{label}"
        );
        assert_eq!(
            output::status(&state, case.kind).state,
            case.pending.state,
            "{label}"
        );

        output::fail_command(&mut state, case.kind, "new failure");
        output::succeed(&mut state, case.kind);

        assert_eq!(output::command_error(&state, case.kind), None, "{label}");
        assert_eq!(
            output::command_error(&state, other),
            Some("other failure"),
            "{label}"
        );
    }
}

#[test]
fn output_command_errors_clear_together_on_session_reset() {
    let mut state = app_state();
    output::fail_command(&mut state, OutputKind::Stream, "stream failure");
    output::fail_command(&mut state, OutputKind::Record, "record failure");

    state.clear_output_command_errors();

    assert_eq!(state.last_stream_command_error, None);
    assert_eq!(state.last_record_command_error, None);
}

/// A failed command, its retry, and the status OBS then reports.
struct FailureSequenceCase {
    label: &'static str,
    kind: OutputKind,
    pending: OutputStatus,
    fallback: OutputStatus,
    settled: OutputStatus,
}

const FAILURE_SEQUENCES: [FailureSequenceCase; 2] = [
    FailureSequenceCase {
        label: "stream_command_failure_sequence_keeps_obs_connected_and_record_error",
        kind: OutputKind::Stream,
        pending: OutputStatus::new(false, OutputRunState::Starting),
        fallback: OutputStatus::new(false, OutputRunState::Inactive),
        settled: OutputStatus::new(true, OutputRunState::Active),
    },
    FailureSequenceCase {
        label: "record_command_failure_sequence_keeps_obs_connected_and_stream_error",
        kind: OutputKind::Record,
        pending: OutputStatus::new(true, OutputRunState::Stopping),
        fallback: OutputStatus::new(true, OutputRunState::Active),
        settled: OutputStatus::new(false, OutputRunState::Inactive),
    },
];

#[test]
fn command_failure_sequence_keeps_obs_connected_and_the_other_error() {
    let connected = ObsStatus::Connected {
        obs_version: "32.1.2".to_string(),
    };
    for case in FAILURE_SEQUENCES {
        let label = case.label;
        let other = output::other(case.kind);
        let mut state = app_state();
        state.current_page = Page::Live;
        state.set_obs_status(connected.clone());
        output::fail_command(&mut state, other, "existing failure");

        output::set_pending(&mut state, case.kind, case.pending.clone());
        assert_eq!(
            output::status(&state, case.kind).state,
            case.pending.state,
            "{label}"
        );
        assert_eq!(output::command_error(&state, case.kind), None, "{label}");

        output::fail_with_recovery(
            &mut state,
            case.kind,
            command_failure_recovery("command failed", case.fallback),
        );

        assert_eq!(state.obs_status, connected, "{label}");
        assert_eq!(state.current_page, Page::Live, "{label}");
        assert!(
            !output::status(&state, case.kind).state.is_transitioning(),
            "{label}"
        );
        assert_eq!(
            output::command_error(&state, case.kind),
            Some("command failed"),
            "{label}"
        );
        assert_eq!(
            output::command_error(&state, other),
            Some("existing failure"),
            "{label}"
        );

        output::set_pending(&mut state, case.kind, case.pending);
        assert_eq!(output::command_error(&state, case.kind), None, "{label}");
        assert_eq!(
            output::command_error(&state, other),
            Some("existing failure"),
            "{label}"
        );

        output::set_status(&mut state, case.kind, case.settled);
        output::succeed(&mut state, case.kind);

        assert_eq!(state.obs_status, connected, "{label}");
        assert_eq!(state.current_page, Page::Live, "{label}");
        assert!(
            !output::status(&state, case.kind).state.is_transitioning(),
            "{label}"
        );
        assert_eq!(output::command_error(&state, case.kind), None, "{label}");
        assert_eq!(
            output::command_error(&state, other),
            Some("existing failure"),
            "{label}"
        );
    }
}
