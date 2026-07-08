//! Scene role classification used by Live, Inventory, Graph, and Doctor.

use serde::{Deserialize, Serialize};

/// Local classification assigned to an OBS scene.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneRole {
    /// Live-switchable program scene.
    Primary,
    /// Valid scene hidden from Live by default.
    Secondary,
    /// Reusable nested scene intended as a building block.
    Module,
    /// Source-wrapper scene that should remain a leaf in the graph.
    #[default]
    Raw,
    /// Temporary test scene that should not be on a live path.
    Debug,
    /// Preserved scene excluded from active workflows.
    Archive,
}

impl SceneRole {
    /// User-facing label for a scene with no assigned role.
    pub const UNASSIGNED_LABEL: &'static str = "Unassigned";

    /// Stable display order used by role selectors and summaries.
    pub const ALL: [Self; 6] = [
        Self::Primary,
        Self::Secondary,
        Self::Module,
        Self::Raw,
        Self::Debug,
        Self::Archive,
    ];

    /// Parse the lowercase policy key used by config and registry files.
    pub const fn from_rule_key(key: &str) -> Option<Self> {
        match key.as_bytes() {
            b"primary" => Some(Self::Primary),
            b"secondary" => Some(Self::Secondary),
            b"module" => Some(Self::Module),
            b"raw" => Some(Self::Raw),
            b"debug" => Some(Self::Debug),
            b"archive" => Some(Self::Archive),
            _ => None,
        }
    }

    /// User-facing role name.
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

    /// User-facing role name, falling back to `Unassigned`.
    pub const fn label_or_unassigned(role: Option<Self>) -> &'static str {
        match role {
            Some(role) => role.label(),
            None => Self::UNASSIGNED_LABEL,
        }
    }

    /// Short user-facing explanation for Inventory role selectors.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_role_label_uses_unassigned_fallback() {
        assert_eq!(SceneRole::label_or_unassigned(None), "Unassigned");
        assert_eq!(
            SceneRole::label_or_unassigned(Some(SceneRole::Primary)),
            "Primary"
        );
    }
}
