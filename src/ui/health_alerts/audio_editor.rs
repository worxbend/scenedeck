//! Workspace-owned opt-in rules; unavailable targets remain explicitly selected.
use super::*;
use crate::domain::{
    health::AudioRule, operator_action::ActionTarget, operator_input::resolve_input,
};
use gtk4::CheckButton;

struct Row {
    target: ActionTarget,
    title: Label,
    include: CheckButton,
    enabled: CheckButton,
    silence: CheckButton,
    clipping: CheckButton,
}
fn check(caption: &str, name: &str, active: bool) -> CheckButton {
    let check = CheckButton::new();
    check.set_child(Some(&text(caption)));
    check.update_property(&[gtk4::accessible::Property::Label(caption)]);
    check.set_widget_name(name);
    check.set_active(active);
    check
}
fn row(body: &GtkBox, target: ActionTarget, rule: Option<&AudioRule>, index: usize) -> Row {
    let group = GtkBox::new(Orientation::Vertical, 6);
    let title = text(&target.name);
    title.add_css_class("heading");
    group.append(&title);
    let include = check(
        &fl!(LANGUAGE_LOADER, "health-audio-select"),
        &format!("health-audio-select-{index}"),
        rule.is_some(),
    );
    group.append(&include);
    let options = GtkBox::new(Orientation::Vertical, 6);
    options.set_margin_start(16);
    let enabled = check(
        &fl!(LANGUAGE_LOADER, "binding-enabled"),
        &format!("health-audio-enabled-{index}"),
        rule.is_some_and(|rule| rule.enabled),
    );
    let silence = check(
        &fl!(LANGUAGE_LOADER, "health-audio-check-silence"),
        &format!("health-audio-silence-{index}"),
        rule.is_none_or(|rule| rule.silence),
    );
    let clipping = check(
        &fl!(LANGUAGE_LOADER, "health-audio-check-clipping"),
        &format!("health-audio-clipping-{index}"),
        rule.is_none_or(|rule| rule.clipping),
    );
    for check in [&enabled, &silence, &clipping] {
        options.append(check);
    }
    options.set_sensitive(include.is_active());
    include.connect_toggled({
        let options = options.downgrade();
        move |check| {
            if let Some(options) = options.upgrade() {
                options.set_sensitive(check.is_active());
            }
        }
    });
    group.append(&options);
    body.append(&group);
    Row {
        target,
        title,
        include,
        enabled,
        silence,
        clipping,
    }
}
fn rules(rows: &[Row]) -> Vec<AudioRule> {
    rows.iter()
        .filter(|row| row.include.is_active())
        .map(|row| AudioRule {
            target: row.target.clone(),
            enabled: row.enabled.is_active(),
            silence: row.silence.is_active(),
            clipping: row.clipping.is_active(),
        })
        .collect()
}
fn invalid(rules: &[AudioRule]) -> bool {
    rules.iter().any(|rule| {
        (rule.enabled && !rule.silence && !rule.clipping)
            || rules
                .iter()
                .filter(|other| {
                    rule.target
                        .matches(&other.target.name, other.target.uuid.as_deref())
                })
                .count()
                != 1
    })
}
fn current(nav: &NavigationContext, baseline: &[AudioRule]) -> bool {
    nav.is_current()
        && nav.registry_is_current()
        && nav.state.borrow().registry.audio_alerts == baseline
}

pub(super) fn show(parent: &gtk4::Window, nav: &NavigationContext) {
    if !nav.registry_is_current()
        || nav.state.borrow().desired_registry_target().as_ref()
            != Some(&nav.state.borrow().show_registry.target)
    {
        return;
    }
    let nav = nav.scoped().registry_scoped();
    let baseline = Rc::new(RefCell::new(
        nav.state.borrow().registry.audio_alerts.clone(),
    ));
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .default_width(500)
        .default_height(620)
        .title(fl!(LANGUAGE_LOADER, "health-audio-settings"))
        .build();
    window.set_widget_name("health-audio-editor");
    let outer = GtkBox::new(Orientation::Vertical, 8);
    outer.append(&adw::HeaderBar::new());
    let body = GtkBox::new(Orientation::Vertical, 16);
    body.set_margin_start(16);
    body.set_margin_end(16);
    body.set_margin_top(8);
    body.set_margin_bottom(16);
    body.append(&text(&fl!(LANGUAGE_LOADER, "health-audio-policy")));
    let refresh = Button::new();
    let refresh_label = fl!(LANGUAGE_LOADER, "preflight-refresh");
    refresh.set_child(Some(&text(&refresh_label)));
    refresh.update_property(&[gtk4::accessible::Property::Label(&refresh_label)]);
    refresh.set_widget_name("health-audio-refresh");
    body.append(&refresh);
    refresh.connect_clicked({
        let nav = nav.clone();
        move |_| {
            super::super::workspace::refresh(&nav);
        }
    });
    let rows = Rc::new(RefCell::new(Vec::<Row>::new()));
    for rule in baseline.borrow().iter() {
        let mut rows = rows.borrow_mut();
        let next = row(&body, rule.target.clone(), Some(rule), rows.len());
        rows.push(next);
    }
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&body)
        .build();
    outer.append(&scroll);
    let notice = text("");
    notice.set_widget_name("health-audio-save-notice");
    notice.set_margin_start(16);
    notice.set_margin_end(16);
    outer.append(&notice);
    let save = Button::with_label(&fl!(LANGUAGE_LOADER, "deck-save"));
    save.set_widget_name("health-audio-save");
    outer.append(&save);
    window.set_content(Some(&outer));
    save.connect_clicked({
        let window = window.downgrade();
        let nav = nav.clone();
        let baseline = baseline.clone();
        let rows = rows.clone();
        let notice = notice.downgrade();
        move |_| {
            if window.upgrade().is_none_or(|window| !window.is_visible())
                || !current(&nav, &baseline.borrow())
            {
                return;
            }
            let rules = rules(&rows.borrow());
            if invalid(&rules) {
                if let Some(notice) = notice.upgrade() {
                    notice.set_text(&fl!(LANGUAGE_LOADER, "health-audio-invalid"));
                }
                return;
            }
            if super::super::persist::persist_registry(&nav, "audio alert rules", |registry| {
                if registry.audio_alerts == rules {
                    return false;
                }
                registry.audio_alerts = rules.clone();
                true
            }) {
                *baseline.borrow_mut() = rules;
            }
        }
    });
    let render = {
        let body = body.downgrade();
        let save = save.downgrade();
        let nav = nav.clone();
        let notice = notice.downgrade();
        move || {
            let (Some(body), Some(save), Some(notice)) =
                (body.upgrade(), save.upgrade(), notice.upgrade())
            else {
                return;
            };
            let valid = current(&nav, &baseline.borrow());
            save.set_sensitive(valid);
            refresh.set_sensitive(valid);
            if !valid {
                notice.set_text(&fl!(LANGUAGE_LOADER, "health-audio-retired"));
                return;
            }
            let state = nav.state.borrow();
            let inputs = state
                .workspace
                .can_control()
                .then_some(())
                .and(state.workspace.input_catalog.as_ref())
                .and_then(|result| result.as_ref().ok());
            let mut rows = rows.borrow_mut();
            if let Some(inputs) = inputs {
                for audio in &state.workspace.audio {
                    let target = ActionTarget {
                        name: audio.id.clone(),
                        uuid: audio.uuid.clone(),
                    };
                    if resolve_input(inputs, &target).is_some()
                        && !rows
                            .iter()
                            .any(|row| row.target.matches(&target.name, target.uuid.as_deref()))
                    {
                        let next = row(&body, target, None, rows.len());
                        rows.push(next);
                    }
                }
            }
            for row in rows.iter() {
                let input = inputs.and_then(|inputs| resolve_input(inputs, &row.target));
                row.title.set_text(
                    &input
                        .map(|input| input.target.name.clone())
                        .unwrap_or_else(|| {
                            format!(
                                "{} — {}",
                                row.target.name,
                                fl!(LANGUAGE_LOADER, "health-unobserved")
                            )
                        }),
                );
            }
            use super::super::persist::{registry_save_status, RegistrySaveStatus};
            let draft = rules(&rows);
            notice.set_text(&match registry_save_status() {
                RegistrySaveStatus::Saved if invalid(&draft) => {
                    fl!(LANGUAGE_LOADER, "health-audio-invalid")
                }
                RegistrySaveStatus::Saved if draft != *baseline.borrow() => {
                    fl!(LANGUAGE_LOADER, "health-audio-unsaved")
                }
                RegistrySaveStatus::Saved => fl!(LANGUAGE_LOADER, "health-audio-saved"),
                RegistrySaveStatus::Saving => fl!(LANGUAGE_LOADER, "preflight-saving"),
                RegistrySaveStatus::Failed(error) => {
                    fl!(LANGUAGE_LOADER, "preflight-save-failed", error = error)
                }
            });
        }
    };
    render();
    window.present();
    super::super::workspace::refresh(&nav);
    glib::timeout_add_local(Duration::from_millis(250), {
        let window = window.downgrade();
        move || {
            if window.upgrade().is_none_or(|window| !window.is_visible()) {
                return glib::ControlFlow::Break;
            }
            render();
            glib::ControlFlow::Continue
        }
    });
}

#[cfg(test)]
mod tests;
