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

type MixerRefreshTracker = Rc<MixerPageHandle>;

/// Stable visible cards. Audio echoes never replace controls or their draft state.
pub(crate) struct MixerPageHandle {
    root: glib::WeakRef<GtkBox>,
    nav: NavigationContext,
    requested_scene: RefCell<Option<String>>,
    signature: RefCell<Option<String>>,
    binding: RefCell<Option<(u64, crate::domain::workflow::ScopeBinding)>>,
    cards: RefCell<BTreeMap<String, audio_card::AudioCardHandle>>,
    controls_signature: RefCell<Option<String>>,
    groups: RefCell<Vec<glib::WeakRef<PreferencesGroup>>>,
    summary: glib::WeakRef<adw::ActionRow>,
    controls: glib::WeakRef<PreferencesGroup>,
    notice: glib::WeakRef<gtk4::Spinner>,
}

impl MixerPageHandle {
    fn is_current(&self) -> bool {
        let state = self.nav.state.borrow();
        matches!(
            state.obs_status,
            crate::controller::state::ObsStatus::Connected { .. }
        ) && self.binding.borrow().as_ref()
            == Some(&(state.workspace.epoch, state.workflow_scope()))
    }

    /// Apply an OBS mute echo to the existing visible input card.
    pub(crate) fn update_mute(&self, input: &str, muted: bool) {
        if self.is_current() {
            if let Some(card) = self.cards.borrow().get(input) {
                card.update_mute(muted);
            }
        }
    }

    /// Apply an OBS gain echo without resetting unrelated controls or search drafts.
    pub(crate) fn update_volume(&self, input: &str, mul: f64, db: f64) {
        if self.is_current() {
            if let Some(card) = self.cards.borrow().get(input) {
                card.update_volume(mul, db);
            }
        }
    }

    fn update_summary(&self, state: &crate::controller::state::AppState) {
        if let Some(summary) = self.summary.upgrade() {
            summary.set_subtitle(&source_summary(
                state.mixer.mode,
                state.scene_inventory.current_id.as_deref(),
                state
                    .mixer_scene_refresh_target_details()
                    .map(|target| (target.scene.to_owned(), target.reason)),
            ));
        }
        if let Some(controls) = self.controls.upgrade() {
            controls.set_description(Some(&state.mixer.mode.description()));
        }
    }

    fn refresh(self: &Rc<Self>) {
        let Some(root) = self.root.upgrade() else {
            return;
        };
        let state = self.nav.state.borrow();
        self.update_summary(&state);
        let binding = (state.workspace.epoch, state.workflow_scope());
        let loading = matches!(
            state.visible_mixer_render_source(),
            MixerVisibleRenderSource::Scene {
                status: MixerVisibleAudioStatus::Loading,
                ..
            }
        );
        // These controls address collection-scoped inputs, so a scene membership
        // read must not disable/dim them or cancel an in-progress fader drag.
        if loading
            && self.binding.borrow().as_ref() == Some(&binding)
            && !self.cards.borrow().is_empty()
        {
            if let Some(notice) = self.notice.upgrade() {
                notice.set_spinning(true);
                notice.set_opacity(1.0);
            }
            return;
        }
        if let Some(notice) = self.notice.upgrade() {
            notice.set_spinning(false);
            notice.set_opacity(0.0);
        }
        let signature = mixer_structure(&state);
        if self.signature.borrow().as_ref() == Some(&signature) {
            let inputs = visible_inputs(&state);
            for input in inputs {
                if let Some(card) = self.cards.borrow().get(&input.id) {
                    card.set_available(true);
                }
                self.update_mute(&input.id, input.muted);
                self.update_volume(&input.id, input.volume_mul, input.volume_db);
            }
            return;
        }
        let binding = (state.workspace.epoch, state.workflow_scope());
        drop(state);
        let view = MixerViewState::capture(&root);
        self.cards.borrow_mut().clear();
        if self.binding.borrow().as_ref() != Some(&binding) {
            self.requested_scene.borrow_mut().take();
        }
        *self.binding.borrow_mut() = Some(binding);
        *self.signature.borrow_mut() = Some(signature);
        populate(&root, &self.nav, self);
        view.restore(&root);
    }
}

fn visible_inputs(state: &crate::controller::state::AppState) -> &[AudioInput] {
    match state.visible_mixer_render_source() {
        MixerVisibleRenderSource::ActiveScene(inputs)
        | MixerVisibleRenderSource::Scene {
            status: MixerVisibleAudioStatus::Loaded(inputs),
            ..
        } => inputs,
        _ => &[],
    }
}

fn mixer_structure(state: &crate::controller::state::AppState) -> String {
    let inputs: Vec<_> = visible_inputs(state)
        .iter()
        .map(|input| {
            (
                &input.id,
                &input.name,
                &input.display_name,
                input.source_scope,
                &input.parent_scene_path,
                state.registry.input_icon(&input.id),
            )
        })
        .collect();
    let source = match state.visible_mixer_render_source() {
        MixerVisibleRenderSource::ActiveScene(_) => "active".to_string(),
        MixerVisibleRenderSource::MissingScene => "missing-scene".to_string(),
        MixerVisibleRenderSource::Scene { scene, status } => match status {
            MixerVisibleAudioStatus::Loaded(_) => format!("loaded:{scene}"),
            _ => format!("{scene}:{status:?}"),
        },
    };
    let selection = (
        &state.mixer.mode,
        &state.mixer.selected_scene,
        &state.mixer.pinned_scene,
        &state.mixer.grouping,
        &state.mixer.search,
    );
    format!(
        "{:?}{:?}{:?}{:?}{selection:?}{source:?}{inputs:?}",
        state.workspace.epoch,
        state.workflow_scope(),
        state.obs_status,
        (
            state.config.language,
            state
                .scene_inventory
                .scenes
                .iter()
                .map(|s| &s.id)
                .collect::<Vec<_>>()
        )
    )
}

#[derive(Default)]
struct MixerViewState {
    search_draft: Option<String>,
    expanded: bool,
    scroll: Option<f64>,
}

fn visit_mixer(widget: &gtk4::Widget, visit: &mut impl FnMut(&gtk4::Widget)) {
    visit(widget);
    let mut child = widget.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        visit_mixer(&widget, visit);
    }
}

impl MixerViewState {
    fn capture(root: &GtkBox) -> Self {
        let mut view = Self::default();
        visit_mixer(root.upcast_ref(), &mut |widget| {
            if let Some(search) = widget.downcast_ref::<EntryRow>() {
                view.search_draft = Some(search.text().to_string());
            }
            if let Some(options) = widget.downcast_ref::<adw::ExpanderRow>() {
                view.expanded = options.is_expanded();
            }
            if let Some(scroll) = widget.downcast_ref::<gtk4::ScrolledWindow>() {
                view.scroll
                    .get_or_insert_with(|| scroll.vadjustment().value());
            }
        });
        view
    }

    fn restore(self, root: &GtkBox) {
        visit_mixer(root.upcast_ref(), &mut |widget| {
            if let (Some(search), Some(draft)) = (
                widget.downcast_ref::<EntryRow>(),
                self.search_draft.as_deref(),
            ) {
                search.set_text(draft);
            }
            if let Some(options) = widget.downcast_ref::<adw::ExpanderRow>() {
                options.set_expanded(self.expanded);
            }
        });
        let Some(position) = self.scroll else { return };
        let frames = std::cell::Cell::new(0);
        root.add_tick_callback(move |root, _| {
            // The replacement page needs an allocation before its adjustment can scroll.
            frames.set(frames.get() + 1);
            if frames.get() < 2 {
                return glib::ControlFlow::Continue;
            }
            let mut restored = false;
            visit_mixer(root.upcast_ref(), &mut |widget| {
                if !restored {
                    if let Some(scroll) = widget.downcast_ref::<gtk4::ScrolledWindow>() {
                        scroll.vadjustment().set_value(position);
                        restored = true;
                    }
                }
            });
            glib::ControlFlow::Break
        });
    }
}
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

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>, Rc<MixerPageHandle>) {
    let root = GtkBox::builder()
        .orientation(gtk4::Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    root.add_css_class("app-page");
    root.add_css_class("mixer-page");
    let handle = Rc::new(MixerPageHandle {
        root: root.downgrade(),
        nav,
        requested_scene: RefCell::new(None),
        signature: RefCell::new(None),
        binding: RefCell::new(None),
        cards: RefCell::new(BTreeMap::new()),
        controls_signature: RefCell::new(None),
        groups: RefCell::new(Vec::new()),
        summary: glib::WeakRef::new(),
        controls: glib::WeakRef::new(),
        notice: glib::WeakRef::new(),
    });
    let refresh: Rc<dyn Fn()> = Rc::new({
        let handle = handle.clone();
        move || handle.refresh()
    });
    root.connect_map({
        let handle = Rc::downgrade(&handle);
        move |_| {
            let handle = handle.clone();
            glib::idle_add_local_once(move || {
                if let Some(handle) = handle.upgrade() {
                    handle.refresh();
                }
            });
        }
    });
    handle.refresh();
    (root.upcast(), refresh, handle)
}

fn populate(root: &GtkBox, nav: &NavigationContext, refresh_tracker: &MixerRefreshTracker) {
    // One immutable borrow for the whole build, rather than cloning all of
    // AppState — config, registry, stats history, and every mixer snapshot —
    // on each rebuild. Build-time code below only ever reads state; anything
    // that mutates runs later, inside a signal handler.
    let state = nav.state.borrow();
    let inventory = &state.scene_inventory;
    let mixer = &state.mixer;
    let active_scene = inventory.current_id.clone();
    let target_scene = state.mixer_scene_refresh_target().map(str::to_string);
    let inspection_snapshot = state.mixer_inspection_snapshot();
    let target_details = state
        .mixer_scene_refresh_target_details()
        .map(|target| (target.scene.to_string(), target.reason));

    let controls_key = format!(
        "{:?}{:?}{:?}{:?}",
        state.workspace.epoch,
        state.workflow_scope(),
        state.config.language,
        inventory.scenes.iter().map(|s| &s.id).collect::<Vec<_>>()
    );
    let existing = (refresh_tracker.controls_signature.borrow().as_ref() == Some(&controls_key))
        .then(|| root.first_child().and_downcast::<PreferencesPage>())
        .flatten();
    let page = if let Some(page) = existing {
        for group in refresh_tracker
            .groups
            .borrow_mut()
            .drain(..)
            .filter_map(|g| g.upgrade())
        {
            page.remove(&group);
        }
        while let Some(child) = page.next_sibling() {
            root.remove(&child);
        }
        page
    } else {
        crate::ui::clear_children(root);
        refresh_tracker.groups.borrow_mut().clear();
        *refresh_tracker.controls_signature.borrow_mut() = Some(controls_key);
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
        let grouping_row = build_grouping_row(nav, mixer.grouping, refresh_tracker);
        let search_row = build_search_row(nav, &mixer.search, refresh_tracker);

        refresh_tracker.controls.set(Some(&controls));
        controls.add(&mode_row);
        controls.add(&scene_row);
        page.add(&controls);
        let options = PreferencesGroup::new();
        let options_row = adw::ExpanderRow::builder()
            .use_markup(false)
            .title(fl!(LANGUAGE_LOADER, "mixer-secondary-options"))
            .build();
        options_row.add_row(&grouping_row);
        options_row.add_row(&search_row);
        options.add(&options_row);
        page.add(&options);

        let summary_group = PreferencesGroup::new();
        let summary = adw::ActionRow::builder()
            .use_markup(false)
            .title(fl!(LANGUAGE_LOADER, "mixer-summary-title"))
            .build();
        summary.set_subtitle(&source_summary(
            mixer.mode,
            active_scene.as_deref(),
            target_details,
        ));
        refresh_tracker.summary.set(Some(&summary));
        summary_group.add(&summary);
        let notice = gtk4::Spinner::builder()
            .valign(Align::Center)
            .width_request(16)
            .height_request(16)
            .opacity(0.0)
            .tooltip_text(fl!(LANGUAGE_LOADER, "mixer-loading-title"))
            .build();
        summary.add_suffix(&notice);
        refresh_tracker.notice.set(Some(&notice));
        page.add(&summary_group);

        root.append(&page);
        page
    };

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
                append_loading_placeholder(root, &inspection_snapshot, scene);
                drop(state);
                request_visible_mixer_scene_audio(
                    nav,
                    refresh_tracker,
                    MixerRefreshRequestIntent::Automatic,
                );
                return;
            }
        },
    };
    let inputs = filter_inputs(&source_inputs, &mixer.search);
    let inspection_status = append_mixer_inputs(
        MixerSurfaces {
            root,
            page: &page,
            tracker: refresh_tracker,
        },
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
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "mixer-mode-row-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "mixer-mode-row-subtitle"))
        .model(&model)
        .selected(index_of(&MixerMode::ALL, selected))
        .build();
    row.add_css_class("scenedeck-combo-row");

    row.connect_selected_notify({
        let nav = nav.clone();
        let refresh_tracker = Rc::downgrade(refresh_tracker);
        move |row| {
            let Some(refresh_tracker) = refresh_tracker.upgrade() else {
                return;
            };
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
            refresh_tracker.refresh();
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
    let names: Vec<&str> = scenes.iter().map(|scene| scene.id.as_str()).collect();
    let model = StringList::new(&names);
    let selected = selected_scene
        .and_then(|selected| scenes.iter().position(|scene| scene.id == selected))
        .unwrap_or(0) as u32;

    let row = ComboRow::builder()
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "mixer-scene-row-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "mixer-scene-row-subtitle"))
        .model(&model)
        .selected(selected)
        .build();
    row.add_css_class("scenedeck-combo-row");

    row.connect_selected_notify({
        let nav = nav.clone();
        let refresh_tracker = Rc::downgrade(refresh_tracker);
        let scene_ids: Vec<_> = scenes.iter().map(|scene| scene.id.clone()).collect();
        move |row| {
            let Some(refresh_tracker) = refresh_tracker.upgrade() else {
                return;
            };
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
            refresh_tracker.refresh();
        }
    });

    row
}

fn build_grouping_row(
    nav: &NavigationContext,
    selected: MixerGrouping,
    refresh_tracker: &MixerRefreshTracker,
) -> ComboRow {
    let labels: Vec<String> = MixerGrouping::ALL
        .iter()
        .map(|grouping| grouping.label())
        .collect();
    let model = string_list(&labels);
    let row = ComboRow::builder()
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "mixer-grouping-row-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "mixer-grouping-row-subtitle"))
        .model(&model)
        .selected(index_of(&MixerGrouping::ALL, selected))
        .build();
    row.add_css_class("scenedeck-combo-row");

    row.connect_selected_notify({
        let nav = nav.clone();
        let refresh_tracker = Rc::downgrade(refresh_tracker);
        move |row| {
            let Some(refresh_tracker) = refresh_tracker.upgrade() else {
                return;
            };
            nav.state.borrow_mut().mixer.grouping = grouping_at(row.selected());
            persist_mixer_selection(&nav);
            refresh_tracker.refresh();
        }
    });

    row
}

fn build_search_row(
    nav: &NavigationContext,
    search: &str,
    refresh_tracker: &MixerRefreshTracker,
) -> EntryRow {
    let row = EntryRow::builder()
        .use_markup(false)
        .name("mixer-search-entry")
        .title(fl!(LANGUAGE_LOADER, "mixer-search-row-title"))
        .text(search)
        .show_apply_button(true)
        .build();

    row.connect_apply({
        let nav = nav.clone();
        let refresh_tracker = Rc::downgrade(refresh_tracker);
        move |row| {
            let Some(refresh_tracker) = refresh_tracker.upgrade() else {
                return;
            };
            nav.state.borrow_mut().mixer.search = row.text().trim().to_string();
            refresh_tracker.refresh();
        }
    });

    row
}

/// Where the mixer renders: cards go inside the scrolling preferences page,
/// full-page status views take the whole root.
struct MixerSurfaces<'a> {
    root: &'a GtkBox,
    page: &'a PreferencesPage,
    tracker: &'a MixerRefreshTracker,
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
    let root = surfaces.root;
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
                &surfaces,
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
                    append_group(&surfaces, nav, &title, &inputs);
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
                    append_group(&surfaces, nav, &title, &inputs);
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
            let active = nav.state.borrow().mixer.mode == MixerMode::ActiveScene;
            if active {
                nav.state.borrow_mut().invalidate_active_audio();
                nav.dispatch(AppCommand::RefreshData);
            } else {
                request_visible_mixer_scene_audio(
                    &nav,
                    &refresh_tracker,
                    MixerRefreshRequestIntent::Explicit,
                );
            }
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
    surfaces: &MixerSurfaces<'_>,
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
        surfaces
            .tracker
            .cards
            .borrow_mut()
            .insert(input.id.clone(), card);
    }

    group.add(&flow);
    surfaces.tracker.groups.borrow_mut().push(group.downgrade());
    surfaces.page.add(&group);
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
        let mut tracked_scene = refresh_tracker.requested_scene.borrow_mut();
        prepare_mixer_scene_audio_request(
            intent,
            scene,
            state.visible_mixer_audio_status(scene),
            &mut tracked_scene,
        )
    };

    if let Some(scene) = command {
        let request = nav.state.borrow_mut().set_mixer_audio_loading(scene);
        nav.dispatch(AppCommand::RefreshMixerSceneAudio(request));
    }
}

fn prepare_mixer_scene_audio_request(
    intent: MixerRefreshRequestIntent,
    scene: &str,
    visible_status: MixerVisibleAudioStatus<'_>,
    tracked_scene: &mut Option<String>,
) -> Option<String> {
    if !should_request_mixer_scene_audio(intent, visible_status, scene, tracked_scene.as_deref()) {
        return None;
    }

    *tracked_scene = Some(scene.to_string());
    Some(scene.to_string())
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
    let mut tracked_scene = refresh_tracker.requested_scene.borrow_mut();
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
    use crate::controller::state::{
        AppState, MixerAudioError, MixerInspectionStatus, MixerSceneRefreshTargetReason,
        MixerVisibleAudioStatus,
    };
    use crate::domain::audio::AudioInput;
    use crate::domain::mixer::MixerMode;
    use crate::services::audio_service::AudioService;

    #[test]
    fn mixer_structure_ignores_audio_echoes_but_tracks_membership_and_scope() {
        let mut state = app_state();
        state.apply_active_audio(
            None,
            Ok(vec![AudioInput::new("Mic".into(), false, 1.0, 0.0)]),
        );
        let initial = super::mixer_structure(&state);
        state.audio_inputs[0].muted = true;
        state.audio_inputs[0].volume_mul = 0.5;
        state.audio_inputs[0].volume_db = -6.0;
        state.mixer.locked_inputs.insert("Mic".into());
        assert_eq!(initial, super::mixer_structure(&state));
        state
            .audio_inputs
            .push(AudioInput::new("Other".into(), false, 1.0, 0.0));
        assert_ne!(initial, super::mixer_structure(&state));
        state.audio_inputs.pop();
        state.workspace.epoch += 1;
        assert_ne!(initial, super::mixer_structure(&state));
    }

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
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_failure(request, "OBS failed".to_string());
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
        assert_eq!(command.as_deref(), Some("Scene A"));
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
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request, Vec::new());
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
        let request = state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success(request, vec![input("Mic", false, 1.0, 0.0)]);
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

        assert_eq!(command.as_deref(), Some("scene-a"));
        assert_eq!(tracked_scene.as_deref(), Some("scene-a"));

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Automatic,
            "scene-a",
            MixerVisibleAudioStatus::Missing,
            &mut tracked_scene,
        );

        assert_eq!(command, None);
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

        assert_eq!(command, None);
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

        assert_eq!(command.as_deref(), Some("scene-a"));
        assert_eq!(tracked_scene.as_deref(), Some("scene-a"));

        let command = prepare_mixer_scene_audio_request(
            MixerRefreshRequestIntent::Explicit,
            "scene-a",
            MixerVisibleAudioStatus::Error(&error),
            &mut tracked_scene,
        );

        assert_eq!(command, None);
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

        assert_eq!(command, None);
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

        assert_eq!(command, None);
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

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_mixer_echoes_preserve_cards_drafts_focus_and_popovers() {
        crate::ui::test_support::native_test("ui::pages::mixer::layout_tests::native_mixer_echoes_preserve_cards_drafts_focus_and_popovers", || {
            use std::time::Duration;
            let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
            {
                let mut state = nav.state.borrow_mut();
                state.obs_status = crate::controller::state::ObsStatus::Connected { obs_version: "fixture".into() };
                state.scene_collections.current = Some("Fixture".into());
                state.scene_inventory.scenes = vec![Scene { id: "Main".into() }];
                state.scene_inventory.current_id = Some("Main".into());
                state.apply_active_audio(Some("Main".into()), Ok(vec![audio_input("Mic"), audio_input("Music")]));
            }
            let (page, refresh, handle) = build(nav.clone());
            stack.add_named(&page, Some("mixer-fixture"));
            let window = gtk4::Window::builder().default_width(1000).default_height(1100).child(&stack).build();
            window.present();
            crate::ui::test_support::pump(Duration::from_millis(150));
            let mut search = None;
            let mut options = None;
            visit_mixer(&page, &mut |widget| {
                if let Some(row) = widget.downcast_ref::<EntryRow>() { search = Some(row.clone()); }
                if let Some(row) = widget.downcast_ref::<adw::ExpanderRow>() { options = Some(row.clone()); }
            });
            let search = search.unwrap();
            let options = options.unwrap();
            options.set_expanded(true);
            search.set_text("unapplied <draft> & search");
            let first = handle.cards.borrow()["Mic"].root.clone();
            let second = handle.cards.borrow()["Music"].root.clone();
            let mut scale = None;
            visit_mixer(second.upcast_ref(), &mut |widget| {
                if let Some(fader) = widget.downcast_ref::<gtk4::Scale>() { scale = Some(fader.clone()); }
            });
            let scale = scale.unwrap();
            crate::ui::test_support::pump(Duration::from_millis(80));
            assert!(scale.grab_focus());
            scale.set_value(-6.0);
            crate::ui::test_support::pump(crate::services::audio_service::VOLUME_SLIDER_DEBOUNCE + Duration::from_millis(30));
            scale.set_value(-9.0);
            let sent = f64::from(AudioService::volume_db_to_mul(-6.0) as f32);
            let sent_db = AudioService::volume_mul_to_db(sent);
            {
                let mut state = nav.state.borrow_mut();
                state.audio_inputs[1].volume_mul = sent;
                state.audio_inputs[1].volume_db = sent_db;
            }
            handle.update_volume("Music", sent, sent_db);
            refresh();
            assert_eq!(scale.value(), -9.0, "a delayed same-input echo reset the newer edit");
            let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
            for index in 0..128 {
                let mul = 0.1 + index as f64 / 200.0;
                let db = AudioService::volume_mul_to_db(mul);
                {
                    let mut state = nav.state.borrow_mut();
                    state.audio_inputs[0].muted = index % 2 == 0;
                    state.audio_inputs[0].volume_mul = mul;
                    state.audio_inputs[0].volume_db = db;
                }
                handle.update_mute("Mic", index % 2 == 0);
                handle.update_volume("Mic", mul, db);
                refresh();
                assert_eq!(handle.cards.borrow()["Mic"].root, first);
                assert_eq!(handle.cards.borrow()["Music"].root, second);
            }
            assert_eq!(gtk4::prelude::RootExt::focus(&window), Some(focus));
            assert_eq!(scale.value(), -9.0, "an unrelated echo reset the active fader");
            assert_eq!(search.text(), "unapplied <draft> & search");
            let mut menu = None;
            visit_mixer(first.upcast_ref(), &mut |widget| {
                if let Some(button) = widget.downcast_ref::<gtk4::MenuButton>() { menu = Some(button.clone()); }
            });
            let menu = menu.unwrap();
            menu.popup();
            crate::ui::test_support::pump(Duration::from_millis(100));
            let popover = menu.popover().unwrap();
            assert!(popover.is_mapped());
            for _ in 0..64 { handle.update_mute("Mic", true); }
            assert!(popover.is_mapped());
            assert_eq!(handle.cards.borrow().len(), 2);
            popover.popdown();
            let preferences = page.first_child().unwrap();
            nav.state.borrow_mut().invalidate_active_audio();
            refresh();
            assert_eq!(page.first_child(), Some(preferences.clone()));
            assert_eq!(handle.cards.borrow()["Mic"].root, first);
            assert!(first.is_sensitive(), "membership reads must not dim collection-scoped input controls");
            nav.state.borrow_mut().apply_active_audio(Some("Main".into()), Ok(vec![audio_input("Mic"), audio_input("Music")]));
            refresh();
            assert_eq!(handle.cards.borrow()["Mic"].root, first);
            assert!(first.is_sensitive());
            nav.state.borrow_mut().audio_inputs.push(audio_input("Third"));
            refresh();
            let mut restored_draft = None;
            visit_mixer(&page, &mut |widget| {
                if let Some(row) = widget.downcast_ref::<EntryRow>() { restored_draft = Some(row.text().to_string()); }
            });
            assert_eq!(restored_draft.as_deref(), Some("unapplied <draft> & search"));
            assert_eq!(handle.cards.borrow().len(), 3);
            assert_eq!(page.first_child(), Some(preferences), "membership changes replaced the entire Mixer");
            assert!(search.parent().is_some(), "membership changes detached search controls");
            assert!(options.is_expanded());
            let weak_scale = scale.downgrade();
            drop((scale, first, second, search, options, menu, popover));
            crate::ui::test_support::pump(Duration::from_millis(80));
            assert!(weak_scale.upgrade().is_none(), "structural replacement retained the old fader");
            *handle.requested_scene.borrow_mut() = Some("Main".into());
            nav.state.borrow_mut().workspace.epoch += 1;
            refresh();
            assert!(handle.requested_scene.borrow().is_none(), "old scope suppressed a new same-name request");
            let weak_page = page.downgrade();
            window.destroy();
            stack.remove(&page);
            drop((window, page, refresh, handle, nav, stack));
            crate::ui::test_support::pump(Duration::from_millis(250));
            assert!(weak_page.upgrade().is_none());
        });
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_inspected_scene_without_snapshot_issues_one_request_without_borrowing_state_twice() {
        crate::ui::test_support::native_test(
            "ui::pages::mixer::layout_tests::native_inspected_scene_without_snapshot_issues_one_request_without_borrowing_state_twice",
            || {
                for mode in [MixerMode::SelectedScene, MixerMode::PinnedScene] {
                    let (nav, stack, _runtime, _events) = crate::ui::test_support::navigation();
                    {
                        let mut state = nav.state.borrow_mut();
                        state.obs_status = crate::controller::state::ObsStatus::Connected {
                            obs_version: "fixture".into(),
                        };
                        state.scene_collections.current = Some("Fixture".into());
                        state.scene_inventory.scenes = vec![
                            Scene { id: "Program".into() },
                            Scene { id: "Inspected".into() },
                        ];
                        state.scene_inventory.current_id = Some("Program".into());
                        state.mixer.mode = mode;
                        state.mixer.selected_scene = Some("Inspected".into());
                        state.mixer.pinned_scene = Some("Inspected".into());
                    }
                    let (page, refresh, handle) = build(nav.clone());
                    assert_eq!(nav.state.borrow().mixer_audio_refresh.requested_scene(), Some("Inspected"));
                    assert_eq!(handle.requested_scene.borrow().as_deref(), Some("Inspected"));
                    refresh();
                    assert_eq!(nav.state.borrow().mixer_audio_refresh.requested_scene(), Some("Inspected"));
                    let weak_page = page.downgrade();
                    drop((page, refresh, handle, nav, stack));
                    crate::ui::test_support::pump(std::time::Duration::from_millis(20));
                    assert!(weak_page.upgrade().is_none());
                }
            },
        );
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
            }],
            current_id: Some("Main".to_string()),
            previous_id: None,
        };
        let inputs = (0..count)
            .map(|index| audio_input(&format!("Source {index}")))
            .collect();
        state.apply_active_audio(Some("Main".into()), Ok(inputs));

        let nav =
            NavigationContext::new(Rc::new(RefCell::new(state)), gtk4::Stack::new(), controller);
        let (widget, _refresh, _handle) = build(nav);
        widget.measure(gtk4::Orientation::Vertical, -1).0
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn mixer_page_minimum_height_does_not_grow_with_the_card_count() {
        crate::ui::test_support::native_test("ui::pages::mixer::layout_tests::mixer_page_minimum_height_does_not_grow_with_the_card_count", || {
        let one = measure_mixer_page_min_height(1);
        let many = measure_mixer_page_min_height(24);

        println!("min height: 1 source = {one}px, 24 sources = {many}px");
        assert_eq!(
            one, many,
            "the page must scroll its cards instead of demanding room for them"
        );
        });
    }
}
