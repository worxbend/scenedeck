//! Studio controls follow authoritative OBS state; refresh never emits a command.
use crate::{
    controller::state::ObsStatus,
    domain::control::ControlAction,
    infra::i18n::LANGUAGE_LOADER,
    ui::{navigation::NavigationContext, workspace},
};
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, DropDown, Label, Orientation, SpinButton, Switch};
use i18n_embed_fl::fl;
use std::{cell::Cell, rc::Rc, time::Duration};

pub(crate) fn ready(nav: &NavigationContext) -> bool {
    let state = nav.state.borrow();
    matches!(state.obs_status, ObsStatus::Connected { .. })
        && !state.workspace.loading
        && !state.workspace.pending
        && !state.workspace.dirty
        && state.workspace.error.is_none()
        && state
            .workspace
            .observed_at
            .is_some_and(|time| time.elapsed() < Duration::from_secs(30))
}

pub(crate) fn toggle(nav: NavigationContext) -> GtkBox {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    let label = Label::new(Some(&fl!(LANGUAGE_LOADER, "studio-mode-label")));
    let switch = Switch::builder().valign(gtk4::Align::Center).build();
    switch.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "control-studio"
    ))]);
    root.append(&label);
    root.append(&switch);
    let syncing = Rc::new(Cell::new(false));
    switch.connect_state_set({
        let syncing = syncing.clone();
        let nav = nav.clone();
        move |_, desired| {
            let allowed = {
                let state = nav.state.borrow();
                state
                    .workspace
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.studio.enabled)
                    .is_some_and(|current| current != desired)
                    && state
                        .available_requests
                        .iter()
                        .any(|r| r == "SetStudioModeEnabled")
            };
            if !syncing.get() && allowed && ready(&nav) {
                workspace::act(&nav, ControlAction::SetStudioMode(desired));
            }
            glib::Propagation::Stop
        }
    });
    let weak = root.downgrade();
    let last_observed = Cell::new(None::<Option<bool>>);
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if weak.upgrade().is_none() {
            return glib::ControlFlow::Break;
        }
        let state = nav.state.borrow();
        let observed = state
            .workspace
            .snapshot
            .as_ref()
            .and_then(|s| s.studio.enabled);
        syncing.set(true);
        // Do not reset GTK's tentative active value before its toggle animation emits state-set.
        if last_observed.replace(Some(observed)) != Some(observed)
            || switch.state() != observed.unwrap_or(false)
            || state.workspace.pending
            || state.workspace.error.is_some()
        {
            switch.set_state(observed.unwrap_or(false));
            switch.set_active(observed.unwrap_or(false));
        }
        switch.set_sensitive(
            observed.is_some()
                && ready(&nav)
                && state
                    .available_requests
                    .iter()
                    .any(|r| r == "SetStudioModeEnabled"),
        );
        syncing.set(false);
        glib::ControlFlow::Continue
    });
    root
}

fn studio_ready(nav: &NavigationContext) -> bool {
    ready(nav)
        && nav
            .state
            .borrow()
            .workspace
            .snapshot
            .as_ref()
            .is_some_and(|s| s.studio.enabled == Some(true))
}

pub(crate) fn transition_row(nav: NavigationContext) -> GtkBox {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.add_css_class("live-transition-toolbar");
    let transitions = DropDown::from_strings(&[]);
    transitions.set_hexpand(true);
    transitions.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "control-transition"
    ))]);
    let duration = SpinButton::with_range(50.0, 20000.0, 50.0);
    duration.set_width_chars(5);
    duration.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "control-duration"
    ))]);
    let units = Label::new(Some("ms"));
    let take = Button::with_label(&fl!(LANGUAGE_LOADER, "control-take"));
    take.add_css_class("suggested-action");
    root.append(&transitions);
    root.append(&duration);
    root.append(&units);
    root.append(&take);
    let syncing = Rc::new(Cell::new(false));
    transitions.connect_selected_notify({
        let nav = nav.clone();
        let syncing = syncing.clone();
        move |drop| {
            if syncing.get() || !studio_ready(&nav) {
                return;
            }
            if let Some(item) = drop
                .selected_item()
                .and_then(|o| o.downcast::<gtk4::StringObject>().ok())
            {
                workspace::act(
                    &nav,
                    ControlAction::SetTransition(item.string().to_string()),
                );
            }
        }
    });
    duration.connect_value_changed({
        let nav = nav.clone();
        let syncing = syncing.clone();
        move |spin| {
            if !syncing.get() && studio_ready(&nav) {
                workspace::act(
                    &nav,
                    ControlAction::SetTransitionDuration(spin.value_as_int().into()),
                );
            }
        }
    });
    take.connect_clicked({
        let nav = nav.clone();
        move |_| {
            if studio_ready(&nav) {
                workspace::act(&nav, ControlAction::Take);
            }
        }
    });
    let weak = root.downgrade();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let Some(root) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let state = nav.state.borrow();
        let studio = state.workspace.snapshot.as_ref().map(|s| &s.studio);
        root.set_visible(studio.is_some_and(|s| s.enabled == Some(true)));
        if let Some(studio) = studio {
            syncing.set(true);
            let values: Vec<_> = studio.transitions.iter().map(String::as_str).collect();
            let old = transitions
                .model()
                .and_then(|m| m.downcast::<gtk4::StringList>().ok());
            if old.as_ref().is_none_or(|m| {
                (0..m.n_items())
                    .filter_map(|i| m.string(i))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    != studio.transitions
            }) {
                transitions.set_model(Some(&gtk4::StringList::new(&values)));
            }
            transitions.set_selected(
                studio
                    .transitions
                    .iter()
                    .position(|name| Some(name) == studio.transition.as_ref())
                    .map_or(gtk4::INVALID_LIST_POSITION, |i| i as u32),
            );
            if !duration.has_focus() {
                if let Some(ms) = studio.transition_duration_ms {
                    duration.set_value(ms as f64);
                }
            }
            duration
                .set_visible(!studio.transition_fixed && studio.transition_duration_ms.is_some());
            units.set_visible(duration.is_visible());
            let supported = |request: &str| state.available_requests.iter().any(|r| r == request);
            transitions.set_sensitive(ready(&nav) && supported("SetCurrentSceneTransition"));
            duration.set_sensitive(ready(&nav) && supported("SetCurrentSceneTransitionDuration"));
            take.set_sensitive(ready(&nav) && supported("TriggerStudioModeTransition"));
            syncing.set(false);
        }
        glib::ControlFlow::Continue
    });
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::{app_controller::AppController, event::AppEvent, state::AppState};
    use crate::domain::control::{ControlSnapshot, StudioState};
    use crate::storage::{config::AppConfig, registry::SceneRegistry};
    use std::{cell::RefCell, sync::mpsc, time::Instant};

    fn pump(duration: Duration) {
        let context = glib::MainContext::default();
        let until = Instant::now() + duration;
        while Instant::now() < until {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// This runs the real GTK animation and the production 100 ms readback timer.
    /// Its controller has no OBS client, so it cannot affect any broadcast.
    #[test]
    #[ignore = "requires a native GTK display; run alone with --test-threads=1"]
    fn native_studio_toggle_readback_and_user_activation_are_separate() {
        // Each display test needs its own process: Rust otherwise runs GTK on different threads.
        if std::env::var_os("SCENEDECK_STUDIO_NATIVE_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "ui::widgets::studio_controls::tests::native_studio_toggle_readback_and_user_activation_are_separate", "--ignored", "--test-threads=1"])
                .env("SCENEDECK_STUDIO_NATIVE_CHILD", "1")
                .output().unwrap();
            assert!(
                output.status.success(),
                "native child failed: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        gtk4::init().expect("native GTK display");
        let settings = gtk4::Settings::default().expect("GTK settings");
        let animations = settings.property::<bool>("gtk-enable-animations");
        settings.set_property("gtk-enable-animations", true);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let (tx, events) = mpsc::sync_channel(8);
        let state = Rc::new(RefCell::new(AppState::new(
            AppConfig::default(),
            SceneRegistry::default(),
            None,
            None,
        )));
        {
            let mut state = state.borrow_mut();
            state.obs_status = ObsStatus::Connected {
                obs_version: "native UI fixture".into(),
            };
            state.scene_collections.current = Some("Disposable fixture".into());
            state.available_requests = vec!["SetStudioModeEnabled".into()];
            state.workspace.observed_at = Some(Instant::now());
            state.workspace.snapshot = Some(ControlSnapshot {
                studio: StudioState {
                    enabled: Some(false),
                    ..Default::default()
                },
                ..Default::default()
            });
        }
        let controller = Rc::new(RefCell::new(AppController::new(
            runtime.handle().clone(),
            tx,
        )));
        let nav = NavigationContext::new(state.clone(), gtk4::Stack::new(), controller);
        let root = toggle(nav);
        let switch = root.last_child().and_downcast::<Switch>().unwrap();
        let window = gtk4::Window::builder()
            .title("SceneDeck Studio toggle regression fixture")
            .default_width(320)
            .default_height(100)
            .child(&root)
            .build();
        window.present();
        pump(Duration::from_millis(150));
        assert!(switch.is_sensitive());
        assert_eq!(state.borrow().workspace.sequence, 0);

        // An external OBS readback updates the switch, without echoing a setter.
        state
            .borrow_mut()
            .workspace
            .snapshot
            .as_mut()
            .unwrap()
            .studio
            .enabled = Some(true);
        pump(Duration::from_millis(150));
        assert!(switch.is_active());
        assert!(switch.state());
        assert_eq!(state.borrow().workspace.sequence, 0);
        assert!(events.try_recv().is_err());
        state
            .borrow_mut()
            .workspace
            .snapshot
            .as_mut()
            .unwrap()
            .studio
            .enabled = Some(false);
        pump(Duration::from_millis(150));
        assert!(!switch.is_active());

        // A real widget activation must finish its animation despite periodic readback.
        assert!(switch.activate());
        pump(Duration::from_millis(350));
        assert_eq!(state.borrow().workspace.sequence, 1);
        assert!(state.borrow().workspace.pending);
        assert_eq!(state.borrow().workspace.activity.entries.len(), 1);
        assert!(!switch.is_sensitive());
        assert!(matches!(
            events.try_recv().unwrap(),
            AppEvent::WorkspaceLoaded { action: true, .. }
        ));
        switch.activate();
        pump(Duration::from_millis(350));
        assert_eq!(
            state.borrow().workspace.sequence,
            1,
            "pending mode change cannot duplicate"
        );
        assert!(events.try_recv().is_err());

        // Unknown mode stays disabled and cannot originate an OBS command.
        {
            let mut state = state.borrow_mut();
            state.workspace.pending = false;
            state.workspace.loading = false;
            state.workspace.snapshot.as_mut().unwrap().studio.enabled = None;
        }
        pump(Duration::from_millis(150));
        assert!(!switch.is_sensitive());
        assert_eq!(state.borrow().workspace.sequence, 1);
        // A readback can lose mode while an otherwise valid user animation is in flight.
        state
            .borrow_mut()
            .workspace
            .snapshot
            .as_mut()
            .unwrap()
            .studio
            .enabled = Some(false);
        pump(Duration::from_millis(150));
        switch.activate();
        state
            .borrow_mut()
            .workspace
            .snapshot
            .as_mut()
            .unwrap()
            .studio
            .enabled = None;
        pump(Duration::from_millis(350));
        assert_eq!(
            state.borrow().workspace.sequence,
            1,
            "losing known mode during activation must not dispatch"
        );
        state
            .borrow_mut()
            .workspace
            .snapshot
            .as_mut()
            .unwrap()
            .studio
            .enabled = Some(false);
        pump(Duration::from_millis(150));
        assert!(
            !switch.is_active(),
            "a known Direct readback must clear the aborted tentative Studio value"
        );
        assert!(!switch.state());
        window.close();
        window.set_child(gtk4::Widget::NONE);
        drop(root);
        pump(Duration::from_millis(150));
        settings.set_property("gtk-enable-animations", animations);
    }
}
