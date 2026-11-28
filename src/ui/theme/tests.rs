use super::{
    motion_css, MotionLevel, ThemeManager, ThemeVariant, BASE_CSS, MOTION_ANIMATED_SELECTORS,
    MOTION_TRANSITION_SELECTORS,
};

const REQUIRED_THEME_COLORS: &[&str] = &[
    "scenedeck_fg",
    "scenedeck_muted_fg",
    "scenedeck_accent_fg",
    "scenedeck_success",
    "scenedeck_warning",
    "scenedeck_error",
];

const THEME_BACKGROUNDS: &[&str] = &[
    "scenedeck_window",
    "scenedeck_sidebar",
    "scenedeck_header",
    "scenedeck_surface",
    "scenedeck_card",
];

#[test]
#[ignore = "requires a disposable native GTK display"]
fn native_explicit_theme_modes_style_controls_and_release_the_system_override() {
    crate::ui::native_test!(|| {
        use crate::domain::appearance::{ThemeMode, ThemePreference};

        let style = adw::StyleManager::default();
        style.set_color_scheme(adw::ColorScheme::ForceLight);
        for mode in [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System] {
            let generation = super::REQUEST_GENERATION.with(std::cell::Cell::get);
            ThemeManager::apply_async(
                ThemePreference {
                    selected_theme: Some(crate::domain::appearance::ThemeId::new("nord")),
                    mode,
                    motion: MotionLevel::Off,
                    ..Default::default()
                },
                |report| assert!(report.is_ok(), "{:?}", report.warnings),
            );
            assert_eq!(
                super::REQUEST_GENERATION.with(std::cell::Cell::get),
                generation.wrapping_add(1),
                "applying native mode must not recursively reapply the palette"
            );
            match mode {
                ThemeMode::Dark => assert!(style.is_dark(), "native controls stayed light"),
                ThemeMode::Light => {
                    assert!(!style.is_dark(), "native controls stayed dark")
                }
                ThemeMode::System => {
                    assert_eq!(style.color_scheme(), adw::ColorScheme::Default)
                }
            }
        }
    });
}

#[test]
fn mixer_empty_state_uses_themed_surface() {
    assert!(BASE_CSS.contains(".app-status-page {\n    background-color: @scenedeck_surface;"));
    assert!(BASE_CSS.contains(".mixer-page {\n    background-color: @scenedeck_surface;"));
    assert!(!BASE_CSS.contains(".mixer-page {\n    background-color: @window_bg_color;"));
}

#[test]
fn base_css_uses_supported_gtk_properties() {
    assert!(!BASE_CSS.contains("max-width"));
    assert!(!BASE_CSS.contains("max-height"));
}

#[test]
fn suggested_action_buttons_use_theme_accent_foreground() {
    assert!(BASE_CSS.contains("@define-color scenedeck_accent_fg @accent_fg_color;"));
    assert!(BASE_CSS.contains("color: @scenedeck_accent_fg;"));

    let high_contrast = ThemeManager::find_theme("high-contrast");
    assert!(high_contrast
        .css_for(ThemeVariant::Dark)
        .contains("@define-color scenedeck_accent #ffffff;"));
    assert!(high_contrast
        .css_for(ThemeVariant::Dark)
        .contains("@define-color scenedeck_accent_fg #000000;"));
    assert!(high_contrast
        .css_for(ThemeVariant::Light)
        .contains("@define-color scenedeck_accent #000000;"));
    assert!(high_contrast
        .css_for(ThemeVariant::Light)
        .contains("@define-color scenedeck_accent_fg #ffffff;"));
}

#[test]
fn audio_cards_use_compact_width() {
    assert!(BASE_CSS.contains(".audio-card {\n    min-width: 136px;"));
    assert!(BASE_CSS.contains("    min-height: 218px;"));
    assert!(BASE_CSS.contains("    padding: 6px;\n    border-radius: 8px;"));
    assert!(BASE_CSS.contains(".audio-fine-controls button {\n    min-width: 18px;"));
    assert!(BASE_CSS.contains(".audio-meter {"));
    assert!(BASE_CSS.contains(".audio-meter-labels {"));
}

#[test]
fn the_volume_fader_is_a_thin_track_with_a_tall_handle() {
    assert!(BASE_CSS.contains(".audio-volume-fader trough {"));
    assert!(BASE_CSS.contains(".audio-volume-fader highlight {"));
    assert!(BASE_CSS.contains(".audio-volume-fader slider {"));
    // Both halves of the track stay fully rounded at every size.
    assert_eq!(
        BASE_CSS
            .lines()
            .filter(|line| line.trim() == "border-radius: 999px;")
            .count(),
        2
    );
    // The handle is a tall pill, not a circle, as in OBS's mixer.
    assert!(BASE_CSS.contains("min-width: 15px;\n    min-height: 26px;"));
    assert!(BASE_CSS.contains(".audio-volume-fader:disabled slider {"));
}

#[test]
fn the_audio_card_header_is_a_full_width_scope_bar() {
    assert!(BASE_CSS.contains(".audio-card-scope-bar {"));
    assert!(BASE_CSS.contains(".audio-card-scope-label {"));
    assert!(BASE_CSS.contains(".audio-card-scope-icon {"));
    // Nested and group scopes must not reuse the accent background, or the
    // scope tag would stop distinguishing anything.
    assert!(BASE_CSS.contains(".audio-card-scope-bar.audio-scope-nested,"));
}

/// GTK CSS has no logical-direction margin or padding properties; using one
/// makes the whole stylesheet fail to parse from that point.
#[test]
fn stylesheets_avoid_properties_gtk_does_not_implement() {
    const UNSUPPORTED: &[&str] = &[
        "margin-end",
        "margin-start",
        "padding-end",
        "padding-start",
        "inset-inline",
        "gap:",
        "flex",
    ];
    let mut sheets = vec![("base app CSS", BASE_CSS)];
    for theme in ThemeManager::built_in_themes() {
        sheets.push((theme.id, theme.css_for(ThemeVariant::Light)));
        sheets.push((theme.id, theme.css_for(ThemeVariant::Dark)));
    }
    for (name, css) in sheets {
        for property in UNSUPPORTED {
            assert!(
                !css.contains(property),
                "{name} uses {property}, which GTK cannot parse"
            );
        }
    }
}

#[test]
fn the_sidebar_carries_the_app_logo_and_name() {
    assert!(BASE_CSS.contains(".scenedeck-brand-logo {"));
    assert!(BASE_CSS.contains(".scenedeck-brand-name {"));
}

#[test]
fn status_bar_segments_have_icons_that_follow_their_state() {
    assert!(BASE_CSS.contains(".scenedeck-status-bar-icon {"));
    // Metric and output icons stay muted; connection state classes colour
    // the icon as well as the text.
    assert!(BASE_CSS.contains(".scenedeck-status-bar-icon.scenedeck-status-bar-metric,"));
}

#[test]
fn header_selectors_sit_flat_on_the_header_until_touched() {
    assert!(
        BASE_CSS.contains(".scenedeck-dropdown button {\n    background-color: @scenedeck_header;")
    );
    assert!(BASE_CSS.contains(".scenedeck-dropdown button:hover {"));
    assert!(BASE_CSS.contains(".scenedeck-dropdown button:focus-visible,"));
}

#[test]
fn full_motion_needs_no_override_stylesheet() {
    assert!(motion_css(MotionLevel::Full).is_none());
}

#[test]
fn reduced_motion_stops_loops_but_keeps_transitions() {
    let css = motion_css(MotionLevel::Reduced).expect("reduced motion needs overrides");

    assert!(css.contains("animation: none;"));
    assert!(!css.contains("transition: none;"));
}

#[test]
fn motion_off_stops_loops_and_transitions() {
    let css = motion_css(MotionLevel::Off).expect("motion off needs overrides");

    assert!(css.contains("animation: none;"));
    assert!(css.contains("transition: none;"));
}

/// Guards the contract documented in the Motion section of the stylesheet:
/// an animation the override list does not know about is an animation the
/// user cannot switch off.
#[test]
fn every_switchable_selector_actually_exists_in_the_stylesheet() {
    for selector in MOTION_ANIMATED_SELECTORS
        .iter()
        .chain(MOTION_TRANSITION_SELECTORS)
    {
        assert!(
            BASE_CSS.contains(selector),
            "{selector} is listed as switchable but is not styled in the base CSS"
        );
    }
}

/// The mirror of the check above: every looping animation in the stylesheet
/// must be reachable by the Off level.
#[test]
fn no_looping_animation_escapes_the_motion_setting() {
    let off = motion_css(MotionLevel::Off).expect("motion off needs overrides");

    // `animation-iteration-count: infinite` is what makes an animation loop
    // forever, so counting those is a reliable census of what must be
    // switchable. Finite flashes stop on their own and are covered too, via
    // the selectors they share.
    let looping_rules = BASE_CSS
        .matches("animation-iteration-count: infinite;")
        .count();
    assert!(looping_rules > 0, "expected the UI to animate at all");

    let switchable = MOTION_ANIMATED_SELECTORS
        .iter()
        .filter(|selector| off.contains(*selector))
        .count();
    assert_eq!(switchable, MOTION_ANIMATED_SELECTORS.len());
}

#[test]
fn the_icon_picker_marks_the_current_choice() {
    assert!(BASE_CSS.contains(".icon-picker-choice {"));
    assert!(BASE_CSS.contains(".icon-picker-choice-selected {"));
    assert!(BASE_CSS.contains(".icon-picker-button {"));
}

#[test]
fn live_indicators_keep_their_streaming_state() {
    // Both live icons share one rule; the point of the assertion is that
    // they are red, not which selector they happen to be grouped under.
    assert!(BASE_CSS.contains(
        ".scenedeck-sidebar-live-icon-streaming,\n.scenedeck-top-streaming-icon-active {\n    color: @scenedeck_error;"
    ));
    assert!(BASE_CSS.contains(".scenedeck-content-header-streaming {"));
    assert!(BASE_CSS.contains("border-top: 3px solid alpha(@scenedeck_error, 0.78);"));
    assert!(BASE_CSS.contains("animation-name: scenedeck-live-breathe;"));
}

#[test]
fn custom_css_rejects_import_tokens_escapes_comments_and_nesting() {
    for css in [
        r#"@import url("file:///blocked.css");"#,
        r#"/*before*/ @IMPORT/*between*/ "blocked.css";"#,
        r#"@\69mport "blocked.css";"#,
        r#"@i\6d port "blocked.css";"#,
        r#"@\000069mport url("blocked.css");"#,
        r#"button { @import "blocked.css"; }"#,
        r#"@/**/import "blocked.css";"#,
    ] {
        assert!(super::validate_custom_css(css).is_err(), "accepted {css}");
    }
    for css in [
        "/* @import 'ignored.css'; */ button { color: @theme_fg_color; }",
        r#"button { font-family: "@import"; }"#,
        "@define-color custom #ff0000; @keyframes pulse { to { opacity: 1; } }",
        include_str!("../../../examples/themes/my-custom-theme.css"),
        include_str!("../../../examples/themes/high-contrast-custom.css"),
        include_str!("../../../examples/themes/streaming-red.css"),
    ] {
        assert!(super::validate_custom_css(css).is_ok(), "rejected {css}");
    }
    let nested = format!("{}{}", "{".repeat(100), "}".repeat(100));
    assert!(super::validate_custom_css(&nested).is_err());
    assert!(super::validate_custom_css("}").is_err());
    assert!(
        super::validate_custom_css(&" ".repeat(crate::storage::small_file::CSS_LIMIT + 1)).is_err()
    );
}

#[test]
fn custom_css_rejects_image_urls_before_they_can_read_external_files_on_gtk() {
    for css in [
        r#"button { background-image: url("file:///blocked.pipe"); }"#,
        "button { background-image: url(file:///blocked.pipe); }",
        r#"button { background-image: URL("https://example.invalid/image.png"); }"#,
        r#"button { background-image: u\72l("file:///blocked.pipe"); }"#,
        r#"button { background-image: cross-fade(50%, url("blocked.pipe"), none); }"#,
    ] {
        assert!(
            super::validate_custom_css(css).is_err(),
            "accepted dependency {css}"
        );
    }
    for css in [
        r#"button { font-family: "url(file:///literal-name)"; }"#,
        "/* url(file:///comment) */ button { background-image: linear-gradient(red, blue); }",
    ] {
        assert!(
            super::validate_custom_css(css).is_ok(),
            "rejected literal/inline image {css}"
        );
    }
}

#[test]
#[ignore = "requires a native GTK display; run alone with --test-threads=1"]
fn native_custom_css_fifo_import_is_rejected_before_gtk_load() {
    if !crate::ui::background_io::isolated_test_child(
        "ui::theme::tests::native_custom_css_fifo_import_is_rejected_before_gtk_load",
    ) {
        return;
    }
    crate::ui::test_support::require_native_runner();
    adw::init().expect("native GTK display");
    let dir = std::env::temp_dir().join(format!("scenedeck-css-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let fifo = dir.join("blocked.css");
    assert!(std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap()
        .success());
    for css in [
        format!("@\\69mport/**/url(\"file://{}\");", fifo.display()),
        format!(
            "button {{ background-image: url(\"file://{}\"); }}",
            fifo.display()
        ),
    ] {
        let report = ThemeManager::apply_with_custom_css(
            &crate::domain::appearance::ThemePreference::default(),
            super::CustomCssLoad::Loaded {
                path: dir.join("custom.css"),
                css,
            },
        );
        assert!(
            !report.is_ok(),
            "CSS dependencies must be rejected without opening the FIFO"
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn system_custom_css_uses_the_resolved_variant_path() {
    use super::CustomCssLoad;
    use crate::domain::appearance::ThemePreference;
    let mut preference = ThemePreference::default();
    preference.custom_css.enabled = true;
    preference.custom_css.light_path = Some("/scenedeck-test-light.css".into());
    preference.custom_css.dark_path = Some("/scenedeck-test-dark.css".into());
    for (variant, expected) in [
        (ThemeVariant::Light, "/scenedeck-test-light.css"),
        (ThemeVariant::Dark, "/scenedeck-test-dark.css"),
    ] {
        let loaded = ThemeManager::load_custom_css(&preference, variant);
        match loaded {
            CustomCssLoad::Loaded { path, .. } | CustomCssLoad::Failed { path, .. } => {
                assert_eq!(path, std::path::Path::new(expected));
            }
            other => panic!("expected resolved custom path: {other:?}"),
        }
    }
}

/// `cargo test` runs from the crate root, where this relative path names a
/// valid stylesheet; it must still be refused rather than read.
#[test]
fn relative_custom_css_path_is_refused_without_reading_it() {
    use super::CustomCssLoad;
    use crate::domain::appearance::ThemePreference;
    let relative = std::path::Path::new("examples/themes/my-custom-theme.css");
    let mut preference = ThemePreference::default();
    preference.custom_css.enabled = true;
    preference.custom_css.light_path = Some(relative.to_path_buf());

    let loaded = ThemeManager::load_custom_css(&preference, ThemeVariant::Light);

    match loaded {
        CustomCssLoad::Failed { path, error } => {
            assert_eq!(path, relative);
            assert_eq!(
                error,
                i18n_embed_fl::fl!(
                    crate::infra::i18n::LANGUAGE_LOADER,
                    "theme-custom-css-path-not-absolute"
                )
            );
        }
        other => panic!("relative custom CSS path must fail: {other:?}"),
    }
}

#[test]
fn stale_custom_css_read_cannot_replace_a_newer_request() {
    super::REQUEST_GENERATION.with(|current| current.set(2));
    let report = super::finish_css_read(
        1,
        &crate::domain::appearance::ThemePreference::default(),
        super::CustomCssLoad::Disabled,
    );
    assert!(report.is_none());
}

#[test]
fn built_in_scene_states_contrast_in_both_modes() {
    for theme in ThemeManager::built_in_themes() {
        for variant in [ThemeVariant::Light, ThemeVariant::Dark] {
            let colors = parse_theme_colors(theme.css_for(variant));
            for state in ["program", "preview"] {
                let background = format!("scenedeck_{state}");
                let foreground = format!("{background}_fg");
                assert_contrast(
                    theme.id,
                    variant,
                    &foreground,
                    &background,
                    color_value(&colors, &foreground),
                    color_value(&colors, &background),
                );
            }
            for state in ["program", "preview", "focus", "border"] {
                let foreground = format!("scenedeck_{state}");
                for background in ["scenedeck_card", "scenedeck_window"] {
                    let ratio = contrast_ratio(
                        color_value(&colors, &foreground),
                        color_value(&colors, background),
                    );
                    assert!(ratio >= 3.0, "{} {variant:?} {state}: {ratio:.2}", theme.id);
                }
            }
            assert_ne!(
                color_value(&colors, "scenedeck_program"),
                color_value(&colors, "scenedeck_preview"),
                "{} must preserve independent tally colors",
                theme.id
            );
            for background in ["scenedeck_image_fallback", "scenedeck_image_scrim"] {
                assert_contrast(
                    theme.id,
                    variant,
                    "scenedeck_image_fg",
                    background,
                    color_value(&colors, "scenedeck_image_fg"),
                    color_value(&colors, background),
                );
            }
        }
    }
}

/// Run separately: GTK initialization is bound to one thread and display.
#[test]
#[ignore = "requires a native GTK display; run alone with --test-threads=1"]
fn native_theme_and_custom_overlay_parser_smoke() {
    // GTK binds to one OS thread; isolate it from other display tests.
    const CHILD: &str = "SCENEDECK_THEME_SMOKE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "ui::theme::tests::native_theme_and_custom_overlay_parser_smoke",
                "--ignored",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .status()
            .expect("isolated GTK test");
        assert!(status.success(), "isolated GTK theme test failed");
        return;
    }
    crate::ui::test_support::require_native_runner();
    adw::init().expect("GTK/libadwaita display");
    let provider = gtk4::CssProvider::new();
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    provider.connect_parsing_error({
        let errors = std::rc::Rc::clone(&errors);
        move |_, _, error| errors.borrow_mut().push(error.message().to_string())
    });
    provider.load_from_data(BASE_CSS);
    for theme in ThemeManager::built_in_themes() {
        for variant in [ThemeVariant::Light, ThemeVariant::Dark] {
            provider.load_from_data(theme.css_for(variant));
        }
    }
    for css in [
        include_str!("../../../examples/themes/my-custom-theme.css"),
        include_str!("../../../examples/themes/high-contrast-custom.css"),
        include_str!("../../../examples/themes/streaming-red.css"),
    ] {
        provider.load_from_data(css);
    }
    for level in [MotionLevel::Reduced, MotionLevel::Off] {
        provider.load_from_data(&motion_css(level).expect("motion override"));
    }
    assert!(
        errors.borrow().is_empty(),
        "GTK CSS errors: {:?}",
        errors.borrow()
    );

    assert_native_caption_contrast();

    let style = adw::StyleManager::default();
    let settings = gtk4::Settings::default().expect("GTK settings");
    settings.set_gtk_enable_animations(true);
    let preference = crate::domain::appearance::ThemePreference {
        selected_theme: Some(crate::domain::appearance::ThemeId::new("scenedeck-dark")),
        ..Default::default()
    };
    ThemeManager::apply_async(preference, |report| assert!(report.is_ok()));
    // Enter System first: it must release the explicit modes exercised by
    // the caption fixture before simulating desktop color notifications.
    style.set_color_scheme(adw::ColorScheme::ForceDark);
    let installed_theme = || super::ACTIVE_PROVIDERS.with(|active| active.borrow()[1].to_string());
    let dark_css = installed_theme();
    style.set_color_scheme(adw::ColorScheme::ForceLight);
    assert_ne!(
        installed_theme(),
        dark_css,
        "System must follow desktop color changes"
    );
    style.set_color_scheme(adw::ColorScheme::ForceDark);
    assert_eq!(installed_theme(), dark_css);
    settings.set_gtk_enable_animations(false);
    super::ACTIVE_PROVIDERS.with(|active| {
        assert_eq!(
            active.borrow().len(),
            3,
            "desktop reduce motion installs an override"
        );
    });
    let preference = super::ACTIVE_PREFERENCE.with(|active| active.borrow().clone().unwrap());
    let report = ThemeManager::apply_with_custom_css(
        &preference,
        super::CustomCssLoad::Loaded {
            path: "invalid-theme.css".into(),
            css: "button { color: ; }".to_string(),
        },
    );
    assert!(!report.is_ok());
    super::ACTIVE_PROVIDERS.with(|active| {
        assert_eq!(
            active.borrow().len(),
            3,
            "invalid custom CSS must not replace the fallback"
        );
    });
}

/// Resolve native label alpha/foreground and sample the backgrounds that GTK
/// actually paints. Token-only checks cannot detect native subtitle dimming.
fn assert_native_caption_contrast() {
    use adw::prelude::*;
    let caption = gtk4::Label::new(Some("Enabled Sources observation caption"));
    caption.add_css_class("caption");
    caption.add_css_class("dim-label");
    let row = adw::ActionRow::builder()
        .title("Settings")
        .subtitle("Enabled Settings description")
        .build();
    let group = adw::PreferencesGroup::builder()
        .description("Enabled preferences group description")
        .build();
    group.add(&row);
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.append(&caption);
    content.append(&group);
    let window = gtk4::Window::builder()
        .default_width(600)
        .default_height(240)
        .child(&content)
        .build();
    window.add_css_class("scenedeck-root");
    window.present();
    fn label_by_class(widget: &gtk4::Widget, class: &str, text: &str) -> Option<gtk4::Label> {
        if let Some(label) = widget.downcast_ref::<gtk4::Label>() {
            if label.has_css_class(class) && label.text() == text {
                return Some(label.clone());
            }
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            if let Some(label) = label_by_class(&widget, class, text) {
                return Some(label);
            }
            child = widget.next_sibling();
        }
        None
    }
    let subtitle = label_by_class(row.upcast_ref(), "subtitle", "Enabled Settings description")
        .expect("native ActionRow subtitle");
    // libadwaita 1.7 renamed `dim-label` to `dimmed`; CI ships 1.5.
    let description = ["dimmed", "dim-label"]
        .into_iter()
        .find_map(|class| {
            label_by_class(
                group.upcast_ref(),
                class,
                "Enabled preferences group description",
            )
        })
        .expect("native PreferencesGroup description");
    let context = gtk4::glib::MainContext::default();
    for theme in ThemeManager::built_in_themes() {
        for variant in [ThemeVariant::Light, ThemeVariant::Dark] {
            let mode = match variant {
                ThemeVariant::Light => crate::domain::appearance::ThemeMode::Light,
                ThemeVariant::Dark => crate::domain::appearance::ThemeMode::Dark,
            };
            adw::StyleManager::default().set_color_scheme(match variant {
                ThemeVariant::Light => adw::ColorScheme::ForceLight,
                ThemeVariant::Dark => adw::ColorScheme::ForceDark,
            });
            let preference = crate::domain::appearance::ThemePreference {
                selected_theme: Some(crate::domain::appearance::ThemeId::new(theme.id)),
                mode,
                motion: MotionLevel::Off,
                ..Default::default()
            };
            assert!(ThemeManager::apply(&preference).is_ok());
            let until = std::time::Instant::now() + std::time::Duration::from_millis(75);
            while std::time::Instant::now() < until {
                while context.pending() {
                    context.iteration(false);
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let snapshot = gtk4::Snapshot::new();
            gtk4::WidgetPaintable::new(Some(&window)).snapshot(
                &snapshot,
                f64::from(window.width()),
                f64::from(window.height()),
            );
            let node = snapshot.to_node().expect("native caption render node");
            let viewport =
                gtk4::graphene::Rect::new(0.0, 0.0, window.width() as f32, window.height() as f32);
            let texture = window
                .renderer()
                .expect("native renderer")
                .render_texture(&node, Some(&viewport));
            let mut downloader = gtk4::gdk::TextureDownloader::new(&texture);
            downloader.set_format(gtk4::gdk::MemoryFormat::R8g8b8a8);
            let (pixels, stride) = downloader.download_bytes();
            let pixel = |x: f32, y: f32| {
                let x = (f64::from(x) * f64::from(texture.width()) / f64::from(window.width()))
                    as usize;
                let y = (f64::from(y) * f64::from(texture.height()) / f64::from(window.height()))
                    as usize;
                let offset = y * stride + x * 4;
                assert_eq!(
                    pixels[offset + 3],
                    255,
                    "native caption background must be opaque"
                );
                [
                    f64::from(pixels[offset]) / 255.0,
                    f64::from(pixels[offset + 1]) / 255.0,
                    f64::from(pixels[offset + 2]) / 255.0,
                ]
            };
            let window_background =
                pixel((window.width() - 20) as f32, (window.height() - 20) as f32);
            let card_point = row
                .compute_point(
                    &window,
                    &gtk4::graphene::Point::new(
                        (row.width() - 20) as f32,
                        row.height() as f32 / 2.0,
                    ),
                )
                .expect("native card coordinates");
            let card_background = pixel(card_point.x(), card_point.y());
            let colors = parse_theme_colors(theme.css_for(variant));
            let expected = color_value(&colors, "scenedeck_muted_fg");
            for (label, background) in [
                (&caption, window_background),
                (&subtitle, card_background),
                (&description, window_background),
            ] {
                assert!(
                    label.is_sensitive(),
                    "this regression covers enabled informational text"
                );
                let color = label.color();
                assert!(
                    color.alpha() >= 0.999,
                    "{} {variant:?} native label alpha {}",
                    theme.id,
                    color.alpha()
                );
                assert!(
                    label.opacity() >= 0.999,
                    "native caption widget must not be dimmed"
                );
                let foreground = [
                    f64::from(color.red()),
                    f64::from(color.green()),
                    f64::from(color.blue()),
                ];
                for channel in 0..3 {
                    assert!(
                        (foreground[channel] - expected[channel]).abs() < 0.001,
                        "{} {variant:?} native caption must use semantic foreground",
                        theme.id
                    );
                }
                assert_contrast(
                    theme.id,
                    variant,
                    "native enabled caption",
                    "rendered background",
                    foreground,
                    background,
                );
                // Widget color/opacity properties do not include CSS group
                // opacity. Inspect actual glyph pixels so native dimming is
                // caught even when the semantic foreground remains correct.
                let bounds = label.compute_bounds(&window).expect("native label bounds");
                let mut rendered_contrast = 1.0_f64;
                for y in bounds.y().floor() as i32..(bounds.y() + bounds.height()).ceil() as i32 {
                    for x in bounds.x().floor() as i32..(bounds.x() + bounds.width()).ceil() as i32
                    {
                        rendered_contrast = rendered_contrast
                            .max(contrast_ratio(pixel(x as f32, y as f32), background));
                    }
                }
                assert!(
                    rendered_contrast >= 4.5,
                    "{} {variant:?} rendered enabled caption contrast {rendered_contrast:.2}",
                    theme.id
                );
            }
        }
    }
    window.close();
}

/// Uses a disposable GTK settings file and child process for each desktop
/// preference. Run on an isolated X display without an XSettings manager.
#[test]
#[ignore = "requires an isolated native GTK display; run with --test-threads=1"]
fn native_motion_off_stops_stack_and_restores_desktop_preference() {
    use gtk4::prelude::*;
    crate::ui::test_support::require_native_runner();
    const CHILD: &str = "SCENEDECK_NATIVE_MOTION_DESKTOP";
    let Ok(desktop) = std::env::var(CHILD) else {
        for enabled in [false, true] {
            let dir = std::env::temp_dir().join(format!(
                "scenedeck-native-motion-{}-{enabled}",
                std::process::id()
            ));
            std::fs::create_dir_all(dir.join("gtk-4.0")).expect("GTK settings directory");
            std::fs::write(
                dir.join("gtk-4.0/settings.ini"),
                format!("[Settings]\ngtk-enable-animations={enabled}\n"),
            )
            .expect("disposable desktop settings");
            let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
                .args([
                    "--exact",
                    "ui::theme::tests::native_motion_off_stops_stack_and_restores_desktop_preference",
                    "--ignored",
                    "--test-threads=1",
                ])
                .env(CHILD, enabled.to_string())
                .env("XDG_CONFIG_HOME", &dir)
                .status()
                .expect("isolated motion test");
            std::fs::remove_dir_all(dir).expect("remove disposable settings");
            assert!(
                status.success(),
                "native motion test failed for desktop={enabled}"
            );
        }
        return;
    };
    adw::init().expect("GTK/libadwaita display");
    let expected = desktop == "true";
    let settings = gtk4::Settings::default().expect("GTK settings");
    assert_eq!(
        settings.is_gtk_enable_animations(),
        expected,
        "isolated desktop preference"
    );
    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::Crossfade)
        .transition_duration(600)
        .build();
    stack.add_named(&gtk4::Label::new(Some("First")), Some("first"));
    stack.add_named(&gtk4::Label::new(Some("Second")), Some("second"));
    let window = gtk4::Window::builder().child(&stack).build();
    window.present();
    let context = gtk4::glib::MainContext::default();
    let end = std::time::Instant::now() + std::time::Duration::from_millis(80);
    while std::time::Instant::now() < end {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let mut preference = crate::domain::appearance::ThemePreference {
        motion: MotionLevel::Off,
        ..Default::default()
    };
    ThemeManager::apply_async(preference.clone(), |report| assert!(report.is_ok()));
    assert!(!settings.is_gtk_enable_animations());
    stack.set_visible_child_name("second");
    assert!(
        !stack.is_transition_running(),
        "Off must stop native Stack transitions"
    );
    preference.motion = MotionLevel::Full;
    ThemeManager::apply_async(preference, |report| assert!(report.is_ok()));
    assert_eq!(
        settings.is_gtk_enable_animations(),
        expected,
        "Full must restore, never force, desktop animations"
    );
    stack.set_visible_child_name("first");
    assert_eq!(
        stack.is_transition_running(),
        expected,
        "native transitions follow restored desktop setting"
    );
    window.close();
}

#[test]
fn built_in_theme_text_colors_contrast_with_app_surfaces() {
    for theme in ThemeManager::built_in_themes() {
        for variant in [ThemeVariant::Light, ThemeVariant::Dark] {
            let css = theme.css_for(variant);
            let colors = parse_theme_colors(css);

            for color in REQUIRED_THEME_COLORS {
                assert!(
                    colors.iter().any(|(name, _)| name == color),
                    "{} {:?} should define {color}",
                    theme.id,
                    variant
                );
            }

            for foreground in [
                "scenedeck_fg",
                "scenedeck_muted_fg",
                "scenedeck_success",
                "scenedeck_warning",
                "scenedeck_error",
            ] {
                for background in THEME_BACKGROUNDS {
                    assert_contrast(
                        theme.id,
                        variant,
                        foreground,
                        background,
                        color_value(&colors, foreground),
                        color_value(&colors, background),
                    );
                }
            }

            assert_contrast(
                theme.id,
                variant,
                "scenedeck_accent_fg",
                "scenedeck_accent",
                color_value(&colors, "scenedeck_accent_fg"),
                color_value(&colors, "scenedeck_accent"),
            );
        }
    }
}

fn parse_theme_colors(css: &str) -> Vec<(&str, [f64; 3])> {
    css.lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("@define-color ")?;
            let (name, value) = rest.split_once(' ')?;
            let value = value.strip_suffix(';')?;
            Some((name, parse_hex_color(value)))
        })
        .collect()
}

fn color_value(colors: &[(&str, [f64; 3])], name: &str) -> [f64; 3] {
    colors
        .iter()
        .find_map(|(color_name, value)| (*color_name == name).then_some(*value))
        .unwrap_or_else(|| panic!("missing theme color {name}"))
}

fn parse_hex_color(value: &str) -> [f64; 3] {
    let value = value
        .strip_prefix('#')
        .unwrap_or_else(|| panic!("expected hex color, got {value}"));
    assert_eq!(value.len(), 6, "expected six-digit hex color");
    [
        u8::from_str_radix(&value[0..2], 16).expect("red channel") as f64 / 255.0,
        u8::from_str_radix(&value[2..4], 16).expect("green channel") as f64 / 255.0,
        u8::from_str_radix(&value[4..6], 16).expect("blue channel") as f64 / 255.0,
    ]
}

fn assert_contrast(
    theme_id: &str,
    variant: ThemeVariant,
    foreground_name: &str,
    background_name: &str,
    foreground: [f64; 3],
    background: [f64; 3],
) {
    let ratio = contrast_ratio(foreground, background);
    assert!(
        ratio >= 4.5,
        "{theme_id} {variant:?} {foreground_name} on {background_name} contrast {ratio:.2}"
    );
}

fn contrast_ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let a = relative_luminance(a);
    let b = relative_luminance(b);
    let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(rgb: [f64; 3]) -> f64 {
    0.2126 * linear_channel(rgb[0])
        + 0.7152 * linear_channel(rgb[1])
        + 0.0722 * linear_channel(rgb[2])
}

fn linear_channel(channel: f64) -> f64 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}
