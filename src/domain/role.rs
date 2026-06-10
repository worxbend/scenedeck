use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneRole {
    Primary,
    Secondary,
    Module,
    #[default]
    Raw,
    Debug,
    Archive,
}

impl SceneRole {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Secondary => "Secondary",
            Self::Module => "Module",
            Self::Raw => "Raw",
            Self::Debug => "Debug",
            Self::Archive => "Archive",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Primary => "Live-switchable scene",
            Self::Secondary => "Valid scene, hidden from Live by default",
            Self::Module => "Reusable nested scene, not directly switchable",
            Self::Raw => "Hardware or source wrapper scene",
            Self::Debug => "Temporary test scene",
            Self::Archive => "Preserved but excluded from all workflows",
        }
    }

    /// CSS class applied to role badges in the UI.
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Primary => "role-primary",
            Self::Secondary => "role-secondary",
            Self::Module => "role-module",
            Self::Raw => "role-raw",
            Self::Debug => "role-debug",
            Self::Archive => "role-archive",
        }
    }

    /// Lowercase key used in `RuleConfig` (matches the serde representation).
    pub const fn rule_key(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Module => "module",
            Self::Raw => "raw",
            Self::Debug => "debug",
            Self::Archive => "archive",
        }
    }

    /// Whether this role is directly switchable from the Live page.
    pub const fn is_live_switchable(self) -> bool {
        matches!(self, Self::Primary)
    }
}
