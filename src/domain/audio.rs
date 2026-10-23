use i18n_embed_fl::fl;

use crate::infra::i18n::LANGUAGE_LOADER;

/// Minimum fader gain; this position denotes exact digital silence.
pub const MIN_VOLUME_DB: f64 = -100.0;
/// Maximum gain allowed by SceneDeck's audio controls.
pub const MAX_VOLUME_DB: f64 = 0.0;

/// Convert an observed gain to decibels; invalid/nonpositive gain is silence.
pub fn volume_mul_to_db(mul: f64) -> f64 {
    if !mul.is_finite() || mul <= 0.0 {
        f64::NEG_INFINITY
    } else {
        20.0 * mul.log10()
    }
}

/// Convert a fader position to linear gain, preserving exact silence.
/// Command boundaries must validate finiteness/range before calling this.
pub fn volume_db_to_mul(db: f64) -> f64 {
    let db = sanitize_volume_db(db);
    if db <= MIN_VOLUME_DB {
        0.0
    } else {
        10.0_f64.powf(db / 20.0)
    }
}

/// Clamp an observed gain to the finite fader range.
pub fn sanitize_volume_db(db: f64) -> f64 {
    if db.is_finite() {
        db.clamp(MIN_VOLUME_DB, MAX_VOLUME_DB)
    } else {
        MIN_VOLUME_DB
    }
}

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
    pub fn label(self) -> String {
        match self {
            Self::Global => fl!(LANGUAGE_LOADER, "audio-scope-global"),
            Self::ActiveScene => fl!(LANGUAGE_LOADER, "audio-scope-active"),
            Self::NestedScene => fl!(LANGUAGE_LOADER, "audio-scope-nested"),
            Self::Group => fl!(LANGUAGE_LOADER, "audio-scope-group"),
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
