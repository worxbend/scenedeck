//! Output events: readback failures and stop evidence against newer status.

use crate::ui::window::*;

#[test]
#[ignore = "requires isolated GTK navigation and synthetic output observations"]
fn native_output_read_failure_cannot_replace_newer_evidence_or_invent_a_stop() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        nav.state.borrow_mut().health.connected();
        let start = Instant::now() - Duration::from_secs(60);
        apply_event(
            &nav,
            AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus::active(),
                start + Duration::from_secs(20),
            ),
            None,
        );
        apply_event(
            &nav,
            AppEvent::RecordingStopObserved {
                path: "closed.mkv".into(),
                command_at: start,
            },
            None,
        );
        assert!(
            nav.state.borrow().record_status.active,
            "completed stop evidence does not replace current state"
        );
        assert_eq!(
            nav.state.borrow().last_recording_path.as_deref(),
            Some("closed.mkv")
        );
        for second in [10, 30] {
            apply_event(
                &nav,
                AppEvent::OutputReadbackFailed {
                    kind: OutputKind::Record,
                    error: crate::infra::error::AppError::Request("fixture unavailable".into()),
                    observed_at: start + Duration::from_secs(second),
                },
                None,
            );
            assert_eq!(
                nav.state
                    .borrow()
                    .health
                    .output_activity(OutputKind::Record),
                if second == 10 { Some(true) } else { None }
            );
            assert!(nav.state.borrow().health.alerts(Instant::now()).is_empty());
        }
        apply_event(
            &nav,
            AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus::inactive(),
                start + Duration::from_secs(25),
            ),
            None,
        );
        assert_eq!(
            nav.state.borrow().record_status.state,
            crate::domain::output::OutputRunState::Unknown
        );
        assert!(nav.state.borrow().health.alerts(Instant::now()).is_empty());
        apply_event(
            &nav,
            AppEvent::OutputStatusUpdated(
                OutputKind::Record,
                OutputStatus::inactive(),
                start + Duration::from_secs(40),
            ),
            None,
        );
        assert_eq!(
            nav.state.borrow().health.alerts(Instant::now())[0].kind,
            crate::domain::health::AlertKind::RecordStopped
        );
    });
}
