//! Persistent cross-page health summary with read-only incident details.
mod audio_editor;
use super::navigation::NavigationContext;
use crate::{
    controller::state::Page,
    domain::health::{Alert, AlertKind, AlertPhase},
    infra::i18n::LANGUAGE_LOADER,
};
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation};
use i18n_embed_fl::fl;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

fn text(value: &str) -> Label {
    Label::builder()
        .label(value)
        .use_markup(false)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .xalign(0.0)
        .build()
}
fn title(kind: AlertKind, health: &crate::domain::health::HealthMonitor) -> String {
    match kind {
        AlertKind::ConnectionLost => fl!(LANGUAGE_LOADER, "health-connection-lost"),
        AlertKind::StreamStopped => fl!(LANGUAGE_LOADER, "health-stream-stopped"),
        AlertKind::RecordStopped => fl!(LANGUAGE_LOADER, "health-record-stopped"),
        AlertKind::NetworkDrops => fl!(LANGUAGE_LOADER, "health-network"),
        AlertKind::RenderOverload => fl!(LANGUAGE_LOADER, "health-render"),
        AlertKind::EncoderOverload => fl!(LANGUAGE_LOADER, "health-encoder"),
        AlertKind::LowDisk => fl!(LANGUAGE_LOADER, "health-disk"),
        AlertKind::AudioSilence(index) => fl!(
            LANGUAGE_LOADER,
            "health-audio-silence",
            input = health.audio.name(index)
        ),
        AlertKind::AudioClipping(index) => fl!(
            LANGUAGE_LOADER,
            "health-audio-clipping",
            input = health.audio.name(index)
        ),
    }
}
fn description(kind: AlertKind) -> String {
    match kind {
        AlertKind::ConnectionLost => fl!(LANGUAGE_LOADER, "health-connection-help"),
        AlertKind::StreamStopped => fl!(LANGUAGE_LOADER, "health-stream-stopped-help"),
        AlertKind::RecordStopped => fl!(LANGUAGE_LOADER, "health-record-stopped-help"),
        AlertKind::NetworkDrops => fl!(LANGUAGE_LOADER, "health-network-help"),
        AlertKind::RenderOverload => fl!(LANGUAGE_LOADER, "health-render-help"),
        AlertKind::EncoderOverload => fl!(LANGUAGE_LOADER, "health-encoder-help"),
        AlertKind::LowDisk => fl!(LANGUAGE_LOADER, "health-disk-help"),
        AlertKind::AudioSilence(_) => fl!(LANGUAGE_LOADER, "health-audio-silence-help"),
        AlertKind::AudioClipping(_) => fl!(LANGUAGE_LOADER, "health-audio-clipping-help"),
    }
}
fn caption(alert: Alert, health: &crate::domain::health::HealthMonitor) -> String {
    let phase = match alert.phase {
        AlertPhase::Active => fl!(LANGUAGE_LOADER, "health-active"),
        AlertPhase::Recovering => fl!(LANGUAGE_LOADER, "health-recovering"),
        AlertPhase::Unobserved => fl!(LANGUAGE_LOADER, "health-unobserved"),
    };
    fl!(
        LANGUAGE_LOADER,
        "health-caption",
        title = title(alert.kind, health),
        phase = phase
    )
}

/// App-owned tick: survives a hidden/closed shell while detached Stats is open.
pub(crate) fn monitor(nav: NavigationContext) -> impl Fn() {
    let attempted = std::cell::Cell::new(None::<Instant>);
    move || {
        let now = Instant::now();
        let refresh = {
            let mut state = nav.state.borrow_mut();
            state.observe_audio_health(now)
                && !state.workspace.loading
                && !state.workspace.pending
                && (state.workspace.dirty
                    || state.workspace.observed_at.is_none_or(|at| {
                        now.saturating_duration_since(at) >= Duration::from_secs(10)
                    }))
                && attempted
                    .get()
                    .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_secs(10))
        };
        if refresh {
            attempted.set(Some(now));
            super::workspace::refresh(&nav);
        }
    }
}

pub(crate) fn button(nav: NavigationContext) -> Button {
    let button = Button::new();
    let caption = fl!(LANGUAGE_LOADER, "health-title");
    button.set_child(Some(&text(&caption)));
    button.update_property(&[gtk4::accessible::Property::Label(&caption)]);
    button.set_widget_name("health-open");
    button.connect_clicked(move |button| {
        if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
            show(&parent, &nav);
        }
    });
    button
}

pub(crate) fn build(nav: NavigationContext) -> (GtkBox, glib::SourceId) {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.set_widget_name("health-alerts");
    root.set_margin_start(12);
    root.set_margin_end(12);
    root.set_margin_top(4);
    root.set_margin_bottom(4);
    let label = text("");
    label.set_widget_name("health-summary");
    label.set_hexpand(true);
    root.append(&label);
    let details = Button::from_icon_name("dialog-warning-symbolic");
    details.set_widget_name("health-details");
    details.set_valign(gtk4::Align::Center);
    let accessible = fl!(LANGUAGE_LOADER, "health-title");
    details.set_tooltip_text(Some(&accessible));
    details.update_property(&[gtk4::accessible::Property::Label(&accessible)]);
    root.append(&details);
    details.connect_clicked({
        let nav = nav.clone();
        move |button| {
            if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
                show(&parent, &nav);
            }
        }
    });
    root.set_visible(false);
    let source = glib::timeout_add_local(Duration::from_millis(250), {
        let root = root.downgrade();
        let label = label.downgrade();
        move || {
            let (Some(root), Some(label)) = (root.upgrade(), label.upgrade()) else {
                return glib::ControlFlow::Break;
            };
            let state = nav.state.borrow();
            let health = &state.health;
            let alerts = health.alerts(Instant::now());
            root.set_visible(!alerts.is_empty());
            if let Some(first) = alerts.first() {
                let summary = caption(*first, health);
                label.set_text(&if alerts.len() > 1 {
                    fl!(
                        LANGUAGE_LOADER,
                        "health-more",
                        summary = summary,
                        count = (alerts.len() - 1).to_string()
                    )
                } else {
                    summary
                });
                root.set_tooltip_text(Some(
                    &alerts
                        .into_iter()
                        .map(|alert| caption(alert, health))
                        .collect::<Vec<_>>()
                        .join("\n"),
                ));
            }
            glib::ControlFlow::Continue
        }
    });
    (root, source)
}
fn show(parent: &gtk4::Window, nav: &NavigationContext) {
    let dialog = adw::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .heading(fl!(LANGUAGE_LOADER, "health-title"))
        .build();
    dialog.set_widget_name("health-dialog");
    dialog.add_responses(&[
        ("close", &fl!(LANGUAGE_LOADER, "preflight-close")),
        ("stats", &fl!(LANGUAGE_LOADER, "health-open-stats")),
        ("acknowledge", &fl!(LANGUAGE_LOADER, "health-acknowledge")),
    ]);
    dialog.set_close_response("close");
    dialog.set_default_response(Some("close"));
    let label = text("");
    label.set_widget_name("health-report");
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.append(&text(&fl!(LANGUAGE_LOADER, "health-policy")));
    content.append(&text(&fl!(LANGUAGE_LOADER, "health-event-policy")));
    content.append(&label);
    let audio_settings = Button::new();
    audio_settings.set_child(Some(&text(&fl!(LANGUAGE_LOADER, "health-audio-settings"))));
    audio_settings.set_widget_name("health-audio-settings");
    audio_settings.connect_clicked({
        let nav = nav.clone();
        move |button| {
            if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
                audio_editor::show(&parent, &nav);
            }
        }
    });
    content.append(&audio_settings);
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(120)
        .max_content_height(360)
        .propagate_natural_height(true)
        .child(&content)
        .build();
    dialog.set_extra_child(Some(&scroll));
    let displayed = Rc::new(RefCell::new(Vec::new()));
    let render = {
        let audio_settings = audio_settings.downgrade();
        let displayed = displayed.clone();
        let dialog = dialog.downgrade();
        let nav = nav.clone();
        let label = label.downgrade();
        move || {
            let Some(label) = label.upgrade() else {
                return;
            };
            let state = nav.state.borrow();
            let health = &state.health;
            let alerts = health.alerts(Instant::now());
            if let Some(button) = audio_settings.upgrade() {
                button.set_sensitive(
                    nav.registry_is_current()
                        && state.desired_registry_target().as_ref()
                            == Some(&state.show_registry.target),
                );
            }
            *displayed.borrow_mut() = alerts.clone();
            if let Some(dialog) = dialog.upgrade() {
                dialog.set_response_enabled(
                    "acknowledge",
                    alerts.iter().any(|alert| alert.event_id.is_some()),
                );
            }
            let report = if alerts.is_empty() {
                fl!(LANGUAGE_LOADER, "health-no-alerts")
            } else {
                alerts
                    .into_iter()
                    .map(|alert| format!("{}\n{}", caption(alert, health), description(alert.kind)))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            };
            let audio = health
                .audio
                .states(Instant::now())
                .into_iter()
                .map(|(name, status)| {
                    use crate::domain::health::audio::AudioState;
                    let status = match status {
                        AudioState::Disabled => fl!(LANGUAGE_LOADER, "health-audio-disabled"),
                        AudioState::Unavailable => fl!(LANGUAGE_LOADER, "health-unobserved"),
                        AudioState::Missing => fl!(LANGUAGE_LOADER, "health-audio-missing"),
                        AudioState::Monitoring => fl!(LANGUAGE_LOADER, "health-audio-monitoring"),
                        AudioState::Muted => fl!(LANGUAGE_LOADER, "health-audio-muted"),
                    };
                    format!("{name} — {status}")
                })
                .collect::<Vec<_>>()
                .join("\n");
            label.set_text(&format!("{report}\n\n{audio}"));
        }
    };
    dialog.connect_response(None, {
        let nav = nav.clone();
        let handled = std::cell::Cell::new(false);
        move |_, response| {
            if handled.replace(true) {
                return;
            }
            match response {
                "stats" => nav.switch_to_page(Page::Stats),
                "acknowledge" => {
                    for alert in displayed.borrow().iter() {
                        nav.state.borrow_mut().health.acknowledge(*alert);
                    }
                }
                _ => {}
            }
        }
    });
    render();
    dialog.present();
    glib::timeout_add_local(Duration::from_millis(250), {
        let dialog = dialog.downgrade();
        move || {
            if dialog.upgrade().is_none_or(|dialog| !dialog.is_visible()) {
                return glib::ControlFlow::Break;
            }
            render();
            glib::ControlFlow::Continue
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{
        preflight::tests::{find, window},
        test_support::{native_test, navigation, pump},
    };
    #[test]
    #[ignore = "requires isolated GTK"]
    fn native_health_summary_survives_page_changes_and_reports_stale_evidence() {
        native_test("ui::health_alerts::tests::native_health_summary_survives_page_changes_and_reports_stale_evidence", || {
            let (nav, stack, _runtime, _events) = navigation();
            for page in [Page::Live, Page::Mixer, Page::Sources, Page::Stats] { stack.add_named(&Label::new(Some(page.id())), Some(page.id())); }
            gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
            let (root, source) = build(nav.clone());
            let body = GtkBox::new(Orientation::Vertical, 4); body.append(&root); body.append(&stack);
            let parent = adw::Window::builder().default_width(360).default_height(600).build(); parent.set_content(Some(&body)); parent.present(); pump(Duration::from_millis(300));
            assert!(!root.is_visible());
            let start = Instant::now() - Duration::from_secs(6);
            crate::domain::health::tests::raise(&mut nav.state.borrow_mut().health, AlertKind::NetworkDrops, start);
            pump(Duration::from_millis(300)); assert!(root.is_visible());
            let label = find(&root, "health-summary").downcast::<Label>().unwrap(); assert!(label.text().contains(&fl!(LANGUAGE_LOADER, "health-network")));
            assert!(root.measure(Orientation::Horizontal, -1).0 <= 360);
            for page in [Page::Mixer, Page::Sources, Page::Stats, Page::Live] { nav.switch_to_page(page); pump(Duration::from_millis(40)); assert!(root.is_visible()); assert_eq!(find(&root, "health-summary"), label.clone().upcast::<gtk4::Widget>()); }
            find(&root, "health-details").downcast::<Button>().unwrap().emit_clicked();
            let dialog = window("health-dialog").downcast::<adw::MessageDialog>().unwrap();
            dialog.set_default_size(360, 600); pump(Duration::from_millis(80));
            assert!(dialog.measure(Orientation::Horizontal, -1).0 <= 360);
            assert!(dialog.measure(Orientation::Vertical, 360).0 <= 600);
            assert!(find(&dialog, "health-report").downcast::<Label>().unwrap().text().contains(&fl!(LANGUAGE_LOADER, "health-network-help")));
            let mut stale = crate::domain::health::HealthMonitor::default();
            crate::domain::health::tests::raise(&mut stale, AlertKind::NetworkDrops, Instant::now() - Duration::from_secs(20));
            nav.state.borrow_mut().health = stale; pump(Duration::from_millis(300));
            assert!(label.text().contains(&fl!(LANGUAGE_LOADER, "health-unobserved")));
            dialog.response("stats"); assert_eq!(nav.state.borrow().current_page, Page::Stats);
            nav.state.borrow_mut().health.clear_samples(); pump(Duration::from_millis(300)); assert!(!root.is_visible());
            let weak = root.downgrade(); parent.close(); source.remove(); drop((root, body, label, dialog, parent)); pump(Duration::from_millis(300)); assert!(weak.upgrade().is_none());
        });
    }
    #[test]
    #[ignore = "requires isolated GTK"]
    fn native_health_event_acknowledgement_cannot_clear_new_or_cancelled_incidents() {
        native_test("ui::health_alerts::tests::native_health_event_acknowledgement_cannot_clear_new_or_cancelled_incidents", || {
            use crate::domain::output::{OutputKind, OutputStatus};
            let (nav, _stack, _runtime, _events) = navigation();
            let stop = || {
                let mut state = nav.state.borrow_mut(); state.health.connected();
                state.health.observe_output(OutputKind::Record, &OutputStatus::active(), Instant::now());
                state.health.observe_output(OutputKind::Record, &OutputStatus::inactive(), Instant::now());
            };
            let (root, source) = build(nav.clone()); let parent = adw::Window::new(); parent.set_content(Some(&root)); parent.present(); stop(); pump(Duration::from_millis(300));
            let open = || { find(&root, "health-details").downcast::<Button>().unwrap().emit_clicked(); window("health-dialog").downcast::<adw::MessageDialog>().unwrap() };
            let old = open(); assert!(old.is_response_enabled("acknowledge")); stop(); old.response("acknowledge");
            assert_eq!(nav.state.borrow().health.alerts(Instant::now()).len(), 1);
            let cancelled = open(); cancelled.response("close"); cancelled.response("acknowledge");
            assert_eq!(nav.state.borrow().health.alerts(Instant::now()).len(), 1);
            open().response("acknowledge"); pump(Duration::from_millis(300)); assert!(!root.is_visible());
            nav.state.borrow_mut().reset_obs_session(crate::controller::state::ObsStatus::Error("lost".into())); pump(Duration::from_millis(300)); assert!(root.is_visible());
            assert!(find(&root, "health-summary").downcast::<Label>().unwrap().text().contains(&fl!(LANGUAGE_LOADER, "health-connection-lost")));
            nav.state.borrow_mut().health.connected(); pump(Duration::from_millis(300)); assert!(!root.is_visible());
            parent.close(); source.remove();
        });
    }
}
