use super::*;
use crate::domain::output::OutputRunState;

#[test]
#[ignore = "requires a disposable GTK display; no OBS or keyring access"]
fn native_selector_readbacks_retain_model_rows() {
    crate::ui::native_test!(|| {
        let selector = NamedSelector::build("Fixture", "Fixture");
        let list = ObsNamedList {
            items: vec!["First".into(), "Other".into()],
            current: Some("First".into()),
        };
        selector.update(&list);
        let first = selector.model.item(0).unwrap();
        let changes = Rc::new(Cell::new(0));
        selector.model.connect_items_changed({
            let changes = changes.clone();
            move |_, _, _, _| changes.set(changes.get() + 1)
        });
        selector.update(&list);
        assert_eq!(
            changes.get(),
            0,
            "identical readback must not splice the model"
        );
        selector.dropdown.set_selected(1);
        selector.update(&list);
        assert_eq!(
            selector.dropdown.selected(),
            0,
            "readback must correct an optimistic selection"
        );
        assert_eq!(selector.model.item(0).unwrap(), first);
        assert_eq!(changes.get(), 0);
        selector.update(&ObsNamedList {
            current: Some("Other".into()),
            ..list
        });
        assert_eq!(selector.dropdown.selected(), 1);
        assert_eq!(changes.get(), 0);
        selector.update(&ObsNamedList {
            items: vec!["New".into()],
            current: Some("New".into()),
        });
        assert_eq!(changes.get(), 1);
        assert_eq!(selector.model.string(0).as_deref(), Some("New"));
        selector.update(&ObsNamedList::default());
        assert_eq!(changes.get(), 2);
        assert!(!selector.root.is_visible());
    });
}

#[test]
#[ignore = "requires a disposable native GTK display"]
fn native_obs_setup_popover_follows_the_selected_palette() {
    crate::ui::native_test!(|| {
        use crate::domain::appearance::{MotionLevel, ThemeId, ThemeMode, ThemePreference};
        use crate::ui::theme::ThemeManager;

        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let selectors = HeaderSelectors::build(&nav);
        let names = ObsNamedList {
            items: vec!["Untitled".into(), "Other".into()],
            current: Some("Untitled".into()),
        };
        selectors.update_profiles(&names);
        selectors.update_scene_collections(&names);
        let header = adw::HeaderBar::new();
        selectors.pack_into(&header);
        let root = GtkBox::new(Orientation::Vertical, 0);
        root.append(&header);
        root.append(&Label::new(Some("Settings")));
        let window = gtk4::Window::builder()
            .default_width(600)
            .default_height(400)
            .child(&root)
            .build();
        window.add_css_class("scenedeck-root");
        window.present();
        let popover = selectors.menu.popover().unwrap();
        for (mode, expected) in [
            (ThemeMode::Dark, [46_u8, 52, 64]),
            (ThemeMode::Light, [246, 248, 251]),
            (ThemeMode::Dark, [46, 52, 64]),
        ] {
            let preference = ThemePreference {
                selected_theme: Some(ThemeId::new("nord")),
                mode,
                motion: MotionLevel::Off,
                ..Default::default()
            };
            assert!(ThemeManager::apply(&preference).is_ok());
            // Theme swap drops allocation. Popup on a 0x0 window: no monitor, garbage size, BadAlloc.
            assert!(
                crate::ui::test_support::pump_until(std::time::Duration::from_secs(5), || {
                    window.width() > 0 && selectors.menu.width() > 0
                }),
                "setup button never got allocated"
            );
            selectors.menu.popup();
            crate::ui::test_support::pump(std::time::Duration::from_millis(100));
            assert!(popover.is_mapped());
            let contents = popover.first_child().expect("native popover contents");
            assert_eq!(contents.css_name(), "contents");
            let point = contents
                .compute_point(
                    &popover,
                    &gtk4::graphene::Point::new(
                        contents.width() as f32 / 2.0,
                        (contents.height() - 8) as f32,
                    ),
                )
                .unwrap();
            let snapshot = gtk4::Snapshot::new();
            gtk4::WidgetPaintable::new(Some(&popover)).snapshot(
                &snapshot,
                f64::from(popover.width()),
                f64::from(popover.height()),
            );
            let node = snapshot.to_node().expect("popover render node");
            let texture = popover
                .native()
                .unwrap()
                .renderer()
                .unwrap()
                .render_texture(
                    &node,
                    Some(&gtk4::graphene::Rect::new(
                        0.0,
                        0.0,
                        popover.width() as f32,
                        popover.height() as f32,
                    )),
                );
            let mut downloader = gtk4::gdk::TextureDownloader::new(&texture);
            downloader.set_format(gtk4::gdk::MemoryFormat::R8g8b8a8);
            let (pixels, stride) = downloader.download_bytes();
            let x = (point.x() * texture.width() as f32 / popover.width() as f32) as usize;
            let y = (point.y() * texture.height() as f32 / popover.height() as f32) as usize;
            let offset = y * stride + x * 4;
            if let Some(directory) = std::env::var_os("SCENEDECK_THEME_CAPTURE_DIR") {
                let path = std::path::PathBuf::from(directory)
                    .join(format!("obs-setup-{}.png", mode.as_str()));
                texture.save_to_png(path).unwrap();
            }
            let actual = &pixels[offset..offset + 3];
            assert!(
                actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1),
                "{mode:?} setup popover painted {actual:?}, expected theme card {expected:?}"
            );
        }
        popover.popdown();
        window.close();
    });
}

#[test]
fn connection_model_covers_every_state() {
    let idle = OutputStatus::inactive();
    let live_out = OutputStatus::new(true, OutputRunState::Active);
    let connecting = sidebar_connection_model(&ObsStatus::Connecting, &idle, &idle);
    assert_eq!(connecting.css_class, "obs-connecting");
    assert!(!connecting.button.sensitive);
    assert!(!connecting.button.suggested);
    assert!(!connecting.button.destructive);

    let connected = sidebar_connection_model(
        &ObsStatus::Connected {
            obs_version: "30.1.2".to_string(),
        },
        &live_out,
        &idle,
    );
    assert_eq!(connected.css_class, "obs-connected");
    assert!(connected.button.sensitive);
    assert!(connected.button.destructive);
    assert!(!connected.button.suggested);
    assert!(connected.status_text.contains("30.1.2"));

    let disconnected = sidebar_connection_model(&ObsStatus::Disconnected, &idle, &idle);
    assert_eq!(disconnected.css_class, "obs-disconnected");
    assert!(disconnected.button.sensitive);
    assert!(disconnected.button.suggested);
    assert!(!disconnected.button.destructive);

    let failed = sidebar_connection_model(&ObsStatus::Error("refused".to_string()), &idle, &idle);
    assert_eq!(failed.css_class, "obs-error");
    assert!(failed.button.sensitive);
    assert!(failed.button.suggested);
    assert!(!failed.button.destructive);
    assert!(failed.status_text.contains("refused"));
}

#[test]
fn connected_button_names_only_running_outputs() {
    let connected = ObsStatus::Connected {
        obs_version: "32.2.2".to_string(),
    };
    let idle = OutputStatus::inactive();
    let running = OutputStatus::new(true, OutputRunState::Active);
    let starting = OutputStatus::new(false, OutputRunState::Starting);
    let button = |stream: &OutputStatus, record: &OutputStatus| {
        sidebar_connection_model(&connected, stream, record).button
    };

    let plain = button(&idle, &idle);
    assert_eq!(
        plain.label,
        fl!(LANGUAGE_LOADER, "window-connect-btn-disconnect")
    );
    assert!(!plain.destructive);

    for (stream, record, key) in [
        (&running, &idle, "window-stop-stream-disconnect-label"),
        (&starting, &idle, "window-stop-stream-disconnect-label"),
        (&idle, &running, "window-stop-record-disconnect-label"),
        (&running, &running, "window-stop-disconnect-label"),
    ] {
        let model = button(stream, record);
        assert_eq!(model.label, LANGUAGE_LOADER.get(key));
        assert!(model.destructive);
    }
}

#[test]
fn every_connection_css_class_is_cleared_before_replacement() {
    for status in [
        ObsStatus::Connecting,
        ObsStatus::Connected {
            obs_version: String::new(),
        },
        ObsStatus::Disconnected,
        ObsStatus::Error(String::new()),
    ] {
        assert!(
            status_bar::CONNECTION_CSS_CLASSES.contains(&status.css_class()),
            "{:?} uses a class the widgets never clear",
            status
        );
    }
}
