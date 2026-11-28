//! Main application window.
//!
//! Builds the `adw::NavigationSplitView` shell, wires the GTK→Controller
//! command path via `NavigationContext`, and applies Controller→GTK events
//! that `event_pump.rs` drains.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

type RefreshFn = Rc<dyn Fn()>;
type StreamingChromeRef = Rc<RefCell<Option<StreamingChrome>>>;

use adw::prelude::*;
use gtk4::{Button, Image, Stack, StackTransitionType};
use i18n_embed_fl::fl;

use crate::app_info::APP_NAME;
use crate::controller::app_controller::AppController;
use crate::controller::command::AppCommand;
use crate::controller::event::{AppEvent, FailedOperation};
use crate::controller::state::{AppState, MixerAudioRefreshTransition, ObsStatus, Page};
use crate::domain::appearance::ThemeMode;
use crate::domain::output::{OutputKind, OutputStatus};
use crate::infra::error::AppError;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::markup::plain_toast;
use crate::ui::navigation::NavigationContext;
use crate::ui::pages::live::LivePageHandle;
use crate::ui::pages::stats::PopoutSlot as StatsPopoutSlot;
use crate::ui::presenters::output_status::output_status_line;
use crate::ui::register_resources;
use crate::ui::sidebar::{self, HeaderSelectors, SidebarControls, NAV_PAGES};
use crate::ui::theme::ThemeManager;
use crate::ui::widgets::status_bar::{self, StatusBarHandle};

const DEFAULT_WIDTH: i32 = 1100;
const DEFAULT_HEIGHT: i32 = 740;

/// Sources may retain page handles while running, but never outlive the shell.
/// Destroying a Source is idempotent even if its callback already returned Break.
#[derive(Default)]
struct WindowSources(Vec<glib::Source>);

impl WindowSources {
    fn track(&mut self, id: glib::SourceId) {
        if let Some(source) = glib::MainContext::default().find_source_by_id(&id) {
            self.0.push(source);
        }
    }
}

impl Drop for WindowSources {
    fn drop(&mut self) {
        for source in &self.0 {
            source.destroy();
        }
    }
}

pub fn build_main_window(
    app: &adw::Application,
    state: Rc<RefCell<AppState>>,
    controller: Rc<RefCell<AppController>>,
    event_rx: mpsc::Receiver<AppEvent>,
) -> adw::ApplicationWindow {
    super::report_history::register(&state);
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
    let mut sources = WindowSources::default();

    // ── Content stack ─────────────────────────────────────────────────────────
    let content_stack = Stack::builder()
        .vexpand(true)
        .hexpand(true)
        .hhomogeneous(false)
        .vhomogeneous(false)
        .transition_type(StackTransitionType::Crossfade)
        .build();
    content_stack.add_css_class("scenedeck-content-stack");

    let credentials = controller.borrow().credentials();
    let nav = NavigationContext::new(state.clone(), content_stack.clone(), controller);
    let (live_handle, refreshers) = add_pages(&content_stack, &nav, &mut sources, credentials);

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
    let event_ui = Rc::new(RefCell::new(Some(EventUiContext {
        live: live_handle.clone(),
        toast: toast_overlay.clone(),
        refreshers: refreshers.clone(),
        header_selectors: header_selectors.clone(),
        sidebar_controls: sidebar_controls.clone(),
        streaming_chrome: streaming_chrome.clone(),
        status_bar: status_bar.clone(),
    })));

    super::previews::install(live_handle.clone(), nav.clone());
    install_event_polling(app, &nav, event_rx, &event_ui, &refreshers.stats_popout);
    sources.track(install_output_clock(&state, &status_bar));

    // OBS performance stats are not polled from here: the session task owns a
    // poll loop that runs for as long as the connection is up, so the status
    // bar and Stats history are reconciled by the application pump regardless
    // of the open page, including when only a detached Stats window survives.

    let content_header = build_content_header(&window, &nav, &header_selectors, &streaming_chrome);

    let content_toolbar = adw::ToolbarView::new();
    content_toolbar.add_css_class("scenedeck-content-toolbar");
    content_toolbar.add_top_bar(&content_header);
    let (health_alerts, health_source) = super::health_alerts::build(nav.clone());
    content_toolbar.add_top_bar(&health_alerts);
    sources.track(health_source);
    content_toolbar.set_content(Some(&content_stack));

    content_toolbar.add_bottom_bar(&crate::ui::widgets::transport_bar::build(nav.clone()));

    let content_page = adw::NavigationPage::builder()
        .title(APP_NAME)
        .child(&content_toolbar)
        .build();

    // ── Navigation split view ─────────────────────────────────────────────────
    let split = adw::NavigationSplitView::new();
    split.add_css_class("scenedeck-split");
    split.set_sidebar(Some(&sidebar_page));
    split.set_content(Some(&content_page));
    let breakpoint = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("max-width: 850sp").expect("static breakpoint"),
    );
    breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
    breakpoint.add_setter(&content_header, "show-title", Some(&false.to_value()));
    window.add_breakpoint(breakpoint);
    let sidebar_toggle = Button::from_icon_name("sidebar-show-symbolic");
    sidebar_toggle.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "window-open-navigation")));
    sidebar_toggle.set_visible(false);
    content_header.set_show_back_button(false);
    content_header.pack_start(&sidebar_toggle);
    split.connect_collapsed_notify({
        let button = sidebar_toggle.downgrade();

        move |split| {
            if let Some(button) = button.upgrade() {
                button.set_visible(split.is_collapsed());
            }
        }
    });
    let split_copy = split.downgrade();
    sidebar_toggle.connect_clicked(move |_| {
        if let Some(split) = split_copy.upgrade() {
            split.set_show_content(false);
        }
    });
    sidebar_list.connect_row_selected({
        let split = split.downgrade();
        move |_, row| {
            if row.is_some() {
                if let Some(split) = split.upgrade() {
                    split.set_show_content(true);
                }
            }
        }
    });
    sidebar_list.connect_row_activated({
        let split = split.downgrade();
        move |_, _| {
            if let Some(split) = split.upgrade() {
                split.set_show_content(true);
            }
        }
    });
    split.set_show_content(true);

    toast_overlay.add_css_class("scenedeck-toast-overlay");
    toast_overlay.set_child(Some(&split));

    let outer_toolbar = adw::ToolbarView::new();
    outer_toolbar.add_css_class("scenedeck-outer-toolbar");
    outer_toolbar.set_content(Some(&toast_overlay));
    outer_toolbar.add_bottom_bar(&status_bar.root);
    window.set_content(Some(&outer_toolbar));

    window.connect_hide({
        let shortcuts = nav.global_shortcuts.clone();
        move |_| shortcuts.borrow_mut().stop()
    });
    super::hotkeys::install(&window, &nav, live_handle.clone(), &toast_overlay);

    super::actions::install(app, &window, nav.clone());
    let sources = Rc::new(RefCell::new(Some(sources)));
    app.connect_window_removed({
        let window = window.downgrade();
        let sources = sources.clone();
        let event_ui = event_ui.clone();
        move |app, removed| {
            if window
                .upgrade()
                .is_some_and(|window| window.upcast_ref::<gtk4::Window>() == removed)
            {
                // GtkWidget::destroy is emitted only at finalization, too late
                // to break application-action retention of the window itself.
                sources.borrow_mut().take();
                event_ui.borrow_mut().take();
                for action in ["about", "reconnect", "settings", "help", "command-palette"] {
                    app.remove_action(action);
                }
            }
        }
    });
    window.connect_destroy({
        let sources = sources.clone();
        let event_ui = event_ui.clone();
        move |_| {
            // Also cover finalization without normal application removal.
            sources.borrow_mut().take();
            event_ui.borrow_mut().take();
        }
    });

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
    sources: &mut WindowSources,
    credentials: Arc<dyn crate::domain::connection::credential::CredentialStore>,
) -> (Rc<LivePageHandle>, PageRefreshers) {
    let live_handle = Rc::new(crate::ui::pages::live::build(nav.clone()));
    let (stats_widget, stats_refresh, stats_popout) = crate::ui::pages::stats::build(nav.clone());
    let (mixer_widget, mixer_refresh, mixer_handle) = crate::ui::pages::mixer::build(nav.clone());
    let (graph_widget, graph_refresh) = crate::ui::pages::graph::build(nav.clone());
    let (inventory_widget, inventory_refresh) = crate::ui::pages::inventory::build(nav.clone());
    let (doctor_widget, doctor_refresh) = crate::ui::pages::doctor::build(nav.clone());
    let (settings_widget, settings_refresh) = crate::ui::pages::settings::build(nav, credentials);
    let (help_widget, help_refresh) = crate::ui::pages::help::build(nav.clone());
    let (sources_widget, sources_refresh) = crate::ui::pages::sources::build(nav.clone());
    let weak_live = live_handle.root.downgrade();
    let weak_sources = sources_widget.downgrade();
    let last_revision = Rc::new(std::cell::Cell::new(u64::MAX));
    let last_registry_generation = std::cell::Cell::new((u64::MAX, u64::MAX));
    let registry_refreshers = [
        (Page::Inventory, inventory_refresh.clone()),
        (Page::Doctor, doctor_refresh.clone()),
        (Page::Graph, graph_refresh.clone()),
        (Page::Mixer, mixer_refresh.clone()),
    ];
    sources.track(glib::timeout_add_local(Duration::from_millis(200), {
        let nav = nav.clone();
        let live = live_handle.clone();
        let sources_refresh = sources_refresh.clone();
        move || {
            let Some(root) = weak_live.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let sources_visible = weak_sources.upgrade().is_some_and(|w| w.is_mapped());
            let generation = {
                let state = nav.state.borrow();
                (state.show_registry.generation, state.show_registry.revision)
            };
            if last_registry_generation.replace(generation) != generation {
                let inventory = nav.state.borrow().scene_inventory.clone();
                crate::ui::pages::live::rebuild_scene_cards(&live, &inventory, &nav);
                let current = nav.state.borrow().current_page;
                for (page, refresh) in &registry_refreshers {
                    if *page == current {
                        refresh();
                    }
                }
            }
            let (revision, refresh) = {
                let mut state = nav.state.borrow_mut();
                if state
                    .workspace
                    .output_results
                    .expire(crate::controller::workspace::unix_ms())
                {
                    state.workspace.touch();
                }
                (
                    state.workspace.revision(),
                    (root.is_mapped()
                        || sources_visible
                        || !state.workspace.operator_inspections.is_empty()
                        || (state
                            .registry
                            .action_bindings
                            .iter()
                            .any(|binding| binding.enabled)
                            && root
                                .root()
                                .and_downcast::<gtk4::Window>()
                                .is_some_and(|window| window.is_active())))
                        && (state.workspace.dirty
                            || state
                                .workspace
                                .observed_at
                                .is_none_or(|t| t.elapsed() >= Duration::from_secs(10)))
                        && !state.workspace.loading
                        && !state.workspace.pending
                        && matches!(state.obs_status, ObsStatus::Connected { .. }),
                )
            };
            if revision != last_revision.get() {
                last_revision.set(revision);
                if sources_visible {
                    sources_refresh();
                }
                let inventory = nav.state.borrow().scene_inventory.clone();
                crate::ui::pages::live::rebuild_scene_cards(&live, &inventory, &nav);
            }
            if refresh {
                super::workspace::refresh(&nav);
            }
            glib::ControlFlow::Continue
        }
    }));

    for (page, widget) in [
        (
            Page::Live,
            live_handle.root.clone().upcast::<gtk4::Widget>(),
        ),
        (Page::Sources, sources_widget),
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
        mixer_handle,
        graph: graph_refresh,
        inventory: inventory_refresh,
        doctor: doctor_refresh,
        settings: settings_refresh,
        help: help_refresh,
        sources: sources_refresh,
    };

    (live_handle, refreshers)
}

/// Route controller events to the widgets via `event_pump` (50 ms ticks,
/// at most 128 events per tick so a burst cannot stall the main loop).
fn install_event_polling(
    app: &adw::Application,
    nav: &NavigationContext,
    event_rx: mpsc::Receiver<AppEvent>,
    event_ui: &Rc<RefCell<Option<EventUiContext>>>,
    stats_popout: &StatsPopoutSlot,
) {
    let nav = nav.clone(); // Navigation holds only weak stack/sidebar references.
    let event_ui = Rc::downgrade(event_ui);
    let stats_popout = stats_popout.clone();
    let health_tick = super::health_alerts::monitor(nav.clone());
    let report_nav = nav.clone();
    super::event_pump::install(
        app,
        event_rx,
        move |event| {
            nav.state
                .borrow_mut()
                .collect_report(Instant::now(), crate::controller::workspace::unix_ms());
            let shell = event_ui.upgrade().and_then(|slot| slot.borrow().clone());
            let before = {
                let state = nav.state.borrow();
                (
                    state.stats_history.latest().copied(),
                    state.workspace.revision(),
                )
            };
            apply_event(&nav, event, shell.as_ref());
            nav.state
                .borrow_mut()
                .collect_report(Instant::now(), crate::controller::workspace::unix_ms());
            let changed = {
                let state = nav.state.borrow();
                before
                    != (
                        state.stats_history.latest().copied(),
                        state.workspace.revision(),
                    )
            };
            if shell.is_none() && changed {
                // The surviving pop-out is independent of all shell page handles.
                // Meter-only traffic must not redraw one-second telemetry charts.
                let refresh = stats_popout.borrow().clone();
                if let Some(refresh) = refresh {
                    refresh();
                }
            }
        },
        move || {
            health_tick();
            report_nav
                .state
                .borrow_mut()
                .collect_report(Instant::now(), crate::controller::workspace::unix_ms());
            super::report_history::tick(&report_nav.state);
        },
    );
}

/// Re-render the stream and record status lines once a second.
///
/// OBS does not push anything when an output is simply still running, but the
/// elapsed time in those lines has to keep counting, so this ticks on its own.
fn install_output_clock(
    state: &Rc<RefCell<AppState>>,
    status_bar: &StatusBarHandle,
) -> glib::SourceId {
    glib::timeout_add_local(Duration::from_secs(1), {
        let state = state.clone();
        let status_bar = status_bar.clone();
        move || {
            render_output_slot(&state, &status_bar, OutputKind::Stream);
            render_output_slot(&state, &status_bar, OutputKind::Record);
            glib::ControlFlow::Continue
        }
    })
}

/// Build the header bar above the content area.
///
/// Also publishes the header and its "live" icon into `streaming_chrome`, so
/// output events can tint the bar while a stream is running.
fn build_content_header(
    window: &adw::ApplicationWindow,
    nav: &NavigationContext,
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
            let window = window.downgrade();
            move || {
                if let Some(window) = window.upgrade() {
                    super::actions::show_about(&window);
                }
            }
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
            move || {
                // Readbacks update the relevant widgets; do not redraw old data first.
                nav.dispatch(AppCommand::RefreshData);
            }
        },
    );
    content_header.pack_start(&stream_live_icon);
    content_header.pack_start(&refresh_btn);
    header_selectors.pack_into(&content_header);
    content_header.pack_end(&crate::ui::widgets::studio_controls::toggle(nav.clone()));
    let sources = header_button(
        "view-list-symbolic",
        fl!(LANGUAGE_LOADER, "sources-title"),
        {
            let nav = nav.clone();
            move || nav.switch_to_page(Page::Sources)
        },
    );
    content_header.pack_end(&sources);
    let palette = Button::from_icon_name("system-search-symbolic");
    palette.set_action_name(Some("app.command-palette"));
    palette.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "palette-title")));
    palette.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "palette-title"
    ))]);
    content_header.pack_end(&palette);

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

fn refresh_scene_inventory_views(nav: &NavigationContext, ui: Option<&EventUiContext>) {
    use crate::ui::pages::live::{rebuild_audio_cards, rebuild_scene_cards};
    if let Some(ui) = ui {
        let inventory = nav.state.borrow().scene_inventory.clone();
        rebuild_scene_cards(&ui.live, &inventory, nav);
        let inputs = nav.state.borrow().audio_inputs.clone();
        rebuild_audio_cards(&ui.live, &inputs, nav);
        ui.refreshers.call_visible_among(
            nav,
            &[Page::Mixer, Page::Inventory, Page::Doctor, Page::Graph],
        );
    }
}

/// Apply one `AppEvent` to the widgets.
///
/// The match is exhaustive on purpose. Connection and output events are
/// forwarded to their own handlers because those two families are long enough
/// to read badly inline, but the routing happens *inside* the match rather than
/// in a pre-filter above it, so adding a variant to `AppEvent` is a compile
/// error here instead of a panic when that event first arrives at runtime.
fn apply_event(nav: &NavigationContext, event: AppEvent, ui: Option<&EventUiContext>) {
    use crate::ui::pages::live::{rebuild_audio_cards, rebuild_scene_cards, show_live_view};

    match event {
        AppEvent::SessionRetired(terminal) => {
            let current = nav.accepts_session_event(terminal.generation, None);
            if current {
                let message = crate::infra::error::AppError::Connection(terminal.reason.clone())
                    .user_message();
                let status = ObsStatus::Error(message.clone());
                show_no_session_ui(
                    nav,
                    ui,
                    &status,
                    &fl!(LANGUAGE_LOADER, "window-obs-connection-failed"),
                );
                if let Some(ui) = ui {
                    ui.toast.add_toast(plain_toast(&message));
                }
            }
            nav.state
                .borrow_mut()
                .apply_terminal_evidence(terminal, current);
            if current {
                if let Some(ui) = ui {
                    refresh_output_status_bar(nav, ui, OutputKind::Stream);
                    refresh_output_status_bar(nav, ui, OutputKind::Record);
                }
            }
        }
        AppEvent::SessionEvent {
            generation,
            collection,
            event,
        } => {
            let accepted = nav.accepts_session_event(generation, collection)
                && nav
                    .state
                    .borrow()
                    .accepts_session_publication(generation, &event);
            if accepted {
                let connected = matches!(*event, AppEvent::Connected(_));
                apply_event(nav, *event, ui);
                if connected {
                    nav.state.borrow_mut().reports.bind_generation(generation);
                }
            }
        }
        AppEvent::Observation { revision, event } => {
            let event = nav
                .state
                .borrow_mut()
                .reconcile_observation(revision, *event);
            if let Some(event) = event {
                apply_event(nav, event, ui);
            }
        }
        AppEvent::ActiveAudioLoading { scene } => {
            if nav.state.borrow().scene_inventory.current_id != scene {
                return;
            }
            nav.state.borrow_mut().invalidate_active_audio();
            if let Some(ui) = ui {
                rebuild_audio_cards(&ui.live, &[], nav);
                ui.refreshers.call_if_visible(nav, Page::Mixer);
            }
        }
        AppEvent::ActiveAudioObserved { scene, result } => {
            nav.state.borrow_mut().apply_active_audio(scene, result);
            if let Some(ui) = ui {
                let inputs = nav.state.borrow().audio_inputs.clone();
                rebuild_audio_cards(&ui.live, &inputs, nav);
                ui.refreshers.call_if_visible(nav, Page::Mixer);
            }
        }
        AppEvent::WorkspaceAcknowledged {
            epoch,
            sequence,
            result_id,
        } => {
            super::event_reconciliation::acknowledge(
                &mut nav.state.borrow_mut(),
                epoch,
                sequence,
                result_id,
            );
        }
        AppEvent::CommandFailed { operation, error } => {
            match operation {
                FailedOperation::ProfileReadback => nav.state.borrow_mut().profiles.current = None,
                FailedOperation::CollectionReadback => {
                    apply_event(nav, AppEvent::WorkspaceScopeChanging, ui);
                }
                FailedOperation::Command
                | FailedOperation::Profile
                | FailedOperation::Collection => {}
            }
            report_failure(nav, ui, command_failure_message(operation, &error));
        }
        AppEvent::EventsResynchronized => {
            report_failure(nav, ui, fl!(LANGUAGE_LOADER, "events-resync-warning"));
        }
        AppEvent::GraphInvalidated => {
            nav.state.borrow_mut().set_topology_loading();
            if let Some(ui) = ui {
                ui.refreshers
                    .call_visible_among(nav, &[Page::Graph, Page::Doctor]);
            }
        }
        AppEvent::GraphFailed(error) | AppEvent::GraphRefreshRejected(error) => {
            let message = error.user_message();
            nav.state.borrow_mut().set_topology_failed(message.clone());
            if let Some(ui) = ui {
                ui.toast.add_toast(plain_toast(&message));
            }
            if let Some(ui) = ui {
                ui.refreshers
                    .call_visible_among(nav, &[Page::Graph, Page::Doctor]);
            }
        }
        AppEvent::ThumbnailLoaded { request, result } => {
            if let Some(ui) = ui {
                super::previews::apply(&ui.live, nav, request, result);
            }
        }
        AppEvent::WorkspaceMediaLoaded {
            epoch,
            sequence,
            input,
            result,
        } => {
            super::event_reconciliation::media_loaded(
                &mut nav.state.borrow_mut(),
                epoch,
                sequence,
                input,
                result,
            );
        }
        AppEvent::WorkspaceLoaded {
            epoch,
            sequence,
            result_id,
            acknowledged,
            uncertain,
            result,
            action,
        } => {
            let accepted = super::event_reconciliation::complete_workspace(
                &mut nav.state.borrow_mut(),
                super::event_reconciliation::WorkspaceCompletion {
                    epoch,
                    sequence,
                    result_id,
                    acknowledged,
                    uncertain,
                    result,
                    action,
                },
            );
            if !accepted {
                return;
            }
            if let Some(ui) = ui {
                let inventory = nav.state.borrow().scene_inventory.clone();
                rebuild_scene_cards(&ui.live, &inventory, nav);
                refresh_output_status_bar(nav, ui, OutputKind::Record);
                refresh_output_status_bar(nav, ui, OutputKind::Stream);
                ui.refreshers.call_if_visible(nav, Page::Doctor);
            }
            super::workspace::dispatch_queued_toggle(nav);
        }
        AppEvent::WorkspaceInvalidated => {
            nav.state.borrow_mut().workspace.invalidate_controls();
        }
        AppEvent::WorkspaceValuesChanged => {
            let mut state = nav.state.borrow_mut();
            if state.workspace.can_control() {
                state.workspace.observe_global_controls();
            }
            state.workspace.mark_dirty();
        }
        AppEvent::StudioModeChanged(enabled) => {
            nav.state
                .borrow_mut()
                .workspace
                .observe_studio_mode(enabled, Instant::now());
        }
        AppEvent::VirtualCameraChanged(active) => {
            nav.state
                .borrow_mut()
                .workspace
                .observe_virtual_camera(active, Instant::now());
        }
        AppEvent::WorkspaceScopeChanging => {
            if let Some(ui) = ui {
                super::previews::clear(&ui.live, nav);
            }
            nav.dispatch(AppCommand::CancelWorkspace);
            let mut state = nav.state.borrow_mut();
            state.invalidate_collection_runtime();
            drop(state);
            let Some(ui) = ui else {
                return;
            };
            let collections = nav.state.borrow().scene_collections.clone();
            ui.header_selectors.update_scene_collections(&collections);
            rebuild_scene_cards(&ui.live, &Default::default(), nav);
            rebuild_audio_cards(&ui.live, &[], nav);
            ui.refreshers.call_visible_among(
                nav,
                &[
                    Page::Mixer,
                    Page::Inventory,
                    Page::Graph,
                    Page::Doctor,
                    Page::Sources,
                ],
            );
        }
        AppEvent::WorkspaceAudioFailed(error) => {
            if let Some(ui) = ui {
                ui.toast.add_toast(plain_toast(&error));
            }
            super::event_reconciliation::audio_failed(&mut nav.state.borrow_mut(), error);
        }
        AppEvent::PresetProgress {
            epoch,
            run,
            index,
            outcome,
        } => {
            nav.state
                .borrow_mut()
                .preset_progress(epoch, run, index, outcome);
        }
        AppEvent::PresetRejected { epoch, run } => {
            nav.state.borrow_mut().reject_preset(epoch, run);
        }
        AppEvent::ReplaySaved(path) => {
            super::event_reconciliation::replay_saved(&mut nav.state.borrow_mut(), path);
        }
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
            let obs_status = nav.state.borrow_mut().apply_connected(info);
            if let Some(ui) = ui {
                sync_output_indicators(nav, &ui.sidebar_controls, &ui.streaming_chrome);
                status_bar::set_connection(&ui.status_bar, &obs_status);
                show_live_view(&ui.live);
            }
        }

        AppEvent::OutputReadbackFailed {
            kind,
            error,
            observed_at,
        } => {
            let message = error.user_message();
            let changed = {
                let mut state = nav.state.borrow_mut();
                if !state.accept_output_observation(kind, observed_at) {
                    return;
                }
                let changed = match kind {
                    OutputKind::Stream => state.last_stream_command_error.as_deref(),
                    OutputKind::Record => state.last_record_command_error.as_deref(),
                } != Some(message.as_str());
                state.health.output_unobserved(kind);
                let unknown =
                    OutputStatus::new(false, crate::domain::output::OutputRunState::Unknown);
                match kind {
                    OutputKind::Stream => {
                        state.set_stream_status(unknown);
                        state.last_stream_command_error = Some(message.clone());
                    }
                    OutputKind::Record => {
                        state.set_record_status(unknown);
                        state.last_record_command_error = Some(message.clone());
                    }
                }
                if changed {
                    state.workspace.log(
                        crate::domain::workflow::ActivityOrigin::Failed,
                        message.clone(),
                    );
                }
                changed
            };
            if let Some(ui) = ui.filter(|_| changed) {
                ui.toast.add_toast(plain_toast(&message));
            }
            if let Some(ui) = ui {
                refresh_output_status_bar(nav, ui, kind);
            }
        }
        AppEvent::RecordingStopObserved { .. }
        | AppEvent::OutputStatusUpdated(..)
        | AppEvent::OutputCommandPending(..)
        | AppEvent::OutputCommandSucceeded(_)
        | AppEvent::OutputCommandFailed(..) => {
            apply_output_event(nav, event, ui);
        }

        AppEvent::SceneInventoryUpdated(inventory) => {
            // Same-scope inventory readbacks must not blank valid cached stills.
            // Session/collection retirement and the preview scheduler own invalidation.
            if let Some(ui) = ui {
                show_live_view(&ui.live);
            }
            nav.state.borrow_mut().apply_scene_inventory(inventory);
            if super::persist::registry_import_busy() {
                let scoped = nav.scoped();
                let after_ui = ui.cloned();
                if !super::persist::after_registry_import(move || {
                    if scoped.is_current() {
                        super::persist::reconcile_registry(&scoped);
                        refresh_scene_inventory_views(&scoped, after_ui.as_ref());
                    }
                }) {
                    report_failure(
                        nav,
                        ui,
                        fl!(LANGUAGE_LOADER, "registry-scene-rename-backlog"),
                    );
                }
            } else {
                super::persist::reconcile_registry(nav);
                refresh_scene_inventory_views(nav, ui);
            }
        }

        AppEvent::SceneRenamed { uuid, old, new } => {
            nav.state.borrow_mut().workspace.invalidate_controls();
            let scoped = nav.scoped();
            let ui_after = ui.cloned();
            let admitted = super::persist::after_registry_import(move || {
                if !scoped.is_current() {
                    return;
                }
                use crate::storage::registry::SceneRenameResult;
                let mut result = SceneRenameResult::Unchanged;
                super::persist::persist_registry(&scoped, "observed scene rename", |registry| {
                    result = registry.rename_scene(&old, &new, uuid.as_deref());
                    result == SceneRenameResult::Renamed
                });
                if result == SceneRenameResult::Renamed {
                    scoped.state.borrow_mut().show_registry.generation += 1;
                }
                if result == SceneRenameResult::Conflict {
                    report_failure(
                        &scoped,
                        ui_after.as_ref(),
                        fl!(
                            LANGUAGE_LOADER,
                            "registry-scene-rename-conflict",
                            old = old.as_str(),
                            new = new.as_str()
                        ),
                    );
                }
                if let Some(ui) = &ui_after {
                    let inventory = scoped.state.borrow().scene_inventory.clone();
                    rebuild_scene_cards(&ui.live, &inventory, &scoped);
                    ui.refreshers.call_visible_among(
                        &scoped,
                        &[Page::Inventory, Page::Doctor, Page::Mixer, Page::Graph],
                    );
                }
            });
            if !admitted {
                report_failure(
                    nav,
                    ui,
                    fl!(LANGUAGE_LOADER, "registry-scene-rename-backlog"),
                );
            }
        }

        AppEvent::ProfilesUpdated(profiles) => {
            nav.state.borrow_mut().profiles = profiles.clone();
            if let Some(ui) = ui {
                ui.header_selectors.update_profiles(&profiles);
            }
        }

        AppEvent::SceneCollectionsUpdated(collections) => {
            {
                let mut state = nav.state.borrow_mut();
                if state.scene_collections.current != collections.current {
                    state.invalidate_collection_runtime();
                }
                state.scene_collections = collections.clone();
            }
            if let Some(ui) = ui {
                ui.header_selectors.update_scene_collections(&collections);
            }
            let refresh_nav = nav.clone();
            let refresh_ui = ui.cloned();
            super::persist::select_registry_scope(nav, ui.map(|ui| ui.toast.clone()), move || {
                if let Some(ui) = &refresh_ui {
                    let state = refresh_nav.state.borrow();
                    let inventory = state.scene_inventory.clone();
                    let inputs = state.audio_inputs.clone();
                    drop(state);
                    rebuild_scene_cards(&ui.live, &inventory, &refresh_nav);
                    rebuild_audio_cards(&ui.live, &inputs, &refresh_nav);
                    ui.refreshers.call_visible_among(
                        &refresh_nav,
                        &[Page::Inventory, Page::Doctor, Page::Mixer, Page::Graph],
                    );
                }
            });
        }

        AppEvent::CurrentSceneChanged(scene_id) => {
            let inventory = {
                let mut state = nav.state.borrow_mut();
                state.workspace.log(
                    crate::domain::workflow::ActivityOrigin::Observed,
                    fl!(
                        LANGUAGE_LOADER,
                        "window-current-scene",
                        scene = scene_id.clone()
                    ),
                );
                state.workspace.mark_dirty();
                state.set_current_program(scene_id);
                state.scene_inventory.clone()
            };
            if let Some(ui) = ui {
                rebuild_scene_cards(&ui.live, &inventory, nav);
            }
            if let Some(ui) = ui {
                let inputs = nav.state.borrow().audio_inputs.clone();
                rebuild_audio_cards(&ui.live, &inputs, nav);
                ui.refreshers.call_if_visible(nav, Page::Mixer);
            }
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
            if let Some(ui) = ui {
                let toast = plain_toast(&fl!(
                    LANGUAGE_LOADER,
                    "window-toast-obs-error",
                    error = message
                ));
                toast.set_timeout(8);
                ui.toast.add_toast(toast);
            }
        }

        AppEvent::MixerAudioInputsUpdated { request, inputs } => {
            let transition = nav
                .state
                .borrow_mut()
                .set_mixer_audio_success(request, inputs);
            if let Some(ui) = ui {
                apply_mixer_audio_transition(nav, &ui.refreshers, transition);
            }
        }

        AppEvent::MixerAudioInputsFailed { request, message } => {
            let transition = nav
                .state
                .borrow_mut()
                .set_mixer_audio_failure(request, message);
            if let Some(ui) = ui {
                apply_mixer_audio_transition(nav, &ui.refreshers, transition);
            }
        }

        AppEvent::InputMuteChanged { input, muted } => {
            let rebuild_mixer = {
                let mut state = nav.state.borrow_mut();
                state.apply_input_mute(&input, muted);
                state.should_rebuild_visible_mixer_for_input_event(&input)
            };

            if let Some(ui) = ui {
                update_live_audio_card(&ui.live, &input, |card| card.update_mute(muted));
                (ui.live.quick_actions_refresh)();
                if rebuild_mixer {
                    ui.refreshers.mixer_handle.update_mute(&input, muted);
                }
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
                state.should_rebuild_visible_mixer_for_input_event(&input)
            };

            if let Some(ui) = ui {
                update_live_audio_card(&ui.live, &input, |card| {
                    card.update_volume(volume_mul, volume_db)
                });
                if rebuild_mixer {
                    ui.refreshers
                        .mixer_handle
                        .update_volume(&input, volume_mul, volume_db);
                }
            }
        }

        // Meter widgets read this from `AppState` on their own frames, so
        // there is nothing to rebuild here — storing it is the whole job.
        AppEvent::InputLevelsUpdated {
            levels,
            observed_at,
        } => {
            nav.state
                .borrow_mut()
                .set_audio_levels_at(levels, observed_at);
        }

        AppEvent::GraphUpdated(graph) => {
            nav.state.borrow_mut().set_topology_ready(graph);
            if let Some(ui) = ui {
                ui.refreshers
                    .call_visible_among(nav, &[Page::Graph, Page::Doctor]);
            }
        }

        AppEvent::StatsUpdated {
            observed_at,
            stats,
            bitrate_kbps,
            stream,
        } => {
            let streaming = {
                let mut state = nav.state.borrow_mut();
                state.set_obs_stats_at(stats, bitrate_kbps, stream, observed_at);
                state.stream_status.active
            };
            if let Some(ui) = ui {
                status_bar::set_stats(&ui.status_bar, &stats, bitrate_kbps, streaming);
            }
            // History is recorded on every sample; only redraw when the charts
            // are actually on screen (embedded, or in a detached pop-out).
            if let Some(ui) = ui {
                ui.refreshers.refresh_stats(nav);
            }
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
    ui: Option<&EventUiContext>,
    status: &ObsStatus,
    live_hint: &str,
) {
    use crate::ui::pages::live::{rebuild_audio_cards, show_disconnected_view};

    if let Some(ui) = ui {
        super::previews::clear(&ui.live, nav);
    }
    nav.state.borrow_mut().reset_obs_session(status.clone());
    let Some(ui) = ui else {
        return;
    };

    sync_output_indicators(nav, &ui.sidebar_controls, &ui.streaming_chrome);

    status_bar::set_connection(&ui.status_bar, status);
    status_bar::clear_stats(&ui.status_bar);

    show_disconnected_view(&ui.live, live_hint);
    rebuild_audio_cards(&ui.live, &[], nav);

    // The header lists belong to the session that just ended.
    ui.header_selectors.clear();
    ui.refreshers.call_visible_among(
        nav,
        &[
            Page::Mixer,
            Page::Inventory,
            Page::Graph,
            Page::Doctor,
            Page::Sources,
        ],
    );
    ui.refreshers.refresh_stats(nav);
}

/// Redraw one output's status-bar slot and the indicators that follow it.
///
/// Every output event ends this way: whatever the event changed, the status
/// bar is re-rendered from `AppState` rather than from the event, so the two
/// can never disagree. Each mutation below stores the status it was given, so
/// reading it back here shows the same value the event carried.
fn refresh_output_status_bar(nav: &NavigationContext, ui: &EventUiContext, kind: OutputKind) {
    render_output_slot(&nav.state, &ui.status_bar, kind);
    sync_output_indicators(nav, &ui.sidebar_controls, &ui.streaming_chrome);
}

/// Write one output's status-bar slot from `AppState`.
///
/// The output events and the once-a-second clock both draw the slot through
/// here, so neither overwrites the other with a different rendering. The
/// borrow ends before the widget is touched.
fn render_output_slot(state: &RefCell<AppState>, status_bar: &StatusBarHandle, kind: OutputKind) {
    let (label, active) = {
        let state = state.borrow();
        let (status, active_since) = match kind {
            OutputKind::Stream => (&state.stream_status, state.stream_active_since),
            OutputKind::Record => (&state.record_status, state.record_active_since),
        };
        let label = output_status_line(kind, status, active_since);
        (label, status.active)
    };
    match kind {
        OutputKind::Stream => status_bar::set_stream(status_bar, &label, active),
        OutputKind::Record => status_bar::set_record(status_bar, &label, active),
    }
}

fn apply_output_event(nav: &NavigationContext, event: AppEvent, ui: Option<&EventUiContext>) {
    if let (Some(ui), AppEvent::OutputCommandFailed(_, failure)) = (ui, &event) {
        ui.toast.add_toast(plain_toast(failure.message()));
    }
    let kind = nav
        .state
        .borrow_mut()
        .apply_output_event(&event, crate::controller::workspace::unix_ms());
    // `apply_event` forwards only output variants here.
    let Some(kind) = kind else {
        debug_assert!(false, "non-output event routed to output handler");
        return;
    };
    if let Some(ui) = ui {
        refresh_output_status_bar(nav, ui, kind);
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

/// Toast and log a failure; the next render shows the log entry.
fn report_failure(nav: &NavigationContext, ui: Option<&EventUiContext>, message: String) {
    if let Some(ui) = ui {
        ui.toast.add_toast(plain_toast(&message));
    }
    let mut state = nav.state.borrow_mut();
    state
        .workspace
        .log(crate::domain::workflow::ActivityOrigin::Failed, message);
}

fn command_failure_message(operation: FailedOperation, error: &AppError) -> String {
    let message = error.user_message();
    match operation {
        FailedOperation::Command => fl!(
            LANGUAGE_LOADER,
            "error-notification-title",
            message = message
        ),
        FailedOperation::Profile | FailedOperation::ProfileReadback => {
            format!(
                "{}: {message}",
                fl!(LANGUAGE_LOADER, "window-selector-profile-label")
            )
        }
        FailedOperation::Collection | FailedOperation::CollectionReadback => {
            format!(
                "{}: {message}",
                fl!(LANGUAGE_LOADER, "window-selector-collection-label")
            )
        }
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
    mixer_handle: Rc<crate::ui::pages::mixer::MixerPageHandle>,
    graph: RefreshFn,
    inventory: RefreshFn,
    doctor: RefreshFn,
    settings: RefreshFn,
    help: RefreshFn,
    sources: RefreshFn,
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
            Page::Sources => (self.sources)(),
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
mod tests;
