//! Stable operator intentions, resolved against current observations at execution.

use serde::{Deserialize, Serialize};

/// An OBS object identity with a name fallback for older servers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionTarget {
    pub name: String,
    pub uuid: Option<String>,
}

impl ActionTarget {
    /// A known UUID never falls back to a reused name. New identity information
    /// also retires an old name-only target until it is selected again.
    pub fn matches(&self, name: &str, uuid: Option<&str>) -> bool {
        match (&self.uuid, uuid) {
            (Some(expected), Some(actual)) => expected == actual,
            (None, None) => self.name == name,
            _ => false,
        }
    }
}

/// Actions shared by the palette and configurable operator controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub(crate) enum OperatorAction {
    Scene {
        target: ActionTarget,
    },
    OpenSource {
        target: ActionTarget,
    },
    SetMuted {
        target: ActionTarget,
        muted: bool,
    },
    ToggleMuted {
        target: ActionTarget,
    },
    SaveReplay,
    RestartMedia {
        target: ActionTarget,
    },
    SetFilterEnabled {
        target: ActionTarget,
        filter: String,
        enabled: bool,
    },
    ToggleFilter {
        target: ActionTarget,
        filter: String,
    },
}

/// One explicitly enabled control in the workspace's ordered Live strip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OperatorPin {
    pub action: OperatorAction,
    #[serde(default)]
    pub enabled: bool,
}
