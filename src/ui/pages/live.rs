//! Live page — primary daily-operation surface.
//!
//! Returns a `LivePageHandle` so `ui::window` can push state updates into the
//! widgets without rebuilding the page.

use adw::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, FlowBox, Label, Orientation, Paned, PolicyType, ScrolledWindow,
    Stack, StackTransitionType,
};

use crate::controller::command::AppCommand;
use crate::domain::output::OutputStatus;
use crate::domain::scene::SceneInventory;
use crate::storage::registry::read_registry;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::{audio_card, scene_card};

/// Widget handles that `ui::window` updates when `AppEvent`s arrive.
pub(crate) struct LivePageHandle {
    pub(crate) root: Stack,
    pub(crate) stream_label: Label,
    pub(crate) stream_btn: Button,
    pub(crate) record_label: Label,
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
        .margin_top(10)
        .margin_bottom(10)
        .hexpand(true)
        .build();
    banner.append(&banner_inner);

    let stream_btn = Button::builder()
        .label("Start Stream")
        .valign(Align::Center)
        .sensitive(false)
        .build();
    stream_btn.set_tooltip_text(Some("Start or stop streaming"));
    stream_btn.connect_clicked({
        let nav = nav.clone();
        move |button| {
            let active = nav.state.borrow().stream_status.active;
            let command = if active {
                AppCommand::StopStreaming
            } else {
                AppCommand::StartStreaming
            };
            let should_confirm = {
                let state = nav.state.borrow();
                if active {
                    state.output_confirmations.confirm_stop_stream
                } else {
                    state.output_confirmations.confirm_start_stream
                }
            };
            if should_confirm {
                confirm_output_action(
                    button,
                    if active {
                        "Stop Stream?"
                    } else {
                        "Start Stream?"
                    },
                    if active {
                        "OBS will stop sending the live stream."
                    } else {
                        "OBS will start sending the live stream."
                    },
                    if active {
                        "Stop Stream"
                    } else {
                        "Start Stream"
                    },
                    command,
                    nav.clone(),
                );
            } else {
                nav.dispatch(command);
            }
        }
    });

    let stream_label = Label::builder()
        .label("Stream: Inactive")
        .xalign(0.0)
        .build();
    stream_label.add_css_class("caption");
    stream_label.add_css_class("dim-label");

    let record_btn = Button::builder()
        .label("Start Record")
        .valign(Align::Center)
        .sensitive(false)
        .build();
    record_btn.set_tooltip_text(Some("Start or stop recording"));
    record_btn.connect_clicked({
        let nav = nav.clone();
        move |button| {
            let active = nav.state.borrow().record_status.active;
            let command = if active {
                AppCommand::StopRecording
            } else {
                AppCommand::StartRecording
            };
            let should_confirm = {
                let state = nav.state.borrow();
                if active {
                    state.output_confirmations.confirm_stop_recording
                } else {
                    state.output_confirmations.confirm_start_recording
                }
            };
            if should_confirm {
                confirm_output_action(
                    button,
                    if active {
                        "Stop Recording?"
                    } else {
                        "Start Recording?"
                    },
                    if active {
                        "OBS will stop the current recording."
                    } else {
                        "OBS will start a new recording."
                    },
                    if active {
                        "Stop Recording"
                    } else {
                        "Start Recording"
                    },
                    command,
                    nav.clone(),
                );
            } else {
                nav.dispatch(command);
            }
        }
    });

    let record_label = Label::builder()
        .label("Record: Inactive")
        .xalign(0.0)
        .build();
    record_label.add_css_class("caption");
    record_label.add_css_class("dim-label");

    let record_path_btn = Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy last recording path")
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
                button.set_tooltip_text(Some("Copied last recording path"));
            }
        }
    });

    let stream_control = build_output_control(&stream_btn, &stream_label, None);
    let record_control = build_output_control(&record_btn, &record_label, Some(&record_path_btn));

    banner_inner.append(&stream_control);
    banner_inner.append(&record_control);
    page.append(&banner);

    // ── Program scene label ───────────────────────────────────────────────────
    let current_label = Label::builder()
        .label("Current scene: —")
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
    live_split.set_resize_start_child(true);
    live_split.set_resize_end_child(true);
    live_split.set_shrink_start_child(false);
    live_split.set_shrink_end_child(false);
    page.append(&live_split);

    // ── Scene cards ───────────────────────────────────────────────────────────
    let scenes_pane = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .vexpand(true)
        .hexpand(true)
        .build();
    let scenes_section_label = Label::builder().label("Scenes").xalign(0.0).build();
    scenes_section_label.add_css_class("caption-heading");
    scenes_pane.append(&scenes_section_label);

    let scenes_box = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(12)
        .row_spacing(12)
        .homogeneous(false)
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(3)
        .margin_end(3)
        .min_children_per_line(1)
        .max_children_per_line(4)
        .build();
    insert_scene_placeholder(&scenes_box, "Connect to OBS to load scenes.");

    let scenes_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .min_content_height(160)
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

    let audio_section_label = Label::builder().label("Audio").xalign(0.0).build();
    audio_section_label.add_css_class("caption-heading");
    audio_pane.append(&audio_section_label);

    let audio_box = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(8)
        .row_spacing(8)
        .homogeneous(false)
        .min_children_per_line(1)
        .max_children_per_line(10)
        .build();

    let audio_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .min_content_height(160)
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
        stream_btn,
        record_label,
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

pub(crate) fn show_live_view(handle: &LivePageHandle) {
    handle.root.set_visible_child_name("live");
}

pub(crate) fn update_stream_status(
    handle: &LivePageHandle,
    status: &OutputStatus,
    elapsed: Option<String>,
) {
    handle
        .stream_label
        .set_text(&output_label("Stream", status, elapsed.as_deref()));
    set_output_button(&handle.stream_btn, status, "Start Stream", "Stop Stream");
}

pub(crate) fn update_record_status(
    handle: &LivePageHandle,
    status: &OutputStatus,
    elapsed: Option<String>,
    last_path: Option<&str>,
) {
    handle
        .record_label
        .set_text(&output_label("Record", status, elapsed.as_deref()));
    handle.record_label.set_tooltip_text(last_path);
    handle.record_path_btn.set_sensitive(last_path.is_some());
    if let Some(path) = last_path {
        handle
            .record_path_btn
            .set_tooltip_text(Some(&format!("Copy recording path: {path}")));
    } else {
        handle
            .record_path_btn
            .set_tooltip_text(Some("Copy last recording path"));
    }
    set_output_button(&handle.record_btn, status, "Start Record", "Stop Record");
}

pub(crate) fn reset_output_controls(handle: &LivePageHandle) {
    handle.stream_label.set_text("Stream: Inactive");
    handle.record_label.set_text("Record: Inactive");
    handle.record_label.set_tooltip_text(None);
    handle.record_path_btn.set_sensitive(false);
    handle
        .record_path_btn
        .set_tooltip_text(Some("Copy last recording path"));
    handle.stream_btn.set_label("Start Stream");
    handle.record_btn.set_label("Start Record");
    handle.stream_btn.set_sensitive(false);
    handle.record_btn.set_sensitive(false);
    handle.stream_btn.remove_css_class("destructive-action");
    handle.record_btn.remove_css_class("destructive-action");
}

fn build_output_control(button: &Button, label: &Label, suffix: Option<&Button>) -> GtkBox {
    let control = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .valign(Align::Center)
        .build();
    control.add_css_class("output-control");
    control.append(button);
    control.append(label);
    if let Some(suffix) = suffix {
        control.append(suffix);
    }
    control
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
        .label("Connect to OBS to use Live controls")
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();
    title.add_css_class("title-2");

    let detail = Label::builder()
        .label("Use the connection control at the bottom of the sidebar.")
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
        button.set_label(match status.state {
            crate::domain::output::OutputRunState::Starting => "Starting…",
            crate::domain::output::OutputRunState::Stopping => "Stopping…",
            crate::domain::output::OutputRunState::Reconnecting => "Reconnecting…",
            _ => "Working…",
        });
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

fn output_label(kind: &str, status: &OutputStatus, elapsed: Option<&str>) -> String {
    match elapsed {
        Some(elapsed) if status.active => format!("{kind}: {} · {elapsed}", status.state.label()),
        _ => format!("{kind}: {}", status.state.label()),
    }
}

fn confirm_output_action(
    parent: &impl IsA<gtk4::Widget>,
    heading: &str,
    body: &str,
    confirm_label: &str,
    command: AppCommand,
    nav: NavigationContext,
) {
    let parent_window = parent
        .root()
        .and_then(|root| root.downcast::<gtk4::Window>().ok());
    let dialog = adw::MessageDialog::new(parent_window.as_ref(), Some(heading), Some(body));
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("confirm", confirm_label);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
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
    scenes_box.insert(&hint, -1);
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
            .label("No Primary-role scenes found. Assign roles in Inventory.")
            .wrap(true)
            .xalign(0.0)
            .build();
        hint.add_css_class("dim-label");
        handle.scenes_box.insert(&hint, -1);
        return;
    }

    for scene in primary_scenes {
        let is_active = inventory.current_id.as_deref() == Some(&scene.id);
        let card = scene_card::build(&scene.name, scene.id.clone(), is_active, nav.clone());
        handle.scenes_box.insert(&card, -1);
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
            .label("No audio inputs configured.")
            .xalign(0.0)
            .build();
        hint.add_css_class("dim-label");
        handle.audio_box.insert(&hint, -1);
        return;
    }

    for input in inputs {
        let card = audio_card::build(input, nav.clone());
        handle.audio_box.insert(&card.root, -1);
        cards.push(card);
    }
}
