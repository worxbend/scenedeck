//! Audio input queries and mutations.  Phase 1: stubs.

use std::time::Duration;

use crate::domain::audio::AudioInput;

pub const VOLUME_SLIDER_DEBOUNCE: Duration = Duration::from_millis(120);
const VOLUME_MEANINGFUL_DELTA: f64 = 0.005;
const MIN_VOLUME_DB: f64 = -100.0;
const MAX_VOLUME_DB: f64 = 0.0;

pub struct AudioService;

impl AudioService {
    pub fn filter_configured(inputs: &[AudioInput], configured: &[String]) -> Vec<AudioInput> {
        if configured.is_empty() {
            return inputs.to_vec();
        }
        inputs
            .iter()
            .filter(|i| configured.contains(&i.name))
            .cloned()
            .collect()
    }

    pub fn volume_mul_to_db(mul: f64) -> f64 {
        if mul <= 0.0 {
            f64::NEG_INFINITY
        } else {
            20.0 * mul.log10()
        }
    }

    pub fn volume_db_to_mul(db: f64) -> f64 {
        let db = Self::sanitize_volume_db(db);
        if db <= MIN_VOLUME_DB {
            0.0
        } else {
            10.0_f64.powf(db / 20.0)
        }
    }

    pub fn format_db(db: f64) -> String {
        if db <= -100.0 || !db.is_finite() {
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
        if volume_db.is_finite() {
            volume_db.clamp(MIN_VOLUME_DB, MAX_VOLUME_DB)
        } else {
            MIN_VOLUME_DB
        }
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

#[derive(Debug, Clone)]
pub struct VolumeChangeDebouncer {
    last_sent: Option<f64>,
    pending: Option<f64>,
    meaningful_delta: f64,
}

impl VolumeChangeDebouncer {
    pub fn new(initial_volume: f64) -> Self {
        Self {
            last_sent: Some(AudioService::sanitize_volume_mul(initial_volume)),
            pending: None,
            meaningful_delta: VOLUME_MEANINGFUL_DELTA,
        }
    }

    pub fn queue(&mut self, volume_mul: f64) {
        self.pending = Some(AudioService::sanitize_volume_mul(volume_mul));
    }

    pub fn take_due(&mut self) -> Option<f64> {
        let pending = self.pending.take()?;
        if self.should_send(pending) {
            self.last_sent = Some(pending);
            Some(pending)
        } else {
            None
        }
    }

    pub fn mark_sent(&mut self, volume_mul: f64) {
        self.last_sent = Some(AudioService::sanitize_volume_mul(volume_mul));
        self.pending = None;
    }

    pub fn reset_to_observed(&mut self, volume_mul: f64) {
        self.last_sent = Some(AudioService::sanitize_volume_mul(volume_mul));
        self.pending = None;
    }

    fn should_send(&self, volume_mul: f64) -> bool {
        self.last_sent
            .map(|last| (volume_mul - last).abs() >= self.meaningful_delta)
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
