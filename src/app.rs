//! Application bootstrap.
//!
//! Creates the tokio runtime, the UI↔Controller event channel, and the GTK
//! application object.  Lifetime rule: the Runtime outlives the GTK main loop
//! while controller cancellation/join and persistence draining run explicitly.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use adw::prelude::*;

use crate::app_info::APP_ID;
use crate::controller::app_controller::AppController;
use crate::controller::event::AppEvent;
use crate::controller::state::AppState;
use crate::infra::i18n;
use crate::storage::config::read_config;
use crate::storage::registry::read_registry;
use crate::storage::secret;
use crate::ui;

struct StartupData {
    config: crate::storage::config::LoadedConfig,
    registry: crate::storage::registry::SceneRegistry,
    password: Option<String>,
    bookmarks: Result<crate::storage::bookmarks::BookmarkArchive, String>,
    history: Result<crate::storage::report_history::HistoryArchive, String>,
}

/// Initialize runtime state and run the GTK application.
pub fn run() {
    // The Runtime must be created before the GTK application so that any task
    // it spawns can access a live executor during the GTK main loop.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    let tokio_handle = runtime.handle().clone();
    let (event_tx, event_rx) = mpsc::sync_channel::<AppEvent>(128);
    let startup = std::thread::spawn(|| {
        let config = read_config();
        let password =
            secret::get_obs_password(&config.config.connection_id).unwrap_or_else(|error| {
                tracing::warn!(%error, "could not preload OBS password from keyring");
                None
            });
        StartupData {
            config,
            registry: read_registry(),
            password,
            bookmarks: crate::storage::bookmarks::read(&crate::storage::xdg::bookmarks_path())
                .map_err(|error| error.to_string()),
            history: crate::storage::report_history::read(
                &crate::storage::xdg::report_history_path(),
            )
            .map_err(|error| error.to_string()),
        }
    })
    .join()
    .unwrap_or_else(|_| {
        tracing::error!("startup persistence worker panicked; using defaults");
        StartupData {
            config: crate::storage::config::LoadedConfig {
                config: Default::default(),
                startup_notice: None,
            },
            registry: Default::default(),
            password: None,
            bookmarks: Err("Startup bookmark reader failed".into()),
            history: Err("Startup session history reader failed".into()),
        }
    });

    let event_rx = Rc::new(RefCell::new(Some(event_rx)));

    let mut app_builder = adw::Application::builder().application_id(APP_ID);
    if std::env::var_os("SNAP").is_some() {
        // The snap does not declare a DBus slot, so avoid owning APP_ID on the
        // session bus while confined.
        app_builder = app_builder.flags(gio::ApplicationFlags::NON_UNIQUE);
    }
    let app = app_builder.build();

    // GTK calls `activate` every time the application is asked to appear, not
    // only the first time: launching it again while it is running, a desktop
    // file action, or a D-Bus `Activate` all arrive here. The window is built
    // once and re-presented afterwards, which is what brings the running
    // instance to the front instead of building a second one on top of the
    // first — and instead of panicking on the receiver that only exists once.
    let window = glib::WeakRef::<adw::ApplicationWindow>::new();
    let controller_slot: Rc<RefCell<Option<Rc<RefCell<AppController>>>>> =
        Rc::new(RefCell::new(None));
    let activation_controller = controller_slot.clone();

    app.connect_shutdown(|app| {
        for window in app.windows() {
            window.set_sensitive(false);
        }
        ui::report_history::finish_for_shutdown();
        // Shutdown can also be requested by the application Quit action, not
        // only window close. Iterate GTK until accepted writes publish results.
        let flush_loop = glib::MainLoop::new(None, false);
        ui::background_io::flush_pending(std::time::Duration::from_secs(5), {
            let flush_loop = flush_loop.clone();
            move |outcome| {
                if let Err(error) = outcome {
                    tracing::error!(?error, "shutdown persistence drain incomplete");
                }
                flush_loop.quit();
            }
        });
        flush_loop.run();
    });

    app.connect_activate(move |app| {
        if let Some(existing) = window.upgrade() {
            existing.present();
            return;
        }

        let Some(rx) = event_rx.borrow_mut().take() else {
            // Closing the shell can leave a detached Stats window alive. Do
            // not recreate the single-use controller channel or retain the
            // destroyed shell just to support application reactivation.
            if let Some(existing) = app.windows().first() {
                existing.present();
                return;
            }
            tracing::error!("activated with no event receiver and no window; ignoring");
            return;
        };

        let controller = Rc::new(RefCell::new(AppController::new(
            tokio_handle.clone(),
            event_tx.clone(),
        )));

        *activation_controller.borrow_mut() = Some(controller.clone());
        window.set(Some(&build_ui(app, controller, rx, &startup)));
    });

    app.run();

    if let Some(controller) = controller_slot.borrow_mut().take() {
        let shutdown = controller.borrow_mut().shutdown();
        match runtime.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(3), shutdown).await
        }) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::error!(%error, "controller shutdown task failed"),
            Err(_) => tracing::warn!("controller shutdown join deadline exceeded"),
        }
    }
    // Runtime drop cancels async tasks; it does not gracefully join them.
    // Bound any remaining blocking settings/keyring workers separately.
    runtime.shutdown_timeout(std::time::Duration::from_secs(3));
}

/// App document windows can own the last accepted edits after the shell closes.
fn document_window(window: &gtk4::Window) -> bool {
    window.is::<gtk4::ApplicationWindow>() || window.is::<adw::Window>()
}

fn document_windows(app: &adw::Application) -> Vec<gtk4::Window> {
    app.windows().into_iter().filter(document_window).collect()
}

struct CloseGuard {
    app: glib::WeakRef<adw::Application>,
    window: glib::WeakRef<gtk4::Window>,
    main: bool,
    quitting: Rc<std::cell::Cell<bool>>,
    allowed: std::cell::Cell<bool>,
    pending: std::cell::Cell<bool>,
    abandon: std::cell::Cell<bool>,
    exit: RefCell<Option<ui::report_history::ExitCheckpoint>>,
}

impl CloseGuard {
    fn exiting(&self) -> bool {
        self.quitting.get()
            || self
                .app
                .upgrade()
                .is_none_or(|app| document_windows(&app).len() <= 1)
    }

    fn sensitivity(&self, sensitive: bool) {
        if self.exiting() {
            if let Some(app) = self.app.upgrade() {
                for window in app.windows() {
                    window.set_sensitive(sensitive);
                }
            }
        } else if let Some(window) = self.window.upgrade() {
            window.set_sensitive(sensitive);
        }
    }

    fn prepare_exit(&self) {
        if !self.exiting() {
            return;
        }
        if self.abandon.get() {
            ui::report_history::abandon_exit(true);
            return;
        }
        let mut exit = self.exit.borrow_mut();
        if exit.is_none() {
            *exit = ui::report_history::ExitCheckpoint::new();
        }
        if let Some(exit) = exit.as_mut() {
            exit.prepare();
        }
    }

    fn request(self: &Rc<Self>) -> glib::Propagation {
        if self.allowed.get() || (!self.main && !self.exiting()) {
            return glib::Propagation::Proceed;
        }
        if self.pending.replace(true) {
            return glib::Propagation::Stop;
        }
        self.sensitivity(false);
        if self.exiting() {
            self.prepare_exit();
        } else if !self.abandon.get() {
            ui::report_history::checkpoint_before_close();
        }
        self.drain();
        glib::Propagation::Stop
    }

    fn drain(self: &Rc<Self>) {
        let guard = self.clone();
        ui::background_io::flush_pending(std::time::Duration::from_secs(5), move |outcome| {
            let Some(window) = guard.window.upgrade() else {
                return;
            };
            match outcome {
                Ok(()) => {
                    // Another document may have closed while the shell drained.
                    guard.prepare_exit();
                    let needs_drain = guard
                        .exit
                        .borrow_mut()
                        .as_mut()
                        .is_some_and(|exit| exit.checkpoint_again());
                    if needs_drain {
                        guard.drain();
                        return;
                    }
                    guard.pending.set(false);
                    guard.allowed.set(true);
                    guard.exit.borrow_mut().take();
                    if guard.quitting.get() {
                        if let Some(app) = guard.app.upgrade() {
                            app.quit();
                        }
                    } else {
                        window.close();
                    }
                }
                Err(error) => {
                    guard.pending.set(false);
                    ui::report_history::abandon_exit(false);
                    if let Some(exit) = guard.exit.borrow_mut().take() {
                        exit.resume();
                    }
                    guard.sensitivity(true);
                    show_persistence_close_failure(&window, error, guard.clone());
                }
            }
        });
    }
}

/// Guard the shell and every last detached document; retain no shell widgets.
fn install_persistence_close_guard(app: &adw::Application, window: &adw::ApplicationWindow) {
    let quitting = Rc::new(std::cell::Cell::new(false));
    let main = window.clone().upcast::<gtk4::Window>().downgrade();
    let guarded = Rc::new(RefCell::new(Vec::<glib::WeakRef<gtk4::Window>>::new()));
    let attach: Rc<dyn Fn(&gtk4::Window)> = Rc::new({
        let app = app.downgrade();
        let main = main.clone();
        let quitting = quitting.clone();
        move |window| {
            if !document_window(window) {
                return;
            }
            let mut guarded = guarded.borrow_mut();
            guarded.retain(|weak| weak.upgrade().is_some());
            if guarded
                .iter()
                .any(|weak| weak.upgrade().as_ref() == Some(window))
            {
                return;
            }
            guarded.push(window.downgrade());
            let guard = Rc::new(CloseGuard {
                app: app.clone(),
                window: window.downgrade(),
                main: main.upgrade().as_ref() == Some(window),
                quitting: quitting.clone(),
                allowed: std::cell::Cell::new(false),
                pending: std::cell::Cell::new(false),
                abandon: std::cell::Cell::new(false),
                exit: RefCell::new(None),
            });
            window.connect_close_request(move |_| guard.request());
        }
    });
    for window in app.windows() {
        attach(&window);
    }
    app.connect_window_added(move |_, window| attach(window));
    app.remove_action("quit");
    let quit = gio::SimpleAction::new("quit", None);
    let app_weak = app.downgrade();
    quit.connect_activate(move |_, _| {
        let Some(app) = app_weak.upgrade() else {
            return;
        };
        quitting.set(true);
        let windows = document_windows(&app);
        let target = main
            .upgrade()
            .filter(|main| windows.contains(main))
            .or_else(|| app.active_window().filter(document_window))
            .or_else(|| windows.first().cloned());
        if let Some(window) = target {
            window.close();
        } else {
            app.quit();
        }
    });
    app.add_action(&quit);
}

fn show_persistence_close_failure(
    window: &gtk4::Window,
    error: ui::background_io::FlushError,
    guard: Rc<CloseGuard>,
) {
    let message = match error {
        ui::background_io::FlushError::TimedOut => i18n_embed_fl::fl!(
            crate::infra::i18n::LANGUAGE_LOADER,
            "app-persistence-drain-timeout"
        ),
        ui::background_io::FlushError::WorkerFailed => {
            if let ui::persist::RegistrySaveStatus::Failed(detail) =
                ui::persist::registry_save_status()
            {
                i18n_embed_fl::fl!(
                    crate::infra::i18n::LANGUAGE_LOADER,
                    "settings-failed-to-save",
                    err = detail
                )
            } else {
                i18n_embed_fl::fl!(
                    crate::infra::i18n::LANGUAGE_LOADER,
                    "app-persistence-drain-worker-failed"
                )
            }
        }
    };
    let heading = i18n_embed_fl::fl!(
        crate::infra::i18n::LANGUAGE_LOADER,
        "workflow-activity-failed"
    );
    let dialog = adw::MessageDialog::new(Some(window), Some(&heading), Some(&message));
    dialog.set_widget_name("persistence-close-failure");
    dialog.add_response(
        "cancel",
        &i18n_embed_fl::fl!(
            crate::infra::i18n::LANGUAGE_LOADER,
            "inventory-dialog-cancel-label"
        ),
    );
    dialog.add_response(
        "retry",
        &i18n_embed_fl::fl!(
            crate::infra::i18n::LANGUAGE_LOADER,
            "window-connect-btn-retry"
        ),
    );
    let evidence = ui::background_io::failure_evidence();
    if error == ui::background_io::FlushError::WorkerFailed {
        dialog.add_response(
            "abandon",
            &i18n_embed_fl::fl!(
                crate::infra::i18n::LANGUAGE_LOADER,
                "app-persistence-close-anyway"
            ),
        );
        dialog.set_response_appearance("abandon", adw::ResponseAppearance::Destructive);
    }
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let handled = std::cell::Cell::new(false);
    dialog.connect_response(None, move |dialog, response| {
        if handled.replace(true) {
            return;
        }
        dialog.close();
        match response {
            "retry" | "abandon" => {
                guard.abandon.set(response == "abandon");
                if response == "abandon" {
                    ui::background_io::acknowledge_failures(evidence);
                } else {
                    ui::persist::retry_registry_writes();
                    ui::report_history::retry_active();
                }
                if let Some(window) = guard.window.upgrade() {
                    window.close();
                }
            }
            _ => {
                guard.quitting.set(false);
                guard.abandon.set(false);
            }
        }
    });
    dialog.present();
}

/// Build the main window and hand it back so re-activation can present it.
fn build_ui(
    app: &adw::Application,
    controller: Rc<RefCell<AppController>>,
    event_rx: mpsc::Receiver<AppEvent>,
    startup: &StartupData,
) -> adw::ApplicationWindow {
    let loaded = &startup.config;
    i18n::init(loaded.config.language);
    let notice = loaded.startup_notice.as_ref().map(|n| n.user_message());

    let state = Rc::new(RefCell::new(AppState::new(
        loaded.config.clone(),
        startup.registry.clone(),
        startup.password.clone(),
        notice,
    )));

    state.borrow_mut().load_bookmarks(startup.bookmarks.clone());
    state
        .borrow_mut()
        .load_report_history(startup.history.clone());
    let window = ui::build_main_window(app, state, controller, event_rx);
    install_persistence_close_guard(app, &window);
    window
}

#[cfg(test)]
mod tests {
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

    fn fixture() -> (adw::Application, adw::ApplicationWindow) {
        let app = adw::Application::builder()
            .application_id("io.github.worxbend.scenedeck.CloseFixture")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let window = adw::ApplicationWindow::builder().application(&app).build();
        install_persistence_close_guard(&app, &window);
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
        let registry = crate::storage::registry::read_registry_from_path(
            &crate::storage::xdg::registry_path(),
        )
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
        ui::test_support::native_test("app::tests::native_last_detached_report_close_supports_cancel_retry_and_informed_abandon", || {
            let (app, main) = fixture();
            let (nav, _stack, _runtime, _events) = ui::test_support::navigation();
            ui::report_history::register(&nav.state);
            nav.state.borrow_mut().configure_report_history(true, 8);
            connect_report(&mut nav.state.borrow_mut());
            let detached = adw::Window::builder().application(&app).build();
            detached.present();
            main.close();
            iterate_until(|| !main.is_visible());
            assert!(nav.state.borrow().reports.entries.back().unwrap().ended_unix_ms.is_none());
            let path = crate::storage::xdg::report_history_path();
            let backup = path.with_extension("backup");
            std::fs::rename(&path, &backup).unwrap();
            std::fs::create_dir(&path).unwrap();
            detached.close();
            iterate_until(|| close_failure().is_some());
            assert!(detached.is_visible() && detached.is_sensitive());
            assert!(nav.state.borrow().reports.entries.back().unwrap().ended_unix_ms.is_none(), "failed exit resumes reporting");
            close_failure().unwrap().response("cancel");
            assert!(detached.is_visible());
            nav.state.borrow_mut().workspace.log(crate::domain::workflow::ActivityOrigin::Observed, "after cancelled exit".into());
            detached.close();
            iterate_until(|| close_failure().is_some());
            std::fs::remove_dir(&path).unwrap();
            std::fs::rename(&backup, &path).unwrap();
            close_failure().unwrap().response("retry");
            iterate_until(|| !detached.is_visible());
            let saved = crate::storage::report_history::read(&path).unwrap();
            assert!(saved.entries[0].ended_unix_ms.is_some());
            assert!(serde_json::to_string(&saved).unwrap().contains("after cancelled exit"));

            // A corrupt startup file with new local history needs an informed exit,
            // even though no write can be admitted until it is reloaded/repaired.
            let detached = adw::Window::builder().application(&app).build();
            detached.present();
            nav.state.borrow_mut().reset_obs_session(crate::controller::state::ObsStatus::Disconnected);
            connect_report(&mut nav.state.borrow_mut());
            std::fs::write(&path, b"broken history").unwrap();
            nav.state.borrow_mut().load_report_history(Err("fixture corrupt startup history".into()));
            detached.close();
            iterate_until(|| close_failure().is_some());
            assert!(detached.is_visible());
            close_failure().unwrap().response("abandon");
            iterate_until(|| !detached.is_visible());
            assert_eq!(std::fs::read(&path).unwrap(), b"broken history");
        });
    }

    fn final_stats_exit(quit: bool) {
        use crate::controller::dependencies::{
            AppConfigProvider, ControllerDependencies, ObsPasswordProvider,
        };
        struct NoServices;
        impl AppConfigProvider for NoServices {
            fn load_config(&self) -> crate::storage::config::AppConfig {
                panic!("no config service in fixture")
            }
        }
        impl ObsPasswordProvider for NoServices {
            fn obs_password(
                &self,
                _: &str,
            ) -> Result<Option<String>, crate::infra::error::AppError> {
                panic!("no keyring in fixture")
            }
        }
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
                let controller = Rc::new(RefCell::new(AppController::with_dependencies(runtime.clone(), tx.clone(), ControllerDependencies::new(std::sync::Arc::new(NoServices), std::sync::Arc::new(NoServices)))));
                let mut initial = AppState::new(Default::default(), Default::default(), None, None);
                initial.config.onboarding.welcome_shown = true;
                initial.configure_report_history(true, 8);
                let state = Rc::new(RefCell::new(initial));
                *weak_state.borrow_mut() = Rc::downgrade(&state);
                let window = ui::window::build_main_window(app, state.clone(), controller, rx);
                install_persistence_close_guard(app, &window);
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
        ui::test_support::native_test(
            "app::tests::native_last_stats_close_saves_final_report_before_state_release",
            || final_stats_exit(false),
        );
    }

    #[test]
    #[ignore = "requires isolated GTK and scratch history; no OBS/keyring"]
    fn native_quit_after_shell_close_saves_final_stats_report_before_shutdown() {
        ui::test_support::native_test(
            "app::tests::native_quit_after_shell_close_saves_final_stats_report_before_shutdown",
            || final_stats_exit(true),
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display; injected Config write failure only"]
    fn native_config_write_error_requires_an_explicit_close_decision() {
        crate::ui::test_support::native_test(
            "app::tests::native_config_write_error_requires_an_explicit_close_decision",
            || {
                let (_app, window) = fixture();
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
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display; scratch config only, no OBS/keyring"]
    fn native_inventory_role_edit_and_rebuild_are_admitted_before_close() {
        crate::ui::test_support::native_test(
            "app::tests::native_inventory_role_edit_and_rebuild_are_admitted_before_close",
            || {
                let directory = isolated_config_dir("close");
                let (_app, window) = fixture();
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
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display; scratch config only, no OBS/keyring"]
    fn native_inventory_role_edit_is_admitted_before_quit_action() {
        crate::ui::test_support::native_test(
            "app::tests::native_inventory_role_edit_is_admitted_before_quit_action",
            || {
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
                            if let Some(dialog) = gtk4::Window::list_toplevels()
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
                        install_persistence_close_guard(app, &window);
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
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display; deliberately holds the registry writer past the save deadline"]
    fn native_quit_timeout_preserves_role_edits_until_explicit_retry() {
        crate::ui::test_support::native_test(
            "app::tests::native_quit_timeout_preserves_role_edits_until_explicit_retry",
            || {
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
                        install_persistence_close_guard(app, &window);
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
                            let dialog =
                                gtk4::Window::list_toplevels()
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
                                    assert!(
                                        observed_window.is_visible()
                                            && observed_window.is_sensitive()
                                    );
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
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display; scratch registry only, no OBS/keyring"]
    fn native_same_value_registry_edits_retry_failed_disk_writes() {
        crate::ui::test_support::native_test(
            "app::tests::native_same_value_registry_edits_retry_failed_disk_writes",
            || {
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
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display; scratch registry only, no OBS/keyring"]
    fn native_registry_write_error_is_visible_and_close_retry_recovers() {
        crate::ui::test_support::native_test(
            "app::tests::native_registry_write_error_is_visible_and_close_retry_recovers",
            || {
                let directory = isolated_config_dir("registry-retry");
                let target = crate::storage::xdg::registry_path();
                let parent = target.parent().unwrap();
                std::fs::write(parent, b"blocked directory").unwrap();
                let (_app, window) = fixture();
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
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_close_waits_for_accepted_save_and_gtk_publication() {
        crate::ui::test_support::native_test(
            "app::tests::native_close_waits_for_accepted_save_and_gtk_publication",
            || {
                let (_app, window) = fixture();
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
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_save_worker_failure_keeps_window_open_and_offers_cancel() {
        crate::ui::test_support::native_test(
            "app::tests::native_save_worker_failure_keeps_window_open_and_offers_cancel",
            || {
                let (_app, window) = fixture();
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
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_quit_action_obeys_the_pre_quit_save_barrier() {
        crate::ui::test_support::native_test(
            "app::tests::native_quit_action_obeys_the_pre_quit_save_barrier",
            || {
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
                        install_persistence_close_guard(app, &window);
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
            },
        );
    }
}
