use crate::domain::scene::SceneId;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MixerMode {
    #[default]
    ActiveScene,
    SelectedScene,
    PinnedScene,
}

impl MixerMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ActiveScene => "Active",
            Self::SelectedScene => "Selected",
            Self::PinnedScene => "Pinned",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::ActiveScene => "Follow the OBS program scene.",
            Self::SelectedScene => "Inspect the selected scene without following OBS.",
            Self::PinnedScene => "Keep the selected scene stable while operating.",
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
    pub const fn label(self) -> &'static str {
        match self {
            Self::Scope => "Scope",
            Self::ScenePath => "Scene Path",
            Self::None => "None",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixerSelection {
    pub mode: MixerMode,
    pub selected_scene: Option<SceneId>,
    pub pinned_scene: Option<SceneId>,
    pub search: String,
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
