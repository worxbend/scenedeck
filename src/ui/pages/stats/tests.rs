use super::*;

#[test]
#[ignore = "requires a native GTK display"]
fn native_popout_window_does_not_own_its_strong_window_slot() {
    crate::ui::native_test!(|| {
        let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
        let weak_state = Rc::downgrade(&nav.state);
        nav.state.borrow_mut().workspace.log(
            crate::domain::workflow::ActivityOrigin::Observed,
            "Stats lifetime fixture".into(),
        );
        let window_slot = Rc::new(RefCell::new(None));
        let refresh_slot = Rc::new(RefCell::new(None));
        let button = build_popout_button(&nav, &window_slot, &refresh_slot);
        for _ in 0..3 {
            button.emit_clicked();
            let window = window_slot.borrow().as_ref().unwrap().clone();
            let weak_window = window.downgrade();
            let weak_content = window.content().unwrap().downgrade();
            refresh_slot.borrow().as_ref().unwrap()();
            button.emit_clicked();
            assert_eq!(window_slot.borrow().as_ref(), Some(&window));
            window.close();
            assert!(window_slot.borrow().is_none());
            assert!(refresh_slot.borrow().is_none());
            drop(window);
            crate::ui::test_support::pump(std::time::Duration::from_millis(50));
            assert!(
                weak_window.upgrade().is_none(),
                "closed Stats window retained"
            );
            assert!(
                weak_content.upgrade().is_none(),
                "Stats activity/chart callbacks retained content"
            );
        }
        // Also cover destruction without a close-request, with the
        // embedded page's slot/button released afterwards.
        button.emit_clicked();
        let window = window_slot.borrow().as_ref().unwrap().clone();
        let weak_window = window.downgrade();
        let weak_slot = Rc::downgrade(&window_slot);
        window.destroy();
        drop(window);
        drop(button);
        drop(window_slot);
        drop(refresh_slot);
        drop((nav, stack));
        crate::ui::test_support::pump(std::time::Duration::from_millis(50));
        assert!(weak_slot.upgrade().is_none());
        assert!(weak_window.upgrade().is_none());
        assert!(
            weak_state.upgrade().is_none(),
            "Stats callbacks retained navigation after teardown"
        );
    });
}

#[test]
#[ignore = "requires a disposable GTK display"]
fn native_popout_application_removal_releases_refresh_subscription() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let app = adw::Application::builder()
            .application_id("io.github.worxbend.scenedeck.StatsRemovalFixture")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let window_slot = Rc::new(RefCell::new(None));
        let refresh_slot = Rc::new(RefCell::new(None));
        let button = build_popout_button(&nav, &window_slot, &refresh_slot);
        let host = adw::ApplicationWindow::builder()
            .application(&app)
            .content(&button)
            .build();
        host.present();
        button.emit_clicked();
        let popout = window_slot.borrow().as_ref().unwrap().clone();
        assert!(refresh_slot.borrow().is_some());
        app.remove_window(&popout);
        assert!(window_slot.borrow().is_none());
        assert!(
            refresh_slot.borrow().is_none(),
            "removed Stats still subscribed to application telemetry"
        );
        let weak_content = popout.content().unwrap().downgrade();
        popout.destroy();
        drop(popout);
        assert!(weak_content.upgrade().is_none());
        host.close();
    });
}

fn stats_sample(configured_fps: Option<f64>) -> crate::domain::stats::ObsStats {
    crate::domain::stats::ObsStats {
        configured_fps,
        active_fps: 120.0,
        disk_space_mb: None,
        cpu_usage_percent: 1.0,
        memory_usage_mb: 100.0,
        average_frame_render_time_ms: 1.0,
        render_skipped_frames: 0,
        render_total_frames: 100,
        output_skipped_frames: 0,
        output_total_frames: 100,
    }
}

#[test]
fn configured_target_comes_from_latest_sample_not_the_history_ceiling() {
    let mut state = crate::controller::test_support::state::app_state();
    assert_eq!(fps_target(&state), None);
    for target in [
        Some(30.0),
        Some(59.94),
        Some(60.0),
        Some(120.0),
        Some(144.0),
        None,
        Some(0.0),
        Some(f64::NAN),
    ] {
        state.stats_history.push(stats_sample(target), None, None);
        let expected = target.filter(|target| target.is_finite() && *target > 0.0);
        assert_eq!(fps_target(&state), expected);
    }
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_stats_fixture_exposes_congestion_and_configured_fps_health() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let gauges = build_gauges(&nav);
        assert!(gauges.all.iter().all(|gauge| gauge
            .widget()
            .tooltip_text()
            .unwrap()
            .contains(&fl!(LANGUAGE_LOADER, "output-state-unknown"))));
        let stream = crate::domain::stats::StreamHealth {
            active: true,
            reconnecting: false,
            congestion: 0.8,
            skipped_frames: 0,
            total_frames: 100,
            bytes: 1000,
        };
        nav.state
            .borrow_mut()
            .stats_history
            .push(stats_sample(Some(120.0)), None, Some(stream));
        for gauge in &gauges.all {
            gauge.redraw();
        }
        assert!(gauges.all[0]
            .widget()
            .tooltip_text()
            .unwrap()
            .contains("120"));
        assert!(gauges.all[0]
            .widget()
            .tooltip_text()
            .unwrap()
            .contains(&fl!(LANGUAGE_LOADER, "edge-status-ok-label")));
        let congestion = gauges.all[3].widget();
        assert!(congestion
            .tooltip_text()
            .unwrap()
            .contains(&fl!(LANGUAGE_LOADER, "stats-gauge-congestion")));
        assert!(congestion.tooltip_text().unwrap().contains("80%"));
        assert!(congestion
            .tooltip_text()
            .unwrap()
            .contains(&fl!(LANGUAGE_LOADER, "diag-label-error")));
        for property in [
            gtk4::AccessibleProperty::Label,
            gtk4::AccessibleProperty::Description,
            gtk4::AccessibleProperty::ValueNow,
            gtk4::AccessibleProperty::ValueText,
        ] {
            assert!(gtk4::test_accessible_has_property(congestion, property));
        }
        nav.state.borrow_mut().stats_history.push(
            stats_sample(None),
            None,
            Some(crate::domain::stats::StreamHealth {
                congestion: 0.1,
                ..stream
            }),
        );
        for gauge in &gauges.all {
            gauge.redraw();
        }
        assert!(gauges.all[0]
            .widget()
            .tooltip_text()
            .unwrap()
            .contains(&fl!(LANGUAGE_LOADER, "output-state-unknown")));
        assert!(gauges.all[3]
            .widget()
            .tooltip_text()
            .unwrap()
            .contains("10%"));
    });
}

#[test]
fn frame_budget_follows_the_target_frame_rate() {
    assert!((frame_budget_ms(Some(60.0)).unwrap() - 16.666).abs() < 0.01);
    assert!((frame_budget_ms(Some(30.0)).unwrap() - 33.333).abs() < 0.01);
}

#[test]
fn frame_budget_is_unknown_when_the_target_is_unusable() {
    for target in [
        None,
        Some(0.0),
        Some(-30.0),
        Some(f64::NAN),
        Some(f64::INFINITY),
    ] {
        assert_eq!(frame_budget_ms(target), None);
        assert_eq!(fps_severity(30.0, target), Severity::Unknown);
        assert_eq!(
            frame_time_severity(10.0, frame_budget_ms(target)),
            Severity::Unknown
        );
    }
}

#[test]
fn fps_severity_escalates_as_the_render_falls_behind_target() {
    assert_eq!(fps_severity(60.0, Some(60.0)), Severity::Good);
    assert_eq!(fps_severity(58.0, Some(60.0)), Severity::Good);
    assert_eq!(fps_severity(55.0, Some(60.0)), Severity::Warning);
    assert_eq!(fps_severity(40.0, Some(60.0)), Severity::Critical);
}

#[test]
fn fps_severity_is_unknown_without_a_usable_target() {
    assert_eq!(fps_severity(0.0, None), Severity::Unknown);
    assert_eq!(fps_severity(f64::NAN, Some(60.0)), Severity::Unknown);
}

#[test]
fn configured_fps_health_is_independent_of_rounded_chart_scale() {
    for target in [30.0, 59.94, 60.0, 120.0, 144.0] {
        for jitter in [-0.01, 0.0, 0.01] {
            assert_eq!(fps_severity(target + jitter, Some(target)), Severity::Good);
        }
        assert_eq!(fps_severity(target * 0.90, Some(target)), Severity::Warning);
        assert_eq!(
            fps_severity(target * 0.70, Some(target)),
            Severity::Critical
        );
        assert_eq!(
            fps_severity(target * FPS_WARNING_RATIO, Some(target)),
            Severity::Good
        );
        assert_eq!(
            fps_severity(target * FPS_CRITICAL_RATIO, Some(target)),
            Severity::Warning
        );
        assert_eq!(
            frame_time_severity(0.25 * 1000.0 / target, frame_budget_ms(Some(target))),
            Severity::Good
        );
    }
    assert_eq!(chart::nice_upper_bound(60.01, FPS_BASELINE), 100.0);
    assert_eq!(fps_severity(60.01, Some(60.0)), Severity::Good);
}

#[test]
fn frame_time_severity_is_measured_against_the_frame_budget() {
    let budget = frame_budget_ms(Some(60.0));

    assert_eq!(frame_time_severity(4.0, budget), Severity::Good);
    assert_eq!(frame_time_severity(10.0, budget), Severity::Warning);
    assert_eq!(frame_time_severity(15.0, budget), Severity::Critical);
}

#[test]
fn dropped_severity_uses_the_one_and_five_percent_thresholds() {
    assert_eq!(dropped_severity(0.0), Severity::Good);
    assert_eq!(dropped_severity(0.005), Severity::Good);
    assert_eq!(dropped_severity(0.02), Severity::Warning);
    assert_eq!(dropped_severity(0.08), Severity::Critical);
}

#[test]
fn frames_are_formatted_with_their_share_of_the_total() {
    assert_eq!(format_frames(5, 1_000, 0.005), "5 of 1000 (0.50%)");
}
