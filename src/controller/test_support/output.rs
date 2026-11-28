//! `AppState`'s stream and record slots addressed by `OutputKind`.
//!
//! The reducers keep one method per output, so tests that run the same case
//! over both outputs pick the method through these helpers instead of
//! repeating the case for each output.

use crate::controller::event::OutputCommandFailureRecovery;
use crate::controller::state::AppState;
use crate::domain::output::{OutputKind, OutputStatus};

/// The output whose state a command on `kind` must leave alone.
pub(crate) fn other(kind: OutputKind) -> OutputKind {
    match kind {
        OutputKind::Stream => OutputKind::Record,
        OutputKind::Record => OutputKind::Stream,
    }
}

pub(crate) fn status(state: &AppState, kind: OutputKind) -> &OutputStatus {
    match kind {
        OutputKind::Stream => &state.stream_status,
        OutputKind::Record => &state.record_status,
    }
}

pub(crate) fn command_error(state: &AppState, kind: OutputKind) -> Option<&str> {
    match kind {
        OutputKind::Stream => state.last_stream_command_error.as_deref(),
        OutputKind::Record => state.last_record_command_error.as_deref(),
    }
}

pub(crate) fn set_status(state: &mut AppState, kind: OutputKind, status: OutputStatus) {
    match kind {
        OutputKind::Stream => state.set_stream_status(status),
        OutputKind::Record => state.set_record_status(status),
    }
}

pub(crate) fn set_pending(state: &mut AppState, kind: OutputKind, status: OutputStatus) {
    match kind {
        OutputKind::Stream => state.set_stream_command_pending(status),
        OutputKind::Record => state.set_record_command_pending(status),
    }
}

pub(crate) fn succeed(state: &mut AppState, kind: OutputKind) {
    match kind {
        OutputKind::Stream => state.set_stream_command_success(),
        OutputKind::Record => state.set_record_command_success(),
    }
}

pub(crate) fn fail_with_recovery(
    state: &mut AppState,
    kind: OutputKind,
    recovery: OutputCommandFailureRecovery,
) {
    match kind {
        OutputKind::Stream => state.set_stream_command_failure_with_recovery(recovery),
        OutputKind::Record => state.set_record_command_failure_with_recovery(recovery),
    }
}

/// Record a failed command through the real reducer, leaving the output's
/// status exactly as it is.
pub(crate) fn fail_command(state: &mut AppState, kind: OutputKind, message: &str) {
    let unchanged = status(state, kind).clone();
    fail_with_recovery(
        state,
        kind,
        OutputCommandFailureRecovery::for_test_unchecked(message.to_string(), unchanged),
    );
}
