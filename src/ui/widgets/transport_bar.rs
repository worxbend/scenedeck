//! Compact Live outputs; contextual operations preserve scoped OBS safeguards.

use crate::controller::state::{AppState, ObsStatus, Page};
use crate::domain::{
    control::ControlAction,
    output::{OutputKind, OutputRunState, OutputStatus},
};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::{navigation::NavigationContext, workspace};
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, FlowBox, MenuButton, Orientation, Popover};
use i18n_embed_fl::fl;
use std::time::Duration;

fn pause_action(record: &OutputStatus) -> Option<bool> {
    if !record.active {
        return None;
    }
    match record.state {
        OutputRunState::Active => Some(true),
        OutputRunState::Paused => Some(false),
        _ => None,
    }
}

#[derive(Clone)]
struct Controls {
    stream: Button,
    record: Button,
    replay: Button,
    camera: Button,
    pause: Button,
    save: Button,
    split: Button,
    chapter: Button,
    chapter_name: Entry,
}

pub(crate) fn build(nav: NavigationContext) -> gtk4::Widget {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.add_css_class("transport-bar");
    root.add_css_class("output-toolbar");
    let actions = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .homogeneous(true)
        .min_children_per_line(1)
        .max_children_per_line(4)
        .row_spacing(6)
        .column_spacing(8)
        .hexpand(true)
        .build();
    let controls = Controls {
        stream: Button::with_label(""),
        record: Button::with_label(""),
        replay: Button::with_label(""),
        camera: Button::with_label(""),
        pause: Button::with_label(&fl!(LANGUAGE_LOADER, "control-pause")),
        save: Button::with_label(&fl!(LANGUAGE_LOADER, "control-replay-save")),
        split: Button::with_label(&fl!(LANGUAGE_LOADER, "control-split")),
        chapter: Button::with_label(&fl!(LANGUAGE_LOADER, "control-chapter")),
        chapter_name: Entry::builder()
            .placeholder_text(fl!(LANGUAGE_LOADER, "control-chapter-name"))
            .build(),
    };
    for (name, button) in [
        ("stream", &controls.stream),
        ("record", &controls.record),
        ("replay", &controls.replay),
        ("camera", &controls.camera),
    ] {
        button.set_widget_name(&format!("output-{name}"));
        button.set_hexpand(true);
        if let Some(label) = button.child().and_downcast::<gtk4::Label>() {
            label.set_wrap(true);
            label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            label.set_max_width_chars(14);
            label.set_justify(gtk4::Justification::Center);
        }
        actions.insert(button, -1);
        if let Some(wrapper) = button.parent() {
            wrapper.set_focusable(false);
        }
    }
    root.append(&actions);
    controls.stream.connect_clicked({
        let nav = nav.clone();
        move |button| crate::ui::pages::live::handle_stream_output_toggle(button, &nav)
    });
    controls.record.connect_clicked({
        let nav = nav.clone();
        move |button| crate::ui::pages::live::handle_record_output_toggle(button, &nav)
    });
    controls.replay.connect_clicked({
        let nav = nav.clone();
        move |_| {
            let active = nav
                .state
                .borrow()
                .workspace
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.replay_active);
            if let Some(active) = active {
                workspace::act(&nav, ControlAction::SetReplayActive(!active));
            }
        }
    });
    controls.camera.connect_clicked({
        let nav = nav.clone();
        move |_| {
            let active = nav
                .state
                .borrow()
                .workspace
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.virtual_camera_active);
            if let Some(active) = active {
                workspace::act(&nav, ControlAction::SetVirtualCamera(!active));
            }
        }
    });
    controls.pause.connect_clicked({
        let nav = nav.clone();
        move |_| {
            let paused = pause_action(&nav.state.borrow().record_status);
            if let Some(paused) = paused {
                workspace::act(&nav, ControlAction::SetRecordingPaused(paused));
            }
        }
    });
    for (button, action) in [
        (&controls.save, ControlAction::SaveReplay),
        (&controls.split, ControlAction::SplitRecording),
    ] {
        button.connect_clicked({
            let nav = nav.clone();
            move |_| workspace::act(&nav, action.clone())
        });
    }
    let chapter_name = controls.chapter_name.clone();
    chapter_name.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "control-chapter-name"
    ))]);
    controls.chapter.connect_clicked({
        let nav = nav.clone();
        let name = chapter_name.clone();
        move |_| {
            let title = name.text().trim().to_string();
            if !title.is_empty() {
                workspace::act(&nav, ControlAction::RecordChapter(title));
            }
        }
    });
    let content = GtkBox::new(Orientation::Vertical, 8);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    for button in [&controls.pause, &controls.save, &controls.split] {
        content.append(button);
    }
    content.append(&chapter_name);
    content.append(&controls.chapter);
    let evidence = GtkBox::new(Orientation::Vertical, 0);
    content.append(&evidence);
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .max_content_height(440)
        .propagate_natural_height(true)
        .child(&content)
        .build();
    let popover = Popover::builder().child(&scroll).build();
    popover.connect_show({
        let nav = nav.clone();
        move |_| {
            crate::ui::clear_children(&evidence);
            evidence.append(&super::output_results_tray::build(&nav));
        }
    });
    let more = MenuButton::builder()
        .icon_name("view-more-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "output-options"))
        .popover(&popover)
        .valign(gtk4::Align::Center)
        .build();
    more.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "output-options"
    ))]);
    root.append(&more);
    update(&root, &nav, &controls);
    let weak = root.downgrade();
    glib::timeout_add_local(Duration::from_millis(250), move || {
        let Some(root) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        update(&root, &nav, &controls);
        glib::ControlFlow::Continue
    });
    root.upcast()
}

fn output_enabled(status: &OutputStatus, connected: bool) -> bool {
    connected && status.state != OutputRunState::Unknown && !status.state.is_transitioning()
}

fn set_active(button: &Button, active: bool, pending: bool) {
    for (class, enabled) in [("output-active", active), ("output-pending", pending)] {
        if enabled {
            button.add_css_class(class);
        } else {
            button.remove_css_class(class);
        }
    }
}

fn supports(state: &AppState, request: &str) -> bool {
    state
        .available_requests
        .iter()
        .any(|available| available == request)
}

fn output_text(button: &Button, name: &str, status: &str, summary: &str) {
    if let Some(label) = button.child().and_downcast::<gtk4::Label>() {
        label.set_markup(&format!(
            "{}\n<span size=\"smaller\" weight=\"normal\">{}</span>",
            glib::markup_escape_text(name),
            glib::markup_escape_text(status)
        ));
    }
    button.update_property(&[gtk4::accessible::Property::Label(summary)]);
}

fn update(root: &GtkBox, nav: &NavigationContext, controls: &Controls) {
    let state = nav.state.borrow();
    root.set_visible(state.current_page == Page::Live);
    let connected = matches!(state.obs_status, ObsStatus::Connected { .. });
    for (button, kind, status, start, stop) in [
        (
            &controls.stream,
            OutputKind::Stream,
            &state.stream_status,
            "StartStream",
            "StopStream",
        ),
        (
            &controls.record,
            OutputKind::Record,
            &state.record_status,
            "StartRecord",
            "StopRecord",
        ),
    ] {
        output_text(
            button,
            &kind.label(),
            &status.state.label(),
            &crate::ui::pages::live::output_label(kind, status, None),
        );
        button.set_sensitive(
            output_enabled(status, connected)
                && supports(&state, if status.active { stop } else { start }),
        );
        let action = match (kind, status.active) {
            (OutputKind::Stream, false) => fl!(LANGUAGE_LOADER, "window-sidebar-start-stream"),
            (OutputKind::Stream, true) => fl!(LANGUAGE_LOADER, "window-sidebar-stop-stream"),
            (OutputKind::Record, false) => fl!(LANGUAGE_LOADER, "window-sidebar-start-recording"),
            (OutputKind::Record, true) => fl!(LANGUAGE_LOADER, "window-sidebar-stop-recording"),
        };
        button.set_tooltip_text(Some(&action));
        button.update_property(&[gtk4::accessible::Property::Description(&action)]);
        set_active(button, status.active, status.state.is_transitioning());
    }
    let ready = connected && state.workspace.can_control();
    let snapshot = state.workspace.snapshot.as_ref();
    for (button, active, start, stop, camera) in [
        (
            &controls.replay,
            snapshot.and_then(|s| s.replay_active),
            "StartReplayBuffer",
            "StopReplayBuffer",
            false,
        ),
        (
            &controls.camera,
            snapshot.and_then(|s| s.virtual_camera_active),
            "StartVirtualCam",
            "StopVirtualCam",
            true,
        ),
    ] {
        let label = match active {
            Some(true) => OutputRunState::Active,
            Some(false) => OutputRunState::Inactive,
            None => OutputRunState::Unknown,
        }
        .label();
        let summary = if camera {
            fl!(
                LANGUAGE_LOADER,
                "output-camera-label",
                state = label.clone()
            )
        } else {
            fl!(
                LANGUAGE_LOADER,
                "output-replay-label",
                state = label.clone()
            )
        };
        let name = if camera {
            fl!(LANGUAGE_LOADER, "output-camera-name")
        } else {
            fl!(LANGUAGE_LOADER, "output-replay-name")
        };
        output_text(button, &name, &label, &summary);
        let action = match (camera, active == Some(true)) {
            (true, false) => fl!(LANGUAGE_LOADER, "control-camera-start"),
            (true, true) => fl!(LANGUAGE_LOADER, "control-camera-stop"),
            (false, false) => fl!(LANGUAGE_LOADER, "control-replay-start"),
            (false, true) => fl!(LANGUAGE_LOADER, "control-replay-stop"),
        };
        button.set_tooltip_text(Some(&action));
        button.update_property(&[gtk4::accessible::Property::Description(&action)]);
        button.set_sensitive(
            ready
                && active.is_some()
                && supports(&state, if active == Some(true) { stop } else { start }),
        );
        set_active(button, active == Some(true), state.workspace.pending);
    }
    let paused = state.record_status.state == OutputRunState::Paused;
    controls.pause.set_label(&if paused {
        fl!(LANGUAGE_LOADER, "control-resume")
    } else {
        fl!(LANGUAGE_LOADER, "control-pause")
    });
    controls.pause.set_sensitive(
        ready
            && pause_action(&state.record_status).is_some()
            && supports(
                &state,
                if paused {
                    "ResumeRecord"
                } else {
                    "PauseRecord"
                },
            ),
    );
    controls.save.set_sensitive(
        ready
            && snapshot.and_then(|s| s.replay_active) == Some(true)
            && supports(&state, "SaveReplayBuffer"),
    );
    let recording = pause_action(&state.record_status).is_some();
    controls
        .split
        .set_sensitive(ready && recording && supports(&state, "SplitRecordFile"));
    controls.chapter.set_sensitive(
        ready
            && recording
            && !controls.chapter_name.text().trim().is_empty()
            && supports(&state, "CreateRecordChapter"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_toggle_requires_known_settled_state_and_connection() {
        assert!(output_enabled(&OutputStatus::inactive(), true));
        assert!(output_enabled(&OutputStatus::active(), true));
        assert!(!output_enabled(&OutputStatus::active(), false));
        for state in [
            OutputRunState::Unknown,
            OutputRunState::Starting,
            OutputRunState::Stopping,
            OutputRunState::Reconnecting,
        ] {
            assert!(!output_enabled(&OutputStatus::new(true, state), true));
        }
    }
    #[test]
    fn pause_requires_authoritative_active_or_paused_recording() {
        assert_eq!(
            pause_action(&OutputStatus::new(true, OutputRunState::Active)),
            Some(true)
        );
        assert_eq!(
            pause_action(&OutputStatus::new(true, OutputRunState::Paused)),
            Some(false)
        );
        for state in [
            OutputRunState::Unknown,
            OutputRunState::Starting,
            OutputRunState::Stopping,
            OutputRunState::Reconnecting,
            OutputRunState::Inactive,
        ] {
            assert_eq!(pause_action(&OutputStatus::new(true, state)), None);
        }
        assert_eq!(
            pause_action(&OutputStatus::new(false, OutputRunState::Active)),
            None
        );
    }
}

#[cfg(test)]
#[path = "transport_bar_tests.rs"]
mod native_layout_tests;
