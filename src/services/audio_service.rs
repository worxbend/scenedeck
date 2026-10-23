//! Audio input queries and mutations: decibel conversion, slider range
//! sanitizing, and debouncing of fader drags into OBS volume commands.

use std::{collections::VecDeque, time::Duration};

use crate::domain::audio::{self, MAX_VOLUME_DB, MIN_VOLUME_DB};

pub const VOLUME_SLIDER_DEBOUNCE: Duration = Duration::from_millis(120);

/// Smallest loudness change worth a round trip to OBS.
///
/// Loudness is logarithmic, so the threshold is in decibels, not linear
/// multiplier: a fixed linear delta is a hairline near unity but many
/// decibels near silence, where it would swallow audible changes whole.
/// A drag that ends inside the threshold is never sent, leaving the fader
/// and OBS disagreeing by less than 0.1 dB — below anything audible.
const VOLUME_MEANINGFUL_DELTA_DB: f64 = 0.1;

pub struct AudioService;

impl AudioService {
    pub fn volume_mul_to_db(mul: f64) -> f64 {
        audio::volume_mul_to_db(mul)
    }

    pub fn volume_db_to_mul(db: f64) -> f64 {
        audio::volume_db_to_mul(db)
    }

    pub fn format_db(db: f64) -> String {
        if db <= MIN_VOLUME_DB || !db.is_finite() {
            "-inf dB".to_string()
        } else if db.abs() < 0.05 {
            "0.0 dB".to_string()
        } else {
            format!("{db:.1} dB")
        }
    }

    pub fn adjust_volume_db(current_db: f64, delta_db: f64) -> f64 {
        let base = if current_db.is_finite() {
            current_db
        } else {
            MIN_VOLUME_DB
        };
        (base + delta_db).clamp(MIN_VOLUME_DB, MAX_VOLUME_DB)
    }

    pub fn min_volume_db() -> f64 {
        MIN_VOLUME_DB
    }

    pub fn max_volume_db() -> f64 {
        MAX_VOLUME_DB
    }

    pub fn max_volume_mul() -> f64 {
        Self::volume_db_to_mul(MAX_VOLUME_DB)
    }

    pub fn sanitize_volume_db(volume_db: f64) -> f64 {
        audio::sanitize_volume_db(volume_db)
    }

    pub fn slider_db_from_mul(volume_mul: f64) -> f64 {
        Self::sanitize_volume_db(Self::volume_mul_to_db(Self::sanitize_volume_mul(
            volume_mul,
        )))
    }

    pub fn sanitize_volume_mul(volume_mul: f64) -> f64 {
        if volume_mul.is_finite() {
            volume_mul.clamp(0.0, Self::max_volume_mul())
        } else {
            0.0
        }
    }
}

// OBS gain events carry no command ID. Retain a bounded history of actual
// sends, including acknowledged values (snapshots can repeat an echo). An
// unmatched observation clears the history and remains authoritative.
const MAX_SENT_GAIN_HISTORY: usize = 32;

#[derive(Debug, Clone)]
pub struct VolumeChangeDebouncer {
    last_sent: Option<f64>,
    pending: Option<f64>,
    sent_history: VecDeque<f64>,
}

impl VolumeChangeDebouncer {
    pub fn new(initial_volume: f64) -> Self {
        Self {
            last_sent: Some(AudioService::sanitize_volume_mul(initial_volume)),
            pending: None,
            sent_history: VecDeque::new(),
        }
    }

    pub fn queue(&mut self, volume_mul: f64) {
        self.pending = Some(AudioService::sanitize_volume_mul(volume_mul));
    }

    pub fn take_due(&mut self) -> Option<f64> {
        let pending = self.pending.take()?;
        if self.should_send(pending) {
            self.mark_sent(pending);
            Some(pending)
        } else {
            None
        }
    }

    pub fn mark_sent(&mut self, volume_mul: f64) {
        let volume_mul = AudioService::sanitize_volume_mul(volume_mul);
        self.last_sent = Some(volume_mul);
        if self.sent_history.len() == MAX_SENT_GAIN_HISTORY {
            self.sent_history.pop_front();
        }
        self.sent_history.push_back(volume_mul);
        self.pending = None;
    }

    /// Return whether the observation should replace the visible local value.
    /// An echo of any retained send cannot supersede a newer unsent edit. Other
    /// observations remain authoritative, including readback after a failed send.
    pub fn reset_to_observed(&mut self, volume_mul: f64) -> bool {
        let volume_mul = AudioService::sanitize_volume_mul(volume_mul);
        let sent_echo = self.sent_history.iter().any(|sent| {
            // OBS receives f32 gain; tolerate its round-trip representation only.
            (sent - volume_mul).abs() <= f64::from(f32::EPSILON) * sent.abs().max(volume_mul.abs())
        });
        if sent_echo && self.pending.is_some() {
            // Do not move the send baseline backwards to an older acknowledgement.
            return false;
        }
        self.last_sent = Some(volume_mul);
        if !sent_echo {
            self.sent_history.clear();
        }
        self.pending = None;
        true
    }

    fn should_send(&self, volume_mul: f64) -> bool {
        self.last_sent
            .map(|last| {
                (AudioService::volume_mul_to_db(volume_mul) - AudioService::volume_mul_to_db(last))
                    .abs()
                    >= VOLUME_MEANINGFUL_DELTA_DB
            })
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_db_for_live_controls() {
        assert_eq!(AudioService::format_db(f64::NEG_INFINITY), "-inf dB");
        assert_eq!(AudioService::format_db(-120.0), "-inf dB");
        assert_eq!(AudioService::format_db(0.01), "0.0 dB");
        assert_eq!(AudioService::format_db(-6.24), "-6.2 dB");
    }

    #[test]
    fn converts_db_to_volume_multiplier() {
        assert_eq!(AudioService::volume_db_to_mul(f64::NEG_INFINITY), 0.0);
        assert!((AudioService::volume_db_to_mul(0.0) - 1.0).abs() < 0.0001);
        assert_eq!(AudioService::volume_db_to_mul(6.0), 1.0);
    }

    #[test]
    fn clamps_fine_adjustment_range() {
        assert_eq!(AudioService::adjust_volume_db(-0.2, 1.0), 0.0);
        assert_eq!(AudioService::adjust_volume_db(6.0, 1.0), 0.0);
        assert_eq!(AudioService::adjust_volume_db(-99.8, -1.0), -100.0);
    }

    #[test]
    fn sanitizes_volume_multiplier_for_obs_mixer_range() {
        assert_eq!(AudioService::sanitize_volume_mul(f64::NAN), 0.0);
        assert_eq!(AudioService::sanitize_volume_mul(-0.5), 0.0);
        assert_eq!(AudioService::max_volume_mul(), 1.0);
        assert_eq!(
            AudioService::sanitize_volume_mul(AudioService::max_volume_mul() + 1.0),
            1.0
        );
    }

    #[test]
    fn maps_slider_position_to_obs_decibel_range() {
        assert_eq!(AudioService::sanitize_volume_db(f64::NAN), -100.0);
        assert_eq!(AudioService::sanitize_volume_db(6.0), 0.0);
        assert_eq!(AudioService::sanitize_volume_db(-120.0), -100.0);
        assert_eq!(AudioService::slider_db_from_mul(2.0), 0.0);
        assert_eq!(AudioService::slider_db_from_mul(0.0), -100.0);
    }

    #[test]
    fn debouncer_coalesces_pending_slider_values() {
        let mut debouncer = VolumeChangeDebouncer::new(0.5);

        debouncer.queue(0.54);
        debouncer.queue(0.61);

        assert_eq!(debouncer.take_due(), Some(0.61));
        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn debouncer_ignores_tiny_changes_after_debounce() {
        let mut debouncer = VolumeChangeDebouncer::new(0.5);

        debouncer.queue(0.503);

        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn debouncer_measures_significance_in_decibels_not_multiplier() {
        // A 0.0049 linear step is nothing near unity but over 15 dB near
        // silence, where swallowing it would leave the fader and OBS
        // audibly disagreeing.
        let mut debouncer = VolumeChangeDebouncer::new(0.001);

        debouncer.queue(0.0059);

        assert_eq!(debouncer.take_due(), Some(0.0059));
    }

    #[test]
    fn volume_mul_to_db_maps_nonsense_to_silence() {
        assert_eq!(AudioService::volume_mul_to_db(f64::NAN), f64::NEG_INFINITY);
        assert_eq!(
            AudioService::volume_mul_to_db(f64::INFINITY),
            f64::NEG_INFINITY
        );
        assert_eq!(AudioService::volume_mul_to_db(0.0), f64::NEG_INFINITY);
    }

    #[test]
    fn delayed_sent_echo_preserves_newer_pending_gain() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        let sent = AudioService::volume_db_to_mul(-6.0);
        let newer = AudioService::volume_db_to_mul(-9.0);
        debouncer.queue(sent);
        assert_eq!(debouncer.take_due(), Some(sent));
        debouncer.queue(newer);

        // OBS accepts single-precision gain, so its echo need not be bit-identical.
        debouncer.reset_to_observed(f64::from(sent as f32));

        assert_eq!(debouncer.take_due(), Some(newer));
        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn multiple_outstanding_sends_preserve_newer_pending_gain() {
        for reverse_echoes in [false, true] {
            let mut debouncer = VolumeChangeDebouncer::new(1.0);
            let a = AudioService::volume_db_to_mul(-6.0);
            let b = AudioService::volume_db_to_mul(-9.0);
            let c = AudioService::volume_db_to_mul(-12.0);
            debouncer.queue(a);
            assert_eq!(debouncer.take_due(), Some(a));
            debouncer.queue(b);
            assert_eq!(debouncer.take_due(), Some(b));
            debouncer.queue(c);

            let echoes = if reverse_echoes { [b, a] } else { [a, b] };
            for echo in echoes {
                assert!(
                    !debouncer.reset_to_observed(f64::from(echo as f32)),
                    "an outstanding own echo replaced the newer local edit"
                );
            }
            assert_eq!(debouncer.take_due(), Some(c));
            assert_eq!(debouncer.take_due(), None);
        }
    }

    #[test]
    fn old_echo_does_not_rebase_a_newer_send() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        debouncer.mark_sent(0.5);
        debouncer.mark_sent(0.3);
        debouncer.queue(0.5);

        assert!(!debouncer.reset_to_observed(0.5));
        assert_eq!(debouncer.take_due(), Some(0.5));
    }

    #[test]
    fn unmatched_external_gain_retires_all_sent_history() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        debouncer.mark_sent(0.5);
        debouncer.mark_sent(0.3);
        debouncer.queue(0.2);
        // Close to A, but well outside f32 round-trip precision.
        assert!(debouncer.reset_to_observed(0.50001));
        assert_eq!(debouncer.take_due(), None);
        debouncer.queue(0.1);
        assert!(debouncer.reset_to_observed(0.5));
        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn sent_gain_history_is_bounded_and_evicts_oldest_first() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        for index in 1..=MAX_SENT_GAIN_HISTORY + 1 {
            debouncer.mark_sent(index as f64 / 100.0);
        }
        assert_eq!(debouncer.sent_history.len(), MAX_SENT_GAIN_HISTORY);
        debouncer.queue(0.7);
        assert!(!debouncer.reset_to_observed(0.02));
        assert!(
            debouncer.reset_to_observed(0.01),
            "evicted sends are no longer own echoes"
        );
        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn repeated_sent_echoes_preserve_the_same_pending_gain() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        debouncer.mark_sent(0.5);
        debouncer.queue(0.3);

        assert!(!debouncer.reset_to_observed(0.5));
        assert!(!debouncer.reset_to_observed(0.5));
        assert_eq!(debouncer.take_due(), Some(0.3));
    }

    #[test]
    fn external_gain_supersedes_pending_local_intent() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        debouncer.mark_sent(0.5);
        debouncer.queue(0.3);

        assert!(debouncer.reset_to_observed(0.7));
        assert_eq!(debouncer.take_due(), None);
        debouncer.queue(0.7);
        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn failed_send_readback_reconciles_without_suppressing_retry() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        debouncer.mark_sent(0.5);
        debouncer.queue(0.3);

        assert!(debouncer.reset_to_observed(1.0));
        assert_eq!(debouncer.take_due(), None);
        debouncer.queue(0.5);
        assert_eq!(debouncer.take_due(), Some(0.5));
    }

    #[test]
    fn initial_observation_is_not_mistaken_for_a_sent_echo() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        debouncer.queue(0.3);

        assert!(debouncer.reset_to_observed(1.0));
        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn sent_echo_without_a_newer_edit_reconciles_normally() {
        let mut debouncer = VolumeChangeDebouncer::new(1.0);
        debouncer.mark_sent(0.5);

        assert!(debouncer.reset_to_observed(0.5));
        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn debouncer_resets_to_observed_obs_volume() {
        let mut debouncer = VolumeChangeDebouncer::new(0.5);

        debouncer.queue(0.8);
        debouncer.reset_to_observed(0.8);
        debouncer.queue(0.802);

        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn debouncer_immediate_send_clears_pending_slider_value() {
        let mut debouncer = VolumeChangeDebouncer::new(0.5);

        debouncer.queue(0.8);
        debouncer.mark_sent(0.6);

        assert_eq!(debouncer.take_due(), None);
    }

    #[test]
    fn debouncer_sanitizes_queued_and_immediate_values() {
        let mut debouncer = VolumeChangeDebouncer::new(0.5);

        debouncer.queue(f64::NAN);
        assert_eq!(debouncer.take_due(), Some(0.0));

        debouncer.mark_sent(AudioService::max_volume_mul() + 1.0);
        debouncer.queue(f64::INFINITY);
        assert_eq!(debouncer.take_due(), Some(0.0));
    }
}
