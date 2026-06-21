//! Main application window.
//!
//! Builds the `adw::NavigationSplitView` shell, wires the GTK→Controller
//! command path via `NavigationContext`, and drives the Controller→GTK event
//! path via a 50 ms glib polling timer.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

type RefreshFn = Rc<dyn Fn()>;

use adw::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Label, ListBox, Orientation, SelectionMode, Stack,
    StackTransitionType, StringList,
};

use crate::app_info::APP_NAME;
use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::AppEvent;
use crate::controller::state::{AppState, ObsStatus, Page};
use crate::domain::appearance::ThemeMode;
use crate::domain::obs::ObsNamedList;
use crate::ui::navigation::NavigationContext;
use crate::ui::pages::live::LivePageHandle;
use crate::ui::register_resources;
use crate::ui::theme::ThemeManager;

const DEFAULT_WIDTH: i32 = 1100;
const DEFAULT_HEIGHT: i32 = 740;

const NAV_PAGES: [Page; 6] = [
    Page::Live,
    Page::Mixer,
    Page::Graph,
    Page::Inventory,
    Page::Doctor,
    Page::Settings,
];

pub fn build_main_window(
    app: &adw::Application,
    state: Rc<RefCell<AppState>>,
    controller: Rc<RefCell<AppController>>,
    event_rx: mpsc::Receiver<AppEvent>,
) -> adw::ApplicationWindow {
    let style_manager = adw::StyleManager::default();
    apply_color_scheme(&style_manager, state.borrow().theme_mode);

    register_resources();
    let theme_report =
        ThemeManager::apply(&crate::storage::config::read_config().config.appearance);
    for warning in &theme_report.warnings {
        tracing::warn!(%warning, "theme warning");
    }

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(DEFAULT_WIDTH)
        .default_height(DEFAULT_HEIGHT)
        .build();
    window.add_css_class("scenedeck-root");

    // ── Content stack ─────────────────────────────────────────────────────────
    let content_stack = Stack::builder()
        .vexpand(true)
        .hexpand(true)
        .transition_type(StackTransitionType::Crossfade)
        .build();

    let nav = NavigationContext::new(state.clone(), content_stack.clone(), controller);

    // Build pages — live returns a handle; others return (widget, refresh_fn).
    let live_handle = crate::ui::pages::live::build(nav.clone());
    let (mixer_widget, mixer_refresh) = crate::ui::pages::mixer::build(nav.clone());
    let (graph_widget, graph_refresh) = crate::ui::pages::graph::build(nav.clone());
    let (inventory_widget, inventory_refresh) = crate::ui::pages::inventory::build(nav.clone());
    let (doctor_widget, doctor_refresh) = crate::ui::pages::doctor::build(nav.clone());
    let (settings_widget, settings_refresh) = crate::ui::pages::settings::build(nav.clone());

    content_stack.add_titled(&live_handle.root, Some(Page::Live.id()), Page::Live.title());
    content_stack.add_titled(&mixer_widget, Some(Page::Mixer.id()), Page::Mixer.title());
    content_stack.add_titled(&graph_widget, Some(Page::Graph.id()), Page::Graph.title());
    content_stack.add_titled(
        &inventory_widget,
        Some(Page::Inventory.id()),
        Page::Inventory.title(),
    );
    content_stack.add_titled(
        &doctor_widget,
        Some(Page::Doctor.id()),
        Page::Doctor.title(),
    );
    content_stack.add_titled(
        &settings_widget,
        Some(Page::Settings.id()),
        Page::Settings.title(),
    );

    let refreshers = PageRefreshers {
        mixer: mixer_refresh,
        graph: graph_refresh,
        inventory: inventory_refresh,
        doctor: doctor_refresh,
        settings: settings_refresh,
    };

    let current_page = state.borrow().current_page;
    content_stack.set_visible_child_name(current_page.id());

    let header_selectors = build_header_selectors(&nav);

    // ── Sidebar ───────────────────────────────────────────────────────────────
    let (sidebar_page, sidebar_list, sidebar_controls) = build_sidebar(&nav);

    sidebar_list.connect_row_selected({
        let nav = nav.clone();
        move |_, row| {
            if let Some(row) = row {
                if let Some(&page) = NAV_PAGES.get(row.index() as usize) {
                    nav.switch_to_page(page);
                }
            }
        }
    });

    // ── Toast overlay (created early so the event poller can reference it) ────
    let toast_overlay = adw::ToastOverlay::new();
    let elapsed_stream_label = live_handle.stream_label.clone();
    let elapsed_record_label = live_handle.record_label.clone();

    // ── Event polling ─────────────────────────────────────────────────────────
    // 50 ms gives responsive-enough UI updates without burning CPU.
    glib::timeout_add_local(Duration::from_millis(50), {
        let nav = nav.clone();
        let toast_overlay = toast_overlay.clone();
        let refreshers = refreshers.clone();
        let header_selectors = header_selectors.clone();
        let sidebar_controls = sidebar_controls.clone();
        move || {
            loop {
                match event_rx.try_recv() {
                    Ok(event) => apply_event(
                        &nav,
                        event,
                        &live_handle,
                        &toast_overlay,
                        &refreshers,
                        &header_selectors,
                        &sidebar_controls,
                    ),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        }
    });

    glib::timeout_add_local(Duration::from_secs(1), {
        let state = state.clone();
        let stream_label = elapsed_stream_label;
        let record_label = elapsed_record_label;
        move || {
            let state = state.borrow();
            stream_label.set_text(&format!(
                "Stream: {}{}",
                state.stream_status.state.label(),
                elapsed_suffix(state.stream_active_since)
            ));
            record_label.set_text(&format!(
                "Record: {}{}",
                state.record_status.state.label(),
                elapsed_suffix(state.record_active_since)
            ));
            glib::ControlFlow::Continue
        }
    });

    // ── Content header bar ────────────────────────────────────────────────────
    let content_header = adw::HeaderBar::new();
    content_header.add_css_class("flat");

    let about_btn = gtk4::Button::builder()
        .icon_name("help-about-symbolic")
        .tooltip_text("About SceneDeck")
        .build();
    about_btn.connect_clicked({
        let window = window.clone();
        move |_| show_about(&window)
    });
    content_header.pack_end(&about_btn);

    let refresh_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh current page")
        .build();
    refresh_btn.connect_clicked({
        let nav = nav.clone();
        let refreshers = refreshers.clone();
        move |_| {
            // Kick off a data re-fetch from OBS (no-op if disconnected).
            nav.dispatch(AppCommand::RefreshData);
            // Also immediately rebuild the current page from AppState.
            let page = nav.state.borrow().current_page;
            refreshers.call(page);
        }
    });
    content_header.pack_start(&refresh_btn);
    content_header.pack_start(&header_selectors.scene_collections.root);
    content_header.pack_start(&header_selectors.profiles.root);

    let content_toolbar = adw::ToolbarView::new();
    content_toolbar.add_top_bar(&content_header);
    content_toolbar.set_content(Some(&content_stack));

    let content_page = adw::NavigationPage::builder()
        .title(APP_NAME)
        .child(&content_toolbar)
        .build();

    // ── Navigation split view ─────────────────────────────────────────────────
    let split = adw::NavigationSplitView::new();
    split.set_sidebar(Some(&sidebar_page));
    split.set_content(Some(&content_page));

    toast_overlay.set_child(Some(&split));
    window.set_content(Some(&toast_overlay));

    super::actions::install(app, &window, nav);

    window.present();
    window
}

// ── Event handler ─────────────────────────────────────────────────────────────

fn apply_event(
    nav: &NavigationContext,
    event: AppEvent,
    live: &LivePageHandle,
    toast: &adw::ToastOverlay,
    refreshers: &PageRefreshers,
    header_selectors: &HeaderSelectors,
    sidebar_controls: &SidebarControls,
) {
    use crate::ui::pages::live::{
        rebuild_audio_cards, rebuild_scene_cards, reset_output_controls, show_disconnected_view,
        show_live_view, update_record_status, update_stream_status,
    };

    match event {
        AppEvent::Connecting => {
            {
                let mut state = nav.state.borrow_mut();
                state.set_obs_status(ObsStatus::Connecting);
                state.stream_active_since = None;
                state.record_active_since = None;
            }
            sidebar_controls.status_label.set_text("Connecting to OBS…");
            set_status_class(&sidebar_controls.status_label, "obs-connecting");
            sidebar_controls.connect_btn.set_label("Connecting…");
            sidebar_controls.connect_btn.set_sensitive(false);
            show_disconnected_view(live, "Connecting to OBS…");
            live.current_scene_label.set_text("Current scene: —");
            rebuild_audio_cards(live, &[], nav);
            reset_output_controls(live);
            update_named_selector(&header_selectors.profiles, &ObsNamedList::default());
            update_named_selector(
                &header_selectors.scene_collections,
                &ObsNamedList::default(),
            );
        }

        AppEvent::Connected(info) => {
            nav.state.borrow_mut().set_obs_status(ObsStatus::Connected {
                obs_version: info.obs_version.clone(),
            });
            sidebar_controls
                .status_label
                .set_text(&format!("Connected — OBS {}", info.obs_version));
            set_status_class(&sidebar_controls.status_label, "obs-connected");
            sidebar_controls.connect_btn.set_label("Disconnect");
            sidebar_controls.connect_btn.set_sensitive(true);
            sidebar_controls
                .connect_btn
                .remove_css_class("suggested-action");
            sidebar_controls
                .connect_btn
                .add_css_class("destructive-action");
            show_live_view(live);
        }

        AppEvent::Disconnected => {
            {
                let mut state = nav.state.borrow_mut();
                state.set_obs_status(ObsStatus::Disconnected);
                state.stream_active_since = None;
                state.record_active_since = None;
            }
            sidebar_controls.status_label.set_text("Disconnected");
            set_status_class(&sidebar_controls.status_label, "obs-disconnected");
            sidebar_controls.connect_btn.set_label("Connect to OBS");
            sidebar_controls.connect_btn.set_sensitive(true);
            sidebar_controls
                .connect_btn
                .add_css_class("suggested-action");
            sidebar_controls
                .connect_btn
                .remove_css_class("destructive-action");
            show_disconnected_view(live, "Connect to OBS to use Live controls");
            live.current_scene_label.set_text("Current scene: —");
            rebuild_audio_cards(live, &[], nav);
            reset_output_controls(live);
            update_named_selector(&header_selectors.profiles, &ObsNamedList::default());
            update_named_selector(
                &header_selectors.scene_collections,
                &ObsNamedList::default(),
            );
        }

        AppEvent::SceneInventoryUpdated(inventory) => {
            show_live_view(live);
            nav.state.borrow_mut().scene_inventory = inventory.clone();
            // Update the current scene label from the inventory's known active scene.
            let scene_text = inventory.current_id.as_deref().unwrap_or("—");
            live.current_scene_label
                .set_text(&format!("Current scene: {scene_text}"));
            rebuild_scene_cards(live, &inventory, nav);
            // Refresh pages that display inventory data if they're currently visible.
            let page = nav.state.borrow().current_page;
            if matches!(page, Page::Mixer | Page::Inventory | Page::Doctor) {
                refreshers.call(page);
            }
        }

        AppEvent::ProfilesUpdated(profiles) => {
            nav.state.borrow_mut().profiles = profiles.clone();
            update_named_selector(&header_selectors.profiles, &profiles);
        }

        AppEvent::SceneCollectionsUpdated(collections) => {
            nav.state.borrow_mut().scene_collections = collections.clone();
            update_named_selector(&header_selectors.scene_collections, &collections);
        }

        AppEvent::CurrentSceneChanged(scene_id) => {
            live.current_scene_label
                .set_text(&format!("Current scene: {scene_id}"));
            let inventory = {
                let mut state = nav.state.borrow_mut();
                state.scene_inventory.current_id = Some(scene_id);
                state.scene_inventory.clone()
            };
            rebuild_scene_cards(live, &inventory, nav);
            if nav.state.borrow().current_page == Page::Mixer {
                refreshers.call(Page::Mixer);
            }
        }

        AppEvent::Error(err) => {
            {
                let mut state = nav.state.borrow_mut();
                state.set_obs_status(ObsStatus::Error(err.to_string()));
                state.stream_active_since = None;
                state.record_active_since = None;
            }
            sidebar_controls
                .status_label
                .set_text(&format!("Error: {err}"));
            set_status_class(&sidebar_controls.status_label, "obs-error");
            sidebar_controls.connect_btn.set_label("Retry");
            sidebar_controls.connect_btn.set_sensitive(true);
            sidebar_controls
                .connect_btn
                .add_css_class("suggested-action");
            sidebar_controls
                .connect_btn
                .remove_css_class("destructive-action");
            reset_output_controls(live);
            show_disconnected_view(live, "OBS connection failed");
            live.current_scene_label.set_text("Current scene: —");

            // Surface the error as a dismissable toast so it's visible even
            // when the user is on a different page.
            toast.add_toast(
                adw::Toast::builder()
                    .title(format!("OBS error: {err}"))
                    .timeout(8)
                    .build(),
            );
        }

        AppEvent::AudioInputsUpdated(inputs) => {
            nav.state.borrow_mut().audio_inputs = inputs.clone();
            rebuild_audio_cards(live, &inputs, nav);
            if nav.state.borrow().current_page == Page::Mixer {
                refreshers.call(Page::Mixer);
            }
        }

        AppEvent::MixerAudioInputsUpdated { scene, inputs } => {
            {
                let mut state = nav.state.borrow_mut();
                state.mixer_audio_scene = Some(scene);
                state.mixer_audio_inputs = inputs;
            }
            if nav.state.borrow().current_page == Page::Mixer {
                refreshers.call(Page::Mixer);
            }
        }

        AppEvent::InputMuteChanged { input, muted } => {
            if let Some(a) = nav
                .state
                .borrow_mut()
                .audio_inputs
                .iter_mut()
                .find(|a| a.id == input)
            {
                a.muted = muted;
            }
            if let Some(a) = nav
                .state
                .borrow_mut()
                .mixer_audio_inputs
                .iter_mut()
                .find(|a| a.id == input)
            {
                a.muted = muted;
            }
            for card in live.audio_cards.borrow().iter() {
                if card.input_id == input {
                    card.update_mute(muted);
                    break;
                }
            }
        }

        AppEvent::InputVolumeChanged {
            input,
            volume_mul,
            volume_db,
        } => {
            if let Some(a) = nav
                .state
                .borrow_mut()
                .audio_inputs
                .iter_mut()
                .find(|a| a.id == input)
            {
                a.volume_mul = volume_mul;
                a.volume_db = volume_db;
            }
            if let Some(a) = nav
                .state
                .borrow_mut()
                .mixer_audio_inputs
                .iter_mut()
                .find(|a| a.id == input)
            {
                a.volume_mul = volume_mul;
                a.volume_db = volume_db;
            }
            for card in live.audio_cards.borrow().iter() {
                if card.input_id == input {
                    card.update_volume(volume_mul, volume_db);
                    break;
                }
            }
        }

        AppEvent::StreamStatusUpdated(status) => {
            let elapsed = {
                let mut state = nav.state.borrow_mut();
                update_active_since(status.active, &mut state.stream_active_since);
                state.stream_status = status.clone();
                state.stream_active_since.map(format_elapsed)
            };
            update_stream_status(live, &status, elapsed);
        }

        AppEvent::RecordStatusUpdated(status) => {
            let (elapsed, last_path) = {
                let mut state = nav.state.borrow_mut();
                update_active_since(status.active, &mut state.record_active_since);
                if let Some(path) = status.detail.as_ref().filter(|path| !path.is_empty()) {
                    state.last_recording_path = Some(path.clone());
                }
                state.record_status = status.clone();
                (
                    state.record_active_since.map(format_elapsed),
                    state.last_recording_path.clone(),
                )
            };
            update_record_status(live, &status, elapsed, last_path.as_deref());
        }

        AppEvent::GraphUpdated(graph) => {
            nav.state.borrow_mut().scene_graph = graph;
            // Refresh pages that display graph data if they're currently visible.
            let page = nav.state.borrow().current_page;
            if matches!(page, Page::Graph | Page::Doctor) {
                refreshers.call(page);
            }
        }

        AppEvent::DiagnosticsUpdated(diagnostics) => {
            nav.state.borrow_mut().diagnostics = diagnostics;
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn set_status_class(label: &gtk4::Label, new_class: &str) {
    for class in &[
        "obs-connected",
        "obs-disconnected",
        "obs-connecting",
        "obs-error",
    ] {
        label.remove_css_class(class);
    }
    label.add_css_class(new_class);
}

fn update_active_since(active: bool, active_since: &mut Option<Instant>) {
    match (active, active_since.is_some()) {
        (true, false) => *active_since = Some(Instant::now()),
        (false, true) => *active_since = None,
        _ => {}
    }
}

fn elapsed_suffix(active_since: Option<Instant>) -> String {
    active_since
        .map(|since| format!(" · {}", format_elapsed(since)))
        .unwrap_or_default()
}

fn format_elapsed(since: Instant) -> String {
    let elapsed = since.elapsed().as_secs();
    let hours = elapsed / 3600;
    let minutes = (elapsed % 3600) / 60;
    let seconds = elapsed % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn build_header_selectors(nav: &NavigationContext) -> HeaderSelectors {
    let profiles = build_named_selector("Profile", "Switch OBS profile");
    {
        let nav = nav.clone();
        let model = profiles.model.clone();
        let updating = profiles.updating.clone();
        profiles.dropdown.connect_selected_notify(move |dropdown| {
            if updating.get() {
                return;
            }
            let selected = dropdown.selected();
            if let Some(name) = model.string(selected) {
                nav.dispatch(AppCommand::SetCurrentProfile(name.to_string()));
            }
        });
    }

    let scene_collections = build_named_selector("Collection", "Switch OBS scene collection");
    {
        let nav = nav.clone();
        let model = scene_collections.model.clone();
        let updating = scene_collections.updating.clone();
        scene_collections
            .dropdown
            .connect_selected_notify(move |dropdown| {
                if updating.get() {
                    return;
                }
                let selected = dropdown.selected();
                if let Some(name) = model.string(selected) {
                    nav.dispatch(AppCommand::SetCurrentSceneCollection(name.to_string()));
                }
            });
    }

    HeaderSelectors {
        profiles,
        scene_collections,
    }
}

fn build_named_selector(label: &str, tooltip: &str) -> NamedSelector {
    let root = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .valign(gtk4::Align::Center)
        .build();
    root.set_visible(false);
    root.add_css_class("header-selector");

    let caption = Label::builder()
        .label(label)
        .valign(gtk4::Align::Center)
        .build();
    caption.add_css_class("caption");

    let model = StringList::new(&[]);
    let dropdown = DropDown::builder()
        .model(&model)
        .selected(gtk4::INVALID_LIST_POSITION)
        .sensitive(false)
        .build();
    dropdown.set_tooltip_text(Some(tooltip));
    dropdown.set_enable_search(true);
    dropdown.set_width_request(170);

    root.append(&caption);
    root.append(&dropdown);

    NamedSelector {
        root,
        dropdown,
        model,
        updating: Rc::new(Cell::new(false)),
    }
}

fn update_named_selector(selector: &NamedSelector, list: &ObsNamedList) {
    selector.updating.set(true);

    let additions: Vec<&str> = list.items.iter().map(String::as_str).collect();
    selector
        .model
        .splice(0, selector.model.n_items(), &additions);

    let selected = list
        .current
        .as_ref()
        .and_then(|current| list.items.iter().position(|item| item == current))
        .map(|idx| idx as u32)
        .unwrap_or(gtk4::INVALID_LIST_POSITION);

    let has_items = !list.items.is_empty();
    selector.root.set_visible(has_items);
    selector.dropdown.set_sensitive(has_items);
    selector.dropdown.set_selected(selected);
    selector.updating.set(false);
}

fn build_sidebar(nav: &NavigationContext) -> (adw::NavigationPage, ListBox, SidebarControls) {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .vexpand(true)
        .build();
    list.add_css_class("navigation-sidebar");

    for page in NAV_PAGES {
        let icon = gtk4::Image::from_icon_name(page.icon_name());
        let row = adw::ActionRow::builder()
            .title(page.title())
            .activatable(true)
            .build();
        row.add_prefix(&icon);
        list.append(&row);
    }

    if let Some(row) = list.row_at_index(0) {
        list.select_row(Some(&row));
    }

    let status_label = Label::builder()
        .label(ObsStatus::Disconnected.label())
        .xalign(0.0)
        .wrap(true)
        .build();
    status_label.add_css_class("obs-disconnected");

    let connect_btn = Button::builder()
        .label("Connect to OBS")
        .halign(gtk4::Align::Fill)
        .hexpand(true)
        .build();
    connect_btn.add_css_class("suggested-action");
    connect_btn.connect_clicked({
        let nav = nav.clone();
        move |_| {
            let status = nav.state.borrow().obs_status.clone();
            match status {
                ObsStatus::Disconnected | ObsStatus::Error(_) => {
                    nav.dispatch(AppCommand::Connect);
                }
                ObsStatus::Connected { .. } | ObsStatus::Connecting => {
                    nav.dispatch(AppCommand::Disconnect);
                }
            }
        }
    });

    let footer = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    footer.add_css_class("sidebar-obs-footer");
    footer.append(&status_label);
    footer.append(&connect_btn);

    let sidebar_content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    sidebar_content.append(&list);
    sidebar_content.append(&footer);

    let sidebar_header = adw::HeaderBar::builder().show_title(false).build();
    let sidebar_toolbar = adw::ToolbarView::new();
    sidebar_toolbar.add_top_bar(&sidebar_header);
    sidebar_toolbar.set_content(Some(&sidebar_content));

    let nav_page = adw::NavigationPage::builder()
        .title(APP_NAME)
        .child(&sidebar_toolbar)
        .build();

    (
        nav_page,
        list,
        SidebarControls {
            status_label,
            connect_btn,
        },
    )
}

fn show_about(parent: &adw::ApplicationWindow) {
    use crate::app_info::{APP_ID, APP_NAME, APP_VERSION};
    adw::AboutWindow::builder()
        .application_name(APP_NAME)
        .application_icon(APP_ID)
        .version(APP_VERSION)
        .developer_name("worxbend")
        .license_type(gtk4::License::MitX11)
        .transient_for(parent)
        .build()
        .present();
}

// ── Per-page refresh callbacks ─────────────────────────────────────────────────

#[derive(Clone)]
struct HeaderSelectors {
    profiles: NamedSelector,
    scene_collections: NamedSelector,
}

#[derive(Clone)]
struct NamedSelector {
    root: GtkBox,
    dropdown: DropDown,
    model: StringList,
    updating: Rc<Cell<bool>>,
}

#[derive(Clone)]
struct SidebarControls {
    status_label: Label,
    connect_btn: Button,
}

#[derive(Clone)]
struct PageRefreshers {
    mixer: RefreshFn,
    graph: RefreshFn,
    inventory: RefreshFn,
    doctor: RefreshFn,
    settings: RefreshFn,
}

impl PageRefreshers {
    /// Call the refresh function for `page` if it has one.
    /// Live page is always kept current by `apply_event`, so it is a no-op here.
    fn call(&self, page: Page) {
        match page {
            Page::Mixer => (self.mixer)(),
            Page::Graph => (self.graph)(),
            Page::Inventory => (self.inventory)(),
            Page::Doctor => (self.doctor)(),
            Page::Settings => (self.settings)(),
            Page::Live => {}
        }
    }
}

pub(crate) fn apply_color_scheme(style_manager: &adw::StyleManager, mode: ThemeMode) {
    let scheme = match mode {
        ThemeMode::System => adw::ColorScheme::Default,
        ThemeMode::Light => adw::ColorScheme::ForceLight,
        ThemeMode::Dark => adw::ColorScheme::ForceDark,
    };
    style_manager.set_color_scheme(scheme);
}
