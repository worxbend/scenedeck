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
    let tools = gtk4::FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(2)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    crate::ui::insert_compact_flow_child(&tools, &crate::ui::health_alerts::button(nav.clone()));
    crate::ui::insert_compact_flow_child(&tools, &crate::ui::bookmarks::button(nav.clone()));
    crate::ui::insert_compact_flow_child(&tools, &crate::ui::session_reports::button(nav.clone()));
    page.append(&tools);

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
    let (health, health_tick) = crate::ui::health_alerts::build(nav.clone());
    toolbar.add_top_bar(&health);
    toolbar.set_content(Some(&scroll));

    let window = adw::Window::builder()
        .title(fl!(LANGUAGE_LOADER, "stats-page-title"))
        .default_width(640)
        .default_height(720)
        .content(&toolbar)
        .build();
    if let Some(source) = glib::MainContext::default().find_source_by_id(&health_tick) {
        window.connect_destroy({
            let source = source.clone();
            move |_| source.destroy()
        });
        window.connect_notify_local(Some("application"), move |window, _| {
            if window.application().is_none() {
                source.destroy();
            }
        });
    }

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
mod tests;
