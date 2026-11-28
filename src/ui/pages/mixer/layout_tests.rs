use super::*;
use crate::controller::app_controller::AppController;
use crate::controller::test_support::state::{app_state, input};
use crate::domain::scene::{Scene, SceneInventory};

#[test]
#[ignore = "requires a native GTK display"]
fn native_mixer_echoes_preserve_cards_drafts_focus_and_popovers() {
    crate::ui::native_test!(|| {
        use std::time::Duration;
        let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.obs_status = crate::controller::state::ObsStatus::Connected {
                obs_version: "fixture".into(),
            };
            state.scene_collections.current = Some("Fixture".into());
            state.scene_inventory.scenes = vec![Scene {
                uuid: None,
                id: "Main".into(),
            }];
            state.scene_inventory.current_id = Some("Main".into());
            state.apply_active_audio(Some("Main".into()), Ok(vec![input("Mic"), input("Music")]));
        }
        let (page, refresh, handle) = build(nav.clone());
        stack.add_named(&page, Some("mixer-fixture"));
        let window = gtk4::Window::builder()
            .default_width(1000)
            .default_height(1100)
            .child(&stack)
            .build();
        window.present();
        crate::ui::test_support::pump(Duration::from_millis(150));
        let mut search = None;
        let mut options = None;
        visit_mixer(&page, &mut |widget| {
            if let Some(row) = widget.downcast_ref::<EntryRow>() {
                search = Some(row.clone());
            }
            if let Some(row) = widget.downcast_ref::<adw::ExpanderRow>() {
                options = Some(row.clone());
            }
        });
        let search = search.unwrap();
        let options = options.unwrap();
        options.set_expanded(true);
        search.set_text("unapplied <draft> & search");
        let first = handle.cards.borrow()["Mic"].root.clone();
        let second = handle.cards.borrow()["Music"].root.clone();
        let mut scale = None;
        visit_mixer(second.upcast_ref(), &mut |widget| {
            if let Some(fader) = widget.downcast_ref::<gtk4::Scale>() {
                scale = Some(fader.clone());
            }
        });
        let scale = scale.unwrap();
        crate::ui::test_support::pump(Duration::from_millis(80));
        assert!(scale.grab_focus());
        scale.set_value(-6.0);
        crate::ui::test_support::pump(
            crate::services::audio_service::VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30),
        );
        scale.set_value(-9.0);
        let sent = f64::from(AudioService::volume_db_to_mul(-6.0) as f32);
        let sent_db = AudioService::volume_mul_to_db(sent);
        {
            let mut state = nav.state.borrow_mut();
            state.audio_inputs[1].volume_mul = sent;
            state.audio_inputs[1].volume_db = sent_db;
        }
        handle.update_volume("Music", sent, sent_db);
        refresh();
        assert_eq!(
            scale.value(),
            -9.0,
            "a delayed same-input echo reset the newer edit"
        );
        let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
        for index in 0..128 {
            let mul = 0.1 + index as f64 / 200.0;
            let db = AudioService::volume_mul_to_db(mul);
            {
                let mut state = nav.state.borrow_mut();
                state.audio_inputs[0].muted = index % 2 == 0;
                state.audio_inputs[0].volume_mul = mul;
                state.audio_inputs[0].volume_db = db;
            }
            handle.update_mute("Mic", index % 2 == 0);
            handle.update_volume("Mic", mul, db);
            refresh();
            assert_eq!(handle.cards.borrow()["Mic"].root, first);
            assert_eq!(handle.cards.borrow()["Music"].root, second);
        }
        assert_eq!(gtk4::prelude::RootExt::focus(&window), Some(focus));
        assert_eq!(
            scale.value(),
            -9.0,
            "an unrelated echo reset the active fader"
        );
        assert_eq!(search.text(), "unapplied <draft> & search");
        let mut menu = None;
        visit_mixer(first.upcast_ref(), &mut |widget| {
            if let Some(button) = widget.downcast_ref::<gtk4::MenuButton>() {
                menu = Some(button.clone());
            }
        });
        let menu = menu.unwrap();
        menu.popup();
        crate::ui::test_support::pump(Duration::from_millis(100));
        let popover = menu.popover().unwrap();
        assert!(popover.is_mapped());
        for _ in 0..64 {
            handle.update_mute("Mic", true);
        }
        assert!(popover.is_mapped());
        assert_eq!(handle.cards.borrow().len(), 2);
        popover.popdown();
        let preferences = page.first_child().unwrap();
        nav.state.borrow_mut().invalidate_active_audio();
        refresh();
        assert_eq!(page.first_child(), Some(preferences.clone()));
        assert_eq!(handle.cards.borrow()["Mic"].root, first);
        assert!(
            first.is_sensitive(),
            "membership reads must not dim collection-scoped input controls"
        );
        nav.state
            .borrow_mut()
            .apply_active_audio(Some("Main".into()), Ok(vec![input("Mic"), input("Music")]));
        refresh();
        assert_eq!(handle.cards.borrow()["Mic"].root, first);
        assert!(first.is_sensitive());
        nav.state.borrow_mut().audio_inputs.push(input("Third"));
        refresh();
        let mut restored_draft = None;
        visit_mixer(&page, &mut |widget| {
            if let Some(row) = widget.downcast_ref::<EntryRow>() {
                restored_draft = Some(row.text().to_string());
            }
        });
        assert_eq!(
            restored_draft.as_deref(),
            Some("unapplied <draft> & search")
        );
        assert_eq!(handle.cards.borrow().len(), 3);
        assert_eq!(
            page.first_child(),
            Some(preferences),
            "membership changes replaced the entire Mixer"
        );
        assert!(
            search.parent().is_some(),
            "membership changes detached search controls"
        );
        assert!(options.is_expanded());
        let weak_scale = scale.downgrade();
        drop((scale, first, second, search, options, menu, popover));
        crate::ui::test_support::pump(Duration::from_millis(80));
        assert!(
            weak_scale.upgrade().is_none(),
            "structural replacement retained the old fader"
        );
        *handle.requested_scene.borrow_mut() = Some("Main".into());
        nav.state.borrow_mut().workspace.epoch += 1;
        refresh();
        assert!(
            handle.requested_scene.borrow().is_none(),
            "old scope suppressed a new same-name request"
        );
        let weak_page = page.downgrade();
        window.destroy();
        stack.remove(&page);
        drop((window, page, refresh, handle, nav, stack));
        crate::ui::test_support::pump(Duration::from_millis(250));
        assert!(weak_page.upgrade().is_none());
    });
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_inspected_scene_without_snapshot_issues_one_request_without_borrowing_state_twice() {
    crate::ui::native_test!(|| {
        for mode in [MixerMode::SelectedScene, MixerMode::PinnedScene] {
            let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
            {
                let mut state = nav.state.borrow_mut();
                state.obs_status = crate::controller::state::ObsStatus::Connected {
                    obs_version: "fixture".into(),
                };
                state.scene_collections.current = Some("Fixture".into());
                state.scene_inventory.scenes = vec![
                    Scene {
                        uuid: None,
                        id: "Program".into(),
                    },
                    Scene {
                        uuid: None,
                        id: "Inspected".into(),
                    },
                ];
                state.scene_inventory.current_id = Some("Program".into());
                state.mixer.mode = mode;
                state.mixer.selected_scene = Some("Inspected".into());
                state.mixer.pinned_scene = Some("Inspected".into());
            }
            let (page, refresh, handle) = build(nav.clone());
            assert_eq!(
                nav.state.borrow().mixer_audio_refresh.requested_scene(),
                Some("Inspected")
            );
            assert_eq!(
                handle.requested_scene.borrow().as_deref(),
                Some("Inspected")
            );
            refresh();
            assert_eq!(
                nav.state.borrow().mixer_audio_refresh.requested_scene(),
                Some("Inspected")
            );
            let weak_page = page.downgrade();
            drop((page, refresh, handle, nav, stack));
            crate::ui::test_support::pump(std::time::Duration::from_millis(20));
            assert!(weak_page.upgrade().is_none());
        }
    });
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_mixer_error_status_shows_obs_scene_and_message_literally() {
    crate::ui::native_test!(|| {
        const SCENE: &str = "<b>x</b> & y";
        const MESSAGE: &str = "read <i>failed</i> & gave up";
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        nav.update(|state| {
            state.scene_inventory = SceneInventory {
                scenes: vec![Scene {
                    uuid: None,
                    id: SCENE.to_string(),
                }],
                current_id: Some(SCENE.to_string()),
                previous_id: None,
            };
            state.apply_active_audio(Some(SCENE.to_string()), Err(MESSAGE.to_string()));
        })
        .expect("no other AppState borrow is live");

        let (page, _refresh, _handle) = build(nav);

        let description =
            crate::ui::test_support::find_where(page.upcast_ref::<gtk4::Widget>(), |widget| {
                widget.downcast_ref::<gtk4::Label>().is_some_and(|label| {
                    let text = label.text();
                    text.contains(SCENE) && text.contains(MESSAGE)
                })
            });
        assert!(
            description.is_some(),
            "the error description must show the OBS scene name and message as typed"
        );
    });
}

/// Build the Mixer page against a state holding `count` audio sources.
fn measure_mixer_page_min_height(count: usize) -> i32 {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    let (event_tx, _event_rx) = std::sync::mpsc::sync_channel(16);
    let controller = Rc::new(RefCell::new(AppController::new(
        runtime.handle().clone(),
        event_tx,
    )));

    let mut state = app_state();
    state.scene_inventory = SceneInventory {
        scenes: vec![Scene {
            uuid: None,
            id: "Main".to_string(),
        }],
        current_id: Some("Main".to_string()),
        previous_id: None,
    };
    let inputs = (0..count)
        .map(|index| input(&format!("Source {index}")))
        .collect();
    state.apply_active_audio(Some("Main".into()), Ok(inputs));

    let nav = NavigationContext::new(Rc::new(RefCell::new(state)), gtk4::Stack::new(), controller);
    let (widget, _refresh, _handle) = build(nav);
    widget.measure(gtk4::Orientation::Vertical, -1).0
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_mixer_page_minimum_height_does_not_grow_with_the_card_count() {
    crate::ui::native_test!(|| {
        let one = measure_mixer_page_min_height(1);
        let many = measure_mixer_page_min_height(24);

        println!("min height: 1 source = {one}px, 24 sources = {many}px");
        assert_eq!(
            one, many,
            "the page must scroll its cards instead of demanding room for them"
        );
    });
}
