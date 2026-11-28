use super::*;
use std::cell::Cell;
use std::time::{Duration, Instant};

fn iterate_until(mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let context = glib::MainContext::default();
    while !done() {
        assert!(
            Instant::now() < deadline,
            "GTK completion deadline exceeded"
        );
        context.iteration(false);
        std::thread::yield_now();
    }
}

fn fixture(state: std::rc::Weak<RefCell<AppState>>) -> (adw::Application, adw::ApplicationWindow) {
    let app = adw::Application::builder()
        .application_id("io.github.worxbend.scenedeck.CloseFixture")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(None::<&gio::Cancellable>).unwrap();
    let window = adw::ApplicationWindow::builder().application(&app).build();
    install_persistence_close_guard(&app, &window, state);
    window.present();
    // First-frame shader/font setup on a disposable software-rendered
    // display can outlast the close assertion's two-second deadline.
    // Establish a painted window before measuring persistence completion.
    let painted = Rc::new(Cell::new(false));
    let clock = window.frame_clock().expect("presented fixture frame clock");
    let handler = clock.connect_after_paint({
        let painted = painted.clone();
        move |_| painted.set(true)
    });
    window.queue_draw();
    assert!(
        ui::test_support::pump_until(Duration::from_secs(20), || painted.get()),
        "fixture did not paint its first frame"
    );
    clock.disconnect(handler);
    (app, window)
}

fn inventory_role_selector(widget: &gtk4::Widget) -> Option<gtk4::DropDown> {
    if let Some(selector) = widget.downcast_ref::<gtk4::DropDown>() {
        return Some(selector.clone());
    }
    let mut child = widget.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        if let Some(selector) = inventory_role_selector(&widget) {
            return Some(selector);
        }
    }
    None
}

fn set_inventory_role(page: &gtk4::Widget, role: crate::domain::role::SceneRole) {
    iterate_until(|| inventory_role_selector(page).is_some());
    let selector = inventory_role_selector(page).expect("production Inventory role selector");
    let index = crate::domain::role::SceneRole::ALL
        .iter()
        .position(|candidate| *candidate == role)
        .unwrap() as u32
        + 1;
    selector.set_selected(index);
}

fn assert_saved_inventory_role() {
    let registry =
        crate::storage::registry::read_registry_from_path(&crate::storage::xdg::registry_path())
            .unwrap();
    assert_eq!(
        registry.scenes.get("Fixture").and_then(|entry| entry.role),
        Some(crate::domain::role::SceneRole::Secondary)
    );
}

fn isolated_config_dir(suffix: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "scenedeck-persistence-{suffix}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &path);
    path
}

fn connect_report(state: &mut AppState) {
    state.apply_connected(crate::controller::event::ConnectionInfo {
        connection_id: "report-fixture".into(),
        endpoint: "fixture:4455".into(),
        obs_version: "fixture".into(),
        websocket_version: "5".into(),
        available_requests: Vec::new(),
        supported_image_formats: Vec::new(),
    });
}

fn close_failure() -> Option<adw::MessageDialog> {
    gtk4::Window::list_toplevels()
        .into_iter()
        .find_map(|window| {
            window
                .downcast::<adw::MessageDialog>()
                .ok()
                .filter(|dialog| {
                    dialog.is_visible() && dialog.widget_name() == "persistence-close-failure"
                })
        })
}

#[test]
#[ignore = "requires isolated GTK and scratch history; no OBS/keyring"]
fn native_last_detached_report_close_supports_cancel_retry_and_informed_abandon() {
    crate::ui::native_test!(|| {
        let (app, main) = fixture(std::rc::Weak::new());
        let (nav, _stack, _runtime, _events) = ui::test_support::navigation();
        ui::report_history::register(&nav.state);
        nav.state.borrow_mut().configure_report_history(true, 8);
        connect_report(&mut nav.state.borrow_mut());
        let detached = adw::Window::builder().application(&app).build();
        detached.present();
        main.close();
        iterate_until(|| !main.is_visible());
        assert!(nav
            .state
            .borrow()
            .reports
            .entries
            .back()
            .unwrap()
            .ended_unix_ms
            .is_none());
        let path = crate::storage::xdg::report_history_path();
        let backup = path.with_extension("backup");
        std::fs::rename(&path, &backup).unwrap();
        std::fs::create_dir(&path).unwrap();
        detached.close();
        iterate_until(|| close_failure().is_some());
        assert!(detached.is_visible() && detached.is_sensitive());
        assert!(
            nav.state
                .borrow()
                .reports
                .entries
                .back()
                .unwrap()
                .ended_unix_ms
                .is_none(),
            "failed exit resumes reporting"
        );
        close_failure().unwrap().response("cancel");
        assert!(detached.is_visible());
        nav.state.borrow_mut().workspace.log(
            crate::domain::workflow::ActivityOrigin::Observed,
            "after cancelled exit".into(),
        );
        detached.close();
        iterate_until(|| close_failure().is_some());
        std::fs::remove_dir(&path).unwrap();
        std::fs::rename(&backup, &path).unwrap();
        close_failure().unwrap().response("retry");
        iterate_until(|| !detached.is_visible());
        let saved = crate::storage::report_history::read(&path).unwrap();
        assert!(saved.entries[0].ended_unix_ms.is_some());
        assert!(serde_json::to_string(&saved)
            .unwrap()
            .contains("after cancelled exit"));

        // A corrupt startup file with new local history needs an informed exit,
        // even though no write can be admitted until it is reloaded/repaired.
        let detached = adw::Window::builder().application(&app).build();
        detached.present();
        nav.state
            .borrow_mut()
            .reset_obs_session(crate::controller::state::ObsStatus::Disconnected);
        connect_report(&mut nav.state.borrow_mut());
        std::fs::write(&path, b"broken history").unwrap();
        nav.state
            .borrow_mut()
            .load_report_history(Err("fixture corrupt startup history".into()));
        detached.close();
        iterate_until(|| close_failure().is_some());
        assert!(detached.is_visible());
        close_failure().unwrap().response("abandon");
        iterate_until(|| !detached.is_visible());
        assert_eq!(std::fs::read(&path).unwrap(), b"broken history");
    });
}

fn final_stats_exit(quit: bool) {
    use crate::controller::{
        dependencies::ControllerDependencies,
        test_support::deps::{NoPassword, NoServices},
    };
    fn find(
        widget: &gtk4::Widget,
        predicate: &impl Fn(&gtk4::Widget) -> bool,
    ) -> Option<gtk4::Widget> {
        if predicate(widget) {
            return Some(widget.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if let Some(found) = find(&widget, predicate) {
                return Some(found);
            }
        }
        None
    }
    let app = adw::Application::builder()
        .application_id("io.github.worxbend.scenedeck.ReportExitFixture")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let weak_state = Rc::new(RefCell::new(std::rc::Weak::<RefCell<AppState>>::new()));
    let completed = Rc::new(Cell::new(false));
    app.connect_activate({
        let runtime = runtime.handle().clone();
        let weak_state = weak_state.clone();
        let completed = completed.clone();
        move |app| {
            let (tx, rx) = mpsc::sync_channel(128);
            let controller = Rc::new(RefCell::new(AppController::with_dependencies(runtime.clone(), tx.clone(), ControllerDependencies::new(std::sync::Arc::new(NoServices), std::sync::Arc::new(NoPassword)))));
            let mut initial = crate::controller::test_support::state::app_state();
            initial.config.onboarding.welcome_shown = true;
            initial.configure_report_history(true, 8);
            let state = Rc::new(RefCell::new(initial));
            *weak_state.borrow_mut() = Rc::downgrade(&state);
            let window = ui::window::build_main_window(app, state.clone(), controller, rx);
            install_persistence_close_guard(app, &window, Rc::downgrade(&state));
            connect_report(&mut state.borrow_mut());
            let stack = find(window.upcast_ref(), &|w| w.has_css_class("scenedeck-content-stack")).unwrap().downcast::<gtk4::Stack>().unwrap();
            stack.set_visible_child_name(crate::controller::state::Page::Stats.id());
            let stats = stack.child_by_name(crate::controller::state::Page::Stats.id()).unwrap();
            find(&stats, &|w| w.downcast_ref::<gtk4::Button>().is_some_and(|b| b.icon_name().as_deref() == Some("window-new-symbolic"))).unwrap().downcast::<gtk4::Button>().unwrap().emit_clicked();
            let popout = document_windows(app).into_iter().find(|w| w != window.upcast_ref::<gtk4::Window>()).unwrap();
            drop((stack, stats));
            window.close();
            assert!(ui::test_support::pump_until(Duration::from_secs(20), || !window.is_visible()));
            assert!(state.borrow().reports.entries.back().unwrap().ended_unix_ms.is_none());
            let weak_window = window.downgrade();
            drop(window);
            assert!(ui::test_support::pump_until(Duration::from_secs(3), || weak_window.upgrade().is_none()));
            {
                let mut state = state.borrow_mut();
                state.capture_bookmark("final note", Instant::now(), 20).unwrap();
                let scope = state.workflow_scope();
                state.workspace.output_results.request(crate::domain::output_result::OutputResultKind::ReplaySave, scope, 21);
            }
            drop(state);
            let (release, gate) = mpsc::channel();
            ui::background_io::run_serialized(ui::background_io::SerialLane::ReportHistory, move || gate.recv_timeout(Duration::from_secs(5)).unwrap(), |()| {});
            if quit { app.activate_action("quit", None); } else { popout.close(); }
            assert!(popout.is_visible() && !popout.is_sensitive());
            assert!(weak_state.borrow().upgrade().is_some());
            // Keep the real application's receiver open until last-window removal.
            let app_weak = app.downgrade();
            let completed = completed.clone();
            app.connect_shutdown(move |_| {
                let saved = crate::storage::report_history::read(&crate::storage::xdg::report_history_path()).unwrap();
                assert_eq!(saved.entries.len(), 1);
                assert!(saved.entries[0].ended_unix_ms.is_some());
                assert_eq!(saved.entries[0].bookmark_ids, [1]);
                assert!(saved.entries[0].events.iter().any(|record| matches!(&record.event, crate::domain::session_report::ReportEvent::Output(result) if result.status == crate::domain::output_result::OutputResultStatus::Unconfirmed)));
                assert!(app_weak.upgrade().is_some());
                let _ = &tx;
                completed.set(true);
            });
            release.send(()).unwrap();
        }
    });
    assert_eq!(app.run_with_args::<&str>(&[]), glib::ExitCode::SUCCESS);
    assert!(completed.get());
    // GtkApplication::quit stops the loop without destroying its windows.
    for window in app.windows() {
        window.destroy();
    }
    drop(app);
    assert!(
        ui::test_support::pump_until(Duration::from_secs(3), || weak_state
            .borrow()
            .upgrade()
            .is_none()),
        "last document/save must release application state"
    );
}

#[test]
#[ignore = "requires isolated GTK and scratch history; no OBS/keyring"]
fn native_last_stats_close_saves_final_report_before_state_release() {
    crate::ui::native_test!(|| final_stats_exit(false));
}

#[test]
#[ignore = "requires isolated GTK and scratch history; no OBS/keyring"]
fn native_quit_after_shell_close_saves_final_stats_report_before_shutdown() {
    crate::ui::native_test!(|| final_stats_exit(true));
}

#[test]
#[ignore = "requires a disposable GTK display; injected Config write failure only"]
fn native_config_write_error_requires_an_explicit_close_decision() {
    crate::ui::native_test!(|| {
        let (_app, window) = fixture(std::rc::Weak::new());
        ui::background_io::run_config_snapshot(|| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "fixture Config write refused",
            ))
        });
        let visible_dialog = || {
            gtk4::Window::list_toplevels()
                .into_iter()
                .find_map(|window| {
                    window
                        .downcast::<adw::MessageDialog>()
                        .ok()
                        .filter(|dialog| dialog.is_visible())
                })
        };
        window.close();
        iterate_until(|| !window.is_visible() || visible_dialog().is_some());
        assert!(
            window.is_visible(),
            "returned save errors must not silently close"
        );
        let dialog = visible_dialog().expect("failed Config save decision");
        assert!(dialog.has_response("abandon"));
        dialog.response("cancel");
        assert!(window.is_visible() && window.is_sensitive());
        window.close();
        iterate_until(|| visible_dialog().is_some());
        let dialog = visible_dialog().unwrap();
        assert!(window.is_visible(), "Cancel must retain failure evidence");
        dialog.response("abandon");
        iterate_until(|| !window.is_visible());
    });
}

#[test]
#[ignore = "requires a disposable GTK display; scratch config only, no OBS/keyring"]
fn native_inventory_role_edit_and_rebuild_are_admitted_before_close() {
    crate::ui::native_test!(|| {
        let directory = isolated_config_dir("close");
        let (_app, window) = fixture(std::rc::Weak::new());
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        nav.state
            .borrow_mut()
            .scene_inventory
            .scenes
            .push(crate::domain::scene::Scene {
                uuid: None,
                id: "Fixture".into(),
            });
        let (page, refresh) = ui::pages::inventory::build(nav);
        window.set_content(Some(&page));
        set_inventory_role(&page, crate::domain::role::SceneRole::Primary);
        refresh(); // Rebuilding rows must not lose an accepted registry edit.
        set_inventory_role(&page, crate::domain::role::SceneRole::Secondary);
        window.close(); // No 300 ms debounce wait before initiating Close.
        assert!(window.is_visible());
        iterate_until(|| !window.is_visible());
        assert_saved_inventory_role();
        std::fs::remove_dir_all(directory).unwrap();
    });
}

#[test]
#[ignore = "requires a disposable GTK display; scratch config only, no OBS/keyring"]
fn native_inventory_role_edit_is_admitted_before_quit_action() {
    crate::ui::native_test!(|| {
        let directory = isolated_config_dir("quit");
        let app = adw::Application::builder()
            .application_id("io.github.worxbend.scenedeck.RoleQuitFixture")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        nav.state
            .borrow_mut()
            .scene_inventory
            .scenes
            .push(crate::domain::scene::Scene {
                uuid: None,
                id: "Fixture".into(),
            });
        let guard_dialog = Rc::new(RefCell::new(None::<String>));
        app.connect_activate({
            let guard_dialog = guard_dialog.clone();
            move |app| {
                // Keep an interactive guard outcome distinct from a blocked main loop.
                // The assertion after run still fails; this never retries a save silently.
                let app_weak = app.downgrade();
                let guard_dialog = guard_dialog.clone();
                glib::timeout_add_local(Duration::from_millis(20), move || {
                    let Some(app) = app_weak.upgrade() else {
                        return glib::ControlFlow::Break;
                    };
                    if let Some(dialog) =
                        gtk4::Window::list_toplevels()
                            .into_iter()
                            .find_map(|window| {
                                window
                                    .downcast::<adw::MessageDialog>()
                                    .ok()
                                    .filter(|dialog| dialog.is_visible())
                            })
                    {
                        *guard_dialog.borrow_mut() = Some(dialog.body().to_string());
                        app.quit();
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
                let (page, refresh) = ui::pages::inventory::build(nav.clone());
                let window = adw::ApplicationWindow::builder()
                    .application(app)
                    .content(&page)
                    .build();
                install_persistence_close_guard(app, &window, Rc::downgrade(&nav.state));
                window.present();
                set_inventory_role(&page, crate::domain::role::SceneRole::Primary);
                refresh();
                set_inventory_role(&page, crate::domain::role::SceneRole::Secondary);
                app.activate_action("quit", None);
                assert!(window.is_visible() && !window.is_sensitive());
            }
        });
        assert_eq!(app.run_with_args::<&str>(&[]), glib::ExitCode::SUCCESS);
        assert!(
            guard_dialog.borrow().is_none(),
            "Quit reached a persistence decision rather than deadlocking: {:?}",
            guard_dialog.borrow()
        );
        assert_saved_inventory_role();
        std::fs::remove_dir_all(directory).unwrap();
    });
}

#[test]
#[ignore = "requires a disposable GTK display; deliberately holds the registry writer past the save deadline"]
fn native_quit_timeout_preserves_role_edits_until_explicit_retry() {
    crate::ui::native_test!(|| {
        let directory = isolated_config_dir("quit-timeout");
        let app = adw::Application::builder()
            .application_id("io.github.worxbend.scenedeck.RoleTimeoutFixture")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        nav.state
            .borrow_mut()
            .scene_inventory
            .scenes
            .push(crate::domain::scene::Scene {
                uuid: None,
                id: "Fixture".into(),
            });
        let timed_out = Rc::new(Cell::new(false));
        let gate_published = Rc::new(Cell::new(false));
        let saves_published = Rc::new(Cell::new(false));
        let explicit_retry_sent = Rc::new(Cell::new(false));
        app.connect_activate({
            let timed_out = timed_out.clone();
            let gate_published = gate_published.clone();
            let saves_published = saves_published.clone();
            let explicit_retry_sent = explicit_retry_sent.clone();
            move |app| {
                let (release, held) = std::sync::mpsc::channel();
                ui::background_io::run_serialized(
                    ui::background_io::SerialLane::Registry,
                    move || held.recv().unwrap(),
                    {
                        let gate_published = gate_published.clone();
                        move |()| gate_published.set(true)
                    },
                );
                let (page, refresh) = ui::pages::inventory::build(nav.clone());
                let window = adw::ApplicationWindow::builder()
                    .application(app)
                    .content(&page)
                    .build();
                install_persistence_close_guard(app, &window, Rc::downgrade(&nav.state));
                window.present();
                set_inventory_role(&page, crate::domain::role::SceneRole::Primary);
                refresh();
                set_inventory_role(&page, crate::domain::role::SceneRole::Secondary);
                let deadline = Instant::now() + Duration::from_secs(15);
                let timed_out = timed_out.clone();
                let gate_published = gate_published.clone();
                let saves_published = saves_published.clone();
                let explicit_retry_sent = explicit_retry_sent.clone();
                let observed_window = window.clone();
                let mut release = Some(release);
                glib::timeout_add_local(Duration::from_millis(20), move || {
                    assert!(
                        Instant::now() < deadline,
                        "save timeout decision was not presented"
                    );
                    let dialog = gtk4::Window::list_toplevels()
                        .into_iter()
                        .find_map(|window| {
                            window
                                .downcast::<adw::MessageDialog>()
                                .ok()
                                .filter(|dialog| dialog.is_visible())
                        });
                    let Some(dialog) = dialog else {
                        return glib::ControlFlow::Continue;
                    };
                    assert_eq!(
                        dialog.body().as_str(),
                        i18n_embed_fl::fl!(
                            crate::infra::i18n::LANGUAGE_LOADER,
                            "app-persistence-drain-timeout"
                        )
                    );
                    assert!(observed_window.is_visible() && observed_window.is_sensitive());
                    assert!(
                        !gate_published.get(),
                        "blocked save must not publish success"
                    );
                    assert!(
                        !dialog.has_response("abandon"),
                        "a pending save cannot be abandoned as a finished failure"
                    );
                    timed_out.set(true);
                    release.take().unwrap().send(()).unwrap();
                    // Keep this decision open until real writes AND GTK publication
                    // finish; an immediate Retry would start another storage deadline.
                    ui::background_io::flush_pending(Duration::from_secs(15), {
                        let observed_window = observed_window.clone();
                        let gate_published = gate_published.clone();
                        let saves_published = saves_published.clone();
                        let explicit_retry_sent = explicit_retry_sent.clone();
                        move |outcome| {
                            assert_eq!(outcome, Ok(()), "accepted role saves must publish");
                            assert!(gate_published.get());
                            saves_published.set(true);
                            assert!(observed_window.is_visible() && observed_window.is_sensitive());
                            assert!(
                                dialog.is_visible(),
                                "late publication must not answer the decision"
                            );
                            explicit_retry_sent.set(true);
                            dialog.response("retry");
                            assert!(
                                !observed_window.is_sensitive(),
                                "explicit Retry must re-enter the save barrier"
                            );
                        }
                    });
                    glib::ControlFlow::Break
                });
                app.activate_action("quit", None);
                assert!(window.is_visible() && !window.is_sensitive());
            }
        });
        assert_eq!(app.run_with_args::<&str>(&[]), glib::ExitCode::SUCCESS);
        assert!(
            timed_out.get(),
            "the actual five-second save deadline must be exercised"
        );
        assert!(
            gate_published.get() && saves_published.get(),
            "Quit must follow all accepted-save publication"
        );
        assert!(
            explicit_retry_sent.get(),
            "late persistence cannot substitute for an explicit Retry"
        );
        assert_saved_inventory_role();
        std::fs::remove_dir_all(directory).unwrap();
    });
}

#[test]
#[ignore = "requires a disposable GTK display; scratch registry only, no OBS/keyring"]
fn native_same_value_registry_edits_retry_failed_disk_writes() {
    crate::ui::native_test!(|| {
        let directory = isolated_config_dir("same-value-registry-retry");
        let target = crate::storage::xdg::registry_path();
        let parent = target.parent().unwrap();
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        for field_write in [false, true] {
            std::fs::write(parent, b"blocked directory").unwrap();
            let edit = || {
                if field_write {
                    ui::persist::persist_registry_field(
                        &nav,
                        "input icon",
                        |registry| registry.set_input_icon("Mic", Some("nf-md-microphone")),
                        |registry| registry.set_input_icon("Mic", Some("nf-md-microphone")),
                    );
                } else {
                    ui::persist::persist_registry(&nav, "scene icon", |registry| {
                        registry.set_scene_icon("Camera", Some("nf-md-camera"))
                    });
                }
            };
            edit();
            let failed = Rc::new(Cell::new(false));
            ui::background_io::flush_pending(Duration::from_secs(5), {
                let failed = failed.clone();
                move |result| {
                    assert_eq!(result, Err(ui::background_io::FlushError::WorkerFailed));
                    failed.set(true);
                }
            });
            iterate_until(|| failed.get());
            let (page, _refresh) = ui::pages::inventory::build(nav.clone());
            let status = page.first_child().expect("registry status row");
            assert_eq!(status.widget_name(), "inventory-registry-save-status");
            assert!(status.is_visible());
            let ui::persist::RegistrySaveStatus::Failed(detail) =
                ui::persist::registry_save_status()
            else {
                panic!("failed status must retain error detail");
            };
            assert_eq!(
                status
                    .first_child()
                    .unwrap()
                    .downcast::<gtk4::Label>()
                    .unwrap()
                    .text(),
                i18n_embed_fl::fl!(
                    crate::infra::i18n::LANGUAGE_LOADER,
                    "settings-failed-to-save",
                    err = detail
                )
            );
            std::fs::remove_file(parent).unwrap();
            std::fs::create_dir(parent).unwrap();
            edit(); // Cached value is identical: this must still admit a retry.
            let saved = Rc::new(Cell::new(false));
            ui::background_io::flush_pending(Duration::from_secs(5), {
                let saved = saved.clone();
                move |result| {
                    assert_eq!(result, Ok(()));
                    saved.set(true);
                }
            });
            iterate_until(|| saved.get());
            assert_eq!(
                ui::persist::registry_save_status(),
                ui::persist::RegistrySaveStatus::Saved
            );
            let disk = crate::storage::registry::read_registry_from_path(&target).unwrap();
            if field_write {
                assert_eq!(disk.input_icon("Mic"), Some("nf-md-microphone"));
            } else {
                assert_eq!(disk.scene_icon("Camera"), Some("nf-md-camera"));
            }
            std::fs::remove_dir_all(parent).unwrap();
        }
        std::fs::remove_dir_all(directory).unwrap();
    });
}

#[test]
#[ignore = "requires a disposable GTK display; scratch bookmarks only, no OBS/keyring"]
fn native_bookmark_write_error_close_retry_recovers() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = ui::test_support::navigation();
        {
            let mut state = nav.state.borrow_mut();
            connect_report(&mut state);
            state.scene_collections.current = Some("Interview".into());
            state.scene_inventory.current_id = Some("Camera".into());
        }
        let (_app, window) = fixture(Rc::downgrade(&nav.state));
        let live = ui::pages::live::build(nav.clone());
        window.set_content(Some(&live.root));
        let path = crate::storage::xdg::bookmarks_path();
        std::fs::create_dir_all(&path).unwrap();
        ui::test_support::expect_named::<gtk4::Button>(window.upcast_ref(), "bookmarks-open")
            .emit_clicked();
        let bookmarks = gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|window| {
                window
                    .downcast::<gtk4::Window>()
                    .ok()
                    .filter(|window| window.widget_name() == "bookmarks-window")
            })
            .expect("production Bookmarks window");
        ui::test_support::expect_named::<gtk4::Entry>(bookmarks.upcast_ref(), "bookmark-name")
            .set_text("Keep after close");
        ui::test_support::expect_named::<gtk4::Button>(bookmarks.upcast_ref(), "bookmark-add")
            .emit_clicked();
        iterate_until(|| nav.state.borrow().bookmarks.save_error.is_some());
        bookmarks.close();

        window.close();
        iterate_until(|| close_failure().is_some());
        assert!(window.is_visible(), "a failed bookmark save must not close");
        std::fs::remove_dir(&path).unwrap();
        close_failure().unwrap().response("retry");
        iterate_until(|| !window.is_visible());
        let saved = crate::storage::bookmarks::read(&path).unwrap();
        assert_eq!(saved.entries[0].name, "Keep after close");
    });
}

#[test]
#[ignore = "requires a disposable GTK display; scratch config only, no OBS/keyring"]
fn native_config_write_error_close_retry_recovers() {
    crate::ui::native_test!(|| {
        let (nav, _stack, _runtime, _events) = ui::test_support::navigation();
        let (_app, window) = fixture(Rc::downgrade(&nav.state));
        let path = crate::storage::xdg::config_path();
        std::fs::create_dir_all(&path).unwrap();
        ui::persist::persist_config(&nav, |config| config.onboarding.welcome_shown = true);

        window.close();
        iterate_until(|| close_failure().is_some());
        assert!(window.is_visible(), "a failed config save must not close");
        std::fs::remove_dir(&path).unwrap();
        close_failure().unwrap().response("retry");
        iterate_until(|| !window.is_visible());
        let saved = crate::storage::config::read_config_from_path(&path);
        assert!(saved.startup_notice.is_none());
        assert!(saved.config.onboarding.welcome_shown);
    });
}

#[test]
#[ignore = "requires a disposable GTK display; scratch config only, no OBS/keyring"]
fn native_startup_saves_a_migrated_config_through_the_config_lane() {
    crate::ui::native_test!(|| {
        let path = crate::storage::xdg::config_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"version":2}"#).unwrap();
        let loaded = read_startup_config();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), r#"{"version":2}"#);

        persist_startup_migration(&loaded);

        iterate_until(|| {
            crate::storage::config::read_config_from_path(&path)
                .pending_repair
                .is_none()
        });
        let saved = crate::storage::config::read_config_from_path(&path);
        assert!(saved.startup_notice.is_none());
        assert!(saved.config.onboarding.welcome_shown);
    });
}

#[test]
#[ignore = "requires a disposable GTK display; scratch registry only, no OBS/keyring"]
fn native_registry_write_error_is_visible_and_close_retry_recovers() {
    crate::ui::native_test!(|| {
        let directory = isolated_config_dir("registry-retry");
        let target = crate::storage::xdg::registry_path();
        let parent = target.parent().unwrap();
        std::fs::write(parent, b"blocked directory").unwrap();
        let (_app, window) = fixture(std::rc::Weak::new());
        let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
        assert!(ui::persist::persist_registry(
            &nav,
            "fixture icon",
            |registry| { registry.set_scene_icon("Camera", Some("nf-md-camera")) }
        ));
        let failed = Rc::new(Cell::new(false));
        ui::background_io::flush_pending(Duration::from_secs(5), {
            let failed = failed.clone();
            move |result| {
                assert_eq!(result, Err(ui::background_io::FlushError::WorkerFailed));
                failed.set(true);
            }
        });
        iterate_until(|| failed.get());
        assert!(matches!(
            ui::persist::registry_save_status(),
            ui::persist::RegistrySaveStatus::Failed(_)
        ));
        window.close();
        iterate_until(|| window.is_sensitive());
        assert!(window.is_visible());
        let dialog = gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|window| {
                window
                    .downcast::<adw::MessageDialog>()
                    .ok()
                    .filter(|dialog| dialog.is_visible())
            })
            .expect("returned write error must require an informed decision");
        std::fs::remove_file(parent).unwrap();
        std::fs::create_dir(parent).unwrap();
        dialog.response("retry");
        iterate_until(|| !window.is_visible());
        assert_eq!(
            crate::storage::registry::read_registry_from_path(&target)
                .unwrap()
                .scene_icon("Camera"),
            Some("nf-md-camera")
        );
        assert_eq!(
            nav.state.borrow().registry.scene_icon("Camera"),
            Some("nf-md-camera")
        );
        std::fs::remove_dir_all(directory).unwrap();
    });
}

#[test]
#[ignore = "requires a disposable GTK display"]
fn native_close_waits_for_accepted_save_and_gtk_publication() {
    crate::ui::native_test!(|| {
        let (_app, window) = fixture(std::rc::Weak::new());
        let (release, held) = std::sync::mpsc::channel();
        let published = Rc::new(Cell::new(false));
        ui::background_io::run(move || held.recv().unwrap(), {
            let published = published.clone();
            move |()| published.set(true)
        });
        window.close();
        assert!(window.is_visible());
        assert!(!window.is_sensitive());
        assert!(!published.get());
        release.send(()).unwrap();
        iterate_until(|| !window.is_visible());
        assert!(published.get(), "close must follow GTK save publication");
    });
}

#[test]
#[ignore = "requires a disposable GTK display"]
fn native_save_worker_failure_keeps_window_open_and_offers_cancel() {
    crate::ui::native_test!(|| {
        let (_app, window) = fixture(std::rc::Weak::new());
        let failed = Rc::new(Cell::new(false));
        ui::background_io::run(
            || panic!("intentional fixture save failure"),
            |(): ()| panic!("failed worker must not publish success"),
        );
        ui::background_io::flush_pending(Duration::from_secs(1), {
            let failed = failed.clone();
            move |outcome| {
                assert_eq!(outcome, Err(ui::background_io::FlushError::WorkerFailed));
                failed.set(true);
            }
        });
        iterate_until(|| failed.get());
        window.close();
        iterate_until(|| window.is_sensitive());
        assert!(window.is_visible(), "failed drain must not silently quit");
        let dialog = gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|window| window.downcast::<adw::MessageDialog>().ok())
            .expect("visible failure decision");
        dialog.response("cancel");
        assert!(window.is_visible());
        let published = Rc::new(Cell::new(false));
        ui::background_io::run_serialized(ui::background_io::SerialLane::Config, || (), {
            let published = published.clone();
            move |()| published.set(true)
        });
        window.close();
        iterate_until(|| published.get() && window.is_sensitive());
        let dialog = gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|window| {
                window
                    .downcast::<adw::MessageDialog>()
                    .ok()
                    .filter(|dialog| dialog.is_visible())
            })
            .expect("successful later save must not erase the prior failure");
        assert!(dialog.has_response("abandon"));
        dialog.response("retry");
        iterate_until(|| window.is_sensitive());
        assert!(window.is_visible());
        let dialog = gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|window| {
                window
                    .downcast::<adw::MessageDialog>()
                    .ok()
                    .filter(|dialog| dialog.is_visible())
            })
            .expect("retry retains failure evidence and informed exit");
        let (release, held) = std::sync::mpsc::channel();
        let final_published = Rc::new(Cell::new(false));
        ui::background_io::run_serialized(
            ui::background_io::SerialLane::Config,
            move || held.recv().unwrap(),
            {
                let final_published = final_published.clone();
                move |()| final_published.set(true)
            },
        );
        dialog.response("abandon");
        assert!(
            window.is_visible() && !window.is_sensitive(),
            "abandonment must re-enter the save barrier"
        );
        release.send(()).unwrap();
        iterate_until(|| !window.is_visible());
        assert!(
            published.get() && final_published.get(),
            "informed abandonment must still drain successful writes"
        );
    });
}

#[test]
#[ignore = "requires a disposable GTK display"]
fn native_quit_action_obeys_the_pre_quit_save_barrier() {
    crate::ui::native_test!(|| {
        let app = adw::Application::builder()
            .application_id("io.github.worxbend.scenedeck.QuitFixture")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        let published = Rc::new(Cell::new(false));
        let observed = Rc::new(Cell::new(false));
        app.connect_shutdown({
            let published = published.clone();
            let observed = observed.clone();
            move |_| {
                assert!(published.get());
                observed.set(true);
            }
        });
        app.connect_activate({
            let published = published.clone();
            move |app| {
                let window = adw::ApplicationWindow::builder().application(app).build();
                install_persistence_close_guard(app, &window, std::rc::Weak::new());
                window.present();
                let (release, held) = std::sync::mpsc::channel();
                ui::background_io::run(move || held.recv().unwrap(), {
                    let published = published.clone();
                    move |()| published.set(true)
                });
                app.activate_action("quit", None);
                assert!(!published.get());
                assert!(window.is_visible() && !window.is_sensitive());
                release.send(()).unwrap();
            }
        });
        assert_eq!(app.run_with_args::<&str>(&[]), glib::ExitCode::SUCCESS);
        assert!(observed.get());
    });
}

#[test]
fn unreadable_registry_loads_defaults_and_names_the_file_in_a_startup_notice() {
    let dir = crate::storage::test_support::TempDir::new("startup-registry");
    let path = dir.path().join("registry.json");
    std::fs::write(&path, "{ not json").unwrap();
    let read =
        crate::storage::registry::read_registry_from_path(&path).map_err(|error| error.to_string());

    let (registry, notice) = startup_registry(&read);

    assert_eq!(registry, SceneRegistry::default());
    let notice = notice.expect("an unreadable registry must be reported");
    assert!(notice.contains(&path.display().to_string()), "{notice}");
}

#[test]
fn readable_registry_loads_without_a_startup_notice() {
    let saved = SceneRegistry {
        scene_order: vec!["Host".into()],
        ..SceneRegistry::default()
    };

    let (registry, notice) = startup_registry(&Ok(saved.clone()));

    assert_eq!(registry, saved);
    assert_eq!(notice, None);
}
