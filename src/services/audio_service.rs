//! Audio input queries and mutations.  Phase 1: stubs.

use crate::domain::audio::AudioInput;

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
        if !db.is_finite() || db <= -100.0 {
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
            -100.0
        };
        (base + delta_db).clamp(-100.0, 26.0)
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
    }

    #[test]
    fn clamps_fine_adjustment_range() {
        assert_eq!(AudioService::adjust_volume_db(25.8, 1.0), 26.0);
        assert_eq!(AudioService::adjust_volume_db(-99.8, -1.0), -100.0);
    }
}
