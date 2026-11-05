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
    infra::{error::AppError, i18n::LANGUAGE_LOADER},
    storage::{
        config::{AppConfig, ObsConfig, SavedConnection},
        secret,
    },
    ui::{navigation::NavigationContext, persist::persist_config_with, string_list},
};

trait ConnectionSecrets: Send + Sync {
    fn read(&self, id: &str) -> Result<Option<String>, AppError>;
    fn write(&self, id: &str, password: Option<&str>) -> Result<(), AppError>;
}

struct KeyringSecrets;
impl ConnectionSecrets for KeyringSecrets {
    fn read(&self, id: &str) -> Result<Option<String>, AppError> {
        secret::get_obs_password(id)
    }
    fn write(&self, id: &str, password: Option<&str>) -> Result<(), AppError> {
        match password {
            Some(password) => secret::set_obs_password(id, password),
            None => secret::delete_obs_password(id),
        }
    }
}

struct Editor {
    nav: NavigationContext,
    secrets: Arc<dyn ConnectionSecrets>,
    group: glib::WeakRef<PreferencesGroup>,
    picker: glib::WeakRef<ComboRow>,
    name: glib::WeakRef<EntryRow>,
    host: glib::WeakRef<EntryRow>,
    port: glib::WeakRef<EntryRow>,
    password: glib::WeakRef<PasswordEntryRow>,
    status: glib::WeakRef<ActionRow>,
    password_status: glib::WeakRef<ActionRow>,
    profiles: RefCell<Vec<(String, String)>>,
    updating: Cell<bool>,
    generation: Cell<u64>,
    writes: RefCell<super::PasswordWrites>,
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
        if let Some(password) = self.password.upgrade() {
            password.set_text(state.obs_password.as_deref().unwrap_or(""));
        }
    }

    fn change_selection(self: &Rc<Self>, id: String, update: impl FnOnce(&mut AppConfig)) {
        self.generation.set(self.generation.get().wrapping_add(1));
        let generation = self.generation.get();
        self.busy(true);
        self.nav.state.borrow_mut().obs_password = None;
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

    fn load_password(self: &Rc<Self>, id: String, generation: u64) {
        let destination = id.clone();
        let secrets = self.secrets.clone();
        let editor = self.clone();
        crate::ui::background_io::run_serialized(
            crate::ui::background_io::SerialLane::Secret,
            move || secrets.read(&destination),
            move |result| {
                if !editor.current(generation, &id) {
                    return;
                }
                match result {
                    Ok(password) => {
                        editor.nav.state.borrow_mut().obs_password = password.clone();
                        *editor.writes.borrow_mut() = super::PasswordWrites::new(password.clone());
                        if let Some(row) = editor.password.upgrade() {
                            row.set_text(password.as_deref().unwrap_or(""));
                        }
                        if let Some(status) = editor.password_status.upgrade() {
                            status.set_subtitle("");
                        }
                    }
                    Err(error) => {
                        editor.nav.state.borrow_mut().obs_password = None;
                        *editor.writes.borrow_mut() = super::PasswordWrites::new(None);
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
            move |result, _| {
                if !editor.current(generation, &id) {
                    return;
                }
                editor.report_save(result);
                editor.refresh_picker();
                editor.busy(false);
            },
        );
    }

    fn save_password(self: &Rc<Self>, password: Option<String>) {
        let id = self.nav.state.borrow().config.connection_id.clone();
        let generation = self.generation.get();
        let destination = id.clone();
        let secrets = self.secrets.clone();
        let editor = self.clone();
        self.writes.borrow().submit(
            password,
            move |password| secrets.write(&destination, password.as_deref()),
            move |result, confirmed| {
                if !editor.current(generation, &id) {
                    return;
                }
                editor.nav.state.borrow_mut().obs_password = confirmed;
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
                editor.busy(false);
                if saved {
                    editor.nav.dispatch(AppCommand::Connect);
                }
            },
        );
    }
}

pub(super) fn build(nav: &NavigationContext) -> (PreferencesGroup, ActionRow) {
    build_with_secrets(nav, Arc::new(KeyringSecrets))
}

fn build_with_secrets(
    nav: &NavigationContext,
    secrets: Arc<dyn ConnectionSecrets>,
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
        secrets,
        group: group.downgrade(),
        picker: picker.downgrade(),
        name: name.downgrade(),
        host: host.downgrade(),
        port: port.downgrade(),
        password: password.downgrade(),
        status: status.downgrade(),
        password_status: password_status.downgrade(),
        profiles: RefCell::new(Vec::new()),
        updating: Cell::new(false),
        generation: Cell::new(0),
        writes: RefCell::new(super::PasswordWrites::new(
            nav.state.borrow().obs_password.clone(),
        )),
    });
    editor.refresh_fields();
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
        move |row| editor.save_password((!row.text().is_empty()).then(|| row.text().to_string()))
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
        hold_write: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
        fail_read: AtomicBool,
    }

    impl ConnectionSecrets for FakeSecrets {
        fn read(&self, id: &str) -> Result<Option<String>, AppError> {
            if self.fail_read.load(Ordering::SeqCst) {
                return Err(AppError::storage(std::io::Error::other(
                    "fixture read failed",
                )));
            }
            Ok(self.passwords.lock().unwrap().get(id).cloned())
        }
        fn write(&self, id: &str, password: Option<&str>) -> Result<(), AppError> {
            if let Some(hold) = self.hold_write.lock().unwrap().take() {
                hold.recv_timeout(Duration::from_secs(10)).unwrap();
            }
            self.writes.lock().unwrap().push(id.into());
            match password {
                Some(password) => {
                    self.passwords
                        .lock()
                        .unwrap()
                        .insert(id.into(), password.into());
                }
                None => {
                    self.passwords.lock().unwrap().remove(id);
                }
            }
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
        crate::ui::test_support::native_test(
            "ui::pages::settings::connections::tests::native_connect_reads_confirmed_selection_and_refuses_failed_save",
            || {
                use crate::controller::{app_controller::AppController, dependencies::{AppConfigProvider, ObsPasswordProvider, ControllerDependencies}, state::AppState};
                #[derive(Default)]
                struct Observed(Mutex<Option<(String, String)>>);
                impl AppConfigProvider for Observed {
                    fn load_config(&self) -> AppConfig {
                        let config = crate::storage::config::read_config().config;
                        *self.0.lock().unwrap() = Some((config.connection_id.clone(), config.obs.host.clone()));
                        config
                    }
                }
                impl ObsPasswordProvider for Observed {
                    fn obs_password(&self, id: &str) -> Result<Option<String>, AppError> {
                        assert_eq!(id, "event");
                        Err(AppError::storage(std::io::Error::other("fixture stops before OBS connection")))
                    }
                }
                let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
                let observed = Arc::new(Observed::default());
                let (tx, _events) = std::sync::mpsc::sync_channel(128);
                let controller = Rc::new(RefCell::new(AppController::with_dependencies(runtime.handle().clone(), tx,
                    ControllerDependencies::new(observed.clone(), observed.clone()))));
                let mut config = AppConfig::default();
                config.save_connection(SavedConnection { id: "event".into(), name: "Event".into(), obs: ObsConfig { host: "event.local".into(), port: 4456 } });
                config.select_connection("event");
                let state = Rc::new(RefCell::new(AppState::new(config, Default::default(), None, None)));
                let stack = gtk4::Stack::new();
                let nav = NavigationContext::new(state, stack, controller);
                let (group, _status) = build_with_secrets(&nav, Arc::new(FakeSecrets::default()));
                let connect = named(&group.clone().upcast(), "settings-connection-connect").unwrap().downcast::<gtk4::Button>().unwrap();
                connect.emit_clicked();
                assert!(observed.0.lock().unwrap().is_none(), "connect must wait for GTK publication of the config save");
                flush();
                assert!(crate::ui::test_support::pump_until(Duration::from_secs(5), || observed.0.lock().unwrap().is_some()));
                assert_eq!(*observed.0.lock().unwrap(), Some(("event".into(), "event.local".into())));
                *observed.0.lock().unwrap() = None;
                let path = crate::storage::xdg::config_path();
                let backup = path.with_extension("json.fixture");
                std::fs::rename(&path, &backup).unwrap();
                std::fs::create_dir(&path).unwrap();
                connect.emit_clicked();
                flush();
                crate::ui::test_support::pump(Duration::from_millis(100));
                assert!(observed.0.lock().unwrap().is_none(), "a failed save must not reconnect with another selection");
                std::fs::remove_dir(&path).unwrap();
                std::fs::rename(backup, path).unwrap();
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display; fake credentials and scratch config, no OBS access"]
    fn native_saved_connections_keep_delayed_credentials_and_endpoints_separate() {
        crate::ui::test_support::native_test(
            "ui::pages::settings::connections::tests::native_saved_connections_keep_delayed_credentials_and_endpoints_separate",
            || {
                let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
                nav.state.borrow_mut().config.save_connection(SavedConnection {
                    id: "event".into(), name: "Event laptop".into(), obs: ObsConfig { host: "event.local".into(), port: 4456 },
                });
                nav.state.borrow_mut().obs_password = Some("original-secret".into());
                let secrets = Arc::new(FakeSecrets::default());
                *secrets.passwords.lock().unwrap() = HashMap::from([
                    ("default".into(), "original-secret".into()), ("event".into(), "event-secret".into()),
                ]);
                let (group, _status) = build_with_secrets(&nav, secrets.clone());
                let root = group.clone().upcast::<gtk4::Widget>();
                let picker = named(&root, "settings-connection-picker").unwrap().downcast::<ComboRow>().unwrap();
                let password = named(&root, "settings-obs-password").unwrap().downcast::<PasswordEntryRow>().unwrap();
                let host = named(&root, "settings-obs-host").unwrap().downcast::<EntryRow>().unwrap();
                let name = named(&root, "settings-connection-name").unwrap().downcast::<EntryRow>().unwrap();
                assert_eq!(password.text(), "original-secret");

                let (release, hold) = std::sync::mpsc::channel();
                *secrets.hold_write.lock().unwrap() = Some(hold);
                password.set_text("updated-original");
                password.emit_by_name::<()>("apply", &[]);
                picker.set_selected(1);
                assert_eq!(nav.state.borrow().config.connection_id, "event");
                assert!(nav.state.borrow().obs_password.is_none());
                assert!(password.text().is_empty());
                assert!(!group.is_sensitive());
                release.send(()).unwrap();
                flush();
                assert_eq!(host.text(), "event.local");
                assert_eq!(password.text(), "event-secret");
                assert_eq!(nav.state.borrow().obs_password.as_deref(), Some("event-secret"));
                assert_eq!(*secrets.writes.lock().unwrap(), ["default"]);

                picker.set_selected(0);
                flush();
                assert_eq!(password.text(), "updated-original");
                named(&root, "settings-connection-add").unwrap().downcast::<gtk4::Button>().unwrap().emit_clicked();
                flush();
                let new_id = nav.state.borrow().config.connection_id.clone();
                assert_ne!(new_id, "default");
                assert_ne!(new_id, "event");
                assert!(password.text().is_empty());
                assert!(nav.state.borrow().obs_password.is_none());
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
                assert!(!raw.contains("original-secret") && !raw.contains("event-secret") && !raw.contains("updated-original"));

                secrets.fail_read.store(true, Ordering::SeqCst);
                picker.set_selected(1);
                flush();
                assert!(password.text().is_empty());
                assert!(nav.state.borrow().obs_password.is_none());
                assert!(group.is_sensitive());
            },
        );
    }
}
