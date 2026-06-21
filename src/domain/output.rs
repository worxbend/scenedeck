#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputRunState {
    Inactive,
    Starting,
    Active,
    Stopping,
    Reconnecting,
    Paused,
    Unknown,
}

impl OutputRunState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inactive => "Inactive",
            Self::Starting => "Starting",
            Self::Active => "Active",
            Self::Stopping => "Stopping",
            Self::Reconnecting => "Reconnecting",
            Self::Paused => "Paused",
            Self::Unknown => "Unknown",
        }
    }

    pub const fn is_transitioning(self) -> bool {
        matches!(self, Self::Starting | Self::Stopping | Self::Reconnecting)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputStatus {
    pub active: bool,
    pub state: OutputRunState,
    pub detail: Option<String>,
}

impl Default for OutputStatus {
    fn default() -> Self {
        Self {
            active: false,
            state: OutputRunState::Inactive,
            detail: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_states_are_identified() {
        assert!(OutputRunState::Starting.is_transitioning());
        assert!(OutputRunState::Stopping.is_transitioning());
        assert!(OutputRunState::Reconnecting.is_transitioning());
        assert!(!OutputRunState::Active.is_transitioning());
        assert!(!OutputRunState::Inactive.is_transitioning());
    }
}
