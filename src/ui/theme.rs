//! Theme loading for built-in light/dark theme families and user CSS files.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::{gdk, CssProvider};

use crate::domain::appearance::{ThemeMode, ThemePreference};

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThemeApplyReport {
    pub(crate) theme_id: String,
    pub(crate) variant: ThemeVariant,
    pub(crate) custom_css_path: Option<PathBuf>,
    pub(crate) warnings: Vec<String>,
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
        let variant = effective_variant(preference.mode);
        let theme = Self::find_theme(preference.selected_theme_id());

        let mut warnings = Vec::new();
        clear_active_providers();

        install_css_provider(BASE_CSS, "base app CSS", &mut warnings);
        install_css_provider(theme.css_for(variant), theme.id, &mut warnings);

        let mut custom_css_path = None;
        if preference.custom_css_enabled() {
            custom_css_path = preference
                .custom_css_path_for_mode(preference.mode)
                .cloned();
            if let Some(path) = custom_css_path.as_deref() {
                match fs::read_to_string(path) {
                    Ok(css) => {
                        install_css_provider(&css, &path.display().to_string(), &mut warnings)
                    }
                    Err(err) => warnings.push(format!(
                        "Custom CSS could not be read from {}: {err}",
                        path.display()
                    )),
                }
            } else {
                warnings
                    .push("Custom CSS is enabled but no matching light/dark file is set.".into());
            }
        }

        ThemeApplyReport {
            theme_id: theme.id.to_string(),
            variant,
            custom_css_path,
            warnings,
        }
    }
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
        warnings.push(format!(
            "{label} was not loaded because no GTK display is available."
        ));
        return;
    };

    let provider = CssProvider::new();
    let parse_errors = Rc::new(RefCell::new(Vec::<String>::new()));
    provider.connect_parsing_error({
        let parse_errors = Rc::clone(&parse_errors);
        let label = label.to_string();
        move |_, _, error| {
            parse_errors
                .borrow_mut()
                .push(format!("{label} CSS parse error: {}", error.message()));
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

const BUILT_IN_THEMES: [BuiltInTheme; 10] = [
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
];
