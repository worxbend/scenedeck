//! Settings page: appearance and OBS connection.

use std::rc::Rc;

use adw::{
    prelude::*, ActionRow, ComboRow, EntryRow, PasswordEntryRow, PreferencesGroup, PreferencesPage,
};
use gtk4::StringList;

use crate::controller::state::ObsStatus;
use crate::domain::appearance::ThemeMode;
use crate::storage::config::write_config;
use crate::storage::secret;
use crate::ui::navigation::NavigationContext;

use super::super::window::apply_color_scheme;

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let page = PreferencesPage::builder()
        .title("Settings")
        .icon_name("preferences-system-symbolic")
        .build();

    // ── Appearance ────────────────────────────────────────────────────────────
    let appearance_group = PreferencesGroup::builder()
        .title("Appearance")
        .description("GNOME apps should follow the system style by default.")
        .build();

    let theme_options = StringList::new(&["System", "Light", "Dark"]);
    let current_index = match nav.state.borrow().theme_mode {
        ThemeMode::System => 0u32,
        ThemeMode::Light => 1,
        ThemeMode::Dark => 2,
    };
    let theme_row = ComboRow::builder()
        .title("Color Scheme")
        .subtitle("Follow the system preference or force light / dark")
        .model(&theme_options)
        .selected(current_index)
        .build();

    theme_row.connect_selected_notify({
        let nav = nav.clone();
        move |row| {
            let mode = match row.selected() {
                1 => ThemeMode::Light,
                2 => ThemeMode::Dark,
                _ => ThemeMode::System,
            };
            nav.state.borrow_mut().set_theme_mode(mode);
            apply_color_scheme(&adw::StyleManager::default(), mode);
            if let Err(err) = persist_config(&nav) {
                tracing::warn!(%err, "failed to save theme preference");
            }
        }
    });

    appearance_group.add(&theme_row);

    // ── OBS Connection ────────────────────────────────────────────────────────
    let cfg = crate::storage::config::read_config().config;

    let obs_group = PreferencesGroup::builder()
        .title("OBS Connection")
        .description("WebSocket settings for OBS Studio (default port: 4455).")
        .build();

    let host_row = EntryRow::builder()
        .title("Host")
        .text(&cfg.obs.host)
        .show_apply_button(true)
        .build();

    let port_row = EntryRow::builder()
        .title("Port")
        .text(cfg.obs.port.to_string())
        .show_apply_button(true)
        .build();

    // Password is stored in the system keyring, never in config.json.
    let password_row = PasswordEntryRow::builder()
        .title("Password (optional)")
        .show_apply_button(true)
        .build();
    if let Ok(Some(existing)) = secret::get_obs_password() {
        password_row.set_text(&existing);
    }

    let status_row = ActionRow::builder()
        .title("OBS Status")
        .subtitle(obs_status_text(&nav))
        .build();

    let save_handler = {
        let _nav = nav.clone();
        let host_row = host_row.clone();
        let port_row = port_row.clone();
        let status_row = status_row.clone();
        move || {
            let host = host_row.text().trim().to_string();
            let port: u16 = match port_row.text().trim().parse() {
                Ok(p) => p,
                Err(_) => {
                    status_row.set_subtitle("Invalid port number.");
                    return;
                }
            };
            // Persist to disk
            let mut cfg = crate::storage::config::read_config().config;
            cfg.obs.host = host;
            cfg.obs.port = port;
            match write_config(&cfg) {
                Ok(()) => status_row.set_subtitle("Settings saved."),
                Err(err) => status_row.set_subtitle(&format!("Failed to save: {err}")),
            }
        }
    };

    host_row.connect_apply({
        let save = save_handler.clone();
        move |_| save()
    });
    port_row.connect_apply({
        let save = save_handler.clone();
        move |_| save()
    });

    password_row.connect_apply({
        let status_row = status_row.clone();
        move |row| {
            let text = row.text();
            let result = if text.is_empty() {
                secret::delete_obs_password()
            } else {
                secret::set_obs_password(&text)
            };
            match result {
                Ok(()) => status_row.set_subtitle("Password saved to keyring."),
                Err(err) => status_row.set_subtitle(&format!("Keyring error: {err}")),
            }
        }
    });

    obs_group.add(&host_row);
    obs_group.add(&port_row);
    obs_group.add(&password_row);

    let status_group = PreferencesGroup::new();
    status_group.add(&status_row);

    page.add(&appearance_group);
    page.add(&obs_group);
    page.add(&status_group);

    // Closure that refreshes the status row when navigating back to this page
    // (or when the external refresh button is pressed).
    let refresh_fn: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        let status_row = status_row.clone();
        move || status_row.set_subtitle(&obs_status_text(&nav))
    });

    page.connect_map({
        let refresh = refresh_fn.clone();
        move |_| refresh()
    });

    (page.upcast(), refresh_fn)
}

fn obs_status_text(nav: &NavigationContext) -> String {
    match nav.state.borrow().obs_status.clone() {
        ObsStatus::Disconnected => "Not connected to OBS.".to_string(),
        ObsStatus::Connecting => "Connecting to OBS…".to_string(),
        ObsStatus::Connected { obs_version } => format!("Connected — OBS {obs_version}"),
        ObsStatus::Error(e) => format!("Error: {e}"),
    }
}

fn persist_config(nav: &NavigationContext) -> Result<(), std::io::Error> {
    let model = nav.state.borrow();
    let mut cfg = crate::storage::config::read_config().config;
    cfg.appearance.mode = model.theme_mode;
    write_config(&cfg)
}
