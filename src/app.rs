//! Application bootstrap.
//!
//! Creates the tokio runtime, the UI↔Controller event channel, and the GTK
//! application object.  Lifetime rule: the Runtime outlives the GTK main loop
//! so all spawned tasks complete before it is dropped.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use adw::prelude::*;

use crate::app_info::APP_ID;
use crate::controller::app_controller::AppController;
use crate::controller::event::AppEvent;
use crate::controller::state::AppState;
use crate::storage::config::read_config;
use crate::ui;

pub fn run() {
    // The Runtime must be created before the GTK application so that any task
    // it spawns can access a live executor during the GTK main loop.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    let tokio_handle = runtime.handle().clone();
    let (event_tx, event_rx) = mpsc::sync_channel::<AppEvent>(128);

    let event_rx = Rc::new(RefCell::new(Some(event_rx)));

    let mut app_builder = adw::Application::builder().application_id(APP_ID);
    if std::env::var_os("SNAP").is_some() {
        // The snap does not declare a DBus slot, so avoid owning APP_ID on the
        // session bus while confined.
        app_builder = app_builder.flags(gio::ApplicationFlags::NON_UNIQUE);
    }
    let app = app_builder.build();

    app.connect_activate(move |app| {
        let rx = event_rx
            .borrow_mut()
            .take()
            .expect("app activated more than once");

        let controller = Rc::new(RefCell::new(AppController::new(
            tokio_handle.clone(),
            event_tx.clone(),
        )));

        build_ui(app, controller, rx);
    });

    app.run();

    // `runtime` is dropped here, after app.run() returns.  Any still-running
    // tasks are given a chance to finish because Runtime::drop blocks until
    // all spawned tasks complete (or are cancelled).
    drop(runtime);
}

fn build_ui(
    app: &adw::Application,
    controller: Rc<RefCell<AppController>>,
    event_rx: mpsc::Receiver<AppEvent>,
) {
    let loaded = read_config();
    let theme_mode = loaded.config.appearance.mode;
    let notice = loaded.startup_notice.map(|n| n.user_message());

    let state = Rc::new(RefCell::new(AppState::new(theme_mode, notice)));

    let _window = ui::build_main_window(app, state, controller, event_rx);
}
