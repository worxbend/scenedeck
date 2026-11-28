//! Meter readings for tests.

use crate::domain::meter::ChannelLevel;

/// A digitally silent channel.
pub(crate) const SILENT_CHANNEL: ChannelLevel = ChannelLevel {
    magnitude_db: f64::NEG_INFINITY,
    peak_db: f64::NEG_INFINITY,
    input_peak_db: f64::NEG_INFINITY,
};
