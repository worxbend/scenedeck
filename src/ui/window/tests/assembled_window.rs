//! The assembled main window: shell sources released on close while the
//! stats pop-out survives.

use crate::controller::test_support::state::app_state;
use crate::ui::window::*;

#[test]
#[ignore = "requires a disposable GTK display; fake config/password, no OBS"]
fn native_assembled_window_releases_shell_sources_while_stats_popout_survives() {
    crate::ui::native_test!(|| {
        use crate::controller::{
            dependencies::ControllerDependencies, test_support::deps::NoServices,
        };
        fn find(
            widget: &gtk4::Widget,
            predicate: &impl Fn(&gtk4::Widget) -> bool,
        ) -> Option<gtk4::Widget> {
            if predicate(widget) {
                return Some(widget.clone());
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                child = widget.next_sibling();
                if let Some(found) = find(&widget, predicate) {
                    return Some(found);
                }
            }
            None
        }
        let app = adw::Application::builder()
            .application_id("io.github.worxbend.scenedeck.ShellFixture")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = mpsc::sync_channel(128);
        let controller = Rc::new(RefCell::new(AppController::with_dependencies(
            runtime.handle().clone(),
            tx.clone(),
            ControllerDependencies::new(
                std::sync::Arc::new(NoServices),
                std::sync::Arc::new(NoServices),
            ),
        )));
        let weak_controller = Rc::downgrade(&controller);
        let mut initial = app_state();
        initial.config.onboarding.welcome_shown = true;
        initial
            .scene_inventory
            .scenes
            .push(crate::domain::scene::Scene {
                uuid: None,
                id: "Fixture".into(),
            });
        let state = Rc::new(RefCell::new(initial));
        let window = build_main_window(&app, state.clone(), controller.clone(), rx);
        crate::ui::test_support::pump(Duration::from_millis(120));
        let stack = find(window.upcast_ref(), &|w| {
            w.has_css_class("scenedeck-content-stack")
        })
        .unwrap()
        .downcast::<gtk4::Stack>()
        .unwrap();
        let split = find(window.upcast_ref(), &|w| w.is::<adw::NavigationSplitView>()).unwrap();
        let stats = stack.child_by_name(Page::Stats.id()).unwrap();
        stack.set_visible_child_name(Page::Stats.id());
        let popout_button = find(&stats, &|w| {
            w.downcast_ref::<gtk4::Button>()
                .is_some_and(|b| b.icon_name().as_deref() == Some("window-new-symbolic"))
        })
        .unwrap()
        .downcast::<gtk4::Button>()
        .unwrap();
        popout_button.emit_clicked();
        crate::ui::test_support::pump(Duration::from_millis(80));
        let popout = app
            .windows()
            .into_iter()
            .find(|w| w != window.upcast_ref::<gtk4::Window>())
            .expect("real Stats pop-out");
        let weak_window = window.downgrade();
        let weak_stack = stack.downgrade();
        let weak_split = split.downgrade();
        let live = stack.child_by_name(Page::Live.id()).unwrap().downgrade();
        let status = find(window.upcast_ref(), &|w| {
            w.has_css_class("scenedeck-status-bar")
        })
        .expect("production status bar")
        .downgrade();
        let sources_page = stack.child_by_name(Page::Sources.id()).unwrap().downgrade();
        drop((stack, split, stats, popout_button));
        window.close();
        let weak_state = Rc::downgrade(&state);
        drop((window, state, controller));
        crate::ui::test_support::pump(Duration::from_millis(250));
        assert!(
            popout.is_visible(),
            "closing the shell must not close Stats"
        );
        assert!(
            weak_window.upgrade().is_none(),
            "application action retained shell"
        );
        assert!(weak_stack.upgrade().is_none(), "assembled stack retained");
        assert!(
            weak_split.upgrade().is_none(),
            "split signal cycle retained shell"
        );
        assert!(live.upgrade().is_none(), "workspace source retained Live");
        assert!(
            status.upgrade().is_none(),
            "output clock retained status bar"
        );
        weak_state.upgrade().unwrap().borrow_mut().apply_connected(
            crate::controller::event::ConnectionInfo {
                connection_id: "report-fixture".into(),
                endpoint: "fixture:4455".into(),
                obs_version: "fixture".into(),
                websocket_version: "5".into(),
                available_requests: Vec::new(),
                supported_image_formats: Vec::new(),
            },
        );
        let sample = crate::domain::stats::ObsStats {
            disk_space_mb: Some(1024.0),
            cpu_usage_percent: 41.0,
            memory_usage_mb: 512.0,
            configured_fps: Some(60.0),
            active_fps: 60.0,
            average_frame_render_time_ms: 2.0,
            render_skipped_frames: 0,
            render_total_frames: 60,
            output_skipped_frames: 0,
            output_total_frames: 60,
        };
        for cpu in [41.0, 42.0] {
            tx.try_send(AppEvent::SessionEvent {
                generation: 0,
                collection: Some(0),
                event: Box::new(AppEvent::Observation {
                    revision: 1,
                    event: Box::new(AppEvent::StatsUpdated {
                        observed_at: std::time::Instant::now(),
                        stats: crate::domain::stats::ObsStats {
                            cpu_usage_percent: cpu,
                            ..sample
                        },
                        bitrate_kbps: None,
                        stream: None,
                    }),
                }),
            })
            .expect("surviving Stats must keep the application receiver alive");
            crate::ui::test_support::pump(Duration::from_millis(80));
            let expected = fl!(
                LANGUAGE_LOADER,
                "stats-value-percent",
                value = format!("{cpu:.1}")
            );
            assert!(
                find(popout.upcast_ref(), &|w| w
                    .downcast_ref::<gtk4::Label>()
                    .is_some_and(|label| label.text() == expected))
                .is_some(),
                "post-shell sample was not rendered in Stats"
            );
        }
        tx.try_send(AppEvent::SessionEvent {
            generation: 1,
            collection: None,
            event: Box::new(AppEvent::StatsUpdated {
                observed_at: std::time::Instant::now(),
                stats: sample,
                bitrate_kbps: None,
                stream: None,
            }),
        })
        .unwrap();
        crate::ui::test_support::pump(Duration::from_millis(80));
        assert_eq!(
            weak_state.upgrade().unwrap().borrow().stats_history.len(),
            2,
            "closed-shell reconciliation must still reject replaced-session publications"
        );
        assert_eq!(weak_state.upgrade().unwrap().borrow().reports.entries.back().unwrap().samples_seen, 2, "reports collect accepted samples after shell closure and reject replaced-session measurements");
        // Monitoring and its warning surface survive without a shell or an active Sources page.
        {
            let state = weak_state.upgrade().unwrap();
            let mut state = state.borrow_mut();
            let fixture = crate::ui::action_catalog::tests::connected();
            state.obs_status = fixture.obs_status;
            state.scene_collections = fixture.scene_collections;
            state.connected_connection_id = Some("audio-fixture".into());
            state.connected_endpoint = Some("ws://fixture:4455".into());
            state.show_registry.target = state.desired_registry_target().unwrap();
            state.show_registry.status = crate::controller::state::RegistryStatus::Ready;
            state.workspace.audio = fixture.workspace.audio;
            state.workspace.input_catalog = fixture.workspace.input_catalog;
            state.workspace.observed_at = Some(Instant::now());
            state.workspace.dirty = false;
            state.registry.audio_alerts = vec![crate::domain::health::AudioRule {
                target: crate::domain::operator_action::ActionTarget {
                    name: "Microphone".into(),
                    uuid: Some("input-1".into()),
                },
                enabled: true,
                silence: true,
                clipping: true,
            }];
            assert!(state.observe_audio_health(Instant::now() - Duration::from_secs(2)));
        }
        for observed_at in [Instant::now() - Duration::from_secs(1), Instant::now()] {
            tx.try_send(AppEvent::InputLevelsUpdated {
                levels: vec![crate::domain::meter::InputLevels::from_mul(
                    "Microphone".into(),
                    &[[1.0; 3]],
                )],
                observed_at,
            })
            .unwrap();
            crate::ui::test_support::pump(Duration::from_millis(100));
        }
        crate::ui::test_support::pump(Duration::from_millis(300));
        let health = find(popout.upcast_ref(), &|w| w.widget_name() == "health-alerts").unwrap();
        assert!(
            health.is_visible(),
            "detached Stats must display new health incidents"
        );
        let expected = fl!(
            LANGUAGE_LOADER,
            "health-audio-clipping",
            input = "Microphone"
        );
        assert!(find(&health, &|w| w
            .downcast_ref::<gtk4::Label>()
            .is_some_and(|label| label.text().contains(&expected)))
        .is_some());
        drop(health);
        tx.try_send(AppEvent::Disconnected).unwrap();
        crate::ui::test_support::pump(Duration::from_millis(80));
        assert!(matches!(
            weak_state.upgrade().unwrap().borrow().obs_status,
            ObsStatus::Disconnected
        ));
        assert!(weak_state.upgrade().unwrap().borrow().obs_stats.is_none());
        assert!(
            weak_state
                .upgrade()
                .unwrap()
                .borrow()
                .reports
                .entries
                .back()
                .unwrap()
                .ended_unix_ms
                .is_some(),
            "disconnect finalizes the report with only detached Stats open"
        );
        let weak_popout = popout.downgrade();
        let weak_popout_content = popout
            .clone()
            .downcast::<adw::Window>()
            .unwrap()
            .content()
            .unwrap()
            .downgrade();
        popout.close();
        drop(popout);
        // Accepted saves may retain navigation until GTK publishes their completion.
        let drained = Rc::new(std::cell::Cell::new(false));
        crate::ui::background_io::flush_pending(Duration::from_secs(10), {
            let drained = drained.clone();
            move |outcome| {
                outcome.expect("accepted saves must drain before checking controller lifetime");
                drained.set(true);
            }
        });
        let deadline = Instant::now() + Duration::from_secs(15);
        while !drained.get() {
            assert!(
                Instant::now() < deadline,
                "accepted-save publication deadline"
            );
            glib::MainContext::default().iteration(false);
            std::thread::yield_now();
        }
        assert!(
            sources_page.upgrade().is_none(),
            "Sources page retained shell"
        );
        assert!(
            weak_popout.upgrade().is_none(),
            "closed Stats window retained"
        );
        assert!(
            weak_popout_content.upgrade().is_none(),
            "closed Stats content retained"
        );
        // Sources' media poll owns navigation, but only weakly reaches its root.
        // Its one-second terminal tick is independent of the save barrier above.
        // Require the root to be gone first, then give that source a bounded
        // chance to return Break. A real navigation/widget cycle still fails.
        let source_deadline = Instant::now() + Duration::from_secs(2);
        while weak_controller.strong_count() != 0 && Instant::now() < source_deadline {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(weak_controller.upgrade().is_none(), "closed shell/pop-out retained controller after saves and terminal source ticks drained");
        assert!(
            weak_state.upgrade().is_none(),
            "application pump retained state after last window closed"
        );
        assert!(
            matches!(
                tx.try_send(AppEvent::GraphInvalidated),
                Err(mpsc::TrySendError::Disconnected(_))
            ),
            "last window removal must retire the application event receiver"
        );
    });
}
