//! Audio level metering — the values behind the mixer volume meters.
//!
//! OBS reports levels as linear multipliers, three per channel, roughly twenty
//! times a second. This module holds the decibel form of those readings and the
//! zone thresholds the meter is read against. The decay, hold, and drawing
//! rules live in `services::meter_service`.

use i18n_embed_fl::fl;

use crate::domain::audio::InputId;
use crate::infra::i18n::LANGUAGE_LOADER;

/// Quietest level the meter shows. Anything below reads as empty.
pub const METER_FLOOR_DB: f64 = -60.0;
/// Loudest level the meter shows; OBS clamps input levels at unity.
pub const METER_CEILING_DB: f64 = 0.0;
/// Level at which the meter leaves the green zone.
pub const METER_WARNING_DB: f64 = -20.0;
/// Level at which the meter enters the red zone.
pub const METER_ERROR_DB: f64 = -9.0;
/// Level treated as clipping.
pub const METER_CLIP_DB: f64 = -0.5;

/// Which of the meter's three zones a level falls in.
///
/// The thresholds match OBS Studio's own volume meter, so a level that looks
/// safe in OBS looks safe here.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum MeterZone {
    /// Green: background music, alerts, and anything that should stay under the
    /// voice.
    #[default]
    Nominal,
    /// Yellow: speech sits in the upper part of this zone, game audio lower.
    Warning,
    /// Red: close to clipping. Speech peaks may touch the lower edge.
    Error,
}

impl MeterZone {
    /// Zone a decibel level falls into.
    pub fn for_db(db: f64) -> Self {
        if db >= METER_ERROR_DB {
            Self::Error
        } else if db >= METER_WARNING_DB {
            Self::Warning
        } else {
            Self::Nominal
        }
    }

    /// CSS class used by the meter legend.
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Nominal => "audio-meter-nominal",
            Self::Warning => "audio-meter-warning",
            Self::Error => "audio-meter-error",
        }
    }

    /// User-facing zone name.
    pub fn label(self) -> String {
        match self {
            Self::Nominal => fl!(LANGUAGE_LOADER, "audio-meter-zone-nominal"),
            Self::Warning => fl!(LANGUAGE_LOADER, "audio-meter-zone-warning"),
            Self::Error => fl!(LANGUAGE_LOADER, "audio-meter-zone-error"),
        }
    }
}

/// One channel's reading from a single OBS volume-meter update.
///
/// All three values are decibels relative to full scale, and are
/// [`f64::NEG_INFINITY`] for digital silence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelLevel {
    /// Sound pressure — the closest of the three to perceived loudness.
    pub magnitude_db: f64,
    /// Sample peak after OBS applies the input's fader.
    pub peak_db: f64,
    /// Sample peak before the fader, so it shows what the device sends in.
    pub input_peak_db: f64,
}

impl ChannelLevel {
    /// Silence, used before the first reading arrives.
    pub const SILENT: Self = Self {
        magnitude_db: f64::NEG_INFINITY,
        peak_db: f64::NEG_INFINITY,
        input_peak_db: f64::NEG_INFINITY,
    };

    /// Convert OBS's `[magnitude, peak, inputPeak]` multipliers to decibels.
    pub fn from_mul(levels: [f32; 3]) -> Self {
        Self {
            magnitude_db: db_from_mul(levels[0]),
            peak_db: db_from_mul(levels[1]),
            input_peak_db: db_from_mul(levels[2]),
        }
    }
}

/// Convert a linear level multiplier to decibels.
///
/// Zero, negative, and non-finite multipliers all mean "no signal" and map to
/// negative infinity rather than to a huge negative number, so callers can test
/// for silence without picking an arbitrary threshold.
pub fn db_from_mul(mul: f32) -> f64 {
    let mul = f64::from(mul);
    if !mul.is_finite() || mul <= 0.0 {
        return f64::NEG_INFINITY;
    }
    20.0 * mul.log10()
}

/// The channels reported for one input in a single update.
///
/// Channel order is OBS's: mono has one, stereo is left then right, and
/// surround runs front left, front right, front centre, LFE, rear left, rear
/// right, side left, side right.
#[derive(Debug, Clone, PartialEq)]
pub struct InputLevels {
    /// Input the reading belongs to.
    pub input: InputId,
    /// One entry per active channel.
    pub channels: Vec<ChannelLevel>,
}

impl InputLevels {
    /// Build a reading from OBS's per-channel multipliers.
    pub fn from_mul(input: InputId, levels: &[[f32; 3]]) -> Self {
        Self {
            input,
            channels: levels.iter().copied().map(ChannelLevel::from_mul).collect(),
        }
    }

    /// How many channels this input reports.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Loudest peak across all channels, or `None` when the input is silent.
    pub fn loudest_peak_db(&self) -> Option<f64> {
        self.channels
            .iter()
            .map(|channel| channel.peak_db)
            .filter(|db| db.is_finite())
            .fold(None, |loudest: Option<f64>, db| {
                Some(loudest.map_or(db, |current| current.max(db)))
            })
    }
}

/// The most recent levels for every input OBS is currently metering.
///
/// Meter widgets poll this rather than being pushed to: OBS sends one update
/// for every active input about twenty times a second, and a widget only needs
/// its own input's row on the frames it actually draws. The sequence number
/// tells a widget whether it has already consumed the reading it is looking at,
/// which is what lets an input that stopped reporting decay to silence instead
/// of freezing on its last value.
#[derive(Debug, Default, Clone)]
pub struct InputLevelSnapshot {
    sequence: u64,
    levels: std::collections::HashMap<InputId, InputLevels>,
}

impl InputLevelSnapshot {
    /// Replace the snapshot with a fresh set of readings.
    pub fn update(&mut self, levels: Vec<InputLevels>) {
        self.sequence = self.sequence.wrapping_add(1);
        self.levels.clear();
        self.levels
            .extend(levels.into_iter().map(|entry| (entry.input.clone(), entry)));
    }

    /// Forget every reading, e.g. when the OBS connection drops.
    pub fn clear(&mut self) {
        if self.levels.is_empty() {
            return;
        }
        self.sequence = self.sequence.wrapping_add(1);
        self.levels.clear();
    }

    /// Counter that changes on every update, including one that clears levels.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Latest reading for one input, if OBS reported it in the last update.
    pub fn get(&self, input: &str) -> Option<&InputLevels> {
        self.levels.get(input)
    }

    /// Whether any input reported levels in the last update.
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }
}

/// User-facing name for a channel position, given the input's channel count.
///
/// Mono and stereo get plain names; anything wider uses the surround order OBS
/// documents, and unexpected extra channels fall back to their index.
pub fn channel_label(channel: usize, channel_count: usize) -> String {
    if channel_count <= 1 {
        return fl!(LANGUAGE_LOADER, "audio-meter-channel-mono");
    }
    if channel_count == 2 {
        return match channel {
            0 => fl!(LANGUAGE_LOADER, "audio-meter-channel-left"),
            _ => fl!(LANGUAGE_LOADER, "audio-meter-channel-right"),
        };
    }
    match channel {
        0 => fl!(LANGUAGE_LOADER, "audio-meter-channel-front-left"),
        1 => fl!(LANGUAGE_LOADER, "audio-meter-channel-front-right"),
        2 => fl!(LANGUAGE_LOADER, "audio-meter-channel-front-center"),
        3 => fl!(LANGUAGE_LOADER, "audio-meter-channel-lfe"),
        4 => fl!(LANGUAGE_LOADER, "audio-meter-channel-rear-left"),
        5 => fl!(LANGUAGE_LOADER, "audio-meter-channel-rear-right"),
        6 => fl!(LANGUAGE_LOADER, "audio-meter-channel-side-left"),
        7 => fl!(LANGUAGE_LOADER, "audio-meter-channel-side-right"),
        other => fl!(
            LANGUAGE_LOADER,
            "audio-meter-channel-numbered",
            index = (other + 1).to_string()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_is_zero_db_and_silence_is_negative_infinity() {
        assert!((db_from_mul(1.0) - 0.0).abs() < 1e-9);
        assert!((db_from_mul(0.5) - (-6.0206)).abs() < 1e-3);
        assert_eq!(db_from_mul(0.0), f64::NEG_INFINITY);
        assert_eq!(db_from_mul(-1.0), f64::NEG_INFINITY);
        assert_eq!(db_from_mul(f32::NAN), f64::NEG_INFINITY);
    }

    #[test]
    fn zones_match_the_obs_thresholds() {
        assert_eq!(MeterZone::for_db(-60.0), MeterZone::Nominal);
        assert_eq!(MeterZone::for_db(-20.1), MeterZone::Nominal);
        assert_eq!(MeterZone::for_db(-20.0), MeterZone::Warning);
        assert_eq!(MeterZone::for_db(-9.1), MeterZone::Warning);
        assert_eq!(MeterZone::for_db(-9.0), MeterZone::Error);
        assert_eq!(MeterZone::for_db(0.0), MeterZone::Error);
        assert_eq!(MeterZone::for_db(f64::NEG_INFINITY), MeterZone::Nominal);
    }

    #[test]
    fn zones_order_from_quiet_to_loud() {
        assert!(MeterZone::Nominal < MeterZone::Warning);
        assert!(MeterZone::Warning < MeterZone::Error);
    }

    #[test]
    fn channel_levels_keep_obs_channel_order() {
        let levels = InputLevels::from_mul(
            "Mic".to_string(),
            &[[1.0, 1.0, 1.0], [0.5, 0.5, 0.25], [0.0, 0.0, 0.0]],
        );

        assert_eq!(levels.channel_count(), 3);
        assert!((levels.channels[0].peak_db - 0.0).abs() < 1e-9);
        assert!((levels.channels[1].magnitude_db - (-6.0206)).abs() < 1e-3);
        assert!((levels.channels[1].input_peak_db - (-12.041)).abs() < 1e-3);
        assert_eq!(levels.channels[2], ChannelLevel::SILENT);
    }

    #[test]
    fn loudest_peak_ignores_silent_channels() {
        let levels = InputLevels::from_mul(
            "Desktop".to_string(),
            &[[0.0, 0.0, 0.0], [0.5, 0.5, 0.5], [0.1, 0.25, 0.25]],
        );

        let loudest = levels.loudest_peak_db().unwrap();
        assert!((loudest - (-6.0206)).abs() < 1e-3);
    }

    #[test]
    fn loudest_peak_is_none_for_a_fully_silent_input() {
        let levels = InputLevels::from_mul("Muted".to_string(), &[[0.0, 0.0, 0.0]]);

        assert_eq!(levels.loudest_peak_db(), None);
        assert_eq!(
            InputLevels::from_mul("None".to_string(), &[]).loudest_peak_db(),
            None
        );
    }

    #[test]
    fn a_snapshot_hands_out_the_latest_reading_per_input() {
        let mut snapshot = InputLevelSnapshot::default();
        assert!(snapshot.is_empty());

        snapshot.update(vec![
            InputLevels::from_mul("Mic".to_string(), &[[0.5, 0.5, 0.5]]),
            InputLevels::from_mul("Music".to_string(), &[[0.25, 0.25, 0.25]]),
        ]);
        let first = snapshot.sequence();

        assert_eq!(snapshot.get("Mic").unwrap().channel_count(), 1);
        assert!(snapshot.get("Missing").is_none());

        // An input that stops reporting drops out of the next update entirely.
        snapshot.update(vec![InputLevels::from_mul(
            "Mic".to_string(),
            &[[0.5, 0.5, 0.5]],
        )]);

        assert!(snapshot.get("Music").is_none());
        assert_ne!(snapshot.sequence(), first);
    }

    #[test]
    fn clearing_a_snapshot_moves_the_sequence_only_when_it_had_levels() {
        let mut snapshot = InputLevelSnapshot::default();
        snapshot.update(vec![InputLevels::from_mul(
            "Mic".to_string(),
            &[[0.5, 0.5, 0.5]],
        )]);

        snapshot.clear();
        let cleared = snapshot.sequence();
        assert!(snapshot.is_empty());
        assert!(snapshot.get("Mic").is_none());

        snapshot.clear();
        assert_eq!(snapshot.sequence(), cleared);
    }

    #[test]
    fn channel_labels_follow_the_reported_channel_count() {
        assert_eq!(channel_label(0, 1), "Mono");
        assert_eq!(channel_label(0, 2), "Left");
        assert_eq!(channel_label(1, 2), "Right");
        assert_eq!(channel_label(3, 6), "LFE");
        assert_eq!(channel_label(7, 8), "Side right");
        assert_eq!(channel_label(9, 10), "Channel 10");
    }
}
