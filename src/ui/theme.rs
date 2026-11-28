//! Theme loading for built-in light/dark theme families and user CSS files.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::{gdk, CssProvider};
use i18n_embed_fl::fl;

use crate::domain::appearance::{MotionLevel, ThemeMode, ThemePreference};
use crate::infra::i18n::LANGUAGE_LOADER;

const BASE_CSS: &str = include_str!("../../assets/scenedeck.css");

thread_local! {
    static ACTIVE_PROVIDERS: RefCell<Vec<CssProvider>> = const { RefCell::new(Vec::new()) };
    static ACTIVE_PREFERENCE: RefCell<Option<ThemePreference>> = const { RefCell::new(None) };
    static REQUEST_GENERATION: Cell<u64> = const { Cell::new(0) };
    static DESKTOP_SIGNALS_INSTALLED: Cell<bool> = const { Cell::new(false) };
    static NATIVE_MOTION_OFF: Cell<bool> = const { Cell::new(false) };
    static UPDATING_NATIVE_MOTION: Cell<bool> = const { Cell::new(false) };
    static NATIVE_COLOR_SCHEME_OVERRIDDEN: Cell<bool> = const { Cell::new(false) };
    static UPDATING_NATIVE_COLOR_SCHEME: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeVariant {
    Light,
    Dark,
}

impl ThemeVariant {
    /// User-facing label for status messages.
    pub(crate) fn label(self) -> String {
        match self {
            Self::Light => fl!(LANGUAGE_LOADER, "theme-variant-light"),
            Self::Dark => fl!(LANGUAGE_LOADER, "theme-variant-dark"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltInTheme {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    light_css: &'static str,
    dark_css: &'static str,
}

impl BuiltInTheme {
    pub(crate) fn css_for(self, variant: ThemeVariant) -> &'static str {
        match variant {
            ThemeVariant::Light => self.light_css,
            ThemeVariant::Dark => self.dark_css,
        }
    }

    /// Localized display name shown in the Settings theme picker.
    pub(crate) fn localized_name(&self) -> String {
        match self.id {
            "adwaita-default" => fl!(LANGUAGE_LOADER, "theme-adwaita-default-name"),
            "scenedeck-dark" => fl!(LANGUAGE_LOADER, "theme-scenedeck-dark-name"),
            "scenedeck-light" => fl!(LANGUAGE_LOADER, "theme-scenedeck-light-name"),
            "obs" => fl!(LANGUAGE_LOADER, "theme-obs-name"),
            "obsidian" => fl!(LANGUAGE_LOADER, "theme-obsidian-name"),
            "nord" => fl!(LANGUAGE_LOADER, "theme-nord-name"),
            "dracula-inspired" => fl!(LANGUAGE_LOADER, "theme-dracula-inspired-name"),
            "solarized-dark" => fl!(LANGUAGE_LOADER, "theme-solarized-dark-name"),
            "high-contrast" => fl!(LANGUAGE_LOADER, "theme-high-contrast-name"),
            "stream-red" => fl!(LANGUAGE_LOADER, "theme-stream-red-name"),
            "studio-purple" => fl!(LANGUAGE_LOADER, "theme-studio-purple-name"),
            "ubuntu-violet" => fl!(LANGUAGE_LOADER, "theme-ubuntu-violet-name"),
            _ => self.name.to_string(),
        }
    }

    /// Localized description shown alongside the theme in the Settings picker.
    pub(crate) fn localized_description(&self) -> String {
        match self.id {
            "adwaita-default" => fl!(LANGUAGE_LOADER, "theme-adwaita-default-desc"),
            "scenedeck-dark" => fl!(LANGUAGE_LOADER, "theme-scenedeck-dark-desc"),
            "scenedeck-light" => fl!(LANGUAGE_LOADER, "theme-scenedeck-light-desc"),
            "obs" => fl!(LANGUAGE_LOADER, "theme-obs-desc"),
            "obsidian" => fl!(LANGUAGE_LOADER, "theme-obsidian-desc"),
            "nord" => fl!(LANGUAGE_LOADER, "theme-nord-desc"),
            "dracula-inspired" => fl!(LANGUAGE_LOADER, "theme-dracula-inspired-desc"),
            "solarized-dark" => fl!(LANGUAGE_LOADER, "theme-solarized-dark-desc"),
            "high-contrast" => fl!(LANGUAGE_LOADER, "theme-high-contrast-desc"),
            "stream-red" => fl!(LANGUAGE_LOADER, "theme-stream-red-desc"),
            "studio-purple" => fl!(LANGUAGE_LOADER, "theme-studio-purple-desc"),
            "ubuntu-violet" => fl!(LANGUAGE_LOADER, "theme-ubuntu-violet-desc"),
            _ => self.description.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThemeApplyReport {
    pub(crate) theme_id: String,
    pub(crate) variant: ThemeVariant,
    pub(crate) custom_css_path: Option<PathBuf>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug)]
pub(crate) enum CustomCssLoad {
    Disabled,
    MissingPath,
    Loaded { path: PathBuf, css: String },
    Failed { path: PathBuf, error: String },
}

impl ThemeApplyReport {
    pub(crate) fn is_ok(&self) -> bool {
        self.warnings.is_empty()
    }

    pub(crate) fn user_message(&self) -> Option<String> {
        if self.warnings.is_empty() {
            None
        } else {
            Some(self.warnings.join("\n"))
        }
    }
}

pub(crate) struct ThemeManager;

impl ThemeManager {
    pub(crate) fn built_in_themes() -> &'static [BuiltInTheme] {
        &BUILT_IN_THEMES
    }

    pub(crate) fn find_theme(id: &str) -> BuiltInTheme {
        Self::built_in_themes()
            .iter()
            .copied()
            .find(|theme| theme.id == id)
            .unwrap_or(BUILT_IN_THEMES[0])
    }

    pub(crate) fn apply(preference: &ThemePreference) -> ThemeApplyReport {
        Self::apply_with_custom_css(preference, CustomCssLoad::Disabled)
    }

    pub(crate) fn apply_async(
        preference: ThemePreference,
        complete: impl FnOnce(ThemeApplyReport) + 'static,
    ) {
        install_desktop_signals();
        ACTIVE_PREFERENCE.with(|active| *active.borrow_mut() = Some(preference.clone()));
        apply_native_color_scheme(preference.mode);
        let generation = REQUEST_GENERATION.with(|current| {
            let next = current.get().wrapping_add(1);
            current.set(next);
            next
        });
        if !preference.custom_css_enabled() {
            complete(Self::apply(&preference));
            return;
        }
        // Resolve System on GTK's thread, before file I/O moves to a worker.
        let variant = effective_variant(preference.mode);
        let worker_preference = preference.clone();
        crate::ui::background_io::run(
            move || Self::load_custom_css(&worker_preference, variant),
            move |custom_css| {
                if let Some(report) = finish_css_read(generation, &preference, custom_css) {
                    complete(report);
                }
            },
        );
    }

    /// Read the effective custom stylesheet. This is blocking and must be
    /// called through `ui::background_io` in production.
    pub(crate) fn load_custom_css(
        preference: &ThemePreference,
        variant: ThemeVariant,
    ) -> CustomCssLoad {
        if !preference.custom_css_enabled() {
            return CustomCssLoad::Disabled;
        }
        let Some(path) = preference
            .custom_css_path_for_mode(match variant {
                ThemeVariant::Light => ThemeMode::Light,
                ThemeVariant::Dark => ThemeMode::Dark,
            })
            .cloned()
        else {
            return CustomCssLoad::MissingPath;
        };
        // Settings and config.json both take free text, and a relative path
        // would resolve against whatever directory SceneDeck was launched from.
        if !path.is_absolute() {
            return CustomCssLoad::Failed {
                path,
                error: fl!(LANGUAGE_LOADER, "theme-custom-css-path-not-absolute"),
            };
        }
        match crate::storage::small_file::read_to_string(
            &path,
            crate::storage::small_file::CSS_LIMIT,
        )
        .and_then(|css| {
            validate_custom_css(&css).map_err(|()| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    fl!(LANGUAGE_LOADER, "theme-custom-css-dependencies-forbidden"),
                )
            })?;
            Ok(css)
        }) {
            Ok(css) => CustomCssLoad::Loaded { path, css },
            Err(error) => CustomCssLoad::Failed {
                path,
                error: error.to_string(),
            },
        }
    }

    pub(crate) fn apply_with_custom_css(
        preference: &ThemePreference,
        custom_css: CustomCssLoad,
    ) -> ThemeApplyReport {
        apply_native_color_scheme(preference.mode);
        let variant = effective_variant(preference.mode);
        let theme = Self::find_theme(preference.selected_theme_id());

        let mut warnings = Vec::new();
        clear_active_providers();

        install_css_provider(BASE_CSS, "base app CSS", &mut warnings);
        install_css_provider(theme.css_for(variant), theme.id, &mut warnings);

        // Installed after the theme so that no built-in theme can reintroduce
        // motion the user has switched off, but before any user stylesheet, so
        // that someone writing their own CSS still has the last word.
        apply_native_motion(preference.motion);
        let motion = preference.motion.resolve(desktop_allows_animation());
        if let Some(css) = motion_css(motion) {
            install_css_provider(&css, "motion overrides", &mut warnings);
        }

        let custom_css_path = match custom_css {
            CustomCssLoad::Disabled => None,
            CustomCssLoad::MissingPath => {
                warnings.push(fl!(LANGUAGE_LOADER, "theme-custom-css-no-matching-file"));
                None
            }
            CustomCssLoad::Loaded { path, css } => {
                // Keep the boundary at installation too: preloaded callers must
                // not bypass the off-thread file loader's dependency policy.
                if validate_custom_css(&css).is_ok() {
                    install_css_provider(&css, &path.display().to_string(), &mut warnings);
                } else {
                    warnings.push(fl!(
                        LANGUAGE_LOADER,
                        "theme-custom-css-dependencies-forbidden"
                    ));
                }
                Some(path)
            }
            CustomCssLoad::Failed { path, error } => {
                warnings.push(fl!(
                    LANGUAGE_LOADER,
                    "theme-custom-css-read-failed",
                    path = path.display().to_string(),
                    err = error
                ));
                Some(path)
            }
        };

        ThemeApplyReport {
            theme_id: theme.id.to_string(),
            variant,
            custom_css_path,
            warnings,
        }
    }
}

/// Tokenization decodes CSS escapes and skips comments. Scan nested blocks too,
/// failing closed on malformed at-signs and excessive nesting before GTK sees it.
fn validate_custom_css(css: &str) -> Result<(), ()> {
    if css.len() > crate::storage::small_file::CSS_LIMIT {
        return Err(());
    }
    fn tokens(input: &mut cssparser::Parser<'_>) -> Result<(), cssparser::ParseError<()>> {
        use cssparser::Token;
        while !input.is_exhausted() {
            match input.next()?.clone() {
                Token::AtKeyword(name) if name.eq_ignore_ascii_case("import") => {
                    return Err(cssparser::ParseError::custom(()))
                }
                Token::Delim('@') | Token::UnquotedUrl(_) => {
                    return Err(cssparser::ParseError::custom(()))
                }
                Token::Function(name) if name.eq_ignore_ascii_case("url") => {
                    return Err(cssparser::ParseError::custom(()))
                }
                token if token.is_parse_error() => return Err(cssparser::ParseError::custom(())),
                Token::Function(_)
                | Token::ParenthesisBlock
                | Token::SquareBracketBlock
                | Token::CurlyBracketBlock => input.parse_nested_block(tokens)?,
                _ => {}
            }
        }
        Ok(())
    }
    tokens(&mut cssparser::Parser::new(css)).map_err(|_| ())
}

fn finish_css_read(
    generation: u64,
    preference: &ThemePreference,
    css: CustomCssLoad,
) -> Option<ThemeApplyReport> {
    // A slower read must not reinstall a theme the user has already replaced.
    REQUEST_GENERATION
        .with(|current| current.get() == generation)
        .then(|| ThemeManager::apply_with_custom_css(preference, css))
}

fn install_desktop_signals() {
    if DESKTOP_SIGNALS_INSTALLED.with(|installed| installed.replace(true)) {
        return;
    }
    adw::StyleManager::default().connect_dark_notify(|_| {
        if !UPDATING_NATIVE_COLOR_SCHEME.with(Cell::get) {
            refresh_desktop_theme(true);
        }
    });
    if let Some(settings) = gtk4::Settings::default() {
        settings.connect_gtk_enable_animations_notify(|_| {
            if !UPDATING_NATIVE_MOTION.with(Cell::get) {
                refresh_desktop_theme(false);
            }
        });
    }
}

/// Keep native controls in the same mode as the app palette. Release only our
/// own override for System, leaving desktop-driven notifications authoritative.
fn apply_native_color_scheme(mode: ThemeMode) {
    let scheme = match mode {
        ThemeMode::Light => {
            NATIVE_COLOR_SCHEME_OVERRIDDEN.with(|active| active.set(true));
            adw::ColorScheme::ForceLight
        }
        ThemeMode::Dark => {
            NATIVE_COLOR_SCHEME_OVERRIDDEN.with(|active| active.set(true));
            adw::ColorScheme::ForceDark
        }
        ThemeMode::System => {
            if !NATIVE_COLOR_SCHEME_OVERRIDDEN.with(|active| active.replace(false)) {
                return;
            }
            adw::ColorScheme::Default
        }
    };
    let style = adw::StyleManager::default();
    if style.color_scheme() != scheme {
        // The resulting dark notification is part of this application, not a
        // second desktop preference change requiring another asynchronous read.
        UPDATING_NATIVE_COLOR_SCHEME.with(|updating| updating.set(true));
        style.set_color_scheme(scheme);
        UPDATING_NATIVE_COLOR_SCHEME.with(|updating| updating.set(false));
    }
}

/// GTK/libadwaita transitions are not CSS animations. Override the setting only
/// inside this process while Off is selected; releasing it lets GTK read the
/// current desktop preference again, including changes made while Off was set.
fn apply_native_motion(motion: MotionLevel) {
    let Some(settings) = gtk4::Settings::default() else {
        return;
    };
    let off = motion == MotionLevel::Off;
    if NATIVE_MOTION_OFF.with(|active| active.replace(off)) == off {
        return;
    }
    UPDATING_NATIVE_MOTION.with(|updating| updating.set(true));
    if off {
        settings.set_gtk_enable_animations(false);
    } else {
        settings.reset_property("gtk-enable-animations");
    }
    UPDATING_NATIVE_MOTION.with(|updating| updating.set(false));
}

fn refresh_desktop_theme(color_changed: bool) {
    let preference = ACTIVE_PREFERENCE.with(|active| active.borrow().clone());
    if let Some(preference) = preference {
        if !color_changed || preference.mode == ThemeMode::System {
            ThemeManager::apply_async(preference, |report| {
                for warning in report.warnings {
                    tracing::warn!(%warning, "desktop theme change");
                }
            });
        }
    }
}

/// Every selector in `assets/scenedeck.css` that carries a looping animation.
///
/// This list is the switch behind the Motion setting: the Reduced and Off
/// levels emit a stylesheet that sets `animation: none` on exactly these
/// selectors. An animation added to the stylesheet but not added here would be
/// one the user has no way to stop, so the two are meant to be edited together.
const MOTION_ANIMATED_SELECTORS: &[&str] = &[
    ".scenedeck-sidebar-live-icon-streaming",
    ".scenedeck-top-streaming-icon-active",
    ".scenedeck-status-bar-live.scenedeck-status-bar-record",
    ".scenedeck-status-bar-live.scenedeck-status-bar-stream",
    ".obs-connecting",
    ".scenedeck-status-bar-dropped-active",
    ".scenedeck-content-header-streaming",
];

/// Selectors carrying one-shot transitions, which survive Reduced motion and
/// are dropped only at Off.
const MOTION_TRANSITION_SELECTORS: &[&str] = &[
    ".scene-card",
    ".scene-card-status-active",
    ".scene-card-status-preview",
    ".scene-card-hotkey",
    ".scenedeck-sidebar-list row",
    ".scenedeck-status-bar-item",
    ".scenedeck-status-bar-icon",
];

/// The stylesheet that steps the interface down to a motion level, or `None`
/// at `Full`, where the base stylesheet already describes what should happen.
///
/// `Reduced` stops the loops but keeps the short transitions, so a scene still
/// visibly *changes* to active without anything pulsing forever. `Off` removes
/// both.
fn motion_css(level: MotionLevel) -> Option<String> {
    if level.allows_looping() && level.allows_transitions() {
        return None;
    }

    let mut css = String::new();
    if !level.allows_looping() {
        css.push_str(&MOTION_ANIMATED_SELECTORS.join(",\n"));
        // `animation-name: none` alone leaves the element wherever the last
        // frame left it; resetting opacity guarantees it lands fully visible.
        css.push_str(" {\n    animation: none;\n    opacity: 1;\n}\n");
    }
    if !level.allows_transitions() {
        css.push_str(&MOTION_TRANSITION_SELECTORS.join(",\n"));
        css.push_str(" {\n    transition: none;\n}\n");
    }
    Some(css)
}

/// Whether the desktop itself still wants animation.
///
/// GNOME's "Reduce Animation" accessibility switch and the equivalent on other
/// desktops surface here as `gtk-enable-animations`. Treating it as authority
/// means a user who has already asked their whole desktop to calm down does not
/// also have to find SceneDeck's own setting.
fn desktop_allows_animation() -> bool {
    gtk4::Settings::default().is_none_or(|settings| settings.is_gtk_enable_animations())
}

fn effective_variant(mode: ThemeMode) -> ThemeVariant {
    match mode {
        ThemeMode::Light => ThemeVariant::Light,
        ThemeMode::Dark => ThemeVariant::Dark,
        ThemeMode::System => {
            if adw::StyleManager::default().is_dark() {
                ThemeVariant::Dark
            } else {
                ThemeVariant::Light
            }
        }
    }
}

fn clear_active_providers() {
    let Some(display) = gdk::Display::default() else {
        return;
    };

    ACTIVE_PROVIDERS.with(|providers| {
        let mut providers = providers.borrow_mut();
        for provider in providers.drain(..) {
            gtk4::style_context_remove_provider_for_display(&display, &provider);
        }
    });
}

fn install_css_provider(css: &str, label: &str, warnings: &mut Vec<String>) {
    let Some(display) = gdk::Display::default() else {
        warnings.push(fl!(
            LANGUAGE_LOADER,
            "theme-css-no-display",
            label = label.to_string()
        ));
        return;
    };

    let provider = CssProvider::new();
    let parse_errors = Rc::new(RefCell::new(Vec::<String>::new()));
    provider.connect_parsing_error({
        let parse_errors = Rc::clone(&parse_errors);
        let label = label.to_string();
        move |_, _, error| {
            parse_errors.borrow_mut().push(fl!(
                LANGUAGE_LOADER,
                "theme-css-parse-error",
                label = label.clone(),
                message = error.message().to_string()
            ));
        }
    });

    provider.load_from_data(css);
    warnings.extend(parse_errors.borrow().iter().cloned());
    if !parse_errors.borrow().is_empty() {
        return;
    }

    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    ACTIVE_PROVIDERS.with(|providers| providers.borrow_mut().push(provider));
}

const BUILT_IN_THEMES: [BuiltInTheme; 12] = [
    BuiltInTheme {
        id: "adwaita-default",
        name: "Adwaita Default",
        description: "Neutral styling that follows GNOME defaults.",
        light_css: include_str!("../../resources/themes/adwaita-default-light.css"),
        dark_css: include_str!("../../resources/themes/adwaita-default-dark.css"),
    },
    BuiltInTheme {
        id: "scenedeck-dark",
        name: "SceneDeck Dark",
        description: "A reserved dark console theme for live operation.",
        light_css: include_str!("../../resources/themes/scenedeck-dark-light.css"),
        dark_css: include_str!("../../resources/themes/scenedeck-dark-dark.css"),
    },
    BuiltInTheme {
        id: "scenedeck-light",
        name: "SceneDeck Light",
        description: "A crisp light console theme with restrained contrast.",
        light_css: include_str!("../../resources/themes/scenedeck-light-light.css"),
        dark_css: include_str!("../../resources/themes/scenedeck-light-dark.css"),
    },
    BuiltInTheme {
        id: "obs",
        name: "OBS",
        description: "OBS Studio's default look, read through libadwaita.",
        light_css: include_str!("../../resources/themes/obs-light.css"),
        dark_css: include_str!("../../resources/themes/obs-dark.css"),
    },
    BuiltInTheme {
        id: "obsidian",
        name: "Obsidian",
        description: "High-legibility graphite surfaces with cool accents.",
        light_css: include_str!("../../resources/themes/obsidian-light.css"),
        dark_css: include_str!("../../resources/themes/obsidian-dark.css"),
    },
    BuiltInTheme {
        id: "nord",
        name: "Nord",
        description: "Cool blue-gray surfaces with frost-toned accents.",
        light_css: include_str!("../../resources/themes/nord-light.css"),
        dark_css: include_str!("../../resources/themes/nord-dark.css"),
    },
    BuiltInTheme {
        id: "dracula-inspired",
        name: "Dracula Inspired",
        description: "A dark expressive palette using original CSS.",
        light_css: include_str!("../../resources/themes/dracula-inspired-light.css"),
        dark_css: include_str!("../../resources/themes/dracula-inspired-dark.css"),
    },
    BuiltInTheme {
        id: "solarized-dark",
        name: "Solarized Dark",
        description: "Low-glare contrast with teal and amber accents.",
        light_css: include_str!("../../resources/themes/solarized-dark-light.css"),
        dark_css: include_str!("../../resources/themes/solarized-dark-dark.css"),
    },
    BuiltInTheme {
        id: "high-contrast",
        name: "High Contrast",
        description: "Stronger outlines and contrast for critical controls.",
        light_css: include_str!("../../resources/themes/high-contrast-light.css"),
        dark_css: include_str!("../../resources/themes/high-contrast-dark.css"),
    },
    BuiltInTheme {
        id: "stream-red",
        name: "Stream Red",
        description: "Broadcast-oriented red accents for live states.",
        light_css: include_str!("../../resources/themes/stream-red-light.css"),
        dark_css: include_str!("../../resources/themes/stream-red-dark.css"),
    },
    BuiltInTheme {
        id: "studio-purple",
        name: "Studio Purple",
        description: "Controlled purple accents without overpowering surfaces.",
        light_css: include_str!("../../resources/themes/studio-purple-light.css"),
        dark_css: include_str!("../../resources/themes/studio-purple-dark.css"),
    },
    BuiltInTheme {
        id: "ubuntu-violet",
        name: "Ubuntu Violet",
        description: "Ubuntu-inspired violet surfaces with a warm live accent.",
        light_css: include_str!("../../resources/themes/ubuntu-violet-light.css"),
        dark_css: include_str!("../../resources/themes/ubuntu-violet-dark.css"),
    },
];

#[cfg(test)]
mod tests;
