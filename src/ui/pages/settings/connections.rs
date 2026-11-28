//! Named OBS connections and identity-bound credential editing.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

use adw::{prelude::*, ActionRow, ComboRow, EntryRow, PasswordEntryRow, PreferencesGroup};
use i18n_embed_fl::fl;

use crate::{
    controller::command::AppCommand,
    domain::connection::{
        credential::{CredentialStore, ObsPassword},
        endpoint::is_loopback,
    },
    infra::{error::AppError, i18n::LANGUAGE_LOADER},
    storage::config::{AppConfig, ObsConfig, SavedConnection},
    ui::{navigation::NavigationContext, persist::persist_config_with, string_list},
};

mod password_writes;

use password_writes::PasswordWrites;

struct Editor {
    nav: NavigationContext,
    credentials: Arc<dyn CredentialStore>,
    group: glib::WeakRef<PreferencesGroup>,
    picker: glib::WeakRef<ComboRow>,
    name: glib::WeakRef<EntryRow>,
    host: glib::WeakRef<EntryRow>,
    port: glib::WeakRef<EntryRow>,
    unencrypted_warning: glib::WeakRef<ActionRow>,
    password: glib::WeakRef<PasswordEntryRow>,
    status: glib::WeakRef<ActionRow>,
    password_status: glib::WeakRef<ActionRow>,
    profiles: RefCell<Vec<(String, String)>>,
    updating: Cell<bool>,
    generation: Cell<u64>,
    writes: PasswordWrites,
}

impl Editor {
    fn current(&self, generation: u64, id: &str) -> bool {
        self.generation.get() == generation && self.nav.state.borrow().config.connection_id == id
    }

    fn busy(&self, busy: bool) {
        if let Some(group) = self.group.upgrade() {
            group.set_sensitive(!busy);
        }
    }

    fn refresh_picker(&self) {
        let config = self.nav.state.borrow().config.clone();
        let profiles = config.connection_profiles();
        self.updating.set(true);
        if let Some(picker) = self.picker.upgrade() {
            let next: Vec<_> = profiles
                .iter()
                .map(|profile| (profile.id.clone(), profile.name.clone()))
                .collect();
            if *self.profiles.borrow() != next {
                // Replacing the model inside selected-notify invalidates GTK's
                // in-flight selection model. A selection changes neither IDs
                // nor labels, so retain the model until its contents change.
                let names: Vec<_> = next.iter().map(|(_, name)| name.clone()).collect();
                *self.profiles.borrow_mut() = next;
                picker.set_model(Some(&string_list(&names)));
            }
            picker.set_selected(
                profiles
                    .iter()
                    .position(|profile| profile.id == config.connection_id)
                    .map_or(u32::MAX, |index| index as u32),
            );
        }
        self.updating.set(false);
    }

    fn refresh_fields(&self) {
        self.refresh_picker();
        let state = self.nav.state.borrow();
        let profile = state
            .config
            .connection_profiles()
            .into_iter()
            .find(|profile| profile.id == state.config.connection_id);
        if let Some(name) = self.name.upgrade() {
            name.set_text(profile.as_ref().map_or("", |profile| &profile.name));
        }
        if let Some(host) = self.host.upgrade() {
            host.set_text(&state.config.obs.host);
        }
        if let Some(port) = self.port.upgrade() {
            port.set_text(&state.config.obs.port.to_string());
        }
        self.show_unencrypted_warning(&state.config.obs.host);
        if let Some(password) = self.password.upgrade() {
            // The keyring is the only copy; load_password fills the row.
            password.set_text("");
        }
    }

    /// Follows the saved host, not the typed text: Connect uses the saved one.
    fn show_unencrypted_warning(&self, saved_host: &str) {
        if let Some(warning) = self.unencrypted_warning.upgrade() {
            warning.set_visible(!is_loopback(saved_host));
        }
    }

    fn change_selection(self: &Rc<Self>, id: String, update: impl FnOnce(&mut AppConfig)) {
        self.generation.set(self.generation.get().wrapping_add(1));
        let generation = self.generation.get();
        self.busy(true);
        let editor = self.clone();
        persist_config_with(&self.nav, update, move |result, _| {
            if !editor.current(generation, &id) {
                return;
            }
            editor.report_save(result);
            editor.load_password(id, generation);
        });
        self.refresh_fields();
    }

    fn load_selected_password(self: &Rc<Self>) {
        let id = self.nav.state.borrow().config.connection_id.clone();
        self.busy(true);
        self.load_password(id, self.generation.get());
    }

    fn load_password(self: &Rc<Self>, id: String, generation: u64) {
        let destination = id.clone();
        let credentials = self.credentials.clone();
        let editor = self.clone();
        crate::ui::background_io::run_serialized(
            crate::ui::background_io::SerialLane::Secret,
            move || credentials.read(&destination),
            move |result| {
                if !editor.current(generation, &id) {
                    return;
                }
                match result {
                    Ok(password) => {
                        if let Some(row) = editor.password.upgrade() {
                            row.set_text(password.as_ref().map_or("", ObsPassword::expose));
                        }
                        if let Some(status) = editor.password_status.upgrade() {
                            status.set_subtitle("");
                        }
                    }
                    Err(error) => {
                        editor.report_secret(Err(error));
                    }
                }
                editor.busy(false);
            },
        );
    }

    fn report_save(&self, result: std::io::Result<()>) {
        if let Some(status) = self.status.upgrade() {
            match result {
                Ok(()) => status.set_subtitle(&fl!(LANGUAGE_LOADER, "settings-saved")),
                Err(error) => super::report_save_failure(&status, &error),
            }
        }
    }

    fn report_secret(&self, result: Result<(), AppError>) {
        if let Some(status) = self.password_status.upgrade() {
            match result {
                Ok(()) => status.set_subtitle(&fl!(LANGUAGE_LOADER, "settings-password-saved")),
                Err(error) => status.set_subtitle(&fl!(
                    LANGUAGE_LOADER,
                    "settings-keyring-error",
                    err = error.to_string()
                )),
            }
        }
    }

    fn save_fields(self: &Rc<Self>) {
        let (Some(name), Some(host), Some(port)) = (
            self.name.upgrade(),
            self.host.upgrade(),
            self.port.upgrade(),
        ) else {
            return;
        };
        let port = port
            .text()
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0);
        if name.text().trim().is_empty() || host.text().trim().is_empty() || port.is_none() {
            if let Some(status) = self.status.upgrade() {
                status.set_subtitle(&fl!(LANGUAGE_LOADER, "settings-connection-invalid"));
            }
            return;
        }
        let id = self.nav.state.borrow().config.connection_id.clone();
        let profile = SavedConnection {
            id: id.clone(),
            name: name.text().trim().into(),
            obs: ObsConfig {
                host: host.text().trim().into(),
                port: port.unwrap(),
            },
        };
        let generation = self.generation.get();
        self.busy(true);
        let editor = self.clone();
        persist_config_with(
            &self.nav,
            move |config| {
                config.save_connection(profile);
            },
            move |result, config| {
                if !editor.current(generation, &id) {
                    return;
                }
                editor.report_save(result);
                editor.refresh_picker();
                editor.show_unencrypted_warning(&config.obs.host);
                editor.busy(false);
            },
        );
    }

    fn save_password(self: &Rc<Self>, password: Option<ObsPassword>) {
        let id = self.nav.state.borrow().config.connection_id.clone();
        let generation = self.generation.get();
        let destination = id.clone();
        let credentials = self.credentials.clone();
        let editor = self.clone();
        self.writes.submit(
            password,
            move |password| match password {
                Some(password) => credentials.write(&destination, password),
                None => credentials.delete(&destination),
            },
            move |result| {
                if !editor.current(generation, &id) {
                    return;
                }
                editor.report_secret(result);
            },
        );
    }

    fn connect(self: &Rc<Self>) {
        let id = self.nav.state.borrow().config.connection_id.clone();
        let generation = self.generation.get();
        self.busy(true);
        let editor = self.clone();
        persist_config_with(
            &self.nav,
            |_| {},
            move |result, _| {
                if !editor.current(generation, &id) {
                    return;
                }
                let saved = result.is_ok();
                editor.report_save(result);
                if saved {
                    editor.connect_after_password_writes(id, generation);
                } else {
                    editor.busy(false);
                }
            },
        );
    }

    /// The controller reads the keyring itself when it connects. The Secret
    /// lane is FIFO, so this empty job completes only after every password
    /// write submitted before it, and Connect never reads a password that is
    /// still being saved.
    fn connect_after_password_writes(self: &Rc<Self>, id: String, generation: u64) {
        let editor = self.clone();
        crate::ui::background_io::run_serialized(
            crate::ui::background_io::SerialLane::Secret,
            || (),
            move |()| {
                if !editor.current(generation, &id) {
                    return;
                }
                editor.busy(false);
                editor.nav.dispatch(AppCommand::Connect);
            },
        );
    }
}

pub(super) fn build(
    nav: &NavigationContext,
    credentials: Arc<dyn CredentialStore>,
) -> (PreferencesGroup, ActionRow) {
    let group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-obs-connection-title"))
        .description(fl!(LANGUAGE_LOADER, "settings-connections-description"))
        .build();
    let picker = ComboRow::builder()
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "settings-connections-picker"))
        .build();
    picker.set_widget_name("settings-connection-picker");
    let row = |title: &str, widget: &str| {
        let row = EntryRow::builder()
            .use_markup(false)
            .title(title)
            .show_apply_button(true)
            .build();
        row.set_widget_name(widget);
        row
    };
    let name = row(
        &fl!(LANGUAGE_LOADER, "settings-connection-name"),
        "settings-connection-name",
    );
    let host = row(
        &fl!(LANGUAGE_LOADER, "settings-host-title"),
        "settings-obs-host",
    );
    let port = row(
        &fl!(LANGUAGE_LOADER, "settings-port-title"),
        "settings-obs-port",
    );
    let unencrypted_warning = unencrypted_remote_warning();
    let password = PasswordEntryRow::builder()
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "settings-password-title"))
        .show_apply_button(true)
        .build();
    password.set_widget_name("settings-obs-password");
    let password_status = ActionRow::builder()
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "settings-password-title"))
        .build();
    let status = ActionRow::builder()
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "settings-obs-status-title"))
        .build();
    status.set_subtitle(&super::obs_status_text(nav));
    let action_row = |title: String, icon: &str, widget: &str| {
        let row = ActionRow::builder().use_markup(false).title(&title).build();
        let button = gtk4::Button::from_icon_name(icon);
        button.set_widget_name(widget);
        button.set_tooltip_text(Some(&title));
        button.update_property(&[gtk4::accessible::Property::Label(&title)]);
        button.set_valign(gtk4::Align::Center);
        row.add_suffix(&button);
        row.set_activatable_widget(Some(&button));
        (row, button)
    };
    let (add_row, add) = action_row(
        fl!(LANGUAGE_LOADER, "settings-connection-add"),
        "list-add-symbolic",
        "settings-connection-add",
    );
    let (connect_row, connect) = action_row(
        fl!(LANGUAGE_LOADER, "settings-connection-connect"),
        "network-server-symbolic",
        "settings-connection-connect",
    );
    for row in [
        picker.clone().upcast::<gtk4::Widget>(),
        name.clone().upcast(),
        host.clone().upcast(),
        port.clone().upcast(),
        unencrypted_warning.clone().upcast(),
        password.clone().upcast(),
        password_status.clone().upcast(),
        add_row.upcast(),
        connect_row.upcast(),
    ] {
        group.add(&row);
    }
    super::with_entry_icon(&host, "nf-md-server-network-symbolic");
    super::with_entry_icon(&port, "nf-md-ethernet-symbolic");
    super::with_entry_icon(&password, "nf-md-key-variant-symbolic");
    let editor = Rc::new(Editor {
        nav: nav.clone(),
        credentials,
        group: group.downgrade(),
        picker: picker.downgrade(),
        name: name.downgrade(),
        host: host.downgrade(),
        port: port.downgrade(),
        unencrypted_warning: unencrypted_warning.downgrade(),
        password: password.downgrade(),
        status: status.downgrade(),
        password_status: password_status.downgrade(),
        profiles: RefCell::new(Vec::new()),
        updating: Cell::new(false),
        generation: Cell::new(0),
        writes: PasswordWrites::default(),
    });
    editor.refresh_fields();
    editor.load_selected_password();
    picker.connect_selected_notify({
        let editor = editor.clone();
        move |picker| {
            if editor.updating.get() {
                return;
            }
            let id = editor
                .profiles
                .borrow()
                .get(picker.selected() as usize)
                .map(|(id, _)| id.clone());
            if let Some(id) = id.filter(|id| *id != editor.nav.state.borrow().config.connection_id)
            {
                let selected = id.clone();
                editor.change_selection(id, move |config| {
                    config.select_connection(&selected);
                });
            }
        }
    });
    for row in [&name, &host, &port] {
        row.connect_apply({
            let editor = editor.clone();
            move |_| editor.save_fields()
        });
    }
    password.connect_apply({
        let editor = editor.clone();
        move |row| {
            editor.save_password(
                (!row.text().is_empty()).then(|| ObsPassword::from(row.text().as_str())),
            )
        }
    });
    add.connect_clicked({
        let editor = editor.clone();
        move |_| {
            let id = glib::uuid_string_random().to_string();
            let number = editor.nav.state.borrow().config.connection_profiles().len() + 1;
            let profile = SavedConnection {
                id: id.clone(),
                name: fl!(
                    LANGUAGE_LOADER,
                    "settings-connection-new-name",
                    number = number
                ),
                obs: ObsConfig::default(),
            };
            let selected = id.clone();
            editor.change_selection(id, move |config| {
                config.save_connection(profile);
                config.select_connection(&selected);
            });
        }
    });
    connect.connect_clicked(move |_| editor.connect());
    (group, status)
}

/// Shown while the saved host is not loopback. obs-websocket speaks plain
/// `ws://`, so the authentication exchange and every command cross the
/// network unencrypted.
fn unencrypted_remote_warning() -> ActionRow {
    // GObject builder property order may set title before use-markup.
    let row = ActionRow::builder()
        .use_markup(false)
        .visible(false)
        .build();
    row.set_title(&fl!(
        LANGUAGE_LOADER,
        "settings-connection-unencrypted-remote"
    ));
    row.set_widget_name("settings-connection-unencrypted-remote");
    super::with_icon(&row, "dialog-warning-symbolic");
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicBool, Ordering},
            Mutex,
        },
        time::Duration,
    };

    #[derive(Default)]
    struct FakeSecrets {
        passwords: Mutex<HashMap<String, String>>,
        writes: Mutex<Vec<String>>,
        reads: Mutex<Vec<Option<String>>>,
        hold_write: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
        fail_read: AtomicBool,
    }

    impl FakeSecrets {
        fn wait_for_release(&self) {
            if let Some(hold) = self.hold_write.lock().unwrap().take() {
                hold.recv_timeout(Duration::from_secs(10)).unwrap();
            }
        }
    }

    impl CredentialStore for FakeSecrets {
        fn read(&self, id: &str) -> Result<Option<ObsPassword>, AppError> {
            let stored = self.passwords.lock().unwrap().get(id).cloned();
            self.reads.lock().unwrap().push(stored.clone());
            if self.fail_read.load(Ordering::SeqCst) {
                return Err(AppError::storage(std::io::Error::other(
                    "fixture read failed",
                )));
            }
            Ok(stored.map(ObsPassword::from))
        }
        fn write(&self, id: &str, password: &ObsPassword) -> Result<(), AppError> {
            self.wait_for_release();
            self.writes.lock().unwrap().push(id.into());
            self.passwords
                .lock()
                .unwrap()
                .insert(id.into(), password.expose().into());
            Ok(())
        }
        fn delete(&self, id: &str) -> Result<(), AppError> {
            self.wait_for_release();
            self.writes.lock().unwrap().push(id.into());
            self.passwords.lock().unwrap().remove(id);
            Ok(())
        }
    }

    fn named(widget: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
        if widget.widget_name() == name {
            return Some(widget.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if let Some(found) = named(&widget, name) {
                return Some(found);
            }
        }
        None
    }

    fn flush() {
        let done = Rc::new(RefCell::new(None));
        crate::ui::background_io::flush_pending(Duration::from_secs(10), {
            let done = done.clone();
            move |result| *done.borrow_mut() = Some(result)
        });
        assert!(crate::ui::test_support::pump_until(
            Duration::from_secs(11),
            || done.borrow().is_some()
        ));
        assert_eq!(*done.borrow(), Some(Ok(())));
    }

    #[test]
    #[ignore = "requires a disposable GTK display; fake connection providers prevent OBS and keyring access"]
    fn native_connect_reads_confirmed_selection_and_refuses_failed_save() {
        crate::ui::native_test!(|| {
            use crate::controller::{
                app_controller::AppController,
                dependencies::{AppConfigProvider, ControllerDependencies},
                state::AppState,
            };
            #[derive(Default)]
            struct Observed(Mutex<Option<(String, String)>>);
            impl AppConfigProvider for Observed {
                fn load_config(&self) -> Result<AppConfig, AppError> {
                    let config = crate::storage::config::read_config().config;
                    *self.0.lock().unwrap() =
                        Some((config.connection_id.clone(), config.obs.host.clone()));
                    Ok(config)
                }
            }
            impl CredentialStore for Observed {
                fn read(&self, id: &str) -> Result<Option<ObsPassword>, AppError> {
                    assert_eq!(id, "event");
                    Err(AppError::storage(std::io::Error::other(
                        "fixture stops before OBS connection",
                    )))
                }
                fn write(&self, _id: &str, _password: &ObsPassword) -> Result<(), AppError> {
                    panic!("the controller never writes a password")
                }
                fn delete(&self, _id: &str) -> Result<(), AppError> {
                    panic!("the controller never deletes a password")
                }
            }
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();
            let observed = Arc::new(Observed::default());
            let (tx, _events) = std::sync::mpsc::sync_channel(128);
            let controller = Rc::new(RefCell::new(AppController::with_dependencies(
                runtime.handle().clone(),
                tx,
                ControllerDependencies::new(observed.clone(), observed.clone()),
            )));
            let mut config = AppConfig::default();
            config.save_connection(SavedConnection {
                id: "event".into(),
                name: "Event".into(),
                obs: ObsConfig {
                    host: "event.local".into(),
                    port: 4456,
                },
            });
            config.select_connection("event");
            let state = Rc::new(RefCell::new(AppState::new(
                config,
                Default::default(),
                Vec::new(),
            )));
            let stack = gtk4::Stack::new();
            let nav = NavigationContext::new(state, stack, controller);
            let (group, _status) = build(&nav, Arc::new(FakeSecrets::default()));
            let connect = named(&group.clone().upcast(), "settings-connection-connect")
                .unwrap()
                .downcast::<gtk4::Button>()
                .unwrap();
            connect.emit_clicked();
            assert!(
                observed.0.lock().unwrap().is_none(),
                "connect must wait for GTK publication of the config save"
            );
            flush();
            assert!(crate::ui::test_support::pump_until(
                Duration::from_secs(5),
                || observed.0.lock().unwrap().is_some()
            ));
            assert_eq!(
                *observed.0.lock().unwrap(),
                Some(("event".into(), "event.local".into()))
            );
            *observed.0.lock().unwrap() = None;
            let path = crate::storage::xdg::config_path();
            let backup = path.with_extension("json.fixture");
            std::fs::rename(&path, &backup).unwrap();
            std::fs::create_dir(&path).unwrap();
            connect.emit_clicked();
            flush();
            crate::ui::test_support::pump(Duration::from_millis(100));
            assert!(
                observed.0.lock().unwrap().is_none(),
                "a failed save must not reconnect with another selection"
            );
            std::fs::remove_dir(&path).unwrap();
            std::fs::rename(backup, path).unwrap();
        });
    }

    #[test]
    #[ignore = "requires a disposable GTK display; fake credentials and scratch config, no OBS access"]
    fn native_connect_waits_for_a_queued_password_write() {
        crate::ui::native_test!(|| {
            use crate::controller::{
                app_controller::AppController, dependencies::ControllerDependencies,
                state::AppState, test_support::deps::StaticConfigProvider,
            };
            let credentials = Arc::new(FakeSecrets::default());
            *credentials.passwords.lock().unwrap() =
                HashMap::from([("default".into(), "old-secret".into())]);
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();
            let (tx, _events) = std::sync::mpsc::sync_channel(128);
            let controller = Rc::new(RefCell::new(AppController::with_dependencies(
                runtime.handle().clone(),
                tx,
                ControllerDependencies::new(
                    Arc::new(StaticConfigProvider(AppConfig::default())),
                    credentials.clone(),
                ),
            )));
            let state = Rc::new(RefCell::new(AppState::new(
                AppConfig::default(),
                Default::default(),
                Vec::new(),
            )));
            let nav = NavigationContext::new(state, gtk4::Stack::new(), controller);
            let (group, status) = build(&nav, credentials.clone());
            let root = group.upcast::<gtk4::Widget>();
            let password = named(&root, "settings-obs-password")
                .unwrap()
                .downcast::<PasswordEntryRow>()
                .unwrap();
            let connect = named(&root, "settings-connection-connect")
                .unwrap()
                .downcast::<gtk4::Button>()
                .unwrap();
            flush();
            // From here every read is the controller's, and failing it stops
            // the session before it opens a socket.
            credentials.fail_read.store(true, Ordering::SeqCst);
            credentials.reads.lock().unwrap().clear();

            let (release, hold) = std::sync::mpsc::channel();
            *credentials.hold_write.lock().unwrap() = Some(hold);
            password.set_text("new-secret");
            password.emit_by_name::<()>("apply", &[]);
            connect.emit_clicked();
            let saved = fl!(LANGUAGE_LOADER, "settings-saved");
            assert!(crate::ui::test_support::pump_until(
                Duration::from_secs(5),
                || status.subtitle().as_deref() == Some(saved.as_str())
            ));
            crate::ui::test_support::pump(Duration::from_millis(200));
            assert!(
                credentials.reads.lock().unwrap().is_empty(),
                "Connect must not read the keyring while a password write is queued"
            );

            release.send(()).unwrap();
            flush();
            assert!(crate::ui::test_support::pump_until(
                Duration::from_secs(5),
                || !credentials.reads.lock().unwrap().is_empty()
            ));
            assert_eq!(
                *credentials.reads.lock().unwrap(),
                [Some("new-secret".to_string())]
            );
        });
    }

    #[test]
    #[ignore = "requires a disposable GTK display; fake credentials and scratch config, no OBS access"]
    fn native_unencrypted_warning_follows_whether_the_saved_host_is_loopback() {
        crate::ui::native_test!(|| {
            let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
            let (group, _status) = build(&nav, Arc::new(FakeSecrets::default()));
            let root = group.upcast::<gtk4::Widget>();
            let host = named(&root, "settings-obs-host")
                .unwrap()
                .downcast::<EntryRow>()
                .unwrap();
            let warning = named(&root, "settings-connection-unencrypted-remote").unwrap();
            flush();
            assert!(!warning.is_visible(), "the default 127.0.0.1 is loopback");

            for (address, remote) in [
                ("192.168.1.20", true),
                ("127.0.0.1", false),
                ("obs.example.net", true),
                ("::1", false),
                ("localhost", false),
            ] {
                host.set_text(address);
                host.emit_by_name::<()>("apply", &[]);
                flush();
                assert_eq!(nav.read(|state| state.config.obs.host.clone()), address);
                assert_eq!(warning.is_visible(), remote, "host {address}");
            }

            host.set_text("192.168.1.20");
            host.emit_by_name::<()>("apply", &[]);
            flush();
            assert!(warning.is_visible());
            named(&root, "settings-connection-add")
                .unwrap()
                .downcast::<gtk4::Button>()
                .unwrap()
                .emit_clicked();
            flush();
            assert!(!warning.is_visible(), "a new connection starts on loopback");
            named(&root, "settings-connection-picker")
                .unwrap()
                .downcast::<ComboRow>()
                .unwrap()
                .set_selected(0);
            flush();
            assert!(warning.is_visible(), "reselecting the remote connection");
        });
    }

    #[test]
    #[ignore = "requires a disposable GTK display; fake credentials and scratch config, no OBS access"]
    fn native_saved_connections_keep_delayed_credentials_and_endpoints_separate() {
        crate::ui::native_test!(|| {
            let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
            nav.state
                .borrow_mut()
                .config
                .save_connection(SavedConnection {
                    id: "event".into(),
                    name: "Event laptop".into(),
                    obs: ObsConfig {
                        host: "event.local".into(),
                        port: 4456,
                    },
                });
            let secrets = Arc::new(FakeSecrets::default());
            *secrets.passwords.lock().unwrap() = HashMap::from([
                ("default".into(), "original-secret".into()),
                ("event".into(), "event-secret".into()),
            ]);
            let (group, _status) = build(&nav, secrets.clone());
            let root = group.clone().upcast::<gtk4::Widget>();
            let picker = named(&root, "settings-connection-picker")
                .unwrap()
                .downcast::<ComboRow>()
                .unwrap();
            let password = named(&root, "settings-obs-password")
                .unwrap()
                .downcast::<PasswordEntryRow>()
                .unwrap();
            let host = named(&root, "settings-obs-host")
                .unwrap()
                .downcast::<EntryRow>()
                .unwrap();
            let name = named(&root, "settings-connection-name")
                .unwrap()
                .downcast::<EntryRow>()
                .unwrap();
            assert!(
                !group.is_sensitive(),
                "the editor waits for its own keyring read"
            );
            flush();
            assert!(group.is_sensitive());
            assert_eq!(password.text(), "original-secret");

            let (release, hold) = std::sync::mpsc::channel();
            *secrets.hold_write.lock().unwrap() = Some(hold);
            password.set_text("updated-original");
            password.emit_by_name::<()>("apply", &[]);
            picker.set_selected(1);
            assert_eq!(nav.state.borrow().config.connection_id, "event");
            assert!(password.text().is_empty());
            assert!(!group.is_sensitive());
            release.send(()).unwrap();
            flush();
            assert_eq!(host.text(), "event.local");
            assert_eq!(password.text(), "event-secret");
            assert!(!format!("{:?}", nav.state.borrow()).contains("event-secret"));
            assert_eq!(*secrets.writes.lock().unwrap(), ["default"]);

            picker.set_selected(0);
            flush();
            assert_eq!(password.text(), "updated-original");
            named(&root, "settings-connection-add")
                .unwrap()
                .downcast::<gtk4::Button>()
                .unwrap()
                .emit_clicked();
            flush();
            let new_id = nav.state.borrow().config.connection_id.clone();
            assert_ne!(new_id, "default");
            assert_ne!(new_id, "event");
            assert!(password.text().is_empty());
            name.set_text("Podcast");
            host.set_text("podcast.local");
            name.emit_by_name::<()>("apply", &[]);
            flush();
            assert_eq!(nav.state.borrow().config.connection_id, new_id);
            let loaded = crate::storage::config::read_config();
            assert_eq!(loaded.config.connection_id, new_id);
            assert_eq!(loaded.config.obs.host, "podcast.local");
            assert_eq!(loaded.config.connection_profiles().len(), 3);
            let raw = std::fs::read_to_string(crate::storage::xdg::config_path()).unwrap();
            assert!(
                !raw.contains("original-secret")
                    && !raw.contains("event-secret")
                    && !raw.contains("updated-original")
            );

            secrets.fail_read.store(true, Ordering::SeqCst);
            picker.set_selected(1);
            flush();
            assert!(password.text().is_empty());
            assert!(group.is_sensitive());
        });
    }
}
