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
use crate::controller::state::{MixerVisibleAudioStatus, MixerVisibleRenderSource};
use crate::domain::audio::AudioInput;
use crate::domain::mixer::{MixerGrouping, MixerMode, MixerSelection};
use crate::storage::config::write_config;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::audio_card;

type MixerRefreshTracker = Rc<RefCell<Option<String>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MixerRefreshRequestIntent {
    Automatic,
    Explicit,
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
    let target_scene = state.visible_mixer_target_scene().map(str::to_string);

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
            target_scene.as_deref(),
        ))
        .build();
    summary_group.add(&summary);
    page.add(&summary_group);

    root.append(&page);

    let source_inputs = match state.visible_mixer_render_source() {
        MixerVisibleRenderSource::ActiveScene(inputs) => inputs.to_vec(),
        MixerVisibleRenderSource::MissingScene => {
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
    append_mixer_inputs(
        root,
        nav,
        &inputs,
        source_inputs.len(),
        mixer.grouping,
        &mixer.search,
        target_scene.as_deref(),
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
) {
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
        } else {
            append_mixer_status(
                root,
                "edit-find-symbolic",
                "No Matching Audio Sources",
                "Adjust the search filter to show available audio sources.",
            );
        }
        return;
    }

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

fn source_summary(
    mode: MixerMode,
    active_scene: Option<&str>,
    target_scene: Option<&str>,
) -> String {
    match mode {
        MixerMode::ActiveScene => {
            format!(
                "Following active OBS scene: {}",
                active_scene.unwrap_or("-")
            )
        }
        MixerMode::SelectedScene => {
            format!(
                "Selected scene: {}",
                target_scene.unwrap_or("none selected")
            )
        }
        MixerMode::PinnedScene => {
            format!("Pinned scene: {}", target_scene.unwrap_or("none selected"))
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
        .visible_mixer_target_scene()
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
        prepare_mixer_scene_audio_request, should_request_mixer_scene_audio,
        MixerRefreshRequestIntent,
    };
    use crate::controller::command::AppCommand;
    use crate::controller::state::{MixerAudioError, MixerVisibleAudioStatus};

    fn mixer_error() -> MixerAudioError {
        MixerAudioError {
            scene: "scene-a".to_string(),
            message: "failed".to_string(),
        }
    }

    fn loaded_status() -> MixerVisibleAudioStatus<'static> {
        MixerVisibleAudioStatus::Loaded(&[])
    }

    fn command_scene(command: Option<AppCommand>) -> Option<String> {
        match command {
            Some(AppCommand::RefreshMixerSceneAudio(scene)) => Some(scene),
            _ => None,
        }
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
