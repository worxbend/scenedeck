//! Bottom status bar — a VSCode-style strip showing live OBS connection,
//! streaming/recording state, and performance counters pulled from
//! `GetStats`. Lives across every page, unlike the per-page Live controls.

use adw::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, Separator};
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

#[derive(Clone)]
pub(crate) struct StatusBarHandle {
    pub(crate) root: GtkBox,
    connection_label: Label,
    stream_label: Label,
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

    let connection_label = segment_label("obs-disconnected");
    connection_label.set_text(&ObsStatus::Disconnected.label());

    let stream_label = segment_label("scenedeck-status-bar-output");
    stream_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-stream-inactive"));

    let record_label = segment_label("scenedeck-status-bar-output");
    record_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-record-inactive"));

    let spacer = GtkBox::builder().hexpand(true).build();

    let fps_label = segment_label("scenedeck-status-bar-metric");
    fps_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-fps-placeholder"));

    let bitrate_label = segment_label("scenedeck-status-bar-metric");
    bitrate_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-bitrate-placeholder"));

    let cpu_label = segment_label("scenedeck-status-bar-metric");
    cpu_label.set_text(&fl!(LANGUAGE_LOADER, "status-bar-cpu-placeholder"));

    let dropped_label = segment_label("scenedeck-status-bar-dropped");
    dropped_label.set_visible(false);

    root.append(&connection_label);
    root.append(&separator());
    root.append(&stream_label);
    root.append(&record_label);
    root.append(&spacer);
    root.append(&dropped_label);
    root.append(&cpu_label);
    root.append(&bitrate_label);
    root.append(&fps_label);

    StatusBarHandle {
        root,
        connection_label,
        stream_label,
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
    for class in CONNECTION_CSS_CLASSES {
        handle.connection_label.remove_css_class(class);
    }
    handle.connection_label.add_css_class(status.css_class());
}

/// Reflect the stream output state and elapsed-time text built by the caller.
pub(crate) fn set_stream(handle: &StatusBarHandle, text: &str, active: bool) {
    handle.stream_label.set_text(text);
    set_live_class(&handle.stream_label, active);
}

/// Reflect the record output state and elapsed-time text built by the caller.
pub(crate) fn set_record(handle: &StatusBarHandle, text: &str, active: bool) {
    handle.record_label.set_text(text);
    set_live_class(&handle.record_label, active);
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

    match format_dropped(stats.dropped_frames()) {
        Some(text) => {
            handle.dropped_label.set_text(&text);
            handle.dropped_label.set_visible(true);
        }
        None => handle.dropped_label.set_visible(false),
    }
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
    handle.dropped_label.set_visible(false);
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

fn format_dropped(dropped_frames: u32) -> Option<String> {
    if dropped_frames == 0 {
        return None;
    }
    Some(fl!(
        LANGUAGE_LOADER,
        "status-bar-dropped",
        count = dropped_frames
    ))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn segment_label(extra_css_class: &str) -> Label {
    let label = Label::builder().xalign(0.0).build();
    label.add_css_class("scenedeck-status-bar-item");
    label.add_css_class(extra_css_class);
    label
}

fn separator() -> Separator {
    let separator = Separator::new(Orientation::Vertical);
    separator.add_css_class("scenedeck-status-bar-separator");
    separator
}

fn set_live_class(label: &Label, active: bool) {
    if active {
        label.add_css_class("scenedeck-status-bar-live");
    } else {
        label.remove_css_class("scenedeck-status-bar-live");
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
    fn dropped_frames_are_hidden_when_zero() {
        assert_eq!(format_dropped(0), None);
    }

    #[test]
    fn dropped_frames_are_shown_when_nonzero() {
        assert_eq!(format_dropped(3), Some("3 dropped".to_string()));
    }
}
