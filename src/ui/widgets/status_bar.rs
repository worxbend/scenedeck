//! Bottom status bar — a VSCode-style strip showing live OBS connection,
//! streaming/recording state, and performance counters pulled from
//! `GetStats`. Lives across every page, unlike the per-page Live controls.

use adw::prelude::*;
use gtk4::{Box as GtkBox, Image, Label, Orientation, Separator};
use i18n_embed_fl::fl;

use crate::controller::state::ObsStatus;
use crate::domain::stats::ObsStats;
use crate::infra::i18n::LANGUAGE_LOADER;

const CONNECTION_CSS_CLASSES: &[&str] = &[
    "obs-connected",
    "obs-disconnected",
    "obs-connecting",
    "obs-error",
];

// Each segment leads with an icon so the bar can be read at a glance without
// parsing the text, and so a glance at the shape tells you which counter is
// which even when the numbers are all similar.
const STREAM_ICON: &str = "nf-md-broadcast-symbolic";
const RECORD_ICON: &str = "nf-md-record-circle-symbolic";
const FPS_ICON: &str = "nf-md-speedometer-symbolic";
const BITRATE_ICON: &str = "nf-md-transfer-up-symbolic";
const CPU_ICON: &str = "nf-oct-cpu-symbolic";
const DROPPED_ICON: &str = "nf-md-alert-symbolic";

/// Icon matching a connection state, so the shape changes with the colour.
///
/// Colour alone is not enough: it is the first thing lost to a projector, a
/// colour-blind viewer, or a glance from across the room.
const fn connection_icon(status: &ObsStatus) -> &'static str {
    match status {
        ObsStatus::Connected { .. } => "nf-md-lan-connect-symbolic",
        ObsStatus::Connecting => "nf-md-lan-pending-symbolic",
        ObsStatus::Disconnected => "nf-md-lan-disconnect-symbolic",
        ObsStatus::Error(_) => "nf-md-alert-circle-symbolic",
    }
}

#[derive(Clone)]
pub(crate) struct StatusBarHandle {
    pub(crate) root: GtkBox,
    connection_icon: Image,
    connection_label: Label,
    stream_icon: Image,
    stream_label: Label,
    record_icon: Image,
    record_label: Label,
    fps_label: Label,
    bitrate_label: Label,
    cpu_label: Label,
    dropped_label: Label,
}

/// Build the status bar and its initial "not connected yet" state.
pub(crate) fn build() -> StatusBarHandle {
    let root = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();
    root.add_css_class("scenedeck-status-bar");

    let (connection_segment, connection_icon, connection_label) = segment(
        connection_icon(&ObsStatus::Disconnected),
        "obs-disconnected",
    );
    connection_label.set_text(&ObsStatus::Disconnected.label());

    let (stream_segment, stream_icon, stream_label) =
        segment(STREAM_ICON, "scenedeck-status-bar-output");
    stream_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-stream-inactive"));

    let (record_segment, record_icon, record_label) =
        segment(RECORD_ICON, "scenedeck-status-bar-output");
    record_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-record-inactive"));

    let spacer = GtkBox::builder().hexpand(true).build();

    let (fps_segment, _, fps_label) = segment(FPS_ICON, "scenedeck-status-bar-metric");
    fps_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-fps-placeholder"));

    let (bitrate_segment, _, bitrate_label) = segment(BITRATE_ICON, "scenedeck-status-bar-metric");
    bitrate_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-bitrate-placeholder"));

    let (cpu_segment, _, cpu_label) = segment(CPU_ICON, "scenedeck-status-bar-metric");
    cpu_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-cpu-placeholder"));

    // Dropped frames stay on screen at all times, including at zero: a counter
    // that only appears once something is wrong cannot be checked at a glance
    // mid-stream to confirm nothing is wrong.
    let (dropped_segment, _, dropped_label) = segment(DROPPED_ICON, "scenedeck-status-bar-dropped");
    dropped_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-dropped-placeholder"));

    root.append(&connection_segment);
    root.append(&separator());
    root.append(&stream_segment);
    root.append(&record_segment);
    root.append(&spacer);
    root.append(&dropped_segment);
    root.append(&cpu_segment);
    root.append(&bitrate_segment);
    root.append(&fps_segment);

    StatusBarHandle {
        root,
        connection_icon,
        connection_label,
        stream_icon,
        stream_label,
        record_icon,
        record_label,
        fps_label,
        bitrate_label,
        cpu_label,
        dropped_label,
    }
}

/// Reflect the current OBS connection lifecycle state.
pub(crate) fn set_connection(handle: &StatusBarHandle, status: &ObsStatus) {
    handle.connection_label.set_text(&status.label());
    handle
        .connection_icon
        .set_icon_name(Some(connection_icon(status)));
    for class in CONNECTION_CSS_CLASSES {
        handle.connection_label.remove_css_class(class);
        handle.connection_icon.remove_css_class(class);
    }
    handle.connection_label.add_css_class(status.css_class());
    handle.connection_icon.add_css_class(status.css_class());
}

/// Reflect the stream output state and elapsed-time text built by the caller.
pub(crate) fn set_stream(handle: &StatusBarHandle, text: &str, active: bool) {
    handle.stream_label.set_text(text);
    set_live_class(&handle.stream_label, active);
    set_live_class(&handle.stream_icon, active);
}

/// Reflect the record output state and elapsed-time text built by the caller.
pub(crate) fn set_record(handle: &StatusBarHandle, text: &str, active: bool) {
    handle.record_label.set_text(text);
    set_live_class(&handle.record_label, active);
    set_live_class(&handle.record_icon, active);
}

/// Apply a fresh `GetStats` snapshot plus a derived bitrate to the
/// performance segments. `streaming` gates whether bitrate is meaningful.
pub(crate) fn set_stats(
    handle: &StatusBarHandle,
    stats: &ObsStats,
    bitrate_kbps: Option<f64>,
    streaming: bool,
) {
    handle.fps_label.set_text(&format_fps(stats.active_fps));
    handle
        .cpu_label
        .set_text(&format_cpu(stats.cpu_usage_percent));
    handle
        .bitrate_label
        .set_text(&format_bitrate(bitrate_kbps, streaming));

    let dropped_frames = stats.dropped_frames();
    handle
        .dropped_label
        .set_text(&format_dropped(dropped_frames));
    set_dropped_alert_class(&handle.dropped_label, dropped_frames > 0);
}

/// Reset performance segments to their placeholder state, e.g. on disconnect.
pub(crate) fn clear_stats(handle: &StatusBarHandle) {
    handle
        .fps_label
        .set_text(&fl!(LANGUAGE_LOADER, "status-bar-fps-placeholder"));
    handle
        .cpu_label
        .set_text(&fl!(LANGUAGE_LOADER, "status-bar-cpu-placeholder"));
    handle
        .bitrate_label
        .set_text(&fl!(LANGUAGE_LOADER, "status-bar-bitrate-placeholder"));
    handle
        .dropped_label
        .set_text(&fl!(LANGUAGE_LOADER, "status-bar-dropped-placeholder"));
    set_dropped_alert_class(&handle.dropped_label, false);
}

// ── Formatting (pure, unit tested) ────────────────────────────────────────────

fn format_fps(active_fps: f64) -> String {
    fl!(
        LANGUAGE_LOADER,
        "status-bar-fps",
        value = format!("{active_fps:.1}")
    )
}

fn format_cpu(cpu_usage_percent: f64) -> String {
    fl!(
        LANGUAGE_LOADER,
        "status-bar-cpu",
        value = format!("{cpu_usage_percent:.1}")
    )
}

fn format_bitrate(bitrate_kbps: Option<f64>, streaming: bool) -> String {
    if !streaming {
        return fl!(LANGUAGE_LOADER, "status-bar-bitrate-placeholder");
    }
    match bitrate_kbps {
        Some(kbps) => fl!(
            LANGUAGE_LOADER,
            "status-bar-bitrate",
            value = format!("{kbps:.0}")
        ),
        None => fl!(LANGUAGE_LOADER, "status-bar-bitrate-placeholder"),
    }
}

fn format_dropped(dropped_frames: u32) -> String {
    fl!(
        LANGUAGE_LOADER,
        "status-bar-dropped",
        count = dropped_frames
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// One status-bar segment: an icon and its text, sharing a state class.
fn segment(icon_name: &str, extra_css_class: &str) -> (GtkBox, Image, Label) {
    let segment = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .valign(gtk4::Align::Center)
        .build();

    let icon = Image::from_icon_name(icon_name);
    icon.add_css_class("scenedeck-status-bar-icon");
    icon.add_css_class(extra_css_class);

    let label = Label::builder().xalign(0.0).build();
    label.add_css_class("scenedeck-status-bar-item");
    label.add_css_class(extra_css_class);

    segment.append(&icon);
    segment.append(&label);
    (segment, icon, label)
}

fn separator() -> Separator {
    let separator = Separator::new(Orientation::Vertical);
    separator.add_css_class("scenedeck-status-bar-separator");
    separator
}

/// Highlight the dropped-frame counter only once frames are actually being
/// lost, so the permanent segment does not read as a permanent warning.
fn set_dropped_alert_class(label: &Label, dropping: bool) {
    if dropping {
        label.add_css_class("scenedeck-status-bar-dropped-active");
    } else {
        label.remove_css_class("scenedeck-status-bar-dropped-active");
    }
}

/// Mark a segment as live, so the icon turns with the text rather than the
/// colour carrying the state on its own.
fn set_live_class(widget: &impl IsA<gtk4::Widget>, active: bool) {
    if active {
        widget.add_css_class("scenedeck-status-bar-live");
    } else {
        widget.remove_css_class("scenedeck-status-bar-live");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_is_formatted_with_one_decimal() {
        assert_eq!(format_fps(59.94), "FPS 59.9");
        assert_eq!(format_fps(60.0), "FPS 60.0");
    }

    #[test]
    fn cpu_is_formatted_as_percent_with_one_decimal() {
        assert_eq!(format_cpu(12.34), "CPU 12.3%");
    }

    #[test]
    fn bitrate_hides_behind_placeholder_while_not_streaming() {
        assert_eq!(format_bitrate(Some(6000.0), false), "Bitrate —");
        assert_eq!(format_bitrate(None, false), "Bitrate —");
    }

    #[test]
    fn bitrate_shows_placeholder_until_a_sample_is_available_while_streaming() {
        assert_eq!(format_bitrate(None, true), "Bitrate —");
    }

    #[test]
    fn bitrate_rounds_to_whole_kbps_while_streaming() {
        assert_eq!(format_bitrate(Some(6042.7), true), "Bitrate 6043 kbps");
    }

    #[test]
    fn dropped_frames_stay_on_screen_at_zero() {
        assert_eq!(format_dropped(0), "0 dropped");
    }

    #[test]
    fn dropped_frames_are_shown_when_nonzero() {
        assert_eq!(format_dropped(3), "3 dropped");
    }
}
