//! Stats page — live streaming telemetry.
//!
//! Renders the samples accumulated in `AppState::stats_history` as gauges,
//! time-series charts, and counter cards. The page owns no polling of its own:
//! the OBS session fills the history whether or not this page is visible, so
//! opening it shows the preceding minutes rather than an empty graph.
//!
//! Widgets are built once and refreshed in place. At a one-second sample rate a
//! rebuild-per-update would churn the whole widget tree for numbers that mostly
//! stay the same shape.

use std::rc::Rc;

use adw::prelude::*;
use gtk4::prelude::IsA;
use gtk4::{
    Align, Box as GtkBox, FlowBox, FlowBoxChild, Label, Orientation, PolicyType, ScrolledWindow,
    SelectionMode,
};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::controller::state::AppState;
use crate::domain::stats::{StatsMetric, StatsSample};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::chart::{
    self, Chart, ChartKind, ChartOptions, Gauge, GaugeReading, Severity,
};

/// Axis floor for the FPS chart and gauge. Most OBS profiles run at 30 or 60,
/// and pinning the scale keeps the trend line from rescaling on every dip.
const FPS_BASELINE: f64 = 60.0;
/// Axis floor for the frame-render-time chart, in milliseconds. A 60 FPS frame
/// budget is 16.7 ms, so this shows the whole budget by default.
const FRAME_TIME_BASELINE_MS: f64 = 20.0;
/// Axis floor for the per-interval dropped-frame charts. Small counts still
/// need a readable scale.
const DROPPED_BASELINE: f64 = 5.0;

/// FPS is healthy above 95% of the observed target and bad below 80%.
const FPS_WARNING_RATIO: f64 = 0.95;
const FPS_CRITICAL_RATIO: f64 = 0.80;
/// Frame render time is a warning past half the frame budget and critical past
/// 80% of it — beyond that OBS starts missing frames outright.
const FRAME_TIME_WARNING_RATIO: f64 = 0.50;
const FRAME_TIME_CRITICAL_RATIO: f64 = 0.80;
/// Dropped-frame ratios, matching the thresholds streamers usually act on.
const DROPPED_WARNING_RATIO: f64 = 0.01;
const DROPPED_CRITICAL_RATIO: f64 = 0.05;
/// Network congestion reported by OBS.
const CONGESTION_WARNING: f64 = 0.30;
const CONGESTION_CRITICAL: f64 = 0.60;

/// Widest layout for each section. Every section reflows to fewer columns as
/// the window narrows — a `FlowBox` breaks lines from the children's own
/// minimum widths, so the page adapts continuously instead of snapping at one
/// hand-picked breakpoint.
const GAUGE_COLUMNS: u32 = 4;
const CHART_COLUMNS: u32 = 2;
const CARD_COLUMNS: u32 = 2;

/// Value labels that are refreshed in place on every sample.
#[derive(Clone)]
struct StatsCards {
    render_frames: Label,
    output_frames: Label,
    stream_frames: Label,
    frame_time: Label,
    cpu: Label,
    memory: Label,
    bitrate: Label,
}

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .build();
    page.add_css_class("app-page");
    page.add_css_class("stats-page");

    page.append(&build_header());

    let gauges = build_gauges(&nav);
    page.append(&gauges.row);

    let charts = build_charts(&nav);
    page.append(&charts.grid);

    let (cards_widget, cards) = build_cards();
    page.append(&cards_widget);

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&page)
        .build();

    let refresh: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        move || {
            for gauge in &gauges.all {
                gauge.redraw();
            }
            for chart in &charts.all {
                chart.redraw();
            }
            update_cards(&cards, &nav.state.borrow());
        }
    });

    // Ask for a sample immediately on open so the page never shows values that
    // are up to one poll interval stale.
    scroll.connect_map({
        let nav = nav.clone();
        let refresh = refresh.clone();
        move |_| {
            nav.dispatch(AppCommand::RefreshStats);
            refresh();
        }
    });

    (scroll.upcast(), refresh)
}

/// A section that lays its children out in up to `columns` equal columns and
/// wraps to fewer as the available width shrinks, down to a single column.
fn responsive_section(columns: u32) -> FlowBox {
    FlowBox::builder()
        .selection_mode(SelectionMode::None)
        .column_spacing(12)
        .row_spacing(12)
        .homogeneous(true)
        .hexpand(true)
        .valign(Align::Start)
        .min_children_per_line(1)
        .max_children_per_line(columns)
        .build()
}

/// Add `widget` to a responsive section, stretching it across its column so a
/// reflowed single-column layout uses the full width instead of hugging left.
fn add_to_section<W: IsA<gtk4::Widget>>(section: &FlowBox, widget: &W) {
    let child = FlowBoxChild::new();
    child.set_halign(Align::Fill);
    child.set_valign(Align::Start);
    child.set_hexpand(true);
    child.set_child(Some(widget));
    section.insert(&child, -1);
}

fn build_header() -> GtkBox {
    let header = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();

    let title = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "stats-page-title"))
        .xalign(0.0)
        .build();
    title.add_css_class("title-2");

    let subtitle = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "stats-page-subtitle"))
        .xalign(0.0)
        .wrap(true)
        .build();
    subtitle.add_css_class("dim-label");

    header.append(&title);
    header.append(&subtitle);
    header
}

// ── Gauges ────────────────────────────────────────────────────────────────────

struct GaugeRow {
    row: FlowBox,
    all: Vec<Gauge>,
}

fn build_gauges(nav: &NavigationContext) -> GaugeRow {
    let row = responsive_section(GAUGE_COLUMNS);
    row.add_css_class("stats-gauge-row");

    let fps = chart::gauge({
        let nav = nav.clone();
        move || {
            let state = nav.state.borrow();
            let target = fps_target(&state);
            read_sample(&state).map_or_else(
                || empty_reading(fl!(LANGUAGE_LOADER, "stats-gauge-fps")),
                |sample| GaugeReading {
                    fraction: chart::clamp_fraction(sample.stats.active_fps, target),
                    value: format!("{:.0}", sample.stats.active_fps),
                    caption: fl!(LANGUAGE_LOADER, "stats-gauge-fps"),
                    severity: fps_severity(sample.stats.active_fps, target),
                },
            )
        }
    });

    let frame_time = chart::gauge({
        let nav = nav.clone();
        move || {
            let state = nav.state.borrow();
            let budget = frame_budget_ms(fps_target(&state));
            read_sample(&state).map_or_else(
                || empty_reading(fl!(LANGUAGE_LOADER, "stats-gauge-frame-time")),
                |sample| {
                    let value = sample.stats.average_frame_render_time_ms;
                    GaugeReading {
                        fraction: chart::clamp_fraction(value, budget),
                        value: format!("{value:.1}"),
                        caption: fl!(LANGUAGE_LOADER, "stats-gauge-frame-time"),
                        severity: frame_time_severity(value, budget),
                    }
                },
            )
        }
    });

    let dropped = chart::gauge({
        let nav = nav.clone();
        move || {
            let state = nav.state.borrow();
            read_sample(&state).map_or_else(
                || empty_reading(fl!(LANGUAGE_LOADER, "stats-gauge-dropped")),
                |sample| {
                    // Prefer the stream output's own ratio; fall back to the
                    // process-wide encoder counter before the stream reports.
                    let ratio = sample.stream.map_or_else(
                        || sample.stats.output_skipped_ratio(),
                        |s| s.dropped_ratio(),
                    );
                    GaugeReading {
                        // Scaled against the critical threshold so a healthy
                        // stream is a small arc rather than an invisible one.
                        fraction: chart::clamp_fraction(ratio, DROPPED_CRITICAL_RATIO),
                        value: format!("{:.2}%", ratio * 100.0),
                        caption: fl!(LANGUAGE_LOADER, "stats-gauge-dropped"),
                        severity: dropped_severity(ratio),
                    }
                },
            )
        }
    });

    let congestion = chart::gauge({
        let nav = nav.clone();
        move || {
            let state = nav.state.borrow();
            read_sample(&state)
                .and_then(|sample| sample.stream)
                .map_or_else(
                    || empty_reading(fl!(LANGUAGE_LOADER, "stats-gauge-congestion")),
                    |stream| GaugeReading {
                        fraction: stream.congestion,
                        value: format!("{:.0}%", stream.congestion * 100.0),
                        caption: fl!(LANGUAGE_LOADER, "stats-gauge-congestion"),
                        severity: Severity::from_thresholds(
                            stream.congestion,
                            CONGESTION_WARNING,
                            CONGESTION_CRITICAL,
                        ),
                    },
                )
        }
    });

    let all = vec![fps, frame_time, dropped, congestion];
    for gauge in &all {
        add_to_section(&row, gauge.widget());
    }

    GaugeRow { row, all }
}

fn empty_reading(caption: String) -> GaugeReading {
    GaugeReading {
        fraction: 0.0,
        value: fl!(LANGUAGE_LOADER, "stats-value-placeholder"),
        caption,
        severity: Severity::Good,
    }
}

// ── Charts ────────────────────────────────────────────────────────────────────

struct ChartGrid {
    grid: FlowBox,
    all: Vec<Chart>,
}

fn build_charts(nav: &NavigationContext) -> ChartGrid {
    let fps = series_chart(
        nav,
        StatsMetric::Fps,
        fl!(LANGUAGE_LOADER, "stats-chart-fps"),
        ChartOptions::new(ChartKind::Area, FPS_BASELINE, |value| format!("{value:.0}"))
            // Classified against the same observed target as the gauge, so a 30 FPS
            // profile is not permanently drawn as failing.
            .with_severity({
                let nav = nav.clone();
                move |value| fps_severity(value, fps_target(&nav.state.borrow()))
            }),
    );

    let frame_time = series_chart(
        nav,
        StatsMetric::FrameRenderTimeMs,
        fl!(LANGUAGE_LOADER, "stats-chart-frame-time"),
        ChartOptions::new(ChartKind::Area, FRAME_TIME_BASELINE_MS, |value| {
            format!("{value:.0}")
        })
        .with_severity({
            let nav = nav.clone();
            move |value| {
                frame_time_severity(value, frame_budget_ms(fps_target(&nav.state.borrow())))
            }
        }),
    );

    let output_skipped = series_chart(
        nav,
        StatsMetric::OutputSkippedPerInterval,
        fl!(LANGUAGE_LOADER, "stats-chart-output-skipped"),
        ChartOptions::new(ChartKind::Bars, DROPPED_BASELINE, |value| {
            format!("{value:.0}")
        })
        .with_severity(|value| {
            if value > 0.0 {
                Severity::Critical
            } else {
                Severity::Good
            }
        }),
    );

    let render_skipped = series_chart(
        nav,
        StatsMetric::RenderSkippedPerInterval,
        fl!(LANGUAGE_LOADER, "stats-chart-render-skipped"),
        ChartOptions::new(ChartKind::Bars, DROPPED_BASELINE, |value| {
            format!("{value:.0}")
        })
        .with_severity(|value| {
            if value > 0.0 {
                Severity::Warning
            } else {
                Severity::Good
            }
        }),
    );

    let grid = responsive_section(CHART_COLUMNS);
    grid.add_css_class("stats-chart-grid");
    for titled in [&fps, &frame_time, &output_skipped, &render_skipped] {
        add_to_section(&grid, &titled.frame);
    }

    ChartGrid {
        grid,
        all: vec![
            fps.chart,
            frame_time.chart,
            output_skipped.chart,
            render_skipped.chart,
        ],
    }
}

struct TitledChart {
    frame: GtkBox,
    chart: Chart,
}

fn series_chart(
    nav: &NavigationContext,
    metric: StatsMetric,
    title: String,
    options: ChartOptions,
) -> TitledChart {
    let chart = chart::chart(options, {
        let nav = nav.clone();
        move || nav.state.borrow().stats_history.series(metric)
    });

    let frame = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();
    frame.add_css_class("stats-chart-frame");

    let label = Label::builder().label(title).xalign(0.0).build();
    label.add_css_class("stats-chart-title");

    frame.append(&label);
    frame.append(chart.widget());

    TitledChart { frame, chart }
}

// ── Counter cards ─────────────────────────────────────────────────────────────

fn build_cards() -> (FlowBox, StatsCards) {
    // Cards reflow individually rather than as two fixed columns, so a narrow
    // window gets one readable column instead of two cramped ones.
    let grid = responsive_section(CARD_COLUMNS);
    grid.add_css_class("stats-card-row");

    let render_frames = card(&grid, fl!(LANGUAGE_LOADER, "stats-card-render-frames"));
    let output_frames = card(&grid, fl!(LANGUAGE_LOADER, "stats-card-output-frames"));
    let stream_frames = card(&grid, fl!(LANGUAGE_LOADER, "stats-card-stream-frames"));
    let frame_time = card(&grid, fl!(LANGUAGE_LOADER, "stats-card-frame-time"));
    let cpu = card(&grid, fl!(LANGUAGE_LOADER, "stats-card-cpu"));
    let memory = card(&grid, fl!(LANGUAGE_LOADER, "stats-card-memory"));
    let bitrate = card(&grid, fl!(LANGUAGE_LOADER, "stats-card-bitrate"));

    (
        grid,
        StatsCards {
            render_frames,
            output_frames,
            stream_frames,
            frame_time,
            cpu,
            memory,
            bitrate,
        },
    )
}

/// Append one caption/value card to `parent` and return its value label.
fn card(parent: &FlowBox, caption: String) -> Label {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .hexpand(true)
        .build();
    row.add_css_class("stats-card");

    let caption_label = Label::builder()
        .label(caption)
        .xalign(0.0)
        .hexpand(true)
        .wrap(true)
        .build();
    caption_label.add_css_class("stats-card-caption");

    let value = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "stats-value-placeholder"))
        .xalign(1.0)
        .halign(Align::End)
        .build();
    value.add_css_class("stats-card-value");

    row.append(&caption_label);
    row.append(&value);
    add_to_section(parent, &row);

    value
}

fn update_cards(cards: &StatsCards, state: &AppState) {
    let Some(sample) = read_sample(state) else {
        for label in [
            &cards.render_frames,
            &cards.output_frames,
            &cards.stream_frames,
            &cards.frame_time,
            &cards.cpu,
            &cards.memory,
            &cards.bitrate,
        ] {
            label.set_text(&fl!(LANGUAGE_LOADER, "stats-value-placeholder"));
        }
        return;
    };

    cards.render_frames.set_text(&format_frames(
        sample.stats.render_skipped_frames,
        sample.stats.render_total_frames,
        sample.stats.render_skipped_ratio(),
    ));
    cards.output_frames.set_text(&format_frames(
        sample.stats.output_skipped_frames,
        sample.stats.output_total_frames,
        sample.stats.output_skipped_ratio(),
    ));
    cards.stream_frames.set_text(&match sample.stream {
        Some(stream) => format_frames(
            stream.skipped_frames,
            stream.total_frames,
            stream.dropped_ratio(),
        ),
        None => fl!(LANGUAGE_LOADER, "stats-value-placeholder"),
    });
    cards.frame_time.set_text(&fl!(
        LANGUAGE_LOADER,
        "stats-value-ms",
        value = format!("{:.2}", sample.stats.average_frame_render_time_ms)
    ));
    cards.cpu.set_text(&fl!(
        LANGUAGE_LOADER,
        "stats-value-percent",
        value = format!("{:.1}", sample.stats.cpu_usage_percent)
    ));
    cards.memory.set_text(&fl!(
        LANGUAGE_LOADER,
        "stats-value-mb",
        value = format!("{:.0}", sample.stats.memory_usage_mb)
    ));
    cards.bitrate.set_text(&match sample.bitrate_kbps {
        Some(kbps) => fl!(
            LANGUAGE_LOADER,
            "stats-value-kbps",
            value = format!("{kbps:.0}")
        ),
        None => fl!(LANGUAGE_LOADER, "stats-value-placeholder"),
    });
}

// ── Presentation helpers (pure, unit tested) ──────────────────────────────────

fn read_sample(state: &AppState) -> Option<StatsSample> {
    state.stats_history.latest().copied()
}

/// Scale for the FPS gauge and its severity. OBS does not report the profile's
/// configured FPS, so the highest rate seen this session is used as the target,
/// floored at [`FPS_BASELINE`] so a fresh history is not scaled off one sample.
fn fps_target(state: &AppState) -> f64 {
    chart::nice_upper_bound(
        chart::series_max(&state.stats_history.series(StatsMetric::Fps)),
        FPS_BASELINE,
    )
}

/// Milliseconds available to render one frame at `target` FPS.
fn frame_budget_ms(target: f64) -> f64 {
    if target <= 0.0 {
        return FRAME_TIME_BASELINE_MS;
    }
    1_000.0 / target
}

/// FPS is inverted relative to the other metrics: falling behind is the failure.
fn fps_severity(fps: f64, target: f64) -> Severity {
    if target <= 0.0 {
        return Severity::Good;
    }
    let ratio = fps / target;
    if ratio < FPS_CRITICAL_RATIO {
        Severity::Critical
    } else if ratio < FPS_WARNING_RATIO {
        Severity::Warning
    } else {
        Severity::Good
    }
}

fn frame_time_severity(value_ms: f64, budget_ms: f64) -> Severity {
    if budget_ms <= 0.0 {
        return Severity::Good;
    }
    Severity::from_thresholds(
        value_ms / budget_ms,
        FRAME_TIME_WARNING_RATIO,
        FRAME_TIME_CRITICAL_RATIO,
    )
}

fn dropped_severity(ratio: f64) -> Severity {
    Severity::from_thresholds(ratio, DROPPED_WARNING_RATIO, DROPPED_CRITICAL_RATIO)
}

/// Counters are pre-formatted as strings so the rendered digits are identical
/// in every locale; Fluent would otherwise be free to group them differently.
fn format_frames(skipped: u32, total: u32, ratio: f64) -> String {
    fl!(
        LANGUAGE_LOADER,
        "stats-value-frames",
        skipped = skipped.to_string(),
        total = total.to_string(),
        percent = format!("{:.2}", ratio * 100.0)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_budget_follows_the_target_frame_rate() {
        assert!((frame_budget_ms(60.0) - 16.666).abs() < 0.01);
        assert!((frame_budget_ms(30.0) - 33.333).abs() < 0.01);
    }

    #[test]
    fn frame_budget_falls_back_when_the_target_is_unusable() {
        assert_eq!(frame_budget_ms(0.0), FRAME_TIME_BASELINE_MS);
        assert_eq!(frame_budget_ms(-30.0), FRAME_TIME_BASELINE_MS);
    }

    #[test]
    fn fps_severity_escalates_as_the_render_falls_behind_target() {
        assert_eq!(fps_severity(60.0, 60.0), Severity::Good);
        assert_eq!(fps_severity(58.0, 60.0), Severity::Good);
        assert_eq!(fps_severity(55.0, 60.0), Severity::Warning);
        assert_eq!(fps_severity(40.0, 60.0), Severity::Critical);
    }

    #[test]
    fn fps_severity_is_neutral_without_a_usable_target() {
        assert_eq!(fps_severity(0.0, 0.0), Severity::Good);
    }

    #[test]
    fn frame_time_severity_is_measured_against_the_frame_budget() {
        let budget = frame_budget_ms(60.0);

        assert_eq!(frame_time_severity(4.0, budget), Severity::Good);
        assert_eq!(frame_time_severity(10.0, budget), Severity::Warning);
        assert_eq!(frame_time_severity(15.0, budget), Severity::Critical);
    }

    #[test]
    fn dropped_severity_uses_the_one_and_five_percent_thresholds() {
        assert_eq!(dropped_severity(0.0), Severity::Good);
        assert_eq!(dropped_severity(0.005), Severity::Good);
        assert_eq!(dropped_severity(0.02), Severity::Warning);
        assert_eq!(dropped_severity(0.08), Severity::Critical);
    }

    #[test]
    fn frames_are_formatted_with_their_share_of_the_total() {
        assert_eq!(format_frames(5, 1_000, 0.005), "5 of 1000 (0.50%)");
    }
}
