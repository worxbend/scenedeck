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
use crate::storage::config::{read_startup_config, LoadedConfig, PendingRepair};
use crate::storage::registry::{read_registry, SceneRegistry};
use crate::ui;

struct StartupData {
    config: LoadedConfig,
    registry: Result<SceneRegistry, String>,
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
    let startup = std::thread::spawn(|| StartupData {
        config: read_startup_config(),
        registry: read_registry().map_err(|error| error.to_string()),
        bookmarks: crate::storage::bookmarks::read(&crate::storage::xdg::bookmarks_path())
            .map_err(|error| error.to_string()),
        history: crate::storage::report_history::read(&crate::storage::xdg::report_history_path())
            .map_err(|error| error.to_string()),
    })
    .join()
    .unwrap_or_else(|_| {
        tracing::error!("startup persistence worker panicked; using defaults");
        StartupData {
            config: LoadedConfig {
                config: Default::default(),
                startup_notice: None,
                pending_repair: None,
            },
            registry: Err("Startup registry reader failed".into()),
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
    // Weak: the last document window must still release application state.
    state: std::rc::Weak<RefCell<AppState>>,
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
        if !self.abandon.get() {
            ui::persist::flush_scene_view();
        }
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
fn install_persistence_close_guard(
    app: &adw::Application,
    window: &adw::ApplicationWindow,
    state: std::rc::Weak<RefCell<AppState>>,
) {
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
                state: state.clone(),
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
                    ui::persist::retry_all_durable_writes(&guard.state);
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
    let (registry, registry_notice) = startup_registry(&startup.registry);
    let notices = loaded
        .startup_notice
        .iter()
        .map(|notice| notice.user_message())
        .chain(registry_notice)
        .collect();

    let state = Rc::new(RefCell::new(AppState::new(
        loaded.config.clone(),
        registry,
        notices,
    )));
    persist_startup_migration(loaded);

    state.borrow_mut().load_bookmarks(startup.bookmarks.clone());
    state
        .borrow_mut()
        .load_report_history(startup.history.clone());
    let owner = Rc::downgrade(&state);
    let window = ui::build_main_window(app, state, controller, event_rx);
    install_persistence_close_guard(app, &window, owner);
    window
}

/// Defaults stand in for an unreadable registry.json, but never silently:
/// empty curation would look like lost work. Registry saves refuse to replace
/// the unreadable file, so the notice is the operator's cue to repair it.
fn startup_registry(read: &Result<SceneRegistry, String>) -> (SceneRegistry, Option<String>) {
    match read {
        Ok(registry) => (registry.clone(), None),
        Err(detail) => {
            tracing::warn!(%detail, "failed to load registry, using defaults");
            let notice = i18n_embed_fl::fl!(
                crate::infra::i18n::LANGUAGE_LOADER,
                "registry-startup-failed",
                detail = detail.as_str()
            );
            (SceneRegistry::default(), Some(notice))
        }
    }
}

/// Save the config the startup read migrated in memory.
///
/// The read never writes. Saving through the ordered config lane once GTK owns
/// persistence lets a failed write block Close like any Settings save.
fn persist_startup_migration(loaded: &LoadedConfig) {
    if loaded.pending_repair == Some(PendingRepair::WriteMigrated) {
        ui::persist::persist_config_snapshot(loaded.config.clone());
    }
}

#[cfg(test)]
mod tests;
