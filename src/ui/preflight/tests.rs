use super::*;
use crate::{
    domain::{
        operator_action::ActionTarget,
        output::{OutputKind, OutputStatus},
        preflight::SavedPreflight,
    },
    ui::test_support::{native_test, navigation, pump, pump_until},
};
use std::cell::Cell;

pub(crate) fn find(root: &impl IsA<gtk4::Widget>, name: &str) -> gtk4::Widget {
    fn search(widget: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
        if widget.widget_name() == name {
            return Some(widget.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if let Some(found) = search(&widget, name) {
                return Some(found);
            }
        }
        None
    }
    search(root.as_ref(), name).unwrap_or_else(|| panic!("missing {name}"))
}
pub(crate) fn window(name: &str) -> gtk4::Window {
    gtk4::Window::list_toplevels()
        .into_iter()
        .find_map(|window| {
            window
                .downcast::<gtk4::Window>()
                .ok()
                .filter(|window| window.widget_name() == name && window.is_visible())
        })
        .unwrap_or_else(|| panic!("missing window {name}"))
}
fn click(root: &impl IsA<gtk4::Widget>, name: &str) {
    find(root, name)
        .downcast::<Button>()
        .unwrap()
        .emit_clicked();
}
fn toggle(root: &impl IsA<gtk4::Widget>, name: &str, active: bool) {
    find(root, name)
        .downcast::<CheckButton>()
        .unwrap()
        .set_active(active);
}
fn entry(root: &impl IsA<gtk4::Widget>, name: &str, value: &str) {
    find(root, name)
        .downcast::<gtk4::Entry>()
        .unwrap()
        .set_text(value);
}
pub(crate) fn connected(nav: &NavigationContext) {
    let mut state = super::super::action_catalog::tests::connected();
    state.connected_connection_id = Some("actual-show".into());
    state.connected_endpoint = Some("ws://fixture:4455".into());
    state.config.connection_id = "settings-selection".into();
    state.workspace.snapshot.as_mut().unwrap().scenes = state.scene_inventory.scenes.clone();
    *nav.state.borrow_mut() = state;
}
fn plan(nav: &NavigationContext) -> PreflightPlan {
    Owner::current(&nav.state.borrow())
        .unwrap()
        .plan(&nav.state.borrow())
}
pub(crate) fn save_plan(nav: &NavigationContext, plan: PreflightPlan) {
    nav.state.borrow_mut().config.preflight_plans = vec![SavedPreflight {
        connection: "actual-show".into(),
        endpoint: "ws://fixture:4455".into(),
        plan,
    }];
}
fn signal(nav: &NavigationContext) {
    nav.state.borrow_mut().audio_levels.update_at(
        vec![crate::domain::meter::InputLevels::from_mul(
            "Microphone".into(),
            &[[0.5; 3]],
        )],
        Instant::now(),
    );
}

#[test]
#[ignore = "requires isolated GTK and config storage"]
fn native_preflight_saves_actual_connection_and_retires_checkmarks() {
    native_test(
        "ui::preflight::tests::native_preflight_saves_actual_connection_and_retires_checkmarks",
        || {
            let (nav, _stack, _runtime, _events) = navigation();
            connected(&nav);
            let parent = adw::Window::new();
            parent.present();
            show(parent.upcast_ref(), &nav, None);
            let report = window("preflight-review");
            click(&report, "preflight-configure");
            let editor = window("preflight-editor");
            assert_eq!(
                find(&editor, "preflight-expected")
                    .downcast::<gtk4::Entry>()
                    .unwrap()
                    .text(),
                "Show"
            );
            toggle(&editor, "preflight-scene-0", true);
            toggle(&editor, "preflight-mic-0", true);
            toggle(&editor, "preflight-check-disk", false);
            find(&editor, "preflight-manual-items")
                .downcast::<gtk4::TextView>()
                .unwrap()
                .buffer()
                .set_text("Guests ready\nMonitor headphones");
            click(&editor, "preflight-save");
            assert!(pump_until(Duration::from_secs(3), || !editor.is_visible()));
            let config = crate::storage::config::read_config().config;
            let saved = config
                .preflight_plan("actual-show", "ws://fixture:4455")
                .unwrap();
            assert_eq!(saved.required_scenes[0].uuid.as_deref(), Some("scene-1"));
            assert_eq!(
                saved.microphone.as_ref().unwrap().uuid.as_deref(),
                Some("input-1")
            );
            assert!(config
                .preflight_plan("settings-selection", "ws://fixture:4455")
                .is_none());
            pump(Duration::from_millis(300));
            signal(&nav);
            toggle(&report, "preflight-manual-0", true);
            toggle(&report, "preflight-manual-1", true);
            pump(Duration::from_millis(300));
            assert_eq!(
                find(&report, "preflight-summary")
                    .downcast::<Label>()
                    .unwrap()
                    .text(),
                fl!(LANGUAGE_LOADER, "preflight-ready")
            );
            let old_check = find(&report, "preflight-manual-0")
                .downcast::<CheckButton>()
                .unwrap();
            click(&report, "preflight-reset");
            old_check.set_active(false);
            old_check.set_active(true);
            assert!(!find(&report, "preflight-manual-0")
                .downcast::<CheckButton>()
                .unwrap()
                .is_active());
            toggle(&report, "preflight-manual-0", true);
            {
                let mut state = nav.state.borrow_mut();
                state.invalidate_collection_runtime();
                state.scene_collections.current = Some("Wrong collection".into());
                state.workspace.observed_at = Some(Instant::now());
                state.workspace.dirty = false;
            }
            pump(Duration::from_millis(300));
            assert!(!find(&report, "preflight-manual-0")
                .downcast::<CheckButton>()
                .unwrap()
                .is_active());
            let checks = find(&report, "preflight-checks")
                .downcast::<Label>()
                .unwrap();
            assert!(
                checks.text().contains("Wrong collection")
                    && checks.text().contains("Show")
                    && checks
                        .text()
                        .contains(&fl!(LANGUAGE_LOADER, "preflight-failed"))
            );
            assert_eq!(plan(&nav).expected_collection, "Show");
            assert!(
                nav.state.borrow().workspace.activity.entries.is_empty(),
                "readiness/configuration never requests an output"
            );
            let weak = report.downgrade();
            report.close();
            editor.close();
            parent.close();
            drop((report, editor, parent, checks, old_check));
            pump(Duration::from_millis(300));
            assert!(weak.upgrade().is_none());
        },
    );
}

#[test]
#[ignore = "requires isolated GTK and config storage"]
fn native_preflight_editor_handles_namespace_changes_validation_and_save_retry() {
    native_test("ui::preflight::tests::native_preflight_editor_handles_namespace_changes_validation_and_save_retry", || {
        let (nav, _stack, _runtime, _events) = navigation(); connected(&nav);
        save_plan(&nav, PreflightPlan { expected_collection: "Missing collection".into(), required_scenes: vec![ActionTarget { name: "Missing <b>scene</b>".into(), uuid: Some("absent".into()) }], microphone: Some(ActionTarget { name: "Missing microphone".into(), uuid: Some("absent-mic".into()) }), ..Default::default() });
        let parent = adw::Window::new(); parent.present();
        editor::show(parent.upcast_ref(), &nav, &Owner::current(&nav.state.borrow()).unwrap());
        let editor = window("preflight-editor");
        let missing = find(&editor, "preflight-scene-0").downcast::<CheckButton>().unwrap(); assert!(missing.is_active());
        let label = missing.child().unwrap().downcast::<Label>().unwrap(); assert!(!label.uses_markup());
        entry(&editor, "preflight-expected", "Show");
        assert!(!find(&editor, "preflight-scene-0").downcast::<CheckButton>().unwrap().is_active(), "foreign UUID cannot become a requirement in another collection");
        toggle(&editor, "preflight-scene-0", true); toggle(&editor, "preflight-mic-0", true);
        entry(&editor, "preflight-minimum-disk", "0"); click(&editor, "preflight-save");
        assert!(editor.is_visible()); assert_eq!(plan(&nav).expected_collection, "Missing collection");
        entry(&editor, "preflight-minimum-disk", "2048");
        let path = crate::storage::xdg::config_path(); std::fs::create_dir_all(&path).unwrap();
        click(&editor, "preflight-save");
        assert!(pump_until(Duration::from_secs(3), || find(&editor, "preflight-save-notice").downcast::<Label>().unwrap().text().contains("Could not save")));
        assert!(editor.is_visible()); std::fs::remove_dir(&path).unwrap(); click(&editor, "preflight-save");
        assert!(pump_until(Duration::from_secs(3), || !editor.is_visible()));
        assert_eq!(crate::storage::config::read_config().config.preflight_plans[0].plan.minimum_disk_mib, Some(2048));
        entry(&editor, "preflight-expected", "Closed editor callback"); click(&editor, "preflight-save");
        assert_eq!(plan(&nav).expected_collection, "Show");
        editor::show(parent.upcast_ref(), &nav, &Owner::current(&nav.state.borrow()).unwrap()); let stale = window("preflight-editor");
        entry(&stale, "preflight-expected", "Do not save"); nav.state.borrow_mut().workspace.epoch += 1; click(&stale, "preflight-save");
        assert_eq!(plan(&nav).expected_collection, "Show"); stale.close();
        {
            let mut state = nav.state.borrow_mut(); state.config.preflight_plans.clear();
            state.scene_collections.current = Some(" Show ".into()); state.workspace.epoch += 1;
        }
        editor::show(parent.upcast_ref(), &nav, &Owner::current(&nav.state.borrow()).unwrap());
        let spaces = window("preflight-editor"); toggle(&spaces, "preflight-scene-0", true);
        click(&spaces, "preflight-save"); assert!(pump_until(Duration::from_secs(3), || !spaces.is_visible()));
        assert_eq!(plan(&nav).expected_collection, " Show ", "OBS identity retains meaningful whitespace");
        assert_eq!(plan(&nav).required_scenes.len(), 1);
        parent.close();
    });
}

#[test]
#[ignore = "requires isolated GTK"]
fn native_preflight_start_review_is_opt_in_explicit_and_scope_bound() {
    native_test(
        "ui::preflight::tests::native_preflight_start_review_is_opt_in_explicit_and_scope_bound",
        || {
            let (nav, _stack, _runtime, _events) = navigation();
            connected(&nav);
            let button = Button::with_label("Start");
            let parent = adw::Window::new();
            parent.set_content(Some(&button));
            parent.present();
            let count = Rc::new(Cell::new(0));
            let open = || {
                review_before_start(&button, &nav, OutputKind::Record, {
                    let count = count.clone();
                    move || count.set(count.get() + 1)
                })
            };
            assert!(!open());
            assert_eq!(count.get(), 0);
            save_plan(
                &nav,
                PreflightPlan {
                    review_before_output: true,
                    ..Default::default()
                },
            );
            assert!(open());
            let report = window("preflight-review")
                .downcast::<adw::MessageDialog>()
                .unwrap();
            report.response("close");
            report.response("continue");
            assert_eq!(count.get(), 0);
            assert!(open());
            let report = window("preflight-review")
                .downcast::<adw::MessageDialog>()
                .unwrap();
            assert!(
                report.is_response_enabled("continue"),
                "unresolved checks can be explicitly accepted"
            );
            report.response("continue");
            report.response("continue");
            assert_eq!(count.get(), 1);
            assert!(open());
            let report = window("preflight-review")
                .downcast::<adw::MessageDialog>()
                .unwrap();
            nav.state.borrow_mut().workspace.epoch += 1;
            report.response("continue");
            assert_eq!(count.get(), 1);
            report.close();
            assert!(open());
            let report = window("preflight-review")
                .downcast::<adw::MessageDialog>()
                .unwrap();
            nav.state.borrow_mut().record_status = OutputStatus::active();
            report.response("continue");
            assert_eq!(count.get(), 1);
            report.close();
            parent.close();
            assert!(nav.state.borrow().workspace.activity.entries.is_empty());
        },
    );
}

#[test]
#[ignore = "requires isolated GTK"]
fn native_preflight_large_text_layout_and_observation_updates_keep_manual_focus() {
    native_test("ui::preflight::tests::native_preflight_large_text_layout_and_observation_updates_keep_manual_focus", || {
        let (nav, _stack, _runtime, _events) = navigation(); connected(&nav);
        gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
        save_plan(&nav, PreflightPlan { expected_collection: "Show".into(), check_microphone: false, minimum_disk_mib: None, manual_items: vec!["A long manual checklist item with <b>literal markup</b> and enough text to wrap on a narrow window".into()], ..Default::default() });
        let open = button(nav.clone()); let parent = adw::Window::builder().default_width(360).default_height(600).build(); parent.set_content(Some(&open)); parent.present();
        assert!(open.measure(gtk4::Orientation::Horizontal, -1).0 <= 360);
        open.emit_clicked(); let report = window("preflight-review"); report.set_default_size(360, 600); pump(Duration::from_millis(80));
        assert!(report.measure(gtk4::Orientation::Horizontal, -1).0 <= 360, "review minimum width {}", report.measure(gtk4::Orientation::Horizontal, -1).0);
        let manual = find(&report, "preflight-manual-0").downcast::<CheckButton>().unwrap(); manual.grab_focus();
        nav.state.borrow_mut().workspace.observed_at = Some(Instant::now()); pump(Duration::from_millis(300));
        assert_eq!(find(&report, "preflight-manual-0"), manual.clone().upcast::<gtk4::Widget>()); assert!(manual.has_focus());
        click(&report, "preflight-configure"); let editor = window("preflight-editor"); editor.set_default_size(360, 600); pump(Duration::from_millis(80));
        assert!(editor.measure(gtk4::Orientation::Horizontal, -1).0 <= 360, "editor minimum width {}", editor.measure(gtk4::Orientation::Horizontal, -1).0);
        assert!(find(&editor, "preflight-save").is_sensitive()); editor.close(); report.close(); parent.close();
    });
}
