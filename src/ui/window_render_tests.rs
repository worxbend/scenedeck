//! Mapped-window checks: widget identity alone does not detect layout flashes.
use super::*;
use crate::domain::{audio::AudioInput, role::SceneRole, scene::Scene};
use crate::ui::test_support::{native_test, navigation, pump};

fn capture(window: &gtk4::Window, name: &str) {
    let Some(directory) = std::env::var_os("SCENEDECK_UI_CAPTURE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let snapshot = gtk4::Snapshot::new();
    gtk4::WidgetPaintable::new(Some(window)).snapshot(
        &snapshot,
        window.width().into(),
        window.height().into(),
    );
    let node = snapshot.to_node().expect("mapped window");
    window
        .renderer()
        .unwrap()
        .render_texture(
            &node,
            Some(&gtk4::graphene::Rect::new(
                0.0,
                0.0,
                window.width() as f32,
                window.height() as f32,
            )),
        )
        .save_to_png(directory.join(format!("{name}.png")))
        .unwrap();
}

#[test]
#[ignore = "requires a disposable GTK display; real toggle timers and animations"]
fn native_global_toggles_only_show_their_own_pending_state() {
    native_test(
        "ui::window::render_tests::native_global_toggles_only_show_their_own_pending_state",
        || {
            use crate::domain::control::{ControlAction, ControlSnapshot, GlobalToggle};
            let (nav, _stack, _runtime, _events) = navigation();
            gtk4::Settings::default()
                .unwrap()
                .set_gtk_enable_animations(true);
            assert!(ThemeManager::apply(&Default::default()).is_ok());
            let mut snapshot = ControlSnapshot::default();
            snapshot.studio.enabled = Some(false);
            snapshot.virtual_camera_active = Some(false);
            {
                let mut state = nav.state.borrow_mut();
                state.obs_status = ObsStatus::Connected {
                    obs_version: "fixture".into(),
                };
                state.scene_collections.current = Some("Fixture".into());
                state.available_requests =
                    ["SetStudioModeEnabled", "StartVirtualCam", "StopVirtualCam"]
                        .map(str::to_owned)
                        .to_vec();
                state.workspace.snapshot = Some(snapshot.clone());
                state.workspace.observed_at = Some(Instant::now());
                state.workspace.observe_global_controls();
            }
            let root = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
            let studio = crate::ui::widgets::studio_controls::toggle(nav.clone());
            let switch = studio.last_child().and_downcast::<gtk4::Switch>().unwrap();
            root.append(&studio);
            let transport = crate::ui::widgets::transport_bar::build(nav.clone());
            root.append(&transport);
            fn find_camera(widget: &gtk4::Widget) -> Option<gtk4::Button> {
                if widget.widget_name() == "output-camera" {
                    return widget.clone().downcast().ok();
                }
                let mut child = widget.first_child();
                while let Some(widget) = child {
                    if let Some(button) = find_camera(&widget) {
                        return Some(button);
                    }
                    child = widget.next_sibling();
                }
                None
            }
            let camera = find_camera(&transport).unwrap();
            let window = gtk4::Window::builder()
                .default_width(640)
                .default_height(220)
                .child(&root)
                .build();
            window.add_css_class("scenedeck-root");
            window.present();
            pump(Duration::from_millis(300));
            assert!(switch.is_sensitive() && camera.is_sensitive());
            let sensitivity_changes = Rc::new(std::cell::Cell::new(0));
            for widget in [
                switch.clone().upcast::<gtk4::Widget>(),
                camera.clone().upcast(),
            ] {
                let changes = sensitivity_changes.clone();
                widget.connect_sensitive_notify(move |_| changes.set(changes.get() + 1));
            }
            capture(&window, "global-controls-before");
            crate::ui::workspace::act(
                &nav,
                ControlAction::ActivateScene {
                    scene: "Other".into(),
                    studio_enabled: false,
                },
            );
            let sequence = nav.state.borrow().workspace.sequence;
            apply_event(&nav, AppEvent::WorkspaceValuesChanged, None);
            pump(Duration::from_millis(350));
            capture(&window, "global-controls-unrelated-action");
            assert_eq!(
                sensitivity_changes.get(),
                0,
                "unrelated activity toggled Studio/Camera sensitivity"
            );
            assert!(
                !camera.has_css_class("output-pending"),
                "an unrelated action applied Camera's pending border"
            );
            assert!(!switch.is_active() && !switch.state());

            // A click during an unrelated write is retained, not silently ignored
            // and not allowed to supersede the admitted command.
            camera.emit_clicked();
            pump(Duration::from_millis(280));
            assert_eq!(nav.state.borrow().workspace.sequence, sequence);
            assert_eq!(nav.state.borrow().workspace.queued_toggles.len(), 1);
            assert!(switch.is_sensitive());
            assert!(!camera.is_sensitive());
            assert!(camera.has_css_class("output-pending"));
            assert!(switch.activate());
            pump(Duration::from_millis(350));
            assert_eq!(nav.state.borrow().workspace.queued_toggles.len(), 2);
            assert!(
                switch.is_active(),
                "the requested Studio thumb bounced back during its pending read"
            );

            let complete = |snapshot: ControlSnapshot| {
                let state = nav.state.borrow();
                let event = AppEvent::WorkspaceLoaded {
                    epoch: state.workspace.epoch,
                    sequence: state.workspace.sequence,
                    result_id: None,
                    acknowledged: true,
                    uncertain: false,
                    action: true,
                    result: Ok(Box::new(
                        crate::controller::workspace::WorkspaceObservation {
                            snapshot,
                            detail: None,
                            usage: vec![],
                            audio: vec![],
                            stream_status: None,
                            record_status: None,
                            screenshot: None,
                            warnings: vec![],
                            observed_at: Instant::now(),
                        },
                    )),
                };
                drop(state);
                nav.state.borrow_mut().workspace.dirty = false;
                apply_event(&nav, event, None);
                pump(Duration::from_millis(280));
            };
            complete(snapshot.clone());
            assert!(matches!(
                nav.state.borrow().workspace.active_action,
                Some(ControlAction::SetVirtualCamera(true))
            ));
            snapshot.virtual_camera_active = Some(true);
            complete(snapshot.clone());
            assert!(matches!(
                nav.state.borrow().workspace.active_action,
                Some(ControlAction::SetStudioMode(true))
            ));
            assert!(camera.is_sensitive() && camera.has_css_class("output-active"));
            assert!(!camera.has_css_class("output-pending"));
            snapshot.studio.enabled = Some(true);
            complete(snapshot);
            assert!(switch.is_sensitive() && switch.is_active() && switch.state());
            assert!(!nav
                .state
                .borrow()
                .workspace
                .toggle_pending(GlobalToggle::StudioMode));
            assert!(nav.state.borrow().workspace.queued_toggles.is_empty());
            capture(&window, "global-controls-complete");
            window.destroy();
        },
    );
}

#[test]
#[ignore = "requires a disposable GTK display; rendered synthetic OBS sequence"]
fn native_scene_readback_retains_audio_and_page_geometry() {
    native_test(
        "ui::window::render_tests::native_scene_readback_retains_audio_and_page_geometry",
        || {
            let (nav, stack, _runtime, _events) = navigation();
            gtk4::Settings::default()
                .unwrap()
                .set_gtk_enable_animations(true);
            assert!(ThemeManager::apply(&Default::default()).is_ok());
            let inputs = vec![
                AudioInput::new("Microphone".into(), false, 1.0, 0.0),
                AudioInput::new("Music".into(), false, 0.5, -6.0),
            ];
            {
                let mut state = nav.state.borrow_mut();
                state.obs_status = ObsStatus::Connected {
                    obs_version: "fixture".into(),
                };
                state.scene_collections.current = Some("Fixture".into());
                state.scene_inventory.scenes =
                    vec![Scene { id: "Main".into() }, Scene { id: "Other".into() }];
                state.scene_inventory.current_id = Some("Main".into());
                for scene in ["Main", "Other"] {
                    state
                        .registry
                        .set_scene_role(scene, Some(SceneRole::Primary));
                }
                state.workspace.snapshot = Some(Default::default());
                state.workspace.snapshot.as_mut().unwrap().studio.enabled = Some(false);
                state.workspace.observed_at = Some(Instant::now());
                state.workspace.observe_scene_controls();
                state.config.live.scene_previews = true;
                state.apply_active_audio(Some("Main".into()), Ok(inputs.clone()));
            }
            let mut sources = WindowSources::default();
            let (live, refreshers) = add_pages(&stack, &nav, &mut sources);
            let (_, _, sidebar_controls) = sidebar::build(&nav);
            let ui = EventUiContext {
                live: live.clone(),
                refreshers,
                sidebar_controls,
                header_selectors: HeaderSelectors::build(&nav),
                toast: adw::ToastOverlay::new(),
                streaming_chrome: Rc::new(RefCell::new(None)),
                status_bar: status_bar::build(),
            };
            let inventory = nav.state.borrow().scene_inventory.clone();
            crate::ui::pages::live::rebuild_scene_cards(&live, &inventory, &nav);
            crate::ui::pages::live::rebuild_audio_cards(&live, &inputs, &nav);
            crate::ui::pages::live::show_live_view(&live);
            stack.set_visible_child_name(Page::Live.id());
            let window = gtk4::Window::builder()
                .default_width(1100)
                .default_height(740)
                .child(&stack)
                .build();
            window.add_css_class("scenedeck-root");
            window.present();
            pump(Duration::from_millis(300));
            let audio = live.audio_cards.borrow()[0].root.clone();
            let scene = live.scene_cards.borrow()[0].root.clone();
            let bounds = live.scene_scroll.compute_bounds(&window).unwrap();
            capture(&window, "live-before");
            // Real transitions also emit value notifications. These must not
            // retire Studio mode and dim every card during the follow-up read.
            apply_event(&nav, AppEvent::WorkspaceValuesChanged, Some(&ui));
            apply_event(
                &nav,
                AppEvent::CurrentSceneChanged("Other".into()),
                Some(&ui),
            );
            apply_event(
                &nav,
                AppEvent::ActiveAudioLoading {
                    scene: Some("Other".into()),
                },
                Some(&ui),
            );
            pump(Duration::from_millis(80));
            capture(&window, "live-reading");
            assert!(
                audio.is_mapped(),
                "scene switching removed the entire audio strip while waiting for OBS"
            );
            assert!(
                audio.is_sensitive(),
                "same-collection readback dimmed a still-valid input"
            );
            assert_eq!(live.scene_cards.borrow()[0].root, scene);
            assert!(
                scene.is_sensitive(),
                "value readback disabled unrelated scene controls"
            );
            assert_eq!(
                live.scene_scroll.compute_bounds(&window).unwrap(),
                bounds,
                "audio loading shifted the scene wall"
            );
            apply_event(
                &nav,
                AppEvent::ActiveAudioObserved {
                    scene: Some("Other".into()),
                    result: Ok(inputs),
                },
                Some(&ui),
            );
            pump(Duration::from_millis(80));
            capture(&window, "live-after");
            assert_eq!(
                live.audio_cards.borrow()[0].root,
                audio,
                "same input was rebuilt after scene readback"
            );
            assert_eq!(live.scene_scroll.compute_bounds(&window).unwrap(), bounds);
            // Exercise both value echoes and the actual fader callback near
            // FlowBox wrap boundaries, with GTK animations running.
            fn fader(widget: &gtk4::Widget) -> Option<gtk4::Scale> {
                if let Some(scale) = widget.downcast_ref::<gtk4::Scale>() {
                    return Some(scale.clone());
                }
                let mut child = widget.first_child();
                while let Some(widget) = child {
                    if let Some(scale) = fader(&widget) {
                        return Some(scale);
                    }
                    child = widget.next_sibling();
                }
                None
            }
            let scale = fader(audio.upcast_ref()).unwrap();
            for width in [640, 760, 1100] {
                window.set_default_size(width, 740);
                pump(Duration::from_millis(100));
                let bounds = live.scene_scroll.compute_bounds(&window).unwrap();
                for db in [-100.0, -60.0, -10.0, -6.0, 0.0] {
                    scale.set_value(db);
                    let mul = 10.0_f64.powf(db / 20.0);
                    apply_event(
                        &nav,
                        AppEvent::InputVolumeChanged {
                            input: "Microphone".into(),
                            volume_mul: mul,
                            volume_db: db,
                        },
                        Some(&ui),
                    );
                    apply_event(
                        &nav,
                        AppEvent::InputMuteChanged {
                            input: "Music".into(),
                            muted: db < -10.0,
                        },
                        Some(&ui),
                    );
                    pump(Duration::from_millis(40));
                    assert_eq!(
                        live.scene_scroll.compute_bounds(&window).unwrap(),
                        bounds,
                        "gain {db} reflowed the page at width {width}"
                    );
                    assert_eq!(live.audio_cards.borrow()[0].root, audio);
                }
            }
            capture(&window, "live-volume");
            window.destroy();
        },
    );
}
