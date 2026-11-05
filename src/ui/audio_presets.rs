//! Workspace-scoped capture and review of selected input settings.

use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, CheckButton, Label, Orientation};
use i18n_embed_fl::fl;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    controller::state::AppState,
    domain::{
        audio_preset::{
            valid_name, AudioPreset, PlannedInput, PresetGain, PresetInput, PresetPlan, PresetRun,
            PRESET_LIMIT,
        },
        operator_action::ActionTarget,
        operator_input::resolve_input,
    },
    infra::i18n::LANGUAGE_LOADER,
    ui::navigation::NavigationContext,
};

fn text(value: &str) -> Label {
    Label::builder()
        .label(value)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .xalign(0.0)
        .hexpand(true)
        .build()
}
fn button(value: &str, name: &str) -> Button {
    let button = Button::new();
    button.set_child(Some(&text(value)));
    button.set_widget_name(name);
    button.update_property(&[gtk4::accessible::Property::Label(value)]);
    button
}

fn snapshot(state: &AppState) -> Option<Vec<PresetInput>> {
    if !state.workspace.can_control() {
        return None;
    }
    let catalog = state.workspace.input_catalog.as_ref()?.as_ref().ok()?;
    state
        .workspace
        .audio
        .iter()
        .map(|audio| {
            let target = ActionTarget {
                name: audio.id.clone(),
                uuid: audio.uuid.clone(),
            };
            let target = resolve_input(catalog, &target)?.target.clone();
            Some(PresetInput {
                target,
                muted: audio.muted,
                gain: PresetGain::try_from(audio.volume_mul).ok()?,
            })
        })
        .collect()
}

fn resolve<'a>(inputs: &'a [PresetInput], target: &ActionTarget) -> Option<&'a PresetInput> {
    let mut matches = inputs
        .iter()
        .filter(|input| target.matches(&input.target.name, input.target.uuid.as_deref()));
    let input = matches.next()?;
    matches.next().is_none().then_some(input)
}
fn value(input: &PresetInput) -> String {
    let mute = if input.muted {
        fl!(LANGUAGE_LOADER, "quick-muted")
    } else {
        fl!(LANGUAGE_LOADER, "quick-unmuted")
    };
    let gain = f64::from(input.gain);
    let db = if gain == 0.0 {
        "−∞".into()
    } else {
        format!("{:.2}", 20.0 * gain.log10())
    };
    format!("{mute} · {db} dB")
}

struct Row {
    target: ActionTarget,
    selected: CheckButton,
    label: Label,
}
struct Editor {
    nav: NavigationContext,
    window: glib::WeakRef<adw::Window>,
    baseline: RefCell<BTreeMap<String, AudioPreset>>,
    selected: RefCell<Option<String>>,
    invalid: Cell<bool>,
    picker: GtkBox,
    name: gtk4::Entry,
    inputs: GtkBox,
    rows: RefCell<Vec<Row>>,
    preview: Label,
    notice: Label,
    save: Button,
    rename: Button,
    delete: Button,
    review: RefCell<Option<PresetPlan>>,
    review_button: Button,
    apply: Button,
    cancel: Button,
    results: Label,
    apply_status: Label,
}

fn capable(state: &AppState) -> bool {
    [
        "GetInputList",
        "GetInputMute",
        "GetInputVolume",
        "SetInputMute",
        "SetInputVolume",
        "GetSceneCollectionList",
    ]
    .iter()
    .all(|request| {
        state
            .available_requests
            .iter()
            .any(|available| available == request)
    })
}
fn review_matches(plan: &PresetPlan, inputs: &[PresetInput]) -> bool {
    plan.valid()
        && plan.inputs.iter().all(
            |row| match (&row.before, resolve(inputs, &row.desired.target)) {
                (None, None) => true,
                (Some(before), Some(now)) => before.same_values(now),
                _ => false,
            },
        )
}

impl Editor {
    fn current(&self) -> bool {
        self.window
            .upgrade()
            .is_some_and(|window| window.is_visible())
            && self.nav.is_current()
            && self.nav.registry_is_current()
            && self.nav.state.borrow().desired_registry_target().as_ref()
                == Some(&self.nav.state.borrow().show_registry.target)
            && self.nav.state.borrow().registry.audio_presets == *self.baseline.borrow()
    }
    fn add_rows(&self, targets: impl IntoIterator<Item = ActionTarget>) {
        let mut rows = self.rows.borrow_mut();
        for target in targets {
            if rows
                .iter()
                .any(|row| row.target.matches(&target.name, target.uuid.as_deref()))
            {
                continue;
            }
            let selected = CheckButton::new();
            selected.set_widget_name(&format!("preset-input-{}", rows.len()));
            let label = text(&target.name);
            selected.set_child(Some(&label));
            self.inputs.append(&selected);
            rows.push(Row {
                target,
                selected,
                label,
            });
        }
    }
    fn choose(&self, name: Option<String>) {
        self.review.borrow_mut().take();
        self.invalid.set(false);
        let preset = name
            .as_ref()
            .and_then(|name| self.baseline.borrow().get(name).cloned());
        self.name.set_text(name.as_deref().unwrap_or(""));
        *self.selected.borrow_mut() = name;
        if let Some(preset) = &preset {
            self.add_rows(preset.inputs.iter().map(|input| input.target.clone()));
        }
        for row in self.rows.borrow().iter() {
            row.selected.set_active(
                preset
                    .as_ref()
                    .is_some_and(|preset| resolve(&preset.inputs, &row.target).is_some()),
            );
        }
        self.render();
    }
    fn picker(self: &Rc<Self>) {
        super::clear_children(&self.picker);
        for name in self.baseline.borrow().keys() {
            let select = button(name, "preset-select");
            let editor = Rc::downgrade(self);
            let name = name.clone();
            select.connect_clicked(move |_| {
                if let Some(editor) = editor.upgrade().filter(|editor| editor.current()) {
                    editor.choose(Some(name.clone()));
                }
            });
            self.picker.append(&select);
        }
    }
    fn replace(self: &Rc<Self>, next: BTreeMap<String, AudioPreset>, selected: Option<String>) {
        if super::persist::persist_registry(&self.nav, "audio presets", |registry| {
            if registry.audio_presets == next {
                return false;
            }
            registry.audio_presets = next.clone();
            true
        }) {
            *self.baseline.borrow_mut() = next;
            self.picker();
            self.choose(selected);
        }
    }
    fn save(self: &Rc<Self>) {
        if !self.current() {
            return;
        }
        let name = self.name.text().trim().to_string();
        let mut next = self.baseline.borrow().clone();
        let inputs = snapshot(&self.nav.state.borrow());
        let selected: Option<Vec<_>> = self
            .rows
            .borrow()
            .iter()
            .filter(|row| row.selected.is_active())
            .map(|row| resolve(inputs.as_deref()?, &row.target).cloned())
            .collect();
        let preset = selected.map(|inputs| AudioPreset { inputs });
        if !valid_name(&name)
            || preset.as_ref().is_none_or(|preset| !preset.valid())
            || (next.contains_key(&name) && self.selected.borrow().as_ref() != Some(&name))
            || (!next.contains_key(&name) && next.len() >= PRESET_LIMIT)
        {
            self.invalid.set(true);
            self.render();
            return;
        }
        next.insert(name.clone(), preset.unwrap());
        self.invalid.set(false);
        self.replace(next, Some(name));
    }
    fn rename(self: &Rc<Self>) {
        if !self.current() {
            return;
        }
        let Some(old) = self.selected.borrow().clone() else {
            return;
        };
        let name = self.name.text().trim().to_string();
        let mut next = self.baseline.borrow().clone();
        if !valid_name(&name) || next.contains_key(&name) {
            self.invalid.set(true);
            self.render();
            return;
        }
        let Some(preset) = next.remove(&old) else {
            return;
        };
        next.insert(name.clone(), preset);
        self.replace(next, Some(name));
    }
    fn delete(self: &Rc<Self>) {
        if !self.current() {
            return;
        }
        let Some(name) = self.selected.borrow().clone() else {
            return;
        };
        let mut next = self.baseline.borrow().clone();
        next.remove(&name);
        self.replace(next, None);
    }
    fn render(&self) {
        let current = self.current();
        let inputs = current
            .then(|| snapshot(&self.nav.state.borrow()))
            .flatten();
        let active = self
            .nav
            .state
            .borrow()
            .preset_run
            .as_ref()
            .is_some_and(PresetRun::active);
        let supported = capable(&self.nav.state.borrow());
        self.review_button.set_sensitive(
            current && supported && !active && inputs.is_some() && self.selected.borrow().is_some(),
        );
        self.apply.set_sensitive(
            current
                && supported
                && !active
                && self.review.borrow().as_ref().is_some_and(|plan| {
                    inputs
                        .as_deref()
                        .is_some_and(|inputs| review_matches(plan, inputs))
                        && plan.inputs.iter().any(|row| row.before.is_some())
                }),
        );
        self.cancel.set_sensitive(active && self.nav.is_current());
        self.apply_status.set_text(&if !current {
            fl!(LANGUAGE_LOADER, "preset-retired")
        } else if !supported {
            fl!(LANGUAGE_LOADER, "preset-unsupported")
        } else if active {
            fl!(LANGUAGE_LOADER, "preset-running")
        } else if inputs.is_none() {
            fl!(LANGUAGE_LOADER, "preset-refresh-needed")
        } else if self.review.borrow().is_none() {
            fl!(LANGUAGE_LOADER, "preset-review")
        } else if self.apply.is_sensitive() {
            fl!(LANGUAGE_LOADER, "preset-reviewed")
        } else {
            fl!(LANGUAGE_LOADER, "preset-changed")
        });
        self.results.set_text(
            &self
                .nav
                .state
                .borrow()
                .preset_run
                .as_ref()
                .map(|run| {
                    let rows = run
                        .plan
                        .inputs
                        .iter()
                        .zip(&run.outcomes)
                        .map(|(input, outcome)| {
                            format!("{}: {}", input.desired.target.name, outcome.description())
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    format!(
                        "{} · {}\n{}\n{rows}",
                        run.plan.name, run.plan.scope.collection, run.plan.scope.endpoint
                    )
                })
                .unwrap_or_default(),
        );
        if let Some(inputs) = &inputs {
            self.add_rows(inputs.iter().map(|input| input.target.clone()));
        }
        for row in self.rows.borrow().iter() {
            let input = inputs
                .as_deref()
                .and_then(|inputs| resolve(inputs, &row.target));
            let caption = input
                .map(|input| format!("{} · {}", input.target.name, value(input)))
                .unwrap_or_else(|| {
                    format!(
                        "{} · {}",
                        row.target.name,
                        fl!(LANGUAGE_LOADER, "preset-missing")
                    )
                });
            row.label.set_text(&caption);
            row.selected.set_sensitive(current);
            row.selected
                .update_property(&[gtk4::accessible::Property::Label(&caption)]);
        }
        self.save
            .set_sensitive(current && inputs.as_ref().is_some_and(|inputs| !inputs.is_empty()));
        let selected = self.selected.borrow();
        self.rename.set_sensitive(current && selected.is_some());
        self.delete.set_sensitive(current && selected.is_some());
        let replacing = selected.as_deref() == Some(self.name.text().trim());
        let caption = if replacing {
            fl!(LANGUAGE_LOADER, "preset-replace")
        } else {
            fl!(LANGUAGE_LOADER, "preset-save")
        };
        if self
            .save
            .child()
            .and_downcast::<Label>()
            .is_none_or(|label| label.text() != caption)
        {
            self.save.set_child(Some(&text(&caption)));
        }
        self.save
            .update_property(&[gtk4::accessible::Property::Label(&caption)]);
        self.preview.set_text(
            &selected
                .as_ref()
                .and_then(|name| self.baseline.borrow().get(name).cloned())
                .map(|preset| {
                    preset
                        .inputs
                        .iter()
                        .map(|saved| {
                            let reviewed = self.review.borrow().as_ref().and_then(|plan| {
                                plan.inputs
                                    .iter()
                                    .find(|row| {
                                        row.desired.target.matches(
                                            &saved.target.name,
                                            saved.target.uuid.as_deref(),
                                        )
                                    })
                                    .cloned()
                            });
                            let observed = if let Some(row) = &reviewed {
                                row.before.as_ref()
                            } else {
                                inputs
                                    .as_deref()
                                    .and_then(|inputs| resolve(inputs, &saved.target))
                            };
                            format!(
                                "{}\n{}\n{}",
                                observed
                                    .map(|input| input.target.name.as_str())
                                    .unwrap_or(&saved.target.name),
                                fl!(
                                    LANGUAGE_LOADER,
                                    "preset-current",
                                    value = observed
                                        .map(value)
                                        .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "preset-missing"))
                                ),
                                fl!(LANGUAGE_LOADER, "preset-saved", value = value(saved))
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n")
                })
                .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "preset-none")),
        );
        use super::persist::{registry_save_status, RegistrySaveStatus};
        if !current {
            self.notice
                .set_text(&fl!(LANGUAGE_LOADER, "preset-retired"));
        } else if inputs.is_none() {
            self.notice
                .set_text(&fl!(LANGUAGE_LOADER, "preset-refresh-needed"));
        } else {
            match registry_save_status() {
                RegistrySaveStatus::Saving => self
                    .notice
                    .set_text(&fl!(LANGUAGE_LOADER, "preflight-saving")),
                RegistrySaveStatus::Failed(error) => self.notice.set_text(&fl!(
                    LANGUAGE_LOADER,
                    "preflight-save-failed",
                    error = error
                )),
                RegistrySaveStatus::Saved => self.notice.set_text(&if self.invalid.get() {
                    fl!(LANGUAGE_LOADER, "preset-invalid")
                } else {
                    String::new()
                }),
            }
        }
    }
    fn review(&self) {
        if !self.current() || !capable(&self.nav.state.borrow()) {
            return;
        }
        let Some(name) = self.selected.borrow().clone() else {
            return;
        };
        let Some(preset) = self.baseline.borrow().get(&name).cloned() else {
            return;
        };
        let state = self.nav.state.borrow();
        if state.preset_run.as_ref().is_some_and(PresetRun::active) {
            return;
        }
        let Some(inputs) = snapshot(&state) else {
            return;
        };
        let plan = PresetPlan {
            id: 0,
            epoch: state.workspace.epoch,
            scope: state.workflow_scope(),
            name,
            reviewed: Instant::now(),
            inputs: preset
                .inputs
                .into_iter()
                .map(|desired| PlannedInput {
                    before: resolve(&inputs, &desired.target).cloned(),
                    desired,
                })
                .collect(),
        };
        drop(state);
        *self.review.borrow_mut() = Some(plan);
        self.render();
    }
    fn apply(&self) {
        if !self.current() || !capable(&self.nav.state.borrow()) {
            return;
        }
        let Some(plan) = self.review.borrow().clone() else {
            return;
        };
        if !snapshot(&self.nav.state.borrow()).is_some_and(|inputs| review_matches(&plan, &inputs))
            || !plan.inputs.iter().any(|row| row.before.is_some())
        {
            return;
        }
        let plan = self.nav.state.borrow_mut().start_preset(plan);
        if let Some(plan) = plan {
            self.review.borrow_mut().take();
            self.nav
                .dispatch(crate::controller::command::AppCommand::ApplyAudioPreset(
                    plan,
                ));
        }
        self.render();
    }
}

pub(super) fn show(parent: &gtk4::Window, nav: &NavigationContext) {
    let nav = nav.scoped().registry_scoped();
    let window = adw::Window::builder()
        .transient_for(parent)
        .destroy_with_parent(true)
        .modal(true)
        .default_width(540)
        .default_height(700)
        .title(fl!(LANGUAGE_LOADER, "preset-title"))
        .build();
    window.set_widget_name("audio-presets");
    let outer = GtkBox::new(Orientation::Vertical, 8);
    outer.append(&adw::HeaderBar::new());
    let body = GtkBox::new(Orientation::Vertical, 12);
    body.set_margin_start(16);
    body.set_margin_end(16);
    body.set_margin_top(8);
    body.set_margin_bottom(16);
    body.append(&text(&fl!(LANGUAGE_LOADER, "preset-shared")));
    let picker = GtkBox::new(Orientation::Vertical, 6);
    body.append(&picker);
    let new = button(&fl!(LANGUAGE_LOADER, "preset-new"), "preset-new");
    body.append(&new);
    let caption = fl!(LANGUAGE_LOADER, "preset-name");
    body.append(&text(&caption));
    let name = gtk4::Entry::builder().max_length(80).hexpand(true).build();
    name.set_widget_name("preset-name");
    name.update_property(&[gtk4::accessible::Property::Label(&caption)]);
    body.append(&name);
    let refresh = button(&fl!(LANGUAGE_LOADER, "preflight-refresh"), "preset-refresh");
    body.append(&refresh);
    let inputs = GtkBox::new(Orientation::Vertical, 8);
    body.append(&inputs);
    let save = button(&fl!(LANGUAGE_LOADER, "preset-save"), "preset-save");
    body.append(&save);
    let rename = button(&fl!(LANGUAGE_LOADER, "preset-rename"), "preset-rename");
    body.append(&rename);
    let delete = button(&fl!(LANGUAGE_LOADER, "preset-delete"), "preset-delete");
    body.append(&delete);
    let notice = text("");
    notice.set_widget_name("preset-notice");
    body.append(&notice);
    body.append(&text(&fl!(LANGUAGE_LOADER, "preset-preview")));
    let preview = text("");
    preview.set_widget_name("preset-preview");
    body.append(&preview);
    body.append(&text(&fl!(LANGUAGE_LOADER, "preset-apply-policy")));
    let apply_status = text("");
    apply_status.set_widget_name("preset-apply-status");
    body.append(&apply_status);
    let review_button = button(&fl!(LANGUAGE_LOADER, "preset-review"), "preset-review");
    body.append(&review_button);
    let apply = button(&fl!(LANGUAGE_LOADER, "preset-apply"), "preset-apply");
    body.append(&apply);
    let cancel = button(&fl!(LANGUAGE_LOADER, "preset-cancel"), "preset-cancel");
    body.append(&cancel);
    let results = text("");
    results.set_widget_name("preset-results");
    body.append(&results);
    outer.append(
        &gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&body)
            .build(),
    );
    window.set_content(Some(&outer));
    let baseline = nav.state.borrow().registry.audio_presets.clone();
    let editor = Rc::new(Editor {
        nav,
        window: window.downgrade(),
        baseline: RefCell::new(baseline),
        selected: RefCell::new(None),
        invalid: Cell::new(false),
        picker,
        name,
        inputs,
        rows: RefCell::new(Vec::new()),
        preview,
        notice,
        save: save.clone(),
        rename: rename.clone(),
        delete: delete.clone(),
        review: RefCell::new(None),
        review_button: review_button.clone(),
        apply: apply.clone(),
        cancel: cancel.clone(),
        results,
        apply_status,
    });
    review_button.connect_clicked({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade() {
                editor.review();
            }
        }
    });
    apply.connect_clicked({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade() {
                editor.apply();
            }
        }
    });
    cancel.connect_clicked({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade().filter(|editor| {
                editor.nav.is_current()
                    && editor
                        .window
                        .upgrade()
                        .is_some_and(|window| window.is_visible())
            }) {
                editor
                    .nav
                    .dispatch(crate::controller::command::AppCommand::CancelAudioPreset);
            }
        }
    });
    new.connect_clicked({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade().filter(|editor| editor.current()) {
                editor.choose(None);
            }
        }
    });
    save.connect_clicked({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade() {
                editor.save();
            }
        }
    });
    rename.connect_clicked({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade() {
                editor.rename();
            }
        }
    });
    delete.connect_clicked({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade() {
                editor.delete();
            }
        }
    });
    refresh.connect_clicked({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade().filter(|editor| editor.current()) {
                super::workspace::refresh(&editor.nav);
            }
        }
    });
    editor.name.connect_changed({
        let editor = Rc::downgrade(&editor);
        move |_| {
            if let Some(editor) = editor.upgrade() {
                editor.render();
            }
        }
    });
    window.present();
    editor.picker();
    editor.render();
    glib::timeout_add_local(Duration::from_millis(250), move || {
        if editor
            .window
            .upgrade()
            .is_none_or(|window| !window.is_visible())
        {
            return glib::ControlFlow::Break;
        }
        editor.render();
        glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests;
