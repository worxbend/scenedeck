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
use crate::domain::hotkey::{slot_badge, SceneHotkeyConfig};
use crate::domain::output::OutputStatus;
use crate::domain::scene::{SceneId, SceneInventory};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::config::OutputConfig;
use crate::storage::registry::SceneRegistry;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::scene_card::{SceneCardModel, SceneShortcut};
use crate::ui::widgets::{audio_card, scene_card};

/// Icon beside the program-scene heading.
const PROGRAM_ICON: &str = "nf-md-television-play-symbolic";
/// Icon beside the Scenes section heading.
const SCENES_ICON: &str = "nf-md-view-grid-symbolic";
/// Icon beside the Audio section heading.
const AUDIO_ICON: &str = "nf-md-tune-vertical-symbolic";

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
struct OutputConfirmationDialog {
    heading: String,
    body: String,
    confirm_label: String,
    appearance: OutputConfirmationAppearance,
}

/// Widget handles that `ui::window` updates when `AppEvent`s arrive.
pub(crate) struct LivePageHandle {
    pub(crate) root: Stack,
    pub(crate) current_scene_label: Label,
    pub(crate) scenes_box: FlowBox,
    /// Caption above the scene grid naming the active scene shortcut binding.
    pub(crate) hotkey_hint: Label,
    pub(crate) audio_box: FlowBox,
    pub(crate) audio_cards: std::cell::RefCell<Vec<audio_card::AudioCardHandle>>,
}

pub(crate) fn build(_nav: NavigationContext) -> LivePageHandle {
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

    // ── Program scene label ───────────────────────────────────────────────────
    let current_label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-current-scene-placeholder"))
        .xalign(0.0)
        .build();
    current_label.add_css_class("heading");
    page.append(&heading_with_icon(PROGRAM_ICON, &current_label));

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
        .margin_bottom(8)
        .vexpand(false)
        .hexpand(true)
        .build();
    let scenes_section_header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .build();
    let scenes_section_label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-scenes-section-label"))
        .xalign(0.0)
        .hexpand(true)
        .build();
    scenes_section_label.add_css_class("caption-heading");
    scenes_section_header.append(&section_icon(SCENES_ICON));
    scenes_section_header.append(&scenes_section_label);

    // Names the shortcut binding once, so each card only needs its digit.
    let hotkey_hint = Label::builder()
        .label("")
        .xalign(1.0)
        .halign(Align::End)
        .visible(false)
        .build();
    hotkey_hint.add_css_class("caption");
    hotkey_hint.add_css_class("live-hotkey-hint");
    scenes_section_header.append(&hotkey_hint);
    scenes_pane.append(&scenes_section_header);

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
        .margin_top(8)
        .vexpand(true)
        .hexpand(true)
        .build();

    let audio_section_label = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "live-audio-section-label"))
        .xalign(0.0)
        .build();
    audio_section_label.add_css_class("caption-heading");
    audio_pane.append(&heading_with_icon(AUDIO_ICON, &audio_section_label));

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
        current_scene_label: current_label,
        scenes_box,
        hotkey_hint,
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

pub(crate) fn handle_stream_output_toggle(button: &Button, nav: &NavigationContext) {
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

pub(crate) fn handle_record_output_toggle(button: &Button, nav: &NavigationContext) {
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

/// Small dimmed icon for a section heading.
fn section_icon(icon_name: &str) -> gtk4::Image {
    let icon = gtk4::Image::from_icon_name(icon_name);
    icon.add_css_class("scenedeck-section-icon");
    icon.set_valign(Align::Center);
    icon
}

/// Pair a heading label with an icon without disturbing the caller's handle on
/// the label, which `ui::window` keeps updating.
fn heading_with_icon(icon_name: &str, label: &Label) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .build();
    row.append(&section_icon(icon_name));
    row.append(label);
    row
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
    // Remove all existing children
    // FlowBox requires removing via the FlowBoxChild wrapper
    while let Some(child) = handle.scenes_box.first_child() {
        handle.scenes_box.remove(&child);
    }

    let (registry, hotkeys) = {
        let state = nav.state.borrow();
        (state.registry.clone(), state.config.hotkeys)
    };
    let scene_order = live_scene_order(inventory, &registry);

    if scene_order.is_empty() {
        // Nothing to bind to, so the shortcut caption would only mislead.
        set_hotkey_hint(handle, None);
        let hint = Label::builder()
            .label(fl!(LANGUAGE_LOADER, "live-scenes-no-primary-hint"))
            .wrap(true)
            .xalign(0.0)
            .build();
        hint.add_css_class("dim-label");
        insert_compact_flow_child(&handle.scenes_box, &hint);
        return;
    }

    set_hotkey_hint(handle, hotkeys.hint_label().as_deref());

    for (slot, scene_id) in scene_order.iter().enumerate() {
        let Some(scene) = inventory.scenes.iter().find(|scene| &scene.id == scene_id) else {
            continue;
        };
        let registry_entry = registry.scenes.get(&scene.id);
        let card = scene_card::build(
            SceneCardModel {
                scene_name: &scene.name,
                scene_id: scene.id.clone(),
                scene_role: registry_entry
                    .and_then(|entry| entry.role)
                    .unwrap_or_default(),
                is_active: inventory.current_id.as_deref() == Some(&scene.id),
                is_previous: inventory.previous_id.as_deref() == Some(&scene.id),
                accent_color: registry_entry.and_then(|entry| entry.accent_color.as_deref()),
                shortcut: shortcut_for_slot(&hotkeys, slot),
                icon: registry_entry.and_then(|entry| entry.icon.as_deref()),
            },
            nav.clone(),
        );
        insert_compact_flow_child(&handle.scenes_box, &card);
    }
}

fn shortcut_for_slot(hotkeys: &SceneHotkeyConfig, slot: usize) -> Option<SceneShortcut> {
    Some(SceneShortcut {
        badge: slot_badge(slot)?,
        label: hotkeys.shortcut_label(slot)?,
    })
}

/// Set the caption above the scene grid, hiding it when there is nothing to say.
pub(crate) fn set_hotkey_hint(handle: &LivePageHandle, text: Option<&str>) {
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
    use crate::domain::hotkey::SceneHotkeyStyle;
    use crate::domain::output::OutputRunState;
    use crate::domain::role::SceneRole;
    use crate::domain::scene::Scene;
    use crate::storage::registry::SceneEntry;

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
                    name: (*id).to_string(),
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
}
