//! Cairo-drawn charts and gauges used by the Stats page.
//!
//! Each widget reads its values through a closure at draw time rather than
//! caching a copy, so a page only has to call [`Chart::redraw`] /
//! [`Gauge::redraw`] when new samples land. The scaling and threshold maths is
//! kept as free functions so it can be unit tested without a GTK display.

use std::f64::consts::PI;

use gtk4::cairo::{Context, FontSlant, FontWeight};
use gtk4::prelude::*;
use gtk4::DrawingArea;

const CHART_HEIGHT: i32 = 132;
/// Floor for a chart's width. Below this the axis gutter crowds out the series;
/// it is also what lets a reflowing container know when to drop to one column.
const CHART_MIN_WIDTH: i32 = 240;
const GAUGE_SIZE: i32 = 148;
const GRID_LINES: usize = 4;

/// An RGB colour in the `0.0..=1.0` range cairo expects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Color(pub f64, pub f64, pub f64);

impl Color {
    fn apply(self, ctx: &Context) {
        ctx.set_source_rgb(self.0, self.1, self.2);
    }

    fn apply_with_alpha(self, ctx: &Context, alpha: f64) {
        ctx.set_source_rgba(self.0, self.1, self.2, alpha);
    }
}

/// How healthy a reading is, which selects the accent colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Severity {
    /// Within the expected operating range.
    Good,
    /// Worth watching, but not yet failing.
    Warning,
    /// Actively degrading the stream.
    Critical,
}

impl Severity {
    /// Classify `value` against ascending warning and critical thresholds.
    pub(crate) fn from_thresholds(value: f64, warning: f64, critical: f64) -> Self {
        if value >= critical {
            Self::Critical
        } else if value >= warning {
            Self::Warning
        } else {
            Self::Good
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Good => Color(0.20, 0.60, 0.95),
            Self::Warning => Color(0.91, 0.62, 0.09),
            Self::Critical => Color(0.87, 0.25, 0.25),
        }
    }
}

/// Theme-aware chrome colours, resolved per draw so a light/dark switch is
/// picked up without rebuilding the widgets.
struct Palette {
    grid: Color,
    axis_text: Color,
    value_text: Color,
    muted_text: Color,
    track: Color,
}

impl Palette {
    fn for_current_theme() -> Self {
        if adw::StyleManager::default().is_dark() {
            Self {
                grid: Color(0.22, 0.23, 0.27),
                axis_text: Color(0.55, 0.58, 0.63),
                value_text: Color(0.93, 0.94, 0.96),
                muted_text: Color(0.62, 0.65, 0.70),
                track: Color(0.20, 0.21, 0.25),
            }
        } else {
            Self {
                grid: Color(0.88, 0.89, 0.91),
                axis_text: Color(0.45, 0.48, 0.53),
                value_text: Color(0.13, 0.15, 0.19),
                muted_text: Color(0.42, 0.46, 0.52),
                track: Color(0.89, 0.90, 0.92),
            }
        }
    }
}

// ── Time-series chart ─────────────────────────────────────────────────────────

/// Whether a series is drawn as a filled trend line or discrete bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChartKind {
    /// Continuous measurements such as FPS or frame render time.
    Area,
    /// Per-interval event counts such as frames dropped since the last poll.
    Bars,
}

/// One rendered series plus the axis behaviour around it.
pub(crate) struct ChartOptions {
    /// How to render the series.
    pub(crate) kind: ChartKind,
    /// Smallest allowed top-of-axis value, so a flat idle series does not get
    /// magnified into meaningless noise.
    pub(crate) baseline_max: f64,
    /// Formats the axis labels for a given value.
    pub(crate) format_axis: Box<dyn Fn(f64) -> String>,
    /// Classifies the newest value, which tints the whole series.
    pub(crate) severity: Box<dyn Fn(f64) -> Severity>,
}

impl ChartOptions {
    /// A chart whose series is always drawn in the "good" accent.
    pub(crate) fn new(
        kind: ChartKind,
        baseline_max: f64,
        format_axis: impl Fn(f64) -> String + 'static,
    ) -> Self {
        Self {
            kind,
            baseline_max,
            format_axis: Box::new(format_axis),
            severity: Box::new(|_| Severity::Good),
        }
    }

    /// Tint the series by classifying its newest value.
    pub(crate) fn with_severity(mut self, severity: impl Fn(f64) -> Severity + 'static) -> Self {
        self.severity = Box::new(severity);
        self
    }
}

/// A time-series chart bound to a value source.
pub(crate) struct Chart {
    widget: DrawingArea,
}

impl Chart {
    /// The drawable widget, ready to be added to a container.
    pub(crate) fn widget(&self) -> &DrawingArea {
        &self.widget
    }

    /// Re-read the source and repaint.
    pub(crate) fn redraw(&self) {
        self.widget.queue_draw();
    }
}

/// Build a chart that pulls its series (oldest to newest) from `source` on
/// every draw.
pub(crate) fn chart(options: ChartOptions, source: impl Fn() -> Vec<f64> + 'static) -> Chart {
    let widget = DrawingArea::builder()
        .content_height(CHART_HEIGHT)
        .content_width(CHART_MIN_WIDTH)
        .hexpand(true)
        .build();
    widget.add_css_class("scenedeck-chart");

    widget.set_draw_func(move |_, ctx, width, height| {
        draw_chart(
            ctx,
            f64::from(width),
            f64::from(height),
            &options,
            &source(),
        );
    });

    Chart { widget }
}

fn draw_chart(ctx: &Context, width: f64, height: f64, options: &ChartOptions, values: &[f64]) {
    let palette = Palette::for_current_theme();
    let plot = PlotArea::new(width, height);
    let upper = nice_upper_bound(series_max(values), options.baseline_max);

    draw_grid(ctx, &plot, &palette, upper, options);

    if values.is_empty() {
        return;
    }

    let accent = (options.severity)(values[values.len() - 1]).color();
    match options.kind {
        ChartKind::Area => draw_area_series(ctx, &plot, accent, values, upper),
        ChartKind::Bars => draw_bar_series(ctx, &plot, accent, values, upper),
    }
}

/// The inner rectangle a series is drawn into, leaving room for axis labels.
struct PlotArea {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

impl PlotArea {
    const LABEL_GUTTER: f64 = 46.0;
    const PADDING: f64 = 8.0;

    fn new(width: f64, height: f64) -> Self {
        Self {
            left: Self::LABEL_GUTTER,
            top: Self::PADDING,
            width: (width - Self::LABEL_GUTTER - Self::PADDING).max(1.0),
            height: (height - Self::PADDING * 2.0).max(1.0),
        }
    }

    fn bottom(&self) -> f64 {
        self.top + self.height
    }

    fn right(&self) -> f64 {
        self.left + self.width
    }

    fn y_for(&self, value: f64, upper: f64) -> f64 {
        self.bottom() - self.height * clamp_fraction(value, upper)
    }

    fn x_for(&self, index: usize, count: usize) -> f64 {
        if count <= 1 {
            return self.right();
        }
        self.left + self.width * (index as f64 / (count - 1) as f64)
    }
}

fn draw_grid(
    ctx: &Context,
    plot: &PlotArea,
    palette: &Palette,
    upper: f64,
    options: &ChartOptions,
) {
    ctx.set_line_width(1.0);
    ctx.select_font_face("Sans", FontSlant::Normal, FontWeight::Normal);
    ctx.set_font_size(10.0);

    for line in 0..=GRID_LINES {
        let fraction = line as f64 / GRID_LINES as f64;
        let value = upper * fraction;
        // Align to the pixel grid so horizontal rules stay crisp.
        let y = (plot.y_for(value, upper)).round() + 0.5;

        palette.grid.apply(ctx);
        ctx.move_to(plot.left, y);
        ctx.line_to(plot.right(), y);
        let _ = ctx.stroke();

        palette.axis_text.apply(ctx);
        let label = (options.format_axis)(value);
        let extents = ctx.text_extents(&label).map(|e| e.width()).unwrap_or(0.0);
        ctx.move_to(plot.left - 6.0 - extents, y + 3.0);
        let _ = ctx.show_text(&label);
    }
}

fn draw_area_series(ctx: &Context, plot: &PlotArea, accent: Color, values: &[f64], upper: f64) {
    let count = values.len();
    let points: Vec<(f64, f64)> = values
        .iter()
        .enumerate()
        .map(|(index, value)| (plot.x_for(index, count), plot.y_for(*value, upper)))
        .collect();

    // Fill under the curve first so the stroke sits on top of it.
    ctx.move_to(points[0].0, plot.bottom());
    for &(x, y) in &points {
        ctx.line_to(x, y);
    }
    ctx.line_to(points[points.len() - 1].0, plot.bottom());
    ctx.close_path();
    accent.apply_with_alpha(ctx, 0.16);
    let _ = ctx.fill();

    accent.apply(ctx);
    ctx.set_line_width(2.0);
    ctx.move_to(points[0].0, points[0].1);
    for &(x, y) in points.iter().skip(1) {
        ctx.line_to(x, y);
    }
    let _ = ctx.stroke();

    // Mark the newest sample so the current value is unambiguous.
    let (x, y) = points[points.len() - 1];
    ctx.arc(x, y, 3.0, 0.0, PI * 2.0);
    let _ = ctx.fill();
}

fn draw_bar_series(ctx: &Context, plot: &PlotArea, accent: Color, values: &[f64], upper: f64) {
    let count = values.len();
    let slot = plot.width / count.max(1) as f64;
    let bar_width = (slot * 0.7).clamp(1.0, 14.0);

    for (index, value) in values.iter().enumerate() {
        // A nonzero count must stay visible even when it rounds to sub-pixel
        // height — a dropped frame the user cannot see is worse than useless.
        let top = plot.y_for(*value, upper);
        let height = if *value > 0.0 {
            (plot.bottom() - top).max(2.0)
        } else {
            0.0
        };
        if height <= 0.0 {
            continue;
        }

        let x = plot.left + slot * index as f64 + (slot - bar_width) / 2.0;
        accent.apply_with_alpha(ctx, 0.85);
        ctx.rectangle(x, plot.bottom() - height, bar_width, height);
        let _ = ctx.fill();
    }
}

// ── Radial gauge ──────────────────────────────────────────────────────────────

/// A single gauge reading resolved at draw time.
pub(crate) struct GaugeReading {
    /// Sweep of the arc, clamped to `0.0..=1.0`.
    pub(crate) fraction: f64,
    /// Large text drawn inside the arc.
    pub(crate) value: String,
    /// Small text drawn under the value.
    pub(crate) caption: String,
    /// Selects the arc colour.
    pub(crate) severity: Severity,
}

/// A radial gauge bound to a reading source.
pub(crate) struct Gauge {
    widget: DrawingArea,
}

impl Gauge {
    /// The drawable widget, ready to be added to a container.
    pub(crate) fn widget(&self) -> &DrawingArea {
        &self.widget
    }

    /// Re-read the source and repaint.
    pub(crate) fn redraw(&self) {
        self.widget.queue_draw();
    }
}

/// Build a gauge that pulls its reading from `source` on every draw.
pub(crate) fn gauge(source: impl Fn() -> GaugeReading + 'static) -> Gauge {
    let widget = DrawingArea::builder()
        .content_width(GAUGE_SIZE)
        .content_height(GAUGE_SIZE)
        .build();
    widget.add_css_class("scenedeck-gauge");

    widget.set_draw_func(move |_, ctx, width, height| {
        draw_gauge(ctx, f64::from(width), f64::from(height), &source());
    });

    Gauge { widget }
}

fn draw_gauge(ctx: &Context, width: f64, height: f64, reading: &GaugeReading) {
    let palette = Palette::for_current_theme();
    let center_x = width / 2.0;
    let center_y = height / 2.0 + 8.0;
    let radius = (width.min(height) / 2.0 - 14.0).max(8.0);
    let thickness = (radius * 0.22).clamp(6.0, 14.0);

    // 240° arc opening downwards, the conventional gauge sweep.
    let start = PI * 0.75;
    let sweep = PI * 1.5;
    let fraction = reading.fraction.clamp(0.0, 1.0);

    ctx.set_line_width(thickness);
    ctx.set_line_cap(gtk4::cairo::LineCap::Round);

    palette.track.apply(ctx);
    ctx.arc(center_x, center_y, radius, start, start + sweep);
    let _ = ctx.stroke();

    if fraction > 0.0 {
        reading.severity.color().apply(ctx);
        ctx.arc(center_x, center_y, radius, start, start + sweep * fraction);
        let _ = ctx.stroke();
    }

    ctx.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    ctx.set_font_size(20.0);
    palette.value_text.apply(ctx);
    draw_centered_text(ctx, center_x, center_y + 2.0, &reading.value);

    ctx.select_font_face("Sans", FontSlant::Normal, FontWeight::Normal);
    ctx.set_font_size(11.0);
    palette.muted_text.apply(ctx);
    draw_centered_text(ctx, center_x, center_y + 20.0, &reading.caption);
}

fn draw_centered_text(ctx: &Context, center_x: f64, y: f64, text: &str) {
    let width = ctx.text_extents(text).map(|e| e.width()).unwrap_or(0.0);
    ctx.move_to(center_x - width / 2.0, y);
    let _ = ctx.show_text(text);
}

// ── Scaling helpers (pure, unit tested) ───────────────────────────────────────

/// Largest value in the series, or zero for an empty one. NaN samples are
/// ignored rather than poisoning the axis.
pub(crate) fn series_max(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(0.0, f64::max)
}

/// Round `max` up to a readable axis top — 1, 2 or 5 times a power of ten —
/// never returning less than `baseline`.
///
/// The baseline keeps a chart stable: an FPS chart pinned to at least 60 does
/// not redraw its whole axis because the render dipped by one frame.
pub(crate) fn nice_upper_bound(max: f64, baseline: f64) -> f64 {
    let baseline = if baseline.is_finite() && baseline > 0.0 {
        baseline
    } else {
        1.0
    };
    if !max.is_finite() || max <= baseline {
        return baseline;
    }

    let magnitude = 10f64.powf(max.log10().floor());
    let normalized = max / magnitude;
    let step = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };

    (step * magnitude).max(baseline)
}

/// `value / max` clamped to `0.0..=1.0`, with a zero or invalid `max` reading
/// as an empty gauge rather than a division blow-up.
pub(crate) fn clamp_fraction(value: f64, max: f64) -> f64 {
    if !value.is_finite() || !max.is_finite() || max <= 0.0 {
        return 0.0;
    }
    (value / max).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_max_ignores_empty_and_non_finite_samples() {
        assert_eq!(series_max(&[]), 0.0);
        assert_eq!(series_max(&[1.0, f64::NAN, 4.0]), 4.0);
        assert_eq!(series_max(&[f64::INFINITY]), 0.0);
    }

    #[test]
    fn upper_bound_never_drops_below_the_baseline() {
        assert_eq!(nice_upper_bound(0.0, 60.0), 60.0);
        assert_eq!(nice_upper_bound(42.0, 60.0), 60.0);
    }

    #[test]
    fn upper_bound_rounds_up_to_one_two_or_five_times_a_power_of_ten() {
        assert_eq!(nice_upper_bound(7.0, 1.0), 10.0);
        assert_eq!(nice_upper_bound(12.0, 1.0), 20.0);
        assert_eq!(nice_upper_bound(31.0, 1.0), 50.0);
        assert_eq!(nice_upper_bound(64.0, 1.0), 100.0);
        assert_eq!(nice_upper_bound(6_200.0, 1.0), 10_000.0);
    }

    #[test]
    fn upper_bound_falls_back_to_one_for_an_unusable_baseline() {
        assert_eq!(nice_upper_bound(0.0, 0.0), 1.0);
        assert_eq!(nice_upper_bound(f64::NAN, -5.0), 1.0);
    }

    #[test]
    fn fractions_are_clamped_into_the_drawable_range() {
        assert_eq!(clamp_fraction(30.0, 60.0), 0.5);
        assert_eq!(clamp_fraction(90.0, 60.0), 1.0);
        assert_eq!(clamp_fraction(-5.0, 60.0), 0.0);
    }

    #[test]
    fn fractions_degrade_to_zero_rather_than_dividing_by_zero() {
        assert_eq!(clamp_fraction(5.0, 0.0), 0.0);
        assert_eq!(clamp_fraction(f64::NAN, 60.0), 0.0);
    }

    #[test]
    fn severity_escalates_at_each_threshold() {
        assert_eq!(Severity::from_thresholds(0.1, 0.3, 0.6), Severity::Good);
        assert_eq!(Severity::from_thresholds(0.3, 0.3, 0.6), Severity::Warning);
        assert_eq!(Severity::from_thresholds(0.9, 0.3, 0.6), Severity::Critical);
    }

    #[test]
    fn plot_area_maps_values_and_indices_onto_the_canvas() {
        let plot = PlotArea::new(200.0, 100.0);

        assert_eq!(plot.y_for(0.0, 60.0), plot.bottom());
        assert_eq!(plot.y_for(60.0, 60.0), plot.top);
        assert_eq!(plot.y_for(30.0, 60.0), plot.top + plot.height / 2.0);

        assert_eq!(plot.x_for(0, 5), plot.left);
        assert_eq!(plot.x_for(4, 5), plot.right());
        // A single sample is pinned to the newest edge of the chart.
        assert_eq!(plot.x_for(0, 1), plot.right());
    }
}
