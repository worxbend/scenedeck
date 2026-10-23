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

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk4::prelude::IsA;
use gtk4::{
    Align, Box as GtkBox, Button, FlowBox, FlowBoxChild, Label, Orientation, PolicyType,
    ScrolledWindow, SelectionMode,
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
    disk: Label,
}

/// Slot the embedded page's refresh callback shares with its pop-out window.
///
/// `None` while no pop-out is open. The application event pump owns this slot
/// independently of shell page handles; `PageRefreshers` calls it while the shell
/// exists, and the pump calls it directly after shell removal.
pub(crate) type PopoutSlot = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>, PopoutSlot) {
    let popout_slot: PopoutSlot = Rc::new(RefCell::new(None));
    let popout_window: Rc<RefCell<Option<adw::Window>>> = Rc::new(RefCell::new(None));

    let popout_button = build_popout_button(&nav, &popout_window, &popout_slot);
    let (page, refresh) = build_content(&nav, Some(popout_button.upcast_ref::<gtk4::Widget>()));

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&page)
        .build();

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

    (scroll.upcast(), refresh, popout_slot)
}

/// Build the gauges/charts/cards content shared by the embedded page and its
/// pop-out window. `header_trailing` is the pop-out button on the embedded
/// page, and `None` on the pop-out window itself (there is nothing to pop out
/// of a window that is already detached).
fn build_content(
    nav: &NavigationContext,
    header_trailing: Option<&gtk4::Widget>,
) -> (GtkBox, Rc<dyn Fn()>) {
    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .build();
    page.add_css_class("app-page");
    page.add_css_class("stats-page");

    page.append(&build_header(header_trailing));

    let gauges = build_gauges(nav);
    page.append(&gauges.row);

    let charts = build_charts(nav);
    page.append(&charts.grid);

    let (cards_widget, cards) = build_cards();
    page.append(&cards_widget);
    let (activity, refresh_activity) = build_activity(nav, charts.all.clone());
    page.append(&activity);

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
            refresh_activity();
        }
    });

    (page, refresh)
}

/// Build the pop-out toggle button. Clicking it opens a detached window with
/// its own copy of the stats content; clicking again while it is open just
/// presents the existing window instead of opening a second one.
fn build_popout_button(
    nav: &NavigationContext,
    popout_window: &Rc<RefCell<Option<adw::Window>>>,
    popout_slot: &PopoutSlot,
) -> Button {
    let button = Button::builder()
        .icon_name("window-new-symbolic")
        .valign(Align::Center)
        .tooltip_text(fl!(LANGUAGE_LOADER, "stats-popout-button"))
        .build();
    button.add_css_class("flat");

    button.connect_clicked({
        let nav = nav.clone();
        let popout_window = popout_window.clone();
        let popout_slot = popout_slot.clone();
        move |button| {
            if let Some(window) = popout_window.borrow().as_ref() {
                window.present();
                return;
            }
            open_popout(button, &nav, &popout_window, &popout_slot);
        }
    });

    button
}

/// Open the detached stats window and register its refresh callback so
/// `PageRefreshers` keeps it live.
fn open_popout(
    button: &Button,
    nav: &NavigationContext,
    popout_window: &Rc<RefCell<Option<adw::Window>>>,
    popout_slot: &PopoutSlot,
) {
    let (content, refresh) = build_content(nav, None);

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&content)
        .build();

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroll));

    let window = adw::Window::builder()
        .title(fl!(LANGUAGE_LOADER, "stats-page-title"))
        .default_width(640)
        .default_height(720)
        .content(&toolbar)
        .build();

    // Best-effort: keep the pop-out under the same application so it shares
    // the app's lifecycle. Nothing relies on this if it is ever unavailable.
    if let Some(app) = button
        .root()
        .and_downcast::<gtk4::Window>()
        .and_then(|w| w.application())
    {
        window.set_application(Some(&app));
        window.connect_notify_local(Some("application"), {
            let popout_window = Rc::downgrade(popout_window);
            let popout_slot = Rc::downgrade(popout_slot);
            move |window, _| {
                if window.application().is_none() {
                    if let Some(slot) = popout_window.upgrade() {
                        slot.borrow_mut().take();
                    }
                    if let Some(slot) = popout_slot.upgrade() {
                        slot.borrow_mut().take();
                    }
                }
            }
        });
    }

    window.connect_close_request({
        let popout_window = Rc::downgrade(popout_window);
        let popout_slot = Rc::downgrade(popout_slot);
        move |_| {
            if let Some(popout_window) = popout_window.upgrade() {
                *popout_window.borrow_mut() = None;
            }
            if let Some(popout_slot) = popout_slot.upgrade() {
                *popout_slot.borrow_mut() = None;
            }
            glib::Propagation::Proceed
        }
    });

    nav.dispatch(AppCommand::RefreshStats);
    refresh();

    *popout_slot.borrow_mut() = Some(refresh);
    window.present();
    *popout_window.borrow_mut() = Some(window);
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

fn build_header(trailing: Option<&gtk4::Widget>) -> GtkBox {
    let titles = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
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

    titles.append(&title);
    titles.append(&subtitle);

    let Some(trailing) = trailing else {
        return titles;
    };

    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    header.append(&titles);
    header.append(trailing);
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
                    fraction: chart::clamp_fraction(
                        sample.stats.active_fps,
                        target.unwrap_or(FPS_BASELINE),
                    ),
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
                        fraction: chart::clamp_fraction(
                            value,
                            budget.unwrap_or(FRAME_TIME_BASELINE_MS),
                        ),
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
        severity: Severity::Unknown,
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
            // Chart scaling must not become a health target.
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

    chart
        .widget()
        .update_property(&[gtk4::accessible::Property::Label(&title)]);
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
    let disk = card(&grid, fl!(LANGUAGE_LOADER, "stats-recording-disk"));

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
            disk,
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
            &cards.disk,
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
    cards.disk.set_text(&match sample
        .stats
        .disk_space_mb
        .filter(|value| value.is_finite() && *value >= 0.0)
    {
        Some(value) => fl!(
            LANGUAGE_LOADER,
            "stats-recording-disk-value",
            value = format!("{value:.0}")
        ),
        None => fl!(LANGUAGE_LOADER, "stats-value-placeholder"),
    });
    cards.bitrate.set_text(&match sample.bitrate_kbps {
        Some(kbps) => fl!(
            LANGUAGE_LOADER,
            "stats-value-kbps",
            value = format!("{kbps:.0}")
        ),
        None => fl!(LANGUAGE_LOADER, "stats-value-placeholder"),
    });
}

/// Activity is bounded independently of the chart window. A retained activity
/// entry can legitimately have no corresponding statistics sample.
fn build_activity(nav: &NavigationContext, charts: Vec<Chart>) -> (GtkBox, Rc<dyn Fn()>) {
    use crate::domain::workflow::{ActivityOrigin, ActivityRecord};
    let section = GtkBox::new(Orientation::Vertical, 8);
    let heading = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "stats-activity-title"))
        .xalign(0.0)
        .build();
    heading.add_css_class("heading");
    section.append(&heading);
    let detail = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "stats-activity-detail"))
        .xalign(0.0)
        .wrap(true)
        .build();
    section.append(&detail);
    let selected = Rc::new(Cell::new(None::<u64>));
    let marker_status = Label::builder().xalign(0.0).wrap(true).build();
    section.append(&marker_status);
    let clear = Button::with_label(&fl!(LANGUAGE_LOADER, "stats-activity-clear-marker"));
    clear.set_halign(Align::Start);
    section.append(&clear);
    {
        let selected = selected.clone();
        let charts = charts.clone();
        let status = marker_status.clone();
        clear.connect_clicked(move |_| {
            selected.set(None);
            for chart in &charts {
                chart.set_marker(None);
            }
            status.set_label("");
        });
    }
    let entries = GtkBox::new(Orientation::Vertical, 4);
    section.append(&entries);
    let cached = RefCell::new(None::<Vec<ActivityRecord>>);
    let nav = nav.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        let state = nav.state.borrow();
        let fraction = selected
            .get()
            .and_then(|timestamp| state.stats_history.event_fraction(timestamp));
        for chart in &charts {
            chart.set_marker(fraction);
        }
        if selected.get().is_some() {
            marker_status.set_label(&if fraction.is_some() {
                fl!(LANGUAGE_LOADER, "stats-activity-marker-visible")
            } else {
                fl!(LANGUAGE_LOADER, "stats-activity-marker-unavailable")
            });
        }
        let records: Vec<_> = state
            .workspace
            .activity
            .entries
            .iter()
            .rev()
            .take(15)
            .cloned()
            .collect();
        if cached.borrow().as_ref() == Some(&records) {
            return;
        }
        crate::ui::clear_children(&entries);
        if records.is_empty() {
            entries.append(
                &Label::builder()
                    .label(fl!(LANGUAGE_LOADER, "stats-activity-empty"))
                    .xalign(0.0)
                    .build(),
            );
        }
        for record in &records {
            let origin = match record.origin {
                ActivityOrigin::Requested => fl!(LANGUAGE_LOADER, "stats-activity-requested"),
                ActivityOrigin::Observed => fl!(LANGUAGE_LOADER, "stats-activity-observed"),
                ActivityOrigin::Failed => fl!(LANGUAGE_LOADER, "stats-activity-failed"),
                ActivityOrigin::Connection => fl!(LANGUAGE_LOADER, "stats-activity-connection"),
            };
            let time = i64::try_from(record.timestamp_unix_ms / 1000)
                .ok()
                .and_then(|seconds| glib::DateTime::from_unix_local(seconds).ok())
                .and_then(|date| date.format("%H:%M:%S").ok())
                .map(|time| time.to_string())
                .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "stats-value-placeholder"));
            let text = fl!(
                LANGUAGE_LOADER,
                "stats-activity-entry",
                time = time,
                origin = origin,
                description = record.description.clone()
            );
            let button = Button::new();
            button.set_child(Some(
                &Label::builder().label(text).xalign(0.0).wrap(true).build(),
            ));
            button.add_css_class("flat");
            let nav = nav.clone();
            let selected = selected.clone();
            let charts = charts.clone();
            let status = marker_status.clone();
            let timestamp = record.timestamp_unix_ms;
            button.connect_clicked(move |_| {
                selected.set(Some(timestamp));
                let fraction = nav.state.borrow().stats_history.event_fraction(timestamp);
                for chart in &charts {
                    chart.set_marker(fraction);
                }
                status.set_label(&if fraction.is_some() {
                    fl!(LANGUAGE_LOADER, "stats-activity-marker-visible")
                } else {
                    fl!(LANGUAGE_LOADER, "stats-activity-marker-unavailable")
                });
            });
            entries.append(&button);
        }
        *cached.borrow_mut() = Some(records);
    });
    (section, refresh)
}

// ── Presentation helpers (pure, unit tested) ──────────────────────────────────

fn read_sample(state: &AppState) -> Option<StatsSample> {
    state.stats_history.latest().copied()
}

/// Only OBS's configured video rate defines health; the chart ceiling is visual.
fn fps_target(state: &AppState) -> Option<f64> {
    read_sample(state)
        .and_then(|sample| sample.stats.configured_fps)
        .filter(|target| target.is_finite() && *target > 0.0)
}

/// Milliseconds available to render one frame at `target` FPS.
fn frame_budget_ms(target: Option<f64>) -> Option<f64> {
    target
        .filter(|target| target.is_finite() && *target > 0.0)
        .map(|target| 1_000.0 / target)
}

/// FPS is inverted relative to the other metrics: falling behind is the failure.
fn fps_severity(fps: f64, target: Option<f64>) -> Severity {
    let Some(target) = target.filter(|target| target.is_finite() && *target > 0.0) else {
        return Severity::Unknown;
    };
    if !fps.is_finite() {
        return Severity::Unknown;
    }
    if fps < target * FPS_CRITICAL_RATIO {
        Severity::Critical
    } else if fps < target * FPS_WARNING_RATIO {
        Severity::Warning
    } else {
        Severity::Good
    }
}

fn frame_time_severity(value_ms: f64, budget_ms: Option<f64>) -> Severity {
    let Some(budget_ms) = budget_ms.filter(|budget| budget.is_finite() && *budget > 0.0) else {
        return Severity::Unknown;
    };
    if !value_ms.is_finite() {
        return Severity::Unknown;
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
    #[ignore = "requires a native GTK display"]
    fn native_popout_window_does_not_own_its_strong_window_slot() {
        crate::ui::test_support::native_test(
            "ui::pages::stats::tests::native_popout_window_does_not_own_its_strong_window_slot",
            || {
                let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
                let weak_state = Rc::downgrade(&nav.state);
                nav.state.borrow_mut().workspace.log(
                    crate::domain::workflow::ActivityOrigin::Observed,
                    "Stats lifetime fixture".into(),
                );
                let window_slot = Rc::new(RefCell::new(None));
                let refresh_slot = Rc::new(RefCell::new(None));
                let button = build_popout_button(&nav, &window_slot, &refresh_slot);
                for _ in 0..3 {
                    button.emit_clicked();
                    let window = window_slot.borrow().as_ref().unwrap().clone();
                    let weak_window = window.downgrade();
                    let weak_content = window.content().unwrap().downgrade();
                    refresh_slot.borrow().as_ref().unwrap()();
                    button.emit_clicked();
                    assert_eq!(window_slot.borrow().as_ref(), Some(&window));
                    window.close();
                    assert!(window_slot.borrow().is_none());
                    assert!(refresh_slot.borrow().is_none());
                    drop(window);
                    crate::ui::test_support::pump(std::time::Duration::from_millis(50));
                    assert!(
                        weak_window.upgrade().is_none(),
                        "closed Stats window retained"
                    );
                    assert!(
                        weak_content.upgrade().is_none(),
                        "Stats activity/chart callbacks retained content"
                    );
                }
                // Also cover destruction without a close-request, with the
                // embedded page's slot/button released afterwards.
                button.emit_clicked();
                let window = window_slot.borrow().as_ref().unwrap().clone();
                let weak_window = window.downgrade();
                let weak_slot = Rc::downgrade(&window_slot);
                window.destroy();
                drop(window);
                drop(button);
                drop(window_slot);
                drop(refresh_slot);
                drop((nav, stack));
                crate::ui::test_support::pump(std::time::Duration::from_millis(50));
                assert!(weak_slot.upgrade().is_none());
                assert!(weak_window.upgrade().is_none());
                assert!(
                    weak_state.upgrade().is_none(),
                    "Stats callbacks retained navigation after teardown"
                );
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_popout_application_removal_releases_refresh_subscription() {
        crate::ui::test_support::native_test(
            "ui::pages::stats::tests::native_popout_application_removal_releases_refresh_subscription",
            || {
                let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
                let app = adw::Application::builder()
                    .application_id("io.github.worxbend.scenedeck.StatsRemovalFixture")
                    .flags(gio::ApplicationFlags::NON_UNIQUE).build();
                app.register(None::<&gio::Cancellable>).unwrap();
                let window_slot = Rc::new(RefCell::new(None));
                let refresh_slot = Rc::new(RefCell::new(None));
                let button = build_popout_button(&nav, &window_slot, &refresh_slot);
                let host = adw::ApplicationWindow::builder().application(&app).content(&button).build();
                host.present();
                button.emit_clicked();
                let popout = window_slot.borrow().as_ref().unwrap().clone();
                assert!(refresh_slot.borrow().is_some());
                app.remove_window(&popout);
                assert!(window_slot.borrow().is_none());
                assert!(refresh_slot.borrow().is_none(), "removed Stats still subscribed to application telemetry");
                let weak_content = popout.content().unwrap().downgrade();
                popout.destroy();
                drop(popout);
                assert!(weak_content.upgrade().is_none());
                host.close();
            },
        );
    }

    fn stats_sample(configured_fps: Option<f64>) -> crate::domain::stats::ObsStats {
        crate::domain::stats::ObsStats {
            configured_fps,
            active_fps: 120.0,
            disk_space_mb: None,
            cpu_usage_percent: 1.0,
            memory_usage_mb: 100.0,
            average_frame_render_time_ms: 1.0,
            render_skipped_frames: 0,
            render_total_frames: 100,
            output_skipped_frames: 0,
            output_total_frames: 100,
        }
    }

    #[test]
    fn configured_target_comes_from_latest_sample_not_the_history_ceiling() {
        let mut state = AppState::new(Default::default(), Default::default(), None, None);
        assert_eq!(fps_target(&state), None);
        for target in [
            Some(30.0),
            Some(59.94),
            Some(60.0),
            Some(120.0),
            Some(144.0),
            None,
            Some(0.0),
            Some(f64::NAN),
        ] {
            state.stats_history.push(stats_sample(target), None, None);
            let expected = target.filter(|target| target.is_finite() && *target > 0.0);
            assert_eq!(fps_target(&state), expected);
        }
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_stats_fixture_exposes_congestion_and_configured_fps_health() {
        crate::ui::test_support::native_test("ui::pages::stats::tests::native_stats_fixture_exposes_congestion_and_configured_fps_health", || {
            let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
            let gauges = build_gauges(&nav);
            assert!(gauges.all.iter().all(|gauge| gauge.widget().tooltip_text().unwrap().contains(&fl!(LANGUAGE_LOADER, "output-state-unknown"))));
            let stream = crate::domain::stats::StreamHealth {
                active: true, reconnecting: false, congestion: 0.8, skipped_frames: 0,
                total_frames: 100, bytes: 1000,
            };
            nav.state.borrow_mut().stats_history.push(stats_sample(Some(120.0)), None, Some(stream));
            for gauge in &gauges.all { gauge.redraw(); }
            assert!(gauges.all[0].widget().tooltip_text().unwrap().contains("120"));
            assert!(gauges.all[0].widget().tooltip_text().unwrap().contains(&fl!(LANGUAGE_LOADER, "edge-status-ok-label")));
            let congestion = gauges.all[3].widget();
            assert!(congestion.tooltip_text().unwrap().contains(&fl!(LANGUAGE_LOADER, "stats-gauge-congestion")));
            assert!(congestion.tooltip_text().unwrap().contains("80%"));
            assert!(congestion.tooltip_text().unwrap().contains(&fl!(LANGUAGE_LOADER, "diag-label-error")));
            for property in [gtk4::AccessibleProperty::Label, gtk4::AccessibleProperty::Description,
                gtk4::AccessibleProperty::ValueNow, gtk4::AccessibleProperty::ValueText] {
                assert!(gtk4::test_accessible_has_property(congestion, property));
            }
            nav.state.borrow_mut().stats_history.push(stats_sample(None), None, Some(crate::domain::stats::StreamHealth { congestion: 0.1, ..stream }));
            for gauge in &gauges.all { gauge.redraw(); }
            assert!(gauges.all[0].widget().tooltip_text().unwrap().contains(&fl!(LANGUAGE_LOADER, "output-state-unknown")));
            assert!(gauges.all[3].widget().tooltip_text().unwrap().contains("10%"));
        });
    }

    #[test]
    fn frame_budget_follows_the_target_frame_rate() {
        assert!((frame_budget_ms(Some(60.0)).unwrap() - 16.666).abs() < 0.01);
        assert!((frame_budget_ms(Some(30.0)).unwrap() - 33.333).abs() < 0.01);
    }

    #[test]
    fn frame_budget_is_unknown_when_the_target_is_unusable() {
        for target in [
            None,
            Some(0.0),
            Some(-30.0),
            Some(f64::NAN),
            Some(f64::INFINITY),
        ] {
            assert_eq!(frame_budget_ms(target), None);
            assert_eq!(fps_severity(30.0, target), Severity::Unknown);
            assert_eq!(
                frame_time_severity(10.0, frame_budget_ms(target)),
                Severity::Unknown
            );
        }
    }

    #[test]
    fn fps_severity_escalates_as_the_render_falls_behind_target() {
        assert_eq!(fps_severity(60.0, Some(60.0)), Severity::Good);
        assert_eq!(fps_severity(58.0, Some(60.0)), Severity::Good);
        assert_eq!(fps_severity(55.0, Some(60.0)), Severity::Warning);
        assert_eq!(fps_severity(40.0, Some(60.0)), Severity::Critical);
    }

    #[test]
    fn fps_severity_is_unknown_without_a_usable_target() {
        assert_eq!(fps_severity(0.0, None), Severity::Unknown);
        assert_eq!(fps_severity(f64::NAN, Some(60.0)), Severity::Unknown);
    }

    #[test]
    fn configured_fps_health_is_independent_of_rounded_chart_scale() {
        for target in [30.0, 59.94, 60.0, 120.0, 144.0] {
            for jitter in [-0.01, 0.0, 0.01] {
                assert_eq!(fps_severity(target + jitter, Some(target)), Severity::Good);
            }
            assert_eq!(fps_severity(target * 0.90, Some(target)), Severity::Warning);
            assert_eq!(
                fps_severity(target * 0.70, Some(target)),
                Severity::Critical
            );
            assert_eq!(
                fps_severity(target * FPS_WARNING_RATIO, Some(target)),
                Severity::Good
            );
            assert_eq!(
                fps_severity(target * FPS_CRITICAL_RATIO, Some(target)),
                Severity::Warning
            );
            assert_eq!(
                frame_time_severity(0.25 * 1000.0 / target, frame_budget_ms(Some(target))),
                Severity::Good
            );
        }
        assert_eq!(chart::nice_upper_bound(60.01, FPS_BASELINE), 100.0);
        assert_eq!(fps_severity(60.01, Some(60.0)), Severity::Good);
    }

    #[test]
    fn frame_time_severity_is_measured_against_the_frame_budget() {
        let budget = frame_budget_ms(Some(60.0));

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
