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
type StreamingChromeRef = Rc<RefCell<Option<StreamingChrome>>>;

use adw::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Image, Label, ListBox, Orientation, SelectionMode, Stack,
    StackTransitionType, StringList,
};
use i18n_embed_fl::fl;

use crate::app_info::APP_NAME;
use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::AppEvent;
use crate::controller::state::{
    AppState, MixerAudioRefreshTransition, MixerVisibleAudioStatus, MixerVisibleRenderSource,
    ObsStatus, Page,
};
use crate::domain::appearance::ThemeMode;
use crate::domain::obs::ObsNamedList;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;
use crate::ui::pages::live::{output_label, LivePageHandle};
use crate::ui::register_resources;
use crate::ui::theme::ThemeManager;
use crate::ui::widgets::status_bar::{self, StatusBarHandle};

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
    ThemeManager::apply_async(state.borrow().config.appearance.clone(), |report| {
        for warning in report.warnings {
            tracing::warn!(%warning, "theme warning");
        }
    });

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
    content_stack.add_css_class("scenedeck-content-stack");

    let nav = NavigationContext::new(state.clone(), content_stack.clone(), controller);

    // Build pages — live returns a handle; others return (widget, refresh_fn).
    let live_handle = Rc::new(crate::ui::pages::live::build(nav.clone()));
    let (mixer_widget, mixer_refresh) = crate::ui::pages::mixer::build(nav.clone());
    let (graph_widget, graph_refresh) = crate::ui::pages::graph::build(nav.clone());
    let (inventory_widget, inventory_refresh) = crate::ui::pages::inventory::build(nav.clone());
    let (doctor_widget, doctor_refresh) = crate::ui::pages::doctor::build(nav.clone());
    let (settings_widget, settings_refresh) = crate::ui::pages::settings::build(nav.clone());

    content_stack.add_titled(
        &live_handle.root,
        Some(Page::Live.id()),
        &Page::Live.title(),
    );
    content_stack.add_titled(&mixer_widget, Some(Page::Mixer.id()), &Page::Mixer.title());
    content_stack.add_titled(&graph_widget, Some(Page::Graph.id()), &Page::Graph.title());
    content_stack.add_titled(
        &inventory_widget,
        Some(Page::Inventory.id()),
        &Page::Inventory.title(),
    );
    content_stack.add_titled(
        &doctor_widget,
        Some(Page::Doctor.id()),
        &Page::Doctor.title(),
    );
    content_stack.add_titled(
        &settings_widget,
        Some(Page::Settings.id()),
        &Page::Settings.title(),
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
    let streaming_chrome: StreamingChromeRef = Rc::new(RefCell::new(None));

    // ── Status bar ────────────────────────────────────────────────────────────
    let status_bar = status_bar::build();

    sidebar_list.connect_row_selected({
        let nav = nav.clone();
        let live_handle = Rc::clone(&live_handle);
        move |_, row| {
            if let Some(row) = row {
                if let Some(&page) = NAV_PAGES.get(row.index() as usize) {
                    nav.switch_to_page(page);
                    if page == Page::Live {
                        let inventory = nav.state.borrow().scene_inventory.clone();
                        crate::ui::pages::live::rebuild_scene_cards(&live_handle, &inventory, &nav);
                    }
                }
            }
        }
    });

    // ── Toast overlay (created early so the event poller can reference it) ────
    let toast_overlay = adw::ToastOverlay::new();
    let event_ui = EventUiContext {
        live: live_handle.clone(),
        toast: toast_overlay.clone(),
        refreshers: refreshers.clone(),
        header_selectors: header_selectors.clone(),
        sidebar_controls: sidebar_controls.clone(),
        streaming_chrome: streaming_chrome.clone(),
        status_bar: status_bar.clone(),
    };

    // ── Event polling ─────────────────────────────────────────────────────────
    // 50 ms gives responsive-enough UI updates without burning CPU.
    glib::timeout_add_local(Duration::from_millis(50), {
        let nav = nav.clone();
        let event_ui = event_ui.clone();
        move || {
            loop {
                match event_rx.try_recv() {
                    Ok(event) => apply_event(&nav, event, &event_ui),
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
        let status_bar = status_bar.clone();
        move || {
            let state = state.borrow();
            status_bar::set_stream(
                &status_bar,
                &fl!(
                    LANGUAGE_LOADER,
                    "window-stream-status-line",
                    state = state.stream_status.state.label(),
                    elapsed = elapsed_suffix(state.stream_active_since)
                ),
                state.stream_status.active,
            );
            status_bar::set_record(
                &status_bar,
                &fl!(
                    LANGUAGE_LOADER,
                    "window-record-status-line",
                    state = state.record_status.state.label(),
                    elapsed = elapsed_suffix(state.record_active_since)
                ),
                state.record_status.active,
            );
            glib::ControlFlow::Continue
        }
    });

    // Poll OBS performance stats on a slower cadence than the elapsed-time
    // tick above — CPU/FPS/bitrate don't need per-second precision, and this
    // keeps `GetStats` traffic light while OBS is otherwise idle.
    glib::timeout_add_local(Duration::from_secs(2), {
        let nav = nav.clone();
        move || {
            if matches!(nav.state.borrow().obs_status, ObsStatus::Connected { .. }) {
                nav.dispatch(AppCommand::RefreshStats);
            }
            glib::ControlFlow::Continue
        }
    });

    // ── Content header bar ────────────────────────────────────────────────────
    let content_header = adw::HeaderBar::new();
    content_header.add_css_class("flat");
    content_header.add_css_class("scenedeck-content-header");

    let stream_live_icon = Image::from_icon_name("media-record-symbolic");
    stream_live_icon.add_css_class("scenedeck-top-streaming-icon");
    stream_live_icon.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "window-stream-live-tooltip")));
    stream_live_icon.set_visible(false);

    let about_btn = gtk4::Button::builder()
        .icon_name("help-about-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "window-about-tooltip"))
        .build();
    about_btn.connect_clicked({
        let window = window.clone();
        move |_| show_about(&window)
    });
    content_header.pack_end(&about_btn);

    let refresh_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "window-refresh-tooltip"))
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
    content_header.pack_start(&stream_live_icon);
    content_header.pack_start(&refresh_btn);
    content_header.pack_start(&header_selectors.scene_collections.root);
    content_header.pack_start(&header_selectors.profiles.root);
    *streaming_chrome.borrow_mut() = Some(StreamingChrome {
        header: content_header.clone(),
        top_icon: stream_live_icon,
    });

    let content_toolbar = adw::ToolbarView::new();
    content_toolbar.add_css_class("scenedeck-content-toolbar");
    content_toolbar.add_top_bar(&content_header);
    content_toolbar.set_content(Some(&content_stack));

    let content_page = adw::NavigationPage::builder()
        .title(APP_NAME)
        .child(&content_toolbar)
        .build();

    // ── Navigation split view ─────────────────────────────────────────────────
    let split = adw::NavigationSplitView::new();
    split.add_css_class("scenedeck-split");
    split.set_sidebar(Some(&sidebar_page));
    split.set_content(Some(&content_page));

    toast_overlay.add_css_class("scenedeck-toast-overlay");
    toast_overlay.set_child(Some(&split));

    let outer_toolbar = adw::ToolbarView::new();
    outer_toolbar.add_css_class("scenedeck-outer-toolbar");
    outer_toolbar.set_content(Some(&toast_overlay));
    outer_toolbar.add_bottom_bar(&status_bar.root);
    window.set_content(Some(&outer_toolbar));

    super::actions::install(app, &window, nav);

    window.present();
    window
}

// ── Event handler ─────────────────────────────────────────────────────────────

fn apply_event(nav: &NavigationContext, event: AppEvent, ui: &EventUiContext) {
    if matches!(
        &event,
        AppEvent::Connecting | AppEvent::Connected(_) | AppEvent::Disconnected
    ) {
        apply_connection_event(nav, event, ui);
        return;
    }
    if matches!(
        &event,
        AppEvent::StreamStatusUpdated(_)
            | AppEvent::RecordStatusUpdated(_)
            | AppEvent::StreamCommandPending(_)
            | AppEvent::RecordCommandPending(_)
            | AppEvent::StreamCommandSucceeded
            | AppEvent::RecordCommandSucceeded
            | AppEvent::StreamCommandFailed(_)
            | AppEvent::RecordCommandFailed(_)
    ) {
        apply_output_event(nav, event, ui);
        return;
    }
    let EventUiContext {
        live,
        toast,
        refreshers,
        header_selectors,
        sidebar_controls,
        streaming_chrome,
        status_bar,
    } = ui;
    use crate::ui::pages::live::{
        rebuild_audio_cards, rebuild_scene_cards, show_disconnected_view, show_live_view,
    };

    match event {
        AppEvent::SceneInventoryUpdated(inventory) => {
            show_live_view(live);
            let inventory = {
                let mut state = nav.state.borrow_mut();
                let mut inventory = inventory.clone();
                inventory.previous_id = previous_scene_for_inventory_update(
                    state.scene_inventory.current_id.as_deref(),
                    state.scene_inventory.previous_id.as_deref(),
                    inventory.current_id.as_deref(),
                );
                state.scene_inventory = inventory.clone();
                inventory
            };
            // Update the current scene label from the inventory's known active scene.
            let scene_text = inventory.current_id.as_deref().unwrap_or("—");
            live.current_scene_label.set_text(&fl!(
                LANGUAGE_LOADER,
                "window-current-scene",
                scene = scene_text
            ));
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
            live.current_scene_label.set_text(&fl!(
                LANGUAGE_LOADER,
                "window-current-scene",
                scene = scene_id.clone()
            ));
            let inventory = {
                let mut state = nav.state.borrow_mut();
                state.scene_inventory.set_current_scene(scene_id);
                state.scene_inventory.clone()
            };
            rebuild_scene_cards(live, &inventory, nav);
            if nav.state.borrow().current_page == Page::Mixer {
                refreshers.call(Page::Mixer);
            }
        }

        AppEvent::Error(err) => {
            let obs_status = ObsStatus::Error(err.to_string());
            {
                let mut state = nav.state.borrow_mut();
                state.set_obs_status(obs_status.clone());
                state.set_stream_status(OutputStatus::default());
                state.set_record_status(OutputStatus::default());
                state.scene_inventory = Default::default();
                state.stream_active_since = None;
                state.record_active_since = None;
                state.clear_pending_mixer_audio_refresh();
                state.clear_output_command_errors();
                state.clear_obs_stats();
            }
            sidebar_controls.status_label.set_text(&fl!(
                LANGUAGE_LOADER,
                "window-status-error",
                error = err.to_string()
            ));
            set_status_class(&sidebar_controls.status_label, "obs-error");
            sidebar_controls
                .connect_btn
                .set_label(&fl!(LANGUAGE_LOADER, "window-connect-btn-retry"));
            sidebar_controls.connect_btn.set_sensitive(true);
            sidebar_controls
                .connect_btn
                .add_css_class("suggested-action");
            sidebar_controls
                .connect_btn
                .remove_css_class("destructive-action");
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
            status_bar::set_connection(status_bar, &obs_status);
            status_bar::clear_stats(status_bar);
            show_disconnected_view(live, &fl!(LANGUAGE_LOADER, "window-obs-connection-failed"));
            live.current_scene_label
                .set_text(&fl!(LANGUAGE_LOADER, "window-current-scene-none"));

            // Surface the error as a dismissable toast so it's visible even
            // when the user is on a different page.
            toast.add_toast(
                adw::Toast::builder()
                    .title(fl!(
                        LANGUAGE_LOADER,
                        "window-toast-obs-error",
                        error = err.to_string()
                    ))
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
            let transition = {
                let mut state = nav.state.borrow_mut();
                state.set_mixer_audio_success(scene, inputs)
            };
            if transition == MixerAudioRefreshTransition::StaleSuccess {
                tracing::debug!("ignored stale mixer audio success");
                return;
            }
            if nav.state.borrow().current_page == Page::Mixer {
                refreshers.call(Page::Mixer);
            }
        }

        AppEvent::MixerAudioInputsLoading { scene } => {
            {
                let mut state = nav.state.borrow_mut();
                state.set_mixer_audio_loading(scene);
            }
            if nav.state.borrow().current_page == Page::Mixer {
                refreshers.call(Page::Mixer);
            }
        }

        AppEvent::MixerAudioInputsFailed { scene, message } => {
            let transition = {
                let mut state = nav.state.borrow_mut();
                state.set_mixer_audio_failure(scene, message)
            };
            if transition == MixerAudioRefreshTransition::StaleFailure {
                tracing::debug!("ignored stale mixer audio failure");
                return;
            }
            if nav.state.borrow().current_page == Page::Mixer {
                refreshers.call(Page::Mixer);
            }
        }

        AppEvent::InputMuteChanged { input, muted } => {
            let rebuild_mixer = {
                let mut state = nav.state.borrow_mut();
                if let Some(a) = state.audio_inputs.iter_mut().find(|a| a.id == input) {
                    a.muted = muted;
                }
                state.update_mixer_input_mute(&input, muted);
                should_rebuild_visible_mixer_for_input_event(&state, &input)
            };

            for card in live.audio_cards.borrow().iter() {
                if card.input_id == input {
                    card.update_mute(muted);
                    break;
                }
            }

            if rebuild_mixer {
                refreshers.call(Page::Mixer);
            }
        }

        AppEvent::InputVolumeChanged {
            input,
            volume_mul,
            volume_db,
        } => {
            let rebuild_mixer = {
                let mut state = nav.state.borrow_mut();
                if let Some(a) = state.audio_inputs.iter_mut().find(|a| a.id == input) {
                    a.volume_mul = volume_mul;
                    a.volume_db = volume_db;
                }
                state.update_mixer_input_volume(&input, volume_mul, volume_db);
                should_rebuild_visible_mixer_for_input_event(&state, &input)
            };

            for card in live.audio_cards.borrow().iter() {
                if card.input_id == input {
                    card.update_volume(volume_mul, volume_db);
                    break;
                }
            }

            if rebuild_mixer {
                refreshers.call(Page::Mixer);
            }
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

        AppEvent::StatsUpdated {
            stats,
            bitrate_kbps,
        } => {
            let streaming = {
                let mut state = nav.state.borrow_mut();
                state.set_obs_stats(stats, bitrate_kbps);
                state.stream_status.active
            };
            status_bar::set_stats(status_bar, &stats, bitrate_kbps, streaming);
        }
        _ => unreachable!("specialized event was not routed before general event handling"),
    }
}

fn apply_connection_event(nav: &NavigationContext, event: AppEvent, ui: &EventUiContext) {
    let EventUiContext {
        live,
        header_selectors,
        sidebar_controls,
        streaming_chrome,
        status_bar,
        ..
    } = ui;
    use crate::ui::pages::live::{rebuild_audio_cards, show_disconnected_view, show_live_view};

    match event {
        AppEvent::Connecting => {
            {
                let mut state = nav.state.borrow_mut();
                state.set_obs_status(ObsStatus::Connecting);
                state.set_stream_status(OutputStatus::default());
                state.set_record_status(OutputStatus::default());
                state.scene_inventory = Default::default();
                state.stream_active_since = None;
                state.record_active_since = None;
                state.clear_pending_mixer_audio_refresh();
                state.clear_output_command_errors();
                state.clear_obs_stats();
            }
            sidebar_controls
                .status_label
                .set_text(&fl!(LANGUAGE_LOADER, "window-status-connecting"));
            set_status_class(&sidebar_controls.status_label, "obs-connecting");
            sidebar_controls
                .connect_btn
                .set_label(&fl!(LANGUAGE_LOADER, "window-connect-btn-connecting"));
            sidebar_controls.connect_btn.set_sensitive(false);
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
            status_bar::set_connection(status_bar, &ObsStatus::Connecting);
            status_bar::clear_stats(status_bar);
            show_disconnected_view(live, &fl!(LANGUAGE_LOADER, "window-status-connecting"));
            live.current_scene_label
                .set_text(&fl!(LANGUAGE_LOADER, "window-current-scene-none"));
            rebuild_audio_cards(live, &[], nav);
            update_named_selector(&header_selectors.profiles, &ObsNamedList::default());
            update_named_selector(
                &header_selectors.scene_collections,
                &ObsNamedList::default(),
            );
        }

        AppEvent::Connected(info) => {
            let obs_status = ObsStatus::Connected {
                obs_version: info.obs_version.clone(),
            };
            nav.state.borrow_mut().set_obs_status(obs_status.clone());
            sidebar_controls.status_label.set_text(&fl!(
                LANGUAGE_LOADER,
                "window-status-connected",
                version = info.obs_version.clone()
            ));
            set_status_class(&sidebar_controls.status_label, "obs-connected");
            sidebar_controls
                .connect_btn
                .set_label(&fl!(LANGUAGE_LOADER, "window-connect-btn-disconnect"));
            sidebar_controls.connect_btn.set_sensitive(true);
            sidebar_controls
                .connect_btn
                .remove_css_class("suggested-action");
            sidebar_controls
                .connect_btn
                .add_css_class("destructive-action");
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
            status_bar::set_connection(status_bar, &obs_status);
            show_live_view(live);
        }

        AppEvent::Disconnected => {
            {
                let mut state = nav.state.borrow_mut();
                state.set_obs_status(ObsStatus::Disconnected);
                state.set_stream_status(OutputStatus::default());
                state.set_record_status(OutputStatus::default());
                state.scene_inventory = Default::default();
                state.stream_active_since = None;
                state.record_active_since = None;
                state.clear_pending_mixer_audio_refresh();
                state.clear_output_command_errors();
                state.clear_obs_stats();
            }
            sidebar_controls
                .status_label
                .set_text(&fl!(LANGUAGE_LOADER, "window-status-disconnected"));
            set_status_class(&sidebar_controls.status_label, "obs-disconnected");
            sidebar_controls
                .connect_btn
                .set_label(&fl!(LANGUAGE_LOADER, "window-connect-btn-connect"));
            sidebar_controls.connect_btn.set_sensitive(true);
            sidebar_controls
                .connect_btn
                .add_css_class("suggested-action");
            sidebar_controls
                .connect_btn
                .remove_css_class("destructive-action");
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
            status_bar::set_connection(status_bar, &ObsStatus::Disconnected);
            status_bar::clear_stats(status_bar);
            show_disconnected_view(live, &fl!(LANGUAGE_LOADER, "window-live-disconnected-hint"));
            live.current_scene_label
                .set_text(&fl!(LANGUAGE_LOADER, "window-current-scene-none"));
            rebuild_audio_cards(live, &[], nav);
            update_named_selector(&header_selectors.profiles, &ObsNamedList::default());
            update_named_selector(
                &header_selectors.scene_collections,
                &ObsNamedList::default(),
            );
        }

        _ => unreachable!("non-connection event routed to connection handler"),
    }
}

fn apply_output_event(nav: &NavigationContext, event: AppEvent, ui: &EventUiContext) {
    let EventUiContext {
        sidebar_controls,
        streaming_chrome,
        status_bar,
        ..
    } = ui;

    match event {
        AppEvent::StreamStatusUpdated(status) => {
            let (elapsed, _error) = {
                let mut state = nav.state.borrow_mut();
                update_active_since(status.active, &mut state.stream_active_since);
                state.set_stream_status(status.clone());
                (
                    state.stream_active_since.map(format_elapsed),
                    state.last_stream_command_error.clone(),
                )
            };
            status_bar::set_stream(
                status_bar,
                &output_label(
                    &fl!(LANGUAGE_LOADER, "window-output-kind-stream"),
                    &status,
                    elapsed.as_deref(),
                ),
                status.active,
            );
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
        }

        AppEvent::RecordStatusUpdated(status) => {
            let (elapsed, _last_path, _error) = {
                let mut state = nav.state.borrow_mut();
                update_active_since(status.active, &mut state.record_active_since);
                if let Some(path) = status.detail.as_ref().filter(|path| !path.is_empty()) {
                    state.last_recording_path = Some(path.clone());
                }
                state.set_record_status(status.clone());
                (
                    state.record_active_since.map(format_elapsed),
                    state.last_recording_path.clone(),
                    state.last_record_command_error.clone(),
                )
            };
            status_bar::set_record(
                status_bar,
                &output_label(
                    &fl!(LANGUAGE_LOADER, "window-output-kind-record"),
                    &status,
                    elapsed.as_deref(),
                ),
                status.active,
            );
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
        }

        AppEvent::StreamCommandPending(status) => {
            let (elapsed, _error) = {
                let mut state = nav.state.borrow_mut();
                update_active_since(status.active, &mut state.stream_active_since);
                state.set_stream_command_pending(status.clone());
                (
                    state.stream_active_since.map(format_elapsed),
                    state.last_stream_command_error.clone(),
                )
            };
            status_bar::set_stream(
                status_bar,
                &output_label(
                    &fl!(LANGUAGE_LOADER, "window-output-kind-stream"),
                    &status,
                    elapsed.as_deref(),
                ),
                status.active,
            );
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
        }

        AppEvent::RecordCommandPending(status) => {
            let (elapsed, _last_path, _error) = {
                let mut state = nav.state.borrow_mut();
                update_active_since(status.active, &mut state.record_active_since);
                state.set_record_command_pending(status.clone());
                (
                    state.record_active_since.map(format_elapsed),
                    state.last_recording_path.clone(),
                    state.last_record_command_error.clone(),
                )
            };
            status_bar::set_record(
                status_bar,
                &output_label(
                    &fl!(LANGUAGE_LOADER, "window-output-kind-record"),
                    &status,
                    elapsed.as_deref(),
                ),
                status.active,
            );
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
        }

        AppEvent::StreamCommandSucceeded => {
            let (status, elapsed) = {
                let mut state = nav.state.borrow_mut();
                state.set_stream_command_success();
                (
                    state.stream_status.clone(),
                    state.stream_active_since.map(format_elapsed),
                )
            };
            status_bar::set_stream(
                status_bar,
                &output_label(
                    &fl!(LANGUAGE_LOADER, "window-output-kind-stream"),
                    &status,
                    elapsed.as_deref(),
                ),
                status.active,
            );
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
        }

        AppEvent::RecordCommandSucceeded => {
            let (status, elapsed, _last_path) = {
                let mut state = nav.state.borrow_mut();
                state.set_record_command_success();
                (
                    state.record_status.clone(),
                    state.record_active_since.map(format_elapsed),
                    state.last_recording_path.clone(),
                )
            };
            status_bar::set_record(
                status_bar,
                &output_label(
                    &fl!(LANGUAGE_LOADER, "window-output-kind-record"),
                    &status,
                    elapsed.as_deref(),
                ),
                status.active,
            );
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
        }

        AppEvent::StreamCommandFailed(failure) => {
            let (status, elapsed, _error) = {
                let mut state = nav.state.borrow_mut();
                state.set_stream_command_failure_with_recovery(failure);
                (
                    state.stream_status.clone(),
                    state.stream_active_since.map(format_elapsed),
                    state.last_stream_command_error.clone(),
                )
            };
            status_bar::set_stream(
                status_bar,
                &output_label(
                    &fl!(LANGUAGE_LOADER, "window-output-kind-stream"),
                    &status,
                    elapsed.as_deref(),
                ),
                status.active,
            );
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
        }

        AppEvent::RecordCommandFailed(failure) => {
            let (status, elapsed, _last_path, _error) = {
                let mut state = nav.state.borrow_mut();
                state.set_record_command_failure_with_recovery(failure);
                (
                    state.record_status.clone(),
                    state.record_active_since.map(format_elapsed),
                    state.last_recording_path.clone(),
                    state.last_record_command_error.clone(),
                )
            };
            status_bar::set_record(
                status_bar,
                &output_label(
                    &fl!(LANGUAGE_LOADER, "window-output-kind-record"),
                    &status,
                    elapsed.as_deref(),
                ),
                status.active,
            );
            sync_output_indicators(sidebar_controls, streaming_chrome, &nav.state.borrow());
        }
        _ => unreachable!("non-output event routed to output handler"),
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

#[allow(dead_code)]
pub(crate) fn should_rebuild_visible_mixer_for_input_event(
    state: &AppState,
    input_name: &str,
) -> bool {
    if state.current_page != Page::Mixer {
        return false;
    }

    match state.visible_mixer_render_source() {
        MixerVisibleRenderSource::ActiveScene(inputs)
        | MixerVisibleRenderSource::Scene {
            status: MixerVisibleAudioStatus::Loaded(inputs),
            ..
        } => inputs.iter().any(|input| input.id == input_name),
        MixerVisibleRenderSource::Scene {
            status:
                MixerVisibleAudioStatus::Loading
                | MixerVisibleAudioStatus::Error(_)
                | MixerVisibleAudioStatus::Missing,
            ..
        }
        | MixerVisibleRenderSource::MissingScene => false,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SidebarOutputButtonModel {
    label: String,
    sensitive: bool,
    suggested: bool,
    destructive: bool,
}

fn sidebar_output_button_model(
    status: &OutputStatus,
    connected: bool,
    start_label: String,
    stop_label: String,
) -> SidebarOutputButtonModel {
    if status.state.is_transitioning() {
        return SidebarOutputButtonModel {
            label: match status.state {
                OutputRunState::Starting => fl!(LANGUAGE_LOADER, "window-sidebar-output-starting"),
                OutputRunState::Stopping => fl!(LANGUAGE_LOADER, "window-sidebar-output-stopping"),
                OutputRunState::Reconnecting => {
                    fl!(LANGUAGE_LOADER, "window-sidebar-output-reconnecting")
                }
                _ => fl!(LANGUAGE_LOADER, "window-sidebar-output-working"),
            },
            sensitive: false,
            suggested: false,
            destructive: status.active,
        };
    }

    if status.active {
        SidebarOutputButtonModel {
            label: stop_label,
            sensitive: connected,
            suggested: false,
            destructive: connected,
        }
    } else {
        SidebarOutputButtonModel {
            label: start_label,
            sensitive: connected,
            suggested: connected,
            destructive: false,
        }
    }
}

fn apply_sidebar_output_button(button: &Button, model: SidebarOutputButtonModel) {
    button.set_label(&model.label);
    button.set_sensitive(model.sensitive);
    if model.suggested {
        button.add_css_class("suggested-action");
    } else {
        button.remove_css_class("suggested-action");
    }
    if model.destructive {
        button.add_css_class("destructive-action");
    } else {
        button.remove_css_class("destructive-action");
    }
}

fn sync_output_indicators(
    sidebar: &SidebarControls,
    streaming_chrome: &StreamingChromeRef,
    state: &AppState,
) {
    sync_sidebar_output_buttons(sidebar, state);
    sync_streaming_chrome(streaming_chrome, state.stream_status.active);
}

fn sync_sidebar_output_buttons(sidebar: &SidebarControls, state: &AppState) {
    let connected = matches!(state.obs_status, ObsStatus::Connected { .. });
    apply_sidebar_output_button(
        &sidebar.stream_btn,
        sidebar_output_button_model(
            &state.stream_status,
            connected,
            fl!(LANGUAGE_LOADER, "window-sidebar-start-stream"),
            fl!(LANGUAGE_LOADER, "window-sidebar-stop-stream"),
        ),
    );
    apply_sidebar_output_button(
        &sidebar.record_btn,
        sidebar_output_button_model(
            &state.record_status,
            connected,
            fl!(LANGUAGE_LOADER, "window-sidebar-start-recording"),
            fl!(LANGUAGE_LOADER, "window-sidebar-stop-recording"),
        ),
    );
    sync_live_sidebar_icon(sidebar, state.stream_status.active);
}

fn sync_streaming_chrome(streaming_chrome: &StreamingChromeRef, streaming: bool) {
    let Some(chrome) = streaming_chrome.borrow().as_ref().cloned() else {
        return;
    };

    chrome.top_icon.set_visible(streaming);
    if streaming {
        chrome
            .header
            .add_css_class("scenedeck-content-header-streaming");
        chrome
            .top_icon
            .add_css_class("scenedeck-top-streaming-icon-active");
    } else {
        chrome
            .header
            .remove_css_class("scenedeck-content-header-streaming");
        chrome
            .top_icon
            .remove_css_class("scenedeck-top-streaming-icon-active");
    }
}

fn sync_live_sidebar_icon(sidebar: &SidebarControls, streaming: bool) {
    if streaming {
        sidebar
            .live_icon
            .add_css_class("scenedeck-sidebar-live-icon-streaming");
    } else {
        sidebar
            .live_icon
            .remove_css_class("scenedeck-sidebar-live-icon-streaming");
    }
}

fn previous_scene_for_inventory_update(
    old_current: Option<&str>,
    old_previous: Option<&str>,
    new_current: Option<&str>,
) -> Option<String> {
    match (old_current, new_current) {
        (Some(old), Some(new)) if old != new => Some(old.to_string()),
        _ => old_previous.map(str::to_string),
    }
}

fn build_header_selectors(nav: &NavigationContext) -> HeaderSelectors {
    let profiles = build_named_selector(
        &fl!(LANGUAGE_LOADER, "window-selector-profile-label"),
        &fl!(LANGUAGE_LOADER, "window-selector-profile-tooltip"),
    );
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

    let scene_collections = build_named_selector(
        &fl!(LANGUAGE_LOADER, "window-selector-collection-label"),
        &fl!(LANGUAGE_LOADER, "window-selector-collection-tooltip"),
    );
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
    dropdown.add_css_class("scenedeck-dropdown");
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
    list.add_css_class("scenedeck-sidebar-list");

    let mut live_icon = None;
    for page in NAV_PAGES {
        let icon = Image::from_icon_name(page.icon_name());
        if page == Page::Live {
            icon.add_css_class("scenedeck-sidebar-live-icon");
            live_icon = Some(icon.clone());
        }
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
        .label(fl!(LANGUAGE_LOADER, "window-connect-btn-connect"))
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

    let stream_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "window-sidebar-start-stream"))
        .halign(gtk4::Align::Fill)
        .hexpand(true)
        .sensitive(false)
        .build();
    stream_btn.add_css_class("sidebar-output-button");
    stream_btn.connect_clicked({
        let nav = nav.clone();
        move |button| crate::ui::pages::live::handle_stream_output_toggle(button, &nav)
    });

    let record_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "window-sidebar-start-recording"))
        .halign(gtk4::Align::Fill)
        .hexpand(true)
        .sensitive(false)
        .build();
    record_btn.add_css_class("sidebar-output-button");
    record_btn.connect_clicked({
        let nav = nav.clone();
        move |button| crate::ui::pages::live::handle_record_output_toggle(button, &nav)
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
    footer.append(&stream_btn);
    footer.append(&record_btn);
    footer.append(&connect_btn);

    let sidebar_content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    sidebar_content.add_css_class("scenedeck-sidebar");
    sidebar_content.append(&list);
    sidebar_content.append(&footer);

    let sidebar_header = adw::HeaderBar::builder().show_title(false).build();
    sidebar_header.add_css_class("scenedeck-sidebar-header");
    let sidebar_toolbar = adw::ToolbarView::new();
    sidebar_toolbar.add_css_class("scenedeck-sidebar-toolbar");
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
            live_icon: live_icon.expect("Live page icon"),
            stream_btn,
            record_btn,
            connect_btn,
        },
    )
}

fn show_about(parent: &adw::ApplicationWindow) {
    use crate::app_info::{APP_ID, APP_NAME, APP_VERSION};
    let about = adw::AboutWindow::builder()
        .application_name(APP_NAME)
        .application_icon(APP_ID)
        .version(APP_VERSION)
        .developer_name("worxbend")
        .license_type(gtk4::License::MitX11)
        .transient_for(parent)
        .build();
    about.add_css_class("scenedeck-about-window");
    about.present();
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
    live_icon: Image,
    stream_btn: Button,
    record_btn: Button,
    connect_btn: Button,
}

#[derive(Clone)]
struct StreamingChrome {
    header: adw::HeaderBar,
    top_icon: Image,
}

#[derive(Clone)]
struct PageRefreshers {
    mixer: RefreshFn,
    graph: RefreshFn,
    inventory: RefreshFn,
    doctor: RefreshFn,
    settings: RefreshFn,
}

#[derive(Clone)]
struct EventUiContext {
    live: Rc<LivePageHandle>,
    toast: adw::ToastOverlay,
    refreshers: PageRefreshers,
    header_selectors: HeaderSelectors,
    sidebar_controls: SidebarControls,
    streaming_chrome: StreamingChromeRef,
    status_bar: StatusBarHandle,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::audio::AudioInput;
    use crate::domain::mixer::MixerMode;

    fn input(id: &str) -> AudioInput {
        AudioInput::new(id.to_string(), false, 1.0, 0.0)
    }

    fn app_state() -> AppState {
        AppState::new(
            crate::storage::config::AppConfig::default(),
            crate::storage::registry::SceneRegistry::default(),
            None,
            None,
        )
    }

    fn selected_mixer_state() -> AppState {
        let mut state = app_state();
        state.current_page = Page::Mixer;
        state.mixer.mode = MixerMode::SelectedScene;
        state.mixer.selected_scene = Some("Scene A".to_string());
        state
    }

    fn output_status(active: bool, state: OutputRunState) -> OutputStatus {
        OutputStatus {
            active,
            state,
            detail: None,
        }
    }

    #[test]
    fn sidebar_output_button_model_reflects_connection_and_output_state() {
        assert_eq!(
            sidebar_output_button_model(
                &output_status(false, OutputRunState::Inactive),
                false,
                "Start Stream".to_string(),
                "Stop Stream".to_string(),
            ),
            SidebarOutputButtonModel {
                label: "Start Stream".to_string(),
                sensitive: false,
                suggested: false,
                destructive: false,
            }
        );
        assert_eq!(
            sidebar_output_button_model(
                &output_status(false, OutputRunState::Inactive),
                true,
                "Start Stream".to_string(),
                "Stop Stream".to_string(),
            ),
            SidebarOutputButtonModel {
                label: "Start Stream".to_string(),
                sensitive: true,
                suggested: true,
                destructive: false,
            }
        );
        assert_eq!(
            sidebar_output_button_model(
                &output_status(true, OutputRunState::Active),
                true,
                "Start Stream".to_string(),
                "Stop Stream".to_string(),
            ),
            SidebarOutputButtonModel {
                label: "Stop Stream".to_string(),
                sensitive: true,
                suggested: false,
                destructive: true,
            }
        );
        assert_eq!(
            sidebar_output_button_model(
                &output_status(false, OutputRunState::Starting),
                true,
                "Start Stream".to_string(),
                "Stop Stream".to_string(),
            ),
            SidebarOutputButtonModel {
                label: "Starting…".to_string(),
                sensitive: false,
                suggested: false,
                destructive: false,
            }
        );
    }

    #[test]
    fn inventory_refresh_tracks_previous_scene_only_when_current_changes() {
        assert_eq!(
            previous_scene_for_inventory_update(Some("A"), None, Some("B")),
            Some("A".to_string())
        );
        assert_eq!(
            previous_scene_for_inventory_update(Some("A"), Some("Prev"), Some("A")),
            Some("Prev".to_string())
        );
        assert_eq!(
            previous_scene_for_inventory_update(None, Some("Prev"), Some("A")),
            Some("Prev".to_string())
        );
    }

    #[test]
    fn mixer_input_event_rebuilds_for_visible_active_input() {
        let mut state = app_state();
        state.current_page = Page::Mixer;
        state.mixer.mode = MixerMode::ActiveScene;
        state.audio_inputs = vec![input("Mic"), input("Music")];

        assert!(should_rebuild_visible_mixer_for_input_event(&state, "Mic"));
    }

    #[test]
    fn mixer_input_event_ignores_unrelated_active_input() {
        let mut state = app_state();
        state.current_page = Page::Mixer;
        state.mixer.mode = MixerMode::ActiveScene;
        state.audio_inputs = vec![input("Mic")];

        assert!(!should_rebuild_visible_mixer_for_input_event(
            &state, "Music"
        ));
    }

    #[test]
    fn mixer_input_event_rebuilds_for_visible_loaded_selected_input() {
        let mut state = selected_mixer_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic"), input("Music")]);

        assert!(should_rebuild_visible_mixer_for_input_event(&state, "Mic"));
    }

    #[test]
    fn mixer_input_event_selected_mode_follows_render_source_current_scene_fallback() {
        let mut state = app_state();
        state.current_page = Page::Mixer;
        state.mixer.mode = MixerMode::SelectedScene;
        state.scene_inventory.current_id = Some("Current".to_string());
        state.set_mixer_audio_loading("Current".to_string());
        state.set_mixer_audio_success("Current".to_string(), vec![input("Fallback Mic")]);

        assert!(should_rebuild_visible_mixer_for_input_event(
            &state,
            "Fallback Mic"
        ));
    }

    #[test]
    fn mixer_input_event_rebuilds_for_visible_loaded_pinned_input() {
        let mut state = app_state();
        state.current_page = Page::Mixer;
        state.mixer.mode = MixerMode::PinnedScene;
        state.mixer.selected_scene = Some("Scene A".to_string());
        state.mixer.pinned_scene = Some("Pinned".to_string());
        state.set_mixer_audio_loading("Pinned".to_string());
        state.set_mixer_audio_success("Pinned".to_string(), vec![input("Pinned Mic")]);

        assert!(should_rebuild_visible_mixer_for_input_event(
            &state,
            "Pinned Mic"
        ));
    }

    #[test]
    fn mixer_input_event_pinned_mode_follows_render_source_selected_scene_fallback() {
        let mut state = app_state();
        state.current_page = Page::Mixer;
        state.mixer.mode = MixerMode::PinnedScene;
        state.mixer.selected_scene = Some("Selected".to_string());
        state.scene_inventory.current_id = Some("Current".to_string());
        state.set_mixer_audio_loading("Selected".to_string());
        state.set_mixer_audio_success("Selected".to_string(), vec![input("Selected Mic")]);

        assert!(should_rebuild_visible_mixer_for_input_event(
            &state,
            "Selected Mic"
        ));
        assert!(!should_rebuild_visible_mixer_for_input_event(
            &state,
            "Current Mic"
        ));
    }

    #[test]
    fn mixer_input_event_pinned_mode_follows_render_source_current_scene_fallback() {
        let mut state = app_state();
        state.current_page = Page::Mixer;
        state.mixer.mode = MixerMode::PinnedScene;
        state.scene_inventory.current_id = Some("Current".to_string());
        state.set_mixer_audio_loading("Current".to_string());
        state.set_mixer_audio_success("Current".to_string(), vec![input("Current Mic")]);

        assert!(should_rebuild_visible_mixer_for_input_event(
            &state,
            "Current Mic"
        ));
    }

    #[test]
    fn mixer_input_event_ignores_unrelated_loaded_input() {
        let mut state = selected_mixer_state();
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic")]);

        assert!(!should_rebuild_visible_mixer_for_input_event(
            &state, "Music"
        ));
    }

    #[test]
    fn mixer_input_event_ignores_non_mixer_page() {
        let mut state = selected_mixer_state();
        state.current_page = Page::Live;
        state.set_mixer_audio_loading("Scene A".to_string());
        state.set_mixer_audio_success("Scene A".to_string(), vec![input("Mic")]);

        assert!(!should_rebuild_visible_mixer_for_input_event(&state, "Mic"));
    }

    #[test]
    fn mixer_input_event_ignores_loading_error_missing_and_empty_snapshots() {
        let mut loading = selected_mixer_state();
        loading.set_mixer_audio_loading("Scene A".to_string());
        assert!(!should_rebuild_visible_mixer_for_input_event(
            &loading, "Mic"
        ));

        let mut error = selected_mixer_state();
        error.set_mixer_audio_loading("Scene A".to_string());
        error.set_mixer_audio_failure("Scene A".to_string(), "OBS failed".to_string());
        assert!(!should_rebuild_visible_mixer_for_input_event(&error, "Mic"));

        let missing = selected_mixer_state();
        assert!(!should_rebuild_visible_mixer_for_input_event(
            &missing, "Mic"
        ));

        let mut empty = selected_mixer_state();
        empty.set_mixer_audio_loading("Scene A".to_string());
        empty.set_mixer_audio_success("Scene A".to_string(), Vec::new());
        assert!(!should_rebuild_visible_mixer_for_input_event(&empty, "Mic"));
    }

    #[test]
    fn mixer_input_event_ignores_snapshot_for_other_scene() {
        let mut state = selected_mixer_state();
        state.set_mixer_audio_loading("Scene B".to_string());
        state.set_mixer_audio_success("Scene B".to_string(), vec![input("Mic")]);

        assert!(!should_rebuild_visible_mixer_for_input_event(&state, "Mic"));
    }
}
