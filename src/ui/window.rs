//! Main application window.
//!
//! Builds the `adw::NavigationSplitView` shell, wires the GTK→Controller
//! command path via `NavigationContext`, and drives the Controller→GTK event
//! path via a 50 ms glib polling timer.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

type RefreshFn = Rc<dyn Fn()>;
type StreamingChromeRef = Rc<RefCell<Option<StreamingChrome>>>;

use adw::prelude::*;
use gtk4::{Button, Image, Stack, StackTransitionType};
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
use crate::domain::output::{OutputKind, OutputStatus};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;
use crate::ui::pages::live::{output_label, LivePageHandle};
use crate::ui::pages::stats::PopoutSlot as StatsPopoutSlot;
use crate::ui::register_resources;
use crate::ui::sidebar::{self, HeaderSelectors, SidebarControls, NAV_PAGES};
use crate::ui::theme::ThemeManager;
use crate::ui::widgets::status_bar::{self, StatusBarHandle};

const DEFAULT_WIDTH: i32 = 1100;
const DEFAULT_HEIGHT: i32 = 740;

pub fn build_main_window(
    app: &adw::Application,
    state: Rc<RefCell<AppState>>,
    controller: Rc<RefCell<AppController>>,
    event_rx: mpsc::Receiver<AppEvent>,
) -> adw::ApplicationWindow {
    let style_manager = adw::StyleManager::default();
    apply_color_scheme(&style_manager, state.borrow().theme_mode());

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
    let (live_handle, refreshers) = add_pages(&content_stack, &nav);

    let current_page = state.borrow().current_page;
    content_stack.set_visible_child_name(current_page.id());

    let header_selectors = HeaderSelectors::build(&nav);

    // ── Sidebar ───────────────────────────────────────────────────────────────
    let (sidebar_page, sidebar_list, sidebar_controls) = sidebar::build(&nav);
    // From here on, navigating from anywhere moves the sidebar highlight too.
    nav.attach_sidebar(&sidebar_list, &NAV_PAGES);
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

    install_event_polling(&nav, event_rx, &event_ui);
    install_output_clock(&state, &status_bar);

    // OBS performance stats are not polled from here: the session task owns a
    // poll loop that runs for as long as the connection is up, so the status
    // bar and the Stats page history stay current regardless of the open page
    // and regardless of whether the GTK timer fires.

    let content_header = build_content_header(
        &window,
        &nav,
        &refreshers,
        &header_selectors,
        &streaming_chrome,
    );

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

    super::hotkeys::install(&window, &nav, live_handle.clone());

    super::actions::install(app, &window, nav.clone());

    window.present();

    maybe_show_welcome_dialog(&window, &nav);

    window
}

// ── Window assembly phases ────────────────────────────────────────────────────

/// Build every page and add it to the content stack in sidebar order.
///
/// Returns the Live page's handle, which the event loop updates directly, and
/// the refresh callbacks for the pages that rebuild themselves from
/// `AppState`.
fn add_pages(
    content_stack: &Stack,
    nav: &NavigationContext,
) -> (Rc<LivePageHandle>, PageRefreshers) {
    // Live returns a handle; the others return (widget, refresh_fn).
    let live_handle = Rc::new(crate::ui::pages::live::build(nav.clone()));
    let (stats_widget, stats_refresh, stats_popout) = crate::ui::pages::stats::build(nav.clone());
    let (mixer_widget, mixer_refresh) = crate::ui::pages::mixer::build(nav.clone());
    let (graph_widget, graph_refresh) = crate::ui::pages::graph::build(nav.clone());
    let (inventory_widget, inventory_refresh) = crate::ui::pages::inventory::build(nav.clone());
    let (doctor_widget, doctor_refresh) = crate::ui::pages::doctor::build(nav.clone());
    let (settings_widget, settings_refresh) = crate::ui::pages::settings::build(nav.clone());
    let (help_widget, help_refresh) = crate::ui::pages::help::build(nav.clone());

    for (page, widget) in [
        (
            Page::Live,
            live_handle.root.clone().upcast::<gtk4::Widget>(),
        ),
        (Page::Mixer, mixer_widget),
        (Page::Graph, graph_widget),
        (Page::Inventory, inventory_widget),
        (Page::Doctor, doctor_widget),
        (Page::Settings, settings_widget),
        (Page::Help, help_widget),
        (Page::Stats, stats_widget),
    ] {
        content_stack.add_titled(&widget, Some(page.id()), &page.title());
    }

    let refreshers = PageRefreshers {
        stats: stats_refresh,
        stats_popout,
        mixer: mixer_refresh,
        graph: graph_refresh,
        inventory: inventory_refresh,
        doctor: doctor_refresh,
        settings: settings_refresh,
        help: help_refresh,
    };

    (live_handle, refreshers)
}

/// Drain the controller's event channel onto the widgets, forever.
///
/// 50 ms gives responsive-enough UI updates without burning CPU. The loop
/// drains everything queued on each tick so a burst of events costs one tick,
/// not one tick each.
fn install_event_polling(
    nav: &NavigationContext,
    event_rx: mpsc::Receiver<AppEvent>,
    event_ui: &EventUiContext,
) {
    glib::timeout_add_local(Duration::from_millis(50), {
        let nav = nav.clone();
        let event_ui = event_ui.clone();
        move || {
            loop {
                match event_rx.try_recv() {
                    Ok(event) => apply_event(&nav, event, &event_ui),
                    Err(mpsc::TryRecvError::Empty) => break,
                    // The controller is gone; stop the timer with it.
                    Err(mpsc::TryRecvError::Disconnected) => return glib::ControlFlow::Break,
                }
            }
            glib::ControlFlow::Continue
        }
    });
}

/// Re-render the stream and record status lines once a second.
///
/// OBS does not push anything when an output is simply still running, but the
/// elapsed time in those lines has to keep counting, so this ticks on its own.
fn install_output_clock(state: &Rc<RefCell<AppState>>, status_bar: &StatusBarHandle) {
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
}

/// Build the header bar above the content area.
///
/// Also publishes the header and its "live" icon into `streaming_chrome`, so
/// output events can tint the bar while a stream is running.
fn build_content_header(
    window: &adw::ApplicationWindow,
    nav: &NavigationContext,
    refreshers: &PageRefreshers,
    header_selectors: &HeaderSelectors,
    streaming_chrome: &StreamingChromeRef,
) -> adw::HeaderBar {
    let content_header = adw::HeaderBar::new();
    content_header.add_css_class("flat");
    content_header.add_css_class("scenedeck-content-header");

    let stream_live_icon = Image::from_icon_name("media-record-symbolic");
    stream_live_icon.add_css_class("scenedeck-top-streaming-icon");
    stream_live_icon.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "window-stream-live-tooltip")));
    stream_live_icon.set_visible(false);

    let about_btn = header_button(
        "help-about-symbolic",
        fl!(LANGUAGE_LOADER, "window-about-tooltip"),
        {
            let window = window.clone();
            move || super::actions::show_about(&window)
        },
    );
    content_header.pack_end(&about_btn);

    let help_btn = header_button(
        "help-browser-symbolic",
        fl!(LANGUAGE_LOADER, "window-help-tooltip"),
        {
            let nav = nav.clone();
            move || nav.switch_to_page(Page::Help)
        },
    );
    content_header.pack_end(&help_btn);

    let refresh_btn = header_button(
        "view-refresh-symbolic",
        fl!(LANGUAGE_LOADER, "window-refresh-tooltip"),
        {
            let nav = nav.clone();
            let refreshers = refreshers.clone();
            move || {
                // Kick off a data re-fetch from OBS (no-op if disconnected).
                nav.dispatch(AppCommand::RefreshData);
                // Also immediately rebuild the current page from AppState.
                let page = nav.state.borrow().current_page;
                refreshers.call(page);
            }
        },
    );
    content_header.pack_start(&stream_live_icon);
    content_header.pack_start(&refresh_btn);
    header_selectors.pack_into(&content_header);

    *streaming_chrome.borrow_mut() = Some(StreamingChrome {
        header: content_header.clone(),
        top_icon: stream_live_icon,
    });

    content_header
}

fn header_button(icon_name: &str, tooltip: String, on_click: impl Fn() + 'static) -> Button {
    let button = Button::builder()
        .icon_name(icon_name)
        .tooltip_text(tooltip)
        .build();
    button.connect_clicked(move |_| on_click());
    button
}

// ── First-run welcome ─────────────────────────────────────────────────────────

/// Greet a first-time user once, and point them at the Help page.
///
/// The "have we greeted this user" flag lives in `config.json`
/// (`onboarding.welcome_shown`), so the dialog survives at most one launch.
/// It is written as soon as the dialog is shown rather than when a particular
/// button is pressed: a user who closes the window instead of answering has
/// still been greeted, and greeting them again would be a bug, not a service.
fn maybe_show_welcome_dialog(window: &adw::ApplicationWindow, nav: &NavigationContext) {
    if nav.state.borrow().config.onboarding.welcome_shown {
        return;
    }

    crate::ui::persist::persist_config(nav, |config| {
        config.onboarding.welcome_shown = true;
    });

    let dialog = adw::MessageDialog::new(
        Some(window),
        Some(&fl!(LANGUAGE_LOADER, "welcome-dialog-heading")),
        Some(&fl!(LANGUAGE_LOADER, "welcome-dialog-body")),
    );
    dialog.add_response("later", &fl!(LANGUAGE_LOADER, "welcome-dialog-later"));
    dialog.add_response("open", &fl!(LANGUAGE_LOADER, "welcome-dialog-open"));
    dialog.set_response_appearance("open", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("open"));
    dialog.set_close_response("later");
    dialog.connect_response(None, {
        let nav = nav.clone();
        move |dialog, response| {
            if response == "open" {
                nav.switch_to_page(Page::Help);
            }
            dialog.close();
        }
    });
    dialog.present();
}

// ── Event handler ─────────────────────────────────────────────────────────────

/// Apply one `AppEvent` to the widgets.
///
/// The match is exhaustive on purpose. Connection and output events are
/// forwarded to their own handlers because those two families are long enough
/// to read badly inline, but the routing happens *inside* the match rather than
/// in a pre-filter above it, so adding a variant to `AppEvent` is a compile
/// error here instead of a panic when that event first arrives at runtime.
fn apply_event(nav: &NavigationContext, event: AppEvent, ui: &EventUiContext) {
    let EventUiContext {
        live,
        toast,
        refreshers,
        header_selectors,
        sidebar_controls,
        streaming_chrome,
        status_bar,
    } = ui;
    use crate::ui::pages::live::{rebuild_audio_cards, rebuild_scene_cards, show_live_view};

    match event {
        AppEvent::Connecting => show_no_session_ui(
            nav,
            ui,
            &ObsStatus::Connecting,
            &fl!(LANGUAGE_LOADER, "window-status-connecting"),
        ),

        AppEvent::Disconnected => show_no_session_ui(
            nav,
            ui,
            &ObsStatus::Disconnected,
            &fl!(LANGUAGE_LOADER, "window-live-disconnected-hint"),
        ),

        AppEvent::Connected(info) => {
            tracing::info!(
                obs_version = %info.obs_version,
                websocket_version = %info.websocket_version,
                "connected to OBS"
            );
            let obs_status = ObsStatus::Connected {
                obs_version: info.obs_version,
            };
            nav.state.borrow_mut().set_obs_status(obs_status.clone());
            sidebar_controls.apply_connection(&obs_status);
            sync_output_indicators(nav, sidebar_controls, streaming_chrome);
            status_bar::set_connection(status_bar, &obs_status);
            show_live_view(live);
        }

        AppEvent::StreamStatusUpdated(_)
        | AppEvent::RecordStatusUpdated(_)
        | AppEvent::StreamCommandPending(_)
        | AppEvent::RecordCommandPending(_)
        | AppEvent::StreamCommandSucceeded
        | AppEvent::RecordCommandSucceeded
        | AppEvent::StreamCommandFailed(_)
        | AppEvent::RecordCommandFailed(_) => {
            apply_output_event(nav, event, ui);
        }

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
            refreshers.call_visible_among(nav, &[Page::Mixer, Page::Inventory, Page::Doctor]);
        }

        AppEvent::ProfilesUpdated(profiles) => {
            nav.state.borrow_mut().profiles = profiles.clone();
            header_selectors.update_profiles(&profiles);
        }

        AppEvent::SceneCollectionsUpdated(collections) => {
            nav.state.borrow_mut().scene_collections = collections.clone();
            header_selectors.update_scene_collections(&collections);
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
            refreshers.call_if_visible(nav, Page::Mixer);
        }

        AppEvent::Error(err) => {
            // `user_message` rather than `to_string`: the latter is the raw
            // English text from the error type's derive, which would reach a
            // Spanish or Polish user untranslated.
            let message = err.user_message();
            show_no_session_ui(
                nav,
                ui,
                &ObsStatus::Error(message.clone()),
                &fl!(LANGUAGE_LOADER, "window-obs-connection-failed"),
            );

            // Surface the error as a dismissable toast so it's visible even
            // when the user is on a different page.
            toast.add_toast(
                adw::Toast::builder()
                    .title(fl!(
                        LANGUAGE_LOADER,
                        "window-toast-obs-error",
                        error = message
                    ))
                    .timeout(8)
                    .build(),
            );
        }

        AppEvent::AudioInputsUpdated(inputs) => {
            nav.state.borrow_mut().audio_inputs = inputs.clone();
            rebuild_audio_cards(live, &inputs, nav);
            refreshers.call_if_visible(nav, Page::Mixer);
        }

        AppEvent::MixerAudioInputsLoading { scene } => {
            let transition = nav.state.borrow_mut().set_mixer_audio_loading(scene);
            apply_mixer_audio_transition(nav, refreshers, transition);
        }

        AppEvent::MixerAudioInputsUpdated { scene, inputs } => {
            let transition = nav
                .state
                .borrow_mut()
                .set_mixer_audio_success(scene, inputs);
            apply_mixer_audio_transition(nav, refreshers, transition);
        }

        AppEvent::MixerAudioInputsFailed { scene, message } => {
            let transition = nav
                .state
                .borrow_mut()
                .set_mixer_audio_failure(scene, message);
            apply_mixer_audio_transition(nav, refreshers, transition);
        }

        AppEvent::InputMuteChanged { input, muted } => {
            let rebuild_mixer = {
                let mut state = nav.state.borrow_mut();
                state.apply_input_mute(&input, muted);
                should_rebuild_visible_mixer_for_input_event(&state, &input)
            };

            update_live_audio_card(live, &input, |card| card.update_mute(muted));

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
                state.apply_input_volume(&input, volume_mul, volume_db);
                should_rebuild_visible_mixer_for_input_event(&state, &input)
            };

            update_live_audio_card(live, &input, |card| {
                card.update_volume(volume_mul, volume_db)
            });

            if rebuild_mixer {
                refreshers.call(Page::Mixer);
            }
        }

        // Meter widgets read this from `AppState` on their own frames, so
        // there is nothing to rebuild here — storing it is the whole job.
        AppEvent::InputLevelsUpdated(levels) => {
            nav.state.borrow_mut().set_audio_levels(levels);
        }

        AppEvent::GraphUpdated(graph) => {
            nav.state.borrow_mut().scene_graph = graph;
            refreshers.call_visible_among(nav, &[Page::Graph, Page::Doctor]);
        }

        AppEvent::StatsUpdated {
            stats,
            bitrate_kbps,
            stream,
        } => {
            let streaming = {
                let mut state = nav.state.borrow_mut();
                state.set_obs_stats(stats, bitrate_kbps, stream);
                state.stream_status.active
            };
            status_bar::set_stats(status_bar, &stats, bitrate_kbps, streaming);
            // History is recorded on every sample; only redraw when the charts
            // are actually on screen (embedded, or in a detached pop-out).
            refreshers.refresh_stats(nav);
        }
    }
}

/// Redraw the Mixer after a scene-audio refresh moved its state on.
///
/// A reply for a scene the mixer has already left changes nothing on screen,
/// so it is logged and dropped rather than drawn under the wrong scene name.
fn apply_mixer_audio_transition(
    nav: &NavigationContext,
    refreshers: &PageRefreshers,
    transition: MixerAudioRefreshTransition,
) {
    if transition.is_stale() {
        tracing::debug!(?transition, "ignored stale mixer audio refresh");
        return;
    }
    refreshers.call_if_visible(nav, Page::Mixer);
}

/// Put every surface back to "there is no OBS session".
///
/// Connecting, disconnecting, and failing all show the same thing: no scenes,
/// no audio, no profile or collection lists, no performance counters — only
/// the status and the hint differ. They used to say so in three copied
/// blocks, and the copies had drifted.
fn show_no_session_ui(
    nav: &NavigationContext,
    ui: &EventUiContext,
    status: &ObsStatus,
    live_hint: &str,
) {
    use crate::ui::pages::live::{rebuild_audio_cards, show_disconnected_view};

    nav.state.borrow_mut().reset_obs_session(status.clone());

    ui.sidebar_controls.apply_connection(status);
    sync_output_indicators(nav, &ui.sidebar_controls, &ui.streaming_chrome);

    status_bar::set_connection(&ui.status_bar, status);
    status_bar::clear_stats(&ui.status_bar);

    show_disconnected_view(&ui.live, live_hint);
    ui.live
        .current_scene_label
        .set_text(&fl!(LANGUAGE_LOADER, "window-current-scene-none"));
    rebuild_audio_cards(&ui.live, &[], nav);

    // The header lists belong to the session that just ended.
    ui.header_selectors.clear();
}

/// Redraw one output's status-bar slot and the indicators that follow it.
///
/// Every output event ends this way: whatever the event changed, the status
/// bar is re-rendered from `AppState` rather than from the event, so the two
/// can never disagree. Each mutation below stores the status it was given, so
/// reading it back here shows the same value the event carried.
fn refresh_output_status_bar(nav: &NavigationContext, ui: &EventUiContext, kind: OutputKind) {
    let (status, elapsed) = {
        let state = nav.state.borrow();
        match kind {
            OutputKind::Stream => (
                state.stream_status.clone(),
                state.stream_active_since.map(format_elapsed),
            ),
            OutputKind::Record => (
                state.record_status.clone(),
                state.record_active_since.map(format_elapsed),
            ),
        }
    };

    let label = output_label(kind, &status, elapsed.as_deref());
    match kind {
        OutputKind::Stream => status_bar::set_stream(&ui.status_bar, &label, status.active),
        OutputKind::Record => status_bar::set_record(&ui.status_bar, &label, status.active),
    }
    sync_output_indicators(nav, &ui.sidebar_controls, &ui.streaming_chrome);
}

fn apply_output_event(nav: &NavigationContext, event: AppEvent, ui: &EventUiContext) {
    // Each arm makes its one state change; rendering is the shared step that
    // follows it.
    let kind = match event {
        AppEvent::StreamStatusUpdated(status) => {
            let mut state = nav.state.borrow_mut();
            update_active_since(status.active, &mut state.stream_active_since);
            state.set_stream_status(status);
            OutputKind::Stream
        }

        AppEvent::RecordStatusUpdated(status) => {
            let mut state = nav.state.borrow_mut();
            update_active_since(status.active, &mut state.record_active_since);
            remember_recording_path(&mut state, &status);
            state.set_record_status(status);
            OutputKind::Record
        }

        AppEvent::StreamCommandPending(status) => {
            let mut state = nav.state.borrow_mut();
            update_active_since(status.active, &mut state.stream_active_since);
            state.set_stream_command_pending(status);
            OutputKind::Stream
        }

        AppEvent::RecordCommandPending(status) => {
            let mut state = nav.state.borrow_mut();
            update_active_since(status.active, &mut state.record_active_since);
            state.set_record_command_pending(status);
            OutputKind::Record
        }

        AppEvent::StreamCommandSucceeded => {
            nav.state.borrow_mut().set_stream_command_success();
            OutputKind::Stream
        }

        AppEvent::RecordCommandSucceeded => {
            nav.state.borrow_mut().set_record_command_success();
            OutputKind::Record
        }

        AppEvent::StreamCommandFailed(failure) => {
            nav.state
                .borrow_mut()
                .set_stream_command_failure_with_recovery(failure);
            OutputKind::Stream
        }

        AppEvent::RecordCommandFailed(failure) => {
            nav.state
                .borrow_mut()
                .set_record_command_failure_with_recovery(failure);
            OutputKind::Record
        }

        // Unreachable: `apply_event` matches exhaustively and only forwards the
        // eight output variants here.
        _ => {
            debug_assert!(false, "non-output event routed to output handler");
            return;
        }
    };

    refresh_output_status_bar(nav, ui, kind);
}

/// Keep the last non-empty recording path OBS reported.
///
/// OBS sends the path when a recording stops and an empty string at other
/// times; an empty value means "no new path", not "there was no recording".
fn remember_recording_path(state: &mut AppState, status: &OutputStatus) {
    if let Some(path) = status.detail.as_ref().filter(|path| !path.is_empty()) {
        state.last_recording_path = Some(path.clone());
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Apply `change` to the Live page's card for `input_id`, if it has one.
///
/// Input ids are unique, so the search stops at the first match.
fn update_live_audio_card(
    live: &LivePageHandle,
    input_id: &str,
    change: impl FnOnce(&crate::ui::widgets::audio_card::AudioCardHandle),
) {
    if let Some(card) = live
        .audio_cards
        .borrow()
        .iter()
        .find(|card| card.input_id == input_id)
    {
        change(card);
    }
}

fn update_active_since(active: bool, active_since: &mut Option<Instant>) {
    match (active, active_since.is_some()) {
        (true, false) => *active_since = Some(Instant::now()),
        (false, true) => *active_since = None,
        _ => {}
    }
}

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

fn sync_output_indicators(
    nav: &NavigationContext,
    sidebar: &SidebarControls,
    streaming_chrome: &StreamingChromeRef,
) {
    let streaming = {
        let state = nav.state.borrow();
        sidebar.sync_output_buttons(&state);
        state.stream_status.active
    };

    sync_streaming_chrome(streaming_chrome, streaming);
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

#[derive(Clone)]
struct StreamingChrome {
    header: adw::HeaderBar,
    top_icon: Image,
}

#[derive(Clone)]
struct PageRefreshers {
    stats: RefreshFn,
    /// Set while a detached stats window is open; see
    /// `ui::pages::stats::PopoutSlot`.
    stats_popout: StatsPopoutSlot,
    mixer: RefreshFn,
    graph: RefreshFn,
    inventory: RefreshFn,
    doctor: RefreshFn,
    settings: RefreshFn,
    help: RefreshFn,
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
    /// Refresh `page`, but only when it is the page currently on screen.
    ///
    /// Most events land while the user is looking at something else, and
    /// rebuilding a hidden page is wasted work that the page's own `map`
    /// handler would redo on the way back in anyway.
    fn call_if_visible(&self, nav: &NavigationContext, page: Page) {
        if nav.state.borrow().current_page == page {
            self.call(page);
        }
    }

    /// Refresh the Stats page when it is on screen, and the pop-out window
    /// whenever one is open, regardless of which page the sidebar shows.
    fn refresh_stats(&self, nav: &NavigationContext) {
        self.call_if_visible(nav, Page::Stats);
        if let Some(popout_refresh) = self.stats_popout.borrow().as_ref() {
            popout_refresh();
        }
    }

    /// Refresh the visible page when it is one of `pages`.
    ///
    /// For data that several pages render — an inventory update reaches
    /// Mixer, Inventory, and Doctor — only the one on screen needs rebuilding.
    fn call_visible_among(&self, nav: &NavigationContext, pages: &[Page]) {
        let page = nav.state.borrow().current_page;
        if pages.contains(&page) {
            self.call(page);
        }
    }

    /// Call the refresh function for `page` if it has one.
    /// Live page is always kept current by `apply_event`, so it is a no-op here.
    fn call(&self, page: Page) {
        match page {
            Page::Stats => (self.stats)(),
            Page::Mixer => (self.mixer)(),
            Page::Graph => (self.graph)(),
            Page::Inventory => (self.inventory)(),
            Page::Doctor => (self.doctor)(),
            Page::Settings => (self.settings)(),
            Page::Help => (self.help)(),
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
