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

use crate::domain::meter::{channel_label, MeterZone};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::meter_service::{
    meter_db_at, segment_is_lit, InputMeterState, MeterChannelDisplay,
};
use crate::ui::navigation::NavigationContext;

/// Height of the meter, matching the fader it stands next to.
pub(crate) const METER_HEIGHT: i32 = 128;
/// LED segments per channel column.
const SEGMENTS: usize = 22;
/// Horizontal space one channel column occupies, gap included.
const CHANNEL_WIDTH: f64 = 7.0;
/// Radius of one LED dot.
const DOT_RADIUS: f64 = 2.4;
/// Radius of the loudness dot, drawn over the column.
const LOUDNESS_RADIUS: f64 = 1.7;
/// Height of the peak-hold bar.
const HOLD_HEIGHT: f64 = 2.0;
/// Space reserved at the bottom for the input-level dot.
const INPUT_DOT_AREA: f64 = 10.0;
/// Column count assumed before the first reading, so the card does not resize
/// the moment audio starts.
const ASSUMED_CHANNELS: usize = 2;

/// Longest fall-off applied in one frame.
///
/// Coming back to a page that was hidden for a minute should not make the bars
/// jump; this caps the catch-up to something a single frame can justify.
const MAX_FRAME_STEP: Duration = Duration::from_millis(250);

// The three zone colours are deliberately fixed rather than themed: green,
// yellow, and red mean the same thing here as they do in OBS, and a theme that
// recoloured them would be lying about the level.
const NOMINAL_RGBA: (f64, f64, f64) = (0.184, 0.658, 0.310);
const WARNING_RGBA: (f64, f64, f64) = (0.831, 0.725, 0.227);
const ERROR_RGBA: (f64, f64, f64) = (0.816, 0.267, 0.267);

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
        .content_width((ASSUMED_CHANNELS as f64 * CHANNEL_WIDTH) as i32)
        .content_height(METER_HEIGHT)
        .valign(gtk4::Align::Center)
        .build();
    area.add_css_class("audio-meter");

    let state = Rc::new(RefCell::new(MeterWidgetState::new()));
    set_meter_tooltip(&area, 0);

    area.set_draw_func({
        let state = Rc::clone(&state);
        move |_, cr, width, height| {
            draw_meter(
                cr,
                &state.borrow().drawn,
                f64::from(width),
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
        area.set_content_width((channels as f64 * CHANNEL_WIDTH) as i32);
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

/// Paint the columns. Pure Cairo: the theme arrives as `dark` rather than being
/// looked up, so the drawing can be rendered and inspected without a display.
fn draw_meter(cr: &Context, channels: &[MeterChannelDisplay], width: f64, height: f64, dark: bool) {
    let unlit = neutral(dark, if dark { 0.13 } else { 0.20 });
    let column_height = (height - INPUT_DOT_AREA).max(1.0);

    let column_count = drawn_column_count(channels);
    let columns = (0..column_count).map(|index| (index, channels.get(index).copied()));

    for (index, channel) in columns {
        let centre_x = column_centre(index, column_count, width);
        let channel = channel.unwrap_or(MeterChannelDisplay::EMPTY);

        for segment in 0..SEGMENTS {
            let segment_db = segment_db(segment);
            let y = segment_y(segment, column_height);
            let colour = if segment_is_lit(segment_db, channel.peak_db) {
                zone_rgba(MeterZone::for_db(segment_db), 1.0)
            } else {
                unlit
            };
            fill_dot(cr, centre_x, y, DOT_RADIUS, colour);
        }

        if let Some(hold_db) = channel.hold_db {
            let y = level_y(hold_db, column_height);
            fill_bar(
                cr,
                centre_x,
                y,
                CHANNEL_WIDTH - 2.0,
                HOLD_HEIGHT,
                zone_rgba(MeterZone::for_db(hold_db), 1.0),
            );
        }

        if let Some(magnitude_db) = channel.magnitude_db {
            let y = level_y(magnitude_db, column_height);
            fill_dot(cr, centre_x, y, LOUDNESS_RADIUS, neutral(dark, 0.85));
        }

        let input_colour = match channel.input_peak_db {
            Some(db) => zone_rgba(MeterZone::for_db(db), 1.0),
            None => unlit,
        };
        let input_dot_y = height - INPUT_DOT_AREA / 2.0;
        fill_dot(cr, centre_x, input_dot_y, DOT_RADIUS, input_colour);
    }
}

/// How many columns to draw.
///
/// Before the first reading there is no channel count to go on, so unlit
/// placeholder columns stand in rather than an empty box. Once OBS reports, the
/// drawing shows exactly as many columns as the source has — a mono source must
/// not appear to have a dead second channel.
fn drawn_column_count(channels: &[MeterChannelDisplay]) -> usize {
    if channels.is_empty() {
        ASSUMED_CHANNELS
    } else {
        channels.len()
    }
}

/// Horizontal centre of a channel column.
fn column_centre(index: usize, column_count: usize, width: f64) -> f64 {
    let used = column_count as f64 * CHANNEL_WIDTH;
    let offset = ((width - used) / 2.0).max(0.0);
    offset + index as f64 * CHANNEL_WIDTH + CHANNEL_WIDTH / 2.0
}

/// Decibel level at the centre of an LED segment, counting up from the floor.
fn segment_db(segment: usize) -> f64 {
    meter_db_at((segment as f64 + 0.5) / SEGMENTS as f64)
}

/// Vertical centre of an LED segment.
fn segment_y(segment: usize, column_height: f64) -> f64 {
    let fraction = (segment as f64 + 0.5) / SEGMENTS as f64;
    column_height * (1.0 - fraction)
}

/// Vertical position of a decibel level on the column.
fn level_y(db: f64, column_height: f64) -> f64 {
    column_height * (1.0 - crate::services::meter_service::meter_fraction(db))
}

fn zone_rgba(zone: MeterZone, alpha: f64) -> RGBA {
    let (red, green, blue) = match zone {
        MeterZone::Nominal => NOMINAL_RGBA,
        MeterZone::Warning => WARNING_RGBA,
        MeterZone::Error => ERROR_RGBA,
    };
    RGBA::new(red as f32, green as f32, blue as f32, alpha as f32)
}

/// Neutral ink that stays visible on both light and dark themes.
fn neutral(dark: bool, alpha: f64) -> RGBA {
    let level = if dark { 1.0 } else { 0.0 };
    RGBA::new(level, level, level, alpha as f32)
}

fn fill_dot(cr: &Context, centre_x: f64, centre_y: f64, radius: f64, colour: RGBA) {
    set_source(cr, colour);
    cr.arc(centre_x, centre_y, radius, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
}

fn fill_bar(cr: &Context, centre_x: f64, centre_y: f64, width: f64, height: f64, colour: RGBA) {
    set_source(cr, colour);
    cr.rectangle(
        centre_x - width / 2.0,
        centre_y - height / 2.0,
        width,
        height,
    );
    let _ = cr.fill();
}

fn set_source(cr: &Context, colour: RGBA) {
    cr.set_source_rgba(
        f64::from(colour.red()),
        f64::from(colour.green()),
        f64::from(colour.blue()),
        f64::from(colour.alpha()),
    );
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
    fn segments_run_from_the_floor_up_to_the_ceiling() {
        let lowest = segment_db(0);
        let highest = segment_db(SEGMENTS - 1);

        assert!(lowest > METER_FLOOR_DB && lowest < METER_FLOOR_DB + 3.0);
        assert!(highest < METER_CEILING_DB && highest > METER_CEILING_DB - 3.0);
        assert!((0..SEGMENTS)
            .map(segment_db)
            .collect::<Vec<_>>()
            .windows(2)
            .all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn the_top_segments_are_red_and_the_bottom_ones_green() {
        assert_eq!(MeterZone::for_db(segment_db(0)), MeterZone::Nominal);
        assert_eq!(
            MeterZone::for_db(segment_db(SEGMENTS - 1)),
            MeterZone::Error
        );
    }

    #[test]
    fn quiet_levels_draw_near_the_bottom_of_the_column() {
        let height = 100.0;

        assert!((level_y(METER_CEILING_DB, height) - 0.0).abs() < 1e-9);
        assert!((level_y(METER_FLOOR_DB, height) - height).abs() < 1e-9);
        assert!(level_y(-30.0, height) > level_y(-9.0, height));
    }

    #[test]
    fn segment_positions_stay_inside_the_column() {
        let height = 100.0;
        for segment in 0..SEGMENTS {
            let y = segment_y(segment, height);
            assert!((0.0..=height).contains(&y), "segment {segment} at {y}");
        }
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
    fn columns_are_centred_in_the_drawing_area() {
        // A single column in a two-column-wide area sits in the left half.
        let single = column_centre(0, 1, 2.0 * CHANNEL_WIDTH);
        assert!((single - CHANNEL_WIDTH).abs() < 1e-9);

        let left = column_centre(0, 2, 2.0 * CHANNEL_WIDTH);
        let right = column_centre(1, 2, 2.0 * CHANNEL_WIDTH);
        assert!(left < right);
        assert!((right - left - CHANNEL_WIDTH).abs() < 1e-9);
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
    fn zone_colours_are_distinct() {
        let nominal = zone_rgba(MeterZone::Nominal, 1.0);
        let warning = zone_rgba(MeterZone::Warning, 1.0);
        let error = zone_rgba(MeterZone::Error, 1.0);

        assert_ne!(nominal, warning);
        assert_ne!(warning, error);
        assert!(nominal.green() > nominal.red());
        assert!(error.red() > error.green());
    }

    #[test]
    fn neutral_ink_inverts_with_the_theme() {
        assert!(neutral(true, 1.0).red() > 0.5);
        assert!(neutral(false, 1.0).red() < 0.5);
    }
}
