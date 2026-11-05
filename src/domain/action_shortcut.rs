//! Persisted application shortcuts and conflicts independent of GTK.

use crate::domain::hotkey::{Modifiers, SceneHotkeyConfig, SceneHotkeyStyle};
use crate::domain::operator_action::OperatorAction;
use serde::{Deserialize, Serialize};

/// A normalized logical key and exact modifier set, independent of scene order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionShortcut {
    pub key: String,
    pub modifiers: Modifiers,
}

/// A single workspace's explicit operator binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionBinding {
    pub shortcut: ActionShortcut,
    pub action: OperatorAction,
    /// Imports default to disabled until reviewed in the destination workspace.
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortcutConflict {
    Invalid,
    Application,
    SceneSlots,
    Duplicate,
}

impl ActionShortcut {
    pub fn valid(&self) -> bool {
        if self
            .key
            .strip_prefix('F')
            .and_then(|key| key.parse::<u8>().ok())
            .is_some_and(|number| (1..=24).contains(&number) && self.key == format!("F{number}"))
        {
            return true;
        }
        let modified = self.modifiers.ctrl || self.modifiers.alt || self.modifiers.logo;
        modified
            && (self.key == "space" || {
                let mut chars = self.key.chars();
                chars.next().is_some_and(|c| {
                    !c.is_control()
                        && !c.is_whitespace()
                        && c.to_lowercase().to_string() == self.key
                }) && chars.next().is_none()
            })
    }

    /// Detect conflicts even after the positional shortcut settings change.
    pub fn conflict(
        &self,
        slots: &SceneHotkeyConfig,
        others: impl Iterator<Item = Self>,
    ) -> Option<ShortcutConflict> {
        if !self.valid() {
            return Some(ShortcutConflict::Invalid);
        }
        if (self.key == "F1" && self.modifiers.is_empty())
            || (self.modifiers == Modifiers::new(true, false, false, false)
                && matches!(self.key.as_str(), "q" | "r" | "k" | ","))
        {
            return Some(ShortcutConflict::Application);
        }
        if slots.enabled
            && slots.style != SceneHotkeyStyle::Leader
            && slots.style.modifiers() == Some(self.modifiers)
            && self.key.len() == 1
            && self.key.as_bytes()[0].is_ascii_digit()
        {
            return Some(ShortcutConflict::SceneSlots);
        }
        if others.into_iter().any(|other| other == *self) {
            return Some(ShortcutConflict::Duplicate);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicts_cover_reserved_shortcuts_duplicates_and_changed_slot_styles() {
        let slots = SceneHotkeyConfig::default();
        let chord = |key: &str| ActionShortcut {
            key: key.into(),
            modifiers: Modifiers::new(true, false, false, false),
        };
        for key in ["q", "r", "k", ","] {
            assert_eq!(
                chord(key).conflict(&slots, std::iter::empty()),
                Some(ShortcutConflict::Application)
            );
        }
        assert_eq!(
            chord("1").conflict(&slots, std::iter::empty()),
            Some(ShortcutConflict::SceneSlots)
        );
        assert_eq!(
            chord("m").conflict(&slots, [chord("m")].into_iter()),
            Some(ShortcutConflict::Duplicate)
        );
        let alt = ActionShortcut {
            key: "1".into(),
            modifiers: Modifiers::new(false, true, false, false),
        };
        assert_eq!(alt.conflict(&slots, std::iter::empty()), None);
        assert_eq!(
            alt.conflict(
                &SceneHotkeyConfig {
                    style: SceneHotkeyStyle::Alt,
                    ..slots
                },
                std::iter::empty()
            ),
            Some(ShortcutConflict::SceneSlots)
        );
        for key in ["Escape", "Tab", "Return", "F0", "F25", "F02", "A", ""] {
            assert_eq!(
                chord(key).conflict(&slots, std::iter::empty()),
                Some(ShortcutConflict::Invalid)
            );
        }
        let mut bare = chord("m");
        bare.modifiers = Modifiers::NONE;
        assert!(!bare.valid());
        bare.key = "F8".into();
        assert!(bare.valid());
    }
}
