//! Native compact-layout regression using the full application shell and no OBS client.
#[cfg(test)]
use super::*;
use crate::{
    controller::{app_controller::AppController, state::AppState},
    domain::{
        appearance::Language,
        audio::AudioInput,
        control::{ControlSnapshot, StudioState},
        role::SceneRole,
        scene::Scene,
    },
    storage::{config::AppConfig, registry::SceneRegistry},
};
use std::{cell::RefCell, rc::Rc, sync::mpsc, time::Instant};

fn pump() {
    let context = glib::MainContext::default();
    let end = Instant::now() + Duration::from_millis(250);
    while Instant::now() < end {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn find(root: &gtk4::Widget, matches: &impl Fn(&gtk4::Widget) -> bool) -> Option<gtk4::Widget> {
    if matches(root) {
        return Some(root.clone());
    }
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Some(found) = find(&widget, matches) {
            return Some(found);
        }
        child = widget.next_sibling();
    }
    None
}

#[test]
#[ignore = "requires a native GTK display; run with --test-threads=1"]
fn native_compact_large_text_outputs_and_hidden_pages_fit() {
    const CHILD: &str = "SCENEDECK_NATIVE_COMPACT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ui::widgets::transport_bar::native_layout_tests::native_compact_large_text_outputs_and_hidden_pages_fit",
                "--ignored",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "compact native child failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    crate::ui::test_support::require_native_runner();
    adw::init().expect("native GTK display");
    // Labels may contain translated punctuation or user-visible literal markup.
    // The visual markup must not change the text or become executable styling.
    let escaped = Button::with_label("");
    output_text(
        &escaped,
        "Record <&>",
        "Paused > &",
        "Record <&>: Paused > &",
    );
    assert_eq!(
        escaped
            .child()
            .and_downcast::<gtk4::Label>()
            .unwrap()
            .text(),
        "Record <&>\nPaused > &"
    );
    crate::infra::i18n::init(Language::Uk);
    gtk4::Settings::default()
        .unwrap()
        .set_gtk_font_name(Some("Sans 20"));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let (tx, events) = mpsc::sync_channel(64);
    let fixture_events = tx.clone();
    let mut config = AppConfig::default();
    config.onboarding.welcome_shown = true;
    config.live.scene_previews = true;
    let mut registry = SceneRegistry::default();
    registry.set_scene_role("Intro", Some(SceneRole::Primary));
    const LONG_SCENE: &str = "Long translated primary scene — presentation and interview source review Довга назва для перевірки";
    registry.set_scene_role(LONG_SCENE, Some(SceneRole::Primary));
    let state = Rc::new(RefCell::new(AppState::new(config, registry, None, None)));
    {
        let mut state = state.borrow_mut();
        state.obs_status = ObsStatus::Connected {
            obs_version: "synthetic fixture".into(),
        };
        state
            .scene_inventory
            .scenes
            .push(Scene { id: "Intro".into() });
        state.scene_inventory.scenes.push(Scene {
            id: LONG_SCENE.into(),
        });
        state.scene_inventory.current_id = Some("Intro".into());
        state.scene_collections.current = Some("Synthetic collection".into());
        state.available_requests = vec!["StartStream".into(), "StartRecord".into()];
        state.workspace.observed_at = Some(Instant::now());
        state.workspace.snapshot = Some(ControlSnapshot {
            studio: StudioState {
                enabled: Some(true),
                transitions: vec!["Cut".into(), "Fade".into()],
                transition: Some("Fade".into()),
                transition_duration_ms: Some(350),
                ..Default::default()
            },
            replay_active: Some(false),
            virtual_camera_active: Some(false),
            ..Default::default()
        });
    }
    let controller = Rc::new(RefCell::new(AppController::new(
        runtime.handle().clone(),
        tx,
    )));
    let app = adw::Application::builder()
        .application_id("org.scenedeck.NativeCompactRegression")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(None::<&gtk4::gio::Cancellable>).unwrap();
    let inventory = state.borrow().scene_inventory.clone();
    let window = crate::ui::window::build_main_window(&app, state.clone(), controller, events);
    fixture_events
        .send(crate::controller::event::AppEvent::SceneInventoryUpdated(
            inventory.clone(),
        ))
        .unwrap();
    fixture_events
        .send(crate::controller::event::AppEvent::ActiveAudioObserved {
            scene: inventory.current_id.clone(),
            result: Ok(vec![AudioInput::new(
                "Synthetic audio input".into(),
                true,
                1.0,
                0.0,
            )]),
        })
        .unwrap();
    window.set_default_size(640, 600);
    pump();
    let content = find(window.upcast_ref(), &|w| {
        w.has_css_class("scenedeck-content-stack")
    })
    .unwrap()
    .downcast::<gtk4::Stack>()
    .unwrap();
    let live = find(window.upcast_ref(), &|w| w.has_css_class("live-page"))
        .unwrap()
        .downcast::<gtk4::Stack>()
        .unwrap();
    live.set_visible_child_name("live");
    for stack in [&content, &live] {
        let large = gtk4::Label::builder()
            .width_request(2500)
            .height_request(1400)
            .build();
        stack.add_named(&large, Some("oversized-hidden-regression-page"));
    }
    pump();
    assert!(
        window.width() <= 640,
        "hidden content must not enlarge the requested window: {}",
        window.width()
    );
    for stack in [&content, &live] {
        assert!(
            stack.measure(Orientation::Horizontal, -1).0 <= 640,
            "an invisible page must not dictate the visible page minimum"
        );
    }
    let positions: Vec<_> = ["stream", "record", "replay", "camera"]
        .map(|name| {
            let button = find(window.upcast_ref(), &|w| {
                w.widget_name() == format!("output-{name}")
            })
            .unwrap();
            let bounds = button.compute_bounds(&window).unwrap();
            assert!(bounds.x() >= 0.0 && bounds.x() + bounds.width() <= 640.0);
            (bounds.x().round() as i32, bounds.y().round() as i32)
        })
        .to_vec();
    let columns: std::collections::BTreeSet<_> = positions.iter().map(|p| p.0).collect();
    let rows: std::collections::BTreeSet<_> = positions.iter().map(|p| p.1).collect();
    assert!(
        columns.len() >= 2,
        "large translated outputs need two columns: {positions:?}"
    );
    assert!(
        rows.len() <= 2,
        "outputs must leave room for the scene grid: {positions:?}"
    );
    let stream = find(window.upcast_ref(), &|w| w.widget_name() == "output-stream").unwrap();
    let record = find(window.upcast_ref(), &|w| w.widget_name() == "output-record").unwrap();
    assert!(stream.grab_focus());
    assert!(window.child_focus(gtk4::DirectionType::TabForward));
    assert_eq!(gtk4::prelude::RootExt::focus(&window), Some(record),
        "Tab must move directly between output buttons, without a second stop on each FlowBox wrapper");
    for previews in [true, false] {
        if !previews {
            state.borrow_mut().config.live.scene_previews = false;
            fixture_events
                .send(crate::controller::event::AppEvent::SceneInventoryUpdated(
                    inventory.clone(),
                ))
                .unwrap();
            pump();
        }
        // A rebuilt card is allocated on a later frame; wait for it rather
        // than trusting a fixed pump.
        let deadline = Instant::now() + Duration::from_secs(5);
        while find(live.upcast_ref(), &|w| {
            w.has_css_class("scene-card") && w.width() > 4 && w.height() > 4
        })
        .is_none()
            && Instant::now() < deadline
        {
            pump();
        }
        let scroller = find(live.upcast_ref(), &|w| {
            w.is::<gtk4::ScrolledWindow>() && w.is_mapped()
        })
        .unwrap();
        let card = find(live.upcast_ref(), &|w| w.has_css_class("scene-card")).unwrap();
        let title = find(&card, &|w| w.has_css_class("scene-card-title")).unwrap();
        let card_bounds = card.compute_bounds(&scroller).unwrap();
        let title_bounds = title.compute_bounds(&scroller).unwrap();
        assert!(card_bounds.y() >= 0.0 && card_bounds.y() + card_bounds.height() <= scroller.height() as f32,
            "card must fit its short viewport (previews={previews}): {card_bounds:?}; viewport height={}", scroller.height());
        assert!(title_bounds.y() >= 0.0 && title_bounds.y() + title_bounds.height() <= scroller.height() as f32,
            "title must be visible without scrolling (previews={previews}): {title_bounds:?}; viewport height={}", scroller.height());
    }
    // A topbar-only page must clear the old sidebar selection. Selecting Live
    // again should navigate, even when it was the last sidebar page visited.
    let sources = find(window.upcast_ref(), &|w| {
        w.is::<Button>()
            && w.tooltip_text().is_some_and(|t| {
                t == i18n_embed_fl::fl!(crate::infra::i18n::LANGUAGE_LOADER, "sources-title")
            })
    })
    .unwrap()
    .downcast::<Button>()
    .unwrap();
    sources.emit_clicked();
    pump();
    assert_eq!(content.visible_child_name().as_deref(), Some("sources"));
    let sidebar = find(window.upcast_ref(), &|w| {
        w.has_css_class("scenedeck-sidebar-list")
    })
    .unwrap()
    .downcast::<gtk4::ListBox>()
    .unwrap();
    assert!(
        sidebar.selected_row().is_none(),
        "Sources has no selected sidebar row"
    );
    sidebar.select_row(sidebar.row_at_index(0).as_ref());
    pump();
    assert_eq!(content.visible_child_name().as_deref(), Some("live"));
    window.close();
}
