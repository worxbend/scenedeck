//! Focused native OBS inspector. Inspection never changes the Program scene.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use adw::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, CheckButton, DropDown, Label, Orientation, SpinButton};
use i18n_embed_fl::fl;

use crate::controller::state::ObsStatus;
use crate::domain::control::{AudioMonitor, ControlAction, MediaAction, SourceDetail};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::{navigation::NavigationContext, workspace};

#[derive(Default)]
struct LocalState {
    epoch: Option<u64>,
    syncing: bool,
    query: String,
    show_detail: bool,
    usage_expanded: bool,
    text_drafts: HashMap<String, String>,
    text_origins: HashMap<String, String>,
    numeric_drafts: HashMap<(String, String), f64>,
    numeric_text: HashMap<(String, String), String>,
    track_drafts: HashMap<String, [bool; 6]>,
}

impl LocalState {
    fn set_epoch(&mut self, epoch: u64) {
        if self.epoch != Some(epoch) {
            self.text_drafts.clear();
            self.text_origins.clear();
            self.numeric_drafts.clear();
            self.numeric_text.clear();
            self.track_drafts.clear();
            self.query.clear();
            self.show_detail = false;
            self.usage_expanded = false;
            self.epoch = Some(epoch);
        }
    }

    fn text_draft(&mut self, input: &str, observed: &str) -> String {
        if self
            .text_drafts
            .get(input)
            .is_some_and(|draft| draft == observed)
        {
            self.text_drafts.remove(input);
            self.text_origins.remove(input);
        }
        self.text_drafts
            .get(input)
            .cloned()
            .unwrap_or_else(|| observed.to_string())
    }

    fn numeric_draft(&mut self, input: &str, field: &str, observed: f64) -> f64 {
        let key = (input.to_owned(), field.to_owned());
        if self
            .numeric_drafts
            .get(&key)
            .is_some_and(|draft| (draft - observed).abs() < 0.001)
        {
            self.numeric_drafts.remove(&key);
            self.numeric_text.remove(&key);
        }
        self.numeric_drafts.get(&key).copied().unwrap_or(observed)
    }

    fn tracks_draft(&mut self, input: &str, observed: [bool; 6]) -> [bool; 6] {
        if self.track_drafts.get(input) == Some(&observed) {
            self.track_drafts.remove(input);
        }
        self.track_drafts.get(input).copied().unwrap_or(observed)
    }

    fn text_conflict(&self, input: &str, observed: &str) -> bool {
        self.text_origins
            .get(input)
            .is_some_and(|origin| origin != observed)
    }

    fn edit_text(&mut self, input: &str, observed: &str, value: String) {
        self.text_origins
            .entry(input.to_owned())
            .or_insert_with(|| observed.to_owned());
        self.text_drafts.insert(input.to_owned(), value);
    }
}

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let local = Rc::new(RefCell::new(LocalState::default()));
    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    root.add_css_class("sources-page");
    populate(&root, &nav, &local);
    let rendered_content = Rc::new(RefCell::new(sources_content(&nav)));
    let status_local = local.clone();
    let status_nav = nav.clone();
    let status_root = root.downgrade();
    glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
        let Some(root) = status_root.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if root.is_mapped() {
            update_status(&root, &status_nav, &status_local);
        }
        glib::ControlFlow::Continue
    });
    let weak_root = root.downgrade();
    let poll_nav = nav.clone();
    glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
        let Some(root) = weak_root.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if !root.is_mapped()
            || !media_visible(root.upcast_ref())
            || mapped_popover(root.upcast_ref())
        {
            return glib::ControlFlow::Continue;
        }
        let request = {
            let state = poll_nav.state.borrow();
            if state.current_page != crate::controller::state::Page::Sources
                || !matches!(state.obs_status, ObsStatus::Connected { .. })
                || !state.workspace.can_control()
                || !state
                    .workspace
                    .detail
                    .as_ref()
                    .is_some_and(|detail| detail.media.is_some())
            {
                None
            } else {
                Some(crate::controller::workspace::WorkspaceRequest {
                    epoch: state.workspace.epoch,
                    sequence: state.workspace.sequence,
                    scope: state.workflow_scope(),
                    scene: state.workspace.selected_scene.clone(),
                    input: state.workspace.selected_input.clone(),
                    include_sources: true,
                    screenshot: false,
                    result_id: None,
                })
            }
        };
        if let Some(request) = request {
            poll_nav
                .dispatch(crate::controller::command::AppCommand::RefreshWorkspaceMedia(request));
        }
        glib::ControlFlow::Continue
    });
    let map_nav = nav.clone();
    let refresh: Rc<dyn Fn()> = Rc::new({
        let root = root.downgrade();
        move || {
            let Some(root) = root.upgrade() else { return };
            let content = sources_content(&nav);
            update_status(&root, &nav, &local);
            if *rendered_content.borrow() == content {
                return;
            }
            if content.same_structure(&rendered_content.borrow()) {
                update_observed_controls(&root, &content, &local);
                update_status(&root, &nav, &local);
                *rendered_content.borrow_mut() = content;
                return;
            }
            *rendered_content.borrow_mut() = content;
            let focus_root = root.is_mapped().then(|| root.root()).flatten();
            let old_focus = focus_root.as_ref().and_then(|window| window.focus());
            let mut focus = old_focus.clone();
            let mut saved_focus = None;
            let mut owns_focus = false;
            while let Some(widget) = focus {
                if widget == root {
                    owns_focus = true;
                    break;
                }
                if saved_focus.is_none() && widget.widget_name().starts_with("control-focus-") {
                    let cursor = widget
                        .downcast_ref::<gtk4::TextView>()
                        .map(|view| view.buffer().cursor_position());
                    saved_focus = Some((widget.widget_name().to_string(), cursor));
                }
                focus = widget.parent();
            }
            if !owns_focus {
                saved_focus = None;
            }
            let mut scroll_positions = HashMap::new();
            visit(root.upcast_ref(), &mut |widget| {
                if let Some(scroll) = widget.downcast_ref::<gtk4::ScrolledWindow>() {
                    scroll_positions.insert(
                        widget.widget_name().to_string(),
                        scroll.vadjustment().value(),
                    );
                }
            });
            // GTK must release its focus chain before focused descendants are unparented.
            if owns_focus {
                if let Some(window) = &focus_root {
                    window.set_focus(gtk4::Widget::NONE);
                }
            }
            crate::ui::clear_children(&root);
            populate(&root, &nav, &local);
            visit(root.upcast_ref(), &mut |widget| {
                if let Some(scroll) = widget.downcast_ref::<gtk4::ScrolledWindow>() {
                    if let Some(value) = scroll_positions.get(widget.widget_name().as_str()) {
                        scroll.vadjustment().set_value(*value);
                    }
                }
            });
            if let Some((name, cursor)) = saved_focus {
                let root = root.clone();
                glib::idle_add_local_once(move || {
                    // Keep the old focus alive through unparenting and the completed rebuild.
                    let _old_focus = old_focus;
                    if !root.is_mapped() || root.root().and_then(|window| window.focus()).is_some()
                    {
                        return;
                    }
                    visit(root.upcast_ref(), &mut |widget| {
                        if widget.widget_name() != name
                            || !widget.is_sensitive()
                            || !widget.is_visible()
                        {
                            return;
                        }
                        widget.grab_focus();
                        if let (Some(view), Some(cursor)) =
                            (widget.downcast_ref::<gtk4::TextView>(), cursor)
                        {
                            let buffer = view.buffer();
                            buffer.place_cursor(
                                &buffer.iter_at_offset(cursor.min(buffer.char_count())),
                            );
                        }
                    });
                });
            }
        }
    });
    root.connect_map({
        let refresh = refresh.clone();
        move |_| {
            // Live reads intentionally omit source details; entering Sources needs its own read.
            let ready = {
                let mut state = map_nav.state.borrow_mut();
                state.workspace.mark_dirty();
                !state.workspace.loading && !state.workspace.pending
            };
            refresh();
            if ready {
                workspace::refresh(&map_nav);
            }
        }
    });
    (root.upcast(), refresh)
}

fn update_media_values(root: &GtkBox, detail: &SourceDetail, local: &Rc<RefCell<LocalState>>) {
    let Some(media) = &detail.media else { return };
    visit(root.upcast_ref(), &mut |widget| {
        if widget.widget_name() == "sources-media-state" {
            if let Some(label) = widget.downcast_ref::<Label>() {
                label.set_text(&media.state.label());
            }
        }
        if widget.widget_name() == format!("control-focus-seek-position-{}", detail.input) {
            if let Some(spin) = widget.downcast_ref::<SpinButton>() {
                let focused = spin
                    .root()
                    .and_then(|r| r.focus())
                    .is_some_and(|focus| focus == *spin || focus.is_ancestor(spin));
                if focused {
                    return;
                }
                let value = media
                    .cursor_ms
                    .unwrap_or(0)
                    .clamp(0, media.duration_ms.unwrap_or(0).max(0))
                    as f64
                    / 1000.0;
                let value = local
                    .borrow_mut()
                    .numeric_draft(&detail.input, "seek", value);
                let draft = local
                    .borrow()
                    .numeric_text
                    .get(&(detail.input.clone(), "seek".into()))
                    .cloned();
                local.borrow_mut().syncing = true;
                if let Some(draft) = draft {
                    spin.set_text(&draft);
                } else {
                    spin.set_value(value);
                }
                local.borrow_mut().syncing = false;
            }
        }
    });
}

/// Sources-owned content, not the workspace publication clock or Live output
/// observations. Any changed captured target/value takes the existing rebuild path.
#[derive(Clone, PartialEq)]
struct SourcesContent {
    scope: crate::domain::workflow::ScopeBinding,
    epoch: u64,
    connected: bool,
    scene: Option<String>,
    input: Option<String>,
    scenes: Vec<String>,
    sources: Option<(Vec<crate::domain::control::SourceItem>, Vec<String>)>,
    detail: Option<SourceDetail>,
    usage: Vec<crate::domain::control::SourceItem>,
    warnings: Vec<String>,
    capabilities: Vec<String>,
}

impl SourcesContent {
    fn same_structure(&self, previous: &Self) -> bool {
        let mut current = self.clone();
        // Value-only observations have live callbacks; no captured target changes.
        if let (Some((items, _)), Some((old, _))) = (&mut current.sources, &previous.sources) {
            for (item, old) in items.iter_mut().zip(old) {
                item.enabled = old.enabled;
                item.ancestor_enabled = old.ancestor_enabled;
            }
        }
        if let (Some(detail), Some(old)) = (&mut current.detail, &previous.detail) {
            if detail.text.is_some() && old.text.is_some() && !detail.text_from_file {
                detail.text = old.text.clone();
            }
            if detail.audio.is_some() && old.audio.is_some() {
                detail.audio = old.audio.clone();
            }
            for (filter, old) in detail.filters.iter_mut().zip(&old.filters) {
                filter.enabled = old.enabled;
            }
            if let (Some(media), Some(old)) = (&mut detail.media, &old.media) {
                media.state = old.state;
                media.cursor_ms = old.cursor_ms;
            }
        }
        current == *previous
    }
}

fn update_observed_controls(
    root: &GtkBox,
    content: &SourcesContent,
    local: &Rc<RefCell<LocalState>>,
) {
    let mut checks = HashMap::new();
    let mut hidden_parents = HashMap::new();
    if let Some((items, _)) = &content.sources {
        for item in items {
            let key = format!("{}-{}-{}", item.owner, item.item_id, item.path.join("/"));
            checks.insert(format!("control-focus-item-{key}"), item.enabled);
            hidden_parents.insert(
                format!("sources-hidden-parent-{key}"),
                !item.ancestor_enabled,
            );
        }
    }
    if let Some(detail) = &content.detail {
        for filter in &detail.filters {
            checks.insert(
                format!("control-focus-filter-{}-{}", detail.input, filter.name),
                filter.enabled,
            );
        }
        update_media_values(root, detail, local);
        update_detail_values(root, detail, local);
    }
    local.borrow_mut().syncing = true;
    visit(root.upcast_ref(), &mut |widget| {
        if let Some(active) = checks.get(widget.widget_name().as_str()) {
            if let Some(check) = widget.downcast_ref::<CheckButton>() {
                check.set_active(*active);
            }
        }
        if let Some(visible) = hidden_parents.get(widget.widget_name().as_str()) {
            widget.set_visible(*visible);
        }
    });
    local.borrow_mut().syncing = false;
}

fn observed_text(nav: &NavigationContext, input: &str) -> Option<String> {
    nav.state
        .borrow()
        .workspace
        .detail
        .as_ref()
        .filter(|detail| detail.input == input)
        .and_then(|detail| detail.text.clone())
}

fn update_detail_values(root: &GtkBox, detail: &SourceDetail, local: &Rc<RefCell<LocalState>>) {
    let text = detail
        .text
        .as_ref()
        .filter(|_| !detail.text_from_file)
        .map(|observed| local.borrow_mut().text_draft(&detail.input, observed));
    let conflict = detail
        .text
        .as_ref()
        .is_some_and(|observed| local.borrow().text_conflict(&detail.input, observed));
    let tracks = detail
        .audio
        .as_ref()
        .map(|audio| local.borrow_mut().tracks_draft(&detail.input, audio.tracks));
    let mut numbers = HashMap::new();
    if let Some(audio) = &detail.audio {
        for (field, title, value) in [
            (
                "sync",
                fl!(LANGUAGE_LOADER, "control-audio-sync"),
                audio.sync_offset_ms as f64,
            ),
            (
                "balance",
                fl!(LANGUAGE_LOADER, "control-audio-balance"),
                f64::from(audio.balance),
            ),
        ] {
            let value = local
                .borrow_mut()
                .numeric_draft(&detail.input, field, value);
            let draft = local
                .borrow()
                .numeric_text
                .get(&(detail.input.clone(), field.into()))
                .cloned();
            numbers.insert(format!("control-focus-number-{title}"), (value, draft));
        }
    }
    local.borrow_mut().syncing = true;
    visit(root.upcast_ref(), &mut |widget| {
        let name = widget.widget_name();
        if name == format!("control-focus-text-{}", detail.input) {
            if let (Some(view), Some(text)) =
                (widget.downcast_ref::<gtk4::TextView>(), text.as_ref())
            {
                let buffer = view.buffer();
                if buffer.text(&buffer.start_iter(), &buffer.end_iter(), true) != text.as_str() {
                    let cursor = buffer.cursor_position();
                    buffer.set_text(text);
                    buffer.place_cursor(&buffer.iter_at_offset(cursor.min(buffer.char_count())));
                }
            }
        }
        if name == "sources-text-conflict" {
            widget.set_visible(conflict);
        }
        if let (Some(spin), Some((value, draft))) = (
            widget.downcast_ref::<SpinButton>(),
            numbers.get(name.as_str()),
        ) {
            if let Some(draft) = draft {
                spin.set_text(draft);
            } else {
                spin.set_value(*value);
            }
        }
        if name == format!("control-focus-monitor-{}", detail.input) {
            if let (Some(selector), Some(audio)) =
                (widget.downcast_ref::<DropDown>(), detail.audio.as_ref())
            {
                selector.set_selected(match audio.monitor {
                    AudioMonitor::Off => 0,
                    AudioMonitor::MonitorOnly => 1,
                    AudioMonitor::MonitorAndOutput => 2,
                });
            }
        }
        if let Some(tracks) = tracks {
            for (index, active) in tracks.iter().enumerate() {
                if name == format!("control-focus-track-{}-{index}", detail.input) {
                    if let Some(check) = widget.downcast_ref::<CheckButton>() {
                        check.set_active(*active);
                    }
                }
            }
        }
    });
    local.borrow_mut().syncing = false;
}

fn sources_content(nav: &NavigationContext) -> SourcesContent {
    sources_content_from_state(&nav.state.borrow())
}

fn sources_content_from_state(state: &crate::controller::state::AppState) -> SourcesContent {
    let workspace = &state.workspace;
    let mut detail = workspace.detail.clone();
    if let Some(detail) = &mut detail {
        // These Live mixer observations are not rendered by Sources.
        detail.muted = None;
        detail.volume_db = None;
    }
    SourcesContent {
        scope: state.workflow_scope(),
        epoch: workspace.epoch,
        connected: matches!(state.obs_status, ObsStatus::Connected { .. }),
        scene: workspace.selected_scene.clone().or_else(|| {
            workspace
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.scene.clone())
        }),
        input: workspace.selected_input.clone(),
        scenes: state
            .scene_inventory
            .scenes
            .iter()
            .map(|scene| scene.id.clone())
            .collect(),
        sources: workspace
            .snapshot
            .as_ref()
            .map(|snapshot| (snapshot.items.clone(), snapshot.warnings.clone())),
        detail,
        usage: workspace.usage.clone(),
        warnings: workspace.warnings.clone(),
        capabilities: state.available_requests.clone(),
    }
}

/// Observation/status changes never unparent browser/editor controls. Local
/// query and usage expansion already update their own widgets in their signals;
/// show_detail also needs applying when the already-selected source is inspected.
fn update_status(root: &GtkBox, nav: &NavigationContext, local: &Rc<RefCell<LocalState>>) {
    let (connected, loading, mutations_allowed, fresh, error, result, observed, has_snapshot) = {
        let state = nav.state.borrow();
        let workspace = &state.workspace;
        let connected = matches!(state.obs_status, ObsStatus::Connected { .. });
        let fresh = workspace
            .observed_at
            .is_some_and(|time| time.elapsed().as_secs() <= 30)
            && !workspace.dirty;
        (
            connected,
            workspace.loading,
            connected && workspace.can_control(),
            fresh,
            workspace.error.clone(),
            workspace.result.clone(),
            workspace.observed_at,
            workspace.snapshot.is_some(),
        )
    };
    let show_detail = local.borrow().show_detail;
    let text_conflict = nav
        .state
        .borrow()
        .workspace
        .detail
        .as_ref()
        .is_some_and(|detail| {
            detail
                .text
                .as_ref()
                .is_some_and(|text| local.borrow().text_conflict(&detail.input, text))
        });
    visit(root.upcast_ref(), &mut |widget| {
        if let Some(label) = widget.downcast_ref::<Label>() {
            let value = match widget.widget_name().as_str() {
                "sources-status-loading" => {
                    Some((loading, fl!(LANGUAGE_LOADER, "control-loading")))
                }
                "sources-status-error" => {
                    Some((error.is_some(), error.clone().unwrap_or_default()))
                }
                "sources-status-result" => {
                    Some((error.is_none() && !result.is_empty(), result.clone()))
                }
                "sources-status-age" => Some((
                    observed.is_some(),
                    observed
                        .map(|time| {
                            fl!(
                                LANGUAGE_LOADER,
                                "control-observed",
                                seconds = time.elapsed().as_secs()
                            )
                        })
                        .unwrap_or_default(),
                )),
                "sources-status-stale" => Some((
                    has_snapshot && !fresh,
                    fl!(LANGUAGE_LOADER, "control-stale"),
                )),
                _ => None,
            };
            if let Some((visible, value)) = value {
                if label.label() != value {
                    label.set_label(&value);
                    label.set_tooltip_text(Some(&value));
                }
                label.set_visible(visible);
            }
        }
        if widget.widget_name() == "control-focus-refresh" {
            widget.set_sensitive(connected && !loading && !nav.state.borrow().workspace.pending);
        }
        if widget.has_css_class("sources-mutation") {
            widget.set_sensitive(
                mutations_allowed && !(text_conflict && widget.has_css_class("sources-text-save")),
            );
        }
        if widget.has_css_class("sources-inspect") {
            widget.set_sensitive(connected && !nav.state.borrow().workspace.pending);
        }
        if let Some(split) = widget.downcast_ref::<adw::NavigationSplitView>() {
            if split.shows_content() != show_detail {
                split.set_show_content(show_detail);
            }
        }
        if !mutations_allowed {
            if let Some(popover) = widget.downcast_ref::<gtk4::Popover>() {
                popover.popdown();
            }
        }
    });
}

fn mapped_popover(widget: &gtk4::Widget) -> bool {
    let mut mapped = false;
    visit(widget, &mut |child| {
        mapped |= child.is::<gtk4::Popover>() && child.is_mapped();
    });
    mapped
}

fn visit(widget: &gtk4::Widget, action: &mut impl FnMut(&gtk4::Widget)) {
    action(widget);
    let mut child = widget.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        visit(&widget, action);
    }
}

pub(crate) fn media_visible(widget: &gtk4::Widget) -> bool {
    let mut visible = false;
    visit(widget, &mut |child| {
        visible |= child.widget_name() == "source-media-controls" && child.is_mapped();
    });
    visible
}

fn populate(root: &GtkBox, nav: &NavigationContext, local: &Rc<RefCell<LocalState>>) {
    let scoped_nav = nav.scoped();
    let nav = &scoped_nav;
    let state = nav.state.borrow();
    local.borrow_mut().set_epoch(state.workspace.epoch);
    let snapshot = state.workspace.snapshot.clone();
    let detail = state.workspace.detail.clone();
    let selected_input = state.workspace.selected_input.clone();
    let usage = state.workspace.usage.clone();
    let warnings = state.workspace.warnings.clone();
    let loading = state.workspace.loading;
    let pending = state.workspace.pending;
    let dirty = state.workspace.dirty;
    let selected_scene = state.workspace.selected_scene.clone().or_else(|| {
        snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.scene.clone())
    });
    let observed = state.workspace.observed_at;
    let connected = matches!(state.obs_status, ObsStatus::Connected { .. });
    let capabilities = state.available_requests.clone();
    let scenes: Vec<String> = state
        .scene_inventory
        .scenes
        .iter()
        .map(|scene| scene.id.clone())
        .collect();
    drop(state);

    let page = column(10);
    page.set_margin_start(16);
    page.set_margin_end(16);
    page.set_margin_top(16);
    page.set_margin_bottom(16);
    let heading = row();
    let title = text(&fl!(LANGUAGE_LOADER, "sources-title"));
    title.add_css_class("heading");
    title.set_hexpand(true);
    heading.append(&title);
    let refresh = Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "control-refresh"))
        .sensitive(connected && !loading)
        .build();
    refresh.set_widget_name("control-focus-refresh");
    let nav_refresh = nav.clone();
    refresh.connect_clicked(move |_| workspace::refresh(&nav_refresh));
    heading.append(&refresh);
    page.append(&heading);
    title.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "control-inspection-note")));
    if !connected {
        page.append(&text(&fl!(LANGUAGE_LOADER, "control-disconnected")));
        root.append(&page);
        return;
    }
    // Routine read activity belongs in the existing heading. Adding/removing
    // vertical status rows shifted the browser and editor on every command.
    let activity = row();
    activity.set_valign(gtk4::Align::Center);
    heading.insert_child_after(&activity, Some(&title));
    let message_line = row();
    message_line.set_height_request(20);
    page.append(&message_line);
    for (name, class) in [
        ("sources-status-loading", "dim-label"),
        ("sources-status-error", "error"),
        ("sources-status-result", "caption"),
        ("sources-status-age", "caption"),
        ("sources-status-stale", "dim-label"),
    ] {
        let label = text("");
        label.set_widget_name(name);
        label.add_css_class(class);
        label.set_visible(false);
        if matches!(
            name,
            "sources-status-loading" | "sources-status-age" | "sources-status-stale"
        ) {
            label.set_wrap(false);
            label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            label.set_max_width_chars(22);
            activity.append(&label);
        } else {
            // Errors and operation results share one reserved line. Error text
            // remains available in full through the tooltip.
            label.set_wrap(false);
            label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            label.set_hexpand(true);
            message_line.append(&label);
        }
    }
    let source_browser = column(12);
    source_browser.add_css_class("sources-list");
    if !scenes.is_empty() {
        let selected = selected_scene
            .as_ref()
            .and_then(|selected| scenes.iter().position(|scene| scene == selected));
        let selector = string_selector(&scenes, selected);
        selector.set_widget_name("control-focus-scene");
        selector.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "control-scene-selector")));
        selector.update_property(&[gtk4::accessible::Property::Label(&fl!(
            LANGUAGE_LOADER,
            "control-scene-selector"
        ))]);
        let nav_scene = nav.clone();
        selector.connect_selected_notify(move |selector| {
            if let Some(scene) = scenes.get(selector.selected() as usize) {
                workspace::inspect_scene(&nav_scene, Some(scene.clone()));
            }
        });
        source_browser.append(&selector);
    }

    let Some(snapshot) = snapshot else {
        page.append(&text(&fl!(LANGUAGE_LOADER, "control-request-hint")));
        root.append(&page);
        update_status(root, nav, local);
        return;
    };
    let fresh = observed.is_some_and(|time| time.elapsed().as_secs() <= 30) && !dirty;
    let mutations_allowed = !pending && fresh && nav.state.borrow().workspace.error.is_none();
    let search = gtk4::SearchEntry::builder()
        .placeholder_text(fl!(LANGUAGE_LOADER, "control-source-search"))
        .build();
    search.set_widget_name("control-focus-search");
    search.set_text(&local.borrow().query);
    source_browser.append(&search);
    let source_list = column(6);
    let mut search_rows: Vec<(String, GtkBox)> = Vec::new();
    for item in &snapshot.items {
        let item_row = column(3);
        item_row.add_css_class("source-item");
        if selected_input.as_deref() == Some(item.input.as_str()) {
            item_row.add_css_class("source-item-selected");
        }
        let line = row();
        let check = CheckButton::builder()
            .active(item.enabled)
            .sensitive(mutations_allowed && supported(&capabilities, "SetSceneItemEnabled"))
            .hexpand(true)
            .build();
        let name = text(&item.input);
        name.set_selectable(false);
        name.set_max_width_chars(28);
        name.set_hexpand(true);
        check.set_child(Some(&name));
        check.set_tooltip_text(Some(&item.input));
        check.update_property(&[gtk4::accessible::Property::Label(&item.input)]);
        check.set_widget_name(&format!(
            "control-focus-item-{}-{}-{}",
            item.owner,
            item.item_id,
            item.path.join("/")
        ));
        if supported(&capabilities, "SetSceneItemEnabled") {
            check.add_css_class("sources-mutation");
        }
        let nav_toggle = nav.clone();
        let owner = item.owner.clone();
        let item_id = item.item_id;
        let toggle_local = local.clone();
        check.connect_toggled(move |check| {
            if toggle_local.borrow().syncing {
                return;
            }
            workspace::act(
                &nav_toggle,
                ControlAction::SetItemEnabled {
                    owner: owner.clone(),
                    item_id,
                    enabled: check.is_active(),
                },
            )
        });
        line.append(&check);
        let inspect = Button::builder()
            .icon_name("document-properties-symbolic")
            .tooltip_text(fl!(LANGUAGE_LOADER, "control-inspect-source"))
            .sensitive(!loading && !item.is_group && !item.is_scene)
            .build();
        inspect.set_widget_name(&format!(
            "control-focus-inspect-{}-{}-{}",
            item.owner,
            item.item_id,
            item.path.join("/")
        ));
        inspect.update_property(&[gtk4::accessible::Property::Label(&format!(
            "{} · {}",
            fl!(LANGUAGE_LOADER, "control-inspect-source"),
            item.input
        ))]);
        if selected_input.as_deref() == Some(item.input.as_str()) {
            inspect.add_css_class("suggested-action");
        }
        if !item.is_group && !item.is_scene {
            inspect.add_css_class("sources-inspect");
        }
        let nav_input = nav.clone();
        let input = item.input.clone();
        let local_input = local.clone();
        inspect.connect_clicked(move |_| {
            local_input.borrow_mut().show_detail = true;
            workspace::inspect_input(&nav_input, Some(input.clone()));
        });
        line.append(&inspect);
        item_row.append(&line);
        let path = item.path.join(" / ");
        let breadcrumb = format!("{path} · {} #{}", item.owner, item.item_id);
        let caption = text(&breadcrumb);
        caption.add_css_class("caption");
        item_row.append(&caption);
        let hidden = text(&fl!(LANGUAGE_LOADER, "control-hidden-parent"));
        hidden.set_widget_name(&format!(
            "sources-hidden-parent-{}-{}-{}",
            item.owner,
            item.item_id,
            item.path.join("/")
        ));
        hidden.set_visible(!item.ancestor_enabled);
        item_row.append(&hidden);
        let haystack = format!("{} {path} {}", item.input, item.owner).to_lowercase();
        item_row.set_visible(haystack.contains(&local.borrow().query.to_lowercase()));
        search_rows.push((haystack, item_row.clone()));
        source_list.append(&item_row);
    }
    if snapshot.items.is_empty() {
        source_list.append(&text(&fl!(LANGUAGE_LOADER, "control-sources-empty")));
    }
    let no_matches = text(&fl!(LANGUAGE_LOADER, "control-no-matches"));
    no_matches.set_visible(
        !snapshot.items.is_empty() && search_rows.iter().all(|(_, widget)| !widget.is_visible()),
    );
    source_list.append(&no_matches);
    let local_search = local.clone();
    search.connect_search_changed(move |search| {
        let query = search.text().to_string();
        let needle = query.to_lowercase();
        local_search.borrow_mut().query = query;
        let mut matches = 0;
        for (haystack, widget) in &search_rows {
            let visible = haystack.contains(&needle);
            widget.set_visible(visible);
            matches += usize::from(visible);
        }
        no_matches.set_visible(!search_rows.is_empty() && matches == 0);
    });
    let items = snapshot.items.clone();
    let nav_search = nav.clone();
    let local_activate = local.clone();
    search.connect_activate(move |search| {
        let needle = search.text().to_lowercase();
        if let Some(item) = items.iter().find(|item| {
            !item.is_group
                && !item.is_scene
                && format!("{} {} {}", item.input, item.path.join(" / "), item.owner)
                    .to_lowercase()
                    .contains(needle.as_str())
        }) {
            local_activate.borrow_mut().show_detail = true;
            workspace::inspect_input(&nav_search, Some(item.input.clone()));
        }
    });
    let scroll = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&source_list)
        .build();
    scroll.set_widget_name("sources-list-scroll");
    source_browser.append(&scroll);

    for warning in warnings.iter().chain(snapshot.warnings.iter()) {
        source_browser.append(&text(warning));
    }
    let detail_page = column(16);
    detail_page.add_css_class("sources-detail");
    let split = adw::NavigationSplitView::builder()
        .sidebar_width_fraction(0.36)
        .min_sidebar_width(240.0)
        .max_sidebar_width(360.0)
        .show_content(local.borrow().show_detail)
        .vexpand(true)
        .hexpand(true)
        .build();
    let back = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "sources-back"))
        .halign(Align::Start)
        .visible(split.is_collapsed())
        .build();
    back.set_widget_name("control-focus-back");
    back.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "sources-back")));
    back.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "sources-back"
    ))]);
    split.connect_collapsed_notify({
        let back = back.downgrade();
        move |split| {
            if let Some(back) = back.upgrade() {
                back.set_visible(split.is_collapsed());
            }
        }
    });
    back.connect_clicked({
        let split = split.downgrade();
        let local = local.clone();
        move |_| {
            local.borrow_mut().show_detail = false;
            if let Some(split) = split.upgrade() {
                split.set_show_content(false);
            }
        }
    });
    detail_page.append(&back);
    if let Some(detail) = detail {
        let detail_root = column(16);
        let title = text(&detail.input);
        title.add_css_class("heading");
        detail_root.append(&title);
        detail_root.append(&text(&fl!(LANGUAGE_LOADER, "control-shared-source")));
        if !usage.is_empty() {
            let usages = column(4);
            for item in &usage {
                let title = format!(
                    "{} · {} #{}",
                    item.path.join(" / "),
                    item.owner,
                    item.item_id
                );
                let inspect = Button::with_label(&title);
                inspect.set_widget_name(&format!(
                    "control-focus-usage-{}-{}-{}-{:?}",
                    detail.input, item.owner, item.item_id, item.path
                ));
                inspect.set_halign(Align::Fill);
                if let Some(label) = inspect.child().and_downcast::<gtk4::Label>() {
                    label.set_wrap(true);
                    label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
                    label.set_max_width_chars(40);
                    label.set_xalign(0.0);
                }
                inspect.set_tooltip_text(Some(&format!(
                    "{title}\n{}",
                    fl!(LANGUAGE_LOADER, "control-inspect-source")
                )));
                let target = item
                    .path
                    .first()
                    .cloned()
                    .unwrap_or_else(|| item.owner.clone());
                let nav_usage = nav.clone();
                inspect.connect_clicked(move |_| {
                    workspace::inspect_scene(&nav_usage, Some(target.clone()))
                });
                usages.append(&inspect);
            }
            let usage = gtk4::Expander::builder()
                .label(fl!(LANGUAGE_LOADER, "control-used-in"))
                .child(&usages)
                .expanded(local.borrow().usage_expanded)
                .build();
            usage.set_widget_name("control-focus-usage");
            usage.connect_expanded_notify({
                let local = local.clone();
                move |usage| local.borrow_mut().usage_expanded = usage.is_expanded()
            });
            detail_root.append(&usage);
        }
        for warning in &detail.warnings {
            detail_root.append(&text(warning));
        }
        let editor = column(16);
        // Draft-only editors remain focusable during reads. Gate OBS actions,
        // not their parent, so loading cannot steal an active editing gesture.
        build_source_detail(&editor, &detail, nav, local, &capabilities);
        detail_root.append(&editor);
        detail_page.append(&detail_root);
    }
    if detail_page.last_child().as_ref() == Some(back.upcast_ref()) {
        if let Some(input) = selected_input {
            let name = text(&input);
            name.add_css_class("heading");
            detail_page.append(&name);
            detail_page.append(&text(&fl!(LANGUAGE_LOADER, "control-request-hint")));
        } else {
            detail_page.append(&text(&fl!(LANGUAGE_LOADER, "sources-select-hint")));
        }
    }
    let detail_scroll = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&detail_page)
        .build();
    detail_scroll.set_widget_name("sources-detail-scroll");
    split.set_sidebar(Some(&adw::NavigationPage::new(
        &source_browser,
        &fl!(LANGUAGE_LOADER, "sources-title"),
    )));
    split.set_content(Some(&adw::NavigationPage::new(
        &detail_scroll,
        &fl!(LANGUAGE_LOADER, "control-inspect-source"),
    )));
    let responsive = adw::BreakpointBin::builder()
        .width_request(280)
        .height_request(220)
        .child(&split)
        .vexpand(true)
        .build();
    let breakpoint = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("max-width: 680sp").expect("static Sources breakpoint"),
    );
    breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
    responsive.add_breakpoint(breakpoint);
    page.append(&responsive);
    page.set_vexpand(true);
    page.set_hexpand(true);
    root.append(&page);
    update_status(root, nav, local);
}

fn build_source_detail(
    parent: &GtkBox,
    detail: &SourceDetail,
    nav: &NavigationContext,
    local: &Rc<RefCell<LocalState>>,
    caps: &[String],
) {
    if !detail.filters.is_empty() {
        let filters = column(6);
        for filter in &detail.filters {
            let check = CheckButton::builder()
                .label(&filter.name)
                .active(filter.enabled)
                .sensitive(supported(caps, "SetSourceFilterEnabled"))
                .build();
            mark_mutation(&check, supported(caps, "SetSourceFilterEnabled"));
            let nav = nav.clone();
            let input = detail.input.clone();
            let filter = filter.name.clone();
            check.set_widget_name(&format!("control-focus-filter-{}-{}", detail.input, filter));
            let toggle_local = local.clone();
            check.connect_toggled(move |check| {
                if toggle_local.borrow().syncing {
                    return;
                }
                workspace::act(
                    &nav,
                    ControlAction::SetFilterEnabled {
                        input: input.clone(),
                        filter: filter.clone(),
                        enabled: check.is_active(),
                    },
                )
            });
            filters.append(&check);
        }
        expand(
            parent,
            &fl!(LANGUAGE_LOADER, "control-filters"),
            &filters,
            false,
        );
    }
    if let Some(original) = &detail.text {
        let content = column(6);
        if detail.text_from_file {
            content.append(&text(&fl!(LANGUAGE_LOADER, "control-text-file")));
        } else {
            let input = detail.input.clone();
            let draft = local.borrow_mut().text_draft(&input, original);
            let conflict = local.borrow().text_conflict(&input, original);
            let conflict_warning = text(&fl!(LANGUAGE_LOADER, "sources-text-conflict"));
            conflict_warning.add_css_class("warning");
            conflict_warning.set_widget_name("sources-text-conflict");
            conflict_warning.set_visible(conflict);
            content.append(&conflict_warning);
            let buffer = gtk4::TextBuffer::new(None);
            buffer.set_text(&draft);
            let view = gtk4::TextView::builder()
                .buffer(&buffer)
                .wrap_mode(gtk4::WrapMode::WordChar)
                .height_request(90)
                .build();
            view.set_widget_name(&format!("control-focus-text-{}", detail.input));
            view.update_property(&[gtk4::accessible::Property::Label(&format!(
                "{} · {}",
                fl!(LANGUAGE_LOADER, "control-text"),
                detail.input
            ))]);
            content.append(&view);
            let local_edit = local.clone();
            let input_edit = input.clone();
            let nav_edit = nav.clone();
            buffer.connect_changed(move |buffer| {
                if local_edit.borrow().syncing || !nav_edit.is_current() {
                    return;
                }
                let Some(observed) = observed_text(&nav_edit, &input_edit) else {
                    return;
                };
                let value = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                local_edit
                    .borrow_mut()
                    .edit_text(&input_edit, &observed, value);
            });
            let actions = row();
            let save = Button::with_label(&fl!(LANGUAGE_LOADER, "control-text-save"));
            save.set_widget_name(&format!("control-focus-save-{}", detail.input));
            save.set_sensitive(!conflict && supported(caps, "SetInputSettings"));
            mark_mutation(&save, supported(caps, "SetInputSettings"));
            save.add_css_class("sources-text-save");
            let buffer_save = buffer.clone();
            let local_save = local.clone();
            let nav_save = nav.clone();
            save.connect_clicked(move |_| {
                let expected = local_save
                    .borrow()
                    .text_origins
                    .get(&input)
                    .cloned()
                    .or_else(|| observed_text(&nav_save, &input));
                let Some(expected) = expected else { return };
                workspace::save_text(
                    &nav_save,
                    input.clone(),
                    buffer_save
                        .text(&buffer_save.start_iter(), &buffer_save.end_iter(), true)
                        .to_string(),
                    expected,
                )
            });
            let cancel = Button::with_label(&fl!(LANGUAGE_LOADER, "live-cancel-button-label"));
            cancel.set_widget_name(&format!("control-focus-cancel-{}", detail.input));
            let local = local.clone();
            let input = detail.input.clone();
            let save_cancel = save.clone();
            let can_save = supported(caps, "SetInputSettings");
            let nav_cancel = nav.clone();
            cancel.connect_clicked(move |_| {
                if !nav_cancel.is_current() {
                    return;
                }
                let Some(observed) = observed_text(&nav_cancel, &input) else {
                    return;
                };
                local.borrow_mut().syncing = true;
                buffer.set_text(&observed);
                local.borrow_mut().syncing = false;
                conflict_warning.set_visible(false);
                save_cancel
                    .set_sensitive(can_save && nav_cancel.state.borrow().workspace.can_control());
                local.borrow_mut().text_drafts.remove(&input);
                local.borrow_mut().text_origins.remove(&input);
            });
            actions.append(&save);
            actions.append(&cancel);
            content.append(&actions);
        }
        expand(
            parent,
            &fl!(LANGUAGE_LOADER, "control-text"),
            &content,
            false,
        );
    }
    if let Some(media) = &detail.media {
        let content = column(6);
        let status = text(&media.state.label());
        status.set_widget_name("sources-media-state");
        content.append(&status);
        let buttons = gtk4::FlowBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .max_children_per_line(3)
            .row_spacing(4)
            .column_spacing(4)
            .build();
        for (label, action) in [
            (
                fl!(LANGUAGE_LOADER, "control-media-play"),
                MediaAction::Play,
            ),
            (
                fl!(LANGUAGE_LOADER, "control-media-pause"),
                MediaAction::Pause,
            ),
            (
                fl!(LANGUAGE_LOADER, "control-media-stop"),
                MediaAction::Stop,
            ),
            (
                fl!(LANGUAGE_LOADER, "control-media-restart"),
                MediaAction::Restart,
            ),
            (
                fl!(LANGUAGE_LOADER, "control-media-previous"),
                MediaAction::Previous,
            ),
            (
                fl!(LANGUAGE_LOADER, "control-media-next"),
                MediaAction::Next,
            ),
        ] {
            buttons.insert(
                &action_button(
                    &label,
                    nav,
                    ControlAction::Media {
                        input: detail.input.clone(),
                        action,
                    },
                    supported(caps, "TriggerMediaInputAction"),
                ),
                -1,
            );
        }
        content.append(&buttons);
        if let Some(duration) = media.duration_ms.filter(|duration| *duration > 0) {
            let position = SpinButton::with_range(0.0, duration as f64 / 1000.0, 1.0);
            position.set_widget_name(&format!("control-focus-seek-position-{}", detail.input));
            bind_numeric_draft(
                &position,
                local,
                &detail.input,
                "seek",
                media.cursor_ms.unwrap_or(0).clamp(0, duration) as f64 / 1000.0,
            );
            position.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "control-media-seconds")));
            let seek = Button::with_label(&fl!(LANGUAGE_LOADER, "control-media-seek"));
            seek.set_widget_name(&format!("control-focus-seek-{}", detail.input));
            seek.set_sensitive(supported(caps, "SetMediaInputCursor"));
            mark_mutation(&seek, supported(caps, "SetMediaInputCursor"));
            let line = row();
            line.append(&position);
            let input = detail.input.clone();
            let nav = nav.clone();
            let seek_local = local.clone();
            seek.connect_clicked(move |_| {
                {
                    let mut local = seek_local.borrow_mut();
                    local.numeric_drafts.remove(&(input.clone(), "seek".into()));
                    local.numeric_text.remove(&(input.clone(), "seek".into()));
                }
                workspace::act(
                    &nav,
                    ControlAction::SeekMedia {
                        input: input.clone(),
                        position_ms: (position.value() * 1000.0).round() as i64,
                    },
                )
            });
            line.append(&seek);
            content.append(&line);
        }
        expand(
            parent,
            &fl!(LANGUAGE_LOADER, "control-media"),
            &content,
            false,
        );
    }
    if let Some(audio) = &detail.audio {
        let content = column(6);
        content.append(&text(&fl!(LANGUAGE_LOADER, "control-monitor-host")));
        let choices = [
            fl!(LANGUAGE_LOADER, "control-monitor-off"),
            fl!(LANGUAGE_LOADER, "control-monitor-only"),
            fl!(LANGUAGE_LOADER, "control-monitor-output"),
        ];
        let selected = match audio.monitor {
            AudioMonitor::Off => 0,
            AudioMonitor::MonitorOnly => 1,
            AudioMonitor::MonitorAndOutput => 2,
        };
        let selector = string_selector(&choices, Some(selected));
        selector.set_widget_name(&format!("control-focus-monitor-{}", detail.input));
        selector.set_sensitive(supported(caps, "SetInputAudioMonitorType"));
        mark_mutation(&selector, supported(caps, "SetInputAudioMonitorType"));
        let input = detail.input.clone();
        let nav_monitor = nav.clone();
        let monitor_local = local.clone();
        selector.connect_selected_notify(move |selector| {
            if monitor_local.borrow().syncing {
                return;
            }
            let monitor = match selector.selected() {
                0 => AudioMonitor::Off,
                1 => AudioMonitor::MonitorOnly,
                _ => AudioMonitor::MonitorAndOutput,
            };
            workspace::act(
                &nav_monitor,
                ControlAction::SetAudioMonitor {
                    input: input.clone(),
                    monitor,
                },
            );
        });
        content.append(&selector);
        let sync = SpinButton::with_range(-950.0, 20000.0, 1.0);
        bind_numeric_draft(
            &sync,
            local,
            &detail.input,
            "sync",
            audio.sync_offset_ms as f64,
        );
        add_numeric_apply(
            &content,
            &fl!(LANGUAGE_LOADER, "control-audio-sync"),
            &sync,
            nav,
            supported(caps, "SetInputAudioSyncOffset"),
            {
                let input = detail.input.clone();
                move |value| ControlAction::SetAudioSync {
                    input: input.clone(),
                    offset_ms: value as i64,
                }
            },
        );
        let balance = SpinButton::with_range(0.0, 1.0, 0.01);
        balance.set_digits(2);
        bind_numeric_draft(
            &balance,
            local,
            &detail.input,
            "balance",
            audio.balance.into(),
        );
        add_numeric_apply(
            &content,
            &fl!(LANGUAGE_LOADER, "control-audio-balance"),
            &balance,
            nav,
            supported(caps, "SetInputAudioBalance"),
            {
                let input = detail.input.clone();
                move |value| ControlAction::SetAudioBalance {
                    input: input.clone(),
                    balance: value as f32,
                }
            },
        );
        let tracks = row();
        let draft_tracks = local.borrow_mut().tracks_draft(&detail.input, audio.tracks);
        let checks: Vec<CheckButton> = draft_tracks
            .iter()
            .enumerate()
            .map(|(index, active)| {
                let check = CheckButton::builder()
                    .label((index + 1).to_string())
                    .active(*active)
                    .build();
                check.update_property(&[gtk4::accessible::Property::Label(&fl!(
                    LANGUAGE_LOADER,
                    "control-audio-track",
                    track = (index + 1).to_string()
                ))]);
                check.set_widget_name(&format!("control-focus-track-{}-{index}", detail.input));
                let track_local = local.clone();
                let input = detail.input.clone();
                let track_nav = nav.clone();
                check.connect_toggled(move |check| {
                    if track_local.borrow().syncing || !track_nav.is_current() {
                        return;
                    }
                    let observed = track_nav
                        .state
                        .borrow()
                        .workspace
                        .detail
                        .as_ref()
                        .filter(|detail| detail.input == input)
                        .and_then(|detail| detail.audio.as_ref())
                        .map(|audio| audio.tracks);
                    let Some(observed) = observed else { return };
                    track_local
                        .borrow_mut()
                        .track_drafts
                        .entry(input.clone())
                        .or_insert(observed)[index] = check.is_active();
                });
                tracks.append(&check);
                check
            })
            .collect();
        content.append(&text(&fl!(LANGUAGE_LOADER, "control-audio-tracks")));
        content.append(&tracks);
        let apply = Button::with_label(&fl!(LANGUAGE_LOADER, "control-apply"));
        apply.set_widget_name(&format!("control-focus-tracks-apply-{}", detail.input));
        apply.set_sensitive(supported(caps, "SetInputAudioTracks"));
        mark_mutation(&apply, supported(caps, "SetInputAudioTracks"));
        let input = detail.input.clone();
        let nav = nav.clone();
        apply.connect_clicked(move |_| {
            workspace::act(
                &nav,
                ControlAction::SetAudioTracks {
                    input: input.clone(),
                    tracks: std::array::from_fn(|index| checks[index].is_active()),
                },
            )
        });
        content.append(&apply);
        expand(
            parent,
            &fl!(LANGUAGE_LOADER, "control-advanced-audio"),
            &content,
            false,
        );
    }
}

fn bind_numeric_draft(
    spin: &SpinButton,
    local: &Rc<RefCell<LocalState>>,
    input: &str,
    field: &str,
    observed: f64,
) {
    spin.set_value(local.borrow_mut().numeric_draft(input, field, observed));
    let key = (input.to_owned(), field.to_owned());
    if let Some(text) = local.borrow().numeric_text.get(&key) {
        spin.set_text(text);
    }
    spin.connect_changed({
        let local = local.clone();
        let key = key.clone();
        move |spin| {
            if local.borrow().syncing {
                return;
            }
            local
                .borrow_mut()
                .numeric_text
                .insert(key.clone(), spin.text().to_string());
        }
    });
    let local = local.clone();
    spin.connect_value_changed(move |spin| {
        if local.borrow().syncing {
            return;
        }
        local
            .borrow_mut()
            .numeric_drafts
            .insert(key.clone(), spin.value());
    });
}

fn add_numeric_apply(
    parent: &GtkBox,
    label: &str,
    spin: &SpinButton,
    nav: &NavigationContext,
    enabled: bool,
    action: impl Fn(f64) -> ControlAction + 'static,
) {
    parent.append(&text(label));
    spin.update_property(&[gtk4::accessible::Property::Label(label)]);
    spin.set_widget_name(&format!("control-focus-number-{label}"));
    let line = row();
    line.append(spin);
    let button = Button::with_label(&fl!(LANGUAGE_LOADER, "control-apply"));
    button.set_widget_name(&format!("control-focus-apply-{label}"));
    button.set_sensitive(enabled);
    mark_mutation(&button, enabled);
    let spin = spin.clone();
    let nav = nav.clone();
    button.connect_clicked(move |_| workspace::act(&nav, action(spin.value())));
    line.append(&button);
    parent.append(&line);
}
fn mark_mutation(widget: &impl IsA<gtk4::Widget>, supported: bool) {
    if supported {
        widget.add_css_class("sources-mutation");
    }
}

fn supported(caps: &[String], request: &str) -> bool {
    caps.iter().any(|capability| capability == request)
}
fn action_button(
    label: &str,
    nav: &NavigationContext,
    action: ControlAction,
    enabled: bool,
) -> Button {
    let button = Button::with_label(label);
    button.set_sensitive(enabled);
    mark_mutation(&button, enabled);
    button.set_widget_name(&format!("control-focus-action-{action:?}"));
    let nav = nav.clone();
    button.connect_clicked(move |_| workspace::act(&nav, action.clone()));
    button
}
fn string_selector(values: &[String], selected: Option<usize>) -> DropDown {
    let model = gtk4::StringList::new(&values.iter().map(String::as_str).collect::<Vec<_>>());
    DropDown::builder()
        .model(&model)
        .enable_search(true)
        .selected(selected.map_or(gtk4::INVALID_LIST_POSITION, |index| index as u32))
        .build()
}
fn row() -> GtkBox {
    GtkBox::new(Orientation::Horizontal, 6)
}
fn column(spacing: i32) -> GtkBox {
    GtkBox::new(Orientation::Vertical, spacing)
}
fn text(value: &str) -> Label {
    Label::builder()
        .label(value)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .selectable(true)
        .xalign(0.0)
        .halign(Align::Fill)
        .build()
}
fn expand(parent: &GtkBox, title: &str, content: &GtkBox, _expanded: bool) {
    let section = column(8);
    section.add_css_class("source-detail-section");
    let heading = text(title);
    heading.add_css_class("heading");
    section.append(&heading);
    section.append(content);
    if title == fl!(LANGUAGE_LOADER, "control-media") {
        section.set_widget_name("source-media-controls");
    }
    parent.append(&section);
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires a disposable GTK display; synthetic source observations"]
    fn native_visibility_and_filter_readback_retain_widgets_without_echoing_commands() {
        crate::ui::test_support::native_test(
            "ui::widgets::operating_panel::tests::native_visibility_and_filter_readback_retain_widgets_without_echoing_commands",
            || {
                use super::*;
                use crate::domain::control::{ControlSnapshot, SourceItem, SourceFilter};
                let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
                {
                    let mut state = nav.state.borrow_mut();
                    state.obs_status = ObsStatus::Connected { obs_version: "fixture".into() };
                    state.scene_collections.current = Some("Fixture".into());
                    state.available_requests = vec!["SetSceneItemEnabled".into(), "SetSourceFilterEnabled".into()];
                    state.workspace.observed_at = Some(std::time::Instant::now());
                    state.workspace.snapshot = Some(ControlSnapshot {
                        scene: Some("Main".into()),
                        items: vec![SourceItem {
                            owner: "Main".into(), item_id: 1, input: "Camera".into(), input_uuid: String::new(),
                            kind: None, path: vec!["Main".into()], enabled: true, ancestor_enabled: true,
                            is_group: false, is_scene: false,
                        }], ..Default::default()
                    });
                    state.workspace.selected_input = Some("Camera".into());
                    state.workspace.detail = Some(SourceDetail {
                        input: "Camera".into(), filters: vec![SourceFilter { name: "Crop".into(), kind: "crop_filter".into(), enabled: true }],
                        text: Some("Original".into()),
                        audio: Some(crate::domain::control::AudioSettings { balance: 0.5, sync_offset_ms: 0, monitor: AudioMonitor::Off, tracks: [true; 6] }),
                        ..Default::default()
                    });
                }
                let (page, refresh) = build(nav.clone());
                let mut checks = Vec::new();
                visit(&page, &mut |widget| {
                    if widget.widget_name().starts_with("control-focus-item-") || widget.widget_name().starts_with("control-focus-filter-") {
                        if let Some(check) = widget.downcast_ref::<CheckButton>() { checks.push(check.clone()); }
                    }
                });
                assert_eq!(checks.len(), 2);
                let first = page.first_child();
                let sequence = nav.state.borrow().workspace.sequence;
                for enabled in [false, true, false] {
                    {
                        let mut state = nav.state.borrow_mut();
                        state.workspace.snapshot.as_mut().unwrap().items[0].enabled = enabled;
                        state.workspace.snapshot.as_mut().unwrap().items[0].ancestor_enabled = enabled;
                        let detail = state.workspace.detail.as_mut().unwrap();
                        detail.filters[0].enabled = enabled;
                        detail.text = Some(if enabled { "Enabled" } else { "Disabled" }.into());
                        let audio = detail.audio.as_mut().unwrap();
                        audio.monitor = if enabled { AudioMonitor::MonitorOnly } else { AudioMonitor::Off };
                        audio.sync_offset_ms = if enabled { 150 } else { 250 };
                        audio.balance = if enabled { 0.25 } else { 0.75 };
                        audio.tracks = [enabled; 6];
                    }
                    refresh();
                    assert_eq!(page.first_child(), first);
                    assert!(checks.iter().all(|check| check.parent().is_some() && check.is_active() == enabled));
                    visit(&page, &mut |widget| {
                        if let Some(view) = widget.downcast_ref::<gtk4::TextView>() {
                            let buffer = view.buffer();
                            assert_eq!(buffer.text(&buffer.start_iter(), &buffer.end_iter(), true), if enabled { "Enabled" } else { "Disabled" });
                        }
                        if widget.widget_name() == "control-focus-monitor-Camera" {
                            assert_eq!(widget.downcast_ref::<DropDown>().unwrap().selected(), u32::from(enabled));
                        }
                        if widget.widget_name().starts_with("control-focus-track-Camera-") {
                            assert_eq!(widget.downcast_ref::<CheckButton>().unwrap().is_active(), enabled);
                        }
                    });
                    assert_eq!(nav.state.borrow().workspace.sequence, sequence, "readback emitted a command");
                    assert!(nav.state.borrow().workspace.activity.entries.is_empty());
                }
            },
        );
    }

    #[test]
    fn sources_content_ignores_publication_metadata_but_tracks_control_content() {
        use super::*;
        use crate::controller::state::AppState;
        let mut state = AppState::new(Default::default(), Default::default(), None, None);
        state.workspace.snapshot = Some(Default::default());
        state.workspace.detail = Some(SourceDetail {
            input: "Caption".into(),
            text: Some("Original".into()),
            ..Default::default()
        });
        let original = sources_content_from_state(&state);
        state.workspace.sequence += 1;
        state.workspace.touch();
        state.workspace.loading = true;
        state.workspace.pending = true;
        state.workspace.dirty = true;
        state.workspace.result = "Acknowledged".into();
        state.workspace.error = Some("Read failure".into());
        state.workspace.observed_at = Some(std::time::Instant::now());
        state.workspace.snapshot.as_mut().unwrap().studio.program = Some("Live output".into());
        state.workspace.detail.as_mut().unwrap().volume_db = Some(-6.0);
        assert!(original == sources_content_from_state(&state));
        state.workspace.detail.as_mut().unwrap().text = Some("External edit".into());
        assert!(original != sources_content_from_state(&state));
        state.workspace.detail.as_mut().unwrap().text = Some("Original".into());
        assert!(original == sources_content_from_state(&state));
        state.workspace.epoch += 1;
        assert!(original != sources_content_from_state(&state));
        state.workspace.epoch -= 1;
        state.scene_collections.current = Some("Different collection".into());
        assert!(original != sources_content_from_state(&state));
        state.scene_collections.current = None;
        state.available_requests.push("SetInputSettings".into());
        assert!(original != sources_content_from_state(&state));
        state.available_requests.clear();
        state.workspace.selected_input = Some("Other".into());
        assert!(original != sources_content_from_state(&state));
        state.workspace.selected_input = None;
        state.obs_status = ObsStatus::Connected {
            obs_version: "Synthetic".into(),
        };
        assert!(original != sources_content_from_state(&state));
        state.obs_status = ObsStatus::Disconnected;
        assert!(original == sources_content_from_state(&state));
        state
            .workspace
            .snapshot
            .as_mut()
            .unwrap()
            .warnings
            .push("Changed source warning".into());
        assert!(original != sources_content_from_state(&state));
    }

    #[test]
    #[ignore = "requires a native GTK display; run with --test-threads=1"]
    fn native_long_usage_paths_fit_without_hiding_the_editor() {
        use super::*;
        use crate::{
            controller::{app_controller::AppController, state::AppState},
            domain::control::{ControlSnapshot, SourceItem},
            storage::{config::AppConfig, registry::SceneRegistry},
        };
        const CHILD: &str = "SCENEDECK_SOURCE_USAGE_NATIVE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "ui::widgets::operating_panel::tests::native_long_usage_paths_fit_without_hiding_the_editor", "--ignored", "--test-threads=1"])
                .env(CHILD, "1").output().unwrap();
            assert!(
                output.status.success(),
                "native Sources child failed: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        fn pump() {
            let context = glib::MainContext::default();
            let end = std::time::Instant::now() + std::time::Duration::from_millis(250);
            while std::time::Instant::now() < end {
                while context.pending() {
                    context.iteration(false);
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
        fn named(root: &gtk4::Widget, name: &str) -> gtk4::Widget {
            let mut found = None;
            visit(root, &mut |widget| {
                if widget.widget_name() == name {
                    found = Some(widget.clone());
                }
            });
            found.expect(name)
        }
        crate::ui::test_support::require_native_runner();
        adw::init().expect("native GTK display");
        crate::ui::theme::ThemeManager::apply(&Default::default());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let (tx, _events) = std::sync::mpsc::sync_channel(16);
        let state = Rc::new(RefCell::new(AppState::new(
            AppConfig::default(),
            SceneRegistry::default(),
            None,
            None,
        )));
        let owner = "Long owner with localized words — Довга назва сцени для перевірки";
        let item = SourceItem {
            owner: owner.into(), item_id: 1, input: "Shared caption".into(), input_uuid: String::new(),
            kind: None, path: vec![owner.into(), "AReallyLongUnbrokenNestedSceneIdentifierThatMustStillWrapWithoutForcingHorizontalClipping".into()],
            enabled: true, ancestor_enabled: true, is_group: false, is_scene: false,
        };
        {
            let mut state = state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "synthetic fixture".into(),
            };
            state.scene_collections.current = Some("Synthetic collection".into());
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.selected_input = Some("Shared caption".into());
            state.workspace.snapshot = Some(ControlSnapshot {
                scene: Some(owner.into()),
                items: vec![item.clone()],
                ..Default::default()
            });
            state.workspace.detail = Some(SourceDetail {
                input: "Shared caption".into(),
                text: Some("Editable source text".into()),
                audio: Some(crate::domain::control::AudioSettings {
                    monitor: AudioMonitor::Off,
                    sync_offset_ms: 0,
                    balance: 0.5,
                    tracks: [true; 6],
                }),
                ..Default::default()
            });
            state.workspace.usage = (0..10)
                .map(|index| {
                    let mut path = item.clone();
                    path.item_id += index;
                    path.path.push(format!("Structural path {index}"));
                    path
                })
                .collect();
        }
        let nav = NavigationContext::new(
            state,
            gtk4::Stack::new(),
            Rc::new(RefCell::new(AppController::new(
                runtime.handle().clone(),
                tx,
            ))),
        );
        let root = column(0);
        let local = Rc::new(RefCell::new(LocalState::default()));
        local.borrow_mut().set_epoch(0);
        local.borrow_mut().show_detail = true;
        populate(&root, &nav, &local);
        let window = gtk4::Window::builder()
            .default_width(1100)
            .default_height(740)
            .child(&root)
            .build();
        window.present();
        for (width, height) in [(1100, 740), (640, 600)] {
            window.set_default_size(width, height);
            pump();
            let usage = named(root.upcast_ref(), "control-focus-usage")
                .downcast::<gtk4::Expander>()
                .unwrap();
            assert!(
                !usage.is_expanded(),
                "secondary paths should start collapsed"
            );
            let scroll = named(root.upcast_ref(), "sources-detail-scroll");
            let editor = named(root.upcast_ref(), "control-focus-text-Shared caption");
            let bounds = editor.compute_bounds(&scroll).unwrap();
            assert!(bounds.y() >= 0.0 && bounds.y()+bounds.height() <= scroll.height() as f32,
                "editor should be visible before expanding ten structural paths at {width}px: {bounds:?}");
            usage.set_expanded(true);
            pump();
            let button = usage
                .child()
                .unwrap()
                .first_child()
                .unwrap()
                .downcast::<Button>()
                .unwrap();
            let label = button.child().and_downcast::<gtk4::Label>().unwrap();
            assert!(
                label.layout().line_count() > 1,
                "long paths must actually wrap"
            );
            let bounds = button.compute_bounds(&scroll).unwrap();
            assert!(
                bounds.x() >= 0.0 && bounds.x() + bounds.width() <= scroll.width() as f32,
                "expanded path must fit detail viewport at {width}px: {bounds:?}, viewport={}",
                scroll.width()
            );
            assert!(button.tooltip_text().unwrap().contains(owner));
            // Media/readback refresh replaces detail widgets; preserve the user's
            // explicit expansion rather than closing the path list every second.
            gtk4::prelude::RootExt::set_focus(&window, gtk4::Widget::NONE);
            crate::ui::clear_children(&root);
            populate(&root, &nav, &local);
            pump();
            let restored = named(root.upcast_ref(), "control-focus-usage")
                .downcast::<gtk4::Expander>()
                .unwrap();
            assert!(
                restored.is_expanded(),
                "readback rebuild must preserve expanded paths"
            );
            restored.set_expanded(false);
            pump();
        }
        window.close();

        // Exercise the actual page refresh closure, including its named focus
        // capture and idle restoration, rather than only rebuilding test widgets.
        let (page, refresh) = build(nav.clone());
        let window = gtk4::Window::builder()
            .default_width(1100)
            .default_height(740)
            .child(&page)
            .build();
        window.present();
        pump();
        let usage = named(&page, "control-focus-usage")
            .downcast::<gtk4::Expander>()
            .unwrap();
        usage.set_expanded(true);
        pump();
        let path = usage.child().unwrap().first_child().unwrap();
        let identity = path.widget_name();
        assert!(identity.starts_with("control-focus-usage-"));
        assert!(path.grab_focus());
        refresh();
        pump();
        let restored = gtk4::prelude::RootExt::focus(&window).expect("path focus restored");
        assert_eq!(
            restored, path,
            "unchanged refresh must retain the actual focused path widget"
        );
        assert!(
            restored.is::<Button>(),
            "focus must remain on the path button, not the expander"
        );
        assert_eq!(
            restored.widget_name(),
            identity,
            "focus must retain the same structural path"
        );
        // Real page status publications must not replace search, editor or browser
        // GObjects. Keep an unapplied draft and search focus through the entire
        // request -> acknowledgement -> identical readback cycle.
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.loading = false;
            state.workspace.pending = false;
            state.workspace.dirty = false;
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.available_requests.push("SetInputSettings".into());
            state.available_requests.push("SetSceneItemEnabled".into());
        }
        refresh();
        pump();
        let search = named(&page, "control-focus-search")
            .downcast::<gtk4::SearchEntry>()
            .unwrap();
        let editor = named(&page, "control-focus-text-Shared caption")
            .downcast::<gtk4::TextView>()
            .unwrap();
        let save = named(&page, "control-focus-save-Shared caption");
        let browser = named(&page, "sources-list-scroll");
        let sync_name = format!(
            "control-focus-number-{}",
            fl!(LANGUAGE_LOADER, "control-audio-sync")
        );
        let sync = named(&page, &sync_name).downcast::<SpinButton>().unwrap();
        sync.set_value(125.0);
        // Sync offsets are integer spin buttons: a decimal draft is rejected
        // by GTK before refresh. A lone sign is a valid incomplete integer edit.
        sync.set_text("-");
        assert_eq!(
            sync.text(),
            "-",
            "numeric draft fixture must be accepted before any refresh"
        );
        let track = named(&page, "control-focus-track-Shared caption-1")
            .downcast::<CheckButton>()
            .unwrap();
        track.set_active(false);
        editor.buffer().set_text("Unapplied caption draft");
        search.set_text("Shared");
        pump();
        assert!(search.grab_focus());
        let search_focus = gtk4::prelude::RootExt::focus(&window).unwrap();
        let browser_bounds = browser.compute_bounds(&window).unwrap();
        for stage in 0..4 {
            {
                let mut state = nav.state.borrow_mut();
                state.workspace.sequence += 1;
                state.workspace.touch();
                state.workspace.loading = stage == 1 || stage == 2;
                state.workspace.pending = stage == 1;
                state.workspace.result = format!("Synthetic stage {stage}");
                if stage == 3 {
                    state.workspace.observed_at = Some(std::time::Instant::now());
                }
            }
            refresh();
            pump();
            assert_eq!(
                named(&page, "control-focus-search"),
                search.clone().upcast::<gtk4::Widget>()
            );
            assert_eq!(
                named(&page, "control-focus-text-Shared caption"),
                editor.clone().upcast::<gtk4::Widget>()
            );
            assert_eq!(named(&page, "sources-list-scroll"), browser);
            assert_eq!(
                browser.compute_bounds(&window).unwrap(),
                browser_bounds,
                "status readback shifted the Sources browser"
            );
            assert_eq!(
                gtk4::prelude::RootExt::focus(&window).unwrap(),
                search_focus
            );
            assert_eq!(
                editor.buffer().text(
                    &editor.buffer().start_iter(),
                    &editor.buffer().end_iter(),
                    true
                ),
                "Unapplied caption draft"
            );
            assert_eq!(search.text(), "Shared");
            assert_eq!(
                named(&page, &sync_name),
                sync.clone().upcast::<gtk4::Widget>()
            );
            assert_eq!(sync.text(), "-");
            assert_eq!(sync.value(), 125.0);
            assert!(!track.is_active());
            assert_eq!(save.is_sensitive(), stage != 1);
            assert_eq!(
                named(&page, "control-focus-refresh").is_sensitive(),
                stage != 1 && stage != 2
            );
            assert_eq!(
                named(&page, "sources-status-loading").is_visible(),
                stage == 1 || stage == 2
            );
            assert_eq!(
                named(&page, "sources-status-result")
                    .downcast::<Label>()
                    .unwrap()
                    .label(),
                format!("Synthetic stage {stage}")
            );
        }
        assert!(editor.grab_focus());
        let editor_focus = gtk4::prelude::RootExt::focus(&window).unwrap();
        for loading in [true, false] {
            nav.state.borrow_mut().workspace.loading = loading;
            refresh();
            pump();
            assert_eq!(
                gtk4::prelude::RootExt::focus(&window).unwrap(),
                editor_focus
            );
            assert!(
                save.is_sensitive(),
                "background reads must preserve valid editors"
            );
        }
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.observed_at =
                Some(std::time::Instant::now() - std::time::Duration::from_secs(40));
        }
        refresh();
        assert!(!save.is_sensitive());
        assert!(named(&page, "sources-status-stale").is_visible());
        assert!(named(&page, "sources-status-age")
            .downcast::<Label>()
            .unwrap()
            .label()
            .contains("40"));
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.error = Some("Synthetic read failure".into());
            state.workspace.observed_at = None;
        }
        refresh();
        assert_eq!(
            named(&page, "control-focus-text-Shared caption"),
            editor.clone().upcast::<gtk4::Widget>()
        );
        assert_eq!(
            named(&page, "sources-status-error")
                .downcast::<Label>()
                .unwrap()
                .label(),
            "Synthetic read failure"
        );
        assert!(!named(&page, "sources-status-age").is_visible());
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.error = None;
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.detail.as_mut().unwrap().text = Some("External caption".into());
        }
        refresh();
        let replaced = named(&page, "control-focus-text-Shared caption");
        assert_eq!(
            replaced,
            editor.clone().upcast::<gtk4::Widget>(),
            "text readback must preserve the editor and use current origins for Cancel"
        );
        named(&page, "control-focus-cancel-Shared caption")
            .downcast::<Button>()
            .unwrap()
            .emit_clicked();
        let replaced = replaced.downcast::<gtk4::TextView>().unwrap();
        assert_eq!(
            replaced.buffer().text(
                &replaced.buffer().start_iter(),
                &replaced.buffer().end_iter(),
                true
            ),
            "External caption"
        );
        let old_usage = named(&page, "control-focus-usage")
            .downcast::<gtk4::Expander>()
            .unwrap()
            .child()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<Button>()
            .unwrap();
        let old_scope = nav.scoped();
        nav.state.borrow_mut().workspace.epoch += 1;
        refresh();
        assert_ne!(
            named(&page, "control-focus-text-Shared caption"),
            replaced.upcast::<gtk4::Widget>()
        );
        assert!(!old_scope.is_current());
        let sequence = nav.state.borrow().workspace.sequence;
        old_usage.emit_clicked();
        assert_eq!(
            nav.state.borrow().workspace.sequence,
            sequence,
            "detached old callback must reject the new epoch"
        );
        // Keep native dropdown interaction alive across media-only replies,
        // including a reply that was already in flight when the popup opened.
        {
            let mut state = nav.state.borrow_mut();
            state.current_page = crate::controller::state::Page::Sources;
            state.workspace.loading = false;
            state.workspace.dirty = false;
            state.workspace.observed_at = Some(std::time::Instant::now());
            state.workspace.selected_input = Some("Media".into());
            state.workspace.usage.clear();
            state
                .available_requests
                .push("SetInputAudioMonitorType".into());
            state.workspace.detail = Some(SourceDetail {
                input: "Media".into(),
                media: Some(crate::domain::control::MediaStatus {
                    state: crate::domain::control::MediaState::Playing,
                    duration_ms: Some(4000),
                    cursor_ms: Some(0),
                }),
                audio: Some(crate::domain::control::AudioSettings {
                    monitor: AudioMonitor::Off,
                    sync_offset_ms: 0,
                    balance: 0.5,
                    tracks: [true; 6],
                }),
                ..Default::default()
            });
        }
        window.set_default_size(1100, 1100);
        refresh();
        pump();
        let selector = named(&page, "control-focus-monitor-Media");
        let mut popup = None;
        visit(&selector, &mut |widget| {
            if let Some(popover) = widget.downcast_ref::<gtk4::Popover>() {
                popup = Some(popover.clone());
            }
        });
        let popup = popup.expect("native dropdown popover");
        selector
            .first_child()
            .and_downcast::<gtk4::ToggleButton>()
            .unwrap()
            .set_active(true);
        pump();
        assert!(mapped_popover(&page), "dropdown must actually be mapped");
        for cursor in 1..=6 {
            nav.state
                .borrow_mut()
                .workspace
                .detail
                .as_mut()
                .unwrap()
                .media
                .as_mut()
                .unwrap()
                .cursor_ms = Some(cursor * 250);
            refresh();
            pump();
            assert!(
                popup.is_mapped(),
                "media tick must not close an open dropdown"
            );
            assert_eq!(named(&page, "control-focus-monitor-Media"), selector);
        }
        popup.popdown();
        pump();
        refresh();
        pump();
        let resumed = named(&page, "control-focus-monitor-Media");
        assert_eq!(
            resumed, selector,
            "media readback must retain the native selector"
        );
        assert_eq!(
            named(&page, "control-focus-seek-position-Media")
                .downcast::<SpinButton>()
                .unwrap()
                .value(),
            1.5
        );
        let mut popup = None;
        visit(&resumed, &mut |widget| {
            if let Some(popover) = widget.downcast_ref::<gtk4::Popover>() {
                popup = Some(popover.clone());
            }
        });
        let popup = popup.unwrap();
        resumed
            .first_child()
            .and_downcast::<gtk4::ToggleButton>()
            .unwrap()
            .set_active(true);
        pump();
        assert!(popup.is_mapped());
        {
            let mut state = nav.state.borrow_mut();
            state.workspace.error = Some("Synthetic disconnected read".into());
            state.workspace.observed_at = None;
        }
        refresh();
        pump();
        assert!(
            !popup.is_mapped(),
            "read failure must invalidate an open dropdown immediately"
        );
        assert_eq!(
            named(&page, "control-focus-monitor-Media"),
            resumed,
            "status-only failure must disable, not replace the native selector"
        );
        assert!(!resumed.is_sensitive());
        assert!(named(&page, "sources-status-error").is_visible());
        window.close();
    }

    #[test]
    fn editing_keeps_original_value_when_a_refresh_observes_an_external_edit() {
        let mut local = super::LocalState::default();
        local.edit_text("Caption", "Original", "My draft".into());
        assert_eq!(local.text_draft("Caption", "External edit"), "My draft");
        local.edit_text("Caption", "External edit", "My updated draft".into());
        assert_eq!(
            local.text_origins.get("Caption").map(String::as_str),
            Some("Original")
        );
        local.text_draft("Caption", "My updated draft");
        assert!(!local.text_origins.contains_key("Caption"));
    }
    use super::LocalState;

    #[test]
    fn switching_inputs_retains_drafts_and_detects_only_the_changed_origin() {
        let mut local = LocalState::default();
        local.edit_text("Caption", "Original", "Unsent caption".into());
        local.edit_text("Title", "Original title", "Unsent title".into());
        assert!(!local.text_conflict("Caption", "Original"));
        assert!(local.text_conflict("Caption", "Edited externally"));
        assert!(!local.text_conflict("Title", "Original title"));
        assert_eq!(local.text_draft("Title", "Original title"), "Unsent title");
        assert_eq!(
            local.text_draft("Caption", "Edited externally"),
            "Unsent caption"
        );
    }

    #[test]
    fn media_refresh_preserves_numeric_and_track_drafts_until_readback_or_scope_loss() {
        let mut local = LocalState::default();
        local.set_epoch(1);
        local
            .numeric_drafts
            .insert(("Media".into(), "sync".into()), 125.0);
        local
            .track_drafts
            .insert("Media".into(), [true, false, false, false, false, true]);
        assert_eq!(local.numeric_draft("Media", "sync", 0.0), 125.0);
        assert_eq!(local.numeric_draft("Other", "sync", 0.0), 0.0);
        assert!(local.tracks_draft("Media", [true; 6])[5]);
        assert!(!local.tracks_draft("Media", [true; 6])[1]);
        assert_eq!(local.numeric_draft("Media", "sync", 125.0), 125.0);
        assert!(local.numeric_drafts.is_empty());
        local.tracks_draft("Media", [true, false, false, false, false, true]);
        assert!(local.track_drafts.is_empty());
        local
            .numeric_drafts
            .insert(("Media".into(), "sync".into()), 50.0);
        local.set_epoch(2);
        assert_eq!(local.numeric_draft("Media", "sync", 0.0), 0.0);
    }

    #[test]
    fn ordinary_refresh_and_failed_save_preserve_text_draft() {
        let mut local = LocalState::default();
        local.set_epoch(1);
        local
            .text_drafts
            .insert("Caption".into(), "Unsent edit".into());
        local.set_epoch(1);
        assert_eq!(local.text_draft("Caption", "Original"), "Unsent edit");
        assert_eq!(local.text_draft("Caption", "Original"), "Unsent edit");
        assert_eq!(local.text_draft("Caption", "Unsent edit"), "Unsent edit");
        assert!(!local.text_drafts.contains_key("Caption"));
    }

    #[test]
    fn another_session_cannot_inherit_a_same_named_source_draft() {
        let mut local = LocalState::default();
        local.set_epoch(1);
        local
            .text_drafts
            .insert("Caption".into(), "Old session".into());
        local.query = "Old query".into();
        local.usage_expanded = true;
        local.set_epoch(2);
        assert!(!local.usage_expanded);
        assert_eq!(
            local.text_draft("Caption", "New collection"),
            "New collection"
        );
        assert!(local.query.is_empty());
    }
}
