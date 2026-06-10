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
