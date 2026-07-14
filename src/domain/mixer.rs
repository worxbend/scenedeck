use crate::domain::scene::SceneId;
use crate::infra::i18n::LANGUAGE_LOADER;
use i18n_embed_fl::fl;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum MixerMode {
    #[default]
    ActiveScene,
    SelectedScene,
    PinnedScene,
}

impl MixerMode {
    pub fn label(self) -> String {
        match self {
            Self::ActiveScene => fl!(LANGUAGE_LOADER, "mixer-mode-active"),
            Self::SelectedScene => fl!(LANGUAGE_LOADER, "mixer-mode-selected"),
            Self::PinnedScene => fl!(LANGUAGE_LOADER, "mixer-mode-pinned"),
        }
    }

    pub fn description(self) -> String {
        match self {
            Self::ActiveScene => fl!(LANGUAGE_LOADER, "mixer-mode-active-desc"),
            Self::SelectedScene => fl!(LANGUAGE_LOADER, "mixer-mode-selected-desc"),
            Self::PinnedScene => fl!(LANGUAGE_LOADER, "mixer-mode-pinned-desc"),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActiveScene => "active",
            Self::SelectedScene => "selected",
            Self::PinnedScene => "pinned",
        }
    }
}

impl std::str::FromStr for MixerMode {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "selected" => Self::SelectedScene,
            "pinned" => Self::PinnedScene,
            _ => Self::ActiveScene,
        })
    }
}

impl Serialize for MixerMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MixerMode {
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MixerGrouping {
    #[default]
    Scope,
    ScenePath,
    None,
}

impl MixerGrouping {
    pub fn label(self) -> String {
        match self {
            Self::Scope => fl!(LANGUAGE_LOADER, "mixer-grouping-scope"),
            Self::ScenePath => fl!(LANGUAGE_LOADER, "mixer-grouping-scene-path"),
            Self::None => fl!(LANGUAGE_LOADER, "mixer-grouping-none"),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scope => "scope",
            Self::ScenePath => "scene_path",
            Self::None => "none",
        }
    }
}

impl std::str::FromStr for MixerGrouping {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "scene_path" => Self::ScenePath,
            "none" => Self::None,
            _ => Self::Scope,
        })
    }
}

impl Serialize for MixerGrouping {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MixerGrouping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.parse() {
            Ok(grouping) => Ok(grouping),
            Err(never) => match never {},
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MixerSelection {
    #[serde(default)]
    pub mode: MixerMode,
    #[serde(default)]
    pub selected_scene: Option<SceneId>,
    #[serde(default)]
    pub pinned_scene: Option<SceneId>,
    #[serde(skip)]
    pub search: String,
    #[serde(default)]
    pub grouping: MixerGrouping,
}

impl Default for MixerSelection {
    fn default() -> Self {
        Self {
            mode: MixerMode::ActiveScene,
            selected_scene: None,
            pinned_scene: None,
            search: String::new(),
            grouping: MixerGrouping::Scope,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_mode_uses_stable_config_values() {
        assert_eq!(
            serde_json::to_string(&MixerMode::PinnedScene).unwrap(),
            "\"pinned\""
        );
        let mode: MixerMode = serde_json::from_str("\"future\"").unwrap();
        assert_eq!(mode, MixerMode::ActiveScene);
    }

    #[test]
    fn mixer_selection_serializes_without_search_filter() {
        let selection = MixerSelection {
            mode: MixerMode::PinnedScene,
            selected_scene: Some("Main".to_string()),
            pinned_scene: Some("Main".to_string()),
            search: "mic".to_string(),
            grouping: MixerGrouping::ScenePath,
        };

        let json = serde_json::to_string(&selection).unwrap();
        assert!(json.contains(r#""mode":"pinned""#));
        assert!(json.contains(r#""grouping":"scene_path""#));
        assert!(!json.contains("mic"));
    }
}
