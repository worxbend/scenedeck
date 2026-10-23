//! Native scheduler invariants with synthetic state and no OBS connection.
#[cfg(test)]
use super::*;
use crate::{
    controller::{app_controller::AppController, state::AppState},
    domain::{
        control::{ControlSnapshot, StudioState},
        role::SceneRole,
        scene::{Scene, SceneInventory},
    },
    storage::{config::AppConfig, registry::SceneRegistry},
};
use std::{cell::RefCell, sync::mpsc};

fn pump(duration: Duration) {
    let context = glib::MainContext::default();
    let until = Instant::now() + duration;
    while Instant::now() < until {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn image() -> String {
    let bytes = glib::Bytes::from_owned(vec![255_u8, 255, 255, 255]);
    let texture = gdk::MemoryTexture::new(1, 1, gdk::MemoryFormat::R8g8b8a8, &bytes, 4);
    format!(
        "data:image/png;base64,{}",
        glib::base64_encode(texture.save_to_png_bytes().as_ref())
    )
}

#[test]
#[ignore = "requires a native GTK display; run alone with --test-threads=1"]
fn native_preview_scheduler_bounds_scope_and_visibility() {
    // Isolate GTK from other ignored tests and their harness threads.
    if std::env::var_os("SCENEDECK_PREVIEW_NATIVE_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "ui::previews::scheduler_tests::native_preview_scheduler_bounds_scope_and_visibility", "--ignored", "--test-threads=1"])
            .env("SCENEDECK_PREVIEW_NATIVE_CHILD", "1").output().unwrap();
        assert!(
            output.status.success(),
            "native child failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    crate::ui::test_support::require_native_runner();
    adw::init().expect("native GTK display");
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(include_str!("../../assets/scenedeck.css"));
    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let (tx, _events) = mpsc::sync_channel(64);
    let mut config = AppConfig::default();
    config.live.scene_previews = true;
    let mut registry = SceneRegistry::default();
    let inventory = SceneInventory {
        scenes: (0..30)
            .map(|index| Scene {
                id: format!("Scene {index:02}"),
            })
            .collect(),
        current_id: Some("Scene 00".into()),
        ..Default::default()
    };
    for scene in &inventory.scenes {
        registry.set_scene_role(&scene.id, Some(SceneRole::Primary));
    }
    let state = Rc::new(RefCell::new(AppState::new(config, registry, None, None)));
    {
        let mut state = state.borrow_mut();
        state.obs_status = ObsStatus::Connected {
            obs_version: "synthetic fixture".into(),
        };
        state.scene_collections.current = Some("Disposable fixture".into());
        state.available_requests = vec!["GetSourceScreenshot".into()];
        state.scene_inventory = inventory.clone();
        state.workspace.observed_at = Some(Instant::now());
        state.workspace.snapshot = Some(ControlSnapshot {
            studio: StudioState {
                enabled: Some(false),
                ..Default::default()
            },
            ..Default::default()
        });
    }
    let controller = Rc::new(RefCell::new(AppController::new(
        runtime.handle().clone(),
        tx,
    )));
    let nav = NavigationContext::new(state.clone(), gtk4::Stack::new(), controller);
    let handle = Rc::new(crate::ui::pages::live::build(nav.clone()));
    crate::ui::pages::live::rebuild_scene_cards(&handle, &inventory, &nav);
    handle.root.set_visible_child_name("live");
    let window = gtk4::Window::builder()
        .title("SceneDeck preview scheduler regression fixture")
        .default_width(800)
        .default_height(600)
        .child(&handle.root)
        .build();
    window.present();
    pump(Duration::from_millis(200));

    tick(&handle, &nav);
    let first = handle.previews.borrow().pending.as_ref().unwrap().0.clone();
    assert_eq!(
        first.scene, "Scene 00",
        "Program is prioritized in the initial visible set"
    );
    tick(&handle, &nav);
    assert_eq!(
        handle.previews.borrow().pending.as_ref().unwrap().0,
        first,
        "only one capture may be in flight"
    );
    apply(&handle, &nav, first.clone(), Ok(image()));
    assert!(handle.scene_cards.borrow()[0].picture.paintable().is_some());
    tick(&handle, &nav);
    assert!(
        handle.previews.borrow().pending.is_none(),
        "the aggregate rate limit survives a quick reply"
    );

    // Only cards intersecting the actual GTK viewport may be requested.
    let adjustment = handle.scene_scroll.vadjustment();
    adjustment.set_value(adjustment.upper() - adjustment.page_size());
    pump(Duration::from_millis(100));
    handle.previews.borrow_mut().last_request = Some(Instant::now() - INTERVAL);
    tick(&handle, &nav);
    let bottom = handle.previews.borrow().pending.as_ref().unwrap().0.clone();
    let cards = handle.scene_cards.borrow();
    let requested = cards
        .iter()
        .find(|card| card.scene_id == bottom.scene)
        .unwrap();
    let bounds = requested.root.compute_bounds(&handle.scene_scroll).unwrap();
    assert!(bounds.y() + bounds.height() > 0.0 && bounds.y() < handle.scene_scroll.height() as f32);
    assert_ne!(bottom.scene, first.scene);
    drop(cards);

    state.borrow_mut().config.live.scene_previews = false;
    tick(&handle, &nav);
    assert!(handle.previews.borrow().pending.is_none());
    assert!(handle.previews.borrow().images.is_empty());
    apply(&handle, &nav, bottom, Ok(image()));
    assert!(
        handle.previews.borrow().images.is_empty(),
        "late disabled replies stay discarded"
    );
    assert!(handle
        .scene_cards
        .borrow()
        .iter()
        .all(|card| card.picture.paintable().is_none()));

    state.borrow_mut().config.live.scene_previews = true;
    tick(&handle, &nav);
    let hidden = handle.previews.borrow().pending.as_ref().unwrap().0.clone();
    window.set_visible(false);
    tick(&handle, &nav);
    apply(&handle, &nav, hidden, Ok(image()));
    assert!(handle.previews.borrow().pending.is_none());
    assert!(handle.previews.borrow().images.is_empty());
    window.present();
    pump(Duration::from_millis(100));
    tick(&handle, &nav);
    let disconnected = handle.previews.borrow().pending.as_ref().unwrap().0.clone();
    state.borrow_mut().obs_status = ObsStatus::Disconnected;
    tick(&handle, &nav);
    apply(&handle, &nav, disconnected, Ok(image()));
    assert!(handle.previews.borrow().images.is_empty());

    // A valid image cache is bounded independently from the number of scene cards.
    state.borrow_mut().obs_status = ObsStatus::Connected {
        obs_version: "synthetic fixture".into(),
    };
    let scope = state.borrow().workflow_scope();
    let epoch = state.borrow().workspace.epoch;
    for scene in &inventory.scenes {
        let request = ThumbnailRequest {
            epoch,
            generation: 99,
            scope: scope.clone(),
            scene: scene.id.clone(),
        };
        {
            let mut cache = handle.previews.borrow_mut();
            cache.scope = Some((epoch, scope.clone()));
            cache.pending = Some((request.clone(), Instant::now()));
        }
        apply(&handle, &nav, request, Ok(image()));
    }
    assert_eq!(handle.previews.borrow().images.len(), MAX_IMAGES);
    assert_eq!(
        handle
            .scene_cards
            .borrow()
            .iter()
            .filter(|card| card.picture.paintable().is_some())
            .count(),
        MAX_IMAGES,
        "evicted textures must also leave their card widgets"
    );
    for card in handle.scene_cards.borrow().iter() {
        card.picture.set_paintable(gdk::Paintable::NONE);
    }
    state.borrow_mut().workspace.epoch = epoch + 1;
    restore(&handle, &nav);
    assert!(
        handle
            .scene_cards
            .borrow()
            .iter()
            .all(|card| card.picture.paintable().is_none()),
        "old-session cache cannot be restored into matching scene names"
    );
    tick(&handle, &nav);
    assert!(handle.previews.borrow().images.is_empty());

    // The installed timer must not retain a closed page handle.
    let weak = Rc::downgrade(&handle);
    install(handle.clone(), nav);
    window.close();
    window.set_child(gtk4::Widget::NONE);
    drop(handle);
    assert!(weak.upgrade().is_none());
    pump(Duration::from_millis(150));
}
