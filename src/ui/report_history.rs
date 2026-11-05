//! Bounded checkpoint admission and explicit retry for optional session history.

use super::{
    background_io::{self, SerialLane},
    navigation::NavigationContext,
};
use crate::{controller::state::AppState, infra::i18n::LANGUAGE_LOADER};
use adw::prelude::*;
use i18n_embed_fl::fl;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::Arc,
    time::{Duration, Instant},
};

type State = Rc<RefCell<AppState>>;
#[cfg(test)]
mod tests;
type HistoryFailure = (Weak<RefCell<AppState>>, u64, background_io::FailureEvidence);
thread_local! {
    static ACTIVE: RefCell<Weak<RefCell<AppState>>> = const { RefCell::new(Weak::new()) };
    static FAILURES: RefCell<Vec<HistoryFailure>> = const { RefCell::new(Vec::new()) };
    static SETTINGS_FAILURES: RefCell<Vec<(Weak<RefCell<AppState>>, background_io::FailureEvidence)>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn register(state: &State) {
    ACTIVE.with(|active| *active.borrow_mut() = Rc::downgrade(state));
}

pub(crate) fn checkpoint_before_close() {
    if let Some(state) = ACTIVE.with(|active| active.borrow().upgrade()) {
        save(&state, true, false);
    }
}

/// Holds state, never shell widgets, until final checkpoint completion or cancel.
pub(crate) struct ExitCheckpoint {
    state: State,
    report: Option<crate::controller::state::ReportExit>,
}

impl ExitCheckpoint {
    pub(crate) fn new() -> Option<Self> {
        ACTIVE
            .with(|active| active.borrow().upgrade())
            .map(|state| Self {
                state,
                report: None,
            })
    }

    pub(crate) fn prepare(&mut self) {
        if let Some(marker) = self
            .state
            .borrow_mut()
            .prepare_report_exit(&fl!(LANGUAGE_LOADER, "history-exit-requested"))
        {
            self.report = Some(marker);
        }
        save(&self.state, true, false);
    }

    /// Reconcile a connection/terminal event received while the first write ran.
    pub(crate) fn checkpoint_again(&mut self) -> bool {
        self.prepare();
        let state = self.state.borrow();
        let history = &state.report_history;
        state.config.session_history.enabled
            && !history.exit_abandoned
            && (history.busy || history.archive != *history.stored || history.save_error.is_some())
    }

    pub(crate) fn resume(self) {
        if let Some(marker) = &self.report {
            self.state
                .borrow_mut()
                .resume_report_exit(marker, &fl!(LANGUAGE_LOADER, "history-exit-resumed"));
        }
        // A timed-out write still owns its end snapshot; publish the resumed
        // interval after it. Returned failures require explicit user retry.
        let busy = self.state.borrow().report_history.busy;
        if busy {
            self.state.borrow_mut().report_history.force_after_pending = true;
        }
        save(&self.state, false, false);
    }
}

pub(crate) fn abandon_exit(abandoned: bool) {
    if let Some(state) = ACTIVE.with(|active| active.borrow().upgrade()) {
        state.borrow_mut().report_history.exit_abandoned = abandoned;
    }
}

pub(crate) fn finish_for_shutdown() {
    if let Some(state) = ACTIVE.with(|active| active.borrow().upgrade()) {
        state
            .borrow_mut()
            .workspace
            .output_results
            .invalidate(crate::controller::workspace::unix_ms());
        state
            .borrow_mut()
            .finish_report(&fl!(LANGUAGE_LOADER, "history-app-closed"));
        save(&state, true, false);
    }
}

pub(crate) fn retry_active() {
    abandon_exit(false);
    if let Some(state) = ACTIVE.with(|active| active.borrow().upgrade()) {
        reload(&state);
    }
}

pub(crate) fn tick(state: &State) {
    let due = {
        let state = state.borrow();
        let history = &state.report_history;
        state.config.session_history.enabled
            && !history.busy
            && history.load_error.is_none()
            && history.save_error.is_none()
            && (history.urgent
                || history
                    .last_attempt
                    .is_none_or(|at| at.elapsed() >= Duration::from_secs(10)))
    };
    if due {
        save(state, false, false);
    }
}

fn save(state: &State, force: bool, allow_disabled: bool) {
    let owner = Rc::downgrade(state);
    let (expected, replacement, revision) = {
        let mut state = state.borrow_mut();
        if state.report_history.exit_abandoned {
            return;
        }
        if !state.config.session_history.enabled && !allow_disabled {
            return;
        }
        let history = &mut state.report_history;
        if history.busy {
            history.force_after_pending |= force;
            return;
        }
        if !force && history.save_error.is_some() {
            return;
        }
        state.collect_report(Instant::now(), crate::controller::workspace::unix_ms());
        state.stage_report_history();
        let history = &mut state.report_history;
        if let Some(error) = &history.load_error {
            if history.archive != *history.stored && history.save_error.is_none() {
                history.save_error = Some(error.clone());
                FAILURES.with(|failures| {
                    failures.borrow_mut().push((
                        owner.clone(),
                        history.revision,
                        background_io::record_failure(),
                    ))
                });
            }
            return;
        }
        history.last_attempt = Some(Instant::now());
        history.urgent = false;
        if history.archive == *history.stored
            && history.save_error.is_none()
            && !history.clear_pending
        {
            history.saved_revision = history.revision;
            return;
        }
        history.busy = true;
        (
            history.stored.clone(),
            Arc::new(history.archive.clone()),
            history.revision,
        )
    };
    let path = crate::storage::xdg::report_history_path();
    let state = state.clone();
    let written = replacement.clone();
    background_io::run_serialized(
        SerialLane::ReportHistory,
        move || {
            std::panic::catch_unwind(|| {
                crate::storage::report_history::write(&path, &expected, &replacement)
            })
            .unwrap_or_else(|_| Err(std::io::Error::other("Session history writer failed")))
        },
        move |result| {
            let force_again = {
                let mut model = state.borrow_mut();
                let history = &mut model.report_history;
                history.busy = false;
                match result {
                    Ok(()) => {
                        history.stored = written;
                        history.saved_revision = revision;
                        history.save_error = None;
                        history.clear_pending = false;
                        FAILURES.with(|failures| {
                            failures.borrow_mut().retain(|(owner, at, evidence)| {
                                if owner.ptr_eq(&Rc::downgrade(&state)) && *at <= revision {
                                    background_io::resolve_failure(*evidence);
                                    false
                                } else {
                                    true
                                }
                            })
                        });
                    }
                    Err(error) => {
                        history.save_error = Some(error.to_string());
                        FAILURES.with(|failures| {
                            failures.borrow_mut().push((
                                Rc::downgrade(&state),
                                revision,
                                background_io::record_failure(),
                            ))
                        });
                    }
                }
                std::mem::take(&mut history.force_after_pending) && history.save_error.is_none()
            };
            if force_again {
                save(&state, true, allow_disabled);
            }
        },
    );
}

fn reload(state: &State) {
    if state.borrow().report_history.settings_error.is_some() {
        save_settings(state);
    }
    {
        let mut model = state.borrow_mut();
        if model.report_history.busy {
            return;
        }
        model.stage_report_history();
        model.report_history.busy = true;
    }
    let path = crate::storage::xdg::report_history_path();
    let state = state.clone();
    background_io::run_serialized(
        SerialLane::ReportHistory,
        move || crate::storage::report_history::read(&path).map_err(|error| error.to_string()),
        move |result| {
            let allow_disabled = {
                let mut model = state.borrow_mut();
                model.report_history.busy = false;
                model.load_report_history(result);
                model.report_history.clear_pending || model.report_history.save_error.is_some()
            };
            save(&state, true, allow_disabled);
        },
    );
}

fn save_settings(state: &State) {
    let config = {
        let mut model = state.borrow_mut();
        if model.report_history.settings_pending {
            return;
        }
        model.report_history.settings_pending = true;
        model.config.clone()
    };
    let state = state.clone();
    background_io::run_serialized(
        SerialLane::Config,
        move || crate::storage::config::write_config(&config),
        move |result| {
            let mut model = state.borrow_mut();
            model.report_history.settings_pending = false;
            match result {
                Ok(()) => {
                    model.report_history.settings_error = None;
                    SETTINGS_FAILURES.with(|failures| {
                        failures.borrow_mut().retain(|(owner, evidence)| {
                            if owner.ptr_eq(&Rc::downgrade(&state)) {
                                background_io::resolve_failure(*evidence);
                                false
                            } else {
                                true
                            }
                        })
                    });
                }
                Err(error) => {
                    model.report_history.settings_error = Some(error.to_string());
                    SETTINGS_FAILURES.with(|failures| {
                        failures
                            .borrow_mut()
                            .push((Rc::downgrade(&state), background_io::record_failure()))
                    });
                }
            }
        },
    );
}

fn text(value: &str) -> gtk4::Label {
    gtk4::Label::builder()
        .label(value)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .xalign(0.0)
        .build()
}

fn button(value: &str) -> gtk4::Button {
    let button = gtk4::Button::new();
    button.set_child(Some(&text(value)));
    button
}

fn status(state: &AppState) -> String {
    let history = &state.report_history;
    if history.busy || history.settings_pending {
        return fl!(LANGUAGE_LOADER, "history-saving");
    }
    if let Some(error) = history
        .load_error
        .as_ref()
        .or(history.save_error.as_ref())
        .or(history.settings_error.as_ref())
    {
        return fl!(LANGUAGE_LOADER, "history-error", error = error.as_str());
    }
    fl!(
        LANGUAGE_LOADER,
        "history-status",
        count = history.stored.entries.len().to_string(),
        mode = if state.config.session_history.enabled {
            fl!(LANGUAGE_LOADER, "history-on")
        } else {
            fl!(LANGUAGE_LOADER, "history-off")
        }
    )
}

pub(super) fn open_button(nav: NavigationContext) -> gtk4::Button {
    let button = button(&fl!(LANGUAGE_LOADER, "history-title"));
    button.connect_clicked(move |button| {
        if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
            show(&parent, &nav);
        }
    });
    button
}

pub(super) fn settings_group(nav: &NavigationContext) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "history-title"))
        .description(fl!(LANGUAGE_LOADER, "history-settings-description"))
        .build();
    group.add(&open_button(nav.clone()));
    group
}

fn show(parent: &gtk4::Window, nav: &NavigationContext) {
    let window = adw::Window::builder()
        .title(fl!(LANGUAGE_LOADER, "history-title"))
        .transient_for(parent)
        .default_width(580)
        .default_height(540)
        .build();
    window.set_widget_name("history-window");
    if let Some(app) = parent.application() {
        window.set_application(Some(&app));
    }
    let alive = Rc::new(Cell::new(true));
    window.connect_close_request({
        let alive = alive.clone();
        move |_| {
            alive.set(false);
            glib::Propagation::Proceed
        }
    });
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.append(&text(&fl!(LANGUAGE_LOADER, "history-policy")));
    let policy = Rc::new(RefCell::new(
        nav.state.borrow().config.session_history.clone(),
    ));
    let enabled = gtk4::CheckButton::new();
    enabled.set_child(Some(&text(&fl!(LANGUAGE_LOADER, "history-enable"))));
    enabled.set_active(policy.borrow().enabled);
    enabled.set_widget_name("history-enable");
    content.append(&enabled);
    content.append(&text(&fl!(LANGUAGE_LOADER, "history-retain")));
    let retention = gtk4::SpinButton::with_range(
        1.0,
        crate::domain::session_report::HISTORY_LIMIT as f64,
        1.0,
    );
    retention.set_value(policy.borrow().limit() as f64);
    retention.set_widget_name("history-retain");
    retention.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "history-retain"
    ))]);
    content.append(&retention);
    let apply = button(&fl!(LANGUAGE_LOADER, "history-apply"));
    apply.set_widget_name("history-apply");
    content.append(&apply);
    let notice = text("");
    notice.set_widget_name("history-status");
    content.append(&notice);
    let retry = button(&fl!(LANGUAGE_LOADER, "history-retry"));
    retry.set_widget_name("history-retry");
    content.append(&retry);
    let clear = button(&fl!(LANGUAGE_LOADER, "history-clear"));
    clear.set_widget_name("history-clear");
    clear.add_css_class("destructive-action");
    content.append(&clear);
    apply.connect_clicked({
        let nav = nav.clone();
        let alive = alive.clone();
        let policy = policy.clone();
        let notice = notice.clone();
        move |_| {
            if !alive.get()
                || nav.state.borrow().report_history.busy
                || nav.state.borrow().report_history.settings_pending
            {
                return;
            }
            if nav.state.borrow().config.session_history != *policy.borrow() {
                notice.set_text(&fl!(LANGUAGE_LOADER, "history-settings-changed"));
                return;
            }
            nav.state
                .borrow_mut()
                .configure_report_history(enabled.is_active(), retention.value_as_int() as usize);
            *policy.borrow_mut() = nav.state.borrow().config.session_history.clone();
            save_settings(&nav.state);
            save(&nav.state, true, false);
        }
    });
    retry.connect_clicked({
        let state = nav.state.clone();
        let alive = alive.clone();
        move |_| {
            if alive.get() {
                reload(&state);
            }
        }
    });
    clear.connect_clicked({
        let state = nav.state.clone();
        let alive = alive.clone();
        let parent = window.downgrade();
        move |_| {
            if !alive.get()
                || state.borrow().report_history.busy
                || state.borrow().report_history.load_error.is_some()
            {
                return;
            }
            let Some(parent) = parent.upgrade() else {
                return;
            };
            let dialog = adw::MessageDialog::new(
                Some(&parent),
                Some(&fl!(LANGUAGE_LOADER, "history-clear")),
                Some(&fl!(LANGUAGE_LOADER, "history-clear-confirm")),
            );
            dialog.set_widget_name("history-clear-confirm");
            dialog.add_response(
                "cancel",
                &fl!(LANGUAGE_LOADER, "inventory-dialog-cancel-label"),
            );
            dialog.add_response("clear", &fl!(LANGUAGE_LOADER, "history-clear"));
            dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            dialog.connect_response(None, {
                let state = state.clone();
                let alive = alive.clone();
                let used = Cell::new(false);
                move |dialog, response| {
                    if used.replace(true) {
                        return;
                    }
                    dialog.close();
                    if response != "clear" || !alive.get() || state.borrow().report_history.busy {
                        return;
                    }
                    state.borrow_mut().clear_report_history();
                    save(&state, true, true);
                }
            });
            dialog.present();
        }
    });
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&content)
        .build();
    root.append(&scroll);
    window.set_content(Some(&root));
    let state = nav.state.clone();
    let weak_window = window.downgrade();
    let render = move || {
        let state = state.borrow();
        notice.set_text(&if state.config.session_history != *policy.borrow() {
            fl!(LANGUAGE_LOADER, "history-settings-changed")
        } else {
            status(&state)
        });
        apply.set_sensitive(!state.report_history.busy && !state.report_history.settings_pending);
        retry.set_sensitive(!state.report_history.busy && !state.report_history.settings_pending);
        clear
            .set_sensitive(!state.report_history.busy && state.report_history.load_error.is_none());
    };
    render();
    glib::timeout_add_local(Duration::from_millis(250), move || {
        if !alive.get() || weak_window.upgrade().is_none() {
            return glib::ControlFlow::Break;
        }
        render();
        glib::ControlFlow::Continue
    });
    window.present();
}
