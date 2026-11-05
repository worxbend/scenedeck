//! Reviewable report snapshots with explicit, off-main-thread exports.

use super::{background_io, navigation::NavigationContext};
use crate::{
    domain::{
        output_result::{OutputResultKind, OutputResultStatus},
        session_report::{ReportDocument, ReportEvent},
        workflow::ActivityOrigin,
    },
    infra::i18n::LANGUAGE_LOADER,
    storage::report_export::Format,
};
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation};
use i18n_embed_fl::fl;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

thread_local! {
    static EXPORT_FAILURES: RefCell<Vec<(std::path::PathBuf, String, background_io::FailureEvidence)>> = const { RefCell::new(Vec::new()) };
}

fn label(value: &str) -> Label {
    Label::builder()
        .label(value)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .xalign(0.0)
        .selectable(true)
        .build()
}

fn control(value: &str) -> Button {
    let button = Button::new();
    let text = label(value);
    text.set_selectable(false);
    button.set_child(Some(&text));
    button
}

fn date(ms: u64) -> String {
    glib::DateTime::from_unix_utc((ms / 1_000).min(i64::MAX as u64) as i64)
        .and_then(|date| date.format("%Y-%m-%d %H:%M:%S UTC"))
        .map(|value| value.to_string())
        .unwrap_or_else(|_| fl!(LANGUAGE_LOADER, "media-time-unavailable"))
}

fn origin(value: ActivityOrigin) -> String {
    match value {
        ActivityOrigin::Requested => fl!(LANGUAGE_LOADER, "workflow-activity-requested"),
        ActivityOrigin::Observed => fl!(LANGUAGE_LOADER, "workflow-activity-observed"),
        ActivityOrigin::Failed => fl!(LANGUAGE_LOADER, "workflow-activity-failed"),
        ActivityOrigin::Connection => fl!(LANGUAGE_LOADER, "workflow-activity-connection"),
    }
}

fn event_text(event: &ReportEvent) -> String {
    let unknown = || fl!(LANGUAGE_LOADER, "media-time-unavailable");
    match event {
        ReportEvent::Activity(record) => {
            format!("{} · {}", origin(record.origin), record.description)
        }
        ReportEvent::Output(result) => {
            let kind = match result.kind {
                OutputResultKind::ReplaySave => fl!(LANGUAGE_LOADER, "control-replay-save"),
                OutputResultKind::RecordingStop => {
                    fl!(LANGUAGE_LOADER, "output-results-record-stop")
                }
                OutputResultKind::RecordingSplit => fl!(LANGUAGE_LOADER, "control-split"),
                OutputResultKind::RecordingChapter => fl!(LANGUAGE_LOADER, "control-chapter"),
            };
            let status = match result.status {
                OutputResultStatus::Requested => origin(ActivityOrigin::Requested),
                OutputResultStatus::Acknowledged => {
                    fl!(LANGUAGE_LOADER, "output-results-acknowledged")
                }
                OutputResultStatus::Observed => origin(ActivityOrigin::Observed),
                OutputResultStatus::Failed => origin(ActivityOrigin::Failed),
                OutputResultStatus::Unconfirmed => {
                    fl!(LANGUAGE_LOADER, "output-results-unconfirmed")
                }
            };
            let path = result
                .path
                .as_ref()
                .filter(|_| result.status == OutputResultStatus::Observed)
                .map(|path| {
                    fl!(
                        LANGUAGE_LOADER,
                        "output-results-host-path",
                        path = path.as_str()
                    )
                })
                .unwrap_or_default();
            format!(
                "{status} · {kind} · {path} {}",
                result.detail.as_deref().unwrap_or_default()
            )
        }
        ReportEvent::OutputAcknowledged { kind } => format!(
            "{} · {}",
            fl!(LANGUAGE_LOADER, "output-results-acknowledged"),
            kind.label()
        ),
        ReportEvent::Scene { scene, .. } => fl!(
            LANGUAGE_LOADER,
            "reports-scene",
            scene = scene.clone().unwrap_or_else(unknown)
        ),
        ReportEvent::Ended { reason } => {
            fl!(LANGUAGE_LOADER, "reports-ended", reason = reason.as_str())
        }
        ReportEvent::Health { kind, input, phase } => {
            let title = match kind.as_str() {
                "connection_lost" => fl!(LANGUAGE_LOADER, "health-connection-lost"),
                "stream_stopped" => fl!(LANGUAGE_LOADER, "health-stream-stopped"),
                "record_stopped" => fl!(LANGUAGE_LOADER, "health-record-stopped"),
                "network_drops" => fl!(LANGUAGE_LOADER, "health-network"),
                "render_overload" => fl!(LANGUAGE_LOADER, "health-render"),
                "encoder_overload" => fl!(LANGUAGE_LOADER, "health-encoder"),
                "low_disk" => fl!(LANGUAGE_LOADER, "health-disk"),
                "audio_silence" => fl!(
                    LANGUAGE_LOADER,
                    "health-audio-silence",
                    input = input.clone().unwrap_or_else(unknown)
                ),
                "audio_clipping" => fl!(
                    LANGUAGE_LOADER,
                    "health-audio-clipping",
                    input = input.clone().unwrap_or_else(unknown)
                ),
                _ => unknown(),
            };
            let phase = match phase.as_deref() {
                Some("active") => fl!(LANGUAGE_LOADER, "health-active"),
                Some("recovering") => fl!(LANGUAGE_LOADER, "health-recovering"),
                Some("unobserved") => fl!(LANGUAGE_LOADER, "health-unobserved"),
                _ => fl!(LANGUAGE_LOADER, "reports-alert-cleared"),
            };
            format!("{title} · {phase}")
        }
    }
}

fn summary(document: &ReportDocument) -> String {
    let report = &document.report;
    let unknown = || fl!(LANGUAGE_LOADER, "media-time-unavailable");
    fl!(
        LANGUAGE_LOADER,
        "reports-summary",
        start = date(report.started_unix_ms),
        end = report
            .ended_unix_ms
            .map(date)
            .unwrap_or_else(|| if report.interrupted {
                fl!(LANGUAGE_LOADER, "reports-interrupted")
            } else {
                fl!(LANGUAGE_LOADER, "reports-current")
            }),
        connection = report
            .connection_name
            .as_deref()
            .unwrap_or(&report.connection_id),
        endpoint = report.endpoint.as_str(),
        events = report.events.len().to_string(),
        omitted = report.omitted_events.to_string(),
        gaps = report.activity_gaps.to_string(),
        samples = report.samples_seen.to_string(),
        kept = report.samples.len().to_string(),
        cpu = report
            .peak_cpu_percent
            .map(|value| format!("{value:.1}%"))
            .unwrap_or_else(unknown),
        fps = report
            .minimum_fps
            .map(|value| format!("{value:.1}"))
            .unwrap_or_else(unknown),
        bookmarks = document.bookmarks.len().to_string()
    )
}

fn body(document: &ReportDocument) -> String {
    let unknown = || fl!(LANGUAGE_LOADER, "media-time-unavailable");
    let mut lines = vec![fl!(LANGUAGE_LOADER, "reports-view-limit")];
    for record in document.report.events.iter().rev().take(150) {
        lines.push(format!(
            "{} · {}\n{}",
            date(record.timestamp_unix_ms),
            record.collection.clone().unwrap_or_else(unknown),
            event_text(&record.event)
        ));
    }
    lines.push(fl!(LANGUAGE_LOADER, "bookmarks-title"));
    for note in document.bookmarks.iter().rev().take(50) {
        lines.push(format!(
            "{}\n{}",
            note.name,
            super::bookmarks::describe(note)
        ));
    }
    lines.join("\n\n")
}

pub(crate) fn button(nav: NavigationContext) -> Button {
    let button = control(&fl!(LANGUAGE_LOADER, "reports-title"));
    button.connect_clicked(move |button| {
        if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
            show(&parent, &nav);
        }
    });
    button
}

pub(super) fn show(parent: &gtk4::Window, nav: &NavigationContext) {
    let window = adw::Window::builder()
        .title(fl!(LANGUAGE_LOADER, "reports-title"))
        .transient_for(parent)
        .default_width(720)
        .default_height(650)
        .build();
    window.set_widget_name("reports-window");
    if let Some(app) = parent.application() {
        window.set_application(Some(&app));
    }
    let alive = Rc::new(Cell::new(true));
    window.connect_close_request({
        let alive = alive.clone();
        move |_| {
            alive.set(false);
            glib::Propagation::Proceed
        }
    });
    let outer = GtkBox::new(Orientation::Vertical, 0);
    outer.append(&adw::HeaderBar::new());
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.append(&label(&fl!(LANGUAGE_LOADER, "reports-policy")));
    content.append(&super::report_history::open_button(nav.clone()));
    let actions = gtk4::FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(3)
        .row_spacing(6)
        .column_spacing(6)
        .build();
    let previous = control(&fl!(LANGUAGE_LOADER, "reports-previous"));
    let next = control(&fl!(LANGUAGE_LOADER, "reports-next"));
    let refresh = control(&fl!(LANGUAGE_LOADER, "reports-refresh"));
    let json = control(&fl!(LANGUAGE_LOADER, "reports-export-json"));
    let csv = control(&fl!(LANGUAGE_LOADER, "reports-export-csv"));
    for button in [&previous, &next, &refresh, &json, &csv] {
        super::insert_compact_flow_child(&actions, button);
    }
    for (button, name) in [
        (&previous, "reports-previous"),
        (&next, "reports-next"),
        (&refresh, "reports-refresh"),
        (&json, "reports-json"),
        (&csv, "reports-csv"),
    ] {
        button.set_widget_name(name);
    }
    content.append(&actions);
    let title = label("");
    title.set_widget_name("reports-summary");
    content.append(&title);
    let status = label("");
    content.append(&status);
    let text = label("");
    text.set_widget_name("reports-body");
    content.append(&text);
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&content)
        .build();
    outer.append(&scroll);
    window.set_content(Some(&outer));
    let selected = Rc::new(RefCell::new(None::<String>));
    let snapshot = Rc::new(RefCell::new(None::<ReportDocument>));
    let render: Rc<dyn Fn(i32)> = Rc::new({
        let nav = nav.clone();
        let selected = selected.clone();
        let snapshot = snapshot.clone();
        let alive = alive.clone();
        let previous = previous.downgrade();
        let next = next.downgrade();
        let json = json.downgrade();
        let csv = csv.downgrade();
        move |step| {
            if !alive.get() {
                return;
            }
            let (Some(previous), Some(next), Some(json), Some(csv)) = (
                previous.upgrade(),
                next.upgrade(),
                json.upgrade(),
                csv.upgrade(),
            ) else {
                return;
            };
            let mut state = nav.state.borrow_mut();
            state.collect_report(Instant::now(), crate::controller::workspace::unix_ms());
            let entries = state.report_list();
            let index = selected
                .borrow()
                .as_ref()
                .and_then(|id| entries.iter().position(|report| &report.id == id))
                .unwrap_or_else(|| entries.len().saturating_sub(1));
            let index =
                (index as i32 + step).clamp(0, entries.len().saturating_sub(1) as i32) as usize;
            previous.set_sensitive(index > 0);
            next.set_sensitive(index + 1 < entries.len());
            let document = entries.get(index).map(|report| ReportDocument {
                report: (*report).clone(),
                bookmarks: report.bookmarks(&state.bookmarks.archive.entries),
            });
            json.set_sensitive(document.is_some());
            csv.set_sensitive(document.is_some());
            if let Some(document) = &document {
                *selected.borrow_mut() = Some(document.report.id.clone());
                title.set_text(&summary(document));
                text.set_text(&body(document));
            } else {
                title.set_text(&fl!(LANGUAGE_LOADER, "reports-empty"));
                text.set_text("");
            }
            *snapshot.borrow_mut() = document;
        }
    });
    for (button, step) in [(previous, -1), (next, 1), (refresh, 0)] {
        let render = render.clone();
        button.connect_clicked(move |_| render(step));
    }
    let busy = Rc::new(Cell::new(false));
    for (button, format) in [(json, Format::Json), (csv, Format::Csv)] {
        let snapshot = snapshot.clone();
        let status = status.clone();
        let alive = alive.clone();
        let busy = busy.clone();
        let window = window.downgrade();
        button.connect_clicked(move |_| {
            if !alive.get() || busy.get() {
                return;
            }
            let Some(document) = snapshot.borrow().clone() else {
                return;
            };
            let Some(window) = window.upgrade() else {
                return;
            };
            export(
                &window,
                document,
                format,
                &status,
                alive.clone(),
                busy.clone(),
            );
        });
    }
    render(0);
    window.present();
}

fn export(
    parent: &adw::Window,
    document: ReportDocument,
    format: Format,
    status: &Label,
    alive: Rc<Cell<bool>>,
    busy: Rc<Cell<bool>>,
) {
    busy.set(true);
    let dialog = gtk4::FileDialog::builder()
        .title(fl!(LANGUAGE_LOADER, "reports-title"))
        .modal(true)
        .build();
    dialog.set_initial_name(Some(&format!(
        "scenedeck-session-{}.{}",
        document.report.id,
        format.extension()
    )));
    let status = status.clone();
    dialog.save(Some(parent), gio::Cancellable::NONE, move |result| {
        if !alive.get() {
            busy.set(false);
            return;
        }
        let file = match result {
            Ok(file) => file,
            Err(error) => {
                busy.set(false);
                if !error.matches(gtk4::DialogError::Dismissed)
                    && !error.matches(gtk4::DialogError::Cancelled)
                    && !error.matches(gio::IOErrorEnum::Cancelled)
                {
                    status.set_text(&fl!(
                        LANGUAGE_LOADER,
                        "reports-export-error",
                        error = error.to_string()
                    ));
                }
                return;
            }
        };
        let Some(path) = file.path() else {
            busy.set(false);
            status.set_text(&fl!(LANGUAGE_LOADER, "inventory-export-no-file"));
            return;
        };
        export_to_path(path, document, format, status, busy);
    });
}

fn export_to_path(
    path: std::path::PathBuf,
    document: ReportDocument,
    format: Format,
    status: Label,
    busy: Rc<Cell<bool>>,
) {
    busy.set(true);
    let display = path.display().to_string();
    let failure_path = path.clone();
    let report_id = document.report.id.clone();
    status.set_text(&fl!(LANGUAGE_LOADER, "reports-exporting"));
    background_io::run_serialized(
        background_io::SerialLane::ReportExports,
        move || crate::storage::report_export::write(&path, &document, format),
        move |result| {
            busy.set(false);
            match result {
                Ok(()) => {
                    EXPORT_FAILURES.with(|failures| {
                        failures.borrow_mut().retain(|(path, id, evidence)| {
                            if path == &failure_path && id == &report_id {
                                background_io::resolve_failure(*evidence);
                                false
                            } else {
                                true
                            }
                        })
                    });
                    status.set_text(&fl!(LANGUAGE_LOADER, "reports-exported", path = display));
                }
                Err(error) => {
                    EXPORT_FAILURES.with(|failures| {
                        failures.borrow_mut().push((
                            failure_path,
                            report_id,
                            background_io::record_failure(),
                        ))
                    });
                    status.set_text(&fl!(
                        LANGUAGE_LOADER,
                        "reports-export-error",
                        error = error.to_string()
                    ));
                }
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::{event::ConnectionInfo, state::ObsStatus};
    use crate::ui::{
        preflight::tests::{find, window},
        test_support::{native_test, navigation, pump, pump_until},
    };
    use std::time::Duration;

    fn connect(nav: &NavigationContext, name: &str) {
        let mut state = nav.state.borrow_mut();
        state.apply_connected(ConnectionInfo {
            connection_id: name.into(),
            endpoint: "fixture:4455".into(),
            obs_version: "fixture".into(),
            websocket_version: "5".into(),
            available_requests: Vec::new(),
            supported_image_formats: Vec::new(),
        });
        state.scene_collections.current = Some("Interview".into());
        state.scene_inventory.current_id = Some("Camera <b>literal</b>".into());
        state
            .capture_bookmark("Topic one", Instant::now(), 50)
            .unwrap();
        state.collect_report(Instant::now(), 60);
    }

    #[test]
    #[ignore = "requires isolated GTK and temporary XDG files"]
    fn native_reports_snapshot_navigation_export_retry_and_narrow_layout() {
        native_test("ui::session_reports::tests::native_reports_snapshot_navigation_export_retry_and_narrow_layout", || {
            crate::infra::i18n::init(crate::domain::appearance::Language::Uk);
            gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
            let (nav, stack, _runtime, events) = navigation();
            let parent = adw::Window::builder().default_width(360).default_height(600).build();
            parent.set_content(Some(&stack)); parent.present();
            assert!(super::super::action_catalog::execute(&nav, &super::super::action_catalog::CatalogAction::Reports));
            let dialog = window("reports-window"); dialog.set_default_size(360, 600); pump(Duration::from_millis(100));
            assert!(!find(&dialog, "reports-json").is_sensitive());
            connect(&nav, "First");
            let refresh = find(&dialog, "reports-refresh").downcast::<Button>().unwrap(); refresh.emit_clicked();
            let title = find(&dialog, "reports-summary").downcast::<Label>().unwrap();
            let text = find(&dialog, "reports-body").downcast::<Label>().unwrap();
            assert!(title.text().contains("First")); assert!(text.text().contains("<b>literal</b>")); assert!(!text.uses_markup());
            nav.state.borrow_mut().reset_obs_session(ObsStatus::Disconnected);
            connect(&nav, "Second");
            assert!(title.text().contains("First"), "snapshot remains stable until refresh");
            refresh.emit_clicked();
            assert!(title.text().contains("First"), "refresh preserves the selected session");
            find(&dialog, "reports-next").downcast::<Button>().unwrap().emit_clicked(); assert!(title.text().contains("Second"));
            find(&dialog, "reports-previous").downcast::<Button>().unwrap().emit_clicked(); assert!(title.text().contains("First"));
            assert!(dialog.measure(Orientation::Horizontal, -1).0 <= 360);
            assert!(dialog.measure(Orientation::Vertical, 360).0 <= 600);
            let document = {
                let state = nav.state.borrow(); let report = state.reports.entries[0].clone();
                ReportDocument { bookmarks: report.bookmarks(&state.bookmarks.archive.entries), report }
            };
            let path = crate::storage::xdg::bookmarks_path().with_file_name("report-custom.txt");
            std::fs::create_dir_all(&path).unwrap();
            let busy = Rc::new(Cell::new(false)); let status = label("");
            export_to_path(path.clone(), document.clone(), Format::Json, status.clone(), busy.clone());
            assert!(pump_until(Duration::from_secs(4), || !busy.get()));
            assert_eq!(EXPORT_FAILURES.with(|failures| failures.borrow().len()), 1);
            assert_ne!(status.text(), fl!(LANGUAGE_LOADER, "reports-exporting"));
            std::fs::remove_dir(&path).unwrap();
            export_to_path(path.clone(), document, Format::Json, status.clone(), busy.clone());
            assert!(pump_until(Duration::from_secs(4), || !busy.get()));
            assert!(EXPORT_FAILURES.with(|failures| failures.borrow().is_empty()));
            let saved: ReportDocument = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            assert_eq!(saved.report.connection_id, "First"); assert_eq!(saved.bookmarks[0].name, "Topic one");
            assert!(saved.report.ended_unix_ms.is_some());
            assert!(events.try_iter().next().is_none(), "reports must not issue OBS commands");
            dialog.close(); let before = title.text(); refresh.emit_clicked(); assert_eq!(title.text(), before);
            parent.close();
        });
    }
}
