//! Live page — primary daily-operation surface.
//!
//! Returns a `LivePageHandle` so `ui::window` can push state updates into the
//! widgets without rebuilding the page.

use adw::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, FlowBox, Label, Orientation, PolicyType, ScrolledWindow, Stack,
    StackTransitionType,
};
use i18n_embed_fl::fl;

use crate::controller::command::AppCommand;
use crate::controller::state::{ObsStatus, Page};
use crate::domain::hotkey::{slot_badge, SceneHotkeyConfig};
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
use crate::domain::scene::{SceneId, SceneInventory};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::config::OutputConfig;
use crate::storage::registry::SceneRegistry;
use crate::ui::insert_compact_flow_child;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::scene_card::{SceneCardModel, SceneShortcut};
use crate::ui::widgets::{audio_card, scene_card};

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
struct OutputConfirmationDialog {
    heading: String,
    body: String,
    confirm_label: String,
    appearance: OutputConfirmationAppearance,
}

#[derive(PartialEq, Eq)]
struct AudioPresentation {
    epoch: u64,
    scope: crate::domain::workflow::ScopeBinding,
    scene: Option<String>,
    language: crate::domain::appearance::Language,
    inputs: Vec<(String, String, String)>,
}

impl AudioPresentation {
    fn same_owner(&self, other: &Self) -> bool {
        self.epoch == other.epoch
            && self.scope == other.scope
            && self.scene == other.scene
            && self.language == other.language
    }
}

/// Widget handles that `ui::window` updates when `AppEvent`s arrive.
pub(crate) struct LivePageHandle {
    pub(crate) root: Stack,
    pub(crate) scenes_box: FlowBox,
    /// Caption above the scene grid naming the active scene shortcut binding.
    pub(crate) hotkey_hint: Label,
    pub(crate) hotkey_notice_generation: crate::ui::hotkeys::NoticeGeneration,
    pub(crate) audio_box: FlowBox,
    scene_signature: std::cell::RefCell<String>,
    audio_signature: std::cell::RefCell<Option<AudioPresentation>>,
    audio_refresh_notice: Label,
    pub(crate) scene_cards: std::cell::RefCell<Vec<scene_card::SceneCardHandle>>,
    pub(crate) scene_scroll: ScrolledWindow,
    pub(crate) previews: std::rc::Rc<std::cell::RefCell<crate::ui::previews::PreviewState>>,
    pub(crate) audio_expanded: std::rc::Rc<std::cell::Cell<bool>>,

    /// Status line on the disconnected placeholder, kept so
    /// `show_disconnected_view` writes the hint here instead of overwriting
    /// the localized title.
    pub(crate) disconnected_detail: Label,
    pub(crate) audio_cards: std::cell::RefCell<Vec<audio_card::AudioCardHandle>>,
}

pub(crate) fn build(nav: NavigationContext) -> LivePageHandle {
    let root = Stack::builder()
        .vexpand(true)
        .hexpand(true)
        .hhomogeneous(false)
        .vhomogeneous(false)
        .transition_type(StackTransitionType::Crossfade)
        .build();
    root.add_css_class("app-page");
    root.add_css_class("live-page");
    let (disconnected, disconnected_detail) = build_disconnected_view();
    root.add_named(&disconnected, Some("disconnected"));
    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(24)
        .margin_end(24)
        .vexpand(true)
        .hexpand(true)
        .build();
    root.add_named(&page, Some("live"));
    root.set_visible_child_name("disconnected");
    page.add_tick_callback(|page, _| {
        let short = page
            .root()
            .and_downcast::<gtk4::Window>()
            .is_some_and(|window| window.height() < 700);
        let gap = if short { 8 } else { 12 };
        page.set_spacing(gap);
        page.set_margin_top(gap);
        page.set_margin_bottom(gap);
        if short {
            page.add_css_class("live-page-short");
        } else {
            page.remove_css_class("live-page-short");
        }
        glib::ControlFlow::Continue
    });
    page.append(&crate::ui::widgets::studio_controls::transition_row(
        nav.clone(),
    ));
    let hotkey_hint = Label::new(None);
    hotkey_hint.set_visible(false);
    hotkey_hint.set_wrap(true);
    hotkey_hint.set_xalign(0.0);
    hotkey_hint.add_css_class("caption");
    page.append(&hotkey_hint);
    let scenes_box = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(16)
        .row_spacing(16)
        .homogeneous(true)
        .halign(Align::Fill)
        .valign(Align::Start)
        .hexpand(true)
        .min_children_per_line(1)
        .max_children_per_line(8)
        .build();
    let scene_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .child(&scenes_box)
        .build();
    page.append(&scene_scroll);
    let audio_strip = GtkBox::new(Orientation::Horizontal, 12);
    audio_strip.add_css_class("live-audio-strip");
    let header = GtkBox::new(Orientation::Vertical, 4);
    header.set_valign(Align::Start);
    let label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-audio-section-label"))
        .hexpand(false)
        .xalign(0.0)
        .build();
    label.add_css_class("heading");
    header.append(&label);
    let audio_refresh_notice = Label::builder()
        .xalign(0.0)
        .wrap(true)
        .max_width_chars(14)
        .visible(false)
        .build();
    audio_refresh_notice.add_css_class("dim-label");
    header.append(&audio_refresh_notice);
    let expand = gtk4::ToggleButton::builder()
        .icon_name("pan-down-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "live-audio-expand"))
        .build();
    expand.add_css_class("flat");
    header.append(&expand);
    audio_strip.append(&header);
    let audio_box = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(12)
        .row_spacing(8)
        .homogeneous(true)
        .halign(Align::Fill)
        .min_children_per_line(1)
        .max_children_per_line(4)
        .hexpand(true)
        .build();
    audio_strip.append(&audio_box);
    let audio_expanded = std::rc::Rc::new(std::cell::Cell::new(false));
    expand.connect_toggled({
        let audio_expanded = audio_expanded.clone();
        let audio_box = audio_box.clone();
        move |button| {
            audio_expanded.set(button.is_active());
            button.set_icon_name(if button.is_active() {
                "pan-up-symbolic"
            } else {
                "pan-down-symbolic"
            });
            button.set_tooltip_text(Some(&if button.is_active() {
                fl!(LANGUAGE_LOADER, "live-audio-collapse")
            } else {
                fl!(LANGUAGE_LOADER, "live-audio-expand")
            }));
            if button.is_active() {
                audio_box.add_css_class("expanded");
            } else {
                audio_box.remove_css_class("expanded");
            }
            let mut child = audio_box.first_child();
            while let Some(row) = child {
                if let Some(card) = row.first_child() {
                    audio_card::set_compact_expanded(&card, button.is_active());
                }
                child = row.next_sibling();
            }
        }
    });
    page.append(&audio_strip);
    LivePageHandle {
        root,
        scenes_box,
        hotkey_hint,
        hotkey_notice_generation: Default::default(),
        audio_box,
        disconnected_detail,
        audio_cards: std::cell::RefCell::new(Vec::new()),
        scene_cards: std::cell::RefCell::new(Vec::new()),
        scene_signature: std::cell::RefCell::new(String::new()),
        audio_signature: std::cell::RefCell::new(None),
        audio_refresh_notice,
        previews: std::rc::Rc::new(std::cell::RefCell::new(Default::default())),
        scene_scroll,
        audio_expanded,
    }
}

pub(crate) fn show_disconnected_view(handle: &LivePageHandle, message: &str) {
    handle.root.set_visible_child_name("disconnected");
    handle.disconnected_detail.set_text(message);
}

/// The command that flips `kind` from its current state.
fn output_toggle_command(kind: OutputKind, active: bool) -> AppCommand {
    match (kind, active) {
        (OutputKind::Stream, false) => AppCommand::StartStreaming,
        (OutputKind::Stream, true) => AppCommand::StopStreaming,
        (OutputKind::Record, false) => AppCommand::StartRecording,
        (OutputKind::Record, true) => AppCommand::StopRecording,
    }
}

/// Start or stop one output, asking first when the user has asked to be asked.
///
/// The stream and record buttons behave identically apart from which output
/// they act on, so the behaviour lives here once and each button supplies its
/// `OutputKind`.
fn handle_output_toggle(button: &Button, nav: &NavigationContext, kind: OutputKind) {
    // Both reads happen under one borrow, which ends with this block. The
    // borrow must not still be held below: `confirm_output_action` opens a
    // dialog and `nav.dispatch` reaches the controller, and either could try to
    // borrow the state again.
    let (active, should_confirm) = {
        let state = nav.state.borrow();
        let active = match kind {
            OutputKind::Stream => state.stream_status.active,
            OutputKind::Record => state.record_status.active,
        };
        (
            active,
            requires_output_confirmation(kind, active, &state.config.outputs),
        )
    };

    let command = output_toggle_command(kind, active);
    if should_confirm {
        let action = output_action_for_active_state(active);
        let dialog = output_confirmation_dialog(kind, action);
        confirm_output_action(button, dialog, command, nav.clone());
    } else {
        nav.dispatch(command);
    }
}

fn requires_disconnect_confirmation(
    stream: &OutputStatus,
    record: &OutputStatus,
    config: &OutputConfig,
) -> bool {
    [stream, record]
        .iter()
        .any(|status| status.state == OutputRunState::Unknown || status.state.is_transitioning())
        || (stream.active && config.confirm_stop_stream)
        || (record.active && config.confirm_stop_recording)
}

pub(crate) fn handle_disconnect(button: &Button, nav: &NavigationContext) {
    let (stream, record, confirm) = {
        let state = nav.state.borrow();
        if !matches!(state.obs_status, ObsStatus::Connected { .. }) {
            return;
        }
        (
            state.stream_status.clone(),
            state.record_status.clone(),
            requires_disconnect_confirmation(
                &state.stream_status,
                &state.record_status,
                &state.config.outputs,
            ),
        )
    };
    if !confirm {
        nav.dispatch(AppCommand::Disconnect);
        return;
    }
    let parent = button
        .root()
        .and_then(|root| root.downcast::<gtk4::Window>().ok());
    let body = fl!(
        LANGUAGE_LOADER,
        "disconnect-confirm-body",
        stream = stream.state.label(),
        recording = record.state.label()
    );
    let dialog = adw::MessageDialog::new(
        parent.as_ref(),
        Some(&fl!(LANGUAGE_LOADER, "disconnect-confirm-heading")),
        Some(&body),
    );
    dialog.add_response("cancel", &fl!(LANGUAGE_LOADER, "live-cancel-button-label"));
    dialog.add_response(
        "confirm",
        &fl!(LANGUAGE_LOADER, "window-stop-disconnect-label"),
    );
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
    let nav = nav.scoped();
    let button = button.clone();
    dialog.connect_response(None, move |dialog, response| {
        dialog.close();
        if response != "confirm" || !nav.is_current() {
            return;
        }
        let unchanged = {
            let state = nav.state.borrow();
            if !matches!(state.obs_status, ObsStatus::Connected { .. }) {
                return;
            }
            state.stream_status == stream && state.record_status == record
        };
        // Another OBS client may change outputs while this dialog is open.
        if unchanged {
            nav.dispatch(AppCommand::Disconnect);
        } else {
            handle_disconnect(&button, &nav);
        }
    });
    dialog.present();
}

pub(crate) fn handle_stream_output_toggle(button: &Button, nav: &NavigationContext) {
    handle_output_toggle(button, nav, OutputKind::Stream);
}

pub(crate) fn handle_record_output_toggle(button: &Button, nav: &NavigationContext) {
    handle_output_toggle(button, nav, OutputKind::Record);
}

pub(crate) fn show_live_view(handle: &LivePageHandle) {
    handle.root.set_visible_child_name("live");
}

fn build_disconnected_view() -> (GtkBox, Label) {
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
    (view, detail)
}

pub(crate) fn output_label(
    kind: OutputKind,
    status: &OutputStatus,
    elapsed: Option<&str>,
) -> String {
    let kind = kind.label();
    let kind = kind.as_str();
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
        (OutputKind::Record, false) => config.confirm_start_recording,
        (OutputKind::Record, true) => config.confirm_stop_recording,
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
        (OutputKind::Record, OutputAction::Start) => OutputConfirmationDialog {
            heading: fl!(LANGUAGE_LOADER, "live-start-recording-confirm-heading"),
            body: fl!(LANGUAGE_LOADER, "live-start-recording-confirm-body"),
            confirm_label: fl!(LANGUAGE_LOADER, "live-start-recording-confirm-label"),
            appearance: OutputConfirmationAppearance::Suggested,
        },
        (OutputKind::Record, OutputAction::Stop) => OutputConfirmationDialog {
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
    let nav = nav.scoped();
    dialog.connect_response(None, move |dialog, response| {
        if response == "confirm" {
            nav.dispatch(command.clone());
        }
        dialog.close();
    });
    dialog.present();
}

/// Scene ids shown on Live, in card order.
///
/// The registry's saved display order decides the order, and therefore the
/// shortcut slot each scene gets; scenes OBS reports but the registry has not
/// seen yet follow in OBS order. Only live-switchable roles are listed, so slot
/// numbering matches exactly what the page renders.
pub(crate) fn live_scene_order(
    inventory: &SceneInventory,
    registry: &SceneRegistry,
) -> Vec<SceneId> {
    registry
        .ordered_scene_ids(inventory.scenes.iter().map(|scene| scene.id.as_str()))
        .into_iter()
        .filter(|scene_id| {
            registry
                .scenes
                .get(scene_id)
                .and_then(|entry| entry.role)
                .is_some_and(|role| role.is_live_switchable())
        })
        .collect()
}

/// Scene bound to a shortcut slot, or `None` when that slot is empty.
pub(crate) fn scene_for_slot(
    inventory: &SceneInventory,
    registry: &SceneRegistry,
    slot: usize,
) -> Option<SceneId> {
    live_scene_order(inventory, registry).into_iter().nth(slot)
}

/// Rebuild the scene cards from the current inventory.
///
/// Called by `ui::window::apply_event` whenever the inventory changes.
pub(crate) fn rebuild_scene_cards(
    handle: &LivePageHandle,
    inventory: &SceneInventory,
    nav: &NavigationContext,
) {
    let (registry, hotkeys, images) = {
        let state = nav.state.borrow();
        (
            state.registry.clone(),
            state.config.hotkeys,
            state.config.live.scene_previews,
        )
    };
    let scene_order = live_scene_order(inventory, &registry);
    let binding = {
        let state = nav.state.borrow();
        (state.workspace.epoch, state.workflow_scope())
    };
    let signature = format!("{scene_order:?}{registry:?}{hotkeys:?}{images}{binding:?}");
    handle
        .hotkey_hint
        .set_tooltip_text(crate::ui::hotkeys::scene_hint(nav, &hotkeys).as_deref());
    if *handle.scene_signature.borrow() == signature {
        for card in handle.scene_cards.borrow().iter() {
            card.update_state(nav, inventory.current_id.as_deref() == Some(&card.scene_id));
        }
        return;
    }
    *handle.scene_signature.borrow_mut() = signature;
    handle.scene_cards.borrow_mut().clear();
    // Remove all existing children
    // FlowBox requires removing via the FlowBoxChild wrapper
    while let Some(child) = handle.scenes_box.first_child() {
        handle.scenes_box.remove(&child);
    }

    if scene_order.is_empty() {
        // Nothing to bind to, so the shortcut caption would only mislead.
        set_hotkey_hint(handle, None);
        let empty = GtkBox::new(Orientation::Vertical, 8);
        let hint = Label::builder()
            .label(if inventory.scenes.is_empty() {
                fl!(LANGUAGE_LOADER, "live-scenes-obs-empty")
            } else {
                fl!(LANGUAGE_LOADER, "live-scenes-no-primary-hint")
            })
            .wrap(true)
            .xalign(0.0)
            .build();
        hint.add_css_class("dim-label");
        empty.append(&hint);
        if !inventory.scenes.is_empty() {
            let organize = Button::with_label(&fl!(LANGUAGE_LOADER, "live-organize-scenes"));
            organize.set_halign(Align::Start);
            let nav = nav.clone();
            organize.connect_clicked(move |_| nav.switch_to_page(Page::Inventory));
            empty.append(&organize);
        }
        insert_compact_flow_child(&handle.scenes_box, &empty);
        return;
    }

    set_hotkey_hint(handle, None);

    for (slot, scene_id) in scene_order.iter().enumerate() {
        let Some(scene) = inventory.scenes.iter().find(|scene| &scene.id == scene_id) else {
            continue;
        };
        let registry_entry = registry.scenes.get(&scene.id);
        let card = scene_card::build(
            SceneCardModel {
                scene_name: &scene.id,
                scene_id: scene.id.clone(),
                scene_role: registry_entry
                    .and_then(|entry| entry.role)
                    .unwrap_or_default(),
                is_active: inventory.current_id.as_deref() == Some(&scene.id),
                accent_color: registry_entry.and_then(|entry| entry.accent_color.as_deref()),
                shortcut: shortcut_for_slot(&hotkeys, slot),
                icon: registry_entry.and_then(|entry| entry.icon.as_deref()),
            },
            nav.clone(),
        );
        handle.scenes_box.insert(&card.root, -1);
        if let Some(wrapper) = card.root.parent() {
            wrapper.set_focusable(false);
        }
        handle.scene_cards.borrow_mut().push(card);
    }
    crate::ui::previews::restore(handle, nav);
}

fn shortcut_for_slot(hotkeys: &SceneHotkeyConfig, slot: usize) -> Option<SceneShortcut> {
    Some(SceneShortcut {
        badge: slot_badge(slot)?,
        label: hotkeys.shortcut_label(slot)?,
    })
}

/// Set the caption above the scene grid, hiding it when there is nothing to say.
pub(crate) fn set_hotkey_hint(handle: &LivePageHandle, text: Option<&str>) {
    handle.hotkey_notice_generation.advance();
    match text {
        Some(text) => {
            handle.hotkey_hint.set_text(text);
            handle.hotkey_hint.set_visible(true);
        }
        None => {
            handle.hotkey_hint.set_text("");
            handle.hotkey_hint.set_visible(false);
        }
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
    let (presentation, notice) = {
        let state = nav.state.borrow();
        (
            AudioPresentation {
                epoch: state.workspace.epoch,
                scope: state.workflow_scope(),
                scene: state.scene_inventory.current_id.clone(),
                language: state.config.language,
                inputs: inputs
                    .iter()
                    .map(|input| {
                        (
                            input.id.clone(),
                            input.name.clone(),
                            input.display_name.clone(),
                        )
                    })
                    .collect(),
            },
            state.active_audio_notice(),
        )
    };
    if let Some(notice) = notice.as_ref() {
        let same_owner = handle
            .audio_signature
            .borrow()
            .as_ref()
            .is_some_and(|previous| previous.same_owner(&presentation));
        if same_owner && !handle.audio_cards.borrow().is_empty() {
            for card in handle.audio_cards.borrow().iter() {
                card.root.set_sensitive(false);
            }
            handle.audio_refresh_notice.set_text(notice);
            handle.audio_refresh_notice.set_visible(true);
            return;
        }
    } else if !inputs.is_empty() && handle.audio_signature.borrow().as_ref() == Some(&presentation)
    {
        handle.audio_refresh_notice.set_visible(false);
        for (card, input) in handle.audio_cards.borrow().iter().zip(inputs) {
            card.root.set_sensitive(true);
            card.update_mute(input.muted);
            card.update_volume(input.volume_mul, input.volume_db);
            audio_card::set_compact_expanded(card.root.upcast_ref(), handle.audio_expanded.get());
        }
        return;
    }
    handle.audio_refresh_notice.set_visible(false);
    if notice.is_some() || inputs.is_empty() {
        let text = notice.unwrap_or_else(|| fl!(LANGUAGE_LOADER, "live-audio-empty-hint"));
        // A status-only update does not need a new placeholder either.
        if handle.audio_cards.borrow().is_empty() {
            if let Some(label) = handle
                .audio_box
                .first_child()
                .and_then(|row| row.first_child())
                .and_downcast::<Label>()
            {
                label.set_text(&text);
                return;
            }
        }
        clear_audio_cards(handle);
        let hint = Label::builder().label(text).xalign(0.0).wrap(true).build();
        hint.add_css_class("dim-label");
        insert_compact_flow_child(&handle.audio_box, &hint);
        return;
    }
    clear_audio_cards(handle);
    for input in inputs {
        let card = audio_card::build_compact(input, nav.clone(), handle.audio_expanded.get());
        insert_compact_flow_child(&handle.audio_box, &card.root);
        handle.audio_cards.borrow_mut().push(card);
    }
    *handle.audio_signature.borrow_mut() = Some(presentation);
}

fn clear_audio_cards(handle: &LivePageHandle) {
    while let Some(child) = handle.audio_box.first_child() {
        handle.audio_box.remove(&child);
    }
    handle.audio_cards.borrow_mut().clear();
    *handle.audio_signature.borrow_mut() = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::hotkey::SceneHotkeyStyle;
    use crate::domain::output::OutputRunState;
    use crate::domain::role::SceneRole;
    use crate::domain::scene::Scene;
    use crate::storage::registry::SceneEntry;

    #[test]
    #[ignore = "requires a disposable GTK display; no OBS or keyring access"]
    fn native_audio_refresh_retains_controls_until_their_owner_changes() {
        crate::ui::test_support::native_test("ui::pages::live::tests::native_audio_refresh_retains_controls_until_their_owner_changes", || {
            let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
            let input = crate::domain::audio::AudioInput::new("Mic".into(), false, 0.5, -6.0);
            nav.state.borrow_mut().set_current_program("Main".into());
            nav.state.borrow_mut().apply_active_audio(Some("Main".into()), Ok(vec![input.clone()]));
            let live = build(nav.clone());
            rebuild_audio_cards(&live, std::slice::from_ref(&input), &nav);
            let root = live.audio_cards.borrow()[0].root.clone();
            rebuild_audio_cards(&live, std::slice::from_ref(&input), &nav);
            assert_eq!(live.audio_cards.borrow()[0].root, root, "identical audio readback replaced its controls");
            nav.state.borrow_mut().invalidate_active_audio();
            rebuild_audio_cards(&live, &[], &nav);
            assert_eq!(live.audio_cards.borrow()[0].root, root, "same-scene loading should disable, not replace, controls");
            assert!(!root.is_sensitive());
            nav.state.borrow_mut().apply_active_audio(Some("Main".into()), Ok(vec![input.clone()]));
            rebuild_audio_cards(&live, std::slice::from_ref(&input), &nav);
            assert_eq!(live.audio_cards.borrow()[0].root, root);
            assert!(root.is_sensitive());
            nav.state.borrow_mut().set_current_program("Other".into());
            rebuild_audio_cards(&live, &[], &nav);
            assert!(live.audio_cards.borrow().is_empty(), "a different Program must not retain stale controls");
        });
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_active_audio_readiness_never_presents_previous_program_controls() {
        crate::ui::test_support::native_test("ui::pages::live::tests::native_active_audio_readiness_never_presents_previous_program_controls", || {
            let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
            {
                let mut state = nav.state.borrow_mut();
                state.set_current_program("A".into());
                state.apply_active_audio(Some("A".into()), Ok(vec![crate::domain::audio::AudioInput::new("MicA".into(), false, 0.5, -6.0)]));
            }
            let live = build(nav.clone());
            let inputs = nav.state.borrow().audio_inputs.clone();
            rebuild_audio_cards(&live, &inputs, &nav);
            assert_eq!(live.audio_cards.borrow().len(), 1);
            nav.state.borrow_mut().set_current_program("B".into());
            rebuild_audio_cards(&live, &[], &nav);
            assert!(live.audio_cards.borrow().is_empty());
            let rendered_hint = || live.audio_box.first_child().unwrap().first_child().unwrap().downcast::<Label>().unwrap().text().to_string();
            assert_eq!(rendered_hint(), nav.state.borrow().active_audio_notice().unwrap());
            nav.state.borrow_mut().apply_active_audio(Some("B".into()), Err("fixture observation unavailable".into()));
            rebuild_audio_cards(&live, &[], &nav);
            assert!(live.audio_cards.borrow().is_empty());
            assert!(rendered_hint().contains("fixture observation unavailable"));
            nav.state.borrow_mut().apply_active_audio(Some("B".into()), Ok(Vec::new()));
            rebuild_audio_cards(&live, &[], &nav);
            assert_eq!(rendered_hint(), fl!(LANGUAGE_LOADER, "live-audio-empty-hint"));
        });
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_observed_studio_changes_update_hints_without_replacing_cards() {
        crate::ui::test_support::native_test("ui::pages::live::tests::native_observed_studio_changes_update_hints_without_replacing_cards", || {
            let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
            {
                let mut state = nav.state.borrow_mut();
                state.obs_status = ObsStatus::Connected { obs_version: "fixture".into() };
                state.scene_collections.current = Some("Fixture".into());
                state.scene_inventory.scenes = vec![Scene { id: "Main".into() }];
                state.scene_inventory.current_id = Some("Main".into());
                state.registry.scenes.insert("Main".into(), SceneEntry { role: Some(SceneRole::Primary), ..Default::default() });
                state.workspace.observed_at = Some(std::time::Instant::now());
                state.workspace.snapshot = Some(Default::default());
                state.workspace.snapshot.as_mut().unwrap().studio.enabled = Some(false);
            }
            let live = build(nav.clone());
            let inventory = nav.state.borrow().scene_inventory.clone();
            rebuild_scene_cards(&live, &inventory, &nav);
            let root = live.scene_cards.borrow()[0].root.clone();
            for studio in [false, true, false] {
                {
                    let mut state = nav.state.borrow_mut();
                    state.workspace.snapshot.as_mut().unwrap().studio.enabled = Some(studio);
                    state.workspace.pending = false;
                    state.workspace.loading = false;
                    state.workspace.observed_at = Some(std::time::Instant::now());
                }
                rebuild_scene_cards(&live, &inventory, &nav);
                assert_eq!(live.scene_cards.borrow()[0].root, root);
                let hint = if studio { fl!(LANGUAGE_LOADER, "scene-card-stage-hint") }
                    else { fl!(LANGUAGE_LOADER, "scene-card-tooltip-ready") };
                assert!(root.tooltip_text().unwrap().contains(&hint));
                assert_eq!(live.hotkey_hint.tooltip_text().as_deref(), crate::ui::hotkeys::scene_hint(&nav, &nav.state.borrow().config.hotkeys).as_deref());
                root.emit_clicked();
                let expected = if studio { fl!(LANGUAGE_LOADER, "control-preview", scene = "Main") }
                    else { fl!(LANGUAGE_LOADER, "control-program", scene = "Main") };
                assert_eq!(nav.state.borrow().workspace.activity.entries.back().unwrap().description, expected);
            }
        });
    }

    #[test]
    fn disconnect_confirmation_respects_stops_and_unknown_states() {
        let mut config = OutputConfig {
            confirm_stop_stream: false,
            confirm_stop_recording: false,
            ..Default::default()
        };
        assert!(!requires_disconnect_confirmation(
            &OutputStatus::inactive(),
            &OutputStatus::inactive(),
            &config
        ));
        assert!(!requires_disconnect_confirmation(
            &OutputStatus::active(),
            &OutputStatus::active(),
            &config
        ));
        config.confirm_stop_stream = true;
        assert!(requires_disconnect_confirmation(
            &OutputStatus::active(),
            &OutputStatus::inactive(),
            &config
        ));
        assert!(!requires_disconnect_confirmation(
            &OutputStatus::inactive(),
            &OutputStatus::active(),
            &config
        ));
        config.confirm_stop_recording = true;
        assert!(requires_disconnect_confirmation(
            &OutputStatus::inactive(),
            &OutputStatus::active(),
            &config
        ));
        for state in [
            OutputRunState::Unknown,
            OutputRunState::Starting,
            OutputRunState::Stopping,
            OutputRunState::Reconnecting,
        ] {
            assert!(requires_disconnect_confirmation(
                &OutputStatus::new(false, state),
                &OutputStatus::inactive(),
                &config
            ));
        }
    }

    fn scene_entry(role: SceneRole) -> SceneEntry {
        SceneEntry {
            role: Some(role),
            tags: Vec::new(),
            protected: false,
            accent_color: None,
            icon: None,
        }
    }

    /// Inventory in OBS order, with every scene registered under `roles`.
    fn live_fixture(
        roles: &[(&str, SceneRole)],
        order: &[&str],
    ) -> (SceneInventory, SceneRegistry) {
        let inventory = SceneInventory {
            scenes: roles
                .iter()
                .map(|(id, _)| Scene {
                    id: (*id).to_string(),
                })
                .collect(),
            current_id: None,
            previous_id: None,
        };
        let mut registry = SceneRegistry {
            scene_order: order.iter().map(|id| (*id).to_string()).collect(),
            ..SceneRegistry::default()
        };
        for (id, role) in roles {
            registry
                .scenes
                .insert((*id).to_string(), scene_entry(*role));
        }
        (inventory, registry)
    }

    #[test]
    fn live_scene_order_follows_the_registry_order() {
        let (inventory, registry) = live_fixture(
            &[
                ("Camera", SceneRole::Primary),
                ("Slides", SceneRole::Primary),
                ("Break", SceneRole::Primary),
            ],
            &["Break", "Camera"],
        );

        assert_eq!(
            live_scene_order(&inventory, &registry),
            ["Break", "Camera", "Slides"]
        );
    }

    #[test]
    fn live_scene_order_skips_scenes_that_are_not_switchable() {
        let (inventory, registry) = live_fixture(
            &[
                ("Camera", SceneRole::Primary),
                ("Overlay", SceneRole::Module),
                ("Slides", SceneRole::Primary),
            ],
            &[],
        );

        assert_eq!(
            live_scene_order(&inventory, &registry),
            ["Camera", "Slides"]
        );
    }

    #[test]
    fn slots_index_the_rendered_card_order() {
        let (inventory, registry) = live_fixture(
            &[
                ("Camera", SceneRole::Primary),
                ("Overlay", SceneRole::Module),
                ("Slides", SceneRole::Primary),
            ],
            &["Slides"],
        );

        assert_eq!(
            scene_for_slot(&inventory, &registry, 0).as_deref(),
            Some("Slides")
        );
        assert_eq!(
            scene_for_slot(&inventory, &registry, 1).as_deref(),
            Some("Camera")
        );
        assert_eq!(scene_for_slot(&inventory, &registry, 2), None);
    }

    #[test]
    fn card_shortcuts_show_the_digit_and_name_the_full_binding() {
        let hotkeys = SceneHotkeyConfig::default();

        assert_eq!(
            shortcut_for_slot(&hotkeys, 0),
            Some(SceneShortcut {
                badge: "1".to_string(),
                label: "Ctrl+1".to_string(),
            })
        );
        assert_eq!(
            shortcut_for_slot(&hotkeys, 9),
            Some(SceneShortcut {
                badge: "0".to_string(),
                label: "Ctrl+0".to_string(),
            })
        );
        assert_eq!(shortcut_for_slot(&hotkeys, 10), None);
    }

    #[test]
    fn cards_past_the_last_slot_and_disabled_hotkeys_get_no_badge() {
        let disabled = SceneHotkeyConfig {
            enabled: false,
            style: SceneHotkeyStyle::Plain,
            ..SceneHotkeyConfig::default()
        };

        assert_eq!(shortcut_for_slot(&disabled, 0), None);
    }

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
            OutputKind::Record,
            false,
            &config
        ));
        assert!(requires_output_confirmation(
            OutputKind::Record,
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
            OutputKind::Record,
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
            OutputKind::Record,
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
            output_confirmation_dialog(OutputKind::Record, OutputAction::Start),
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
            output_confirmation_dialog(OutputKind::Record, OutputAction::Stop),
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
    fn output_label_shows_elapsed_time_only_while_active() {
        assert_eq!(
            output_label(
                OutputKind::Stream,
                &output_status(true, OutputRunState::Active),
                Some("00:01:23")
            ),
            "Stream: Active · 00:01:23"
        );
        assert_eq!(
            output_label(
                OutputKind::Record,
                &output_status(false, OutputRunState::Inactive),
                Some("00:01:23")
            ),
            "Record: Inactive"
        );
        assert_eq!(
            output_label(
                OutputKind::Stream,
                &output_status(true, OutputRunState::Active),
                None
            ),
            "Stream: Active"
        );
    }
}
