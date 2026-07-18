//! Settings page: appearance and OBS connection.

use std::path::PathBuf;
use std::rc::Rc;

use adw::{
    prelude::*, ActionRow, ComboRow, EntryRow, PasswordEntryRow, PreferencesGroup, PreferencesPage,
    SwitchRow,
};
use gtk4::StringList;
use i18n_embed_fl::fl;

use crate::controller::state::ObsStatus;
use crate::domain::appearance::{Language, ThemeId, ThemeMode};
use crate::infra::i18n;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::config::{write_config, OutputConfig};
use crate::storage::secret;
use crate::ui::navigation::NavigationContext;
use crate::ui::theme::ThemeManager;

use super::super::window::apply_color_scheme;

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let page = PreferencesPage::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-page-title"))
        .icon_name("preferences-system-symbolic")
        .build();
    page.add_css_class("app-page");
    page.add_css_class("settings-page");
    page.add_css_class("app-preferences-page");

    // ── Appearance ────────────────────────────────────────────────────────────
    let appearance_group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-appearance-title"))
        .description(fl!(LANGUAGE_LOADER, "settings-appearance-description"))
        .build();

    let cfg = nav.state.borrow().config.clone();

    let theme_mode_strings: Vec<String> = vec![
        fl!(LANGUAGE_LOADER, "settings-theme-mode-system"),
        fl!(LANGUAGE_LOADER, "settings-theme-mode-light"),
        fl!(LANGUAGE_LOADER, "settings-theme-mode-dark"),
    ];
    let theme_mode_names: Vec<&str> = theme_mode_strings.iter().map(|s| s.as_str()).collect();
    let theme_options = StringList::new(&theme_mode_names);
    let current_index = match nav.state.borrow().theme_mode {
        ThemeMode::System => 0u32,
        ThemeMode::Light => 1,
        ThemeMode::Dark => 2,
    };
    let theme_row = ComboRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-color-scheme-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-color-scheme-subtitle"))
        .model(&theme_options)
        .selected(current_index)
        .build();
    theme_row.add_css_class("scenedeck-combo-row");

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
            persist_config(&nav, |config| config.appearance.mode = mode);
            apply_theme_logging(nav.state.borrow().config.appearance.clone());
        }
    });

    appearance_group.add(&theme_row);

    let themes = ThemeManager::built_in_themes();
    let theme_name_strings: Vec<String> =
        themes.iter().map(|theme| theme.localized_name()).collect();
    let theme_names: Vec<&str> = theme_name_strings.iter().map(|s| s.as_str()).collect();
    let selected_theme_index = themes
        .iter()
        .position(|theme| theme.id == cfg.appearance.selected_theme_id())
        .unwrap_or(0) as u32;
    let theme_model = StringList::new(&theme_names);
    let selected_theme = themes
        .get(selected_theme_index as usize)
        .copied()
        .unwrap_or(themes[0]);
    let family_row = ComboRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-theme-title"))
        .subtitle(theme_subtitle(selected_theme))
        .model(&theme_model)
        .selected(selected_theme_index)
        .build();
    family_row.add_css_class("scenedeck-combo-row");

    let theme_status_row = ActionRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-theme-status-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-theme-status-initial"))
        .build();

    family_row.connect_selected_notify({
        let theme_status_row = theme_status_row.clone();
        let nav = nav.clone();
        move |row| {
            let selected = row.selected() as usize;
            let Some(theme) = ThemeManager::built_in_themes().get(selected).copied() else {
                return;
            };

            let row = row.clone();
            let theme_status_row = theme_status_row.clone();
            persist_config_with(
                &nav,
                move |config| config.appearance.selected_theme = Some(ThemeId::new(theme.id)),
                move |result, config| match result {
                    Ok(()) => {
                        row.set_subtitle(&theme_subtitle(theme));
                        apply_theme_with_status(config.appearance, theme_status_row);
                    }
                    Err(err) => theme_status_row.set_subtitle(&fl!(
                        LANGUAGE_LOADER,
                        "settings-failed-to-save",
                        err = err.to_string()
                    )),
                },
            );
        }
    });

    appearance_group.add(&family_row);

    let custom_css_row = SwitchRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-custom-css-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-custom-css-subtitle"))
        .active(cfg.appearance.custom_css.enabled)
        .build();

    let light_css_row = EntryRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-custom-light-css-title"))
        .text(path_text(cfg.appearance.custom_css.light_path.as_ref()))
        .show_apply_button(true)
        .build();

    let dark_css_row = EntryRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-custom-dark-css-title"))
        .text(path_text(cfg.appearance.custom_css.dark_path.as_ref()))
        .show_apply_button(true)
        .build();

    custom_css_row.connect_active_notify({
        let theme_status_row = theme_status_row.clone();
        let nav = nav.clone();
        move |row| {
            let active = row.is_active();
            let theme_status_row = theme_status_row.clone();
            persist_config_with(
                &nav,
                move |config| config.appearance.custom_css.enabled = active,
                move |result, config| match result {
                    Ok(()) => {
                        apply_theme_with_status(config.appearance, theme_status_row);
                    }
                    Err(err) => theme_status_row.set_subtitle(&fl!(
                        LANGUAGE_LOADER,
                        "settings-failed-to-save",
                        err = err.to_string()
                    )),
                },
            );
        }
    });

    light_css_row.connect_apply({
        let theme_status_row = theme_status_row.clone();
        let nav = nav.clone();
        move |row| save_custom_css_path(row, CssPathKind::Light, &theme_status_row, &nav)
    });

    dark_css_row.connect_apply({
        let theme_status_row = theme_status_row.clone();
        let nav = nav.clone();
        move |row| save_custom_css_path(row, CssPathKind::Dark, &theme_status_row, &nav)
    });

    let reload_css_row = ActionRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-reload-css-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-reload-css-subtitle"))
        .build();
    let reload_btn = gtk4::Button::builder()
        .label(fl!(LANGUAGE_LOADER, "settings-reload-button"))
        .valign(gtk4::Align::Center)
        .build();
    reload_btn.add_css_class("flat");
    reload_btn.connect_clicked({
        let theme_status_row = theme_status_row.clone();
        let nav = nav.clone();
        move |_| {
            let cfg = nav.state.borrow().config.clone();
            apply_theme_with_status(cfg.appearance, theme_status_row.clone());
        }
    });
    reload_css_row.add_suffix(&reload_btn);

    appearance_group.add(&custom_css_row);
    appearance_group.add(&light_css_row);
    appearance_group.add(&dark_css_row);
    appearance_group.add(&reload_css_row);
    appearance_group.add(&theme_status_row);

    // ── Language ──────────────────────────────────────────────────────────────
    let language_group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-language-title"))
        .description(fl!(LANGUAGE_LOADER, "settings-language-description"))
        .build();

    let language_names: Vec<&str> = Language::ALL.iter().map(|l| l.display_name()).collect();
    let language_model = StringList::new(&language_names);
    let selected_language_index = Language::ALL
        .iter()
        .position(|l| *l == cfg.language)
        .unwrap_or(0) as u32;
    let language_row = ComboRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-display-language-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-display-language-subtitle"))
        .model(&language_model)
        .selected(selected_language_index)
        .build();
    language_row.add_css_class("scenedeck-combo-row");

    let language_status_row = ActionRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-language-status-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-language-status-initial"))
        .build();

    language_row.connect_selected_notify({
        let language_status_row = language_status_row.clone();
        let nav = nav.clone();
        move |row| {
            let selected = row.selected() as usize;
            let Some(language) = Language::ALL.get(selected).copied() else {
                return;
            };

            let language_status_row = language_status_row.clone();
            persist_config_with(
                &nav,
                move |config| config.language = language,
                move |result, _| match result {
                    Ok(()) => {
                        i18n::init(language);
                        language_status_row
                            .set_subtitle(&fl!(LANGUAGE_LOADER, "settings-language-saved"));
                    }
                    Err(err) => {
                        language_status_row.set_subtitle(&fl!(
                            LANGUAGE_LOADER,
                            "settings-failed-to-save",
                            err = err.to_string()
                        ));
                    }
                },
            );
        }
    });

    language_group.add(&language_row);
    language_group.add(&language_status_row);

    // ── OBS Connection ────────────────────────────────────────────────────────

    let obs_group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-obs-connection-title"))
        .description(fl!(LANGUAGE_LOADER, "settings-obs-connection-description"))
        .build();

    let host_row = EntryRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-host-title"))
        .text(&cfg.obs.host)
        .show_apply_button(true)
        .build();

    let port_row = EntryRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-port-title"))
        .text(cfg.obs.port.to_string())
        .show_apply_button(true)
        .build();

    // Password is stored in the system keyring, never in config.json.
    let password_row = PasswordEntryRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-password-title"))
        .show_apply_button(true)
        .build();
    if let Some(existing) = nav.state.borrow().obs_password.as_ref() {
        password_row.set_text(existing);
    }

    let status_row = ActionRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-obs-status-title"))
        .subtitle(obs_status_text(&nav))
        .build();

    let save_handler = {
        let nav = nav.clone();
        let host_row = host_row.clone();
        let port_row = port_row.clone();
        let status_row = status_row.clone();
        move || {
            let host = host_row.text().trim().to_string();
            let port: u16 = match port_row.text().trim().parse() {
                Ok(p) => p,
                Err(_) => {
                    status_row.set_subtitle(&fl!(LANGUAGE_LOADER, "settings-invalid-port"));
                    return;
                }
            };
            let status_row = status_row.clone();
            persist_config_with(
                &nav,
                move |config| {
                    config.obs.host = host;
                    config.obs.port = port;
                },
                move |result, _| match result {
                    Ok(()) => status_row.set_subtitle(&fl!(LANGUAGE_LOADER, "settings-saved")),
                    Err(err) => status_row.set_subtitle(&fl!(
                        LANGUAGE_LOADER,
                        "settings-failed-to-save",
                        err = err.to_string()
                    )),
                },
            );
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
        let nav = nav.clone();
        move |row| {
            let password = (!row.text().is_empty()).then(|| row.text().to_string());
            nav.state.borrow_mut().obs_password = password.clone();
            let status_row = status_row.clone();
            crate::ui::background_io::run(
                move || match password {
                    Some(password) => secret::set_obs_password(&password),
                    None => secret::delete_obs_password(),
                },
                move |result| match result {
                    Ok(()) => {
                        status_row.set_subtitle(&fl!(LANGUAGE_LOADER, "settings-password-saved"))
                    }
                    Err(error) => status_row.set_subtitle(&fl!(
                        LANGUAGE_LOADER,
                        "settings-keyring-error",
                        err = error.to_string()
                    )),
                },
            );
        }
    });

    obs_group.add(&host_row);
    obs_group.add(&port_row);
    obs_group.add(&password_row);

    // ── Output safety ────────────────────────────────────────────────────────
    let output_group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-output-safety-title"))
        .description(fl!(LANGUAGE_LOADER, "settings-output-safety-description"))
        .build();

    let confirm_start_stream = output_switch_row(
        &fl!(LANGUAGE_LOADER, "settings-confirm-start-stream-title"),
        &fl!(LANGUAGE_LOADER, "settings-confirm-start-stream-subtitle"),
        cfg.outputs.confirm_start_stream,
    );
    let confirm_stop_stream = output_switch_row(
        &fl!(LANGUAGE_LOADER, "settings-confirm-stop-stream-title"),
        &fl!(LANGUAGE_LOADER, "settings-confirm-stop-stream-subtitle"),
        cfg.outputs.confirm_stop_stream,
    );
    let confirm_start_recording = output_switch_row(
        &fl!(LANGUAGE_LOADER, "settings-confirm-start-recording-title"),
        &fl!(LANGUAGE_LOADER, "settings-confirm-start-recording-subtitle"),
        cfg.outputs.confirm_start_recording,
    );
    let confirm_stop_recording = output_switch_row(
        &fl!(LANGUAGE_LOADER, "settings-confirm-stop-recording-title"),
        &fl!(LANGUAGE_LOADER, "settings-confirm-stop-recording-subtitle"),
        cfg.outputs.confirm_stop_recording,
    );

    connect_output_switch(&confirm_start_stream, &nav, |outputs, active| {
        outputs.confirm_start_stream = active;
    });
    connect_output_switch(&confirm_stop_stream, &nav, |outputs, active| {
        outputs.confirm_stop_stream = active;
    });
    connect_output_switch(&confirm_start_recording, &nav, |outputs, active| {
        outputs.confirm_start_recording = active;
    });
    connect_output_switch(&confirm_stop_recording, &nav, |outputs, active| {
        outputs.confirm_stop_recording = active;
    });

    output_group.add(&confirm_start_stream);
    output_group.add(&confirm_stop_stream);
    output_group.add(&confirm_start_recording);
    output_group.add(&confirm_stop_recording);

    let status_group = PreferencesGroup::new();
    status_group.add(&status_row);

    page.add(&appearance_group);
    page.add(&language_group);
    page.add(&obs_group);
    page.add(&output_group);
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
        ObsStatus::Disconnected => fl!(LANGUAGE_LOADER, "settings-obs-not-connected"),
        ObsStatus::Connecting => fl!(LANGUAGE_LOADER, "settings-obs-connecting"),
        ObsStatus::Connected { obs_version } => {
            fl!(
                LANGUAGE_LOADER,
                "settings-obs-connected",
                version = obs_version
            )
        }
        ObsStatus::Error(e) => fl!(LANGUAGE_LOADER, "settings-obs-error", err = e),
    }
}

fn persist_config(
    nav: &NavigationContext,
    update: impl FnOnce(&mut crate::storage::config::AppConfig),
) {
    persist_config_with(nav, update, |result, _| {
        if let Err(error) = result {
            tracing::warn!(%error, "failed to save configuration");
        }
    });
}

fn persist_config_with<Update, Complete>(
    nav: &NavigationContext,
    update: Update,
    complete: Complete,
) where
    Update: FnOnce(&mut crate::storage::config::AppConfig),
    Complete: FnOnce(std::io::Result<()>, crate::storage::config::AppConfig) + 'static,
{
    let config = {
        let mut state = nav.state.borrow_mut();
        update(&mut state.config);
        state.config.clone()
    };
    let persisted = config.clone();
    crate::ui::background_io::run(
        move || write_config(&persisted),
        move |result| complete(result, config),
    );
}

fn output_switch_row(title: &str, subtitle: &str, active: bool) -> SwitchRow {
    SwitchRow::builder()
        .title(title)
        .subtitle(subtitle)
        .active(active)
        .build()
}

fn connect_output_switch<F>(row: &SwitchRow, nav: &NavigationContext, update: F)
where
    F: Fn(&mut OutputConfig, bool) + 'static,
{
    row.connect_active_notify({
        let nav = nav.clone();
        move |row| {
            let active = row.is_active();
            {
                let mut state = nav.state.borrow_mut();
                update(&mut state.config.outputs, active);
                state.output_confirmations = state.config.outputs.clone();
            }
            persist_config(&nav, |_| {});
        }
    });
}

fn theme_subtitle(theme: crate::ui::theme::BuiltInTheme) -> String {
    fl!(
        LANGUAGE_LOADER,
        "settings-theme-subtitle",
        description = theme.localized_description(),
        swatches = theme.swatches.join(", ")
    )
}

fn theme_report_text(report: &crate::ui::theme::ThemeApplyReport) -> String {
    if report.is_ok() {
        fl!(
            LANGUAGE_LOADER,
            "settings-theme-loaded",
            theme = report.theme_id.as_str(),
            variant = format!("{:?}", report.variant)
        )
    } else {
        report
            .user_message()
            .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "settings-theme-loaded-with-warnings"))
    }
}

fn apply_theme_with_status(
    preference: crate::domain::appearance::ThemePreference,
    status_row: ActionRow,
) {
    ThemeManager::apply_async(preference, move |report| {
        status_row.set_subtitle(&theme_report_text(&report));
    });
}

fn apply_theme_logging(preference: crate::domain::appearance::ThemePreference) {
    ThemeManager::apply_async(preference, |report| {
        for warning in report.warnings {
            tracing::warn!(%warning, "theme warning");
        }
    });
}

#[derive(Debug, Clone, Copy)]
enum CssPathKind {
    Light,
    Dark,
}

fn save_custom_css_path(
    row: &EntryRow,
    kind: CssPathKind,
    status_row: &ActionRow,
    nav: &NavigationContext,
) {
    let text = row.text().trim().to_string();
    let path = if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    };

    let status_row = status_row.clone();
    persist_config_with(
        nav,
        move |config| match kind {
            CssPathKind::Light => config.appearance.custom_css.light_path = path,
            CssPathKind::Dark => config.appearance.custom_css.dark_path = path,
        },
        move |result, config| match result {
            Ok(()) => {
                apply_theme_with_status(config.appearance, status_row);
            }
            Err(err) => status_row.set_subtitle(&fl!(
                LANGUAGE_LOADER,
                "settings-failed-to-save",
                err = err.to_string()
            )),
        },
    );
}

fn path_text(path: Option<&PathBuf>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_default()
}
