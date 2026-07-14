//! Live page — primary daily-operation surface.
//!
//! Returns a `LivePageHandle` so `ui::window` can push state updates into the
//! widgets without rebuilding the page.

use adw::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, FlowBox, FlowBoxChild, Label, Orientation, Paned, PolicyType,
    ScrolledWindow, Stack, StackTransitionType,
};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::domain::output::OutputStatus;
use crate::domain::scene::SceneInventory;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::config::OutputConfig;
use crate::storage::registry::read_registry;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::{audio_card, scene_card};

const EMPTY_OUTPUT_SLOT: &str = "";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OutputKind {
    Stream,
    Recording,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OutputAction {
    Start,
    Stop,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OutputConfirmationAppearance {
    Suggested,
    Destructive,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct OutputCommandErrorDisplay<'a> {
    label: String,
    tooltip: &'a str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct OutputRecordingPathDisplay<'a> {
    label: String,
    tooltip: &'a str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct OutputConfirmationDialog {
    heading: String,
    body: String,
    confirm_label: String,
    appearance: OutputConfirmationAppearance,
}

/// Widget handles that `ui::window` updates when `AppEvent`s arrive.
pub(crate) struct LivePageHandle {
    pub(crate) root: Stack,
    pub(crate) stream_label: Label,
    pub(crate) stream_progress_label: Label,
    pub(crate) stream_error_label: Label,
    pub(crate) stream_btn: Button,
    pub(crate) record_label: Label,
    pub(crate) record_progress_label: Label,
    pub(crate) record_error_label: Label,
    pub(crate) record_path_label: Label,
    pub(crate) record_btn: Button,
    pub(crate) record_path_btn: Button,
    pub(crate) current_scene_label: Label,
    pub(crate) scenes_box: FlowBox,
    pub(crate) audio_box: FlowBox,
    pub(crate) audio_cards: std::cell::RefCell<Vec<audio_card::AudioCardHandle>>,
}

pub(crate) fn build(nav: NavigationContext) -> LivePageHandle {
    let root = Stack::builder()
        .vexpand(true)
        .hexpand(true)
        .transition_type(StackTransitionType::Crossfade)
        .build();
    root.add_css_class("app-page");
    root.add_css_class("live-page");

    let disconnected = build_disconnected_view();
    root.add_named(&disconnected, Some("disconnected"));

    // ── Outer layout ─────────────────────────────────────────────────────────
    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(20)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .vexpand(true)
        .hexpand(true)
        .build();
    root.add_named(&page, Some("live"));
    root.set_visible_child_name("disconnected");

    // ── Output controls ──────────────────────────────────────────────────────
    let banner = GtkBox::new(Orientation::Horizontal, 0);
    banner.add_css_class("card");

    // Inner box carries the padding so the card class doesn't need to.
    let banner_inner = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_start(16)
        .margin_end(12)
        .margin_top(6)
        .margin_bottom(6)
        .hexpand(true)
        .build();
    banner.append(&banner_inner);

    let stream_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "live-start-stream-label"))
        .valign(Align::Center)
        .sensitive(false)
        .build();
    stream_btn.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "live-stream-toggle-tooltip")));
    stream_btn.connect_clicked({
        let nav = nav.clone();
        move |button| handle_stream_output_toggle(button, &nav)
    });

    let stream_label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-stream-inactive-label"))
        .xalign(0.0)
        .build();
    stream_label.add_css_class("caption");
    stream_label.add_css_class("dim-label");

    let stream_progress_label = build_output_progress_label();
    let stream_error_label = build_output_error_label();

    let record_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "live-start-record-label"))
        .valign(Align::Center)
        .sensitive(false)
        .build();
    record_btn.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "live-record-toggle-tooltip")));
    record_btn.connect_clicked({
        let nav = nav.clone();
        move |button| handle_record_output_toggle(button, &nav)
    });

    let record_label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-record-inactive-label"))
        .xalign(0.0)
        .build();
    record_label.add_css_class("caption");
    record_label.add_css_class("dim-label");

    let record_progress_label = build_output_progress_label();
    let record_error_label = build_output_error_label();
    let record_path_label = build_output_detail_label();

    let record_path_btn = Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text(fl!(
            LANGUAGE_LOADER,
            "live-copy-last-recording-path-tooltip"
        ))
        .valign(Align::Center)
        .sensitive(false)
        .build();
    record_path_btn.add_css_class("flat");
    record_path_btn.add_css_class("circular");
    record_path_btn.connect_clicked({
        let nav = nav.clone();
        move |button| {
            let Some(path) = nav.state.borrow().last_recording_path.clone() else {
                return;
            };
            if let Some(display) = gtk4::gdk::Display::default() {
                display.clipboard().set_text(&path);
                button.set_tooltip_text(Some(&fl!(
                    LANGUAGE_LOADER,
                    "live-copied-recording-path-tooltip"
                )));
            }
        }
    });

    let stream_control = build_output_card(
        &fl!(LANGUAGE_LOADER, "live-stream-card-title"),
        &stream_btn,
        &stream_label,
        &stream_progress_label,
        &stream_error_label,
        None,
        None,
    );
    let record_control = build_output_card(
        &fl!(LANGUAGE_LOADER, "live-recording-card-title"),
        &record_btn,
        &record_label,
        &record_progress_label,
        &record_error_label,
        Some(&record_path_label),
        Some(&record_path_btn),
    );

    banner_inner.append(&stream_control);
    banner_inner.append(&record_control);
    page.append(&banner);

    // ── Program scene label ───────────────────────────────────────────────────
    let current_label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-current-scene-placeholder"))
        .xalign(0.0)
        .build();
    current_label.add_css_class("heading");
    page.append(&current_label);

    // ── Resizeable live panes ─────────────────────────────────────────────────
    let live_split = Paned::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .wide_handle(true)
        .build();
    live_split.set_resize_start_child(false);
    live_split.set_resize_end_child(true);
    live_split.set_shrink_start_child(true);
    live_split.set_shrink_end_child(false);
    page.append(&live_split);

    // ── Scene cards ───────────────────────────────────────────────────────────
    let scenes_pane = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .vexpand(false)
        .hexpand(true)
        .build();
    let scenes_section_label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-scenes-section-label"))
        .xalign(0.0)
        .build();
    scenes_section_label.add_css_class("caption-heading");
    scenes_pane.append(&scenes_section_label);

    let scenes_box = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(6)
        .row_spacing(6)
        .homogeneous(false)
        .halign(Align::Start)
        .valign(Align::Start)
        .hexpand(false)
        .vexpand(false)
        .margin_top(3)
        .margin_bottom(1)
        .margin_start(3)
        .margin_end(3)
        .min_children_per_line(1)
        .max_children_per_line(6)
        .build();
    insert_scene_placeholder(
        &scenes_box,
        &fl!(LANGUAGE_LOADER, "live-scenes-connect-hint"),
    );

    let scenes_scroll = ScrolledWindow::builder()
        .vexpand(false)
        .hexpand(true)
        .min_content_height(72)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&scenes_box)
        .build();
    scenes_scroll.add_css_class("live-pane-scroll");
    scenes_pane.append(&scenes_scroll);
    live_split.set_start_child(Some(&scenes_pane));

    // ── Audio mixer ───────────────────────────────────────────────────────────
    let audio_pane = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .vexpand(true)
        .hexpand(true)
        .build();

    let audio_section_label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-audio-section-label"))
        .xalign(0.0)
        .build();
    audio_section_label.add_css_class("caption-heading");
    audio_pane.append(&audio_section_label);

    let audio_box = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(5)
        .row_spacing(6)
        .homogeneous(false)
        .halign(Align::Start)
        .valign(Align::Start)
        .hexpand(false)
        .vexpand(false)
        .min_children_per_line(1)
        .max_children_per_line(12)
        .build();

    let audio_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .min_content_height(232)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&audio_box)
        .build();
    audio_scroll.add_css_class("live-pane-scroll");
    audio_pane.append(&audio_scroll);
    live_split.set_end_child(Some(&audio_pane));

    LivePageHandle {
        root,
        stream_label,
        stream_progress_label,
        stream_error_label,
        stream_btn,
        record_label,
        record_progress_label,
        record_error_label,
        record_path_label,
        record_btn,
        record_path_btn,
        current_scene_label: current_label,
        scenes_box,
        audio_box,
        audio_cards: std::cell::RefCell::new(Vec::new()),
    }
}

pub(crate) fn show_disconnected_view(handle: &LivePageHandle, message: &str) {
    handle.root.set_visible_child_name("disconnected");
    if let Some(page) = handle.root.child_by_name("disconnected") {
        if let Some(label) = page
            .first_child()
            .and_then(|child| child.downcast::<Label>().ok())
        {
            label.set_text(message);
        }
    }
}

fn handle_stream_output_toggle(button: &Button, nav: &NavigationContext) {
    let active = nav.state.borrow().stream_status.active;
    let command = if active {
        AppCommand::StopStreaming
    } else {
        AppCommand::StartStreaming
    };
    let should_confirm = requires_output_confirmation(
        OutputKind::Stream,
        active,
        &nav.state.borrow().output_confirmations,
    );
    if should_confirm {
        let action = output_action_for_active_state(active);
        let dialog = output_confirmation_dialog(OutputKind::Stream, action);
        confirm_output_action(button, dialog, command, nav.clone());
    } else {
        nav.dispatch(command);
    }
}

fn handle_record_output_toggle(button: &Button, nav: &NavigationContext) {
    let active = nav.state.borrow().record_status.active;
    let command = if active {
        AppCommand::StopRecording
    } else {
        AppCommand::StartRecording
    };
    let should_confirm = requires_output_confirmation(
        OutputKind::Recording,
        active,
        &nav.state.borrow().output_confirmations,
    );
    if should_confirm {
        let action = output_action_for_active_state(active);
        let dialog = output_confirmation_dialog(OutputKind::Recording, action);
        confirm_output_action(button, dialog, command, nav.clone());
    } else {
        nav.dispatch(command);
    }
}

pub(crate) fn show_live_view(handle: &LivePageHandle) {
    handle.root.set_visible_child_name("live");
}

pub(crate) fn update_stream_status(
    handle: &LivePageHandle,
    status: &OutputStatus,
    elapsed: Option<String>,
    error: Option<&str>,
) {
    handle.stream_label.set_text(&output_label(
        &fl!(LANGUAGE_LOADER, "live-output-kind-stream"),
        status,
        elapsed.as_deref(),
    ));
    update_output_progress(&handle.stream_progress_label, OutputKind::Stream, status);
    update_output_error(&handle.stream_error_label, OutputKind::Stream, error);
    set_output_button(
        &handle.stream_btn,
        status,
        &fl!(LANGUAGE_LOADER, "live-start-stream-label"),
        &fl!(LANGUAGE_LOADER, "live-stop-stream-label"),
    );
}

pub(crate) fn update_record_status(
    handle: &LivePageHandle,
    status: &OutputStatus,
    elapsed: Option<String>,
    last_path: Option<&str>,
    error: Option<&str>,
) {
    handle.record_label.set_text(&output_label(
        &fl!(LANGUAGE_LOADER, "live-output-kind-record"),
        status,
        elapsed.as_deref(),
    ));
    update_output_progress(&handle.record_progress_label, OutputKind::Recording, status);
    update_output_error(&handle.record_error_label, OutputKind::Recording, error);
    handle.record_label.set_tooltip_text(last_path);
    update_recording_path_detail(&handle.record_path_label, last_path);
    handle.record_path_btn.set_sensitive(last_path.is_some());
    if let Some(path) = last_path {
        handle.record_path_btn.set_tooltip_text(Some(&fl!(
            LANGUAGE_LOADER,
            "live-copy-recording-path-with-value-tooltip",
            path = path
        )));
    } else {
        handle.record_path_btn.set_tooltip_text(Some(&fl!(
            LANGUAGE_LOADER,
            "live-copy-last-recording-path-tooltip"
        )));
    }
    set_output_button(
        &handle.record_btn,
        status,
        &fl!(LANGUAGE_LOADER, "live-start-record-label"),
        &fl!(LANGUAGE_LOADER, "live-stop-record-label"),
    );
}

pub(crate) fn reset_output_controls(handle: &LivePageHandle) {
    handle
        .stream_label
        .set_text(&fl!(LANGUAGE_LOADER, "live-stream-inactive-label"));
    handle
        .record_label
        .set_text(&fl!(LANGUAGE_LOADER, "live-record-inactive-label"));
    update_output_progress(
        &handle.stream_progress_label,
        OutputKind::Stream,
        &OutputStatus::default(),
    );
    update_output_progress(
        &handle.record_progress_label,
        OutputKind::Recording,
        &OutputStatus::default(),
    );
    update_output_error(&handle.stream_error_label, OutputKind::Stream, None);
    update_output_error(&handle.record_error_label, OutputKind::Recording, None);
    handle.record_label.set_tooltip_text(None);
    update_recording_path_detail(&handle.record_path_label, None);
    handle.record_path_btn.set_sensitive(false);
    handle.record_path_btn.set_tooltip_text(Some(&fl!(
        LANGUAGE_LOADER,
        "live-copy-last-recording-path-tooltip"
    )));
    handle
        .stream_btn
        .set_label(&fl!(LANGUAGE_LOADER, "live-start-stream-label"));
    handle
        .record_btn
        .set_label(&fl!(LANGUAGE_LOADER, "live-start-record-label"));
    handle.stream_btn.set_sensitive(false);
    handle.record_btn.set_sensitive(false);
    handle.stream_btn.remove_css_class("destructive-action");
    handle.record_btn.remove_css_class("destructive-action");
}

fn build_output_card(
    title: &str,
    button: &Button,
    label: &Label,
    progress_label: &Label,
    error_label: &Label,
    detail_label: Option<&Label>,
    suffix: Option<&Button>,
) -> GtkBox {
    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .valign(Align::Center)
        .hexpand(true)
        .build();
    card.add_css_class("output-control");
    card.add_css_class("output-card");

    let title_label = Label::builder().label(title).xalign(0.0).build();
    title_label.add_css_class("caption-heading");
    title_label.add_css_class("output-card-title");
    card.append(&title_label);

    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .valign(Align::Center)
        .build();
    row.append(button);
    row.append(label);
    if let Some(suffix) = suffix {
        row.append(suffix);
    }
    card.append(&row);
    card.append(progress_label);
    card.append(error_label);
    if let Some(detail_label) = detail_label {
        card.append(detail_label);
    }
    card
}

fn build_output_progress_label() -> Label {
    let label = Label::builder()
        .label(EMPTY_OUTPUT_SLOT)
        .xalign(0.0)
        .wrap(true)
        .visible(false)
        .build();
    label.add_css_class("caption");
    label.add_css_class("dim-label");
    label.add_css_class("output-progress");
    label
}

fn build_output_detail_label() -> Label {
    let label = Label::builder()
        .label(EMPTY_OUTPUT_SLOT)
        .xalign(0.0)
        .wrap(true)
        .visible(false)
        .build();
    label.add_css_class("caption");
    label.add_css_class("dim-label");
    label.add_css_class("output-detail");
    label
}

fn build_output_error_label() -> Label {
    let label = Label::builder()
        .label(EMPTY_OUTPUT_SLOT)
        .xalign(0.0)
        .wrap(true)
        .visible(false)
        .build();
    label.add_css_class("caption");
    label.add_css_class("output-command-error");
    label
}

fn update_output_progress(label: &Label, kind: OutputKind, status: &OutputStatus) {
    set_optional_output_text(label, &output_progress_copy(kind, status));
}

fn update_output_error(label: &Label, kind: OutputKind, error: Option<&str>) {
    if let Some(display) = output_command_error_display(kind, error) {
        set_optional_output_text(label, &display.label);
        label.set_tooltip_text(Some(display.tooltip));
    } else {
        set_optional_output_text(label, EMPTY_OUTPUT_SLOT);
        label.set_tooltip_text(None);
    }
}

fn update_recording_path_detail(label: &Label, last_path: Option<&str>) {
    if let Some(display) = output_recording_path_display(last_path) {
        set_optional_output_text(label, &display.label);
        label.set_tooltip_text(Some(display.tooltip));
    } else {
        set_optional_output_text(label, EMPTY_OUTPUT_SLOT);
        label.set_tooltip_text(None);
    }
}

fn set_optional_output_text(label: &Label, text: &str) {
    label.set_text(text);
    label.set_visible(!text.is_empty());
}

fn output_command_error_display(
    kind: OutputKind,
    error: Option<&str>,
) -> Option<OutputCommandErrorDisplay<'_>> {
    let tooltip = error.filter(|error| !error.is_empty())?;
    let label = match kind {
        OutputKind::Stream => fl!(LANGUAGE_LOADER, "live-stream-command-error-label"),
        OutputKind::Recording => fl!(LANGUAGE_LOADER, "live-recording-command-error-label"),
    };

    Some(OutputCommandErrorDisplay { label, tooltip })
}

fn output_recording_path_display(path: Option<&str>) -> Option<OutputRecordingPathDisplay<'_>> {
    let tooltip = path.filter(|path| !path.is_empty())?;
    Some(OutputRecordingPathDisplay {
        label: fl!(
            LANGUAGE_LOADER,
            "live-last-recording-detail",
            path = tooltip
        ),
        tooltip,
    })
}

fn output_progress_copy(kind: OutputKind, status: &OutputStatus) -> String {
    match (kind, status.state) {
        (OutputKind::Stream, crate::domain::output::OutputRunState::Starting) => {
            fl!(LANGUAGE_LOADER, "live-starting-stream")
        }
        (OutputKind::Stream, crate::domain::output::OutputRunState::Stopping) => {
            fl!(LANGUAGE_LOADER, "live-stopping-stream")
        }
        (OutputKind::Stream, crate::domain::output::OutputRunState::Reconnecting) => {
            fl!(LANGUAGE_LOADER, "live-reconnecting-stream")
        }
        (OutputKind::Recording, crate::domain::output::OutputRunState::Starting) => {
            fl!(LANGUAGE_LOADER, "live-starting-recording")
        }
        (OutputKind::Recording, crate::domain::output::OutputRunState::Stopping) => {
            fl!(LANGUAGE_LOADER, "live-stopping-recording")
        }
        (OutputKind::Recording, crate::domain::output::OutputRunState::Reconnecting) => {
            fl!(LANGUAGE_LOADER, "live-reconnecting-recording")
        }
        _ => EMPTY_OUTPUT_SLOT.to_string(),
    }
}

fn build_disconnected_view() -> GtkBox {
    let view = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .valign(Align::Center)
        .halign(Align::Center)
        .vexpand(true)
        .hexpand(true)
        .build();
    view.add_css_class("live-disconnected-view");

    let title = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-disconnected-title"))
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();
    title.add_css_class("title-2");

    let detail = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-disconnected-detail"))
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();
    detail.add_css_class("dim-label");

    view.append(&title);
    view.append(&detail);
    view
}

fn set_output_button(button: &Button, status: &OutputStatus, start_label: &str, stop_label: &str) {
    if status.state.is_transitioning() {
        let transition_label = match status.state {
            crate::domain::output::OutputRunState::Starting => {
                fl!(LANGUAGE_LOADER, "live-button-starting")
            }
            crate::domain::output::OutputRunState::Stopping => {
                fl!(LANGUAGE_LOADER, "live-button-stopping")
            }
            crate::domain::output::OutputRunState::Reconnecting => {
                fl!(LANGUAGE_LOADER, "live-button-reconnecting")
            }
            _ => fl!(LANGUAGE_LOADER, "live-button-working"),
        };
        button.set_label(&transition_label);
        button.set_sensitive(false);
        if status.active {
            button.add_css_class("destructive-action");
        } else {
            button.remove_css_class("destructive-action");
        }
    } else if status.active {
        button.set_label(stop_label);
        button.set_sensitive(true);
        button.add_css_class("destructive-action");
    } else {
        button.set_label(start_label);
        button.set_sensitive(true);
        button.remove_css_class("destructive-action");
    }
}

pub(crate) fn output_label(kind: &str, status: &OutputStatus, elapsed: Option<&str>) -> String {
    match elapsed {
        Some(elapsed) if status.active => fl!(
            LANGUAGE_LOADER,
            "live-output-label-with-elapsed",
            kind = kind,
            state = status.state.label(),
            elapsed = elapsed
        ),
        _ => fl!(
            LANGUAGE_LOADER,
            "live-output-label",
            kind = kind,
            state = status.state.label()
        ),
    }
}

fn requires_output_confirmation(kind: OutputKind, active: bool, config: &OutputConfig) -> bool {
    match (kind, active) {
        (OutputKind::Stream, false) => config.confirm_start_stream,
        (OutputKind::Stream, true) => config.confirm_stop_stream,
        (OutputKind::Recording, false) => config.confirm_start_recording,
        (OutputKind::Recording, true) => config.confirm_stop_recording,
    }
}

fn output_action_for_active_state(active: bool) -> OutputAction {
    if active {
        OutputAction::Stop
    } else {
        OutputAction::Start
    }
}

fn output_confirmation_dialog(kind: OutputKind, action: OutputAction) -> OutputConfirmationDialog {
    match (kind, action) {
        (OutputKind::Stream, OutputAction::Start) => OutputConfirmationDialog {
            heading: fl!(LANGUAGE_LOADER, "live-start-stream-confirm-heading"),
            body: fl!(LANGUAGE_LOADER, "live-start-stream-confirm-body"),
            confirm_label: fl!(LANGUAGE_LOADER, "live-start-stream-label"),
            appearance: OutputConfirmationAppearance::Suggested,
        },
        (OutputKind::Stream, OutputAction::Stop) => OutputConfirmationDialog {
            heading: fl!(LANGUAGE_LOADER, "live-stop-stream-confirm-heading"),
            body: fl!(LANGUAGE_LOADER, "live-stop-stream-confirm-body"),
            confirm_label: fl!(LANGUAGE_LOADER, "live-stop-stream-label"),
            appearance: OutputConfirmationAppearance::Destructive,
        },
        (OutputKind::Recording, OutputAction::Start) => OutputConfirmationDialog {
            heading: fl!(LANGUAGE_LOADER, "live-start-recording-confirm-heading"),
            body: fl!(LANGUAGE_LOADER, "live-start-recording-confirm-body"),
            confirm_label: fl!(LANGUAGE_LOADER, "live-start-recording-confirm-label"),
            appearance: OutputConfirmationAppearance::Suggested,
        },
        (OutputKind::Recording, OutputAction::Stop) => OutputConfirmationDialog {
            heading: fl!(LANGUAGE_LOADER, "live-stop-recording-confirm-heading"),
            body: fl!(LANGUAGE_LOADER, "live-stop-recording-confirm-body"),
            confirm_label: fl!(LANGUAGE_LOADER, "live-stop-recording-confirm-label"),
            appearance: OutputConfirmationAppearance::Destructive,
        },
    }
}

fn to_adw_response_appearance(appearance: OutputConfirmationAppearance) -> adw::ResponseAppearance {
    match appearance {
        OutputConfirmationAppearance::Suggested => adw::ResponseAppearance::Suggested,
        OutputConfirmationAppearance::Destructive => adw::ResponseAppearance::Destructive,
    }
}

fn confirm_output_action(
    parent: &impl IsA<gtk4::Widget>,
    metadata: OutputConfirmationDialog,
    command: AppCommand,
    nav: NavigationContext,
) {
    let parent_window = parent
        .root()
        .and_then(|root| root.downcast::<gtk4::Window>().ok());
    let dialog = adw::MessageDialog::new(
        parent_window.as_ref(),
        Some(metadata.heading.as_str()),
        Some(metadata.body.as_str()),
    );
    dialog.add_response("cancel", &fl!(LANGUAGE_LOADER, "live-cancel-button-label"));
    dialog.add_response("confirm", &metadata.confirm_label);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("confirm", to_adw_response_appearance(metadata.appearance));
    dialog.connect_response(None, move |dialog, response| {
        if response == "confirm" {
            nav.dispatch(command.clone());
        }
        dialog.close();
    });
    dialog.present();
}

fn insert_scene_placeholder(scenes_box: &FlowBox, message: &str) {
    let hint = Label::builder()
        .label(message)
        .wrap(true)
        .xalign(0.0)
        .build();
    hint.add_css_class("dim-label");
    insert_compact_flow_child(scenes_box, &hint);
}

/// Rebuild the scene cards from the current inventory.
///
/// Called by `ui::window::apply_event` whenever the inventory changes.
pub(crate) fn rebuild_scene_cards(
    handle: &LivePageHandle,
    inventory: &SceneInventory,
    nav: &NavigationContext,
) {
    // Remove all existing children
    // FlowBox requires removing via the FlowBoxChild wrapper
    while let Some(child) = handle.scenes_box.first_child() {
        handle.scenes_box.remove(&child);
    }

    let registry = read_registry();

    let primary_scenes: Vec<_> = inventory
        .scenes
        .iter()
        .filter(|s| {
            registry
                .scenes
                .get(&s.id)
                .map(|e| e.role.is_live_switchable())
                .unwrap_or(false)
        })
        .collect();

    if primary_scenes.is_empty() {
        let hint = Label::builder()
            .label(fl!(LANGUAGE_LOADER, "live-scenes-no-primary-hint"))
            .wrap(true)
            .xalign(0.0)
            .build();
        hint.add_css_class("dim-label");
        insert_compact_flow_child(&handle.scenes_box, &hint);
        return;
    }

    for scene in primary_scenes {
        let is_active = inventory.current_id.as_deref() == Some(&scene.id);
        let is_previous = inventory.previous_id.as_deref() == Some(&scene.id);
        let scene_role = registry
            .scenes
            .get(&scene.id)
            .map(|entry| entry.role)
            .unwrap_or_default();
        let card = scene_card::build(
            &scene.name,
            scene.id.clone(),
            scene_role,
            is_active,
            is_previous,
            nav.clone(),
        );
        insert_compact_flow_child(&handle.scenes_box, &card);
    }
}

/// Rebuild the audio mixer cards from the given input list.
///
/// Called by `ui::window::apply_event` whenever `AudioInputsUpdated` arrives
/// or the connection is reset.
pub(crate) fn rebuild_audio_cards(
    handle: &LivePageHandle,
    inputs: &[crate::domain::audio::AudioInput],
    nav: &NavigationContext,
) {
    while let Some(child) = handle.audio_box.first_child() {
        handle.audio_box.remove(&child);
    }
    let mut cards = handle.audio_cards.borrow_mut();
    cards.clear();

    if inputs.is_empty() {
        let hint = Label::builder()
            .label(fl!(LANGUAGE_LOADER, "live-audio-empty-hint"))
            .xalign(0.0)
            .build();
        hint.add_css_class("dim-label");
        insert_compact_flow_child(&handle.audio_box, &hint);
        return;
    }

    for input in inputs {
        let card = audio_card::build(input, nav.clone());
        insert_compact_flow_child(&handle.audio_box, &card.root);
        cards.push(card);
    }
}

fn insert_compact_flow_child<W: IsA<gtk4::Widget>>(flow: &FlowBox, widget: &W) {
    let child = FlowBoxChild::new();
    child.set_halign(Align::Start);
    child.set_valign(Align::Start);
    child.set_hexpand(false);
    child.set_vexpand(false);
    child.set_child(Some(widget));
    flow.insert(&child, -1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::output::OutputRunState;

    fn output_status(active: bool, state: OutputRunState) -> OutputStatus {
        OutputStatus {
            active,
            state,
            detail: None,
        }
    }

    #[test]
    fn output_confirmation_defaults_match_output_actions() {
        let config = OutputConfig::default();

        assert!(!requires_output_confirmation(
            OutputKind::Stream,
            false,
            &config
        ));
        assert!(requires_output_confirmation(
            OutputKind::Stream,
            true,
            &config
        ));
        assert!(!requires_output_confirmation(
            OutputKind::Recording,
            false,
            &config
        ));
        assert!(requires_output_confirmation(
            OutputKind::Recording,
            true,
            &config
        ));
    }

    #[test]
    fn output_confirmation_honors_disabled_stop_preferences() {
        let config = OutputConfig {
            confirm_stop_stream: false,
            confirm_stop_recording: false,
            ..OutputConfig::default()
        };

        assert!(!requires_output_confirmation(
            OutputKind::Stream,
            true,
            &config
        ));
        assert!(!requires_output_confirmation(
            OutputKind::Recording,
            true,
            &config
        ));
    }

    #[test]
    fn output_confirmation_honors_enabled_start_preferences() {
        let config = OutputConfig {
            confirm_start_stream: true,
            confirm_start_recording: true,
            ..OutputConfig::default()
        };

        assert!(requires_output_confirmation(
            OutputKind::Stream,
            false,
            &config
        ));
        assert!(requires_output_confirmation(
            OutputKind::Recording,
            false,
            &config
        ));
    }

    #[test]
    fn stream_start_confirmation_metadata_is_suggested() {
        assert_eq!(
            output_confirmation_dialog(OutputKind::Stream, OutputAction::Start),
            OutputConfirmationDialog {
                heading: "Start Stream?".to_string(),
                body: "OBS will start sending the live stream.".to_string(),
                confirm_label: "Start Stream".to_string(),
                appearance: OutputConfirmationAppearance::Suggested,
            }
        );
    }

    #[test]
    fn stream_stop_confirmation_metadata_is_destructive() {
        assert_eq!(
            output_confirmation_dialog(OutputKind::Stream, OutputAction::Stop),
            OutputConfirmationDialog {
                heading: "Stop Stream?".to_string(),
                body: "OBS will stop sending the live stream.".to_string(),
                confirm_label: "Stop Stream".to_string(),
                appearance: OutputConfirmationAppearance::Destructive,
            }
        );
    }

    #[test]
    fn recording_start_confirmation_metadata_is_suggested() {
        assert_eq!(
            output_confirmation_dialog(OutputKind::Recording, OutputAction::Start),
            OutputConfirmationDialog {
                heading: "Start Recording?".to_string(),
                body: "OBS will start a new recording.".to_string(),
                confirm_label: "Start Recording".to_string(),
                appearance: OutputConfirmationAppearance::Suggested,
            }
        );
    }

    #[test]
    fn recording_stop_confirmation_metadata_is_destructive() {
        assert_eq!(
            output_confirmation_dialog(OutputKind::Recording, OutputAction::Stop),
            OutputConfirmationDialog {
                heading: "Stop Recording?".to_string(),
                body: "OBS will stop the current recording.".to_string(),
                confirm_label: "Stop Recording".to_string(),
                appearance: OutputConfirmationAppearance::Destructive,
            }
        );
    }

    #[test]
    fn active_state_maps_to_output_action() {
        assert_eq!(output_action_for_active_state(false), OutputAction::Start);
        assert_eq!(output_action_for_active_state(true), OutputAction::Stop);
    }

    #[test]
    fn stream_command_error_display_uses_concise_label_and_raw_tooltip() {
        assert_eq!(
            output_command_error_display(OutputKind::Stream, Some("OBS said no: backend details")),
            Some(OutputCommandErrorDisplay {
                label: "Stream command failed".to_string(),
                tooltip: "OBS said no: backend details",
            })
        );
    }

    #[test]
    fn recording_command_error_display_uses_concise_label_and_raw_tooltip() {
        assert_eq!(
            output_command_error_display(
                OutputKind::Recording,
                Some("recording output unavailable")
            ),
            Some(OutputCommandErrorDisplay {
                label: "Recording command failed".to_string(),
                tooltip: "recording output unavailable",
            })
        );
    }

    #[test]
    fn command_error_display_ignores_absent_or_empty_errors() {
        assert_eq!(output_command_error_display(OutputKind::Stream, None), None);
        assert_eq!(
            output_command_error_display(OutputKind::Recording, Some("")),
            None
        );
    }

    #[test]
    fn output_progress_copy_reports_stream_pending_states() {
        assert_eq!(
            output_progress_copy(
                OutputKind::Stream,
                &output_status(false, OutputRunState::Starting)
            ),
            "Starting stream…"
        );
        assert_eq!(
            output_progress_copy(
                OutputKind::Stream,
                &output_status(true, OutputRunState::Stopping)
            ),
            "Stopping stream…"
        );
        assert_eq!(
            output_progress_copy(
                OutputKind::Stream,
                &output_status(true, OutputRunState::Reconnecting)
            ),
            "Reconnecting stream…"
        );
    }

    #[test]
    fn output_progress_copy_reports_recording_pending_states() {
        assert_eq!(
            output_progress_copy(
                OutputKind::Recording,
                &output_status(false, OutputRunState::Starting)
            ),
            "Starting recording…"
        );
        assert_eq!(
            output_progress_copy(
                OutputKind::Recording,
                &output_status(true, OutputRunState::Stopping)
            ),
            "Stopping recording…"
        );
        assert_eq!(
            output_progress_copy(
                OutputKind::Recording,
                &output_status(true, OutputRunState::Reconnecting)
            ),
            "Reconnecting recording…"
        );
    }

    #[test]
    fn output_progress_copy_is_empty_for_inactive_states() {
        for kind in [OutputKind::Stream, OutputKind::Recording] {
            for state in [
                OutputRunState::Inactive,
                OutputRunState::Active,
                OutputRunState::Paused,
                OutputRunState::Unknown,
            ] {
                assert_eq!(
                    output_progress_copy(
                        kind,
                        &output_status(state == OutputRunState::Active, state)
                    ),
                    EMPTY_OUTPUT_SLOT
                );
            }
        }
    }

    #[test]
    fn output_label_shows_elapsed_time_only_while_active() {
        assert_eq!(
            output_label(
                "Stream",
                &output_status(true, OutputRunState::Active),
                Some("00:01:23")
            ),
            "Stream: Active · 00:01:23"
        );
        assert_eq!(
            output_label(
                "Record",
                &output_status(false, OutputRunState::Inactive),
                Some("00:01:23")
            ),
            "Record: Inactive"
        );
        assert_eq!(
            output_label("Stream", &output_status(true, OutputRunState::Active), None),
            "Stream: Active"
        );
    }

    #[test]
    fn recording_path_display_uses_visible_copy_and_raw_tooltip() {
        assert_eq!(
            output_recording_path_display(Some("/tmp/scenedeck-output.mkv")),
            Some(OutputRecordingPathDisplay {
                label: "Last recording: /tmp/scenedeck-output.mkv".to_string(),
                tooltip: "/tmp/scenedeck-output.mkv",
            })
        );
    }

    #[test]
    fn recording_path_display_ignores_absent_or_empty_paths() {
        assert_eq!(output_recording_path_display(None), None);
        assert_eq!(output_recording_path_display(Some("")), None);
    }
}
