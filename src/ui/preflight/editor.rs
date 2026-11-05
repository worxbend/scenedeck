//! Edit saved requirements without changing OBS or adopting its collection implicitly.
use super::*;
use crate::domain::{operator_action::ActionTarget, preflight::SavedPreflight};
use std::cell::Cell;

#[derive(Default)]
struct Choices {
    scenes: Vec<(ActionTarget, CheckButton)>,
    microphones: Vec<(Option<ActionTarget>, CheckButton)>,
}
fn check(label: &str, name: &str) -> CheckButton {
    let button = CheckButton::new();
    button.set_child(Some(&text(label)));
    button.set_widget_name(name);
    button
}
fn add_field(body: &GtkBox, title: &str, widget: &impl IsA<gtk4::Widget>) {
    let label = text(title);
    label.add_css_class("heading");
    body.append(&label);
    body.append(widget);
    widget
        .as_ref()
        .update_property(&[gtk4::accessible::Property::Label(title)]);
}

pub(super) fn show(parent: &gtk4::Window, nav: &NavigationContext, owner: &Owner) {
    if !owner.matches(&nav.state.borrow()) || !nav.state.borrow().workspace.can_control() {
        return;
    }
    let nav = nav.scoped();
    let (original, observed_collection, scenes, audio) = {
        let state = nav.state.borrow();
        (
            owner.plan(&state),
            state.scene_collections.current.clone().unwrap_or_default(),
            state
                .workspace
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.scenes.clone())
                .unwrap_or_default(),
            state
                .workspace
                .audio
                .iter()
                .filter(|audio| {
                    state
                        .workspace
                        .input_catalog
                        .as_ref()
                        .and_then(|result| result.as_ref().ok())
                        .is_some_and(|inputs| {
                            crate::domain::operator_input::resolve_input(
                                inputs,
                                &ActionTarget {
                                    name: audio.id.clone(),
                                    uuid: audio.uuid.clone(),
                                },
                            )
                            .is_some()
                        })
                })
                .map(|audio| ActionTarget {
                    name: audio.id.clone(),
                    uuid: audio.uuid.clone(),
                })
                .collect::<Vec<_>>(),
        )
    };
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .default_width(540)
        .default_height(620)
        .title(fl!(LANGUAGE_LOADER, "preflight-configure"))
        .build();
    window.set_widget_name("preflight-editor");
    let outer = GtkBox::new(Orientation::Vertical, 8);
    outer.append(&adw::HeaderBar::new());
    let body = GtkBox::new(Orientation::Vertical, 12);
    body.set_margin_start(16);
    body.set_margin_end(16);
    body.set_margin_top(8);
    body.set_margin_bottom(16);
    let expected = gtk4::Entry::builder()
        .text(if original.expected_collection.is_empty() {
            &observed_collection
        } else {
            &original.expected_collection
        })
        .build();
    expected.set_widget_name("preflight-expected");
    add_field(
        &body,
        &fl!(LANGUAGE_LOADER, "preflight-expected"),
        &expected,
    );
    body.append(&text(&fl!(
        LANGUAGE_LOADER,
        "preflight-targets-description"
    )));
    let scene_rows = GtkBox::new(Orientation::Vertical, 6);
    add_field(
        &body,
        &fl!(LANGUAGE_LOADER, "preflight-scenes"),
        &scene_rows,
    );
    let microphone_enabled = check(
        &fl!(LANGUAGE_LOADER, "preflight-check-mic"),
        "preflight-check-mic",
    );
    microphone_enabled.set_active(original.check_microphone);
    body.append(&microphone_enabled);
    let microphone_rows = GtkBox::new(Orientation::Vertical, 6);
    body.append(&microphone_rows);
    let choices = Rc::new(RefCell::new(Choices::default()));
    let populate: Rc<dyn Fn(&str)> = Rc::new({
        let scene_rows = scene_rows.downgrade();
        let microphone_rows = microphone_rows.downgrade();
        let choices = choices.clone();
        let original = original.clone();
        move |expected| {
            let (Some(scene_rows), Some(microphone_rows)) =
                (scene_rows.upgrade(), microphone_rows.upgrade())
            else {
                return;
            };
            super::super::clear_children(&scene_rows);
            super::super::clear_children(&microphone_rows);
            let mut choices = choices.borrow_mut();
            *choices = Choices::default();
            let preserve = expected == original.expected_collection;
            let observed = expected == observed_collection;
            let mut targets: Vec<_> = if observed {
                scenes
                    .iter()
                    .map(|scene| ActionTarget {
                        name: scene.id.clone(),
                        uuid: scene.uuid.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            if preserve {
                for target in &original.required_scenes {
                    if !targets
                        .iter()
                        .any(|other| target.matches(&other.name, other.uuid.as_deref()))
                    {
                        targets.push(target.clone());
                    }
                }
            }
            for (index, target) in targets.into_iter().enumerate() {
                let selected = preserve
                    && original
                        .required_scenes
                        .iter()
                        .any(|saved| saved.matches(&target.name, target.uuid.as_deref()));
                let button = check(&target.name, &format!("preflight-scene-{index}"));
                button.set_active(selected);
                scene_rows.append(&button);
                choices.scenes.push((target, button));
            }
            if choices.scenes.is_empty() {
                scene_rows.append(&text(&fl!(LANGUAGE_LOADER, "preflight-no-scene-choices")));
            }
            let none = check(
                &fl!(LANGUAGE_LOADER, "preflight-no-microphone"),
                "preflight-mic-none",
            );
            none.set_active(true);
            microphone_rows.append(&none);
            choices.microphones.push((None, none.clone()));
            let mut targets = if observed { audio.clone() } else { Vec::new() };
            if let Some(target) = original.microphone.as_ref().filter(|_| preserve) {
                if !targets
                    .iter()
                    .any(|other| target.matches(&other.name, other.uuid.as_deref()))
                {
                    targets.push(target.clone());
                }
            }
            for (index, target) in targets.into_iter().enumerate() {
                let button = check(&target.name, &format!("preflight-mic-{index}"));
                button.set_group(Some(&none));
                button.set_active(
                    preserve
                        && original.microphone.as_ref().is_some_and(|saved| {
                            saved.matches(&target.name, target.uuid.as_deref())
                        }),
                );
                microphone_rows.append(&button);
                choices.microphones.push((Some(target), button));
            }
        }
    });
    populate(expected.text().as_str());
    expected.connect_changed(move |entry| populate(entry.text().as_str()));
    let disk_enabled = check(
        &fl!(LANGUAGE_LOADER, "preflight-check-disk"),
        "preflight-check-disk",
    );
    disk_enabled.set_active(original.minimum_disk_mib.is_some());
    body.append(&disk_enabled);
    let disk = gtk4::Entry::builder()
        .text(original.minimum_disk_mib.unwrap_or(1024).to_string())
        .input_purpose(gtk4::InputPurpose::Digits)
        .build();
    disk.set_widget_name("preflight-minimum-disk");
    add_field(
        &body,
        &fl!(LANGUAGE_LOADER, "preflight-minimum-disk"),
        &disk,
    );
    let manual = gtk4::TextView::builder()
        .wrap_mode(gtk4::WrapMode::WordChar)
        .height_request(120)
        .build();
    manual.set_widget_name("preflight-manual-items");
    manual.buffer().set_text(&original.manual_items.join("\n"));
    add_field(
        &body,
        &fl!(LANGUAGE_LOADER, "preflight-manual-items"),
        &manual,
    );
    let review_before_output = check(
        &fl!(LANGUAGE_LOADER, "preflight-review-before-output"),
        "preflight-review-before-output",
    );
    review_before_output.set_active(original.review_before_output);
    body.append(&review_before_output);
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&body)
        .build();
    outer.append(&scroll);
    let notice = text("");
    notice.set_widget_name("preflight-save-notice");
    notice.set_margin_start(16);
    notice.set_margin_end(16);
    outer.append(&notice);
    let save = Button::with_label(&fl!(LANGUAGE_LOADER, "deck-save"));
    save.set_widget_name("preflight-save");
    save.set_margin_start(16);
    save.set_margin_end(16);
    save.set_margin_bottom(16);
    outer.append(&save);
    window.set_content(Some(&outer));
    let busy = Rc::new(Cell::new(false));
    let baseline = Rc::new(RefCell::new(original.clone()));
    save.connect_clicked({
        let nav = nav.clone();
        let owner = owner.clone();
        let window = window.downgrade();
        let notice = notice.downgrade();
        let busy = busy.clone();
        let baseline = baseline.clone();
        move |_| {
            if window.upgrade().is_none_or(|window| !window.is_visible())
                || busy.get()
                || !nav.is_current()
                || !owner.matches(&nav.state.borrow())
                || owner.plan(&nav.state.borrow()) != *baseline.borrow()
            {
                return;
            }
            let Some(notice) = notice.upgrade() else {
                return;
            };
            let expected_collection = expected.text().to_string();
            let minimum_disk_mib = if disk_enabled.is_active() {
                disk.text()
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value > 0)
            } else {
                None
            };
            let buffer = manual.buffer();
            let manual_items: Vec<_> = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect();
            if expected_collection.trim().is_empty()
                || (disk_enabled.is_active() && minimum_disk_mib.is_none())
                || manual_items
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != manual_items.len()
            {
                notice.set_text(&fl!(LANGUAGE_LOADER, "preflight-invalid-plan"));
                return;
            }
            let choices = choices.borrow();
            let plan = PreflightPlan {
                expected_collection,
                required_scenes: choices
                    .scenes
                    .iter()
                    .filter(|(_, check)| check.is_active())
                    .map(|(target, _)| target.clone())
                    .collect(),
                check_microphone: microphone_enabled.is_active(),
                microphone: choices
                    .microphones
                    .iter()
                    .find(|(_, check)| check.is_active())
                    .and_then(|(target, _)| target.clone()),
                minimum_disk_mib,
                manual_items,
                review_before_output: review_before_output.is_active(),
            };
            drop(choices);
            *baseline.borrow_mut() = plan.clone();
            busy.set(true);
            notice.set_text(&fl!(LANGUAGE_LOADER, "preflight-saving"));
            let owner = owner.clone();
            let busy = busy.clone();
            let notice = notice.downgrade();
            let window = window.clone();
            super::super::persist::persist_config_with(
                &nav,
                move |config| {
                    config.preflight_plans.retain(|saved| {
                        saved.connection != owner.connection || saved.endpoint != owner.endpoint
                    });
                    config.preflight_plans.push(SavedPreflight {
                        connection: owner.connection,
                        endpoint: owner.endpoint,
                        plan,
                    });
                },
                move |result, _| {
                    busy.set(false);
                    match result {
                        Ok(()) => {
                            if let Some(window) = window.upgrade() {
                                window.close();
                            }
                        }
                        Err(error) => {
                            if let Some(notice) = notice.upgrade() {
                                notice.set_text(&fl!(
                                    LANGUAGE_LOADER,
                                    "preflight-save-failed",
                                    error = error.to_string()
                                ));
                            }
                        }
                    }
                },
            );
        }
    });
    window.present();
    glib::timeout_add_local(Duration::from_millis(250), {
        let window = window.downgrade();
        let save = save.downgrade();
        let nav = nav.clone();
        let owner = owner.clone();
        move || {
            if window.upgrade().is_none_or(|window| !window.is_visible()) {
                return glib::ControlFlow::Break;
            }
            let current = nav.is_current()
                && owner.matches(&nav.state.borrow())
                && owner.plan(&nav.state.borrow()) == *baseline.borrow();
            if let Some(save) = save.upgrade() {
                save.set_sensitive(!busy.get() && current);
            }
            if !current {
                notice.set_text(&fl!(LANGUAGE_LOADER, "preflight-editor-retired"));
            }
            glib::ControlFlow::Continue
        }
    });
}
