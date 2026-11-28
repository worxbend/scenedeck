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
    registry_generation: u64,
    registry_status: crate::controller::state::RegistryStatus,
    scope: crate::domain::workflow::ScopeBinding,
    language: crate::domain::appearance::Language,
    inputs: Vec<(String, String, String)>,
}

#[derive(PartialEq, Eq)]
struct ScenePresentation {
    epoch: u64,
    registry_generation: u64,
    registry_status: crate::controller::state::RegistryStatus,
    scope: crate::domain::workflow::ScopeBinding,
    language: crate::domain::appearance::Language,
    images: bool,
    has_scenes: bool,
    hotkeys: SceneHotkeyConfig,
    filter: crate::storage::registry::SceneFilter,
    scenes: Vec<SceneCardPresentation>,
}

#[derive(PartialEq, Eq)]
struct SceneCardPresentation {
    id: String,
    slot: usize,
    role: Option<crate::domain::role::SceneRole>,
    icon: Option<String>,
    accent: Option<String>,
}

impl AudioPresentation {
    fn same_owner(&self, other: &Self) -> bool {
        self.epoch == other.epoch
            && self.scope == other.scope
            && self.language == other.language
            && self.registry_generation == other.registry_generation
            && self.registry_status == other.registry_status
    }
}

/// Widget handles that `ui::window` updates when `AppEvent`s arrive.
pub(crate) struct LivePageHandle {
    pub(crate) root: Stack,
    pub(crate) scenes_box: FlowBox,
    deck_controls: GtkBox,
    pub(crate) quick_actions_refresh: std::rc::Rc<dyn Fn()>,
    /// Caption above the scene grid naming the active scene shortcut binding.
    pub(crate) hotkey_hint: Label,
    pub(crate) hotkey_notice_generation: crate::ui::hotkeys::NoticeGeneration,
    pub(crate) audio_box: FlowBox,
    scene_signature: std::cell::RefCell<Option<ScenePresentation>>,
    audio_signature: std::cell::RefCell<Option<AudioPresentation>>,
    audio_refresh_notice: gtk4::Spinner,
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
    let deck_controls = GtkBox::new(Orientation::Vertical, 0);
    page.append(&deck_controls);
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
    let scene_content = GtkBox::new(Orientation::Vertical, 12);
    let (quick_actions, quick_actions_refresh) = crate::ui::quick_actions::build(nav.clone());
    let checks = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(2)
        .column_spacing(8)
        .row_spacing(8)
        .build();
    crate::ui::insert_compact_flow_child(&checks, &crate::ui::preflight::button(nav.clone()));
    crate::ui::insert_compact_flow_child(&checks, &crate::ui::health_alerts::button(nav.clone()));
    crate::ui::insert_compact_flow_child(&checks, &crate::ui::bookmarks::button(nav.clone()));
    scene_content.append(&checks);
    scene_content.append(&quick_actions);
    scene_content.append(&scenes_box);
    let scene_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .child(&scene_content)
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
    let audio_heading = GtkBox::new(Orientation::Horizontal, 4);
    audio_heading.append(&label);
    header.append(&audio_heading);
    // Reserve a small activity slot. Adding a loading label changed the strip's
    // minimum height and resized every scene preview during an ordinary read.
    let audio_refresh_notice = gtk4::Spinner::builder()
        .valign(Align::Center)
        .width_request(16)
        .height_request(16)
        .opacity(0.0)
        .build();
    audio_heading.append(&audio_refresh_notice);
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
    // Short windows keep the scene grid usable: audio collapses to one header
    // row until expanded. Mixer keeps the full controls.
    page.add_tick_callback({
        let audio_box = audio_box.clone();
        let expand = expand.clone();
        move |page, _| {
            let collapsed =
                audio_collapsed(page.has_css_class("live-page-short"), expand.is_active());
            audio_box.set_visible(!collapsed);
            header.set_orientation(if collapsed {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            });
            glib::ControlFlow::Continue
        }
    });
    LivePageHandle {
        root,
        scenes_box,
        deck_controls,
        quick_actions_refresh,
        hotkey_hint,
        hotkey_notice_generation: Default::default(),
        audio_box,
        disconnected_detail,
        audio_cards: std::cell::RefCell::new(Vec::new()),
        scene_cards: std::cell::RefCell::new(Vec::new()),
        scene_signature: std::cell::RefCell::new(None),
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

/// Start or stop one output, asking first when the user has asked to be asked.
///
/// The stream and record buttons behave identically apart from which output
/// they act on, so the behaviour lives here once and each button supplies its
/// `OutputKind`.
fn handle_output_toggle(button: &Button, nav: &NavigationContext, kind: OutputKind) {
    let active = {
        let state = nav.state.borrow();
        match kind {
            OutputKind::Stream => state.stream_status.active,
            OutputKind::Record => state.record_status.active,
        }
    };
    if !active
        && crate::ui::preflight::review_before_start(button, nav, kind, {
            let button = button.downgrade();
            let nav = nav.scoped();
            move || {
                if let Some(button) = button.upgrade() {
                    finish_output_toggle(&button, &nav, kind, false);
                }
            }
        })
    {
        return;
    }
    finish_output_toggle(button, nav, kind, active);
}

fn finish_output_toggle(button: &Button, nav: &NavigationContext, kind: OutputKind, active: bool) {
    if !nav.is_current() {
        return;
    }
    let should_confirm =
        requires_output_confirmation(kind, active, &nav.state.borrow().config.outputs);
    let command = AppCommand::SetOutput {
        kind,
        active: !active,
    };
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

/// Short windows hide the audio cards unless the user expanded them.
fn audio_collapsed(short: bool, expanded: bool) -> bool {
    short && !expanded
}

fn output_running(status: &OutputStatus) -> bool {
    status.active || status.state.is_transitioning()
}

/// Whether disconnecting would stop a running stream or recording.
pub(crate) fn disconnect_stops_outputs(stream: &OutputStatus, record: &OutputStatus) -> bool {
    output_running(stream) || output_running(record)
}

/// Names only the outputs a disconnect would actually stop.
pub(crate) fn disconnect_label(stream: &OutputStatus, record: &OutputStatus) -> String {
    match (output_running(stream), output_running(record)) {
        (true, true) => fl!(LANGUAGE_LOADER, "window-stop-disconnect-label"),
        (true, false) => fl!(LANGUAGE_LOADER, "window-stop-stream-disconnect-label"),
        (false, true) => fl!(LANGUAGE_LOADER, "window-stop-record-disconnect-label"),
        (false, false) => fl!(LANGUAGE_LOADER, "window-connect-btn-disconnect"),
    }
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
    dialog.add_response("confirm", &disconnect_label(&stream, &record));
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
            state.stream_status.same_run_state(&stream)
                && state.record_status.same_run_state(&record)
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
/// seen yet follow in OBS order. Only live-switchable roles are assigned slots.
/// Filtering and favorite display order never change these assignments.
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

fn visible_scene_slots(
    inventory: &SceneInventory,
    registry: &SceneRegistry,
) -> Vec<(usize, SceneId)> {
    let mut scenes: Vec<_> = live_scene_order(inventory, registry)
        .into_iter()
        .enumerate()
        .filter(|(_, name)| registry.scene_matches_filter(name))
        .collect();
    scenes.sort_by_key(|(_, name)| {
        !registry
            .scenes
            .get(name)
            .is_some_and(|entry| entry.favorite)
    });
    scenes
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
    (handle.quick_actions_refresh)();
    let (registry, hotkeys, images) = {
        let state = nav.state.borrow();
        (
            state.registry.clone(),
            state.config.hotkeys,
            state.config.live.scene_previews,
        )
    };
    let availability = crate::ui::persist::registry_availability_message(nav);
    let scene_order = if availability.is_none() {
        visible_scene_slots(inventory, &registry)
    } else {
        Vec::new()
    };
    let signature = {
        let state = nav.state.borrow();
        ScenePresentation {
            epoch: state.workspace.epoch,
            registry_generation: state.show_registry.generation,
            registry_status: state.show_registry.status.clone(),
            scope: state.workflow_scope(),
            language: state.config.language,
            images,
            has_scenes: !inventory.scenes.is_empty(),
            hotkeys,
            filter: registry.scene_filter.clone(),
            scenes: scene_order
                .iter()
                .map(|(slot, id)| {
                    let entry = registry.scenes.get(id);
                    SceneCardPresentation {
                        id: id.clone(),
                        slot: *slot,
                        role: entry.and_then(|e| e.role),
                        icon: entry.and_then(|e| e.icon.clone()),
                        accent: entry.and_then(|e| e.accent_color.clone()),
                    }
                })
                .collect(),
        }
    };
    let deck_owner_changed = handle
        .scene_signature
        .borrow()
        .as_ref()
        .is_none_or(|previous| {
            previous.registry_generation != signature.registry_generation
                || previous.registry_status != signature.registry_status
                || previous.language != signature.language
        });
    if deck_owner_changed {
        crate::ui::clear_children(&handle.deck_controls);
        if availability.is_none() {
            handle
                .deck_controls
                .append(&crate::ui::widgets::deck_controls::build(nav, || {}));
        }
    } else if let Some(controls) = handle.deck_controls.first_child() {
        crate::ui::widgets::deck_controls::sync(&controls);
    }
    handle
        .hotkey_hint
        .set_tooltip_text(crate::ui::hotkeys::scene_hint(nav, &hotkeys).as_deref());
    if handle.scene_signature.borrow().as_ref() == Some(&signature) {
        for card in handle.scene_cards.borrow().iter() {
            card.update_state(nav, inventory.current_id.as_deref() == Some(&card.scene_id));
        }
        return;
    }
    *handle.scene_signature.borrow_mut() = Some(signature);
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
            .label(if let Some(message) = availability {
                message
            } else if inventory.scenes.is_empty() {
                fl!(LANGUAGE_LOADER, "live-scenes-obs-empty")
            } else if registry.scene_filter.is_active()
                && !live_scene_order(inventory, &registry).is_empty()
            {
                fl!(LANGUAGE_LOADER, "deck-no-results")
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

    for (slot, scene_id) in &scene_order {
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
                shortcut: shortcut_for_slot(&hotkeys, *slot),
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
/// Called by `ui::window::apply_event` when active-scene audio is observed
/// or the connection is reset.
pub(crate) fn rebuild_audio_cards(
    handle: &LivePageHandle,
    inputs: &[crate::domain::audio::AudioInput],
    nav: &NavigationContext,
) {
    let (presentation, notice, loading) = {
        let state = nav.state.borrow();
        (
            AudioPresentation {
                epoch: state.workspace.epoch,
                registry_generation: state.show_registry.generation,
                registry_status: state.show_registry.status.clone(),
                scope: state.workflow_scope(),
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
            matches!(
                state.active_audio_status,
                crate::controller::state::ActiveAudioStatus::Loading
            ),
        )
    };
    if let Some(notice) = notice.as_ref() {
        let same_owner = handle
            .audio_signature
            .borrow()
            .as_ref()
            .is_some_and(|previous| previous.same_owner(&presentation));
        if same_owner && loading && !handle.audio_cards.borrow().is_empty() {
            for card in handle.audio_cards.borrow().iter() {
                // An input is collection-scoped, not scene-scoped. The existing
                // controls still address that exact input while membership is read.
                card.set_available(loading);
            }
            handle.audio_refresh_notice.set_tooltip_text(Some(notice));
            handle.audio_refresh_notice.set_spinning(loading);
            handle.audio_refresh_notice.set_opacity(1.0);
            return;
        }
    } else if !inputs.is_empty() && handle.audio_signature.borrow().as_ref() == Some(&presentation)
    {
        handle.audio_refresh_notice.set_spinning(false);
        handle.audio_refresh_notice.set_opacity(0.0);
        for (card, input) in handle.audio_cards.borrow().iter().zip(inputs) {
            card.set_available(true);
            card.update_mute(input.muted);
            card.update_volume(input.volume_mul, input.volume_db);
            audio_card::set_compact_expanded(card.root.upcast_ref(), handle.audio_expanded.get());
        }
        return;
    }
    handle.audio_refresh_notice.set_spinning(false);
    handle.audio_refresh_notice.set_opacity(0.0);
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
    let previous = handle.audio_signature.borrow_mut().take();
    let same_owner = previous
        .as_ref()
        .is_some_and(|old| old.same_owner(&presentation));
    let mut retained = std::mem::take(&mut *handle.audio_cards.borrow_mut());
    retained.retain(|card| {
        let unchanged = same_owner
            && presentation.inputs.iter().any(|input| {
                input.0 == card.input_id && previous.as_ref().unwrap().inputs.contains(input)
            });
        if !unchanged {
            if let Some(wrapper) = card.root.parent() {
                handle.audio_box.remove(&wrapper);
            }
        }
        unchanged
    });
    // Clear a status placeholder, never a still-valid input card.
    if retained.is_empty() {
        while let Some(child) = handle.audio_box.first_child() {
            handle.audio_box.remove(&child);
        }
    }
    for (index, input) in inputs.iter().enumerate() {
        let card = retained
            .iter()
            .position(|card| card.input_id == input.id)
            .map(|index| retained.remove(index))
            .unwrap_or_else(|| {
                audio_card::build_compact(input, nav.clone(), handle.audio_expanded.get())
            });
        if let Some(wrapper) = card.root.parent() {
            if handle
                .audio_box
                .child_at_index(index as i32)
                .as_ref()
                .map(|child| child.upcast_ref::<gtk4::Widget>())
                != Some(&wrapper)
            {
                handle.audio_box.remove(&wrapper);
                handle.audio_box.insert(&wrapper, index as i32);
            }
        } else {
            handle.audio_box.insert(&card.root, index as i32);
            if let Some(wrapper) = card.root.parent() {
                wrapper.set_focusable(false);
            }
        }
        card.set_available(true);
        card.update_mute(input.muted);
        card.update_volume(input.volume_mul, input.volume_db);
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
mod tests;
