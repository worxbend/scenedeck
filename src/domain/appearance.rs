//! Appearance preferences that are independent from GTK widgets.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl std::str::FromStr for ThemeMode {
    /// Parsing never fails: unknown persisted values fall back to `System`.
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        })
    }
}

impl Serialize for ThemeMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ThemeMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.parse() {
            Ok(mode) => Ok(mode),
            Err(never) => match never {},
        }
    }
}

/// UI language preference. `System` follows the desktop locale; every other
/// variant pins the UI to a specific shipped translation regardless of the
/// desktop locale.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    #[default]
    System,
    En,
    EnGb,
    De,
    DeCh,
    Es,
    It,
    Pl,
    PtPt,
    Uk,
}

impl Language {
    /// All selectable languages in the order they should appear in the
    /// Settings language picker, `System` first.
    pub const ALL: [Self; 10] = [
        Self::System,
        Self::En,
        Self::EnGb,
        Self::De,
        Self::DeCh,
        Self::Es,
        Self::It,
        Self::Pl,
        Self::PtPt,
        Self::Uk,
    ];

    /// Persisted config value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::En => "en",
            Self::EnGb => "en-gb",
            Self::De => "de",
            Self::DeCh => "de-ch",
            Self::Es => "es",
            Self::It => "it",
            Self::Pl => "pl",
            Self::PtPt => "pt-pt",
            Self::Uk => "uk",
        }
    }

    /// Fluent/BCP-47 locale tag to request from the loader, or `None` for
    /// `System`, meaning "use the desktop locale".
    pub const fn locale_tag(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::En => Some("en"),
            Self::EnGb => Some("en-GB"),
            Self::De => Some("de"),
            Self::DeCh => Some("de-CH"),
            Self::Es => Some("es"),
            Self::It => Some("it"),
            Self::Pl => Some("pl"),
            Self::PtPt => Some("pt-PT"),
            Self::Uk => Some("uk"),
        }
    }

    /// Name shown in the Settings language picker, in the language's own
    /// autonym so a user can find their language regardless of the UI's
    /// current language.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::System => "System Default",
            Self::En => "English",
            Self::EnGb => "English (UK)",
            Self::De => "Deutsch",
            Self::DeCh => "Deutsch (Schweiz)",
            Self::Es => "Español",
            Self::It => "Italiano",
            Self::Pl => "Polski",
            Self::PtPt => "Português (Portugal)",
            Self::Uk => "Українська",
        }
    }
}

impl std::str::FromStr for Language {
    /// Parsing never fails: unknown persisted values fall back to `System`.
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "en" => Self::En,
            "en-gb" => Self::EnGb,
            "de" => Self::De,
            "de-ch" => Self::DeCh,
            "es" => Self::Es,
            "it" => Self::It,
            "pl" => Self::Pl,
            "pt-pt" => Self::PtPt,
            "uk" => Self::Uk,
            _ => Self::System,
        })
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.parse() {
            Ok(language) => Ok(language),
            Err(never) => match never {},
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeId(pub String);

impl ThemeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ThemeId {
    fn default() -> Self {
        Self::new("adwaita-default")
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ThemeSource {
    #[default]
    BuiltIn,
    UserCssFile,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum UiDensity {
    Compact,
    #[default]
    Comfortable,
}

impl UiDensity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Comfortable => "comfortable",
        }
    }
}

impl std::str::FromStr for UiDensity {
    /// Parsing never fails: unknown persisted values fall back to `Comfortable`.
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "compact" => Self::Compact,
            _ => Self::Comfortable,
        })
    }
}

impl Serialize for UiDensity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for UiDensity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.parse() {
            Ok(density) => Ok(density),
            Err(never) => match never {},
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomCssPreference {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub light_path: Option<PathBuf>,
    #[serde(default)]
    pub dark_path: Option<PathBuf>,
    #[serde(default, rename = "path", skip_serializing)]
    legacy_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePreference {
    #[serde(default)]
    pub mode: ThemeMode,
    #[serde(default = "default_selected_theme", rename = "theme")]
    pub selected_theme: Option<ThemeId>,
    #[serde(default)]
    pub ui_density: UiDensity,
    #[serde(default)]
    pub custom_css: CustomCssPreference,
}

impl Default for ThemePreference {
    fn default() -> Self {
        Self {
            mode: ThemeMode::System,
            selected_theme: default_selected_theme(),
            ui_density: UiDensity::Comfortable,
            custom_css: CustomCssPreference::default(),
        }
    }
}

impl ThemePreference {
    pub fn selected_theme_id(&self) -> &str {
        self.selected_theme
            .as_ref()
            .map(ThemeId::as_str)
            .unwrap_or("adwaita-default")
    }

    pub fn custom_css_enabled(&self) -> bool {
        self.custom_css.enabled
    }

    pub fn custom_css_path(&self) -> Option<&PathBuf> {
        self.custom_css
            .light_path
            .as_ref()
            .or(self.custom_css.dark_path.as_ref())
            .or(self.custom_css.legacy_path.as_ref())
    }

    pub fn custom_css_path_for_mode(&self, mode: ThemeMode) -> Option<&PathBuf> {
        match mode {
            ThemeMode::Light => self
                .custom_css
                .light_path
                .as_ref()
                .or(self.custom_css.legacy_path.as_ref()),
            ThemeMode::Dark => self
                .custom_css
                .dark_path
                .as_ref()
                .or(self.custom_css.legacy_path.as_ref()),
            ThemeMode::System => self.custom_css_path(),
        }
    }

    pub fn migrate_legacy_custom_css_path(&mut self) -> bool {
        let Some(path) = self.custom_css.legacy_path.take() else {
            return false;
        };

        let mut changed = false;
        if self.custom_css.light_path.is_none() {
            self.custom_css.light_path = Some(path.clone());
            changed = true;
        }
        if self.custom_css.dark_path.is_none() {
            self.custom_css.dark_path = Some(path);
            changed = true;
        }
        changed
    }
}

fn default_selected_theme() -> Option<ThemeId> {
    Some(ThemeId::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_theme_mode_falls_back_to_system() {
        assert_eq!("unexpected".parse::<ThemeMode>(), Ok(ThemeMode::System));
    }

    #[test]
    fn unknown_language_falls_back_to_system() {
        assert_eq!("xx".parse::<Language>(), Ok(Language::System));
    }

    #[test]
    fn language_round_trips_through_persisted_string() {
        for language in Language::ALL {
            assert_eq!(language.as_str().parse::<Language>(), Ok(language));
        }
    }

    #[test]
    fn language_serializes_as_persisted_string() {
        assert_eq!(serde_json::to_string(&Language::DeCh).unwrap(), "\"de-ch\"");
    }

    #[test]
    fn system_language_has_no_locale_tag() {
        assert_eq!(Language::System.locale_tag(), None);
        assert_eq!(Language::PtPt.locale_tag(), Some("pt-PT"));
    }

    #[test]
    fn serializes_as_lowercase_config_value() {
        assert_eq!(serde_json::to_string(&ThemeMode::Dark).unwrap(), "\"dark\"");
    }

    #[test]
    fn deserializes_unknown_config_value_as_system() {
        let mode: ThemeMode = serde_json::from_str("\"future-mode\"").unwrap();

        assert_eq!(mode, ThemeMode::System);
    }

    #[test]
    fn unknown_ui_density_falls_back_to_comfortable() {
        let density: UiDensity = serde_json::from_str("\"future-density\"").unwrap();

        assert_eq!(density, UiDensity::Comfortable);
    }

    #[test]
    fn theme_preference_uses_default_theme() {
        let preference: ThemePreference = serde_json::from_str("{}").unwrap();

        assert_eq!(preference.mode, ThemeMode::System);
        assert_eq!(preference.selected_theme_id(), "adwaita-default");
        assert_eq!(preference.ui_density, UiDensity::Comfortable);
        assert!(!preference.custom_css_enabled());
    }

    #[test]
    fn legacy_custom_css_path_migrates_to_light_and_dark_paths() {
        let mut preference: ThemePreference = serde_json::from_str(
            r#"{
              "custom_css": {
                "enabled": true,
                "path": "/tmp/scenedeck.css"
              }
            }"#,
        )
        .unwrap();

        assert!(preference.migrate_legacy_custom_css_path());
        assert_eq!(
            preference.custom_css.light_path.as_deref(),
            Some(std::path::Path::new("/tmp/scenedeck.css"))
        );
        assert_eq!(
            preference.custom_css.dark_path.as_deref(),
            Some(std::path::Path::new("/tmp/scenedeck.css"))
        );
    }
}
