//! Health alerts derived from observed outputs and the connection lifecycle.

use std::time::Instant;

use super::fixtures::app_state;
use crate::controller::event::OutputCommandFailureRecovery;
use crate::controller::state::{AppState, ObsStatus};
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};

#[test]
fn delayed_output_publication_uses_source_times_for_stop_correlation() {
    use crate::{controller::event::AppEvent, domain::health::AlertKind};
    let start = Instant::now() - std::time::Duration::from_secs(90);
    for (stop_second, expected_incident) in [(5, false), (32, true), (0, true)] {
        let mut state = app_state();
        state.health.connected();
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(OutputKind::Record, OutputStatus::active(), start),
            0,
        );
        // Both events are applied now, well after their source times.
        state.apply_output_event(
            &AppEvent::OutputCommandPending(
                OutputKind::Record,
                OutputStatus::new(true, OutputRunState::Stopping),
                start + std::time::Duration::from_secs(1),
            ),
            90_000,
        );
        state.apply_output_event(
            &AppEvent::OutputCommandSucceeded(OutputKind::Record),
            90_001,
        );
        assert!(state.health.alerts(Instant::now()).is_empty());
        state.apply_output_event(
            &AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus::inactive(),
                start + std::time::Duration::from_secs(stop_second),
            ),
            90_002,
        );
        assert_eq!(
            state
                .health
                .alerts(Instant::now())
                .iter()
                .any(|alert| alert.kind == AlertKind::RecordStopped),
            expected_incident,
            "correlation must use source times, including observations preceding admission"
        );
    }
}

#[test]
fn health_alerts_follow_observed_outputs_and_connection_lifecycle_not_fallbacks() {
    use crate::{
        controller::{command::AppCommand, event::AppEvent},
        domain::{health::AlertKind, obs::ConnectionInfo},
    };
    let info = || ConnectionInfo {
        connection_id: "test".into(),
        endpoint: "ws://fixture:4455".into(),
        obs_version: "fixture".into(),
        websocket_version: "fixture".into(),
        available_requests: vec![],
        supported_image_formats: vec![],
    };
    let mut state = app_state();
    state.apply_connected(info());
    let kinds = |state: &AppState| {
        state
            .health
            .alerts(Instant::now())
            .iter()
            .map(|alert| alert.kind)
            .collect::<Vec<_>>()
    };
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::active(),
            std::time::Instant::now(),
        ),
        0,
    );
    state.note_command_intent(
        &AppCommand::SetOutput {
            kind: OutputKind::Record,
            active: false,
        },
        0,
    );
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::inactive(),
            std::time::Instant::now(),
        ),
        1,
    );
    assert_eq!(
        kinds(&state),
        [AlertKind::RecordStopped],
        "a click alone does not establish command admission"
    );
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::active(),
            std::time::Instant::now(),
        ),
        2,
    );
    state.apply_output_event(
        &AppEvent::OutputCommandPending(
            OutputKind::Record,
            OutputStatus::new(true, OutputRunState::Stopping),
            std::time::Instant::now(),
        ),
        3,
    );
    state.apply_output_event(&AppEvent::OutputCommandSucceeded(OutputKind::Record), 4);
    assert!(kinds(&state).is_empty());
    assert_eq!(
        state.record_status.state,
        OutputRunState::Stopping,
        "acknowledgement alone does not observe stopped state"
    );
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::inactive(),
            std::time::Instant::now(),
        ),
        5,
    );
    assert!(kinds(&state).is_empty());
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::active(),
            std::time::Instant::now(),
        ),
        6,
    );
    state.apply_output_event(
        &AppEvent::OutputCommandFailed(
            OutputKind::Record,
            OutputCommandFailureRecovery::with_failed_command_fallback_status(
                "failed".into(),
                OutputStatus::inactive(),
            ),
        ),
        7,
    );
    assert!(
        kinds(&state).is_empty(),
        "fallback state is not observed termination"
    );
    assert_eq!(
        state.health.output_activity(OutputKind::Record),
        Some(true),
        "fallback cannot declare the output inactive for health checks"
    );
    state.health.output_unobserved(OutputKind::Record);
    assert_eq!(state.health.output_activity(OutputKind::Record), None);
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::inactive(),
            std::time::Instant::now(),
        ),
        8,
    );
    assert_eq!(kinds(&state), [AlertKind::RecordStopped]);
    state.note_command_intent(&AppCommand::Connect, 9);
    state.reset_obs_session(ObsStatus::Connecting);
    assert_eq!(
        kinds(&state),
        [AlertKind::RecordStopped],
        "reconnecting does not hide an already observed stop"
    );
    state.apply_connected(info());
    state.apply_output_event(
        &AppEvent::OutputStatusUpdated(
            OutputKind::Record,
            OutputStatus::active(),
            std::time::Instant::now(),
        ),
        10,
    );
    state.reset_obs_session(ObsStatus::Error("transport ended".into()));
    assert_eq!(kinds(&state), [AlertKind::ConnectionLost]);
    state.apply_connected(info());
    assert!(kinds(&state).is_empty());
    state.note_command_intent(&AppCommand::Disconnect, 11);
    state.reset_obs_session(ObsStatus::Disconnected);
    assert!(kinds(&state).is_empty());
}
