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
}
