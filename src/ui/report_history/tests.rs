use super::*;
use crate::ui::{
    preflight::tests::{find, window},
    test_support::{native_test, navigation, pump, pump_until},
};
use crate::{
    controller::{event::ConnectionInfo, state::ObsStatus},
    domain::{session_report::SessionReport, workflow::ActivityOrigin},
    storage::report_history::{read, write},
};

fn connect(nav: &NavigationContext, name: &str) {
    let mut state = nav.state.borrow_mut();
    state.apply_connected(ConnectionInfo {
        connection_id: name.into(),
        endpoint: "fixture:4455".into(),
        obs_version: "fixture".into(),
        websocket_version: "5".into(),
        available_requests: Vec::new(),
        supported_image_formats: Vec::new(),
    });
    state.scene_collections.current = Some("Podcast".into());
    state.collect_report(Instant::now(), crate::controller::workspace::unix_ms());
}

fn done(nav: &NavigationContext) {
    assert!(pump_until(Duration::from_secs(5), || {
        let state = nav.state.borrow();
        !state.report_history.busy && !state.report_history.settings_pending
    }));
    pump(Duration::from_millis(270));
}

fn click(root: &impl IsA<gtk4::Widget>, name: &str) {
    find(root, name)
        .downcast::<gtk4::Button>()
        .unwrap()
        .emit_clicked();
}

#[test]
#[ignore = "requires isolated GTK and temporary XDG history files"]
fn native_history_opt_in_retention_reload_clear_failures_and_large_text() {
    native_test("ui::report_history::tests::native_history_opt_in_retention_reload_clear_failures_and_large_text", || {
        crate::infra::i18n::init(crate::domain::appearance::Language::Uk);
        gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
        let (nav, stack, _runtime, events) = navigation(); connect(&nav, "First");
        let path = crate::storage::xdg::report_history_path(); tick(&nav.state); assert!(!path.exists());
        let parent = adw::Window::builder().default_width(360).default_height(600).build(); parent.set_content(Some(&stack)); parent.present();
        show(parent.upcast_ref(), &nav); let dialog = window("history-window"); dialog.set_default_size(360, 600);
        let enabled = find(&dialog, "history-enable").downcast::<gtk4::CheckButton>().unwrap();
        let retain = find(&dialog, "history-retain").downcast::<gtk4::SpinButton>().unwrap();
        enabled.set_active(true); retain.set_value(2.0); click(&dialog, "history-apply"); done(&nav);
        assert_eq!(read(&path).unwrap().entries.len(), 1);
        let persisted: crate::storage::config::AppConfig = serde_json::from_slice(&std::fs::read(crate::storage::xdg::config_path()).unwrap()).unwrap();
        assert!(persisted.session_history.enabled); assert_eq!(persisted.session_history.limit(), 2);

        let original = path.with_extension("backup"); std::fs::rename(&path, &original).unwrap(); std::fs::create_dir(&path).unwrap();
        nav.state.borrow_mut().workspace.log(ActivityOrigin::Observed, "Retain this after failure".into());
        save(&nav.state, true, false); done(&nav);
        assert!(nav.state.borrow().report_history.save_error.is_some());
        assert_eq!(FAILURES.with(|failures| failures.borrow().len()), 1);
        assert!(find(&dialog, "history-status").downcast::<gtk4::Label>().unwrap().text().contains(nav.state.borrow().report_history.save_error.as_deref().unwrap()));
        std::fs::remove_dir(&path).unwrap(); std::fs::rename(&original, &path).unwrap();
        click(&dialog, "history-retry"); done(&nav);
        assert!(nav.state.borrow().report_history.save_error.is_none()); assert!(FAILURES.with(|failures| failures.borrow().is_empty()));
        assert!(serde_json::to_string(&read(&path).unwrap()).unwrap().contains("Retain this after failure"));

        enabled.set_active(false); click(&dialog, "history-apply"); done(&nav); let before = std::fs::read(&path).unwrap();
        nav.state.borrow_mut().workspace.log(ActivityOrigin::Observed, "Disabled".into());
        save(&nav.state, true, false); tick(&nav.state); done(&nav); assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut restarted = AppState::new(Default::default(), Default::default(), None, None); restarted.load_report_history(Ok(read(&path).unwrap()));
        assert!(restarted.report_list()[0].interrupted); assert!(restarted.report_list()[0].ended_unix_ms.is_none());

        enabled.set_active(true); click(&dialog, "history-apply"); done(&nav);
        for name in ["Second", "Third"] { nav.state.borrow_mut().reset_obs_session(ObsStatus::Disconnected); connect(&nav, name); }
        save(&nav.state, true, false); done(&nav);
        assert_eq!(read(&path).unwrap().entries.iter().map(|report| report.connection_id.as_str()).collect::<Vec<_>>(), ["Second", "Third"]);
        nav.state.borrow_mut().capture_bookmark("Keep this bookmark", Instant::now(), 10).unwrap();
        click(&dialog, "history-clear"); window("history-clear-confirm").downcast::<adw::MessageDialog>().unwrap().response("cancel"); done(&nav);
        assert_eq!(read(&path).unwrap().entries.len(), 2);
        click(&dialog, "history-clear"); window("history-clear-confirm").downcast::<adw::MessageDialog>().unwrap().response("clear"); done(&nav);
        assert!(read(&path).unwrap().entries.is_empty()); tick(&nav.state); done(&nav); assert!(read(&path).unwrap().entries.is_empty());
        assert_eq!(nav.state.borrow().bookmarks.archive.entries[0].name, "Keep this bookmark");

        let config_path = crate::storage::xdg::config_path(); let config_backup = config_path.with_extension("backup");
        std::fs::rename(&config_path, &config_backup).unwrap(); std::fs::create_dir(&config_path).unwrap();
        enabled.set_active(false); click(&dialog, "history-apply"); done(&nav);
        assert!(nav.state.borrow().report_history.settings_error.is_some()); assert_eq!(SETTINGS_FAILURES.with(|failures| failures.borrow().len()), 1);
        std::fs::remove_dir(&config_path).unwrap(); std::fs::rename(&config_backup, &config_path).unwrap(); click(&dialog, "history-retry"); done(&nav);
        assert!(nav.state.borrow().report_history.settings_error.is_none()); assert!(SETTINGS_FAILURES.with(|failures| failures.borrow().is_empty()));
        assert!(dialog.measure(gtk4::Orientation::Horizontal, -1).0 <= 360); assert!(dialog.measure(gtk4::Orientation::Vertical, 360).0 <= 600);
        let apply = find(&dialog, "history-apply").downcast::<gtk4::Button>().unwrap(); dialog.close(); enabled.set_active(true); apply.emit_clicked();
        assert!(!nav.state.borrow().config.session_history.enabled); assert!(events.try_iter().next().is_none()); parent.close();
    });
}

#[test]
#[ignore = "requires isolated GTK and temporary XDG history files"]
fn native_history_shutdown_chains_latest_snapshot_and_conflict_retry_preserves_other_sessions() {
    native_test("ui::report_history::tests::native_history_shutdown_chains_latest_snapshot_and_conflict_retry_preserves_other_sessions", || {
        let (nav, _stack, _runtime, _events) = navigation(); register(&nav.state); nav.state.borrow_mut().configure_report_history(true, 3); connect(&nav, "First");
        let (release, gate) = std::sync::mpsc::channel();
        background_io::run_serialized(SerialLane::ReportHistory, move || gate.recv_timeout(Duration::from_secs(5)).unwrap(), |()| {});
        save(&nav.state, true, false); assert!(nav.state.borrow().report_history.busy);
        {
            let mut state = nav.state.borrow_mut(); state.capture_bookmark("Final note", Instant::now(), 20).unwrap();
            let scope = state.workflow_scope(); state.workspace.output_results.request(crate::domain::output_result::OutputResultKind::ReplaySave, scope, 21);
        }
        finish_for_shutdown(); assert!(nav.state.borrow().report_history.force_after_pending); release.send(()).unwrap();
        let drained = Rc::new(Cell::new(false)); background_io::flush_pending(Duration::from_secs(5), { let drained = drained.clone(); move |result| { result.unwrap(); drained.set(true); } });
        assert!(pump_until(Duration::from_secs(6), || drained.get()));
        let path = crate::storage::xdg::report_history_path(); let saved = read(&path).unwrap();
        assert!(saved.entries[0].ended_unix_ms.is_some()); assert_eq!(saved.entries[0].bookmark_ids, [1]);
        assert!(saved.entries[0].events.iter().any(|record| matches!(&record.event, crate::domain::session_report::ReportEvent::Output(result) if result.status == crate::domain::output_result::OutputResultStatus::Unconfirmed)));
        assert!(!serde_json::to_string(&saved).unwrap().contains("Final note"), "disk history stores only bookmark references");

        let mut external = saved.clone(); external.upsert(SessionReport::new(999, 40, "Another app".into(), "other:4455".into())).unwrap();
        write(&path, &saved, &external).unwrap(); connect(&nav, "Second"); save(&nav.state, true, false); done(&nav);
        assert!(nav.state.borrow().report_history.save_error.is_some()); assert_eq!(read(&path).unwrap(), external);
        reload(&nav.state); done(&nav);
        let merged = read(&path).unwrap(); assert!(merged.entries.iter().any(|report| report.connection_id == "Another app")); assert!(merged.entries.iter().any(|report| report.connection_id == "Second"));
        assert!(nav.state.borrow().report_history.save_error.is_none()); assert!(FAILURES.with(|failures| failures.borrow().is_empty()));
    });
}
