//! Theme loading for built-in light/dark theme families and user CSS files.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::{gdk, CssProvider};
use i18n_embed_fl::fl;

use crate::domain::appearance::{MotionLevel, ThemeMode, ThemePreference};
use crate::infra::i18n::LANGUAGE_LOADER;

const BASE_CSS: &str = include_str!("../../assets/scenedeck.css");

thread_local! {
    static ACTIVE_PROVIDERS: RefCell<Vec<CssProvider>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeVariant {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltInTheme {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) swatches: &'static [&'static str],
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
        if !preference.custom_css_enabled() {
            complete(Self::apply(&preference));
            return;
        }
        let worker_preference = preference.clone();
        crate::ui::background_io::run(
            move || Self::load_custom_css(&worker_preference),
            move |custom_css| complete(Self::apply_with_custom_css(&preference, custom_css)),
        );
    }

    /// Read the effective custom stylesheet. This is blocking and must be
    /// called through `ui::background_io` in production.
    pub(crate) fn load_custom_css(preference: &ThemePreference) -> CustomCssLoad {
        if !preference.custom_css_enabled() {
            return CustomCssLoad::Disabled;
        }
        let Some(path) = preference
            .custom_css_path_for_mode(preference.mode)
            .cloned()
        else {
            return CustomCssLoad::MissingPath;
        };
        match fs::read_to_string(&path) {
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
        let variant = effective_variant(preference.mode);
        let theme = Self::find_theme(preference.selected_theme_id());

        let mut warnings = Vec::new();
        clear_active_providers();

        install_css_provider(BASE_CSS, "base app CSS", &mut warnings);
        install_css_provider(theme.css_for(variant), theme.id, &mut warnings);

        // Installed after the theme so that no built-in theme can reintroduce
        // motion the user has switched off, but before any user stylesheet, so
        // that someone writing their own CSS still has the last word.
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
                install_css_provider(&css, &path.display().to_string(), &mut warnings);
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
    ".scene-card-status-previous",
    ".scene-card-status-ready",
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
        swatches: &["#3584e4", "#ffffff", "#241f31"],
        light_css: include_str!("../../resources/themes/adwaita-default-light.css"),
        dark_css: include_str!("../../resources/themes/adwaita-default-dark.css"),
    },
    BuiltInTheme {
        id: "scenedeck-dark",
        name: "SceneDeck Dark",
        description: "A reserved dark console theme for live operation.",
        swatches: &["#4f8cff", "#15171c", "#e8eaf0"],
        light_css: include_str!("../../resources/themes/scenedeck-dark-light.css"),
        dark_css: include_str!("../../resources/themes/scenedeck-dark-dark.css"),
    },
    BuiltInTheme {
        id: "scenedeck-light",
        name: "SceneDeck Light",
        description: "A crisp light console theme with restrained contrast.",
        swatches: &["#1f6feb", "#f8fafc", "#17202a"],
        light_css: include_str!("../../resources/themes/scenedeck-light-light.css"),
        dark_css: include_str!("../../resources/themes/scenedeck-light-dark.css"),
    },
    BuiltInTheme {
        id: "obs",
        name: "OBS",
        description: "OBS Studio's default look, read through libadwaita.",
        swatches: &["#3a7ebf", "#232629", "#eff0f1"],
        light_css: include_str!("../../resources/themes/obs-light.css"),
        dark_css: include_str!("../../resources/themes/obs-dark.css"),
    },
    BuiltInTheme {
        id: "obsidian",
        name: "Obsidian",
        description: "High-legibility graphite surfaces with cool accents.",
        swatches: &["#8ab4f8", "#111318", "#f1f3f4"],
        light_css: include_str!("../../resources/themes/obsidian-light.css"),
        dark_css: include_str!("../../resources/themes/obsidian-dark.css"),
    },
    BuiltInTheme {
        id: "nord",
        name: "Nord",
        description: "Cool blue-gray surfaces with frost-toned accents.",
        swatches: &["#5e81ac", "#eceff4", "#2e3440"],
        light_css: include_str!("../../resources/themes/nord-light.css"),
        dark_css: include_str!("../../resources/themes/nord-dark.css"),
    },
    BuiltInTheme {
        id: "dracula-inspired",
        name: "Dracula Inspired",
        description: "A dark expressive palette using original CSS.",
        swatches: &["#bd93f9", "#282a36", "#f8f8f2"],
        light_css: include_str!("../../resources/themes/dracula-inspired-light.css"),
        dark_css: include_str!("../../resources/themes/dracula-inspired-dark.css"),
    },
    BuiltInTheme {
        id: "solarized-dark",
        name: "Solarized Dark",
        description: "Low-glare contrast with teal and amber accents.",
        swatches: &["#268bd2", "#002b36", "#eee8d5"],
        light_css: include_str!("../../resources/themes/solarized-dark-light.css"),
        dark_css: include_str!("../../resources/themes/solarized-dark-dark.css"),
    },
    BuiltInTheme {
        id: "high-contrast",
        name: "High Contrast",
        description: "Stronger outlines and contrast for critical controls.",
        swatches: &["#ffffff", "#000000", "#ffcc00"],
        light_css: include_str!("../../resources/themes/high-contrast-light.css"),
        dark_css: include_str!("../../resources/themes/high-contrast-dark.css"),
    },
    BuiltInTheme {
        id: "stream-red",
        name: "Stream Red",
        description: "Broadcast-oriented red accents for live states.",
        swatches: &["#d72638", "#fff7f7", "#1f1416"],
        light_css: include_str!("../../resources/themes/stream-red-light.css"),
        dark_css: include_str!("../../resources/themes/stream-red-dark.css"),
    },
    BuiltInTheme {
        id: "studio-purple",
        name: "Studio Purple",
        description: "Controlled purple accents without overpowering surfaces.",
        swatches: &["#7c5cff", "#f7f5ff", "#1c1828"],
        light_css: include_str!("../../resources/themes/studio-purple-light.css"),
        dark_css: include_str!("../../resources/themes/studio-purple-dark.css"),
    },
    BuiltInTheme {
        id: "ubuntu-violet",
        name: "Ubuntu Violet",
        description: "Ubuntu-inspired violet surfaces with a warm live accent.",
        swatches: &["#77216f", "#e95420", "#2c1830"],
        light_css: include_str!("../../resources/themes/ubuntu-violet-light.css"),
        dark_css: include_str!("../../resources/themes/ubuntu-violet-dark.css"),
    },
];

#[cfg(test)]
mod tests {
    use super::{
        motion_css, MotionLevel, ThemeManager, ThemeVariant, BASE_CSS, MOTION_ANIMATED_SELECTORS,
        MOTION_TRANSITION_SELECTORS,
    };

    const REQUIRED_THEME_COLORS: &[&str] = &[
        "scenedeck_fg",
        "scenedeck_muted_fg",
        "scenedeck_accent_fg",
        "scenedeck_success",
        "scenedeck_warning",
        "scenedeck_error",
    ];

    const THEME_BACKGROUNDS: &[&str] = &[
        "scenedeck_window",
        "scenedeck_sidebar",
        "scenedeck_header",
        "scenedeck_surface",
        "scenedeck_card",
    ];

    #[test]
    fn mixer_empty_state_uses_themed_surface() {
        assert!(BASE_CSS.contains(".app-status-page {\n    background-color: @scenedeck_surface;"));
        assert!(BASE_CSS.contains(".mixer-page {\n    background-color: @scenedeck_surface;"));
        assert!(!BASE_CSS.contains(".mixer-page {\n    background-color: @window_bg_color;"));
    }

    #[test]
    fn base_css_uses_supported_gtk_properties() {
        assert!(!BASE_CSS.contains("max-width"));
        assert!(!BASE_CSS.contains("max-height"));
    }

    #[test]
    fn suggested_action_buttons_use_theme_accent_foreground() {
        assert!(BASE_CSS.contains("@define-color scenedeck_accent_fg @accent_fg_color;"));
        assert!(BASE_CSS.contains("color: @scenedeck_accent_fg;"));

        let high_contrast = ThemeManager::find_theme("high-contrast");
        assert!(high_contrast
            .css_for(ThemeVariant::Dark)
            .contains("@define-color scenedeck_accent #ffffff;"));
        assert!(high_contrast
            .css_for(ThemeVariant::Dark)
            .contains("@define-color scenedeck_accent_fg #000000;"));
        assert!(high_contrast
            .css_for(ThemeVariant::Light)
            .contains("@define-color scenedeck_accent #000000;"));
        assert!(high_contrast
            .css_for(ThemeVariant::Light)
            .contains("@define-color scenedeck_accent_fg #ffffff;"));
    }

    #[test]
    fn audio_cards_use_compact_width() {
        assert!(BASE_CSS.contains(".audio-card {\n    min-width: 136px;"));
        assert!(BASE_CSS.contains("    min-height: 218px;"));
        assert!(BASE_CSS.contains("    padding: 6px;\n    border-radius: 8px;"));
        assert!(BASE_CSS.contains(".audio-fine-controls button {\n    min-width: 18px;"));
        assert!(BASE_CSS.contains(".audio-meter {"));
        assert!(BASE_CSS.contains(".audio-meter-labels {"));
    }

    #[test]
    fn the_volume_fader_is_a_thin_track_with_a_tall_handle() {
        assert!(BASE_CSS.contains(".audio-volume-fader trough {"));
        assert!(BASE_CSS.contains(".audio-volume-fader highlight {"));
        assert!(BASE_CSS.contains(".audio-volume-fader slider {"));
        // Both halves of the track stay fully rounded at every size.
        assert_eq!(
            BASE_CSS
                .lines()
                .filter(|line| line.trim() == "border-radius: 999px;")
                .count(),
            2
        );
        // The handle is a tall pill, not a circle, as in OBS's mixer.
        assert!(BASE_CSS.contains("min-width: 15px;\n    min-height: 26px;"));
        assert!(BASE_CSS.contains(".audio-volume-fader:disabled slider {"));
    }

    #[test]
    fn the_audio_card_header_is_a_full_width_scope_bar() {
        assert!(BASE_CSS.contains(".audio-card-scope-bar {"));
        assert!(BASE_CSS.contains(".audio-card-scope-label {"));
        assert!(BASE_CSS.contains(".audio-card-scope-icon {"));
        // Nested and group scopes must not reuse the accent background, or the
        // scope tag would stop distinguishing anything.
        assert!(BASE_CSS.contains(".audio-card-scope-bar.audio-scope-nested,"));
    }

    /// GTK CSS has no logical-direction margin or padding properties; using one
    /// makes the whole stylesheet fail to parse from that point.
    #[test]
    fn stylesheets_avoid_properties_gtk_does_not_implement() {
        const UNSUPPORTED: &[&str] = &[
            "margin-end",
            "margin-start",
            "padding-end",
            "padding-start",
            "inset-inline",
            "gap:",
            "flex",
        ];
        let mut sheets = vec![("base app CSS", BASE_CSS)];
        for theme in ThemeManager::built_in_themes() {
            sheets.push((theme.id, theme.css_for(ThemeVariant::Light)));
            sheets.push((theme.id, theme.css_for(ThemeVariant::Dark)));
        }
        for (name, css) in sheets {
            for property in UNSUPPORTED {
                assert!(
                    !css.contains(property),
                    "{name} uses {property}, which GTK cannot parse"
                );
            }
        }
    }

    #[test]
    fn the_sidebar_carries_the_app_logo_and_name() {
        assert!(BASE_CSS.contains(".scenedeck-brand-logo {"));
        assert!(BASE_CSS.contains(".scenedeck-brand-name {"));
    }

    #[test]
    fn status_bar_segments_have_icons_that_follow_their_state() {
        assert!(BASE_CSS.contains(".scenedeck-status-bar-icon {"));
        // Metric and output icons stay muted; connection state classes colour
        // the icon as well as the text.
        assert!(BASE_CSS.contains(".scenedeck-status-bar-icon.scenedeck-status-bar-metric,"));
    }

    #[test]
    fn header_selectors_sit_flat_on_the_header_until_touched() {
        assert!(BASE_CSS
            .contains(".scenedeck-dropdown button {\n    background-color: @scenedeck_header;"));
        assert!(BASE_CSS.contains(".scenedeck-dropdown button:hover {"));
        assert!(BASE_CSS.contains(".scenedeck-dropdown button:focus-visible,"));
    }

    #[test]
    fn full_motion_needs_no_override_stylesheet() {
        assert!(motion_css(MotionLevel::Full).is_none());
    }

    #[test]
    fn reduced_motion_stops_loops_but_keeps_transitions() {
        let css = motion_css(MotionLevel::Reduced).expect("reduced motion needs overrides");

        assert!(css.contains("animation: none;"));
        assert!(!css.contains("transition: none;"));
    }

    #[test]
    fn motion_off_stops_loops_and_transitions() {
        let css = motion_css(MotionLevel::Off).expect("motion off needs overrides");

        assert!(css.contains("animation: none;"));
        assert!(css.contains("transition: none;"));
    }

    /// Guards the contract documented in the Motion section of the stylesheet:
    /// an animation the override list does not know about is an animation the
    /// user cannot switch off.
    #[test]
    fn every_switchable_selector_actually_exists_in_the_stylesheet() {
        for selector in MOTION_ANIMATED_SELECTORS
            .iter()
            .chain(MOTION_TRANSITION_SELECTORS)
        {
            assert!(
                BASE_CSS.contains(selector),
                "{selector} is listed as switchable but is not styled in the base CSS"
            );
        }
    }

    /// The mirror of the check above: every looping animation in the stylesheet
    /// must be reachable by the Off level.
    #[test]
    fn no_looping_animation_escapes_the_motion_setting() {
        let off = motion_css(MotionLevel::Off).expect("motion off needs overrides");

        // `animation-iteration-count: infinite` is what makes an animation loop
        // forever, so counting those is a reliable census of what must be
        // switchable. Finite flashes stop on their own and are covered too, via
        // the selectors they share.
        let looping_rules = BASE_CSS
            .matches("animation-iteration-count: infinite;")
            .count();
        assert!(looping_rules > 0, "expected the UI to animate at all");

        let switchable = MOTION_ANIMATED_SELECTORS
            .iter()
            .filter(|selector| off.contains(*selector))
            .count();
        assert_eq!(switchable, MOTION_ANIMATED_SELECTORS.len());
    }

    #[test]
    fn the_icon_picker_marks_the_current_choice() {
        assert!(BASE_CSS.contains(".icon-picker-choice {"));
        assert!(BASE_CSS.contains(".icon-picker-choice-selected {"));
        assert!(BASE_CSS.contains(".icon-picker-button {"));
    }

    #[test]
    fn scene_cards_and_live_sidebar_icon_use_compact_live_styling() {
        // Both live icons share one rule; the point of the assertion is that
        // they are red, not which selector they happen to be grouped under.
        assert!(BASE_CSS.contains(
            ".scenedeck-sidebar-live-icon-streaming,\n.scenedeck-top-streaming-icon-active {\n    color: @scenedeck_error;"
        ));
        assert!(BASE_CSS.contains(".scenedeck-content-header-streaming {"));
        assert!(BASE_CSS.contains("border-top: 3px solid alpha(@scenedeck_error, 0.78);"));
        assert!(BASE_CSS.contains("animation-name: scenedeck-live-breathe;"));
        assert!(BASE_CSS.contains(".scene-card {\n    min-width: 132px;"));
        assert!(BASE_CSS.contains("background-image: linear-gradient"));
        assert!(BASE_CSS.contains(".scene-card-previous {"));
    }

    #[test]
    fn built_in_theme_text_colors_contrast_with_app_surfaces() {
        for theme in ThemeManager::built_in_themes() {
            for variant in [ThemeVariant::Light, ThemeVariant::Dark] {
                let css = theme.css_for(variant);
                let colors = parse_theme_colors(css);

                for color in REQUIRED_THEME_COLORS {
                    assert!(
                        colors.iter().any(|(name, _)| name == color),
                        "{} {:?} should define {color}",
                        theme.id,
                        variant
                    );
                }

                for foreground in [
                    "scenedeck_fg",
                    "scenedeck_muted_fg",
                    "scenedeck_success",
                    "scenedeck_warning",
                    "scenedeck_error",
                ] {
                    for background in THEME_BACKGROUNDS {
                        assert_contrast(
                            theme.id,
                            variant,
                            foreground,
                            background,
                            color_value(&colors, foreground),
                            color_value(&colors, background),
                        );
                    }
                }

                assert_contrast(
                    theme.id,
                    variant,
                    "scenedeck_accent_fg",
                    "scenedeck_accent",
                    color_value(&colors, "scenedeck_accent_fg"),
                    color_value(&colors, "scenedeck_accent"),
                );
            }
        }
    }

    fn parse_theme_colors(css: &str) -> Vec<(&str, [f64; 3])> {
        css.lines()
            .filter_map(|line| {
                let line = line.trim();
                let rest = line.strip_prefix("@define-color ")?;
                let (name, value) = rest.split_once(' ')?;
                let value = value.strip_suffix(';')?;
                Some((name, parse_hex_color(value)))
            })
            .collect()
    }

    fn color_value(colors: &[(&str, [f64; 3])], name: &str) -> [f64; 3] {
        colors
            .iter()
            .find_map(|(color_name, value)| (*color_name == name).then_some(*value))
            .unwrap_or_else(|| panic!("missing theme color {name}"))
    }

    fn parse_hex_color(value: &str) -> [f64; 3] {
        let value = value
            .strip_prefix('#')
            .unwrap_or_else(|| panic!("expected hex color, got {value}"));
        assert_eq!(value.len(), 6, "expected six-digit hex color");
        [
            u8::from_str_radix(&value[0..2], 16).expect("red channel") as f64 / 255.0,
            u8::from_str_radix(&value[2..4], 16).expect("green channel") as f64 / 255.0,
            u8::from_str_radix(&value[4..6], 16).expect("blue channel") as f64 / 255.0,
        ]
    }

    fn assert_contrast(
        theme_id: &str,
        variant: ThemeVariant,
        foreground_name: &str,
        background_name: &str,
        foreground: [f64; 3],
        background: [f64; 3],
    ) {
        let ratio = contrast_ratio(foreground, background);
        assert!(
            ratio >= 4.5,
            "{theme_id} {variant:?} {foreground_name} on {background_name} contrast {ratio:.2}"
        );
    }

    fn contrast_ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
        let a = relative_luminance(a);
        let b = relative_luminance(b);
        let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(rgb: [f64; 3]) -> f64 {
        0.2126 * linear_channel(rgb[0])
            + 0.7152 * linear_channel(rgb[1])
            + 0.0722 * linear_channel(rgb[2])
    }

    fn linear_channel(channel: f64) -> f64 {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }
}
