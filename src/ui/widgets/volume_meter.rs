//! Volume meter for one audio input.
//!
//! Draws one LED column per channel, in OBS's channel order, against the same
//! −60…0 dB scale as the fader beside it. Each column carries the four things
//! OBS's own meter shows: the peak bar with its fall-off, the loudest peak of
//! the last twenty seconds, the loudness (VU) reading, and — at the base — the
//! level arriving from the device before the fader touches it.
//!
//! The widget pulls rather than being pushed to. `AppState` holds the newest
//! reading for every input; this widget picks up its own row on the frames it
//! draws, which keeps a twenty-per-second event stream off the widget tree and
//! lets the fall-off run smoothly between readings.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk4::cairo::Context;
use gtk4::gdk;
use gtk4::gdk::RGBA;
use gtk4::prelude::*;
use gtk4::DrawingArea;
use i18n_embed_fl::fl;

use crate::domain::meter::{channel_label, MeterZone, METER_CEILING_DB, METER_FLOOR_DB};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::meter_service::{meter_fraction, InputMeterState, MeterChannelDisplay};
use crate::ui::navigation::NavigationContext;

/// Height of the meter, matching the fader it stands next to.
pub(crate) const METER_HEIGHT: i32 = 128;
/// Width of one channel bar.
const BAR_WIDTH: f64 = 6.0;
/// Gap between the bars of a multi-channel source.
const BAR_GAP: f64 = 1.0;
/// Width reserved on the right for the decibel tick marks.
const TICK_AREA: f64 = 7.0;
/// Length of a tick at a labelled (6 dB) step.
const MAJOR_TICK: f64 = 5.0;
/// Length of a tick at an unlabelled (3 dB) step.
const MINOR_TICK: f64 = 3.0;
/// Height of the peak-hold line.
const HOLD_HEIGHT: f64 = 2.0;
/// Height of the loudness line drawn across the bar.
const LOUDNESS_HEIGHT: f64 = 1.0;
/// Space reserved at the bottom for the input-level square.
const INPUT_DOT_AREA: f64 = 9.0;
/// Side of the input-level square.
const INPUT_DOT_SIZE: f64 = 5.0;
/// Column count assumed before the first reading, so the card does not resize
/// the moment audio starts.
const ASSUMED_CHANNELS: usize = 2;

/// Longest fall-off applied in one frame.
///
/// Coming back to a page that was hidden for a minute should not make the bars
/// jump; this caps the catch-up to something a single frame can justify.
const MAX_FRAME_STEP: Duration = Duration::from_millis(250);

// Zone colours are deliberately fixed rather than themed: green, yellow and red
// mean the same thing here as they do in OBS, and a theme that recoloured them
// would be lying about the level. Each zone has a dim shade for the unlit track
// and a bright one for the level itself, which is what makes the meter readable
// at a glance from across a room.
const NOMINAL_BRIGHT: (f64, f64, f64) = (0.28, 0.92, 0.31);
const NOMINAL_DIM: (f64, f64, f64) = (0.07, 0.40, 0.15);
const WARNING_BRIGHT: (f64, f64, f64) = (0.93, 0.82, 0.20);
const WARNING_DIM: (f64, f64, f64) = (0.42, 0.36, 0.07);
const ERROR_BRIGHT: (f64, f64, f64) = (0.92, 0.25, 0.25);
const ERROR_DIM: (f64, f64, f64) = (0.42, 0.09, 0.09);

/// Widget plus the state its frame callback advances.
pub(crate) struct VolumeMeterHandle {
    /// Drawing area to pack into the card.
    pub(crate) root: DrawingArea,
}

#[derive(Debug)]
struct MeterWidgetState {
    meter: InputMeterState,
    /// Level snapshot generation already consumed, so a reading is never
    /// counted twice and a silent input is allowed to decay.
    consumed_sequence: u64,
    last_frame_us: Option<i64>,
    drawn: Vec<MeterChannelDisplay>,
}

impl MeterWidgetState {
    const fn new() -> Self {
        Self {
            meter: InputMeterState::new(),
            consumed_sequence: 0,
            last_frame_us: None,
            drawn: Vec::new(),
        }
    }
}

/// Build a volume meter bound to `input_id`.
pub(crate) fn build(input_id: &str, nav: NavigationContext) -> VolumeMeterHandle {
    let area = DrawingArea::builder()
        .content_width(meter_width(ASSUMED_CHANNELS))
        .content_height(METER_HEIGHT)
        .valign(gtk4::Align::Center)
        .build();
    area.add_css_class("audio-meter");

    let state = Rc::new(RefCell::new(MeterWidgetState::new()));
    set_meter_tooltip(&area, 0);

    area.set_draw_func({
        let state = Rc::clone(&state);
        move |_, cr, _width, height| {
            draw_meter(
                cr,
                &state.borrow().drawn,
                f64::from(height),
                adw::StyleManager::default().is_dark(),
            );
        }
    });

    area.add_tick_callback({
        let state = Rc::clone(&state);
        let input_id = input_id.to_string();
        move |area, clock| {
            advance_frame(area, clock, &state, &input_id, &nav);
            glib::ControlFlow::Continue
        }
    });

    VolumeMeterHandle { root: area }
}

/// Advance one frame: decay, take any new reading, redraw only if it shows.
fn advance_frame(
    area: &DrawingArea,
    clock: &gdk::FrameClock,
    state: &Rc<RefCell<MeterWidgetState>>,
    input_id: &str,
    nav: &NavigationContext,
) {
    let frame_us = clock.frame_time();
    // Read levels before touching the widget state: `AppState` is shared with
    // every other card, so the borrow is kept as short as possible.
    let reading = {
        let app_state = nav.state.borrow();
        let sequence = app_state.audio_levels.sequence();
        (sequence != state.borrow().consumed_sequence)
            .then(|| (sequence, app_state.audio_levels.get(input_id).cloned()))
    };

    let mut state = state.borrow_mut();
    let elapsed = frame_elapsed(state.last_frame_us, frame_us);
    state.last_frame_us = Some(frame_us);
    state.meter.advance(elapsed);

    if let Some((sequence, levels)) = reading {
        state.consumed_sequence = sequence;
        if let Some(levels) = levels {
            state.meter.observe(&levels);
        }
    }

    let display = state.meter.display();
    if display == state.drawn {
        return;
    }

    if display.len() != state.drawn.len() {
        // The widget keeps room for two columns even for a mono source, so a
        // row of cards does not jog sideways the moment audio starts.
        let channels = display.len().max(ASSUMED_CHANNELS);
        area.set_content_width(meter_width(channels));
        set_meter_tooltip(area, display.len());
    }
    state.drawn = display;
    drop(state);
    area.queue_draw();
}

/// Time since the previous frame, capped so a hidden page cannot bank decay.
fn frame_elapsed(last_frame_us: Option<i64>, frame_us: i64) -> Duration {
    let Some(last) = last_frame_us else {
        return Duration::ZERO;
    };
    let delta_us = frame_us.saturating_sub(last);
    if delta_us <= 0 {
        return Duration::ZERO;
    }
    Duration::from_micros(delta_us as u64).min(MAX_FRAME_STEP)
}

/// Paint the bars. Pure Cairo: the theme arrives as `dark` rather than being
/// looked up, so the drawing can be rendered and inspected without a display.
fn draw_meter(cr: &Context, channels: &[MeterChannelDisplay], height: f64, dark: bool) {
    let column_height = (height - INPUT_DOT_AREA).max(1.0);
    let column_count = drawn_column_count(channels);

    for index in 0..column_count {
        let x = column_x(index);
        let channel = channels
            .get(index)
            .copied()
            .unwrap_or(MeterChannelDisplay::EMPTY);

        // Unlit track first: the three zones, full height, in their dim shades.
        for zone in MeterZone::ALL {
            let (low_db, high_db) = zone.span();
            fill_span(
                cr,
                x,
                low_db,
                high_db,
                column_height,
                zone_rgba(zone, false),
            );
        }

        // Then the level on top, clipped zone by zone so a bar reaching the red
        // still shows bright green and yellow underneath it.
        if let Some(peak_db) = channel.peak_db {
            for zone in MeterZone::ALL {
                let (low_db, high_db) = zone.span();
                let high_db = high_db.min(peak_db);
                if high_db > low_db {
                    fill_span(cr, x, low_db, high_db, column_height, zone_rgba(zone, true));
                }
            }
        }

        if let Some(magnitude_db) = channel.magnitude_db {
            // Loudness reads as a notch cut across the lit bar, the way OBS's
            // VU indicator does.
            let y = level_y(magnitude_db, column_height);
            fill_rect(
                cr,
                x,
                y - LOUDNESS_HEIGHT,
                BAR_WIDTH,
                LOUDNESS_HEIGHT,
                neutral(dark, 0.55),
            );
        }

        if let Some(hold_db) = channel.hold_db {
            let y = level_y(hold_db, column_height);
            let colour = zone_rgba(MeterZone::for_db(hold_db), true);
            fill_rect(cr, x, y, BAR_WIDTH, HOLD_HEIGHT, colour);
        }

        let input_colour = match channel.input_peak_db {
            Some(db) => zone_rgba(MeterZone::for_db(db), true),
            None => neutral(dark, 0.16),
        };
        fill_rect(
            cr,
            x + (BAR_WIDTH - INPUT_DOT_SIZE) / 2.0,
            height - INPUT_DOT_AREA + (INPUT_DOT_AREA - INPUT_DOT_SIZE) / 2.0,
            INPUT_DOT_SIZE,
            INPUT_DOT_SIZE,
            input_colour,
        );
    }

    draw_ticks(cr, columns_width(column_count), column_height, dark);
}

/// Tick marks down the right-hand edge, long every 6 dB and short every 3 dB.
///
/// They line up with the printed ruler beside the meter, and give the eye
/// something to measure against where the ruler's numbers are too sparse.
fn draw_ticks(cr: &Context, x: f64, column_height: f64, dark: bool) {
    let colour = neutral(dark, 0.35);
    let mut db = METER_CEILING_DB;
    while db >= METER_FLOOR_DB {
        let labelled = (db / 6.0).abs().fract() < 1e-9;
        let length = if labelled { MAJOR_TICK } else { MINOR_TICK };
        let y = level_y(db, column_height);
        // Clamp the topmost tick inside the widget so it is not half drawn.
        let y = y.min(column_height - 1.0).max(0.0);
        fill_rect(cr, x + 1.0, y, length, 1.0, colour);
        db -= 3.0;
    }
}

/// Total width the channel bars occupy, ticks excluded.
fn columns_width(column_count: usize) -> f64 {
    if column_count == 0 {
        return 0.0;
    }
    column_count as f64 * BAR_WIDTH + (column_count - 1) as f64 * BAR_GAP
}

/// Width a meter with `column_count` channels asks for, ticks included.
fn meter_width(column_count: usize) -> i32 {
    (columns_width(column_count) + TICK_AREA).ceil() as i32
}

/// Left edge of a channel bar.
fn column_x(index: usize) -> f64 {
    index as f64 * (BAR_WIDTH + BAR_GAP)
}

/// How many columns to draw.
///
/// Before the first reading there is no channel count to go on, so an unlit
/// track stands in rather than an empty box. Once OBS reports, the drawing
/// shows exactly as many bars as the source has — a mono source must not appear
/// to have a dead second channel.
fn drawn_column_count(channels: &[MeterChannelDisplay]) -> usize {
    if channels.is_empty() {
        ASSUMED_CHANNELS
    } else {
        channels.len()
    }
}

/// Vertical position of a decibel level on the bar.
fn level_y(db: f64, column_height: f64) -> f64 {
    column_height * (1.0 - meter_fraction(db))
}

/// Fill the part of a bar between two decibel levels.
fn fill_span(cr: &Context, x: f64, low_db: f64, high_db: f64, column_height: f64, colour: RGBA) {
    let top = level_y(high_db, column_height);
    let bottom = level_y(low_db, column_height);
    fill_rect(cr, x, top, BAR_WIDTH, (bottom - top).max(0.0), colour);
}

fn zone_rgba(zone: MeterZone, bright: bool) -> RGBA {
    let (red, green, blue) = match (zone, bright) {
        (MeterZone::Nominal, true) => NOMINAL_BRIGHT,
        (MeterZone::Nominal, false) => NOMINAL_DIM,
        (MeterZone::Warning, true) => WARNING_BRIGHT,
        (MeterZone::Warning, false) => WARNING_DIM,
        (MeterZone::Error, true) => ERROR_BRIGHT,
        (MeterZone::Error, false) => ERROR_DIM,
    };
    RGBA::new(red as f32, green as f32, blue as f32, 1.0)
}

/// Neutral ink that stays visible on both light and dark themes.
fn neutral(dark: bool, alpha: f64) -> RGBA {
    let level = if dark { 1.0 } else { 0.0 };
    RGBA::new(level, level, level, alpha as f32)
}

fn fill_rect(cr: &Context, x: f64, y: f64, width: f64, height: f64, colour: RGBA) {
    cr.set_source_rgba(
        f64::from(colour.red()),
        f64::from(colour.green()),
        f64::from(colour.blue()),
        f64::from(colour.alpha()),
    );
    cr.rectangle(x, y, width, height);
    let _ = cr.fill();
}

/// Describe the meter, naming the channels it is currently showing.
fn set_meter_tooltip(area: &DrawingArea, channel_count: usize) {
    if channel_count == 0 {
        area.set_tooltip_text(Some(&fl!(
            LANGUAGE_LOADER,
            "audio-card-meter-tooltip-waiting"
        )));
        return;
    }
    let channels = (0..channel_count)
        .map(|channel| channel_label(channel, channel_count))
        .collect::<Vec<_>>()
        .join(", ");
    let tooltip = [
        fl!(
            LANGUAGE_LOADER,
            "audio-card-meter-tooltip-title",
            channels = channels
        ),
        fl!(LANGUAGE_LOADER, "audio-card-meter-tooltip-zones"),
        fl!(LANGUAGE_LOADER, "audio-card-meter-tooltip-indicators"),
    ]
    .join("\n");
    area.set_tooltip_text(Some(&tooltip));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::meter::{METER_CEILING_DB, METER_FLOOR_DB};

    #[test]
    fn the_bar_runs_from_the_floor_at_the_bottom_to_unity_at_the_top() {
        let height = 100.0;

        assert!((level_y(METER_CEILING_DB, height) - 0.0).abs() < 1e-9);
        assert!((level_y(METER_FLOOR_DB, height) - height).abs() < 1e-9);
        assert!(level_y(-30.0, height) > level_y(-9.0, height));
    }

    #[test]
    fn zone_spans_stack_up_the_bar_without_overlapping() {
        let height = 120.0;
        let mut previous_top = height;
        for zone in MeterZone::ALL {
            let (low_db, high_db) = zone.span();
            let top = level_y(high_db, height);
            let bottom = level_y(low_db, height);
            assert!(top < bottom, "{zone:?} has no height");
            assert!(
                (bottom - previous_top).abs() < 1e-9,
                "{zone:?} does not meet the zone below it"
            );
            previous_top = top;
        }
        assert!(previous_top.abs() < 1e-9, "the top zone must reach the top");
    }

    #[test]
    fn bright_zone_colours_outshine_their_dim_track() {
        for zone in MeterZone::ALL {
            let bright = zone_rgba(zone, true);
            let dim = zone_rgba(zone, false);
            let luma = |colour: RGBA| colour.red() + colour.green() + colour.blue();
            assert!(
                luma(bright) > luma(dim),
                "{zone:?} is not brighter when lit"
            );
        }
        assert!(
            zone_rgba(MeterZone::Nominal, true).green() > zone_rgba(MeterZone::Nominal, true).red()
        );
        assert!(
            zone_rgba(MeterZone::Error, true).red() > zone_rgba(MeterZone::Error, true).green()
        );
    }

    #[test]
    fn the_meter_widens_with_the_channel_count_and_always_leaves_room_for_ticks() {
        let mono = meter_width(1);
        let stereo = meter_width(2);
        let surround = meter_width(6);

        assert!(mono < stereo && stereo < surround);
        assert!(f64::from(mono) >= BAR_WIDTH + TICK_AREA);
        assert_eq!(columns_width(0), 0.0);
        assert!((columns_width(2) - (2.0 * BAR_WIDTH + BAR_GAP)).abs() < 1e-9);
    }

    #[test]
    fn channel_bars_sit_side_by_side_without_overlapping() {
        assert_eq!(column_x(0), 0.0);
        assert!(column_x(1) >= BAR_WIDTH);
        assert!((column_x(2) - column_x(1) - (BAR_WIDTH + BAR_GAP)).abs() < 1e-9);
    }

    #[test]
    fn a_mono_source_draws_one_column_but_an_unread_meter_shows_placeholders() {
        assert_eq!(drawn_column_count(&[]), ASSUMED_CHANNELS);
        assert_eq!(drawn_column_count(&[MeterChannelDisplay::EMPTY]), 1);
        assert_eq!(
            drawn_column_count(&[MeterChannelDisplay::EMPTY; 6]),
            6,
            "surround sources get a column each"
        );
    }

    #[test]
    fn the_first_frame_advances_nothing() {
        assert_eq!(frame_elapsed(None, 1_000), Duration::ZERO);
    }

    #[test]
    fn a_long_gap_between_frames_is_capped() {
        assert_eq!(
            frame_elapsed(Some(0), 10_000_000),
            MAX_FRAME_STEP,
            "a page that was hidden must not bank up decay"
        );
    }

    #[test]
    fn a_normal_frame_gap_passes_through() {
        assert_eq!(
            frame_elapsed(Some(1_000_000), 1_016_000),
            Duration::from_micros(16_000)
        );
    }

    #[test]
    fn a_frame_clock_that_goes_backwards_advances_nothing() {
        assert_eq!(frame_elapsed(Some(2_000), 1_000), Duration::ZERO);
    }

    #[test]
    fn neutral_ink_inverts_with_the_theme() {
        assert!(neutral(true, 1.0).red() > 0.5);
        assert!(neutral(false, 1.0).red() < 0.5);
    }
}
