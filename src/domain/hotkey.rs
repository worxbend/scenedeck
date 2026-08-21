//! Scene-switch keyboard shortcuts for the Live page.
//!
//! Live scene cards are laid out in the order stored in `registry.json`, so the
//! nth switchable card always has the same index. This module turns that index
//! into a keyboard binding: a digit key, optionally qualified by modifiers or
//! preceded by a vim-style leader key.
//!
//! Everything here is GTK-free. `services::hotkey_service` owns the leader
//! state machine, and `ui::window` translates GDK key events into [`KeyStroke`].

use crate::domain::string_enum_serde;
use i18n_embed_fl::fl;
use serde::{Deserialize, Serialize};

use crate::infra::i18n::LANGUAGE_LOADER;

/// Number of scene cards reachable by a digit key: `1`–`9` then `0`.
pub const MAX_SLOTS: usize = 10;

/// Slot index for a digit key, or `None` when the digit has no slot.
///
/// `1`–`9` map to the first nine cards and `0` maps to the tenth, matching the
/// row of digits on a keyboard rather than their numeric value.
pub const fn slot_for_digit(digit: u8) -> Option<usize> {
    match digit {
        1..=9 => Some(digit as usize - 1),
        0 => Some(9),
        _ => None,
    }
}

/// Digit key that activates `slot`, or `None` for slots past [`MAX_SLOTS`].
pub const fn digit_for_slot(slot: usize) -> Option<u8> {
    match slot {
        0..=8 => Some(slot as u8 + 1),
        9 => Some(0),
        _ => None,
    }
}

/// Label printed on a scene card's shortcut badge.
pub fn slot_badge(slot: usize) -> Option<String> {
    digit_for_slot(slot).map(|digit| digit.to_string())
}

// ── Modifiers ─────────────────────────────────────────────────────────────────

/// Modifier keys held down when a key was pressed.
///
/// Compared for equality, never as a subset: `Ctrl+Alt+1` must not trigger a
/// binding configured as `Ctrl` + digit.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct Modifiers {
    /// Control key.
    pub ctrl: bool,
    /// Alt / Meta key.
    pub alt: bool,
    /// Shift key.
    pub shift: bool,
    /// Super / Windows key.
    pub logo: bool,
}

impl Modifiers {
    /// No modifiers held.
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        logo: false,
    };

    /// Build a modifier set from individual flags.
    pub const fn new(ctrl: bool, alt: bool, shift: bool, logo: bool) -> Self {
        Self {
            ctrl,
            alt,
            shift,
            logo,
        }
    }

    /// Whether no modifier is held.
    pub const fn is_empty(self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.logo
    }
}

// ── Binding style ─────────────────────────────────────────────────────────────

/// How a scene slot is reached from the keyboard.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum SceneHotkeyStyle {
    /// Bare digit keys, with no modifier held.
    Plain,
    /// `Ctrl` + digit. The default: it cannot fire while typing.
    #[default]
    Ctrl,
    /// `Alt` + digit.
    Alt,
    /// `Shift` + digit.
    Shift,
    /// `Super` + digit.
    Super,
    /// `Ctrl+Alt` + digit.
    CtrlAlt,
    /// `Ctrl+Shift` + digit.
    CtrlShift,
    /// A leader key pressed and released, then a digit (vim style).
    Leader,
}

impl SceneHotkeyStyle {
    /// Stable display order used by the Settings picker.
    pub const ALL: [Self; 8] = [
        Self::Plain,
        Self::Ctrl,
        Self::Alt,
        Self::Shift,
        Self::Super,
        Self::CtrlAlt,
        Self::CtrlShift,
        Self::Leader,
    ];

    /// Persisted config value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Ctrl => "ctrl",
            Self::Alt => "alt",
            Self::Shift => "shift",
            Self::Super => "super",
            Self::CtrlAlt => "ctrl-alt",
            Self::CtrlShift => "ctrl-shift",
            Self::Leader => "leader",
        }
    }

    /// Modifiers that must be held with the digit, or `None` for [`Self::Leader`],
    /// which is driven by [`crate::services::hotkey_service`] instead.
    pub const fn modifiers(self) -> Option<Modifiers> {
        match self {
            Self::Plain => Some(Modifiers::NONE),
            Self::Ctrl => Some(Modifiers::new(true, false, false, false)),
            Self::Alt => Some(Modifiers::new(false, true, false, false)),
            Self::Shift => Some(Modifiers::new(false, false, true, false)),
            Self::Super => Some(Modifiers::new(false, false, false, true)),
            Self::CtrlAlt => Some(Modifiers::new(true, true, false, false)),
            Self::CtrlShift => Some(Modifiers::new(true, false, true, false)),
            Self::Leader => None,
        }
    }

    /// Whether this style fires on a bare digit, with nothing else held.
    ///
    /// Such bindings are suppressed while a text entry has focus.
    pub const fn is_bare_digit(self) -> bool {
        matches!(self, Self::Plain)
    }

    /// Modifier prefix shown in shortcut labels, e.g. `Ctrl+`.
    fn modifier_prefix(self) -> String {
        match self {
            Self::Plain | Self::Leader => String::new(),
            Self::Ctrl => fl!(LANGUAGE_LOADER, "hotkey-modifier-ctrl"),
            Self::Alt => fl!(LANGUAGE_LOADER, "hotkey-modifier-alt"),
            Self::Shift => fl!(LANGUAGE_LOADER, "hotkey-modifier-shift"),
            Self::Super => fl!(LANGUAGE_LOADER, "hotkey-modifier-super"),
            Self::CtrlAlt => fl!(LANGUAGE_LOADER, "hotkey-modifier-ctrl-alt"),
            Self::CtrlShift => fl!(LANGUAGE_LOADER, "hotkey-modifier-ctrl-shift"),
        }
    }

    /// User-facing name for the Settings picker.
    pub fn label(self) -> String {
        match self {
            Self::Plain => fl!(LANGUAGE_LOADER, "hotkey-style-plain"),
            Self::Ctrl => fl!(LANGUAGE_LOADER, "hotkey-style-ctrl"),
            Self::Alt => fl!(LANGUAGE_LOADER, "hotkey-style-alt"),
            Self::Shift => fl!(LANGUAGE_LOADER, "hotkey-style-shift"),
            Self::Super => fl!(LANGUAGE_LOADER, "hotkey-style-super"),
            Self::CtrlAlt => fl!(LANGUAGE_LOADER, "hotkey-style-ctrl-alt"),
            Self::CtrlShift => fl!(LANGUAGE_LOADER, "hotkey-style-ctrl-shift"),
            Self::Leader => fl!(LANGUAGE_LOADER, "hotkey-style-leader"),
        }
    }
}

impl std::str::FromStr for SceneHotkeyStyle {
    /// Parsing never fails: unknown persisted values fall back to `Ctrl`.
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "plain" | "none" => Self::Plain,
            "alt" => Self::Alt,
            "shift" => Self::Shift,
            "super" | "logo" => Self::Super,
            "ctrl-alt" => Self::CtrlAlt,
            "ctrl-shift" => Self::CtrlShift,
            "leader" => Self::Leader,
            _ => Self::Ctrl,
        })
    }
}

string_enum_serde!(SceneHotkeyStyle);

// ── Leader key ────────────────────────────────────────────────────────────────

/// Key that starts a vim-style two-stroke scene shortcut.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum LeaderKey {
    /// Space bar — the vim convention.
    #[default]
    Space,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `\`
    Backslash,
    /// `` ` ``
    Grave,
}

impl LeaderKey {
    /// Stable display order used by the Settings picker.
    pub const ALL: [Self; 5] = [
        Self::Space,
        Self::Comma,
        Self::Semicolon,
        Self::Backslash,
        Self::Grave,
    ];

    /// Persisted config value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Space => "space",
            Self::Comma => "comma",
            Self::Semicolon => "semicolon",
            Self::Backslash => "backslash",
            Self::Grave => "grave",
        }
    }

    /// Key symbol this leader is pressed as.
    pub const fn symbol(self) -> KeySymbol {
        match self {
            Self::Space => KeySymbol::Space,
            Self::Comma => KeySymbol::Comma,
            Self::Semicolon => KeySymbol::Semicolon,
            Self::Backslash => KeySymbol::Backslash,
            Self::Grave => KeySymbol::Grave,
        }
    }

    /// User-facing name, e.g. `Space`.
    pub fn label(self) -> String {
        match self {
            Self::Space => fl!(LANGUAGE_LOADER, "hotkey-leader-space"),
            Self::Comma => fl!(LANGUAGE_LOADER, "hotkey-leader-comma"),
            Self::Semicolon => fl!(LANGUAGE_LOADER, "hotkey-leader-semicolon"),
            Self::Backslash => fl!(LANGUAGE_LOADER, "hotkey-leader-backslash"),
            Self::Grave => fl!(LANGUAGE_LOADER, "hotkey-leader-grave"),
        }
    }
}

impl std::str::FromStr for LeaderKey {
    /// Parsing never fails: unknown persisted values fall back to `Space`.
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "comma" => Self::Comma,
            "semicolon" => Self::Semicolon,
            "backslash" => Self::Backslash,
            "grave" | "backtick" => Self::Grave,
            _ => Self::Space,
        })
    }
}

string_enum_serde!(LeaderKey);

// ── Key strokes ───────────────────────────────────────────────────────────────

/// The subset of keyboard keys scene hotkeys care about.
///
/// `ui::window` maps GDK keyvals onto this; every other key is [`Self::Other`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum KeySymbol {
    /// A digit from the number row or the keypad.
    Digit(u8),
    /// Space bar.
    Space,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `\`
    Backslash,
    /// `` ` ``
    Grave,
    /// Escape, which cancels an armed leader.
    Escape,
    /// A modifier key on its own. Never cancels an armed leader.
    Modifier,
    /// Any other key.
    Other,
}

/// One key press: a symbol plus the modifiers held with it.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct KeyStroke {
    /// Key that was pressed.
    pub key: KeySymbol,
    /// Modifiers held down at the time.
    pub modifiers: Modifiers,
}

impl KeyStroke {
    /// Build a key stroke.
    pub const fn new(key: KeySymbol, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Digit pressed with no modifiers held, if any.
    pub const fn bare_digit(self) -> Option<u8> {
        match self.key {
            KeySymbol::Digit(digit) if self.modifiers.is_empty() => Some(digit),
            _ => None,
        }
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Smallest leader window a user can configure, in milliseconds.
pub const MIN_LEADER_TIMEOUT_MS: u64 = 250;
/// Largest leader window a user can configure, in milliseconds.
pub const MAX_LEADER_TIMEOUT_MS: u64 = 5_000;
/// Leader window applied when none is configured.
pub const DEFAULT_LEADER_TIMEOUT_MS: u64 = 1_500;

/// User preferences for Live-page scene shortcuts.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneHotkeyConfig {
    /// Whether digit shortcuts switch scenes at all.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Which key combination activates a scene slot.
    #[serde(default)]
    pub style: SceneHotkeyStyle,
    /// Leader key used when `style` is [`SceneHotkeyStyle::Leader`].
    #[serde(default)]
    pub leader: LeaderKey,
    /// How long an armed leader waits for its digit, in milliseconds.
    #[serde(default = "default_leader_timeout_ms")]
    pub leader_timeout_ms: u64,
}

const fn default_enabled() -> bool {
    true
}

const fn default_leader_timeout_ms() -> u64 {
    DEFAULT_LEADER_TIMEOUT_MS
}

impl Default for SceneHotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            style: SceneHotkeyStyle::default(),
            leader: LeaderKey::default(),
            leader_timeout_ms: default_leader_timeout_ms(),
        }
    }
}

impl SceneHotkeyConfig {
    /// Leader window, clamped to the range the Settings page offers.
    ///
    /// Hand-edited config files can hold anything; clamping here keeps a `0`
    /// from making the leader impossible to use.
    pub const fn leader_timeout(self) -> std::time::Duration {
        let millis = if self.leader_timeout_ms < MIN_LEADER_TIMEOUT_MS {
            MIN_LEADER_TIMEOUT_MS
        } else if self.leader_timeout_ms > MAX_LEADER_TIMEOUT_MS {
            MAX_LEADER_TIMEOUT_MS
        } else {
            self.leader_timeout_ms
        };
        std::time::Duration::from_millis(millis)
    }

    /// Full shortcut label for one slot, e.g. `Ctrl+1` or `Space then 1`.
    ///
    /// Returns `None` when hotkeys are off or the slot is past [`MAX_SLOTS`].
    pub fn shortcut_label(&self, slot: usize) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let digit = digit_for_slot(slot)?;
        Some(match self.style {
            SceneHotkeyStyle::Leader => fl!(
                LANGUAGE_LOADER,
                "hotkey-shortcut-leader",
                leader = self.leader.label(),
                digit = digit.to_string()
            ),
            style => format!("{}{digit}", style.modifier_prefix()),
        })
    }

    /// One-line summary shown above the Live scene cards, e.g. `Ctrl+1 … 0`.
    ///
    /// Returns `None` when hotkeys are off, so the hint can be hidden.
    pub fn hint_label(&self) -> Option<String> {
        if !self.enabled {
            return None;
        }
        Some(match self.style {
            SceneHotkeyStyle::Leader => fl!(
                LANGUAGE_LOADER,
                "hotkey-hint-leader",
                leader = self.leader.label()
            ),
            SceneHotkeyStyle::Plain => fl!(LANGUAGE_LOADER, "hotkey-hint-plain"),
            style => fl!(
                LANGUAGE_LOADER,
                "hotkey-hint-modifier",
                modifier = style.modifier_prefix()
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_map_to_slots_with_zero_last() {
        assert_eq!(slot_for_digit(1), Some(0));
        assert_eq!(slot_for_digit(9), Some(8));
        assert_eq!(slot_for_digit(0), Some(9));
        assert_eq!(slot_for_digit(10), None);
    }

    #[test]
    fn slots_round_trip_through_digits() {
        for slot in 0..MAX_SLOTS {
            let digit = digit_for_slot(slot).unwrap();
            assert_eq!(slot_for_digit(digit), Some(slot));
        }
        assert_eq!(digit_for_slot(MAX_SLOTS), None);
    }

    #[test]
    fn styles_require_an_exact_modifier_set() {
        assert_eq!(
            SceneHotkeyStyle::Ctrl.modifiers(),
            Some(Modifiers::new(true, false, false, false))
        );
        assert_eq!(
            SceneHotkeyStyle::CtrlAlt.modifiers(),
            Some(Modifiers::new(true, true, false, false))
        );
        assert_eq!(SceneHotkeyStyle::Plain.modifiers(), Some(Modifiers::NONE));
        assert_eq!(SceneHotkeyStyle::Leader.modifiers(), None);
    }

    #[test]
    fn only_the_plain_style_fires_on_a_bare_digit() {
        assert!(SceneHotkeyStyle::Plain.is_bare_digit());
        for style in SceneHotkeyStyle::ALL {
            if style != SceneHotkeyStyle::Plain {
                assert!(!style.is_bare_digit(), "{style:?} claimed a bare digit");
            }
        }
    }

    #[test]
    fn style_keys_round_trip_and_unknown_values_fall_back_to_ctrl() {
        for style in SceneHotkeyStyle::ALL {
            assert_eq!(style.as_str().parse(), Ok(style));
        }
        assert_eq!(
            "chord".parse::<SceneHotkeyStyle>(),
            Ok(SceneHotkeyStyle::Ctrl)
        );
    }

    #[test]
    fn leader_keys_round_trip_and_unknown_values_fall_back_to_space() {
        for leader in LeaderKey::ALL {
            assert_eq!(leader.as_str().parse(), Ok(leader));
        }
        assert_eq!("meta".parse::<LeaderKey>(), Ok(LeaderKey::Space));
    }

    #[test]
    fn config_serializes_enums_as_lowercase_strings() {
        let config = SceneHotkeyConfig {
            style: SceneHotkeyStyle::CtrlShift,
            leader: LeaderKey::Semicolon,
            ..SceneHotkeyConfig::default()
        };

        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains(r#""style":"ctrl-shift""#));
        assert!(json.contains(r#""leader":"semicolon""#));
        assert_eq!(
            serde_json::from_str::<SceneHotkeyConfig>(&json).unwrap(),
            config
        );
    }

    #[test]
    fn leader_timeout_clamps_hand_edited_values() {
        let timeout = |millis| {
            SceneHotkeyConfig {
                leader_timeout_ms: millis,
                ..SceneHotkeyConfig::default()
            }
            .leader_timeout()
            .as_millis() as u64
        };

        assert_eq!(timeout(0), MIN_LEADER_TIMEOUT_MS);
        assert_eq!(timeout(900), 900);
        assert_eq!(timeout(60_000), MAX_LEADER_TIMEOUT_MS);
    }

    #[test]
    fn shortcut_labels_describe_the_configured_binding() {
        let ctrl = SceneHotkeyConfig::default();
        assert_eq!(ctrl.shortcut_label(0).as_deref(), Some("Ctrl+1"));
        assert_eq!(ctrl.shortcut_label(9).as_deref(), Some("Ctrl+0"));
        assert_eq!(ctrl.shortcut_label(MAX_SLOTS), None);

        let plain = SceneHotkeyConfig {
            style: SceneHotkeyStyle::Plain,
            ..SceneHotkeyConfig::default()
        };
        assert_eq!(plain.shortcut_label(2).as_deref(), Some("3"));

        let leader = SceneHotkeyConfig {
            style: SceneHotkeyStyle::Leader,
            leader: LeaderKey::Space,
            ..SceneHotkeyConfig::default()
        };
        assert_eq!(leader.shortcut_label(0).as_deref(), Some("Space then 1"));
    }

    #[test]
    fn disabled_hotkeys_have_no_labels_or_hint() {
        let config = SceneHotkeyConfig {
            enabled: false,
            ..SceneHotkeyConfig::default()
        };

        assert_eq!(config.shortcut_label(0), None);
        assert_eq!(config.hint_label(), None);
    }

    #[test]
    fn hint_label_matches_the_configured_style() {
        let hint = |style, leader| {
            SceneHotkeyConfig {
                style,
                leader,
                ..SceneHotkeyConfig::default()
            }
            .hint_label()
            .unwrap()
        };

        assert_eq!(
            hint(SceneHotkeyStyle::Plain, LeaderKey::Space),
            "Press 1 … 0"
        );
        assert_eq!(
            hint(SceneHotkeyStyle::Alt, LeaderKey::Space),
            "Press Alt+1 … 0"
        );
        assert_eq!(
            hint(SceneHotkeyStyle::Leader, LeaderKey::Comma),
            "Press Comma, then 1 … 0"
        );
    }

    #[test]
    fn bare_digit_requires_an_empty_modifier_set() {
        let bare = KeyStroke::new(KeySymbol::Digit(4), Modifiers::NONE);
        let with_ctrl = KeyStroke::new(
            KeySymbol::Digit(4),
            Modifiers::new(true, false, false, false),
        );

        assert_eq!(bare.bare_digit(), Some(4));
        assert_eq!(with_ctrl.bare_digit(), None);
        assert_eq!(
            KeyStroke::new(KeySymbol::Space, Modifiers::NONE).bare_digit(),
            None
        );
    }
}
