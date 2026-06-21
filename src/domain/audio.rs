/// Stable identifier for an OBS audio input.  Matches `inputName` in the
/// WebSocket protocol; OBS guarantees uniqueness within a scene collection.
pub type InputId = String;
pub type SceneName = String;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AudioSourceScope {
    Global,
    #[default]
    ActiveScene,
    NestedScene,
    Group,
}

impl AudioSourceScope {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::ActiveScene => "Scene",
            Self::NestedScene => "Nested",
            Self::Group => "Group",
        }
    }

    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Global => "audio-scope-global",
            Self::ActiveScene => "audio-scope-active",
            Self::NestedScene => "audio-scope-nested",
            Self::Group => "audio-scope-group",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioInput {
    pub id: InputId,
    pub name: String,
    pub display_name: String,
    pub source_scope: AudioSourceScope,
    pub parent_scene_path: Vec<SceneName>,
    pub muted: bool,
    /// Linear gain multiplier (1.0 = 0 dB, 0.0 = silence).
    pub volume_mul: f64,
    /// Gain in decibels.
    pub volume_db: f64,
    pub locked_locally: bool,
}

impl AudioInput {
    pub fn new(id: InputId, muted: bool, volume_mul: f64, volume_db: f64) -> Self {
        Self {
            display_name: id.clone(),
            name: id.clone(),
            id,
            source_scope: AudioSourceScope::ActiveScene,
            parent_scene_path: Vec::new(),
            muted,
            volume_mul,
            volume_db,
            locked_locally: false,
        }
    }

    pub fn with_source_context(
        mut self,
        source_scope: AudioSourceScope,
        parent_scene_path: Vec<SceneName>,
    ) -> Self {
        self.source_scope = source_scope;
        self.parent_scene_path = parent_scene_path;
        self
    }

    pub fn source_path_label(&self) -> Option<String> {
        if self.parent_scene_path.is_empty() {
            None
        } else {
            Some(self.parent_scene_path.join(" / "))
        }
    }
}
