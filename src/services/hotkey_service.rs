//! Scene-hotkey resolution: which key press activates which scene slot.
//!
//! [`SceneHotkeyResolver`] is the whole decision layer for Live-page shortcuts,
//! including the vim-style leader state machine. It is deliberately free of GTK
//! and of any clock: callers pass the current [`Instant`], so leader expiry is
//! testable without sleeping.

use std::time::Instant;

use crate::domain::hotkey::{
    slot_for_digit, KeyStroke, KeySymbol, SceneHotkeyConfig, SceneHotkeyStyle,
};

/// What a key press means for the Live page.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HotkeyOutcome {
    /// Not a scene shortcut. The key event must keep propagating.
    Ignored,
    /// A leader key was pressed; the next digit selects a scene.
    LeaderArmed,
    /// An armed leader was cancelled by an unrelated key.
    LeaderCancelled,
    /// Switch to the scene card at this slot index.
    Activate(usize),
}

impl HotkeyOutcome {
    /// Whether the key event was consumed and must not propagate further.
    pub const fn consumes_event(self) -> bool {
        !matches!(self, Self::Ignored)
    }
}

/// Tracks whether a leader key is currently armed.
///
/// One resolver instance lives on the GTK thread for the lifetime of the
/// window. Configuration is passed per call rather than stored, so changing the
/// binding in Settings takes effect on the very next key press.
#[derive(Debug, Default)]
pub struct SceneHotkeyResolver {
    armed_at: Option<Instant>,
}

impl SceneHotkeyResolver {
    /// Create a resolver with no leader armed.
    pub const fn new() -> Self {
        Self { armed_at: None }
    }

    /// Whether a leader is armed and has not yet timed out.
    pub fn is_armed(&self, config: &SceneHotkeyConfig, now: Instant) -> bool {
        self.armed_at
            .is_some_and(|armed_at| now.duration_since(armed_at) <= config.leader_timeout())
    }

    /// Drop any armed leader, e.g. when the Live page is left or OBS drops.
    pub fn disarm(&mut self) {
        self.armed_at = None;
    }

    /// Decide what `stroke` does under `config`.
    pub fn resolve(
        &mut self,
        config: &SceneHotkeyConfig,
        stroke: KeyStroke,
        now: Instant,
    ) -> HotkeyOutcome {
        // Modifier keys are pressed *as part of* a shortcut, so they must never
        // count as the unrelated key that cancels an armed leader.
        if stroke.key == KeySymbol::Modifier {
            return HotkeyOutcome::Ignored;
        }
        if !config.enabled {
            self.disarm();
            return HotkeyOutcome::Ignored;
        }
        if config.style == SceneHotkeyStyle::Leader {
            return self.resolve_leader(config, stroke, now);
        }

        self.disarm();
        let Some(required) = config.style.modifiers() else {
            return HotkeyOutcome::Ignored;
        };
        // Equality, not containment: Ctrl+Alt+1 must not fire a Ctrl binding.
        if stroke.modifiers != required {
            return HotkeyOutcome::Ignored;
        }
        match stroke.key {
            KeySymbol::Digit(digit) => match slot_for_digit(digit) {
                Some(slot) => HotkeyOutcome::Activate(slot),
                None => HotkeyOutcome::Ignored,
            },
            _ => HotkeyOutcome::Ignored,
        }
    }

    fn resolve_leader(
        &mut self,
        config: &SceneHotkeyConfig,
        stroke: KeyStroke,
        now: Instant,
    ) -> HotkeyOutcome {
        let armed = self.is_armed(config, now);
        // An expired leader is simply not armed; the stroke starts a new one.
        if !armed {
            self.disarm();
            return if self.matches_leader(config, stroke) {
                self.armed_at = Some(now);
                HotkeyOutcome::LeaderArmed
            } else {
                HotkeyOutcome::Ignored
            };
        }

        // Pressing the leader again restarts the window rather than cancelling.
        if self.matches_leader(config, stroke) {
            self.armed_at = Some(now);
            return HotkeyOutcome::LeaderArmed;
        }

        self.disarm();
        match stroke.bare_digit().and_then(slot_for_digit) {
            Some(slot) => HotkeyOutcome::Activate(slot),
            None => HotkeyOutcome::LeaderCancelled,
        }
    }

    fn matches_leader(&self, config: &SceneHotkeyConfig, stroke: KeyStroke) -> bool {
        stroke.modifiers.is_empty() && stroke.key == config.leader.symbol()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::domain::hotkey::{LeaderKey, Modifiers};

    fn ctrl_config() -> SceneHotkeyConfig {
        SceneHotkeyConfig::default()
    }

    fn leader_config() -> SceneHotkeyConfig {
        SceneHotkeyConfig {
            style: SceneHotkeyStyle::Leader,
            leader: LeaderKey::Space,
            leader_timeout_ms: 1_000,
            ..SceneHotkeyConfig::default()
        }
    }

    fn stroke(key: KeySymbol, modifiers: Modifiers) -> KeyStroke {
        KeyStroke::new(key, modifiers)
    }

    fn ctrl() -> Modifiers {
        Modifiers::new(true, false, false, false)
    }

    #[test]
    fn modifier_binding_activates_the_matching_slot() {
        let mut resolver = SceneHotkeyResolver::new();
        let now = Instant::now();

        assert_eq!(
            resolver.resolve(&ctrl_config(), stroke(KeySymbol::Digit(3), ctrl()), now),
            HotkeyOutcome::Activate(2)
        );
        assert_eq!(
            resolver.resolve(&ctrl_config(), stroke(KeySymbol::Digit(0), ctrl()), now),
            HotkeyOutcome::Activate(9)
        );
    }

    #[test]
    fn modifier_binding_ignores_extra_and_missing_modifiers() {
        let mut resolver = SceneHotkeyResolver::new();
        let now = Instant::now();

        assert_eq!(
            resolver.resolve(
                &ctrl_config(),
                stroke(KeySymbol::Digit(1), Modifiers::NONE),
                now
            ),
            HotkeyOutcome::Ignored
        );
        assert_eq!(
            resolver.resolve(
                &ctrl_config(),
                stroke(
                    KeySymbol::Digit(1),
                    Modifiers::new(true, true, false, false)
                ),
                now
            ),
            HotkeyOutcome::Ignored
        );
    }

    #[test]
    fn plain_binding_activates_on_a_bare_digit() {
        let config = SceneHotkeyConfig {
            style: SceneHotkeyStyle::Plain,
            ..SceneHotkeyConfig::default()
        };
        let mut resolver = SceneHotkeyResolver::new();
        let now = Instant::now();

        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Digit(2), Modifiers::NONE), now),
            HotkeyOutcome::Activate(1)
        );
        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Digit(2), ctrl()), now),
            HotkeyOutcome::Ignored
        );
    }

    #[test]
    fn disabled_hotkeys_ignore_every_stroke() {
        let config = SceneHotkeyConfig {
            enabled: false,
            style: SceneHotkeyStyle::Plain,
            ..SceneHotkeyConfig::default()
        };
        let mut resolver = SceneHotkeyResolver::new();
        let now = Instant::now();

        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Digit(1), Modifiers::NONE), now),
            HotkeyOutcome::Ignored
        );
    }

    #[test]
    fn leader_then_digit_activates_a_slot() {
        let config = leader_config();
        let mut resolver = SceneHotkeyResolver::new();
        let start = Instant::now();

        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Space, Modifiers::NONE), start),
            HotkeyOutcome::LeaderArmed
        );
        assert!(resolver.is_armed(&config, start));
        assert_eq!(
            resolver.resolve(
                &config,
                stroke(KeySymbol::Digit(5), Modifiers::NONE),
                start + Duration::from_millis(300)
            ),
            HotkeyOutcome::Activate(4)
        );
        assert!(!resolver.is_armed(&config, start + Duration::from_millis(300)));
    }

    #[test]
    fn digit_without_a_leader_is_ignored() {
        let config = leader_config();
        let mut resolver = SceneHotkeyResolver::new();

        assert_eq!(
            resolver.resolve(
                &config,
                stroke(KeySymbol::Digit(1), Modifiers::NONE),
                Instant::now()
            ),
            HotkeyOutcome::Ignored
        );
    }

    #[test]
    fn leader_expires_after_the_configured_timeout() {
        let config = leader_config();
        let mut resolver = SceneHotkeyResolver::new();
        let start = Instant::now();

        resolver.resolve(&config, stroke(KeySymbol::Space, Modifiers::NONE), start);
        let late = start + Duration::from_millis(1_001);

        assert!(!resolver.is_armed(&config, late));
        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Digit(1), Modifiers::NONE), late),
            HotkeyOutcome::Ignored
        );
    }

    #[test]
    fn pressing_the_leader_again_restarts_the_window() {
        let config = leader_config();
        let mut resolver = SceneHotkeyResolver::new();
        let start = Instant::now();

        resolver.resolve(&config, stroke(KeySymbol::Space, Modifiers::NONE), start);
        let again = start + Duration::from_millis(900);
        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Space, Modifiers::NONE), again),
            HotkeyOutcome::LeaderArmed
        );

        // Still armed 900 ms after the *second* press, 1.8 s after the first.
        assert_eq!(
            resolver.resolve(
                &config,
                stroke(KeySymbol::Digit(1), Modifiers::NONE),
                again + Duration::from_millis(900)
            ),
            HotkeyOutcome::Activate(0)
        );
    }

    #[test]
    fn unrelated_key_cancels_an_armed_leader() {
        let config = leader_config();
        let mut resolver = SceneHotkeyResolver::new();
        let start = Instant::now();

        resolver.resolve(&config, stroke(KeySymbol::Space, Modifiers::NONE), start);
        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Escape, Modifiers::NONE), start),
            HotkeyOutcome::LeaderCancelled
        );
        assert!(!resolver.is_armed(&config, start));
    }

    #[test]
    fn modified_digit_cancels_an_armed_leader_instead_of_switching() {
        let config = leader_config();
        let mut resolver = SceneHotkeyResolver::new();
        let start = Instant::now();

        resolver.resolve(&config, stroke(KeySymbol::Space, Modifiers::NONE), start);
        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Digit(1), ctrl()), start),
            HotkeyOutcome::LeaderCancelled
        );
    }

    #[test]
    fn modifier_presses_never_cancel_an_armed_leader() {
        let config = leader_config();
        let mut resolver = SceneHotkeyResolver::new();
        let start = Instant::now();

        resolver.resolve(&config, stroke(KeySymbol::Space, Modifiers::NONE), start);
        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Modifier, Modifiers::NONE), start),
            HotkeyOutcome::Ignored
        );
        assert!(resolver.is_armed(&config, start));
    }

    #[test]
    fn leader_key_is_configurable() {
        let config = SceneHotkeyConfig {
            leader: LeaderKey::Comma,
            ..leader_config()
        };
        let mut resolver = SceneHotkeyResolver::new();
        let start = Instant::now();

        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Space, Modifiers::NONE), start),
            HotkeyOutcome::Ignored
        );
        assert_eq!(
            resolver.resolve(&config, stroke(KeySymbol::Comma, Modifiers::NONE), start),
            HotkeyOutcome::LeaderArmed
        );
    }

    #[test]
    fn switching_style_away_from_leader_drops_an_armed_leader() {
        let mut resolver = SceneHotkeyResolver::new();
        let start = Instant::now();
        resolver.resolve(
            &leader_config(),
            stroke(KeySymbol::Space, Modifiers::NONE),
            start,
        );

        resolver.resolve(&ctrl_config(), stroke(KeySymbol::Digit(1), ctrl()), start);

        assert!(!resolver.is_armed(&leader_config(), start));
    }

    #[test]
    fn only_ignored_outcomes_let_the_event_propagate() {
        assert!(!HotkeyOutcome::Ignored.consumes_event());
        assert!(HotkeyOutcome::LeaderArmed.consumes_event());
        assert!(HotkeyOutcome::LeaderCancelled.consumes_event());
        assert!(HotkeyOutcome::Activate(0).consumes_event());
    }
}
