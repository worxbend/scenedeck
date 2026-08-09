//! Volume-meter ballistics: how a reading becomes something you can read.
//!
//! OBS sends raw levels roughly every 50 ms. Drawing those directly would be
//! unreadable — the bar would flicker and a short peak would vanish before you
//! saw it. This module applies the same treatment OBS's own meter does: the
//! peak bar jumps up instantly and falls off at a fixed rate, and the loudest
//! peak is held for twenty seconds so clipping cannot slip past unnoticed.
//!
//! Everything here is pure. Callers pass the elapsed time, so the ballistics
//! are testable without a clock or a frame.

use std::time::Duration;

use crate::domain::meter::{
    ChannelLevel, InputLevels, MeterZone, METER_CEILING_DB, METER_CLIP_DB, METER_FLOOR_DB,
};

/// Fall-off of the peak bar, in decibels per second.
///
/// Matches the "Fast" audio meter decay rate in OBS's audio settings, which is
/// what a fresh OBS install uses.
pub const PEAK_DECAY_DB_PER_SECOND: f64 = 23.53;

/// How long the peak-hold marker stays put before it drops to the live peak.
pub const PEAK_HOLD: Duration = Duration::from_secs(20);

/// How long a channel keeps its last reading before it is treated as silent.
///
/// OBS stops sending updates for an input that goes inactive; without this the
/// bar would freeze wherever it happened to be.
pub const LEVEL_STALE_AFTER: Duration = Duration::from_millis(500);

/// Ready-to-draw values for one channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterChannelDisplay {
    /// Peak bar, after fall-off. `None` means the bar is empty.
    pub peak_db: Option<f64>,
    /// Sound pressure, the closest of the three to perceived loudness.
    pub magnitude_db: Option<f64>,
    /// Loudest peak within the hold window.
    pub hold_db: Option<f64>,
    /// Pre-fader peak, which shows what the device itself is sending.
    pub input_peak_db: Option<f64>,
}

impl MeterChannelDisplay {
    /// Nothing to draw.
    pub const EMPTY: Self = Self {
        peak_db: None,
        magnitude_db: None,
        hold_db: None,
        input_peak_db: None,
    };

    /// Zone the peak bar currently sits in, or `None` while it is empty.
    pub fn zone(&self) -> Option<MeterZone> {
        self.peak_db.map(MeterZone::for_db)
    }

    /// Whether the channel is at or past the clipping threshold.
    pub fn is_clipping(&self) -> bool {
        self.peak_db.is_some_and(|db| db >= METER_CLIP_DB)
    }
}

/// Peak, hold, and loudness state for one channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct MeterChannelState {
    peak_db: Option<f64>,
    magnitude_db: Option<f64>,
    input_peak_db: Option<f64>,
    hold_db: Option<f64>,
    hold_age: Duration,
    since_update: Duration,
}

impl MeterChannelState {
    /// A channel that has never received a reading.
    pub const fn new() -> Self {
        Self {
            peak_db: None,
            magnitude_db: None,
            input_peak_db: None,
            hold_db: None,
            hold_age: Duration::ZERO,
            since_update: Duration::ZERO,
        }
    }

    /// Take a fresh reading from OBS.
    ///
    /// The peak bar only ever jumps *up* here; coming down is [`Self::advance`]'s
    /// job, which is what makes a brief transient visible at all.
    pub fn observe(&mut self, level: ChannelLevel) {
        self.since_update = Duration::ZERO;
        self.magnitude_db = visible_db(level.magnitude_db);

        if let Some(peak_db) = visible_db(level.peak_db) {
            self.peak_db = Some(self.peak_db.map_or(peak_db, |current| current.max(peak_db)));
            if self.hold_db.is_none_or(|hold| peak_db >= hold) {
                self.hold_db = Some(peak_db);
                self.hold_age = Duration::ZERO;
            }
        }

        if let Some(input_peak_db) = visible_db(level.input_peak_db) {
            self.input_peak_db = Some(
                self.input_peak_db
                    .map_or(input_peak_db, |current| current.max(input_peak_db)),
            );
        }
    }

    /// Advance the fall-off and the hold timer by `elapsed`.
    pub fn advance(&mut self, elapsed: Duration) {
        if elapsed.is_zero() {
            return;
        }
        let fall_db = PEAK_DECAY_DB_PER_SECOND * elapsed.as_secs_f64();
        self.peak_db = decay(self.peak_db, fall_db);
        self.input_peak_db = decay(self.input_peak_db, fall_db);

        self.since_update = self.since_update.saturating_add(elapsed);
        // A channel that stopped reporting is silent, not frozen: the peak bar
        // is still falling, but loudness has no meaningful value any more.
        if self.since_update >= LEVEL_STALE_AFTER {
            self.magnitude_db = None;
        }

        self.hold_age = self.hold_age.saturating_add(elapsed);
        if self.hold_age >= PEAK_HOLD {
            self.hold_db = self.peak_db;
            self.hold_age = Duration::ZERO;
        }
    }

    /// Current values to draw.
    pub const fn display(&self) -> MeterChannelDisplay {
        MeterChannelDisplay {
            peak_db: self.peak_db,
            magnitude_db: self.magnitude_db,
            hold_db: self.hold_db,
            input_peak_db: self.input_peak_db,
        }
    }
}

/// Meter state for one input, one entry per channel.
#[derive(Debug, Clone, Default)]
pub struct InputMeterState {
    channels: Vec<MeterChannelState>,
}

impl InputMeterState {
    /// A meter that has never received a reading.
    pub const fn new() -> Self {
        Self {
            channels: Vec::new(),
        }
    }

    /// Take a fresh reading, growing or shrinking to the reported channel count.
    ///
    /// The count changes when OBS switches an input between mono, stereo, and
    /// surround, so the meter follows rather than keeping stale columns.
    pub fn observe(&mut self, levels: &InputLevels) {
        self.channels
            .resize(levels.channels.len(), MeterChannelState::new());
        for (state, level) in self.channels.iter_mut().zip(levels.channels.iter()) {
            state.observe(*level);
        }
    }

    /// Advance every channel's ballistics by `elapsed`.
    pub fn advance(&mut self, elapsed: Duration) {
        for channel in &mut self.channels {
            channel.advance(elapsed);
        }
    }

    /// How many channels are being metered.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Values to draw, one per channel.
    pub fn display(&self) -> Vec<MeterChannelDisplay> {
        self.channels.iter().map(|state| state.display()).collect()
    }

    /// Whether every channel is empty, so the widget can skip a redraw.
    pub fn is_idle(&self) -> bool {
        self.channels
            .iter()
            .all(|channel| channel.display() == MeterChannelDisplay::EMPTY)
    }
}

/// Position of `db` on the meter, `0.0` at the floor and `1.0` at the ceiling.
///
/// The scale is linear in decibels, matching the ruler printed beside the
/// fader, so a mark and a level at the same height mean the same thing.
pub fn meter_fraction(db: f64) -> f64 {
    if !db.is_finite() {
        return 0.0;
    }
    ((db - METER_FLOOR_DB) / (METER_CEILING_DB - METER_FLOOR_DB)).clamp(0.0, 1.0)
}

/// Decibel level at `fraction` of the way up the meter.
pub fn meter_db_at(fraction: f64) -> f64 {
    METER_FLOOR_DB + (METER_CEILING_DB - METER_FLOOR_DB) * fraction.clamp(0.0, 1.0)
}

/// Whether the LED segment centred on `segment_db` is lit at `peak_db`.
pub fn segment_is_lit(segment_db: f64, peak_db: Option<f64>) -> bool {
    peak_db.is_some_and(|peak| peak >= segment_db)
}

/// Clamp a reading into the range the meter draws, or `None` when inaudible.
fn visible_db(db: f64) -> Option<f64> {
    if !db.is_finite() || db < METER_FLOOR_DB {
        return None;
    }
    Some(db.min(METER_CEILING_DB))
}

/// Drop a level by `fall_db`, returning `None` once it reaches the floor.
fn decay(db: Option<f64>, fall_db: f64) -> Option<f64> {
    let fallen = db? - fall_db;
    (fallen > METER_FLOOR_DB).then_some(fallen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(magnitude: f64, peak: f64, input_peak: f64) -> ChannelLevel {
        ChannelLevel {
            magnitude_db: magnitude,
            peak_db: peak,
            input_peak_db: input_peak,
        }
    }

    fn observed(peak: f64) -> MeterChannelState {
        let mut state = MeterChannelState::new();
        state.observe(level(peak - 3.0, peak, peak));
        state
    }

    #[test]
    fn a_new_channel_has_nothing_to_draw() {
        assert_eq!(
            MeterChannelState::new().display(),
            MeterChannelDisplay::EMPTY
        );
    }

    #[test]
    fn the_peak_bar_jumps_up_immediately() {
        let mut state = observed(-30.0);
        state.observe(level(-15.0, -12.0, -12.0));

        assert_eq!(state.display().peak_db, Some(-12.0));
        assert_eq!(state.display().magnitude_db, Some(-15.0));
    }

    #[test]
    fn the_peak_bar_only_comes_down_by_decaying() {
        let mut state = observed(-12.0);
        state.observe(level(-40.0, -36.0, -36.0));

        assert_eq!(state.display().peak_db, Some(-12.0));

        state.advance(Duration::from_secs(1));
        let peak = state.display().peak_db.unwrap();
        assert!((peak - (-12.0 - PEAK_DECAY_DB_PER_SECOND)).abs() < 1e-9);
    }

    #[test]
    fn decay_empties_the_bar_once_it_passes_the_floor() {
        let mut state = observed(-12.0);

        state.advance(Duration::from_secs(5));

        assert_eq!(state.display().peak_db, None);
        assert_eq!(
            state.display().hold_db,
            Some(-12.0),
            "the hold marker outlives the bar, which is the point of it"
        );
    }

    #[test]
    fn magnitude_follows_the_reading_without_decay() {
        let mut state = observed(-12.0);
        state.observe(level(-50.0, -12.0, -12.0));

        assert_eq!(state.display().magnitude_db, Some(-50.0));
    }

    #[test]
    fn a_channel_that_stops_reporting_loses_its_loudness_reading() {
        let mut state = observed(-12.0);

        state.advance(LEVEL_STALE_AFTER);

        assert_eq!(state.display().magnitude_db, None);
        assert!(
            state.display().peak_db.is_some(),
            "the bar should still be falling"
        );
    }

    #[test]
    fn the_hold_marker_keeps_the_loudest_peak() {
        let mut state = observed(-6.0);
        state.advance(Duration::from_millis(500));
        state.observe(level(-40.0, -36.0, -36.0));

        assert_eq!(state.display().hold_db, Some(-6.0));
    }

    #[test]
    fn a_louder_peak_replaces_the_hold_and_restarts_its_timer() {
        let mut state = observed(-24.0);
        state.advance(Duration::from_secs(19));
        state.observe(level(-6.0, -3.0, -3.0));

        assert_eq!(state.display().hold_db, Some(-3.0));

        state.advance(Duration::from_secs(19));
        assert_eq!(
            state.display().hold_db,
            Some(-3.0),
            "the timer should have restarted"
        );
    }

    #[test]
    fn the_hold_marker_drops_to_the_live_peak_after_the_hold_window() {
        let mut state = observed(-6.0);

        // Keep the bar alive so the hold has something to drop to.
        let mut elapsed = Duration::ZERO;
        while elapsed < PEAK_HOLD + Duration::from_millis(100) {
            state.advance(Duration::from_millis(100));
            state.observe(level(-33.0, -30.0, -30.0));
            elapsed += Duration::from_millis(100);
        }

        let display = state.display();
        assert_eq!(display.hold_db, display.peak_db);
        assert!(display.hold_db.unwrap() < -6.0);
    }

    #[test]
    fn silence_and_nonsense_readings_leave_the_meter_empty() {
        let mut state = MeterChannelState::new();

        state.observe(ChannelLevel::SILENT);
        assert_eq!(state.display(), MeterChannelDisplay::EMPTY);

        state.observe(level(f64::NAN, -90.0, f64::NEG_INFINITY));
        assert_eq!(state.display(), MeterChannelDisplay::EMPTY);
    }

    #[test]
    fn readings_above_unity_are_clamped_to_the_ceiling() {
        let mut state = MeterChannelState::new();
        state.observe(level(3.0, 6.0, 6.0));

        assert_eq!(state.display().peak_db, Some(METER_CEILING_DB));
        assert!(state.display().is_clipping());
    }

    #[test]
    fn advancing_by_nothing_changes_nothing() {
        let mut state = observed(-12.0);
        let before = state.display();

        state.advance(Duration::ZERO);

        assert_eq!(state.display(), before);
    }

    #[test]
    fn the_display_reports_its_zone_and_clipping() {
        let mut state = observed(-30.0);
        assert_eq!(state.display().zone(), Some(MeterZone::Nominal));
        assert!(!state.display().is_clipping());

        state.observe(level(-12.0, -10.0, -10.0));
        assert_eq!(state.display().zone(), Some(MeterZone::Warning));

        state.observe(level(-4.0, -2.0, -2.0));
        assert_eq!(state.display().zone(), Some(MeterZone::Error));
        assert!(!state.display().is_clipping());

        state.observe(level(-1.0, -0.2, -0.2));
        assert!(state.display().is_clipping());

        assert_eq!(MeterChannelDisplay::EMPTY.zone(), None);
    }

    #[test]
    fn an_input_meter_follows_the_reported_channel_count() {
        let mut meter = InputMeterState::new();
        assert!(meter.is_idle());

        meter.observe(&InputLevels::from_mul(
            "Mic".to_string(),
            &[[0.5, 0.5, 0.5], [0.25, 0.25, 0.25]],
        ));
        assert_eq!(meter.channel_count(), 2);
        assert!(!meter.is_idle());

        meter.observe(&InputLevels::from_mul(
            "Mic".to_string(),
            &[[0.5, 0.5, 0.5]],
        ));
        assert_eq!(meter.channel_count(), 1);
    }

    #[test]
    fn an_input_meter_goes_idle_once_every_channel_empties() {
        let mut meter = InputMeterState::new();
        meter.observe(&InputLevels::from_mul(
            "Mic".to_string(),
            &[[0.5, 0.5, 0.5], [0.5, 0.5, 0.5]],
        ));

        meter.advance(Duration::from_secs(5));
        assert!(
            !meter.is_idle(),
            "the hold markers are still on screen and must keep being drawn"
        );

        meter.advance(PEAK_HOLD);

        assert!(meter.is_idle());
        assert!(meter
            .display()
            .iter()
            .all(|channel| *channel == MeterChannelDisplay::EMPTY));
    }

    #[test]
    fn the_scale_is_linear_in_decibels_between_floor_and_ceiling() {
        assert!((meter_fraction(METER_FLOOR_DB) - 0.0).abs() < 1e-9);
        assert!((meter_fraction(METER_CEILING_DB) - 1.0).abs() < 1e-9);
        assert!((meter_fraction(-30.0) - 0.5).abs() < 1e-9);
        assert!((meter_fraction(-120.0) - 0.0).abs() < 1e-9);
        assert!((meter_fraction(12.0) - 1.0).abs() < 1e-9);
        assert!((meter_fraction(f64::NEG_INFINITY) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn the_scale_round_trips_through_a_fraction() {
        for db in [-60.0, -42.0, -20.0, -9.0, 0.0] {
            assert!((meter_db_at(meter_fraction(db)) - db).abs() < 1e-9);
        }
        assert!((meter_db_at(-1.0) - METER_FLOOR_DB).abs() < 1e-9);
        assert!((meter_db_at(2.0) - METER_CEILING_DB).abs() < 1e-9);
    }

    #[test]
    fn segments_light_up_to_the_peak_and_no_further() {
        assert!(segment_is_lit(-30.0, Some(-12.0)));
        assert!(segment_is_lit(-12.0, Some(-12.0)));
        assert!(!segment_is_lit(-6.0, Some(-12.0)));
        assert!(!segment_is_lit(-60.0, None));
    }
}
