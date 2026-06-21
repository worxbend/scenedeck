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
