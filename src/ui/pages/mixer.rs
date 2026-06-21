//! Dedicated audio Mixer page.
//!
//! This first pass provides the navigation and control surface for mixer modes,
//! scene selection, search, grouping, and scoped audio display. Active mode uses
//! the existing active-scene audio snapshot; selected/pinned scene-specific OBS
//! refresh is intentionally left for the next controller/OBS phase.

use std::collections::BTreeMap;
use std::rc::Rc;

use adw::{prelude::*, ComboRow, EntryRow, PreferencesGroup, PreferencesPage, StatusPage};
use gtk4::{Box as GtkBox, FlowBox, Label, Orientation, PolicyType, ScrolledWindow, StringList};

use crate::controller::command::AppCommand;
use crate::domain::audio::AudioInput;
use crate::domain::mixer::{MixerGrouping, MixerMode};
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::audio_card;

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    root.add_css_class("mixer-page");

    populate(&root, &nav);

    let refresh_fn: Rc<dyn Fn()> = Rc::new({
        let root = root.clone();
        let nav = nav.clone();
        move || rebuild(&root, &nav)
    });

    root.connect_map({
        let refresh = refresh_fn.clone();
        move |_| refresh()
    });

    (root.upcast(), refresh_fn)
}

fn rebuild(root: &GtkBox, nav: &NavigationContext) {
    while let Some(child) = root.first_child() {
        root.remove(&child);
    }
    populate(root, nav);
}

fn populate(root: &GtkBox, nav: &NavigationContext) {
    let state = nav.state.borrow().clone();
    let inventory = state.scene_inventory;
    let mixer = state.mixer;
    let active_scene = inventory.current_id.clone();
    let target_scene = mixer_target_scene(
        mixer.mode,
        active_scene.as_deref(),
        mixer.selected_scene.as_deref(),
        mixer.pinned_scene.as_deref(),
    );

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

    let mode_row = build_mode_row(nav, mixer.mode);
    let scene_row = build_scene_row(nav, &inventory.scenes, mixer.selected_scene.as_deref());
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

    let source_inputs = if mixer.mode == MixerMode::ActiveScene {
        state.audio_inputs
    } else if state.mixer_audio_scene.as_deref() == target_scene.as_deref() {
        state.mixer_audio_inputs
    } else {
        Vec::new()
    };
    let inputs = filter_inputs(&source_inputs, &mixer.search);
    append_mixer_inputs(root, nav, &inputs, mixer.grouping);
}

fn build_mode_row(nav: &NavigationContext, selected: MixerMode) -> ComboRow {
    let model = StringList::new(&["Active", "Selected", "Pinned"]);
    let row = ComboRow::builder()
        .title("Mode")
        .subtitle("Active follows OBS; Selected and Pinned keep the chosen scene stable.")
        .model(&model)
        .selected(mode_to_index(selected))
        .build();

    row.connect_selected_notify({
        let nav = nav.clone();
        move |row| {
            let mode = index_to_mode(row.selected());
            let target_scene = {
                let mut state = nav.state.borrow_mut();
                state.mixer.mode = mode;
                mixer_target_scene(
                    mode,
                    state.scene_inventory.current_id.as_deref(),
                    state.mixer.selected_scene.as_deref(),
                    state.mixer.pinned_scene.as_deref(),
                )
            };
            if mode != MixerMode::ActiveScene {
                if let Some(scene) = target_scene {
                    nav.dispatch(AppCommand::RefreshMixerSceneAudio(scene));
                }
            }
            nav.switch_to_page(crate::controller::state::Page::Mixer);
        }
    });

    row
}

fn build_scene_row(
    nav: &NavigationContext,
    scenes: &[crate::domain::scene::Scene],
    selected_scene: Option<&str>,
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
        let scene_ids: Vec<_> = scenes.iter().map(|scene| scene.id.clone()).collect();
        move |row| {
            if let Some(scene_id) = scene_ids.get(row.selected() as usize) {
                let mut state = nav.state.borrow_mut();
                state.mixer.selected_scene = Some(scene_id.clone());
                if state.mixer.mode == MixerMode::PinnedScene {
                    state.mixer.pinned_scene = Some(scene_id.clone());
                }
            }
            let target_scene = {
                let state = nav.state.borrow();
                mixer_target_scene(
                    state.mixer.mode,
                    state.scene_inventory.current_id.as_deref(),
                    state.mixer.selected_scene.as_deref(),
                    state.mixer.pinned_scene.as_deref(),
                )
            };
            if let Some(scene) = target_scene {
                if nav.state.borrow().mixer.mode != MixerMode::ActiveScene {
                    nav.dispatch(AppCommand::RefreshMixerSceneAudio(scene));
                }
            }
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
    grouping: MixerGrouping,
) {
    if inputs.is_empty() {
        let empty = StatusPage::builder()
            .icon_name("audio-volume-muted-symbolic")
            .title("No Matching Audio Sources")
            .description(
                "Adjust the search filter, choose a scene, or refresh after connecting to OBS.",
            )
            .build();
        root.append(&empty);
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

fn mixer_target_scene(
    mode: MixerMode,
    active_scene: Option<&str>,
    selected_scene: Option<&str>,
    pinned_scene: Option<&str>,
) -> Option<String> {
    match mode {
        MixerMode::ActiveScene => active_scene,
        MixerMode::SelectedScene => selected_scene.or(active_scene),
        MixerMode::PinnedScene => pinned_scene.or(selected_scene).or(active_scene),
    }
    .map(str::to_string)
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
