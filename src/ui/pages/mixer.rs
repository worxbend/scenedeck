//! Dedicated audio Mixer page.
//!
//! Active mode uses the app's active-scene audio snapshot. Selected and Pinned
//! modes request scene-specific mixer snapshots through the controller, with
//! UI-side dispatch dedupe for rebuilds and explicit retry semantics for
//! user-driven recovery after scene refresh failures.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use adw::{prelude::*, ComboRow, EntryRow, PreferencesGroup, PreferencesPage, StatusPage};
use gtk4::{
    Box as GtkBox, Button, FlowBox, Label, Orientation, PolicyType, ScrolledWindow, StringList,
};

use crate::controller::command::AppCommand;
use crate::controller::state::{
    MixerInspectionRenderSourceKind, MixerInspectionSnapshot, MixerInspectionStatus,
    MixerSceneRefreshTarget, MixerSceneRefreshTargetReason, MixerVisibleAudioStatus,
    MixerVisibleRenderSource,
};
use crate::domain::audio::AudioInput;
use crate::domain::mixer::{MixerGrouping, MixerMode, MixerSelection};
use crate::services::audio_service::AudioService;
use crate::storage::config::write_config;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::audio_card;

type MixerRefreshTracker = Rc<RefCell<Option<String>>>;
const MIXER_INSPECT_ENV: &str = "SCENEDECK_MIXER_INSPECT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MixerRefreshRequestIntent {
    Automatic,
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MixerRetryInspection {
    visible: bool,
    enabled: bool,
}

impl MixerRetryInspection {
    const HIDDEN: Self = Self {
        visible: false,
        enabled: false,
    };
    const VISIBLE_ENABLED: Self = Self {
        visible: true,
        enabled: true,
    };
}

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    root.add_css_class("mixer-page");

    let refresh_tracker = Rc::new(RefCell::new(None));

    populate(&root, &nav, &refresh_tracker);

    let refresh_fn: Rc<dyn Fn()> = Rc::new({
        let root = root.clone();
        let nav = nav.clone();
        let refresh_tracker = refresh_tracker.clone();
        move || rebuild(&root, &nav, &refresh_tracker)
    });

    root.connect_map({
        let refresh = refresh_fn.clone();
        move |_| refresh()
    });

    (root.upcast(), refresh_fn)
}

fn rebuild(root: &GtkBox, nav: &NavigationContext, refresh_tracker: &MixerRefreshTracker) {
    while let Some(child) = root.first_child() {
        root.remove(&child);
    }
    populate(root, nav, refresh_tracker);
}

fn populate(root: &GtkBox, nav: &NavigationContext, refresh_tracker: &MixerRefreshTracker) {
    let state = nav.state.borrow().clone();
    let inventory = state.scene_inventory.clone();
    let mixer = state.mixer.clone();
    let active_scene = inventory.current_id.clone();
    let target_scene = state.mixer_scene_refresh_target().map(str::to_string);
    let inspection_snapshot = state.mixer_inspection_snapshot();
    let target_details = state
        .mixer_scene_refresh_target_details()
        .map(|target| (target.scene.to_string(), target.reason));

    if inventory.scenes.is_empty() {
        let empty = StatusPage::builder()
            .icon_name("audio-volume-high-symbolic")
            .title("No Mixer Data")
            .description("Connect to OBS to load scenes and audio sources.")
            .build();
        root.append(&empty);
        return;
    }

    let page = PreferencesPage::builder()
        .title("Mixer")
        .vexpand(true)
        .hexpand(true)
        .build();

    let controls = PreferencesGroup::builder()
        .title("Mixer Controls")
        .description(mixer.mode.description())
        .build();

    let mode_row = build_mode_row(nav, mixer.mode, refresh_tracker);
    let scene_row = build_scene_row(
        nav,
        &inventory.scenes,
        mixer.selected_scene.as_deref(),
        refresh_tracker,
    );
    let grouping_row = build_grouping_row(nav, mixer.grouping);
    let search_row = build_search_row(nav, &mixer.search);

    controls.add(&mode_row);
    controls.add(&scene_row);
    controls.add(&grouping_row);
    controls.add(&search_row);
    page.add(&controls);

    let summary_group = PreferencesGroup::new();
    let summary = adw::ActionRow::builder()
        .title("Current Mixer Source")
        .subtitle(source_summary(
            mixer.mode,
            active_scene.as_deref(),
            target_details,
        ))
        .build();
    summary_group.add(&summary);
    page.add(&summary_group);

    root.append(&page);

    let source_inputs = match state.visible_mixer_render_source() {
        MixerVisibleRenderSource::ActiveScene(inputs) => inputs.to_vec(),
        MixerVisibleRenderSource::MissingScene => {
            emit_mixer_inspection(
                &inspection_snapshot,
                MixerInspectionStatus::MissingNoTarget,
                &[],
                MixerRetryInspection::HIDDEN,
            );
            append_mixer_status(
                root,
                "audio-volume-muted-symbolic",
                "No Scene Selected",
                "Choose a scene to load its mixer audio.",
            );
            return;
        }
        MixerVisibleRenderSource::Scene { scene, status } => match status {
            MixerVisibleAudioStatus::Loading => {
                clear_tracked_request(refresh_tracker, scene);
                emit_mixer_inspection(
                    &inspection_snapshot,
                    MixerInspectionStatus::LoadingPlaceholderShown,
                    &[],
                    MixerRetryInspection::HIDDEN,
                );
                append_mixer_status(
                    root,
                    "view-refresh-symbolic",
                    "Loading Mixer Audio",
                    &format!("Loading audio sources for {scene}."),
                );
                return;
            }
            MixerVisibleAudioStatus::Error(error) => {
                clear_tracked_request(refresh_tracker, scene);
                emit_mixer_inspection(
                    &inspection_snapshot,
                    MixerInspectionStatus::ErrorPlaceholderShown(error.message.as_str()),
                    &[],
                    MixerRetryInspection::VISIBLE_ENABLED,
                );
                append_mixer_error_status(root, nav, refresh_tracker, scene, &error.message);
                return;
            }
            MixerVisibleAudioStatus::Loaded(inputs) => {
                clear_tracked_request(refresh_tracker, scene);
                inputs.to_vec()
            }
            MixerVisibleAudioStatus::Missing => {
                request_visible_mixer_scene_audio(
                    nav,
                    refresh_tracker,
                    MixerRefreshRequestIntent::Automatic,
                );
                emit_mixer_inspection(
                    &inspection_snapshot,
                    MixerInspectionStatus::LoadingPlaceholderShown,
                    &[],
                    MixerRetryInspection::HIDDEN,
                );
                append_mixer_status(
                    root,
                    "view-refresh-symbolic",
                    "Loading Mixer Audio",
                    &format!("Loading audio sources for {scene}."),
                );
                return;
            }
        },
    };
    let inputs = filter_inputs(&source_inputs, &mixer.search);
    let inspection_status = append_mixer_inputs(
        root,
        nav,
        &inputs,
        source_inputs.len(),
        mixer.grouping,
        &mixer.search,
        target_scene.as_deref(),
    );
    emit_mixer_inspection(
        &inspection_snapshot,
        inspection_status,
        &inputs,
        MixerRetryInspection::HIDDEN,
    );
}

fn build_mode_row(
    nav: &NavigationContext,
    selected: MixerMode,
    refresh_tracker: &MixerRefreshTracker,
) -> ComboRow {
    let model = StringList::new(&["Active", "Selected", "Pinned"]);
    let row = ComboRow::builder()
        .title("Mode")
        .subtitle("Active follows OBS; Selected and Pinned keep the chosen scene stable.")
        .model(&model)
        .selected(mode_to_index(selected))
        .build();

    row.connect_selected_notify({
        let nav = nav.clone();
        let refresh_tracker = refresh_tracker.clone();
        move |row| {
            let mode = index_to_mode(row.selected());
            {
                let mut state = nav.state.borrow_mut();
                state.mixer.mode = mode;
            }
            request_visible_mixer_scene_audio(
                &nav,
                &refresh_tracker,
                MixerRefreshRequestIntent::Explicit,
            );
            persist_mixer_selection(&nav);
            nav.switch_to_page(crate::controller::state::Page::Mixer);
        }
    });

    row
}

fn build_scene_row(
    nav: &NavigationContext,
    scenes: &[crate::domain::scene::Scene],
    selected_scene: Option<&str>,
    refresh_tracker: &MixerRefreshTracker,
) -> ComboRow {
    let names: Vec<&str> = scenes.iter().map(|scene| scene.name.as_str()).collect();
    let model = StringList::new(&names);
    let selected = selected_scene
        .and_then(|selected| scenes.iter().position(|scene| scene.id == selected))
        .unwrap_or(0) as u32;

    let row = ComboRow::builder()
        .title("Scene")
        .subtitle("Used by Selected and Pinned modes.")
        .model(&model)
        .selected(selected)
        .build();

    row.connect_selected_notify({
        let nav = nav.clone();
        let refresh_tracker = refresh_tracker.clone();
        let scene_ids: Vec<_> = scenes.iter().map(|scene| scene.id.clone()).collect();
        move |row| {
            if let Some(scene_id) = scene_ids.get(row.selected() as usize) {
                let mut state = nav.state.borrow_mut();
                state.mixer.selected_scene = Some(scene_id.clone());
                if state.mixer.mode == MixerMode::PinnedScene {
                    state.mixer.pinned_scene = Some(scene_id.clone());
                }
            }
            request_visible_mixer_scene_audio(
                &nav,
                &refresh_tracker,
                MixerRefreshRequestIntent::Explicit,
            );
            persist_mixer_selection(&nav);
            nav.switch_to_page(crate::controller::state::Page::Mixer);
        }
    });

    row
}

fn build_grouping_row(nav: &NavigationContext, selected: MixerGrouping) -> ComboRow {
    let model = StringList::new(&["Scope", "Scene Path", "None"]);
    let row = ComboRow::builder()
        .title("Group By")
        .subtitle("Controls how audio sources are arranged below.")
        .model(&model)
        .selected(grouping_to_index(selected))
        .build();

    row.connect_selected_notify({
        let nav = nav.clone();
        move |row| {
            nav.state.borrow_mut().mixer.grouping = index_to_grouping(row.selected());
            persist_mixer_selection(&nav);
            nav.switch_to_page(crate::controller::state::Page::Mixer);
        }
    });

    row
}

fn build_search_row(nav: &NavigationContext, search: &str) -> EntryRow {
    let row = EntryRow::builder()
        .title("Search")
        .text(search)
        .show_apply_button(true)
        .build();

    row.connect_apply({
        let nav = nav.clone();
        move |row| {
            nav.state.borrow_mut().mixer.search = row.text().trim().to_string();
            nav.switch_to_page(crate::controller::state::Page::Mixer);
        }
    });

    row
}

fn append_mixer_inputs(
    root: &GtkBox,
    nav: &NavigationContext,
    inputs: &[AudioInput],
    source_count: usize,
    grouping: MixerGrouping,
    search: &str,
    target_scene: Option<&str>,
) -> MixerInspectionStatus<'static> {
    if inputs.is_empty() {
        if source_count == 0 && search.trim().is_empty() {
            append_mixer_status(
                root,
                "audio-volume-muted-symbolic",
                "No Audio Sources",
                &format!(
                    "{} has no matching configured OBS audio sources.",
                    target_scene.unwrap_or("The current scene")
                ),
            );
            MixerInspectionStatus::LoadedNoAudioSources
        } else {
            append_mixer_status(
                root,
                "edit-find-symbolic",
                "No Matching Audio Sources",
                "Adjust the search filter to show available audio sources.",
            );
            MixerInspectionStatus::LoadedNoMatchingAudioSourcesAfterFiltering
        }
    } else {
        match grouping {
            MixerGrouping::None => append_group(root, nav, "All Sources", inputs),
            MixerGrouping::Scope => {
                let mut groups: BTreeMap<String, Vec<AudioInput>> = BTreeMap::new();
                for input in inputs {
                    groups
                        .entry(input.source_scope.label().to_string())
                        .or_default()
                        .push(input.clone());
                }
                for (title, inputs) in groups {
                    append_group(root, nav, &title, &inputs);
                }
            }
            MixerGrouping::ScenePath => {
                let mut groups: BTreeMap<String, Vec<AudioInput>> = BTreeMap::new();
                for input in inputs {
                    groups
                        .entry(
                            input
                                .source_path_label()
                                .unwrap_or_else(|| "Global".to_string()),
                        )
                        .or_default()
                        .push(input.clone());
                }
                for (title, inputs) in groups {
                    append_group(root, nav, &title, &inputs);
                }
            }
        }

        MixerInspectionStatus::LoadedWithVisibleInputCards
    }
}

fn append_mixer_status(root: &GtkBox, icon_name: &str, title: &str, description: &str) {
    let status = StatusPage::builder()
        .icon_name(icon_name)
        .title(title)
        .description(description)
        .build();
    root.append(&status);
}

fn append_mixer_error_status(
    root: &GtkBox,
    nav: &NavigationContext,
    refresh_tracker: &MixerRefreshTracker,
    scene: &str,
    message: &str,
) {
    let status = StatusPage::builder()
        .icon_name("dialog-warning-symbolic")
        .title("Mixer Audio Unavailable")
        .description(format!(
            "Could not load audio sources for {scene}: {message}"
        ))
        .build();
    let retry_btn = Button::builder()
        .label("Retry")
        .tooltip_text("Retry loading mixer audio")
        .build();
    retry_btn.add_css_class("suggested-action");
    retry_btn.connect_clicked({
        let nav = nav.clone();
        let refresh_tracker = refresh_tracker.clone();
        move |_| {
            request_visible_mixer_scene_audio(
                &nav,
                &refresh_tracker,
                MixerRefreshRequestIntent::Explicit,
            );
        }
    });
    status.set_child(Some(&retry_btn));
    root.append(&status);
}

fn append_group(root: &GtkBox, nav: &NavigationContext, title: &str, inputs: &[AudioInput]) {
    let section = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    section.add_css_class("mixer-section");

    let label = Label::builder().label(title).xalign(0.0).build();
    label.add_css_class("caption-heading");
    section.append(&label);

    let flow = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(10)
        .row_spacing(10)
        .min_children_per_line(1)
        .max_children_per_line(8)
        .build();

    for input in inputs {
        let card = audio_card::build(input, nav.clone());
        flow.insert(&card.root, -1);
    }

    let scroll = ScrolledWindow::builder()
        .vexpand(false)
        .hexpand(true)
        .min_content_height(190)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&flow)
        .build();
    scroll.add_css_class("live-pane-scroll");
    section.append(&scroll);
    root.append(&section);
}

fn filter_inputs(inputs: &[AudioInput], search: &str) -> Vec<AudioInput> {
    let needle = search.trim().to_lowercase();
    if needle.is_empty() {
        return inputs.to_vec();
    }

    inputs
        .iter()
        .filter(|input| {
            input.name.to_lowercase().contains(&needle)
                || input.display_name.to_lowercase().contains(&needle)
                || input
                    .source_path_label()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle)
        })
        .cloned()
        .collect()
}

fn emit_mixer_inspection(
    snapshot: &MixerInspectionSnapshot<'_>,
    status: MixerInspectionStatus<'_>,
    visible_cards: &[AudioInput],
    retry: MixerRetryInspection,
) {
    if std::env::var(MIXER_INSPECT_ENV).ok().as_deref() == Some("1") {
        eprintln!(
            "{}",
            format_mixer_inspection_line(snapshot, status, visible_cards, retry)
        );
    }
}

fn format_mixer_inspection_line(
    snapshot: &MixerInspectionSnapshot<'_>,
    status: MixerInspectionStatus<'_>,
    visible_cards: &[AudioInput],
    retry: MixerRetryInspection,
) -> String {
    let cards: Vec<_> = visible_cards
        .iter()
        .map(|input| {
            let volume_db = snapshot
                .inputs
                .iter()
                .find(|snapshot_input| snapshot_input.name == input.name.as_str())
                .map(|snapshot_input| snapshot_input.volume_db)
                .unwrap_or(input.volume_db);
            serde_json::json!({
                "name": input.name,
                "display_name": input.display_name,
                "muted": input.muted,
                "volume_mul": input.volume_mul,
                "volume_db": input.volume_db,
                "volume_label": AudioService::format_db(volume_db),
            })
        })
        .collect();

    let line = serde_json::json!({
        "event": "mixer_inspect",
        "mode": mixer_mode_inspection_label(snapshot.mode),
        "selected_scene": snapshot.selected_scene,
        "pinned_scene": snapshot.pinned_scene,
        "refresh_target": snapshot.refresh_target.map(|target| target.scene),
        "refresh_reason": snapshot
            .refresh_target
            .map(|target| mixer_refresh_reason_inspection_label(target.reason)),
        "render_source": mixer_render_source_inspection_label(snapshot.render_source_kind),
        "render_scene": snapshot.scene,
        "status": mixer_status_inspection_value(status),
        "visible_cards": cards,
        "retry": {
            "visible": retry.visible,
            "enabled": retry.enabled,
        },
    });
    format!("scenedeck_mixer_inspect {line}")
}

fn mixer_status_inspection_value(status: MixerInspectionStatus<'_>) -> serde_json::Value {
    match status {
        MixerInspectionStatus::LoadedWithVisibleInputCards => {
            serde_json::json!({ "kind": "loaded_with_visible_input_cards" })
        }
        MixerInspectionStatus::LoadedNoAudioSources => {
            serde_json::json!({ "kind": "loaded_no_audio_sources" })
        }
        MixerInspectionStatus::LoadedNoMatchingAudioSourcesAfterFiltering => {
            serde_json::json!({ "kind": "loaded_no_matching_audio_sources_after_filtering" })
        }
        MixerInspectionStatus::LoadingPlaceholderShown => {
            serde_json::json!({ "kind": "loading_placeholder_shown" })
        }
        MixerInspectionStatus::ErrorPlaceholderShown(message) => {
            serde_json::json!({ "kind": "error", "message": message })
        }
        MixerInspectionStatus::MissingNoTarget => {
            serde_json::json!({ "kind": "missing_no_target" })
        }
    }
}

fn mixer_mode_inspection_label(mode: MixerMode) -> &'static str {
    match mode {
        MixerMode::ActiveScene => "active",
        MixerMode::SelectedScene => "selected",
        MixerMode::PinnedScene => "pinned",
    }
}

fn mixer_render_source_inspection_label(kind: MixerInspectionRenderSourceKind) -> &'static str {
    match kind {
        MixerInspectionRenderSourceKind::ActiveScene => "active_scene",
        MixerInspectionRenderSourceKind::Scene => "scene",
        MixerInspectionRenderSourceKind::MissingScene => "missing_scene",
    }
}

fn mixer_refresh_reason_inspection_label(reason: MixerSceneRefreshTargetReason) -> &'static str {
    match reason {
        MixerSceneRefreshTargetReason::DirectSelectedScene => "direct_selected_scene",
        MixerSceneRefreshTargetReason::DirectPinnedScene => "direct_pinned_scene",
        MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback => {
            "selected_mode_current_scene_fallback"
        }
        MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback => {
            "pinned_mode_selected_scene_fallback"
        }
        MixerSceneRefreshTargetReason::PinnedModeCurrentSceneFallback => {
            "pinned_mode_current_scene_fallback"
        }
    }
}

fn source_summary(
    mode: MixerMode,
    active_scene: Option<&str>,
    target: Option<(String, MixerSceneRefreshTargetReason)>,
) -> String {
    match mode {
        MixerMode::ActiveScene => {
            format!(
                "Following active OBS scene: {}",
                active_scene.unwrap_or("-")
            )
        }
        MixerMode::SelectedScene | MixerMode::PinnedScene => target
            .map(|(scene, reason)| {
                let target = MixerSceneRefreshTarget {
                    scene: scene.as_str(),
                    reason,
                };
                scene_target_summary(target)
            })
            .unwrap_or_else(|| "No scene selected".to_string()),
    }
}

fn scene_target_summary(target: MixerSceneRefreshTarget<'_>) -> String {
    match target.reason {
        MixerSceneRefreshTargetReason::DirectSelectedScene => {
            format!("Selected scene: {}", target.scene)
        }
        MixerSceneRefreshTargetReason::DirectPinnedScene => {
            format!("Pinned scene: {}", target.scene)
        }
        MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback => {
            format!(
                "Selected scene not set; using active OBS scene: {}",
                target.scene
            )
        }
        MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback => {
            format!(
                "Pinned scene not set; using selected scene: {}",
                target.scene
            )
        }
        MixerSceneRefreshTargetReason::PinnedModeCurrentSceneFallback => {
            format!(
                "Pinned and selected scenes not set; using active OBS scene: {}",
                target.scene
            )
        }
    }
}

fn request_visible_mixer_scene_audio(
    nav: &NavigationContext,
    refresh_tracker: &MixerRefreshTracker,
    intent: MixerRefreshRequestIntent,
) {
    let target_scene = nav
        .state
        .borrow()
        .mixer_scene_refresh_target()
        .map(str::to_string);

    if let Some(scene) = target_scene {
        request_mixer_scene_audio(nav, refresh_tracker, &scene, intent);
    }
}

fn request_mixer_scene_audio(
    nav: &NavigationContext,
    refresh_tracker: &MixerRefreshTracker,
    scene: &str,
    intent: MixerRefreshRequestIntent,
) {
    let command = {
        let state = nav.state.borrow();
        let mut tracked_scene = refresh_tracker.borrow_mut();
        prepare_mixer_scene_audio_request(
            intent,
            scene,
            state.visible_mixer_audio_status(scene),
            &mut tracked_scene,
        )
    };

    if let Some(command) = command {
        nav.dispatch(command);
    }
}

fn prepare_mixer_scene_audio_request(
    intent: MixerRefreshRequestIntent,
    scene: &str,
    visible_status: MixerVisibleAudioStatus<'_>,
    tracked_scene: &mut Option<String>,
) -> Option<AppCommand> {
    if !should_request_mixer_scene_audio(intent, visible_status, scene, tracked_scene.as_deref()) {
        return None;
    }

    *tracked_scene = Some(scene.to_string());
    Some(AppCommand::RefreshMixerSceneAudio(scene.to_string()))
}

pub(crate) fn should_request_mixer_scene_audio(
    intent: MixerRefreshRequestIntent,
    visible_status: MixerVisibleAudioStatus<'_>,
    scene: &str,
    tracked_scene: Option<&str>,
) -> bool {
    if tracked_scene == Some(scene) {
        return false;
    }

    match visible_status {
        MixerVisibleAudioStatus::Loading | MixerVisibleAudioStatus::Loaded(_) => false,
        MixerVisibleAudioStatus::Error(_) => match intent {
            MixerRefreshRequestIntent::Automatic => false,
            MixerRefreshRequestIntent::Explicit => true,
        },
        MixerVisibleAudioStatus::Missing => true,
    }
}

fn clear_tracked_request(refresh_tracker: &MixerRefreshTracker, scene: &str) {
    let mut tracked_scene = refresh_tracker.borrow_mut();
    if tracked_scene.as_deref() == Some(scene) {
        *tracked_scene = None;
    }
}

fn mode_to_index(mode: MixerMode) -> u32 {
    match mode {
        MixerMode::ActiveScene => 0,
        MixerMode::SelectedScene => 1,
        MixerMode::PinnedScene => 2,
    }
}

fn index_to_mode(index: u32) -> MixerMode {
    match index {
        1 => MixerMode::SelectedScene,
        2 => MixerMode::PinnedScene,
        _ => MixerMode::ActiveScene,
    }
}

fn grouping_to_index(grouping: MixerGrouping) -> u32 {
    match grouping {
        MixerGrouping::Scope => 0,
        MixerGrouping::ScenePath => 1,
        MixerGrouping::None => 2,
    }
}

fn index_to_grouping(index: u32) -> MixerGrouping {
    match index {
        1 => MixerGrouping::ScenePath,
        2 => MixerGrouping::None,
        _ => MixerGrouping::Scope,
    }
}

fn persist_mixer_selection(nav: &NavigationContext) {
    let selection = nav.state.borrow().mixer.clone();
    if let Err(err) = write_mixer_selection(selection) {
        tracing::warn!(%err, "failed to save mixer preference");
    }
}

fn write_mixer_selection(selection: MixerSelection) -> Result<(), std::io::Error> {
    let mut cfg = crate::storage::config::read_config().config;
    cfg.mixer = selection;
    write_config(&cfg)
}

#[cfg(test)]
mod tests {
    use super::{
        format_mixer_inspection_line, prepare_mixer_scene_audio_request,
        should_request_mixer_scene_audio, source_summary, MixerRefreshRequestIntent,
        MixerRetryInspection,
    };
    use crate::controller::command::AppCommand;
    use crate::controller::state::{
        AppState, MixerAudioError, MixerInspectionStatus, MixerSceneRefreshTargetReason,
        MixerVisibleAudioStatus,
    };
    use crate::domain::appearance::ThemeMode;
    use crate::domain::audio::AudioInput;
    use crate::domain::mixer::{MixerMode, MixerSelection};
    use crate::services::audio_service::AudioService;
    use crate::storage::config::OutputConfig;

    fn app_state() -> AppState {
        AppState::new(
            ThemeMode::default(),
            MixerSelection::default(),
            OutputConfig::default(),
            None,
        )
    }

    fn summary_target_details(state: &AppState) -> Option<(String, MixerSceneRefreshTargetReason)> {
        state
            .mixer_scene_refresh_target_details()
            .map(|target| (target.scene.to_string(), target.reason))
    }

    fn mixer_summary(state: &AppState) -> String {
        source_summary(
            state.mixer.mode,
            state.scene_inventory.current_id.as_deref(),
            summary_target_details(state),
        )
    }

    fn mixer_error() -> MixerAudioError {
        MixerAudioError {
            scene: "scene-a".to_string(),
            message: "failed".to_string(),
        }
    }

    fn loaded_status() -> MixerVisibleAudioStatus<'static> {
        MixerVisibleAudioStatus::Loaded(&[])
    }

    fn input(id: &str, muted: bool, volume_mul: f64, volume_db: f64) -> AudioInput {
        let mut input = AudioInput::new(id.to_string(), muted, volume_mul, volume_db);
        input.display_name = format!("{id} Display");
        input
    }

    fn inspection_json(line: &str) -> serde_json::Value {
        let payload = line
            .strip_prefix("scenedeck_mixer_inspect ")
            .expect("inspection line prefix");
        serde_json::from_str(payload).expect("valid inspection json")
    }

    fn inspection_status_kind(json: &serde_json::Value) -> &str {
        json["status"]["kind"]
            .as_str()
            .expect("inspection status kind")
    }

    fn command_scene(command: Option<AppCommand>) -> Option<String> {
        match command {
            Some(AppCommand::RefreshMixerSceneAudio(scene)) => Some(scene),
            _ => None,
        }
    }

    #[test]
    fn active_mode_summary_follows_active_obs_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Program".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());

        assert_eq!(mixer_summary(&state), "Following active OBS scene: Program");
    }

    #[test]
    fn selected_mode_summary_names_explicit_selected_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Program".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());

        assert_eq!(mixer_summary(&state), "Selected scene: Selected");
    }

    #[test]
    fn pinned_mode_summary_names_explicit_pinned_scene() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Program".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());

        assert_eq!(mixer_summary(&state), "Pinned scene: Pinned");
    }

    #[test]
    fn selected_mode_summary_describes_current_scene_fallback() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Program".to_string());

        assert_eq!(
            mixer_summary(&state),
            "Selected scene not set; using active OBS scene: Program"
        );
    }

    #[test]
    fn pinned_mode_summary_describes_selected_scene_fallback() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Program".to_string());
        state.mixer.selected_scene = Some("Selected".to_string());

        assert_eq!(
            mixer_summary(&state),
            "Pinned scene not set; using selected scene: Selected"
        );
    }

    #[test]
    fn pinned_mode_summary_describes_current_scene_fallback() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Program".to_string());

        assert_eq!(
            mixer_summary(&state),
            "Pinned and selected scenes not set; using active OBS scene: Program"
        );
    }

    #[test]
    fn scene_specific_mode_summary_reports_no_scene_without_target_or_fallback() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;

        assert_eq!(mixer_summary(&state), "No scene selected");
    }

    #[test]
    fn mixer_inspection_line_reports_visible_cards() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Program".to_string());
        state.audio_inputs = vec![
            input("Music", true, 0.5, -6.24),
            input("Mic", false, 1.0, 0.0),
        ];
        let visible_cards = vec![state.audio_inputs[0].clone()];
        let snapshot = state.mixer_inspection_snapshot();

        let json = inspection_json(&format_mixer_inspection_line(
            &snapshot,
            MixerInspectionStatus::LoadedWithVisibleInputCards,
            &visible_cards,
            MixerRetryInspection::HIDDEN,
        ));

        assert_eq!(json["event"], "mixer_inspect");
        assert_eq!(json["mode"], "active");
        assert_eq!(json["render_source"], "active_scene");
        assert_eq!(json["render_scene"], "Program");
        assert_eq!(json["status"]["kind"], "loaded_with_visible_input_cards");
        assert_eq!(json["retry"]["visible"], false);
        let cards = json["visible_cards"].as_array().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0]["name"], "Music");
        assert_eq!(cards[0]["display_name"], "Music Display");
        assert_eq!(cards[0]["muted"], true);
        assert_eq!(cards[0]["volume_mul"], 0.5);
        assert_eq!(cards[0]["volume_db"], -6.24);
        assert_eq!(cards[0]["volume_label"], "-6.2 dB");
    }

    #[test]
    fn mixer_inspection_line_volume_labels_match_audio_card_formatter() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Program".to_string());
        let volume_cases = [
            ("NegInf", f64::NEG_INFINITY),
            ("BelowFloor", -120.0),
            ("NearZeroPositive", 0.01),
            ("NearZeroNegative", -0.01),
            ("Zero", 0.0),
            ("Normal", -6.24),
        ];
        state.audio_inputs = volume_cases
            .iter()
            .map(|(name, volume_db)| input(name, false, 1.0, *volume_db))
            .collect();
        let visible_cards = state.audio_inputs.clone();
        let snapshot = state.mixer_inspection_snapshot();

        let json = inspection_json(&format_mixer_inspection_line(
            &snapshot,
            MixerInspectionStatus::LoadedWithVisibleInputCards,
            &visible_cards,
            MixerRetryInspection::HIDDEN,
        ));

        let cards = json["visible_cards"].as_array().unwrap();
        assert_eq!(cards.len(), volume_cases.len());
        for ((name, volume_db), card) in volume_cases.iter().zip(cards) {
            assert_eq!(card["name"], *name);
            assert_eq!(
                card["volume_label"],
                AudioService::format_db(*volume_db),
                "inspection label should match rendered audio-card formatter for {name}"
            );
        }
    }

    #[test]
    fn mixer_inspection_line_reports_error_and_retry_state() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Scene A".to_string());
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());
        let snapshot = state.mixer_inspection_snapshot();

        let json = inspection_json(&format_mixer_inspection_line(
            &snapshot,
            MixerInspectionStatus::ErrorPlaceholderShown("OBS failed"),
            &[],
            MixerRetryInspection::VISIBLE_ENABLED,
        ));

        assert_eq!(json["mode"], "selected");
        assert_eq!(json["refresh_target"], "Scene A");
        assert_eq!(json["refresh_reason"], "direct_selected_scene");
        assert_eq!(json["render_source"], "scene");
        assert_eq!(json["render_scene"], "Scene A");
        assert_eq!(json["status"]["kind"], "error");
        assert_eq!(json["status"]["message"], "OBS failed");
        assert_eq!(json["visible_cards"].as_array().unwrap().len(), 0);
        assert_eq!(json["retry"]["visible"], true);
        assert_eq!(json["retry"]["enabled"], true);
    }

    #[test]
    fn mixer_inspection_line_reports_loading_placeholder_after_missing_automatic_request() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Scene A".to_string());
        let snapshot = state.mixer_inspection_snapshot();
        assert_eq!(snapshot.status, MixerInspectionStatus::MissingNoTarget);

        let mut tracked_scene = None;
        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Automatic,
            "Scene A",
            MixerVisibleAudioStatus::Missing,
            &mut tracked_scene,
        );
        assert_eq!(command_scene(command).as_deref(), Some("Scene A"));
        assert_eq!(tracked_scene.as_deref(), Some("Scene A"));

        let json = inspection_json(&format_mixer_inspection_line(
            &snapshot,
            MixerInspectionStatus::LoadingPlaceholderShown,
            &[],
            MixerRetryInspection::HIDDEN,
        ));

        assert_eq!(json["mode"], "selected");
        assert_eq!(json["refresh_target"], "Scene A");
        assert_eq!(json["refresh_reason"], "direct_selected_scene");
        assert_eq!(json["render_source"], "scene");
        assert_eq!(json["render_scene"], "Scene A");
        assert_eq!(inspection_status_kind(&json), "loading_placeholder_shown");
        assert_ne!(inspection_status_kind(&json), "missing_no_target");
        assert_eq!(json["visible_cards"].as_array().unwrap().len(), 0);
        assert_eq!(json["retry"]["visible"], false);
    }

    #[test]
    fn mixer_inspection_line_reports_loaded_empty_audio_sources() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Scene A".to_string());
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), Vec::new());
        let snapshot = state.mixer_inspection_snapshot();
        assert_eq!(snapshot.status, MixerInspectionStatus::LoadedNoAudioSources);

        let json = inspection_json(&format_mixer_inspection_line(
            &snapshot,
            MixerInspectionStatus::LoadedNoAudioSources,
            &[],
            MixerRetryInspection::HIDDEN,
        ));

        assert_eq!(json["mode"], "selected");
        assert_eq!(json["render_source"], "scene");
        assert_eq!(json["render_scene"], "Scene A");
        assert_eq!(inspection_status_kind(&json), "loaded_no_audio_sources");
        assert_ne!(inspection_status_kind(&json), "loading_placeholder_shown");
        assert_ne!(inspection_status_kind(&json), "missing_no_target");
        assert_ne!(inspection_status_kind(&json), "error");
        assert_ne!(
            inspection_status_kind(&json),
            "loaded_no_matching_audio_sources_after_filtering"
        );
        assert_eq!(json["visible_cards"].as_array().unwrap().len(), 0);
        assert_eq!(json["retry"]["visible"], false);
    }

    #[test]
    fn mixer_inspection_line_reports_loaded_filtered_empty_audio_sources() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Scene A".to_string());
        state.mixer.search = "does-not-match".to_string();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic", false, 1.0, 0.0)]);
        let snapshot = state.mixer_inspection_snapshot();
        assert_eq!(
            snapshot.status,
            MixerInspectionStatus::LoadedWithVisibleInputCards
        );

        let json = inspection_json(&format_mixer_inspection_line(
            &snapshot,
            MixerInspectionStatus::LoadedNoMatchingAudioSourcesAfterFiltering,
            &[],
            MixerRetryInspection::HIDDEN,
        ));

        assert_eq!(json["mode"], "selected");
        assert_eq!(json["render_source"], "scene");
        assert_eq!(json["render_scene"], "Scene A");
        assert_eq!(
            inspection_status_kind(&json),
            "loaded_no_matching_audio_sources_after_filtering"
        );
        assert_ne!(inspection_status_kind(&json), "loaded_no_audio_sources");
        assert_ne!(inspection_status_kind(&json), "loading_placeholder_shown");
        assert_ne!(inspection_status_kind(&json), "missing_no_target");
        assert_ne!(inspection_status_kind(&json), "error");
        assert_eq!(json["visible_cards"].as_array().unwrap().len(), 0);
        assert_eq!(json["retry"]["visible"], false);
    }

    #[test]
    fn automatic_request_dedupes_matching_failure() {
        let error = mixer_error();

        assert!(!should_request_mixer_scene_audio(
            MixerRefreshRequestIntent::Automatic,
            MixerVisibleAudioStatus::Error(&error),
            "scene-a",
            None,
        ));
    }

    #[test]
    fn explicit_request_retries_matching_failure() {
        let error = mixer_error();

        assert!(should_request_mixer_scene_audio(
            MixerRefreshRequestIntent::Explicit,
            MixerVisibleAudioStatus::Error(&error),
            "scene-a",
            None,
        ));
    }

    #[test]
    fn request_dedupes_loaded_scene() {
        for intent in [
            MixerRefreshRequestIntent::Automatic,
            MixerRefreshRequestIntent::Explicit,
        ] {
            assert!(!should_request_mixer_scene_audio(
                intent,
                loaded_status(),
                "scene-a",
                None,
            ));
        }
    }

    #[test]
    fn request_dedupes_in_flight_scene() {
        for intent in [
            MixerRefreshRequestIntent::Automatic,
            MixerRefreshRequestIntent::Explicit,
        ] {
            assert!(!should_request_mixer_scene_audio(
                intent,
                MixerVisibleAudioStatus::Loading,
                "scene-a",
                None,
            ));
        }
    }

    #[test]
    fn request_dedupes_tracked_scene() {
        for intent in [
            MixerRefreshRequestIntent::Automatic,
            MixerRefreshRequestIntent::Explicit,
        ] {
            assert!(!should_request_mixer_scene_audio(
                intent,
                MixerVisibleAudioStatus::Missing,
                "scene-a",
                Some("scene-a"),
            ));
        }
    }

    #[test]
    fn adapter_tracks_and_dispatches_missing_request_once() {
        let mut tracked_scene = None;

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Automatic,
            "scene-a",
            MixerVisibleAudioStatus::Missing,
            &mut tracked_scene,
        );

        assert_eq!(command_scene(command).as_deref(), Some("scene-a"));
        assert_eq!(tracked_scene.as_deref(), Some("scene-a"));

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Automatic,
            "scene-a",
            MixerVisibleAudioStatus::Missing,
            &mut tracked_scene,
        );

        assert_eq!(command_scene(command), None);
        assert_eq!(tracked_scene.as_deref(), Some("scene-a"));
    }

    #[test]
    fn adapter_does_not_loop_automatic_rebuild_after_failure() {
        let mut tracked_scene = None;
        let error = mixer_error();

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Automatic,
            "scene-a",
            MixerVisibleAudioStatus::Error(&error),
            &mut tracked_scene,
        );

        assert_eq!(command_scene(command), None);
        assert_eq!(tracked_scene, None);
    }

    #[test]
    fn adapter_allows_one_explicit_retry_after_failure() {
        let mut tracked_scene = None;
        let error = mixer_error();

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Explicit,
            "scene-a",
            MixerVisibleAudioStatus::Error(&error),
            &mut tracked_scene,
        );

        assert_eq!(command_scene(command).as_deref(), Some("scene-a"));
        assert_eq!(tracked_scene.as_deref(), Some("scene-a"));

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Explicit,
            "scene-a",
            MixerVisibleAudioStatus::Error(&error),
            &mut tracked_scene,
        );

        assert_eq!(command_scene(command), None);
    }

    #[test]
    fn adapter_dedupes_explicit_retry_while_loading() {
        let mut tracked_scene = None;

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Explicit,
            "scene-a",
            MixerVisibleAudioStatus::Loading,
            &mut tracked_scene,
        );

        assert_eq!(command_scene(command), None);
        assert_eq!(tracked_scene, None);
    }

    #[test]
    fn adapter_dedupes_explicit_retry_while_tracked() {
        let mut tracked_scene = Some("scene-a".to_string());
        let error = mixer_error();

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Explicit,
            "scene-a",
            MixerVisibleAudioStatus::Error(&error),
            &mut tracked_scene,
        );

        assert_eq!(command_scene(command), None);
        assert_eq!(tracked_scene.as_deref(), Some("scene-a"));
    }
}
