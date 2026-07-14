//! Streaming and recording output state.

use i18n_embed_fl::fl;

use crate::infra::i18n::LANGUAGE_LOADER;

/// OBS output lifecycle state normalized for UI display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputRunState {
    /// Output is fully stopped.
    Inactive,
    /// OBS accepted a start command and is transitioning to active.
    Starting,
    /// Output is actively running.
    Active,
    /// OBS accepted a stop command and is shutting down the output.
    Stopping,
    /// Output is trying to reconnect after a transport failure.
    Reconnecting,
    /// Output is paused but still active.
    Paused,
    /// OBS returned a state SceneDeck does not yet classify.
    Unknown,
}

impl OutputRunState {
    /// User-facing label for compact output status text.
    pub fn label(self) -> String {
        match self {
            Self::Inactive => fl!(LANGUAGE_LOADER, "output-state-inactive"),
            Self::Starting => fl!(LANGUAGE_LOADER, "output-state-starting"),
            Self::Active => fl!(LANGUAGE_LOADER, "output-state-active"),
            Self::Stopping => fl!(LANGUAGE_LOADER, "output-state-stopping"),
            Self::Reconnecting => fl!(LANGUAGE_LOADER, "output-state-reconnecting"),
            Self::Paused => fl!(LANGUAGE_LOADER, "output-state-paused"),
            Self::Unknown => fl!(LANGUAGE_LOADER, "output-state-unknown"),
        }
    }

    pub const fn is_transitioning(self) -> bool {
        matches!(self, Self::Starting | Self::Stopping | Self::Reconnecting)
    }
}

/// Current state for one OBS output, such as streaming or recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputStatus {
    /// Whether OBS considers the output active.
    pub active: bool,
    /// More detailed lifecycle state.
    pub state: OutputRunState,
    /// Optional OBS-provided detail, such as a recording path.
    pub detail: Option<String>,
}

impl Default for OutputStatus {
    fn default() -> Self {
        Self::inactive()
    }
}

impl OutputStatus {
    /// Build an output status with no extra detail.
    pub const fn new(active: bool, state: OutputRunState) -> Self {
        Self {
            active,
            state,
            detail: None,
        }
    }

    /// Build an inactive output status with no extra detail.
    pub const fn inactive() -> Self {
        Self::new(false, OutputRunState::Inactive)
    }

    /// Build an inactive output status with a detail string.
    pub fn inactive_with_detail(detail: impl Into<String>) -> Self {
        Self::inactive().with_detail(detail)
    }

    /// Build an active output status with no extra detail.
    pub const fn active() -> Self {
        Self::new(true, OutputRunState::Active)
    }

    /// Build an active output status with a detail string.
    pub fn active_with_detail(detail: impl Into<String>) -> Self {
        Self::active().with_detail(detail)
    }

    /// Attach a non-empty detail string, such as a completed recording path.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Attach an optional detail string.
    pub fn with_optional_detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail;
        self
    }

    /// Compact user-facing label for one output control.
    pub fn summary(&self, output_name: &str) -> String {
        fl!(
            LANGUAGE_LOADER,
            "output-summary",
            name = output_name,
            state = self.state.label()
        )
    }

    /// Optional detail suitable for a tooltip.
    pub fn detail_tooltip(&self) -> Option<&str> {
        self.detail.as_deref().filter(|detail| !detail.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_uses_output_name_and_run_state_label() {
        let status = OutputStatus::new(true, OutputRunState::Reconnecting);

        assert_eq!(status.summary("Stream"), "Stream: Reconnecting");
    }

    #[test]
    fn detail_tooltip_ignores_empty_details() {
        let mut status = OutputStatus::inactive().with_detail(String::new());
        assert_eq!(status.detail_tooltip(), None);

        status.detail = Some("/tmp/recording.mkv".to_string());
        assert_eq!(status.detail_tooltip(), Some("/tmp/recording.mkv"));
    }

    #[test]
    fn constructors_build_common_output_states() {
        assert_eq!(OutputStatus::default(), OutputStatus::inactive());
        assert_eq!(OutputStatus::inactive().state, OutputRunState::Inactive);
        assert!(!OutputStatus::inactive().active);
        assert_eq!(
            OutputStatus::inactive_with_detail("/tmp/done.mkv")
                .detail
                .as_deref(),
            Some("/tmp/done.mkv")
        );
        assert_eq!(OutputStatus::active().state, OutputRunState::Active);
        assert!(OutputStatus::active().active);
        assert_eq!(
            OutputStatus::active_with_detail("/tmp/live.mkv")
                .detail
                .as_deref(),
            Some("/tmp/live.mkv")
        );
    }

    #[test]
    fn detail_builders_attach_recording_paths() {
        let completed = OutputStatus::inactive().with_detail("/tmp/done.mkv");
        let active = OutputStatus::active().with_optional_detail(Some("/tmp/live.mkv".into()));

        assert_eq!(completed.detail.as_deref(), Some("/tmp/done.mkv"));
        assert_eq!(active.detail.as_deref(), Some("/tmp/live.mkv"));
    }

    #[test]
    fn transition_states_are_identified() {
        assert!(OutputRunState::Starting.is_transitioning());
        assert!(OutputRunState::Stopping.is_transitioning());
        assert!(OutputRunState::Reconnecting.is_transitioning());
        assert!(!OutputRunState::Active.is_transitioning());
        assert!(!OutputRunState::Inactive.is_transitioning());
    }
}
