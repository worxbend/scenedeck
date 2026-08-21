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
use gtk4::{Align, Box as GtkBox, Button, FlowBox, StringList};

use crate::controller::command::AppCommand;
use crate::controller::state::{
    MixerInspectionRenderSourceKind, MixerInspectionSnapshot, MixerInspectionStatus,
    MixerSceneRefreshTarget, MixerSceneRefreshTargetReason, MixerVisibleAudioStatus,
    MixerVisibleRenderSource,
};
use crate::domain::audio::AudioInput;
use crate::domain::mixer::{MixerGrouping, MixerMode};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::audio_service::AudioService;
use crate::ui::navigation::NavigationContext;
use crate::ui::persist::persist_config;
use crate::ui::widgets::audio_card;
use crate::ui::{index_of, insert_compact_flow_child, string_list};
use i18n_embed_fl::fl;

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
    // Tracks which scene an audio refresh has already been requested for, so
    // repeated rebuilds do not re-dispatch the same request. It outlives each
    // rebuild, so it is created here rather than inside `populate`.
    let refresh_tracker: MixerRefreshTracker = Rc::new(RefCell::new(None));

    crate::ui::rebuildable_page("mixer-page", true, move |root| {
        populate(root, &nav, &refresh_tracker);
    })
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
            .title(fl!(LANGUAGE_LOADER, "mixer-empty-title"))
            .description(fl!(LANGUAGE_LOADER, "mixer-empty-description"))
            .build();
        empty.add_css_class("app-status-page");
        root.append(&empty);
        return;
    }

    let page = PreferencesPage::builder()
        .title(fl!(LANGUAGE_LOADER, "mixer-page-title"))
        .vexpand(true)
        .hexpand(true)
        .build();
    page.add_css_class("app-preferences-page");

    let controls = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "mixer-controls-title"))
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
        .title(fl!(LANGUAGE_LOADER, "mixer-summary-title"))
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
                &fl!(LANGUAGE_LOADER, "mixer-no-scene-title"),
                &fl!(LANGUAGE_LOADER, "mixer-no-scene-description"),
            );
            return;
        }
        MixerVisibleRenderSource::Scene { scene, status } => match status {
            MixerVisibleAudioStatus::Loading => {
                clear_tracked_request(refresh_tracker, scene);
                append_loading_placeholder(root, &inspection_snapshot, scene);
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
                append_loading_placeholder(root, &inspection_snapshot, scene);
                return;
            }
        },
    };
    let inputs = filter_inputs(&source_inputs, &mixer.search);
    let inspection_status = append_mixer_inputs(
        MixerSurfaces { root, page: &page },
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
    let labels: Vec<String> = MixerMode::ALL.iter().map(|mode| mode.label()).collect();
    let model = string_list(&labels);
    let row = ComboRow::builder()
        .title(fl!(LANGUAGE_LOADER, "mixer-mode-row-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "mixer-mode-row-subtitle"))
        .model(&model)
        .selected(index_of(&MixerMode::ALL, selected))
        .build();
    row.add_css_class("scenedeck-combo-row");

    row.connect_selected_notify({
        let nav = nav.clone();
        let refresh_tracker = refresh_tracker.clone();
        move |row| {
            let mode = mode_at(row.selected());
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
        .title(fl!(LANGUAGE_LOADER, "mixer-scene-row-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "mixer-scene-row-subtitle"))
        .model(&model)
        .selected(selected)
        .build();
    row.add_css_class("scenedeck-combo-row");

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
    let labels: Vec<String> = MixerGrouping::ALL
        .iter()
        .map(|grouping| grouping.label())
        .collect();
    let model = string_list(&labels);
    let row = ComboRow::builder()
        .title(fl!(LANGUAGE_LOADER, "mixer-grouping-row-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "mixer-grouping-row-subtitle"))
        .model(&model)
        .selected(index_of(&MixerGrouping::ALL, selected))
        .build();
    row.add_css_class("scenedeck-combo-row");

    row.connect_selected_notify({
        let nav = nav.clone();
        move |row| {
            nav.state.borrow_mut().mixer.grouping = grouping_at(row.selected());
            persist_mixer_selection(&nav);
            nav.switch_to_page(crate::controller::state::Page::Mixer);
        }
    });

    row
}

fn build_search_row(nav: &NavigationContext, search: &str) -> EntryRow {
    let row = EntryRow::builder()
        .title(fl!(LANGUAGE_LOADER, "mixer-search-row-title"))
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

/// Where the mixer renders: cards go inside the scrolling preferences page,
/// full-page status views take the whole root.
struct MixerSurfaces<'a> {
    root: &'a GtkBox,
    page: &'a PreferencesPage,
}

fn append_mixer_inputs(
    surfaces: MixerSurfaces<'_>,
    nav: &NavigationContext,
    inputs: &[AudioInput],
    source_count: usize,
    grouping: MixerGrouping,
    search: &str,
    target_scene: Option<&str>,
) -> MixerInspectionStatus<'static> {
    let MixerSurfaces { root, page } = surfaces;
    if inputs.is_empty() {
        if source_count == 0 && search.trim().is_empty() {
            let scene_label = target_scene
                .map(str::to_string)
                .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "mixer-current-scene-fallback"));
            append_mixer_status(
                root,
                "audio-volume-muted-symbolic",
                &fl!(LANGUAGE_LOADER, "mixer-no-audio-sources-title"),
                &fl!(
                    LANGUAGE_LOADER,
                    "mixer-no-audio-sources-description",
                    scene = scene_label
                ),
            );
            MixerInspectionStatus::LoadedNoAudioSources
        } else {
            append_mixer_status(
                root,
                "edit-find-symbolic",
                &fl!(LANGUAGE_LOADER, "mixer-no-matching-title"),
                &fl!(LANGUAGE_LOADER, "mixer-no-matching-description"),
            );
            MixerInspectionStatus::LoadedNoMatchingAudioSourcesAfterFiltering
        }
    } else {
        match grouping {
            MixerGrouping::None => append_group(
                page,
                nav,
                &fl!(LANGUAGE_LOADER, "mixer-group-all-sources"),
                inputs,
            ),
            MixerGrouping::Scope => {
                let mut groups: BTreeMap<String, Vec<AudioInput>> = BTreeMap::new();
                for input in inputs {
                    groups
                        .entry(input.source_scope.label().to_string())
                        .or_default()
                        .push(input.clone());
                }
                for (title, inputs) in groups {
                    append_group(page, nav, &title, &inputs);
                }
            }
            MixerGrouping::ScenePath => {
                let mut groups: BTreeMap<String, Vec<AudioInput>> = BTreeMap::new();
                for input in inputs {
                    groups
                        .entry(
                            input.source_path_label().unwrap_or_else(|| {
                                fl!(LANGUAGE_LOADER, "mixer-group-global-fallback")
                            }),
                        )
                        .or_default()
                        .push(input.clone());
                }
                for (title, inputs) in groups {
                    append_group(page, nav, &title, &inputs);
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
    status.add_css_class("app-status-page");
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
        .title(fl!(LANGUAGE_LOADER, "mixer-error-title"))
        .description(fl!(
            LANGUAGE_LOADER,
            "mixer-error-description",
            scene = scene,
            message = message
        ))
        .build();
    status.add_css_class("app-status-page");
    let retry_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "mixer-retry-button-label"))
        .tooltip_text(fl!(LANGUAGE_LOADER, "mixer-retry-button-tooltip"))
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

/// Add one titled group of audio cards to the preferences page.
///
/// The cards go inside the page rather than beside it: `AdwPreferencesPage`
/// already scrolls, so a tall card cannot push the group past the bottom of the
/// window. A group that owned its own scroller would fight that one and, when
/// the window is short, overflow it.
fn append_group(
    page: &PreferencesPage,
    nav: &NavigationContext,
    title: &str,
    inputs: &[AudioInput],
) {
    let group = PreferencesGroup::builder().title(title).build();
    group.add_css_class("mixer-section");

    let flow = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(5)
        .row_spacing(6)
        .halign(Align::Start)
        .valign(Align::Start)
        .hexpand(true)
        .vexpand(false)
        .min_children_per_line(1)
        .max_children_per_line(12)
        .build();

    for input in inputs {
        let card = audio_card::build(input, nav.clone());
        insert_compact_flow_child(&flow, &card.root);
    }

    group.add(&flow);
    page.add(&group);
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

/// Show the "waiting for this scene's audio" placeholder.
///
/// Two paths render this: the snapshot is already on its way, and the snapshot
/// is missing so one has just been requested. What differs between them is the
/// side effect performed first — clearing the dedupe tracker versus dispatching
/// a refresh — so that stays at each call site and only the identical rendering
/// lives here.
fn append_loading_placeholder(
    root: &GtkBox,
    inspection_snapshot: &MixerInspectionSnapshot<'_>,
    scene: &str,
) {
    emit_mixer_inspection(
        inspection_snapshot,
        MixerInspectionStatus::LoadingPlaceholderShown,
        &[],
        MixerRetryInspection::HIDDEN,
    );
    append_mixer_status(
        root,
        "view-refresh-symbolic",
        &fl!(LANGUAGE_LOADER, "mixer-loading-title"),
        &fl!(LANGUAGE_LOADER, "mixer-loading-description", scene = scene),
    );
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
            serde_json::json!({
                "name": input.name,
                "display_name": input.display_name,
                "muted": input.muted,
                "volume_mul": input.volume_mul,
                "volume_db": input.volume_db,
                "volume_label": AudioService::format_db(AudioService::sanitize_volume_db(input.volume_db)),
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
            fl!(
                LANGUAGE_LOADER,
                "mixer-summary-following-active",
                scene = active_scene.unwrap_or("-")
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
            .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "mixer-summary-no-scene-selected")),
    }
}

fn scene_target_summary(target: MixerSceneRefreshTarget<'_>) -> String {
    match target.reason {
        MixerSceneRefreshTargetReason::DirectSelectedScene => {
            fl!(
                LANGUAGE_LOADER,
                "mixer-summary-selected-scene",
                scene = target.scene
            )
        }
        MixerSceneRefreshTargetReason::DirectPinnedScene => {
            fl!(
                LANGUAGE_LOADER,
                "mixer-summary-pinned-scene",
                scene = target.scene
            )
        }
        MixerSceneRefreshTargetReason::SelectedModeCurrentSceneFallback => {
            fl!(
                LANGUAGE_LOADER,
                "mixer-summary-selected-fallback",
                scene = target.scene
            )
        }
        MixerSceneRefreshTargetReason::PinnedModeSelectedSceneFallback => {
            fl!(
                LANGUAGE_LOADER,
                "mixer-summary-pinned-selected-fallback",
                scene = target.scene
            )
        }
        MixerSceneRefreshTargetReason::PinnedModeCurrentSceneFallback => {
            fl!(
                LANGUAGE_LOADER,
                "mixer-summary-pinned-active-fallback",
                scene = target.scene
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

/// The mode a dropdown row is showing.
///
/// GTK reports "nothing selected" as `u32::MAX`, and a row can also outlive
/// the list it was built from, so an index that is not in `ALL` falls back to
/// the default rather than being treated as an error. That matches what the
/// hand-written index tables this replaced did with their `_` arm.
fn mode_at(index: u32) -> MixerMode {
    MixerMode::ALL
        .get(index as usize)
        .copied()
        .unwrap_or_default()
}

/// The grouping a dropdown row is showing. See [`mode_at`].
fn grouping_at(index: u32) -> MixerGrouping {
    MixerGrouping::ALL
        .get(index as usize)
        .copied()
        .unwrap_or_default()
}

fn persist_mixer_selection(nav: &NavigationContext) {
    // The mixer selection lives in `AppState::mixer` while the page is open;
    // this copies it into the config that gets written.
    let selection = nav.state.borrow().mixer.clone();
    persist_config(nav, move |config| config.mixer = selection);
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
    use crate::domain::audio::AudioInput;
    use crate::domain::mixer::MixerMode;
    use crate::services::audio_service::AudioService;

    fn app_state() -> AppState {
        AppState::new(
            crate::storage::config::AppConfig::default(),
            crate::storage::registry::SceneRegistry::default(),
            None,
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
            ("AboveObsMax", 6.0),
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
                AudioService::format_db(AudioService::sanitize_volume_db(*volume_db)),
                "inspection label should match rendered audio-card formatter for {name}"
            );
        }
    }

    #[test]
    fn mixer_inspection_line_volume_label_uses_visible_card_volume() {
        let mut state = app_state();
        state.mixer.mode = MixerMode::ActiveScene;
        state.scene_inventory.current_id = Some("Program".to_string());
        state.audio_inputs = vec![input("Mic", false, 1.0, -120.0)];
        let snapshot = state.mixer_inspection_snapshot();
        let visible_cards = vec![input("Mic", false, 0.5, -6.24)];

        let json = inspection_json(&format_mixer_inspection_line(
            &snapshot,
            MixerInspectionStatus::LoadedWithVisibleInputCards,
            &visible_cards,
            MixerRetryInspection::HIDDEN,
        ));

        let cards = json["visible_cards"].as_array().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0]["name"], "Mic");
        assert_eq!(cards[0]["volume_db"], -6.24);
        assert_eq!(cards[0]["volume_label"], "-6.2 dB");
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

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::controller::app_controller::AppController;
    use crate::controller::state::AppState;
    use crate::domain::audio::AudioInput;
    use crate::domain::scene::{Scene, SceneInventory};

    fn audio_input(name: &str) -> AudioInput {
        AudioInput::new(name.to_string(), false, 1.0, 0.0)
    }

    /// Build the Mixer page against a state holding `count` audio sources.
    fn measure_mixer_page_min_height(count: usize) -> i32 {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime");
        let (event_tx, _event_rx) = std::sync::mpsc::sync_channel(16);
        let controller = Rc::new(RefCell::new(AppController::new(
            runtime.handle().clone(),
            event_tx,
        )));

        let mut state = AppState::new(Default::default(), Default::default(), None, None);
        state.scene_inventory = SceneInventory {
            scenes: vec![Scene {
                id: "Main".to_string(),
                name: "Main".to_string(),
            }],
            current_id: Some("Main".to_string()),
            previous_id: None,
        };
        state.audio_inputs = (0..count)
            .map(|index| audio_input(&format!("Source {index}")))
            .collect();

        let nav =
            NavigationContext::new(Rc::new(RefCell::new(state)), gtk4::Stack::new(), controller);
        let (widget, _refresh) = build(nav);
        widget.measure(gtk4::Orientation::Vertical, -1).0
    }

    #[test]
    #[ignore = "temporary: needs a display"]
    fn mixer_page_minimum_height_does_not_grow_with_the_card_count() {
        gtk4::init().expect("gtk init");
        crate::ui::register_resources();

        let one = measure_mixer_page_min_height(1);
        let many = measure_mixer_page_min_height(24);

        println!("min height: 1 source = {one}px, 24 sources = {many}px");
        assert_eq!(
            one, many,
            "the page must scroll its cards instead of demanding room for them"
        );
    }
}
