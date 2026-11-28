//! Page redraws: audio events, background reads and refreshes must not
//! blink pages or drop live previews.

use std::sync::Arc;

use crate::controller::test_support::deps::NoPassword;
use crate::domain::audio::AudioInput;
use crate::ui::window::*;

#[test]
#[ignore = "requires a disposable GTK display; synthetic OBS events"]
fn native_audio_events_and_background_reads_do_not_blink_pages() {
    crate::ui::native_test!(|| {
        let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.scene_collections.current = Some("Fixture".into());
            state.scene_inventory.scenes = vec![crate::domain::scene::Scene {
                uuid: None,
                id: "Main".into(),
            }];
            state.scene_inventory.current_id = Some("Main".into());
            state
                .registry
                .set_scene_role("Main", Some(crate::domain::role::SceneRole::Primary));
            state.workspace.observed_at = Some(Instant::now());
            state.workspace.snapshot = Some(Default::default());
            state.workspace.snapshot.as_mut().unwrap().studio.enabled = Some(false);
            state.set_topology_ready(Default::default());
            state.apply_active_audio(
                Some("Main".into()),
                Ok(vec![AudioInput::new("Mic".into(), false, 1.0, 0.0)]),
            );
            let target = crate::domain::operator_action::ActionTarget {
                name: "Mic".into(),
                uuid: None,
            };
            state.workspace.input_catalog =
                Some(Ok(vec![crate::domain::operator_input::InputIdentity {
                    target: target.clone(),
                    kind: "audio".into(),
                }]));
            state.workspace.audio = vec![AudioInput::new("Mic".into(), false, 1.0, 0.0)];
            state.registry.quick_actions = vec![crate::domain::operator_action::OperatorPin {
                action: crate::domain::operator_action::OperatorAction::ToggleMuted { target },
                enabled: true,
            }];
            state.available_requests.push("SetInputMute".into());
        }
        let mut sources = WindowSources::default();
        let (live, refreshers) = add_pages(&stack, &nav, &mut sources, Arc::new(NoPassword));
        let (_, _, sidebar_controls) = sidebar::build(&nav);
        let ui = EventUiContext {
            live: live.clone(),
            toast: adw::ToastOverlay::new(),
            refreshers,
            header_selectors: HeaderSelectors::build(&nav),
            sidebar_controls,
            streaming_chrome: Rc::new(RefCell::new(None)),
            status_bar: status_bar::build(),
        };
        let inventory = nav.state.borrow().scene_inventory.clone();
        crate::ui::pages::live::rebuild_scene_cards(&live, &inventory, &nav);
        let card = live.scene_cards.borrow()[0].root.clone();
        let pages: Vec<_> = [
            Page::Graph,
            Page::Doctor,
            Page::Inventory,
            Page::Settings,
            Page::Help,
        ]
        .into_iter()
        .map(|page| {
            let widget = stack.child_by_name(page.id()).unwrap();
            let child = widget.first_child();
            (page, widget, child)
        })
        .collect();
        let revision = nav.state.borrow().workspace.revision();
        fn pin_state(root: &gtk4::Widget) -> Option<gtk4::Label> {
            if root.widget_name() == "quick-state-0" {
                return root.clone().downcast().ok();
            }
            let mut child = root.first_child();
            while let Some(widget) = child {
                child = widget.next_sibling();
                if let Some(found) = pin_state(&widget) {
                    return Some(found);
                }
            }
            None
        }
        let pin_status = pin_state(live.root.upcast_ref()).unwrap();
        for _ in 0..16 {
            apply_event(
                &nav,
                AppEvent::InputVolumeChanged {
                    input: "Mic".into(),
                    volume_mul: 0.5,
                    volume_db: -6.0,
                },
                Some(&ui),
            );
            apply_event(
                &nav,
                AppEvent::InputMuteChanged {
                    input: "Mic".into(),
                    muted: true,
                },
                Some(&ui),
            );
            assert!(
                pin_status
                    .label()
                    .contains(&fl!(LANGUAGE_LOADER, "quick-muted")),
                "OBS mute echoes update the pin without waiting for a workspace poll"
            );
            assert!(
                !nav.state.borrow().workspace.dirty,
                "complete audio echoes must not invalidate unrelated controls"
            );
            assert_eq!(nav.state.borrow().workspace.revision(), revision);
            crate::ui::pages::live::rebuild_scene_cards(&live, &inventory, &nav);
            assert_eq!(live.scene_cards.borrow()[0].root, card);
            assert!(card.is_sensitive());
            for (page, widget, child) in &pages {
                nav.state.borrow_mut().current_page = *page;
                ui.refreshers.call_if_visible(&nav, *page);
                assert_eq!(
                    widget.first_child(),
                    *child,
                    "unrelated update rebuilt {page:?}"
                );
            }
        }
        crate::ui::workspace::refresh(&nav);
        assert!(nav.state.borrow().workspace.loading);
        assert!(nav.state.borrow().workspace.can_control());
        crate::ui::pages::live::rebuild_scene_cards(&live, &inventory, &nav);
        assert!(
            card.is_sensitive(),
            "background refresh dimmed the entire scene wall"
        );
        nav.state.borrow_mut().workspace.observe_scene_controls();
        nav.dispatch(AppCommand::SwitchPrimaryScene("Main".into()));
        let first_sequence = nav.state.borrow().workspace.sequence;
        assert_eq!(
            nav.state.borrow().workspace.pending_scene.as_deref(),
            Some("Main")
        );
        crate::ui::pages::live::rebuild_scene_cards(&live, &inventory, &nav);
        assert!(
            card.is_sensitive(),
            "scene activation must not disable the scene wall"
        );
        nav.dispatch(AppCommand::SwitchPrimaryScene("Main".into()));
        let next_sequence = nav.state.borrow().workspace.sequence;
        assert!(
            next_sequence > first_sequence,
            "rapid clicks must reach the latest-wins queue"
        );
        let epoch = nav.state.borrow().workspace.epoch;
        apply_event(
            &nav,
            AppEvent::WorkspaceAcknowledged {
                epoch,
                sequence: first_sequence,
                result_id: None,
            },
            Some(&ui),
        );
        assert!(
            nav.state.borrow().workspace.pending,
            "old acknowledgement cleared the newer action"
        );
        let epoch = nav.state.borrow().workspace.epoch;
        apply_event(
            &nav,
            AppEvent::WorkspaceAcknowledged {
                epoch,
                sequence: next_sequence,
                result_id: None,
            },
            Some(&ui),
        );
        {
            // Complete the synthetic scene readback. Acknowledgement
            // alone must keep later global toggles queued.
            let mut state = nav.state.borrow_mut();
            state.workspace.observed_at = Some(Instant::now());
            state.workspace.loading = false;
            state.workspace.active_action = None;
            state.workspace.observe_global_controls();
        }
        crate::ui::workspace::act(
            &nav,
            crate::domain::control::ControlAction::SetStudioMode(true),
        );
        assert!(nav
            .state
            .borrow()
            .workspace
            .scene_control_observation
            .is_none());
        let sequence = nav.state.borrow().workspace.sequence;
        apply_event(
            &nav,
            AppEvent::WorkspaceAcknowledged {
                epoch,
                sequence,
                result_id: None,
            },
            Some(&ui),
        );
        assert!(
            nav.state.borrow().workspace.scene_action("Main").is_none(),
            "Studio mode acknowledgement is not a mode observation"
        );
        apply_event(&nav, AppEvent::WorkspaceInvalidated, Some(&ui));
        crate::ui::workspace::refresh(&nav);
        assert!(nav.state.borrow().workspace.observed_at.is_none());
        assert!(
            !nav.state.borrow().workspace.can_control(),
            "invalidated data became writable while reloading"
        );
        nav.state.borrow_mut().workspace.invalidate();
        crate::ui::pages::live::rebuild_scene_cards(&live, &inventory, &nav);
        assert!(!live.scene_cards.borrow()[0].root.is_sensitive());
    });
}

#[test]
#[ignore = "requires a disposable GTK display; no OBS connection"]
fn native_refresh_button_waits_for_new_data_before_redrawing() {
    crate::ui::native_test!(|| {
        fn find(widget: &gtk4::Widget) -> Option<gtk4::Button> {
            if let Some(button) = widget.downcast_ref::<gtk4::Button>() {
                if button.icon_name().as_deref() == Some("view-refresh-symbolic") {
                    return Some(button.clone());
                }
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                if let Some(button) = find(&widget) {
                    return Some(button);
                }
                child = widget.next_sibling();
            }
            None
        }
        let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
        nav.state.borrow_mut().current_page = Page::Graph;
        let mut sources = WindowSources::default();
        let (_live, _refreshers) = add_pages(&stack, &nav, &mut sources, Arc::new(NoPassword));
        let app = adw::Application::builder()
            .application_id("io.github.worxbend.scenedeck.RefreshFixture")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let window = adw::ApplicationWindow::builder()
            .application(&app)
            .content(&stack)
            .build();
        let header = build_content_header(
            &window,
            &nav,
            &HeaderSelectors::build(&nav),
            &Rc::new(RefCell::new(None)),
        );
        stack.set_visible_child_name(Page::Graph.id());
        window.present();
        crate::ui::test_support::pump(Duration::from_millis(100));
        let graph = stack.child_by_name(Page::Graph.id()).unwrap();
        let content = graph.first_child().unwrap();
        find(header.upcast_ref()).unwrap().emit_clicked();
        assert_eq!(
            graph.first_child(),
            Some(content),
            "Refresh rebuilt old data before any readback"
        );
        window.destroy();
    });
}

#[test]
#[ignore = "requires a disposable GTK display; synthetic state, no OBS connection"]
fn native_identical_inventory_readback_keeps_live_preview_paintables() {
    crate::ui::native_test!(|| {
        let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.config.live.scene_previews = true;
            state.obs_status = ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.connected_endpoint = Some("fixture".into());
            state.scene_collections.current = Some("Fixture".into());
            state.available_requests = vec!["GetSourceScreenshot".into()];
            state.scene_inventory.scenes = vec![crate::domain::scene::Scene {
                uuid: None,
                id: "Main".into(),
            }];
            state.scene_inventory.current_id = Some("Main".into());
            state
                .registry
                .set_scene_role("Main", Some(crate::domain::role::SceneRole::Primary));
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.snapshot = Some(Default::default());
        }
        let mut sources = WindowSources::default();
        let (live, refreshers) = add_pages(&stack, &nav, &mut sources, Arc::new(NoPassword));
        let (_, _, sidebar_controls) = sidebar::build(&nav);
        let ui = EventUiContext {
            live: live.clone(),
            toast: adw::ToastOverlay::new(),
            refreshers,
            header_selectors: HeaderSelectors::build(&nav),
            sidebar_controls,
            streaming_chrome: Rc::new(RefCell::new(None)),
            status_bar: status_bar::build(),
        };
        let inventory = nav.state.borrow().scene_inventory.clone();
        apply_event(
            &nav,
            AppEvent::SceneInventoryUpdated(inventory.clone()),
            Some(&ui),
        );
        stack.set_visible_child_name(Page::Live.id());
        let window = gtk4::Window::builder()
            .default_width(900)
            .default_height(700)
            .child(&stack)
            .build();
        crate::ui::previews::install(live.clone(), nav.clone());
        window.present();
        crate::ui::test_support::pump(Duration::from_millis(150));
        let picture = live.scene_cards.borrow()[0].picture.clone();
        let texture = gtk4::gdk::MemoryTexture::new(
            1,
            1,
            gtk4::gdk::MemoryFormat::R8g8b8a8,
            &glib::Bytes::from_owned(vec![255_u8, 255, 255, 255]),
            4,
        );
        picture.set_paintable(Some(&texture));
        let paintable = picture.paintable().unwrap();
        apply_event(&nav, AppEvent::SceneInventoryUpdated(inventory), Some(&ui));
        assert_eq!(
            picture.paintable(),
            Some(paintable),
            "unchanged inventory blanked the existing preview"
        );
        apply_event(&nav, AppEvent::WorkspaceScopeChanging, Some(&ui));
        assert!(
            picture.paintable().is_none(),
            "retired collection must still clear old previews"
        );
        window.destroy();
    });
}
