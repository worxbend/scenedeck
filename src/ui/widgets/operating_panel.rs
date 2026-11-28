//! Focused native OBS inspector. Inspection never changes the Program scene.

mod media_progress;

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
                    operator_reads: Vec::new(),
                    target_uuid: None,
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
    sources: Option<(
        Vec<crate::domain::control::SourceItem>,
        Vec<crate::domain::control::WorkspaceWarning>,
    )>,
    detail: Option<SourceDetail>,
    usage: Vec<crate::domain::control::SourceItem>,
    warnings: Vec<crate::domain::control::WorkspaceWarning>,
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
    media_progress::update(
        root,
        nav.state
            .borrow()
            .workspace
            .detail
            .as_ref()
            .and_then(|detail| detail.media.as_ref()),
        connected && fresh && error.is_none(),
    );
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
        source_browser.append(&text(&warning.label()));
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
        content.append(&media_progress::build());
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
mod tests;
