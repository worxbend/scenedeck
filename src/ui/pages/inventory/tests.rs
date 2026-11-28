use super::{reordered_scene_ids, role_at, role_row};
use crate::domain::role::SceneRole;

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_scene_search_keeps_focus_empty_results_and_scope_fencing() {
    crate::ui::native_test!(|| {
        use crate::domain::scene::Scene;
        use crate::ui::test_support::{navigation, pump, pump_until};
        use adw::prelude::*;
        use std::time::Duration;
        let (nav, stack, _runtime, _events) = navigation();
        {
            let mut state = nav.state.borrow_mut();
            state.scene_inventory.scenes = ["Intro", "Guest Camera", "Other"]
                .map(|id| Scene {
                    id: id.into(),
                    uuid: None,
                })
                .into();
            state
                .registry
                .set_scene_tags("Guest Camera", ["Interview".into()]);
            state.registry.set_scene_favorite("Guest Camera", true);
            state.registry.scene_order =
                vec!["Intro".into(), "Guest Camera".into(), "Other".into()];
        }
        let (page, refresh) = super::build(nav.clone());
        stack.add_named(&page, Some("inventory"));
        let window = gtk4::Window::builder()
            .default_width(640)
            .default_height(700)
            .child(&stack)
            .build();
        window.present();
        pump(Duration::from_millis(100));
        let search = find(&page, "deck-search")
            .unwrap()
            .downcast::<gtk4::SearchEntry>()
            .unwrap();
        search.grab_focus();
        search.set_text("INTERVIEW guest");
        assert!(pump_until(Duration::from_secs(2), || nav
            .state
            .borrow()
            .registry
            .scene_filter
            .query
            == "INTERVIEW guest"));
        assert!(find(&page, "scene-order-Intro").is_none());
        assert!(find(&page, "scene-order-Guest Camera").is_some());
        assert!(!super::reorder_scenes(&nav, "Guest Camera", "Intro", false));
        search.set_text("zero matches");
        assert!(pump_until(Duration::from_secs(2), || nav
            .state
            .borrow()
            .registry
            .scene_filter
            .query
            == "zero matches"));
        assert!(find(&page, "deck-no-results").unwrap().is_visible());
        assert_eq!(
            find(&page, "deck-search").unwrap(),
            search.clone().upcast::<gtk4::Widget>()
        );
        assert!(gtk4::prelude::RootExt::focus(&window)
            .is_some_and(|focus| focus == search.clone().upcast::<gtk4::Widget>()
                || focus.is_ancestor(&search)));
        find(&page, "deck-reset")
            .unwrap()
            .downcast::<gtk4::Button>()
            .unwrap()
            .emit_clicked();
        assert_eq!(search.text(), "");
        assert!(find(&page, "scene-order-Intro").is_some());
        assert!(!find(&page, "deck-no-results").unwrap().is_visible());
        assert_eq!(
            nav.state.borrow().registry.scene_order,
            ["Intro", "Guest Camera", "Other"]
        );
        search.set_text("draft before debounce");
        nav.state
            .borrow_mut()
            .registry
            .set_scene_role("Other", Some(SceneRole::Debug));
        refresh();
        assert_eq!(search.text(), "draft before debounce");
        nav.state.borrow_mut().show_registry.generation += 1;
        refresh();
        search.emit_by_name::<()>("search-changed", &[]);
        assert_eq!(nav.state.borrow().registry.scene_filter.query, "");
        assert!(pump_until(Duration::from_secs(5), || {
            crate::ui::persist::registry_save_status()
                == crate::ui::persist::RegistrySaveStatus::Saved
        }));
        window.destroy();
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_scene_tags_and_favorite_preserve_editor_and_reject_retired_callbacks() {
    crate::ui::native_test!(|| {
        use crate::domain::scene::Scene;
        use crate::ui::test_support::{navigation, pump, pump_until};
        use adw::prelude::*;
        use std::time::Duration;
        let (nav, stack, _runtime, _events) = navigation();
        nav.state.borrow_mut().scene_inventory.scenes = vec![Scene {
            id: "Main".into(),
            uuid: None,
        }];
        nav.state
            .borrow_mut()
            .registry
            .set_scene_icon("Main", Some("camera"));
        let (page, refresh) = super::build(nav.clone());
        stack.add_named(&page, Some("inventory"));
        let window = gtk4::Window::builder()
            .default_width(640)
            .default_height(700)
            .child(&stack)
            .build();
        window.present();
        pump(Duration::from_millis(100));
        let options = find(&page, "inventory-options-Main")
            .unwrap()
            .downcast::<gtk4::MenuButton>()
            .unwrap();
        options.popup();
        pump(Duration::from_millis(60));
        let popup = options.popover().unwrap();
        let tags = find(popup.upcast_ref(), "inventory-tags-Main")
            .unwrap()
            .downcast::<gtk4::Entry>()
            .unwrap();
        let favorite = find(popup.upcast_ref(), "inventory-favorite-Main")
            .unwrap()
            .downcast::<gtk4::CheckButton>()
            .unwrap();
        let apply = find(popup.upcast_ref(), "inventory-apply-tags-Main")
            .unwrap()
            .downcast::<gtk4::Button>()
            .unwrap();
        tags.set_text(" Podcast , , podcast, Київ ");
        favorite.set_active(true);
        assert!(
            popup.is_mapped(),
            "favorite updates must keep the tag draft open"
        );
        assert_eq!(tags.text(), " Podcast , , podcast, Київ ");
        apply.emit_clicked();
        assert!(popup.is_mapped());
        assert_eq!(tags.text(), "Podcast, Київ");
        let summary = find(&page, "inventory-curation-Main")
            .unwrap()
            .downcast::<gtk4::Label>()
            .unwrap();
        assert!(summary.text().contains("Podcast, Київ"));
        assert!(!summary.uses_markup());
        tags.set_text("unfinished draft");
        nav.state.borrow_mut().scene_inventory.current_id = Some("Main".into());
        refresh();
        assert_eq!(tags.text(), "unfinished draft");
        nav.state.borrow_mut().show_registry.generation += 1;
        refresh();
        apply.emit_clicked();
        favorite.set_active(false);
        assert_eq!(
            nav.state.borrow().registry.scenes["Main"].tags,
            ["Podcast", "Київ"]
        );
        assert!(nav.state.borrow().registry.scenes["Main"].favorite);
        assert!(pump_until(Duration::from_secs(5), || {
            crate::ui::persist::registry_save_status()
                == crate::ui::persist::RegistrySaveStatus::Saved
        }));
        let saved = crate::storage::registry::read_registry().unwrap();
        assert_eq!(saved.scene_icon("Main"), Some("camera"));
        assert_eq!(saved.scenes["Main"].tags, ["Podcast", "Київ"]);
        assert!(saved.scenes["Main"].favorite);
        window.destroy();
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_retained_identity_is_literal_and_removable_without_changing_replacement() {
    crate::ui::native_test!(|| {
        use crate::domain::scene::{Scene, SceneInventory};
        use adw::prelude::*;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let name = "<b>Old camera & guest</b>";
        let inventory = |uuid: &str| SceneInventory {
            scenes: vec![Scene {
                id: name.into(),
                uuid: Some(uuid.into()),
            }],
            ..Default::default()
        };
        {
            let mut state = nav.state.borrow_mut();
            state.registry.set_scene_icon(name, Some("original"));
            state.registry.reconcile_scenes(&inventory("old"));
            state.scene_inventory = inventory("replacement");
            state.registry.reconcile_scenes(&inventory("replacement"));
            state.registry.set_scene_icon(name, Some("replacement"));
        }
        let (page, _) = super::build(nav.clone());
        let group = find(&page, "inventory-stale").unwrap();
        fn row(widget: &gtk4::Widget) -> Option<adw::ActionRow> {
            if let Ok(row) = widget.clone().downcast::<adw::ActionRow>() {
                return Some(row);
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                if let Some(found) = row(&widget) {
                    return Some(found);
                }
                child = widget.next_sibling();
            }
            None
        }
        fn remove_button(widget: &gtk4::Widget) -> Option<gtk4::Button> {
            if let Ok(button) = widget.clone().downcast::<gtk4::Button>() {
                if button.icon_name().as_deref() == Some("list-remove-symbolic") {
                    return Some(button);
                }
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                if let Some(found) = remove_button(&widget) {
                    return Some(found);
                }
                child = widget.next_sibling();
            }
            None
        }
        let retained = row(&group).unwrap();
        assert_eq!(retained.title(), name);
        assert!(!retained.uses_markup());
        remove_button(&group).unwrap().emit_clicked();
        assert!(nav.state.borrow().registry.retained_scenes.is_empty());
        assert_eq!(
            nav.state.borrow().registry.scene_icon(name),
            Some("replacement")
        );
        assert!(find(&page, "inventory-stale").is_none());
        assert!(crate::ui::test_support::pump_until(
            std::time::Duration::from_secs(5),
            || {
                crate::ui::persist::registry_save_status()
                    == crate::ui::persist::RegistrySaveStatus::Saved
            }
        ));
        assert_eq!(
            crate::storage::registry::read_registry()
                .unwrap()
                .scene_icon(name),
            Some("replacement")
        );
    });
}

#[test]
#[ignore = "requires a disposable GTK display; no OBS or keyring access"]
fn native_inventory_rebuild_releases_yaml_rows_and_root() {
    crate::ui::native_test!(|| {
        use crate::ui::test_support::{navigation, pump};
        use adw::prelude::*;
        use std::time::Duration;
        let (nav, stack, _runtime, _events) = navigation();
        for id in ["Intro", "Main"] {
            nav.state
                .borrow_mut()
                .scene_inventory
                .scenes
                .push(crate::domain::scene::Scene {
                    uuid: None,
                    id: id.into(),
                });
        }
        nav.state
            .borrow_mut()
            .registry
            .set_scene_role("Missing <b>scene</b>", Some(SceneRole::Primary));
        let (page, refresh) = super::build(nav.clone());
        stack.add_named(&page, Some("inventory"));
        let window = gtk4::Window::builder()
            .default_width(1000)
            .default_height(700)
            .child(&stack)
            .build();
        window.present();
        pump(Duration::from_millis(80));
        fn descendants(widget: &gtk4::Widget, tracked: &mut Vec<glib::WeakRef<gtk4::Widget>>) {
            if widget.widget_name() == "inventory-yaml-actions"
                || widget.has_css_class("inventory-list")
                || widget.widget_name().starts_with("scene-order-")
            {
                tracked.push(widget.downgrade());
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                child = widget.next_sibling();
                descendants(&widget, tracked);
            }
        }
        for _ in 0..3 {
            let mut old = Vec::new();
            descendants(&page, &mut old);
            assert!(old.len() >= 4, "real mapped YAML/list/scene rows required");
            gtk4::prelude::GtkWindowExt::set_focus(&window, gtk4::Widget::NONE);
            // Only a real scope retirement replaces the full page now.
            nav.state.borrow_mut().workspace.epoch += 1;
            refresh();
            pump(Duration::from_millis(80));
            assert!(
                old.iter().all(|widget| widget.upgrade().is_none()),
                "detached Inventory descendants retained"
            );
        }
        let weak = page.downgrade();
        window.set_child(gtk4::Widget::NONE);
        window.destroy();
        stack.remove(&page);
        drop(page);
        refresh();
        pump(Duration::from_millis(80));
        assert!(
            weak.upgrade().is_none(),
            "Inventory root retained by row or callback"
        );
    });
}

#[test]
fn every_role_round_trips_through_dropdown_including_unassigned() {
    assert_eq!(role_at(role_row(None)), None);
    for role in SceneRole::ALL {
        assert_eq!(role_at(role_row(Some(role))), Some(role));
    }
    assert_eq!(role_at(u32::MAX), None);
}

#[test]
#[ignore = "requires a disposable GTK display; no OBS or keyring access"]
fn native_unchanged_inventory_refresh_keeps_rows_and_open_popovers() {
    crate::ui::native_test!(|| {
        use crate::domain::scene::Scene;
        use crate::ui::test_support::{navigation, pump};
        use adw::prelude::*;
        use std::time::Duration;
        let (nav, stack, _runtime, _events) = navigation();
        nav.state.borrow_mut().scene_inventory.scenes = vec![Scene {
            uuid: None,
            id: "Main".into(),
        }];
        let (page, refresh) = super::build(nav.clone());
        let row = find(&page, "scene-order-Main").unwrap();
        stack.add_named(&page, Some("inventory"));
        let window = gtk4::Window::builder()
            .default_width(640)
            .default_height(700)
            .child(&stack)
            .build();
        window.present();
        pump(Duration::from_millis(120));
        assert_eq!(
            find(&page, "scene-order-Main").unwrap(),
            row,
            "mapping an already current page must not replace its scene rows"
        );
        let options = find(&page, "inventory-options-Main")
            .unwrap()
            .downcast::<gtk4::MenuButton>()
            .unwrap();
        options.popup();
        pump(Duration::from_millis(60));
        let popup = options.popover().unwrap();
        assert!(popup.is_mapped());
        nav.state.borrow_mut().scene_inventory.current_id = Some("Main".into());
        for _ in 0..3 {
            refresh();
            assert_eq!(find(&page, "scene-order-Main").unwrap(), row);
            assert!(popup.is_mapped(), "unchanged refresh closed the scene menu");
            assert!(row.is_mapped(), "refresh unmapped a visible scene row");
        }
        window.destroy();
        pump(Duration::from_millis(120));
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_inventory_changes_reconcile_without_replacing_the_page() {
    crate::ui::native_test!(|| {
        use crate::domain::scene::Scene;
        use crate::ui::test_support::{navigation, pump};
        use adw::prelude::*;
        use std::time::Duration;
        let (nav, stack, _runtime, _events) = navigation();
        nav.state.borrow_mut().scene_inventory.scenes = vec![
            Scene {
                uuid: None,
                id: "Main".into(),
            },
            Scene {
                uuid: None,
                id: "Other".into(),
            },
        ];
        let (page, refresh) = super::build(nav.clone());
        stack.add_named(&page, Some("inventory"));
        let window = gtk4::Window::builder()
            .default_width(640)
            .default_height(700)
            .child(&stack)
            .build();
        window.present();
        pump(Duration::from_millis(100));
        let main = find(&page, "scene-order-Main").unwrap();
        let other = find(&page, "scene-order-Other").unwrap();
        let yaml = find(&page, "inventory-yaml-actions").unwrap();
        let selector = find(&page, "inventory-role-Main")
            .unwrap()
            .downcast::<gtk4::DropDown>()
            .unwrap();
        nav.state
            .borrow_mut()
            .registry
            .set_scene_role("Main", Some(SceneRole::Primary));
        refresh();
        assert_eq!(
            selector.selected(),
            super::role_row(Some(SceneRole::Primary))
        );
        assert_eq!(find(&page, "scene-order-Main").unwrap(), main);
        assert_eq!(find(&page, "scene-order-Other").unwrap(), other);
        nav.state.borrow_mut().scene_inventory.scenes.push(Scene {
            uuid: None,
            id: "New".into(),
        });
        refresh();
        assert!(
            super::reorder_scenes(&nav, "New", "Main", false),
            "retained handlers must use current membership"
        );
        super::rebuild(page.downcast_ref::<gtk4::Box>().unwrap(), &nav);
        assert_eq!(
            find(&page, "scene-order-New")
                .unwrap()
                .downcast::<gtk4::ListBoxRow>()
                .unwrap()
                .index(),
            0
        );
        assert_eq!(find(&page, "scene-order-Main").unwrap(), main);
        nav.state
            .borrow_mut()
            .registry
            .set_scene_icon("Other", Some("nf-md-camera"));
        refresh();
        assert_ne!(find(&page, "scene-order-Other").unwrap(), other);
        assert_eq!(find(&page, "scene-order-Main").unwrap(), main);
        nav.state
            .borrow_mut()
            .registry
            .set_scene_role("Missing", Some(SceneRole::Archive));
        refresh();
        assert!(find(&page, "inventory-stale").is_some());
        nav.state.borrow_mut().registry.scenes.remove("Missing");
        refresh();
        assert!(find(&page, "inventory-stale").is_none());
        assert_eq!(find(&page, "inventory-yaml-actions").unwrap(), yaml);
        nav.state.borrow_mut().workspace.epoch += 1;
        refresh();
        assert_ne!(
            find(&page, "scene-order-Main").unwrap(),
            main,
            "scope retirement must rebind controls"
        );
        window.destroy();
        pump(Duration::from_millis(120));
    });
}

fn find(root: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
    use adw::prelude::*;
    if root.widget_name() == name {
        return Some(root.clone());
    }
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Some(found) = find(&widget, name) {
            return Some(found);
        }
        child = widget.next_sibling();
    }
    None
}

fn assert_native_list(widget: &gtk4::Widget) {
    use adw::prelude::*;
    assert!(
        !widget.is::<gtk4::ColumnView>(),
        "Inventory must not be a table"
    );
    assert_ne!(widget.widget_name(), "inventory-column-widths");
    if let Some(row) = widget.downcast_ref::<adw::PreferencesRow>() {
        assert!(
            !row.uses_markup(),
            "YAML and stale scene text must remain literal"
        );
    }
    if let Some(scroll) = widget.downcast_ref::<gtk4::ScrolledWindow>().filter(|_| {
        widget.widget_name() == "inventory-scroll"
            || widget.widget_name() == "inventory-table-scroll"
    }) {
        assert_eq!(scroll.hscrollbar_policy(), gtk4::PolicyType::Never);
        let adjustment = scroll.hadjustment();
        assert!(
            adjustment.upper() <= adjustment.page_size() + 1.0,
            "Inventory has hidden horizontal overflow: {} > {}",
            adjustment.upper(),
            adjustment.page_size()
        );
    }
    let mut child = widget.first_child();
    while let Some(widget) = child {
        assert_native_list(&widget);
        child = widget.next_sibling();
    }
}

fn capture(window: &gtk4::Window, name: &str) {
    use adw::prelude::*;
    let Some(directory) = std::env::var_os("SCENEDECK_INVENTORY_CAPTURE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let snapshot = gtk4::Snapshot::new();
    gtk4::WidgetPaintable::new(Some(window)).snapshot(
        &snapshot,
        f64::from(window.width()),
        f64::from(window.height()),
    );
    let node = snapshot.to_node().expect("actual Inventory render node");
    let bounds = gtk4::graphene::Rect::new(0.0, 0.0, window.width() as f32, window.height() as f32);
    window
        .renderer()
        .unwrap()
        .render_texture(&node, Some(&bounds))
        .save_to_png(directory.join(format!("{name}.png")))
        .unwrap();
}

#[test]
#[ignore = "requires a disposable GTK display; no OBS or keyring access"]
fn native_inventory_rows_reflow_without_horizontal_scrolling() {
    crate::ui::native_test!(|| {
        use crate::domain::{appearance::Language, scene::Scene};
        use crate::ui::test_support::{navigation, pump};
        use adw::prelude::*;
        use std::time::Duration;
        // Real long, malformed markup and unbroken names must remain literal.
        let scene = "<b>Studio & Hauptübertragung</b> — SehrLangerSzenennameOhneTrennzeichenFürDieDeutscheÜbertragung";
        for (language, locale, dpi) in [
            (Language::En, "en", 98_304),
            (Language::De, "de-200", 196_608),
        ] {
            crate::infra::i18n::init(language);
            let settings = gtk4::Settings::default().unwrap();
            settings.set_property("gtk-font-name", "Sans 10");
            settings.set_property("gtk-xft-dpi", dpi);
            for width in [360, 640, 1100] {
                let (nav, stack, _runtime, _events) = navigation();
                nav.state.borrow_mut().scene_inventory.scenes = vec![
                    Scene {
                        uuid: None,
                        id: scene.into(),
                    },
                    Scene {
                        uuid: None,
                        id: "Main".into(),
                    },
                ];
                nav.state
                    .borrow_mut()
                    .registry
                    .set_scene_role(scene, Some(SceneRole::Primary));
                nav.state
                    .borrow_mut()
                    .registry
                    .set_scene_role("Missing <b>scene & literal", Some(SceneRole::Primary));
                // Compatibility-only persisted settings must have no effect on layout.
                nav.state.borrow_mut().config =
                    serde_json::from_str(r#"{"inventory":{"name_width":1000,"role_width":600}}"#)
                        .unwrap();
                let saved = serde_json::to_value(&nav.state.borrow().config).unwrap();
                let (page, refresh) = super::build(nav.clone());
                stack.add_named(&page, Some("inventory"));
                let window = gtk4::Window::builder()
                    .default_width(width)
                    .default_height(900)
                    .child(&stack)
                    .build();
                window.add_css_class("scenedeck-root");
                let provider = gtk4::CssProvider::new();
                provider.load_from_data(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/scenedeck.css"
                )));
                gtk4::style_context_add_provider_for_display(
                    &gtk4::prelude::WidgetExt::display(&window),
                    &provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
                window.present();
                pump(Duration::from_millis(200));
                assert!(
                    window.width() <= width,
                    "window minimum escaped requested {width}: {}",
                    window.width()
                );
                assert_native_list(&page);
                for name in [
                    format!("scene-order-{scene}"),
                    format!("inventory-name-{scene}"),
                    format!("inventory-description-{scene}"),
                    format!("inventory-role-{scene}"),
                    format!("inventory-icon-{scene}"),
                    format!("inventory-options-{scene}"),
                    "inventory-yaml-actions".into(),
                    "inventory-export".into(),
                    "inventory-import".into(),
                ] {
                    let widget = find(&page, &name).unwrap_or_else(|| panic!("missing {name}"));
                    let bounds = widget.compute_bounds(&page).unwrap();
                    assert!(
                        bounds.width() > 0.0 && bounds.height() > 0.0,
                        "unallocated {name}"
                    );
                    assert!(
                        bounds.x() >= 0.0
                            && bounds.x() + bounds.width() <= page.width() as f32 + 1.0,
                        "{locale}/{width}: {name} escaped viewport: {bounds:?}"
                    );
                }
                let layout = find(&page, &format!("inventory-row-layout-{scene}"))
                    .unwrap()
                    .downcast::<gtk4::Box>()
                    .unwrap();
                assert_eq!(
                    layout.orientation(),
                    if width == 1100 && locale == "en" {
                        gtk4::Orientation::Horizontal
                    } else {
                        gtk4::Orientation::Vertical
                    }
                );
                if width == 1100 {
                    let selector = find(&page, &format!("inventory-role-{scene}")).unwrap();
                    assert!(selector.grab_focus());
                    pump(Duration::from_millis(80));
                    let focused = gtk4::prelude::RootExt::focus(&window).unwrap();
                    assert!(
                        focused == selector || focused.is_ancestor(&selector),
                        "role focus must be established before resizing"
                    );
                    for resized_width in [360, 1100] {
                        window.set_default_size(resized_width, 900);
                        pump(Duration::from_millis(150));
                        assert_eq!(window.width(), resized_width);
                        assert_native_list(&page);
                        assert_eq!(
                            find(&page, &format!("inventory-role-{scene}")).unwrap(),
                            selector,
                            "resizing must retain the actual editing widget"
                        );
                        let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
                        assert!(focus == selector || focus.is_ancestor(&selector), "{locale}/{resized_width}: resizing lost role focus to {} ({:?}); selector bounds {:?}", focus.widget_name(), focus.type_(), selector.compute_bounds(&page));
                        assert_eq!(
                            layout.orientation(),
                            if resized_width == 1100 && locale == "en" {
                                gtk4::Orientation::Horizontal
                            } else {
                                gtk4::Orientation::Vertical
                            }
                        );
                    }
                }
                let title = find(&page, &format!("inventory-name-{scene}"))
                    .unwrap()
                    .downcast::<gtk4::Label>()
                    .unwrap();
                assert_eq!(title.text(), scene);
                assert!(!title.uses_markup());
                for name in [
                    "inventory-export".to_string(),
                    format!("inventory-role-{scene}"),
                    format!("inventory-options-{scene}"),
                ] {
                    let widget = find(&page, &name).unwrap();
                    assert!(widget.grab_focus(), "{name} not keyboard reachable");
                    refresh();
                    pump(Duration::from_millis(100));
                    let target = find(&page, &name).unwrap();
                    let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
                    assert!(
                        focus == target || focus.is_ancestor(&target),
                        "refresh lost {name}"
                    );
                }
                assert_eq!(
                    serde_json::to_value(&nav.state.borrow().config).unwrap(),
                    saved
                );
                // Capture the top of the list rather than the focused bottom row.
                gtk4::prelude::GtkWindowExt::set_focus(&window, gtk4::Widget::NONE);
                find(&page, "inventory-scroll")
                    .unwrap()
                    .downcast::<gtk4::ScrolledWindow>()
                    .unwrap()
                    .vadjustment()
                    .set_value(0.0);
                pump(Duration::from_millis(80));
                capture(&window, &format!("inventory-{locale}-{width}"));
                window.set_child(gtk4::Widget::NONE);
                window.destroy();
                gtk4::style_context_remove_provider_for_display(
                    &gtk4::prelude::WidgetExt::display(&window),
                    &provider,
                );
                drop(refresh);
                stack.remove(&page);
                drop(page);
                pump(Duration::from_millis(120));
            }
        }
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated XDG paths"]
fn native_inventory_keyboard_reorder_preserves_focus_and_admits_edits() {
    crate::ui::native_test!(|| {
        use crate::domain::scene::Scene;
        use crate::ui::test_support::{navigation, pump};
        use adw::prelude::*;
        use std::time::Duration;
        let (nav, stack, _runtime, _events) = navigation();
        nav.state.borrow_mut().scene_inventory.scenes = vec![
            Scene {
                uuid: None,
                id: "Intro".into(),
            },
            Scene {
                uuid: None,
                id: "Main".into(),
            },
        ];
        let (page, _refresh) = super::build(nav.clone());
        stack.add_named(&page, Some("inventory"));
        let window = gtk4::Window::builder()
            .default_width(360)
            .default_height(700)
            .child(&stack)
            .build();
        window.present();
        pump(Duration::from_millis(100));
        let role = find(&page, "inventory-role-Main")
            .unwrap()
            .downcast::<gtk4::DropDown>()
            .unwrap();
        role.set_selected(super::role_row(Some(SceneRole::Primary)));
        assert_eq!(
            nav.state.borrow().registry.scenes["Main"].role,
            Some(SceneRole::Primary)
        );
        let description = find(&page, "inventory-description-Main")
            .unwrap()
            .downcast::<gtk4::Label>()
            .unwrap();
        assert_eq!(
            description.text().as_str(),
            SceneRole::Primary.description()
        );
        let options = find(&page, "inventory-options-Main")
            .unwrap()
            .downcast::<gtk4::MenuButton>()
            .unwrap();
        options.popup();
        pump(Duration::from_millis(80));
        let up = find(
            options.popover().unwrap().upcast_ref(),
            "inventory-move-up-Main",
        )
        .unwrap()
        .downcast::<gtk4::Button>()
        .unwrap();
        assert!(up.grab_focus());
        up.emit_clicked();
        // Acceptance is synchronous, before any idle rebuild or disk publication.
        assert_eq!(
            nav.state
                .borrow()
                .registry
                .ordered_scene_ids(["Intro", "Main"]),
            ["Main", "Intro"]
        );
        drop(up);
        drop(options);
        drop(role);
        drop(description);
        pump(Duration::from_millis(150));
        let target = find(&page, "scene-order-Main").unwrap();
        let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
        assert!(focus == target || focus.is_ancestor(&target));
        window.destroy();
        pump(Duration::from_millis(120));
    });
}

fn order() -> Vec<String> {
    ["One", "Two", "Three", "Four"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[test]
fn dragging_to_top_half_inserts_before_target() {
    assert_eq!(
        reordered_scene_ids(order(), "Four", "Two", false).unwrap(),
        ["One", "Four", "Two", "Three"]
    );
}

#[test]
fn dragging_to_bottom_half_inserts_after_target() {
    assert_eq!(
        reordered_scene_ids(order(), "One", "Three", true).unwrap(),
        ["Two", "Three", "One", "Four"]
    );
}

#[test]
fn adjacent_keyboard_moves_use_the_same_order_as_drag() {
    assert_eq!(
        reordered_scene_ids(order(), "Two", "One", false).unwrap(),
        ["Two", "One", "Three", "Four"]
    );
    assert_eq!(
        reordered_scene_ids(order(), "Three", "Four", true).unwrap(),
        ["One", "Two", "Four", "Three"]
    );
    assert!(reordered_scene_ids(order(), "Missing", "One", false).is_none());
}

#[test]
fn dropping_scene_on_itself_does_not_reorder() {
    assert!(reordered_scene_ids(order(), "Two", "Two", false).is_none());
}

// ── Section characterization (P0-T13) ──────────────────────────────────
//
// Each native test below pins the widget-tree facts one Inventory section
// exposes today: the widget names other code finds, their labels and their
// sensitivity. Splitting this file must keep every one of them.

fn scenes(ids: &[&str]) -> Vec<crate::domain::scene::Scene> {
    ids.iter()
        .map(|id| crate::domain::scene::Scene {
            id: (*id).into(),
            uuid: None,
        })
        .collect()
}

/// Every descendant of `root`, `root` included, that is a `T`, in tree order.
fn widgets_of<T: gtk4::prelude::IsA<gtk4::Widget>>(root: &gtk4::Widget) -> Vec<T> {
    use adw::prelude::*;
    let mut found: Vec<T> = root.downcast_ref::<T>().cloned().into_iter().collect();
    let mut child = root.first_child();
    while let Some(widget) = child {
        found.extend(widgets_of::<T>(&widget));
        child = widget.next_sibling();
    }
    found
}

fn row_names(page: &gtk4::Widget) -> Vec<String> {
    use adw::prelude::*;
    let list = find(page, "inventory-scenes").unwrap();
    widgets_of::<gtk4::ListBoxRow>(&list)
        .iter()
        .map(|row| row.widget_name().to_string())
        .collect()
}

fn options_popover(page: &gtk4::Widget, scene: &str) -> gtk4::Widget {
    use adw::prelude::*;
    find(page, &format!("inventory-options-{scene}"))
        .unwrap()
        .downcast::<gtk4::MenuButton>()
        .unwrap()
        .popover()
        .unwrap()
        .upcast()
}

fn icon_buttons(root: &gtk4::Widget, icon: &str) -> Vec<gtk4::Button> {
    use adw::prelude::*;
    widgets_of::<gtk4::Button>(root)
        .into_iter()
        .filter(|button| button.icon_name().as_deref() == Some(icon))
        .collect()
}

/// Edit `AppState` through `NavigationContext::update`, as pages should.
fn edit_state(
    nav: &crate::ui::navigation::NavigationContext,
    edit: impl FnOnce(&mut crate::controller::state::AppState),
) {
    nav.update(edit)
        .expect("no AppState borrow outlives a test step");
}

fn scene_entry(
    nav: &crate::ui::navigation::NavigationContext,
    scene: &str,
) -> Option<crate::storage::registry::SceneEntry> {
    nav.read(|state| state.registry.scenes.get(scene).cloned())
}

fn registry_writes_settle() -> bool {
    crate::ui::test_support::pump_until(std::time::Duration::from_secs(5), || {
        crate::ui::persist::registry_save_status() == crate::ui::persist::RegistrySaveStatus::Saved
    })
}

#[test]
#[ignore = "requires a disposable GTK display; no OBS or keyring access"]
fn native_inventory_rows_name_identity_widgets_in_registry_order() {
    crate::ui::native_test!(|| {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use adw::prelude::*;
        use i18n_embed_fl::fl;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        edit_state(&nav, |state| {
            state.scene_inventory.scenes = scenes(&["Intro", "Main", "Other"]);
            state.registry.scene_order = vec!["Other".into(), "Intro".into(), "Main".into()];
            state
                .registry
                .set_scene_role("Main", Some(SceneRole::Primary));
            state.registry.set_scene_tags("Main", ["News".into()]);
            state.registry.set_scene_favorite("Main", true);
        });
        let (page, refresh) = super::build(nav.clone());

        assert_eq!(
            row_names(&page),
            ["scene-order-Other", "scene-order-Intro", "scene-order-Main"]
        );
        for id in ["Intro", "Main", "Other"] {
            for prefix in [
                "inventory-row-layout-",
                "inventory-icon-",
                "inventory-role-",
                "inventory-options-",
            ] {
                assert!(
                    find(&page, &format!("{prefix}{id}")).is_some(),
                    "{prefix}{id}"
                );
            }
            let name = find(&page, &format!("inventory-name-{id}"))
                .unwrap()
                .downcast::<gtk4::Label>()
                .unwrap();
            assert_eq!(name.text(), id);
            assert_eq!(name.tooltip_text().as_deref(), Some(id));
            let drag = find(&page, &format!("inventory-drag-{id}")).unwrap();
            assert!(drag.is::<gtk4::Image>() && drag.is_sensitive());
        }
        let description = |id: &str| {
            find(&page, &format!("inventory-description-{id}"))
                .unwrap()
                .downcast::<gtk4::Label>()
                .unwrap()
                .text()
                .to_string()
        };
        assert_eq!(description("Main"), SceneRole::Primary.description());
        assert_eq!(
            description("Intro"),
            fl!(LANGUAGE_LOADER, "inventory-no-role-assigned")
        );
        let curation = |id: &str| {
            find(&page, &format!("inventory-curation-{id}"))
                .unwrap()
                .downcast::<gtk4::Label>()
                .unwrap()
        };
        assert!(curation("Main").is_visible());
        assert_eq!(
            curation("Main").text(),
            format!(
                "{} · News",
                fl!(LANGUAGE_LOADER, "inventory-scene-favorite")
            )
        );
        assert!(!curation("Intro").is_visible());
        assert!(!find(&page, "deck-no-results").unwrap().is_visible());
        assert!(!find(&page, "deck-reorder-hint").unwrap().is_visible());

        edit_state(&nav, |state| {
            state.registry.scene_filter.query = "Main".into()
        });
        refresh();
        assert_eq!(row_names(&page), ["scene-order-Main"]);
        assert!(
            !find(&page, "inventory-drag-Main").unwrap().is_sensitive(),
            "a filtered list cannot be reordered"
        );
        assert!(find(&page, "deck-reorder-hint").unwrap().is_visible());

        edit_state(&nav, |state| state.workspace.epoch += 1);
        refresh();
        assert_eq!(row_names(&page), ["scene-order-Main"]);
        assert!(
            !find(&page, "inventory-drag-Main").unwrap().is_sensitive(),
            "a full rebuild under a filter keeps reordering off"
        );
        assert!(find(&page, "deck-reorder-hint").unwrap().is_visible());
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_inventory_role_cell_lists_every_role_and_persists_the_selection() {
    crate::ui::native_test!(|| {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use adw::prelude::*;
        use i18n_embed_fl::fl;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        edit_state(&nav, |state| {
            state.scene_inventory.scenes = scenes(&["Main"])
        });
        let (page, _refresh) = super::build(nav.clone());
        let unassigned = fl!(LANGUAGE_LOADER, "inventory-no-role-assigned");

        let role = find(&page, "inventory-role-Main")
            .unwrap()
            .downcast::<gtk4::DropDown>()
            .unwrap();
        assert!(role.has_css_class("inventory-role-selector"));
        let model = role
            .model()
            .unwrap()
            .downcast::<gtk4::StringList>()
            .unwrap();
        let labels: Vec<String> = (0..model.n_items())
            .map(|position| model.string(position).unwrap().to_string())
            .collect();
        let mut expected = vec![SceneRole::unassigned_label()];
        expected.extend(SceneRole::ALL.iter().map(|role| role.label()));
        assert_eq!(labels, expected);
        assert_eq!(role.selected(), role_row(None));
        assert_eq!(role.tooltip_text().as_deref(), Some(unassigned.as_str()));

        role.set_selected(role_row(Some(SceneRole::Debug)));
        assert_eq!(
            scene_entry(&nav, "Main").and_then(|entry| entry.role),
            Some(SceneRole::Debug)
        );
        let description = find(&page, "inventory-description-Main")
            .unwrap()
            .downcast::<gtk4::Label>()
            .unwrap();
        assert_eq!(description.text(), SceneRole::Debug.description());
        assert_eq!(
            role.tooltip_text().as_deref(),
            Some(SceneRole::Debug.description().as_str())
        );

        role.set_selected(role_row(None));
        assert_eq!(scene_entry(&nav, "Main").and_then(|entry| entry.role), None);
        assert_eq!(description.text(), unassigned);
        assert!(registry_writes_settle());
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_inventory_options_popover_names_curation_and_accent_controls() {
    crate::ui::native_test!(|| {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use adw::prelude::*;
        use i18n_embed_fl::fl;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        edit_state(&nav, |state| {
            state.scene_inventory.scenes = scenes(&["Main", "Other"]);
            state
                .registry
                .set_scene_accent("Main", Some("#336699".into()));
        });
        let (page, _refresh) = super::build(nav.clone());

        let options = find(&page, "inventory-options-Main").unwrap();
        assert_eq!(
            options.tooltip_text().map(|text| text.to_string()),
            Some(fl!(
                LANGUAGE_LOADER,
                "inventory-scene-options",
                scene = "Main"
            ))
        );
        let popover = options_popover(&page, "Main");
        let favorite = find(&popover, "inventory-favorite-Main")
            .unwrap()
            .downcast::<gtk4::CheckButton>()
            .unwrap();
        assert!(!favorite.is_active());
        assert_eq!(
            favorite.label().map(|label| label.to_string()),
            Some(fl!(LANGUAGE_LOADER, "inventory-scene-favorite"))
        );
        let tags = find(&popover, "inventory-tags-Main")
            .unwrap()
            .downcast::<gtk4::Entry>()
            .unwrap();
        assert_eq!(tags.text(), "");
        let apply = find(&popover, "inventory-apply-tags-Main")
            .unwrap()
            .downcast::<gtk4::Button>()
            .unwrap();
        assert_eq!(
            apply.label().map(|label| label.to_string()),
            Some(fl!(LANGUAGE_LOADER, "inventory-apply-tags"))
        );
        assert!(find(&popover, "inventory-move-up-Main").is_some());
        assert!(find(&popover, "inventory-move-down-Main").is_some());

        let pickers = widgets_of::<gtk4::ColorDialogButton>(&popover);
        assert_eq!(pickers.len(), 1);
        let rgba = pickers[0].rgba();
        assert_eq!(
            crate::storage::registry::scene_accent_hex(rgba.red(), rgba.green(), rgba.blue()),
            "#336699"
        );
        let clear = icon_buttons(&popover, "edit-clear-symbolic");
        assert_eq!(clear.len(), 1);
        assert!(clear[0].is_sensitive());
        let other_clear = icon_buttons(&options_popover(&page, "Other"), "edit-clear-symbolic");
        assert!(
            !other_clear[0].is_sensitive(),
            "no accent, nothing to clear"
        );

        clear[0].emit_clicked();
        assert_eq!(
            scene_entry(&nav, "Main").and_then(|entry| entry.accent_color),
            None
        );
        assert!(!clear[0].is_sensitive());
        let replaced = widgets_of::<gtk4::ColorDialogButton>(&popover);
        assert_eq!(replaced.len(), 1);
        assert_ne!(replaced[0], pickers[0], "clearing installs an unset picker");
        assert!(registry_writes_settle());
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_inventory_move_buttons_follow_position_and_filter() {
    crate::ui::native_test!(|| {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use crate::ui::test_support::pump;
        use adw::prelude::*;
        use i18n_embed_fl::fl;
        use std::time::Duration;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        edit_state(&nav, |state| {
            state.scene_inventory.scenes = scenes(&["One", "Two", "Three"])
        });
        let (page, refresh) = super::build(nav.clone());
        let move_button = |scene: &str, direction: &str| {
            find(
                &options_popover(&page, scene),
                &format!("inventory-move-{direction}-{scene}"),
            )
            .unwrap()
            .downcast::<gtk4::Button>()
            .unwrap()
        };

        assert!(!move_button("One", "up").is_sensitive());
        assert!(move_button("One", "down").is_sensitive());
        assert!(move_button("Two", "up").is_sensitive());
        assert!(move_button("Two", "down").is_sensitive());
        assert!(move_button("Three", "up").is_sensitive());
        assert!(!move_button("Three", "down").is_sensitive());
        assert_eq!(
            move_button("One", "down")
                .child()
                .and_downcast::<gtk4::Label>()
                .unwrap()
                .text(),
            fl!(LANGUAGE_LOADER, "inventory-move-down", scene = "One")
        );

        move_button("One", "down").emit_clicked();
        assert_eq!(
            nav.read(|state| state.registry.ordered_scene_ids(["One", "Two", "Three"])),
            ["Two", "One", "Three"]
        );
        pump(Duration::from_millis(100));
        assert_eq!(
            row_names(&page),
            ["scene-order-Two", "scene-order-One", "scene-order-Three"]
        );
        assert!(move_button("One", "up").is_sensitive());
        assert!(!move_button("Two", "up").is_sensitive());

        edit_state(&nav, |state| {
            state.registry.scene_filter.query = "One".into()
        });
        refresh();
        assert!(!move_button("One", "up").is_sensitive());
        assert!(!move_button("One", "down").is_sensitive());
        assert!(registry_writes_settle());
    });
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_inventory_stale_group_lists_missing_scenes_and_removes_them() {
    crate::ui::native_test!(|| {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use adw::prelude::*;
        use i18n_embed_fl::fl;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        edit_state(&nav, |state| {
            state.scene_inventory.scenes = scenes(&["Main"]);
            state
                .registry
                .set_scene_role("Gone A", Some(SceneRole::Primary));
            state.registry.set_scene_icon("Gone B", Some("camera"));
        });
        let (page, _refresh) = super::build(nav.clone());

        let group = find(&page, "inventory-stale")
            .unwrap()
            .downcast::<adw::PreferencesGroup>()
            .unwrap();
        assert_eq!(
            group.title(),
            fl!(LANGUAGE_LOADER, "inventory-stale-group-title")
        );
        assert_eq!(
            group.description().map(|text| text.to_string()),
            Some(fl!(LANGUAGE_LOADER, "inventory-stale-group-description"))
        );
        let rows = widgets_of::<adw::ActionRow>(group.upcast_ref());
        assert_eq!(
            rows.iter()
                .map(|row| (row.title().to_string(), row.subtitle().map(String::from)))
                .collect::<Vec<_>>(),
            [
                (
                    "Gone A".to_string(),
                    Some(SceneRole::label_or_unassigned(Some(SceneRole::Primary)))
                ),
                (
                    "Gone B".to_string(),
                    Some(SceneRole::label_or_unassigned(None))
                ),
            ]
        );
        assert!(rows.iter().all(|row| !row.uses_markup()));
        let removes = icon_buttons(group.upcast_ref(), "list-remove-symbolic");
        assert_eq!(removes.len(), 2);
        assert!(removes
            .iter()
            .all(|button| button.has_css_class("destructive-action")));

        removes[0].emit_clicked();
        assert!(scene_entry(&nav, "Gone A").is_none());
        let group = find(&page, "inventory-stale").unwrap();
        let rows = widgets_of::<adw::ActionRow>(&group);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title(), "Gone B");

        icon_buttons(&group, "list-remove-symbolic")[0].emit_clicked();
        assert!(
            find(&page, "inventory-stale").is_none(),
            "removing the last stale entry drops the group"
        );
        assert!(registry_writes_settle());
    });
}

#[test]
#[ignore = "requires a disposable GTK display; no OBS or keyring access"]
fn native_inventory_yaml_row_names_export_and_import_actions() {
    crate::ui::native_test!(|| {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use adw::prelude::*;
        use i18n_embed_fl::fl;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        edit_state(&nav, |state| {
            state.scene_inventory.scenes = scenes(&["Main"])
        });
        let (page, _refresh) = super::build(nav.clone());

        let row = find(&page, "inventory-yaml-actions")
            .unwrap()
            .downcast::<adw::ActionRow>()
            .unwrap();
        assert_eq!(
            row.title(),
            fl!(LANGUAGE_LOADER, "inventory-yaml-row-title")
        );
        assert_eq!(
            row.subtitle().map(String::from),
            Some(fl!(LANGUAGE_LOADER, "inventory-yaml-row-subtitle"))
        );
        assert!(!row.uses_markup());
        // The builder sets `icon-name` after `label`, and GTK replaces the
        // label with the icon: today these buttons are icon-only and their
        // text lives in the tooltip.
        for (name, icon, tooltip) in [
            (
                "inventory-export",
                "document-save-symbolic",
                fl!(LANGUAGE_LOADER, "inventory-export-tooltip"),
            ),
            (
                "inventory-import",
                "document-open-symbolic",
                fl!(LANGUAGE_LOADER, "inventory-import-tooltip"),
            ),
        ] {
            let button = find(&page, name)
                .unwrap()
                .downcast::<gtk4::Button>()
                .unwrap();
            assert!(button.is_sensitive(), "{name}");
            assert_eq!(button.icon_name().as_deref(), Some(icon));
            assert_eq!(button.label(), None, "{name}");
            assert_eq!(button.tooltip_text().map(String::from), Some(tooltip));
        }

        let filter = super::yaml_file_filter();
        assert_eq!(
            filter.name().map(String::from),
            Some(fl!(LANGUAGE_LOADER, "inventory-yaml-filter-name"))
        );
        let rules = filter.to_gvariant().print(false).to_string();
        assert!(
            rules.contains("*.yaml") && rules.contains("*.yml"),
            "{rules}"
        );
        let dialog = super::yaml_dialog("Title", "Accept");
        assert_eq!(dialog.title(), "Title");
        assert_eq!(
            dialog.accept_label().map(String::from),
            Some("Accept".into())
        );
        assert!(dialog.is_modal());
        assert_eq!(
            dialog.default_filter().and_then(|filter| filter.name()),
            filter.name()
        );
    });
}

#[test]
fn dialog_cancellation_covers_dismissal_and_cancelled_io_only() {
    let error = |domain_error: gtk4::DialogError| glib::Error::new(domain_error, "fixture");
    assert!(super::dialog_cancelled(&error(
        gtk4::DialogError::Dismissed
    )));
    assert!(super::dialog_cancelled(&error(
        gtk4::DialogError::Cancelled
    )));
    assert!(!super::dialog_cancelled(&error(gtk4::DialogError::Failed)));
    let io = |io_error: gio::IOErrorEnum| glib::Error::new(io_error, "fixture");
    assert!(super::dialog_cancelled(&io(gio::IOErrorEnum::Cancelled)));
    assert!(!super::dialog_cancelled(&io(
        gio::IOErrorEnum::PermissionDenied
    )));
}

#[test]
#[ignore = "requires a disposable GTK display and isolated registry storage"]
fn native_inventory_save_status_reports_a_failed_write_until_retry_succeeds() {
    crate::ui::native_test!(|| {
        use crate::infra::i18n::LANGUAGE_LOADER;
        use crate::ui::persist::{registry_save_status, RegistrySaveStatus};
        use crate::ui::test_support::{pump, pump_until};
        use adw::prelude::*;
        use i18n_embed_fl::fl;
        use std::time::Duration;
        // A regular file where the registry directory belongs fails every
        // write until it is removed.
        let blocker = crate::storage::xdg::registry_path().with_file_name("blocker");
        std::fs::create_dir_all(blocker.parent().unwrap()).unwrap();
        std::fs::write(&blocker, b"").unwrap();
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        edit_state(&nav, |state| {
            state.scene_inventory.scenes = scenes(&["Main"]);
            state.show_registry.target = crate::storage::show_workspaces::RegistryTarget::File(
                blocker.join("registry.json"),
            );
        });
        let (page, _refresh) = super::build(nav.clone());
        let status = find(&page, "inventory-registry-save-status").unwrap();
        let label = status.first_child().and_downcast::<gtk4::Label>().unwrap();
        let retry = find(&page, "inventory-registry-save-retry")
            .unwrap()
            .downcast::<gtk4::Button>()
            .unwrap();
        assert!(!status.is_visible(), "nothing to report while saved");
        assert_eq!(
            retry.label().map(String::from),
            Some(fl!(LANGUAGE_LOADER, "window-connect-btn-retry"))
        );

        find(&page, "inventory-role-Main")
            .unwrap()
            .downcast::<gtk4::DropDown>()
            .unwrap()
            .set_selected(role_row(Some(SceneRole::Primary)));
        assert!(pump_until(Duration::from_secs(5), || matches!(
            registry_save_status(),
            RegistrySaveStatus::Failed(_)
        )));
        let RegistrySaveStatus::Failed(error) = registry_save_status() else {
            unreachable!("the wait above observed a failure");
        };
        pump(Duration::from_millis(150));
        assert!(status.is_visible());
        assert_eq!(
            label.text(),
            fl!(LANGUAGE_LOADER, "settings-failed-to-save", err = error)
        );
        assert!(retry.is_sensitive());

        std::fs::remove_file(&blocker).unwrap();
        retry.emit_clicked();
        assert!(registry_writes_settle());
        pump(Duration::from_millis(150));
        assert!(!status.is_visible(), "a successful retry clears the report");
        assert_eq!(
            crate::storage::registry::read_registry_from_path(&blocker.join("registry.json"))
                .unwrap()
                .scenes["Main"]
                .role,
            Some(SceneRole::Primary)
        );
    });
}
