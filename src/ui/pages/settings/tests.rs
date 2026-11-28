use std::{cell::RefCell, rc::Rc, sync::Arc};

use crate::controller::test_support::deps::NoPassword;

#[test]
#[ignore = "requires a disposable GTK display; no keyring or OBS access"]
fn native_settings_constructor_releases_entries_and_hotkey_rows() {
    crate::ui::native_test!(|| {
        use adw::prelude::*;
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        let (page, refresh) = super::build(&nav, Arc::new(NoPassword));
        fn rows(widget: &gtk4::Widget, tracked: &mut Vec<glib::WeakRef<gtk4::Widget>>) {
            if widget.is::<adw::EntryRow>()
                || widget.is::<adw::ComboRow>()
                || widget.is::<adw::SpinRow>()
            {
                tracked.push(widget.downgrade());
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                child = widget.next_sibling();
                rows(&widget, tracked);
            }
        }
        let mut tracked = Vec::new();
        rows(&page, &mut tracked);
        assert!(
            tracked.len() >= 10,
            "track real host/port and hotkey descendants"
        );
        let weak = page.downgrade();
        drop(page);
        refresh(); // A retained public refresher must not own descendants.
        assert!(weak.upgrade().is_none(), "Settings root retained");
        assert!(
            tracked.iter().all(|row| row.upgrade().is_none()),
            "Settings entries/hotkey rows retained"
        );
    });
}

#[test]
#[ignore = "requires a native GTK display; no keyring or OBS access"]
fn native_settings_rows_render_external_status_and_notice_literally() {
    if !crate::ui::background_io::isolated_test_child("ui::pages::settings::tests::native_settings_rows_render_external_status_and_notice_literally") { return; }
    use crate::controller::{
        app_controller::AppController,
        state::{AppState, ObsStatus},
    };
    use adw::prelude::*;
    crate::ui::test_support::require_native_runner();
    adw::init().expect("native GTK display");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let (tx, _events) = std::sync::mpsc::sync_channel(8);
    let payload = "<a href=\"https://example.invalid\">OBS & path <b>literal</b></a>";
    let state = Rc::new(RefCell::new(AppState::new(
        crate::storage::config::AppConfig::default(),
        crate::storage::registry::SceneRegistry::default(),
        vec![payload.into()],
    )));
    state.borrow_mut().obs_status = ObsStatus::Error(payload.into());
    let nav = crate::ui::navigation::NavigationContext::new(
        state,
        gtk4::Stack::new(),
        Rc::new(RefCell::new(AppController::new(
            runtime.handle().clone(),
            tx,
        ))),
    );
    let (page, _) = super::build(&nav, Arc::new(NoPassword));
    fn visit(widget: &gtk4::Widget, payload: &str, literal_count: &mut usize) {
        if let Some(row) = widget.downcast_ref::<adw::PreferencesRow>() {
            assert!(
                !row.uses_markup(),
                "settings row must be literal: {} ({})",
                row.title(),
                widget.type_().name()
            );
        }
        if let Some(label) = widget.downcast_ref::<gtk4::Label>() {
            if label.text().contains(payload) {
                assert!(!label.uses_markup());
                *literal_count += 1;
            }
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            visit(&widget, payload, literal_count);
            child = widget.next_sibling();
        }
    }
    let mut literal_count = 0;
    visit(&page, payload, &mut literal_count);
    assert!(
        literal_count >= 2,
        "notice and external status must retain the exact payload"
    );
}
