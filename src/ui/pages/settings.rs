//! Settings page: appearance and OBS connection.

use std::path::PathBuf;
use std::rc::Rc;

use crate::ui::string_list;
use adw::{
    prelude::*, ActionRow, ComboRow, EntryRow, PasswordEntryRow, PreferencesGroup, PreferencesPage,
    SwitchRow,
};
use i18n_embed_fl::fl;

use crate::controller::state::ObsStatus;
use crate::domain::appearance::{Language, MotionLevel, ThemeId, ThemeMode};
use crate::domain::hotkey::{
    LeaderKey, SceneHotkeyConfig, SceneHotkeyStyle, MAX_LEADER_TIMEOUT_MS, MAX_SLOTS,
    MIN_LEADER_TIMEOUT_MS,
};
use crate::infra::i18n;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::config::{write_config, AppConfig, OutputConfig};
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

    let cfg = nav.state.borrow().config.clone();

    let appearance_group = build_appearance_group(&nav, &cfg);
    let language_group = build_language_group(&nav, &cfg);
    let (obs_group, status_row) = build_obs_group(&nav, &cfg);
    let output_group = build_output_group(&nav, &cfg);
    let hotkey_group = build_hotkey_group(&nav);

    let status_group = PreferencesGroup::new();
    with_icon(&status_row, "nf-md-lan-connect-symbolic");
    status_group.add(&status_row);

    page.add(&appearance_group);
    page.add(&language_group);
    page.add(&obs_group);
    page.add(&output_group);
    page.add(&hotkey_group);
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

/// Colour scheme, motion, theme family, and custom CSS.
/// The Colour Scheme row: follow the system, or force light or dark.
fn build_color_scheme_row(nav: &NavigationContext) -> ComboRow {
    let theme_mode_strings: Vec<String> = vec![
        fl!(LANGUAGE_LOADER, "settings-theme-mode-system"),
        fl!(LANGUAGE_LOADER, "settings-theme-mode-light"),
        fl!(LANGUAGE_LOADER, "settings-theme-mode-dark"),
    ];
    let theme_options = string_list(&theme_mode_strings);
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

    theme_row
}

/// The Motion row: how much the interface is allowed to animate.
///
/// Motion sits directly under the colour scheme because it is the same kind of
/// choice: how the app should look, not what it should do.
fn build_motion_row(nav: &NavigationContext, cfg: &AppConfig) -> ComboRow {
    let motion_strings: Vec<String> = vec![
        fl!(LANGUAGE_LOADER, "settings-motion-full"),
        fl!(LANGUAGE_LOADER, "settings-motion-reduced"),
        fl!(LANGUAGE_LOADER, "settings-motion-off"),
    ];
    let motion_options = string_list(&motion_strings);
    let motion_index = MotionLevel::ALL
        .iter()
        .position(|level| *level == cfg.appearance.motion)
        .unwrap_or(0) as u32;
    let motion_row = ComboRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-motion-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-motion-subtitle"))
        .model(&motion_options)
        .selected(motion_index)
        .build();
    motion_row.add_css_class("scenedeck-combo-row");

    motion_row.connect_selected_notify({
        let nav = nav.clone();
        move |row| {
            let motion = MotionLevel::ALL
                .get(row.selected() as usize)
                .copied()
                .unwrap_or_default();
            persist_config(&nav, |config| config.appearance.motion = motion);
            // Motion is enforced by a stylesheet layer, so re-applying the
            // theme is what actually starts or stops the animations. The change
            // is visible immediately, with no restart.
            apply_theme_logging(nav.state.borrow().config.appearance.clone());
        }
    });

    motion_row
}

/// The four rows that make up the custom-CSS block.
struct CustomCssRows {
    enabled: SwitchRow,
    light: EntryRow,
    dark: EntryRow,
    reload: ActionRow,
}

/// Build the custom-CSS rows.
///
/// All four report what happened into the shared theme status row, which is
/// why it is passed in rather than made here: it is also written to by the
/// theme family row above them.
fn build_custom_css_rows(
    nav: &NavigationContext,
    cfg: &AppConfig,
    theme_status_row: &ActionRow,
) -> CustomCssRows {
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
            // Re-read the config: the point of Reload is to pick up edits made
            // since the page was built.
            let cfg = nav.state.borrow().config.clone();
            apply_theme_with_status(cfg.appearance, theme_status_row.clone());
        }
    });
    reload_css_row.add_suffix(&reload_btn);

    CustomCssRows {
        enabled: custom_css_row,
        light: light_css_row,
        dark: dark_css_row,
        reload: reload_css_row,
    }
}

fn build_appearance_group(nav: &NavigationContext, cfg: &AppConfig) -> PreferencesGroup {
    let appearance_group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-appearance-title"))
        .description(fl!(LANGUAGE_LOADER, "settings-appearance-description"))
        .build();

    let theme_row = build_color_scheme_row(nav);
    with_icon(&theme_row, "nf-md-theme-light-dark-symbolic");
    appearance_group.add(&theme_row);

    let motion_row = build_motion_row(nav, cfg);
    with_icon(&motion_row, "nf-md-motion-play-outline-symbolic");
    appearance_group.add(&motion_row);

    let themes = ThemeManager::built_in_themes();
    let theme_name_strings: Vec<String> =
        themes.iter().map(|theme| theme.localized_name()).collect();
    let selected_theme_index = themes
        .iter()
        .position(|theme| theme.id == cfg.appearance.selected_theme_id())
        .unwrap_or(0) as u32;
    let theme_model = string_list(&theme_name_strings);
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

    with_icon(&family_row, "nf-md-palette-symbolic");
    appearance_group.add(&family_row);

    let CustomCssRows {
        enabled: custom_css_row,
        light: light_css_row,
        dark: dark_css_row,
        reload: reload_css_row,
    } = build_custom_css_rows(nav, cfg, &theme_status_row);

    with_icon(&custom_css_row, "nf-md-language-css3-symbolic");
    appearance_group.add(&custom_css_row);
    with_entry_icon(&light_css_row, "nf-md-white-balance-sunny-symbolic");
    appearance_group.add(&light_css_row);
    with_entry_icon(&dark_css_row, "nf-md-weather-night-symbolic");
    appearance_group.add(&dark_css_row);
    with_icon(&reload_css_row, "nf-md-refresh-symbolic");
    appearance_group.add(&reload_css_row);
    with_icon(&theme_status_row, "nf-md-information-outline-symbolic");
    appearance_group.add(&theme_status_row);

    appearance_group
}

/// Interface language, plus the note about restarting for a full change.
fn build_language_group(nav: &NavigationContext, cfg: &AppConfig) -> PreferencesGroup {
    let language_group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-language-title"))
        .description(fl!(LANGUAGE_LOADER, "settings-language-description"))
        .build();

    let language_names: Vec<&str> = Language::ALL.iter().map(|l| l.display_name()).collect();
    let language_model = gtk4::StringList::new(&language_names);
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

    with_icon(&language_row, "nf-md-translate-symbolic");
    language_group.add(&language_row);
    language_group.add(&language_status_row);

    language_group
}

/// OBS host, port, and password.
///
/// Returns the status row alongside the group: the row reports what happened
/// to a save and lives in its own group at the bottom of the page, but it is
/// these rows' handlers that write into it.
fn build_obs_group(nav: &NavigationContext, cfg: &AppConfig) -> (PreferencesGroup, ActionRow) {
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
        .subtitle(obs_status_text(nav))
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

    with_entry_icon(&host_row, "nf-md-server-network-symbolic");
    obs_group.add(&host_row);
    with_entry_icon(&port_row, "nf-md-ethernet-symbolic");
    obs_group.add(&port_row);
    with_entry_icon(&password_row, "nf-md-key-variant-symbolic");
    obs_group.add(&password_row);

    (obs_group, status_row)
}

/// Which output actions ask for confirmation first.
fn build_output_group(nav: &NavigationContext, cfg: &AppConfig) -> PreferencesGroup {
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

    connect_output_switch(&confirm_start_stream, nav, |outputs, active| {
        outputs.confirm_start_stream = active;
    });
    connect_output_switch(&confirm_stop_stream, nav, |outputs, active| {
        outputs.confirm_stop_stream = active;
    });
    connect_output_switch(&confirm_start_recording, nav, |outputs, active| {
        outputs.confirm_start_recording = active;
    });
    connect_output_switch(&confirm_stop_recording, nav, |outputs, active| {
        outputs.confirm_stop_recording = active;
    });

    with_icon(&confirm_start_stream, "nf-md-broadcast-symbolic");
    output_group.add(&confirm_start_stream);
    with_icon(&confirm_stop_stream, "nf-md-broadcast-off-symbolic");
    output_group.add(&confirm_stop_stream);
    with_icon(&confirm_start_recording, "nf-md-record-circle-symbolic");
    output_group.add(&confirm_start_recording);
    with_icon(&confirm_stop_recording, "nf-md-stop-circle-symbolic");
    output_group.add(&confirm_stop_recording);

    output_group
}

/// Build the Live scene-hotkey preferences group.
///
/// The rows write straight into `config.hotkeys`; the window's key controller
/// re-reads that on every key press, so edits here take effect immediately and
/// the Live page picks up new badges the next time its cards are rebuilt.
fn build_hotkey_group(nav: &NavigationContext) -> PreferencesGroup {
    let group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-hotkeys-title"))
        .description(fl!(LANGUAGE_LOADER, "settings-hotkeys-description"))
        .build();

    let hotkeys = nav.state.borrow().config.hotkeys;

    let enabled_row = SwitchRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-hotkeys-enabled-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-hotkeys-enabled-subtitle"))
        .active(hotkeys.enabled)
        .build();

    let style_strings: Vec<String> = SceneHotkeyStyle::ALL
        .iter()
        .map(|style| style.label())
        .collect();
    let style_row = ComboRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-hotkeys-style-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-hotkeys-style-subtitle"))
        .model(&string_list(&style_strings))
        .selected(index_of(&SceneHotkeyStyle::ALL, hotkeys.style))
        .build();
    style_row.add_css_class("scenedeck-combo-row");

    let leader_strings: Vec<String> = LeaderKey::ALL.iter().map(|key| key.label()).collect();
    let leader_row = ComboRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-hotkeys-leader-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "settings-hotkeys-leader-subtitle"))
        .model(&string_list(&leader_strings))
        .selected(index_of(&LeaderKey::ALL, hotkeys.leader))
        .build();
    leader_row.add_css_class("scenedeck-combo-row");

    let timeout_row = adw::SpinRow::with_range(
        MIN_LEADER_TIMEOUT_MS as f64,
        MAX_LEADER_TIMEOUT_MS as f64,
        50.0,
    );
    timeout_row.set_title(&fl!(LANGUAGE_LOADER, "settings-hotkeys-timeout-title"));
    timeout_row.set_subtitle(&fl!(LANGUAGE_LOADER, "settings-hotkeys-timeout-subtitle"));
    timeout_row.set_value(hotkeys.leader_timeout().as_millis() as f64);

    let preview_row = ActionRow::builder()
        .title(fl!(LANGUAGE_LOADER, "settings-hotkeys-preview-title"))
        .build();

    // One place decides what the dependent rows show, so every edit path stays
    // consistent without each handler re-deriving it.
    let refresh: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        let style_row = style_row.clone();
        let leader_row = leader_row.clone();
        let timeout_row = timeout_row.clone();
        let preview_row = preview_row.clone();
        move || {
            let hotkeys = nav.state.borrow().config.hotkeys;
            let leader_style = hotkeys.style == SceneHotkeyStyle::Leader;
            style_row.set_sensitive(hotkeys.enabled);
            leader_row.set_sensitive(hotkeys.enabled && leader_style);
            timeout_row.set_sensitive(hotkeys.enabled && leader_style);
            preview_row.set_subtitle(&hotkey_preview_text(&hotkeys));
        }
    });
    refresh();

    enabled_row.connect_active_notify({
        let nav = nav.clone();
        let refresh = refresh.clone();
        move |row| {
            let enabled = row.is_active();
            persist_config(&nav, |config| config.hotkeys.enabled = enabled);
            refresh();
        }
    });

    style_row.connect_selected_notify({
        let nav = nav.clone();
        let refresh = refresh.clone();
        move |row| {
            let Some(style) = SceneHotkeyStyle::ALL.get(row.selected() as usize).copied() else {
                return;
            };
            persist_config(&nav, |config| config.hotkeys.style = style);
            refresh();
        }
    });

    leader_row.connect_selected_notify({
        let nav = nav.clone();
        let refresh = refresh.clone();
        move |row| {
            let Some(leader) = LeaderKey::ALL.get(row.selected() as usize).copied() else {
                return;
            };
            persist_config(&nav, |config| config.hotkeys.leader = leader);
            refresh();
        }
    });

    timeout_row.connect_value_notify({
        let nav = nav.clone();
        let refresh = refresh.clone();
        move |row| {
            let millis = row.value().round().max(0.0) as u64;
            persist_config(&nav, |config| config.hotkeys.leader_timeout_ms = millis);
            refresh();
        }
    });

    with_icon(&enabled_row, "nf-md-keyboard-symbolic");
    group.add(&enabled_row);
    with_icon(&style_row, "nf-md-keyboard-variant-symbolic");
    group.add(&style_row);
    with_icon(&leader_row, "nf-md-keyboard-outline-symbolic");
    group.add(&leader_row);
    with_icon(&timeout_row, "nf-md-timer-outline-symbolic");
    group.add(&timeout_row);
    with_icon(&preview_row, "nf-md-eye-outline-symbolic");
    group.add(&preview_row);
    group
}

/// Summary of the first and last slot bindings, or a note that they are off.
fn hotkey_preview_text(hotkeys: &SceneHotkeyConfig) -> String {
    let (Some(first), Some(last)) = (
        hotkeys.shortcut_label(0),
        hotkeys.shortcut_label(MAX_SLOTS - 1),
    ) else {
        return fl!(LANGUAGE_LOADER, "settings-hotkeys-preview-disabled");
    };
    fl!(
        LANGUAGE_LOADER,
        "settings-hotkeys-preview-subtitle",
        first = first,
        last = last,
        count = MAX_SLOTS.to_string()
    )
}

fn index_of<T: PartialEq>(all: &[T], value: T) -> u32 {
    all.iter().position(|item| *item == value).unwrap_or(0) as u32
}

/// Prefix a preferences row with a small icon.
///
/// A wall of identical rows is hard to scan; the icon gives each one a shape to
/// find it by before the text is read. Entry rows carry `add_prefix` on their
/// own type rather than on `ActionRow`, hence the pair.
fn with_icon<R: IsA<adw::ActionRow>>(row: &R, icon_name: &str) {
    row.as_ref().add_prefix(&row_icon(icon_name));
}

/// Prefix an entry-style preferences row with a small icon.
fn with_entry_icon<R: IsA<adw::EntryRow>>(row: &R, icon_name: &str) {
    row.as_ref().add_prefix(&row_icon(icon_name));
}

fn row_icon(icon_name: &str) -> gtk4::Image {
    let icon = gtk4::Image::from_icon_name(icon_name);
    icon.add_css_class("scenedeck-row-icon");
    icon
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
