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
use gtk4::gdk;
use gtk4::gdk::prelude::DisplayExtManual;
use gtk4::{
    Box as GtkBox, Button, DropDown, Image, Label, ListBox, Orientation, SelectionMode, Stack,
    StackTransitionType, StringList,
};
use i18n_embed_fl::fl;

use crate::app_info::{APP_ID, APP_NAME};
use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::AppEvent;
use crate::controller::state::{
    AppState, MixerAudioRefreshTransition, MixerVisibleAudioStatus, MixerVisibleRenderSource,
    ObsStatus, Page,
};
use crate::domain::appearance::ThemeMode;
use crate::domain::hotkey::{KeyStroke, KeySymbol, Modifiers, SceneHotkeyConfig};
use crate::domain::obs::ObsNamedList;
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::hotkey_service::{HotkeyOutcome, SceneHotkeyResolver};
use crate::ui::navigation::NavigationContext;
use crate::ui::pages::live::{output_label, LivePageHandle};
use crate::ui::register_resources;
use crate::ui::theme::ThemeManager;
use crate::ui::widgets::status_bar::{self, StatusBarHandle};

const DEFAULT_WIDTH: i32 = 1100;
const DEFAULT_HEIGHT: i32 = 740;

const NAV_PAGES: [Page; 8] = [
    Page::Live,
    Page::Mixer,
    Page::Graph,
    Page::Inventory,
    Page::Doctor,
    Page::Settings,
    Page::Help,
    Page::Stats,
];

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

    let header_selectors = build_header_selectors(&nav);

    // ── Sidebar ───────────────────────────────────────────────────────────────
    let (sidebar_page, sidebar_list, sidebar_controls) = build_sidebar(&nav);
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

    install_scene_hotkeys(&window, &nav, live_handle.clone());

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
    let (stats_widget, stats_refresh) = crate::ui::pages::stats::build(nav.clone());
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
    content_header.pack_start(&header_selectors.scene_collections.root);
    content_header.pack_start(&header_selectors.profiles.root);

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

// ── Scene hotkeys ─────────────────────────────────────────────────────────────

/// Wire Live-page scene shortcuts to a window-level key controller.
///
/// The controller runs in the capture phase so bare digits and the leader key
/// are seen before any focused widget consumes them, and it re-reads
/// `config.hotkeys` on every press so changes in Settings apply immediately.
fn install_scene_hotkeys(
    window: &adw::ApplicationWindow,
    nav: &NavigationContext,
    live: Rc<LivePageHandle>,
) {
    let resolver = Rc::new(RefCell::new(SceneHotkeyResolver::new()));
    let controller = gtk4::EventControllerKey::new();
    controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    controller.connect_key_pressed({
        let nav = nav.clone();
        let window = window.clone();
        move |_, keyval, keycode, modifier_state| {
            let hotkeys = {
                let state = nav.state.borrow();
                // Scene shortcuts belong to the Live page; elsewhere the digits
                // are just digits.
                if state.current_page != Page::Live {
                    resolver.borrow_mut().disarm();
                    return glib::Propagation::Proceed;
                }
                state.config.hotkeys
            };
            if !hotkeys.enabled {
                resolver.borrow_mut().disarm();
                return glib::Propagation::Proceed;
            }
            // Bindings that fire without a modifier would swallow typing, so
            // they stand down while a text widget has focus.
            if !binds_with_modifiers(&hotkeys) && text_input_has_focus(&window) {
                return glib::Propagation::Proceed;
            }

            let stroke = KeyStroke::new(
                key_symbol(&window, keyval, keycode),
                modifiers_from_state(modifier_state),
            );
            let outcome = resolver
                .borrow_mut()
                .resolve(&hotkeys, stroke, Instant::now());
            apply_hotkey_outcome(&nav, &live, &hotkeys, outcome);

            if outcome.consumes_event() {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    });
    window.add_controller(controller);
}

/// How long the "no scene in that slot" notice replaces the shortcut caption.
const HOTKEY_MISS_NOTICE: Duration = Duration::from_secs(2);

fn apply_hotkey_outcome(
    nav: &NavigationContext,
    live: &Rc<LivePageHandle>,
    hotkeys: &SceneHotkeyConfig,
    outcome: HotkeyOutcome,
) {
    use crate::ui::pages::live::{scene_for_slot, set_hotkey_hint};

    match outcome {
        HotkeyOutcome::Ignored => {}
        HotkeyOutcome::LeaderArmed => set_hotkey_hint(
            live,
            Some(&fl!(LANGUAGE_LOADER, "hotkey-hint-leader-armed")),
        ),
        HotkeyOutcome::LeaderCancelled => {
            set_hotkey_hint(live, hotkeys.hint_label().as_deref());
        }
        HotkeyOutcome::Activate(slot) => {
            let scene = {
                let state = nav.state.borrow();
                scene_for_slot(&state.scene_inventory, &state.registry, slot)
            };
            match scene {
                Some(scene_id) => {
                    tracing::debug!(slot, scene = %scene_id, "scene hotkey switch");
                    set_hotkey_hint(live, hotkeys.hint_label().as_deref());
                    nav.dispatch(AppCommand::SwitchPrimaryScene(scene_id));
                }
                // An unbound digit is a miss, not an error: say so in place of
                // the caption rather than interrupting with a toast, then put
                // the caption back.
                None => {
                    let notice = fl!(
                        LANGUAGE_LOADER,
                        "hotkey-hint-empty-slot",
                        slot = (slot + 1).to_string()
                    );
                    set_hotkey_hint(live, Some(&notice));
                    glib::timeout_add_local_once(HOTKEY_MISS_NOTICE, {
                        let live = live.clone();
                        let nav = nav.clone();
                        move || {
                            // Only restore what this notice replaced; a later
                            // press may have moved on to something else.
                            if live.hotkey_hint.text() != notice {
                                return;
                            }
                            let hotkeys = nav.state.borrow().config.hotkeys;
                            set_hotkey_hint(&live, hotkeys.hint_label().as_deref());
                        }
                    });
                }
            }
        }
    }
}

/// Whether the configured binding requires at least one modifier to be held.
fn binds_with_modifiers(hotkeys: &SceneHotkeyConfig) -> bool {
    hotkeys
        .style
        .modifiers()
        .is_some_and(|modifiers| !modifiers.is_empty())
}

fn text_input_has_focus(window: &adw::ApplicationWindow) -> bool {
    gtk4::prelude::RootExt::focus(window).is_some_and(|widget| {
        widget.is::<gtk4::Editable>()
            || widget.is::<gtk4::TextView>()
            || widget.is::<gtk4::SearchEntry>()
    })
}

fn modifiers_from_state(state: gdk::ModifierType) -> Modifiers {
    Modifiers::new(
        state.contains(gdk::ModifierType::CONTROL_MASK),
        state.contains(gdk::ModifierType::ALT_MASK),
        state.contains(gdk::ModifierType::SHIFT_MASK),
        state.contains(gdk::ModifierType::SUPER_MASK),
    )
}

/// Classify a key press, leader candidates first so a layout that hides a digit
/// behind one of them cannot steal the leader.
fn key_symbol(window: &adw::ApplicationWindow, keyval: gdk::Key, keycode: u32) -> KeySymbol {
    match keyval {
        gdk::Key::space | gdk::Key::KP_Space => KeySymbol::Space,
        gdk::Key::comma | gdk::Key::KP_Separator => KeySymbol::Comma,
        gdk::Key::semicolon => KeySymbol::Semicolon,
        gdk::Key::backslash => KeySymbol::Backslash,
        gdk::Key::grave | gdk::Key::dead_grave => KeySymbol::Grave,
        gdk::Key::Escape => KeySymbol::Escape,
        gdk::Key::Shift_L
        | gdk::Key::Shift_R
        | gdk::Key::Control_L
        | gdk::Key::Control_R
        | gdk::Key::Alt_L
        | gdk::Key::Alt_R
        | gdk::Key::Meta_L
        | gdk::Key::Meta_R
        | gdk::Key::Super_L
        | gdk::Key::Super_R
        | gdk::Key::Hyper_L
        | gdk::Key::Hyper_R
        | gdk::Key::Caps_Lock
        | gdk::Key::Shift_Lock
        | gdk::Key::Num_Lock
        | gdk::Key::ISO_Level3_Shift => KeySymbol::Modifier,
        _ => match digit_for_keyval(keyval).or_else(|| digit_for_keycode(window, keycode)) {
            Some(digit) => KeySymbol::Digit(digit),
            None => KeySymbol::Other,
        },
    }
}

fn digit_for_keyval(keyval: gdk::Key) -> Option<u8> {
    match keyval {
        gdk::Key::KP_0 => return Some(0),
        gdk::Key::KP_1 => return Some(1),
        gdk::Key::KP_2 => return Some(2),
        gdk::Key::KP_3 => return Some(3),
        gdk::Key::KP_4 => return Some(4),
        gdk::Key::KP_5 => return Some(5),
        gdk::Key::KP_6 => return Some(6),
        gdk::Key::KP_7 => return Some(7),
        gdk::Key::KP_8 => return Some(8),
        gdk::Key::KP_9 => return Some(9),
        _ => {}
    }
    keyval
        .to_unicode()
        .and_then(|c| c.to_digit(10))
        .map(|digit| digit as u8)
}

/// Digit printed on the physical key, whatever level it sits at.
///
/// `Shift+1` arrives as `!` on a US layout, and on layouts such as AZERTY the
/// digits themselves need Shift. Asking the display what the keycode can
/// produce keeps both working, so the number row means what it is labelled.
fn digit_for_keycode(window: &adw::ApplicationWindow, keycode: u32) -> Option<u8> {
    let display = WidgetExt::display(window);
    display
        .map_keycode(keycode)?
        .into_iter()
        .find_map(|(_, keyval)| digit_for_keyval(keyval))
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
            let obs_status = ObsStatus::Connected {
                obs_version: info.obs_version,
            };
            nav.state.borrow_mut().set_obs_status(obs_status.clone());
            apply_sidebar_connection(sidebar_controls, &obs_status);
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
            // are actually on screen.
            refreshers.call_if_visible(nav, Page::Stats);
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

    apply_sidebar_connection(&ui.sidebar_controls, status);
    sync_output_indicators(nav, &ui.sidebar_controls, &ui.streaming_chrome);

    status_bar::set_connection(&ui.status_bar, status);
    status_bar::clear_stats(&ui.status_bar);

    show_disconnected_view(&ui.live, live_hint);
    ui.live
        .current_scene_label
        .set_text(&fl!(LANGUAGE_LOADER, "window-current-scene-none"));
    rebuild_audio_cards(&ui.live, &[], nav);

    // The header lists belong to the session that just ended.
    update_named_selector(&ui.header_selectors.profiles, &ObsNamedList::default());
    update_named_selector(
        &ui.header_selectors.scene_collections,
        &ObsNamedList::default(),
    );
}

/// Which of the two OBS outputs an event is about.
///
/// Stream and record are handled identically apart from the state fields they
/// read, the status-bar slot they write, and the word in their label, so the
/// difference is carried as a value instead of being copied into eight
/// near-identical match arms.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OutputKind {
    Stream,
    Record,
}

impl OutputKind {
    fn label(self) -> String {
        match self {
            Self::Stream => fl!(LANGUAGE_LOADER, "window-output-kind-stream"),
            Self::Record => fl!(LANGUAGE_LOADER, "window-output-kind-record"),
        }
    }
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

    let label = output_label(&kind.label(), &status, elapsed.as_deref());
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

fn set_status_class(label: &gtk4::Label, new_class: &str) {
    for class in status_bar::CONNECTION_CSS_CLASSES {
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
struct SidebarButtonModel {
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
) -> SidebarButtonModel {
    if status.state.is_transitioning() {
        return SidebarButtonModel {
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
        SidebarButtonModel {
            label: stop_label,
            sensitive: connected,
            suggested: false,
            destructive: connected,
        }
    } else {
        SidebarButtonModel {
            label: start_label,
            sensitive: connected,
            suggested: connected,
            destructive: false,
        }
    }
}

/// How the sidebar's status line and Connect button should look for a
/// connection state.
struct SidebarConnectionModel {
    status_text: String,
    css_class: &'static str,
    button: SidebarButtonModel,
}

/// Derive the sidebar's connection chrome from the connection status.
///
/// All four states — connecting, connected, disconnected, failed — used to
/// spell this out by hand at their own call site, which is how the Error arm
/// ended up styling its button like the Disconnected arm but writing its own
/// status text. The CSS class comes from `ObsStatus::css_class` rather than
/// being written out again here, so the sidebar and the status bar cannot
/// disagree about what colour a state is.
fn sidebar_connection_model(status: &ObsStatus) -> SidebarConnectionModel {
    let (status_text, label, sensitive, suggested, destructive) = match status {
        ObsStatus::Connecting => (
            fl!(LANGUAGE_LOADER, "window-status-connecting"),
            fl!(LANGUAGE_LOADER, "window-connect-btn-connecting"),
            false,
            false,
            false,
        ),
        ObsStatus::Connected { obs_version } => (
            fl!(
                LANGUAGE_LOADER,
                "window-status-connected",
                version = obs_version.clone()
            ),
            fl!(LANGUAGE_LOADER, "window-connect-btn-disconnect"),
            true,
            false,
            true,
        ),
        ObsStatus::Disconnected => (
            fl!(LANGUAGE_LOADER, "window-status-disconnected"),
            fl!(LANGUAGE_LOADER, "window-connect-btn-connect"),
            true,
            true,
            false,
        ),
        ObsStatus::Error(error) => (
            fl!(
                LANGUAGE_LOADER,
                "window-status-error",
                error = error.clone()
            ),
            fl!(LANGUAGE_LOADER, "window-connect-btn-retry"),
            true,
            true,
            false,
        ),
    };

    SidebarConnectionModel {
        status_text,
        css_class: status.css_class(),
        button: SidebarButtonModel {
            label,
            sensitive,
            suggested,
            destructive,
        },
    }
}

fn apply_sidebar_connection(sidebar: &SidebarControls, status: &ObsStatus) {
    let model = sidebar_connection_model(status);
    sidebar.status_label.set_text(&model.status_text);
    set_status_class(&sidebar.status_label, model.css_class);
    apply_sidebar_button(&sidebar.connect_btn, model.button);
}

fn apply_sidebar_button(button: &Button, model: SidebarButtonModel) {
    set_button_label(button, &model.label);
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
    nav: &NavigationContext,
    sidebar: &SidebarControls,
    streaming_chrome: &StreamingChromeRef,
) {
    let streaming = {
        let state = nav.state.borrow();
        sync_sidebar_output_buttons(sidebar, &state);
        state.stream_status.active
    };

    sync_streaming_chrome(streaming_chrome, streaming);
}

fn sync_sidebar_output_buttons(sidebar: &SidebarControls, state: &AppState) {
    let connected = matches!(state.obs_status, ObsStatus::Connected { .. });
    apply_sidebar_button(
        &sidebar.stream_btn,
        sidebar_output_button_model(
            &state.stream_status,
            connected,
            fl!(LANGUAGE_LOADER, "window-sidebar-start-stream"),
            fl!(LANGUAGE_LOADER, "window-sidebar-stop-stream"),
        ),
    );
    apply_sidebar_button(
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
    profiles.connect_selection(nav, AppCommand::SetCurrentProfile);

    let scene_collections = build_named_selector(
        &fl!(LANGUAGE_LOADER, "window-selector-collection-label"),
        &fl!(LANGUAGE_LOADER, "window-selector-collection-tooltip"),
    );
    scene_collections.connect_selection(nav, AppCommand::SetCurrentSceneCollection);

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

/// Icon on the sidebar's stream button.
const SIDEBAR_STREAM_ICON: &str = "nf-md-broadcast-symbolic";
/// Icon on the sidebar's record button.
const SIDEBAR_RECORD_ICON: &str = "nf-md-record-circle-symbolic";
/// Icon on the sidebar's connect button.
const SIDEBAR_CONNECT_ICON: &str = "nf-md-lan-connect-symbolic";

/// Icon-and-label content for a wide sidebar button.
///
/// `Button::set_label` would replace this with plain text, so the buttons whose
/// text changes with state go through `set_button_label` instead.
fn button_content(icon_name: &str, label: &str) -> GtkBox {
    let content = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .build();
    content.append(&Image::from_icon_name(icon_name));
    let label = Label::new(Some(label));
    label.add_css_class("scenedeck-button-label");
    content.append(&label);
    content
}

/// Update the text of a button built by [`button_content`].
fn set_button_label(button: &Button, text: &str) {
    let Some(label) = button
        .child()
        .and_then(|content| content.last_child())
        .and_then(|child| child.downcast::<Label>().ok())
    else {
        button.set_label(text);
        return;
    };
    label.set_text(text);
}

/// App logo and name for the top of the sidebar.
///
/// The sidebar header was empty, which left the window with nothing naming it
/// once the title bar is merged into the content header.
fn build_brand() -> GtkBox {
    let brand = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .valign(gtk4::Align::Center)
        .build();
    brand.add_css_class("scenedeck-brand");

    let logo = Image::from_icon_name(APP_ID);
    logo.set_pixel_size(22);
    logo.add_css_class("scenedeck-brand-logo");

    let name = Label::builder().label(APP_NAME).xalign(0.0).build();
    name.add_css_class("scenedeck-brand-name");

    brand.append(&logo);
    brand.append(&name);
    brand
}

/// Build one of the three full-width buttons at the foot of the sidebar.
fn sidebar_button(
    icon_name: &str,
    label: &str,
    css_class: &str,
    sensitive: bool,
    on_click: impl Fn(&Button) + 'static,
) -> Button {
    let button = Button::builder()
        .halign(gtk4::Align::Fill)
        .hexpand(true)
        .sensitive(sensitive)
        .build();
    button.set_child(Some(&button_content(icon_name, label)));
    button.add_css_class(css_class);
    button.connect_clicked(move |button| on_click(button));
    button
}

fn build_sidebar(nav: &NavigationContext) -> (adw::NavigationPage, ListBox, SidebarControls) {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .vexpand(true)
        .build();
    list.add_css_class("navigation-sidebar");
    list.add_css_class("scenedeck-sidebar-list");

    // Built up front rather than captured out of the loop: output events tint
    // this icon while a stream is running, so the sidebar has to keep a handle
    // on it either way, and building it here means there is no "what if the
    // loop never saw the Live row" case to panic over.
    let live_icon = Image::from_icon_name(Page::Live.icon_name());
    live_icon.add_css_class("scenedeck-sidebar-live-icon");

    for page in NAV_PAGES {
        let icon = if page == Page::Live {
            live_icon.clone()
        } else {
            Image::from_icon_name(page.icon_name())
        };
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

    // Connect starts enabled; the two output buttons wait until there is a
    // connection to act on.
    let connect_btn = sidebar_button(
        SIDEBAR_CONNECT_ICON,
        &fl!(LANGUAGE_LOADER, "window-connect-btn-connect"),
        "suggested-action",
        true,
        {
            let nav = nav.clone();
            move |_: &Button| {
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
        },
    );

    let stream_btn = sidebar_button(
        SIDEBAR_STREAM_ICON,
        &fl!(LANGUAGE_LOADER, "window-sidebar-start-stream"),
        "sidebar-output-button",
        false,
        {
            let nav = nav.clone();
            move |button: &Button| crate::ui::pages::live::handle_stream_output_toggle(button, &nav)
        },
    );

    let record_btn = sidebar_button(
        SIDEBAR_RECORD_ICON,
        &fl!(LANGUAGE_LOADER, "window-sidebar-start-recording"),
        "sidebar-output-button",
        false,
        {
            let nav = nav.clone();
            move |button: &Button| crate::ui::pages::live::handle_record_output_toggle(button, &nav)
        },
    );

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
    sidebar_header.pack_start(&build_brand());
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
            live_icon,
            stream_btn,
            record_btn,
            connect_btn,
        },
    )
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

impl NamedSelector {
    /// Dispatch `to_command` whenever the user picks a different entry.
    ///
    /// The `updating` flag guards against an echo. `update_named_selector`
    /// rewrites the model when OBS reports a new profile or scene collection,
    /// and restoring the selection afterwards makes GTK emit the same signal a
    /// real click does. Without the guard, an update that came *from* OBS would
    /// be sent straight back to it as a command.
    ///
    /// `to_command` is a plain function pointer rather than a closure type, so
    /// the bare variant constructors can be passed directly and the body is
    /// compiled once instead of once per call site.
    fn connect_selection(&self, nav: &NavigationContext, to_command: fn(String) -> AppCommand) {
        let nav = nav.clone();
        let model = self.model.clone();
        let updating = self.updating.clone();
        self.dropdown.connect_selected_notify(move |dropdown| {
            if updating.get() {
                return;
            }
            if let Some(name) = model.string(dropdown.selected()) {
                nav.dispatch(to_command(name.to_string()));
            }
        });
    }
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
    stats: RefreshFn,
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

    fn output_status(active: bool, state: OutputRunState) -> OutputStatus {
        OutputStatus {
            active,
            state,
            detail: None,
        }
    }

    #[test]
    fn sidebar_connection_model_covers_every_connection_state() {
        let connecting = sidebar_connection_model(&ObsStatus::Connecting);
        assert_eq!(connecting.css_class, "obs-connecting");
        assert!(!connecting.button.sensitive);
        // Nothing is in progress that the user could confirm or cancel, so the
        // button carries neither accent while it waits.
        assert!(!connecting.button.suggested);
        assert!(!connecting.button.destructive);

        let connected = sidebar_connection_model(&ObsStatus::Connected {
            obs_version: "30.1.2".to_string(),
        });
        assert_eq!(connected.css_class, "obs-connected");
        assert!(connected.button.sensitive);
        // Connected means the button now disconnects, which is destructive.
        assert!(connected.button.destructive);
        assert!(!connected.button.suggested);
        assert!(connected.status_text.contains("30.1.2"));

        let disconnected = sidebar_connection_model(&ObsStatus::Disconnected);
        assert_eq!(disconnected.css_class, "obs-disconnected");
        assert!(disconnected.button.sensitive);
        assert!(disconnected.button.suggested);
        assert!(!disconnected.button.destructive);

        let failed = sidebar_connection_model(&ObsStatus::Error("refused".to_string()));
        assert_eq!(failed.css_class, "obs-error");
        assert!(failed.button.sensitive);
        // A failure offers a retry, which is the same invitation as connecting.
        assert!(failed.button.suggested);
        assert!(!failed.button.destructive);
        assert!(failed.status_text.contains("refused"));
    }

    #[test]
    fn every_connection_css_class_is_one_the_widgets_clear() {
        // `set_status_class` clears this list before adding the current class;
        // a class missing from it would never be removed again.
        for status in [
            ObsStatus::Connecting,
            ObsStatus::Connected {
                obs_version: String::new(),
            },
            ObsStatus::Disconnected,
            ObsStatus::Error(String::new()),
        ] {
            assert!(
                status_bar::CONNECTION_CSS_CLASSES.contains(&status.css_class()),
                "{:?} uses a class the widgets never clear",
                status
            );
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
            SidebarButtonModel {
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
            SidebarButtonModel {
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
            SidebarButtonModel {
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
            SidebarButtonModel {
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
