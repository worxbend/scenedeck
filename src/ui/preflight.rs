//! Read-only readiness review. Expectations are independent of the active collection.
mod editor;
#[cfg(test)]
pub(crate) mod tests;

use super::navigation::NavigationContext;
use crate::{
    controller::{
        command::AppCommand,
        state::{AppState, ObsStatus},
    },
    domain::preflight::{CheckStatus, PreflightPlan, PreflightReview, PreflightSnapshot},
    infra::i18n::LANGUAGE_LOADER,
};
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, CheckButton, Label, Orientation};
use i18n_embed_fl::fl;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, PartialEq, Eq)]
struct Owner {
    connection: String,
    endpoint: String,
}
impl Owner {
    fn current(state: &AppState) -> Option<Self> {
        Some(Self {
            connection: state.connected_connection_id.clone()?,
            endpoint: state.connected_endpoint.clone()?,
        })
    }
    fn matches(&self, state: &AppState) -> bool {
        matches!(state.obs_status, ObsStatus::Connected { .. })
            && Self::current(state).as_ref() == Some(self)
    }
    fn plan(&self, state: &AppState) -> PreflightPlan {
        state
            .config
            .preflight_plan(&self.connection, &self.endpoint)
            .cloned()
            .unwrap_or_default()
    }
}
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
fn wrapped_button(caption: &str) -> Button {
    let button = Button::new();
    button.set_child(Some(&text(caption)));
    button.update_property(&[gtk4::accessible::Property::Label(caption)]);
    button
}
fn status(status: CheckStatus) -> String {
    match status {
        CheckStatus::Passed => fl!(LANGUAGE_LOADER, "preflight-passed"),
        CheckStatus::Failed => fl!(LANGUAGE_LOADER, "preflight-failed"),
        CheckStatus::Unknown => fl!(LANGUAGE_LOADER, "preflight-unknown"),
        CheckStatus::Skipped => fl!(LANGUAGE_LOADER, "preflight-skipped"),
    }
}

pub(crate) fn button(nav: NavigationContext) -> Button {
    let button = wrapped_button(&fl!(LANGUAGE_LOADER, "preflight-title"));
    button.set_widget_name("preflight-open");
    button.set_halign(gtk4::Align::Start);
    button.connect_clicked(move |button| {
        if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
            show(&parent, &nav, None);
        }
    });
    button
}

struct Run {
    epoch: u64,
    plan: PreflightPlan,
    review: Rc<RefCell<PreflightReview>>,
}

struct StartReview {
    nav: NavigationContext,
    owner: Owner,
    kind: crate::domain::output::OutputKind,
    continue_once: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl StartReview {
    fn valid(&self) -> bool {
        if !self.nav.is_current() {
            return false;
        }
        let state = self.nav.state.borrow();
        let output = match self.kind {
            crate::domain::output::OutputKind::Stream => &state.stream_status,
            crate::domain::output::OutputKind::Record => &state.record_status,
        };
        self.owner.matches(&state)
            && !output.active
            && output.state == crate::domain::output::OutputRunState::Inactive
    }
}

/// Intercept only explicitly opted-in starts; reviewing never queues an output.
pub(crate) fn review_before_start(
    parent: &Button,
    nav: &NavigationContext,
    kind: crate::domain::output::OutputKind,
    proceed: impl FnOnce() + 'static,
) -> bool {
    let owner = Owner::current(&nav.state.borrow());
    let Some(owner) = owner.filter(|owner| {
        owner.matches(&nav.state.borrow()) && owner.plan(&nav.state.borrow()).review_before_output
    }) else {
        return false;
    };
    if let Some(parent) = parent.root().and_downcast::<gtk4::Window>() {
        show(
            &parent,
            nav,
            Some(Rc::new(StartReview {
                nav: nav.scoped(),
                owner,
                kind,
                continue_once: RefCell::new(Some(Box::new(proceed))),
            })),
        );
    }
    true
}

fn show(parent: &gtk4::Window, nav: &NavigationContext, start: Option<Rc<StartReview>>) {
    let owner = Owner::current(&nav.state.borrow());
    let dialog = adw::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .heading(fl!(LANGUAGE_LOADER, "preflight-title"))
        .body(fl!(LANGUAGE_LOADER, "preflight-description"))
        .build();
    dialog.set_widget_name("preflight-review");
    dialog.add_response("close", &fl!(LANGUAGE_LOADER, "preflight-close"));
    dialog.set_close_response("close");
    dialog.set_default_response(Some("close"));
    if let Some(start) = &start {
        dialog.add_response(
            "continue",
            &fl!(
                LANGUAGE_LOADER,
                "preflight-continue",
                output = start.kind.label()
            ),
        );
        dialog.connect_response(None, {
            let start = start.clone();
            move |dialog, response| {
                let proceed = start.continue_once.borrow_mut().take();
                if response == "continue" && start.valid() {
                    dialog.close();
                    if let Some(proceed) = proceed {
                        proceed();
                    }
                }
            }
        });
    }
    let content = GtkBox::new(Orientation::Vertical, 12);
    if start.is_some() {
        content.append(&text(&fl!(
            LANGUAGE_LOADER,
            "preflight-continue-description"
        )));
    }
    let summary = text("");
    summary.set_widget_name("preflight-summary");
    content.append(&summary);
    let checks = text("");
    checks.set_widget_name("preflight-checks");
    content.append(&checks);
    let manual = GtkBox::new(Orientation::Vertical, 6);
    content.append(&manual);
    let refresh_button = wrapped_button(&fl!(LANGUAGE_LOADER, "preflight-refresh"));
    refresh_button.set_widget_name("preflight-refresh");
    content.append(&refresh_button);
    let reset = wrapped_button(&fl!(LANGUAGE_LOADER, "preflight-reset"));
    reset.set_widget_name("preflight-reset");
    content.append(&reset);
    let configure = wrapped_button(&fl!(LANGUAGE_LOADER, "preflight-configure"));
    configure.set_widget_name("preflight-configure");
    content.append(&configure);
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(260)
        .max_content_height(420)
        .propagate_natural_height(true)
        .child(&content)
        .build();
    dialog.set_extra_child(Some(&scroll));
    let run: Rc<RefCell<Option<Run>>> = Rc::new(RefCell::new(None));
    let render: Rc<dyn Fn()> = Rc::new({
        let dialog = dialog.downgrade();
        let start = start.clone();
        let configure = configure.downgrade();
        let summary = summary.downgrade();
        let checks = checks.downgrade();
        let manual = manual.downgrade();
        let refresh_button = refresh_button.downgrade();
        let nav = nav.clone();
        let owner = owner.clone();
        let run = run.clone();
        move || {
            let (Some(summary), Some(checks), Some(manual), Some(refresh_button)) = (
                summary.upgrade(),
                checks.upgrade(),
                manual.upgrade(),
                refresh_button.upgrade(),
            ) else {
                return;
            };
            if let (Some(dialog), Some(start)) = (dialog.upgrade(), &start) {
                dialog.set_response_enabled("continue", start.valid());
            }
            let state = nav.state.borrow();
            let connected = owner.as_ref().is_some_and(|owner| owner.matches(&state));
            refresh_button.set_sensitive(connected);
            if let Some(configure) = configure.upgrade() {
                configure.set_sensitive(connected && state.workspace.can_control());
            }
            let plan = owner
                .as_ref()
                .map(|owner| owner.plan(&state))
                .unwrap_or_default();
            let mut run = run.borrow_mut();
            if run
                .as_ref()
                .is_none_or(|run| run.epoch != state.workspace.epoch || run.plan != plan)
            {
                let review = Rc::new(RefCell::new(PreflightReview::new(&plan, Instant::now())));
                super::clear_children(&manual);
                for (index, item) in plan.manual_items.iter().enumerate() {
                    let check = CheckButton::new();
                    check.set_child(Some(&text(item)));
                    check.set_widget_name(&format!("preflight-manual-{index}"));
                    manual.append(&check);
                    let review = Rc::downgrade(&review);
                    let expected = plan.clone();
                    let epoch = state.workspace.epoch;
                    let owner = owner.clone();
                    let nav = nav.clone();
                    check.connect_toggled(move |check| {
                        let state = nav.state.borrow();
                        if state.workspace.epoch != epoch
                            || !owner.as_ref().is_some_and(|owner| {
                                owner.matches(&state) && owner.plan(&state) == expected
                            })
                        {
                            return;
                        }
                        if let Some(review) = review.upgrade() {
                            review.borrow_mut().checked[index] = check.is_active();
                        }
                    });
                }
                *run = Some(Run {
                    epoch: state.workspace.epoch,
                    plan: plan.clone(),
                    review,
                });
            }
            manual.set_sensitive(connected);
            let mut review = run.as_ref().unwrap().review.borrow_mut();
            let report = review.evaluate(
                &plan,
                PreflightSnapshot {
                    connected,
                    fresh: state.workspace.can_control(),
                    collection: state.scene_collections.current.as_deref(),
                    scenes: state
                        .workspace
                        .snapshot
                        .as_ref()
                        .map_or(&[], |snapshot| &snapshot.scenes),
                    inputs: state
                        .workspace
                        .input_catalog
                        .as_ref()
                        .and_then(|result| result.as_ref().ok())
                        .map(Vec::as_slice),
                    audio: &state.workspace.audio,
                    levels: &state.audio_levels,
                    disk: state
                        .obs_stats
                        .as_ref()
                        .and_then(|stats| stats.disk_space_mb)
                        .zip(state.obs_stats_observed_at),
                },
                Instant::now(),
            );
            summary.set_text(&if review.ready(&report) {
                fl!(LANGUAGE_LOADER, "preflight-ready")
            } else {
                fl!(LANGUAGE_LOADER, "preflight-attention")
            });
            let unknown = fl!(LANGUAGE_LOADER, "preflight-unknown");
            let mut lines = vec![fl!(
                LANGUAGE_LOADER,
                "preflight-collection-result",
                expected = if plan.expected_collection.is_empty() {
                    &unknown
                } else {
                    &plan.expected_collection
                },
                actual = state
                    .scene_collections
                    .current
                    .as_deref()
                    .filter(|_| connected)
                    .unwrap_or(&unknown),
                status = status(report.collection)
            )];
            for (target, check) in plan.required_scenes.iter().zip(report.scenes) {
                let name = if check == CheckStatus::Passed {
                    state
                        .workspace
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| {
                            snapshot
                                .scenes
                                .iter()
                                .find(|scene| target.matches(&scene.id, scene.uuid.as_deref()))
                        })
                        .map(|scene| scene.id.as_str())
                        .unwrap_or(&target.name)
                } else {
                    &target.name
                };
                lines.push(fl!(
                    LANGUAGE_LOADER,
                    "preflight-scene-result",
                    name = name,
                    status = status(check)
                ));
            }
            if plan.required_scenes.is_empty() {
                lines.push(fl!(LANGUAGE_LOADER, "preflight-no-scenes"));
            }
            let microphone = report
                .microphone_name
                .as_deref()
                .or_else(|| plan.microphone.as_ref().map(|target| target.name.as_str()))
                .unwrap_or(&unknown);
            lines.push(fl!(
                LANGUAGE_LOADER,
                "preflight-mic-result",
                name = microphone,
                presence = status(report.microphone[0]),
                unmuted = status(report.microphone[1]),
                signal = status(report.microphone[2])
            ));
            lines.push(fl!(
                LANGUAGE_LOADER,
                "preflight-disk-result",
                minimum = plan
                    .minimum_disk_mib
                    .map_or_else(|| "—".into(), |space| space.to_string()),
                status = status(report.disk)
            ));
            if matches!(report.disk, CheckStatus::Passed | CheckStatus::Failed) {
                if let Some(space) = state
                    .obs_stats
                    .as_ref()
                    .and_then(|stats| stats.disk_space_mb)
                {
                    lines.push(fl!(
                        LANGUAGE_LOADER,
                        "preflight-free-space",
                        space = format!("{space:.0}")
                    ));
                }
            }
            checks.set_text(&lines.join("\n\n"));
        }
    });
    reset.connect_clicked({
        let run = run.clone();
        let render = render.clone();
        move |_| {
            run.borrow_mut().take();
            render();
        }
    });
    refresh_button.connect_clicked({
        let owner = owner.clone();
        let nav = nav.clone();
        move |_| {
            if owner
                .as_ref()
                .is_some_and(|owner| owner.matches(&nav.state.borrow()))
            {
                super::workspace::refresh(&nav.scoped());
                nav.dispatch(AppCommand::RefreshStats);
            }
        }
    });
    configure.connect_clicked({
        let nav = nav.clone();
        let owner = owner.clone();
        move |button| {
            if let (Some(parent), Some(owner)) =
                (button.root().and_downcast::<gtk4::Window>(), owner.as_ref())
            {
                editor::show(&parent, &nav, owner);
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
