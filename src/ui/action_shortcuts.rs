//! Application shortcut normalization, dispatch and scoped configuration UI.

use crate::controller::state::AppState;
use crate::domain::{
    action_shortcut::{ActionBinding, ActionShortcut, ShortcutConflict},
    hotkey::Modifiers,
};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::{action_catalog, navigation::NavigationContext, persist::persist_registry};
use adw::prelude::*;
use gdk::prelude::DisplayExtManual;
use gtk4::{gdk, Box as GtkBox, Button, Label, Orientation};
use i18n_embed_fl::fl;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

pub(super) fn chord(
    widget: &impl IsA<gtk4::Widget>,
    key: gdk::Key,
    code: u32,
    state: gdk::ModifierType,
) -> Option<ActionShortcut> {
    if state.intersects(gdk::ModifierType::HYPER_MASK | gdk::ModifierType::META_MASK) {
        return None;
    }
    // Keep shifted number-row keys aligned with the existing slot shortcuts.
    let digit = if code != 0 {
        widget.display().map_keycode(code).and_then(|keys| {
            keys.into_iter()
                .filter(|(map, _)| map.level() == 0)
                .find_map(|(_, key)| key.to_unicode().filter(char::is_ascii_digit))
        })
    } else {
        None
    };
    let key = digit
        .map(|c| c.to_string())
        .or_else(|| {
            key.to_lower()
                .to_unicode()
                .filter(|c| !c.is_control())
                .map(|c| {
                    if c == ' ' {
                        "space".into()
                    } else {
                        c.to_string()
                    }
                })
        })
        .or_else(|| key.name().map(|name| name.to_string()))?;
    Some(ActionShortcut {
        key,
        modifiers: Modifiers::new(
            state.contains(gdk::ModifierType::CONTROL_MASK),
            state.contains(gdk::ModifierType::ALT_MASK),
            state.contains(gdk::ModifierType::SHIFT_MASK),
            state.contains(gdk::ModifierType::SUPER_MASK),
        ),
    })
}

fn label(shortcut: &ActionShortcut) -> String {
    let mut text = String::new();
    for (active, prefix) in [
        (
            shortcut.modifiers.ctrl,
            fl!(LANGUAGE_LOADER, "hotkey-modifier-ctrl"),
        ),
        (
            shortcut.modifiers.alt,
            fl!(LANGUAGE_LOADER, "hotkey-modifier-alt"),
        ),
        (
            shortcut.modifiers.shift,
            fl!(LANGUAGE_LOADER, "hotkey-modifier-shift"),
        ),
        (
            shortcut.modifiers.logo,
            fl!(LANGUAGE_LOADER, "hotkey-modifier-super"),
        ),
    ] {
        if active {
            text.push_str(&prefix);
        }
    }
    text.push_str(&if shortcut.key == "space" {
        fl!(LANGUAGE_LOADER, "hotkey-leader-space")
    } else {
        shortcut.key.to_uppercase()
    });
    text
}

fn conflict(
    state: &AppState,
    shortcut: &ActionShortcut,
    original: Option<&ActionBinding>,
) -> Option<ShortcutConflict> {
    let mut skipped = false;
    shortcut.conflict(
        &state.config.hotkeys,
        state
            .registry
            .action_bindings
            .iter()
            .filter(|binding| {
                if !skipped && Some(*binding) == original {
                    skipped = true;
                    return false;
                }
                binding.enabled
            })
            .map(|binding| binding.shortcut.clone()),
    )
}

/// Conflicting bindings stay inactive; the existing application/slot keys win.
pub(super) fn matching(state: &AppState, shortcut: &ActionShortcut) -> Option<ActionBinding> {
    let mut matches = state
        .registry
        .action_bindings
        .iter()
        .filter(|binding| binding.enabled && binding.shortcut == *shortcut);
    let binding = matches.next()?;
    if matches.next().is_some() || conflict(state, shortcut, Some(binding)).is_some() {
        return None;
    }
    Some(binding.clone())
}

fn conflict_text(conflict: ShortcutConflict) -> String {
    match conflict {
        ShortcutConflict::Invalid => fl!(LANGUAGE_LOADER, "binding-invalid"),
        ShortcutConflict::Application => fl!(LANGUAGE_LOADER, "binding-reserved"),
        ShortcutConflict::SceneSlots => fl!(LANGUAGE_LOADER, "binding-slots-conflict"),
        ShortcutConflict::Duplicate => fl!(LANGUAGE_LOADER, "binding-duplicate"),
    }
}

pub(crate) fn settings_group(nav: &NavigationContext) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "binding-title"))
        .description(fl!(LANGUAGE_LOADER, "binding-description"))
        .build();
    let button = Button::with_label(&fl!(LANGUAGE_LOADER, "binding-manage"));
    button.set_widget_name("binding-manage");
    button.connect_clicked({
        let nav = nav.clone();
        move |button| {
            if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
                show(&parent, &nav);
            }
        }
    });
    group.add(&button);
    group
}

fn literal(text: &str) -> Label {
    Label::builder()
        .label(text)
        .use_markup(false)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .xalign(0.0)
        .build()
}

fn show(parent: &gtk4::Window, nav: &NavigationContext) {
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .title(fl!(LANGUAGE_LOADER, "binding-title"))
        .default_width(560)
        .default_height(480)
        .build();
    window.set_widget_name("binding-manager");
    let nav = nav.registry_scoped();
    let root = GtkBox::new(Orientation::Vertical, 8);
    root.append(&adw::HeaderBar::new());
    let notice = literal(&fl!(LANGUAGE_LOADER, "binding-description"));
    notice.set_margin_start(12);
    notice.set_margin_end(12);
    root.append(&notice);
    let add = Button::with_label(&fl!(LANGUAGE_LOADER, "binding-add"));
    add.set_widget_name("binding-add");
    root.append(&add);
    let rows = GtkBox::new(Orientation::Vertical, 8);
    rows.set_margin_start(12);
    rows.set_margin_end(12);
    rows.set_margin_bottom(12);
    let body = GtkBox::new(Orientation::Vertical, 8);
    body.append(&super::global_shortcuts::controls(&nav));
    body.append(&rows);
    root.append(
        &gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&body)
            .build(),
    );
    window.set_content(Some(&root));
    let rendered = RefCell::new(None);
    let refresh: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        let rows = rows.downgrade();
        let notice = notice.downgrade();
        let add = add.downgrade();
        let window = window.downgrade();
        move || {
            let (Some(rows), Some(notice), Some(add), Some(window)) = (
                rows.upgrade(),
                notice.upgrade(),
                add.upgrade(),
                window.upgrade(),
            ) else {
                return;
            };
            let current = nav.registry_is_current();
            add.set_sensitive(current);
            notice.set_label(&if current {
                fl!(LANGUAGE_LOADER, "binding-description")
            } else {
                fl!(LANGUAGE_LOADER, "palette-scope-changed")
            });
            let items: Vec<_> = {
                let state = nav.state.borrow();
                state
                    .registry
                    .action_bindings
                    .iter()
                    .map(|binding| {
                        let entry = action_catalog::binding_entry(&state, &binding.action);
                        (
                            binding.clone(),
                            entry.label,
                            entry.available,
                            conflict(&state, &binding.shortcut, Some(binding)),
                        )
                    })
                    .collect()
            };
            if *rendered.borrow() == Some((current, items.clone())) {
                return;
            }
            crate::ui::rebuild_page(&rows, |rows| {
                if items.is_empty() {
                    rows.append(&literal(&fl!(LANGUAGE_LOADER, "binding-empty")));
                }
                for (index, (binding, action_label, available, conflict)) in
                    items.iter().enumerate()
                {
                    let row = GtkBox::new(Orientation::Vertical, 6);
                    row.append(&literal(&format!(
                        "{} · {}",
                        label(&binding.shortcut),
                        action_label
                    )));
                    if let Some(conflict) = conflict {
                        row.append(&literal(&conflict_text(*conflict)));
                    } else if !available {
                        row.append(&literal(&fl!(LANGUAGE_LOADER, "palette-unavailable")));
                    }
                    let buttons = gtk4::FlowBox::builder()
                        .selection_mode(gtk4::SelectionMode::None)
                        .min_children_per_line(1)
                        .max_children_per_line(3)
                        .column_spacing(6)
                        .row_spacing(6)
                        .build();
                    let enabled =
                        gtk4::CheckButton::with_label(&fl!(LANGUAGE_LOADER, "binding-enabled"));
                    enabled.set_active(binding.enabled);
                    enabled.set_widget_name(&format!("binding-enabled-{index}"));
                    enabled.connect_toggled({
                        let nav = nav.clone();
                        let expected = binding.clone();
                        move |toggle| {
                            let value = toggle.is_active();
                            if value && conflict_for_enable(&nav, &expected) {
                                toggle.set_active(false);
                                return;
                            }
                            persist_registry(&nav, "action shortcuts", |registry| {
                                if let Some(binding) = registry
                                    .action_bindings
                                    .iter_mut()
                                    .find(|binding| **binding == expected)
                                {
                                    if binding.enabled == value {
                                        return false;
                                    }
                                    binding.enabled = value;
                                    true
                                } else {
                                    false
                                }
                            });
                        }
                    });
                    crate::ui::insert_compact_flow_child(&buttons, &enabled);
                    let edit = Button::with_label(&fl!(LANGUAGE_LOADER, "binding-edit"));
                    edit.set_widget_name(&format!("binding-edit-{index}"));
                    edit.connect_clicked({
                        let nav = nav.clone();
                        let window = window.downgrade();
                        let expected = binding.clone();
                        move |_| {
                            if nav.registry_is_current() {
                                if let Some(window) = window.upgrade() {
                                    editor(window.upcast_ref(), &nav, Some(expected.clone()));
                                }
                            }
                        }
                    });
                    crate::ui::insert_compact_flow_child(&buttons, &edit);
                    let remove = Button::from_icon_name("user-trash-symbolic");
                    remove.set_widget_name(&format!("binding-remove-{index}"));
                    let title = fl!(LANGUAGE_LOADER, "binding-remove");
                    remove.set_tooltip_text(Some(&title));
                    remove.update_property(&[gtk4::accessible::Property::Label(&title)]);
                    remove.connect_clicked({
                        let nav = nav.clone();
                        let expected = binding.clone();
                        move |_| {
                            persist_registry(&nav, "action shortcuts", |registry| {
                                if let Some(index) = registry
                                    .action_bindings
                                    .iter()
                                    .position(|binding| *binding == expected)
                                {
                                    registry.action_bindings.remove(index);
                                    true
                                } else {
                                    false
                                }
                            });
                        }
                    });
                    crate::ui::insert_compact_flow_child(&buttons, &remove);
                    row.append(&buttons);
                    row.set_sensitive(current);
                    rows.append(&row);
                }
            });
            *rendered.borrow_mut() = Some((current, items));
        }
    });
    add.connect_clicked({
        let nav = nav.clone();
        let window = window.downgrade();
        move |_| {
            if nav.registry_is_current() {
                if let Some(window) = window.upgrade() {
                    editor(window.upcast_ref(), &nav, None);
                }
            }
        }
    });
    refresh();
    window.present();
    glib::timeout_add_local(Duration::from_millis(250), {
        let window = window.downgrade();
        move || {
            if window.upgrade().is_none_or(|w| !w.is_visible()) {
                return glib::ControlFlow::Break;
            }
            refresh();
            glib::ControlFlow::Continue
        }
    });
}

fn conflict_for_enable(nav: &NavigationContext, binding: &ActionBinding) -> bool {
    conflict(&nav.state.borrow(), &binding.shortcut, Some(binding)).is_some()
}

fn editor(parent: &gtk4::Window, nav: &NavigationContext, original: Option<ActionBinding>) {
    let nav = nav.registry_scoped();
    let dialog = adw::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .heading(fl!(LANGUAGE_LOADER, "binding-edit"))
        .body(fl!(LANGUAGE_LOADER, "binding-record-help"))
        .build();
    dialog.set_widget_name("binding-editor");
    dialog.add_responses(&[
        ("cancel", &fl!(LANGUAGE_LOADER, "deck-cancel")),
        ("save", &fl!(LANGUAGE_LOADER, "deck-save")),
    ]);
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("save"));
    let content = GtkBox::new(Orientation::Vertical, 10);
    let (picker_widget, picker) = super::operator_picker::ActionPicker::new(
        &nav,
        original.as_ref().map(|binding| &binding.action),
    );
    content.append(&picker_widget);
    let record = Button::with_label(&fl!(LANGUAGE_LOADER, "binding-record"));
    record.set_widget_name("binding-record");
    content.append(&record);
    let status = literal("");
    status.set_widget_name("binding-status");
    content.append(&status);
    dialog.set_extra_child(Some(&content));
    let recording = Rc::new(Cell::new(false));
    let captured = Rc::new(RefCell::new(original.as_ref().map(|b| b.shortcut.clone())));
    let refresh: Rc<dyn Fn()> = Rc::new({
        let dialog = dialog.downgrade();
        let status = status.downgrade();
        let record = record.downgrade();
        let picker = picker.clone();
        let nav = nav.clone();
        let captured = captured.clone();
        let recording = recording.clone();
        let original = original.clone();
        move || {
            let (Some(dialog), Some(status), Some(record)) =
                (dialog.upgrade(), status.upgrade(), record.upgrade())
            else {
                return;
            };
            picker.refresh();
            let current = nav.registry_is_current();
            let error = captured
                .borrow()
                .as_ref()
                .and_then(|key| conflict(&nav.state.borrow(), key, original.as_ref()));
            let text = if !current {
                fl!(LANGUAGE_LOADER, "palette-scope-changed")
            } else if let Some(error) = error {
                conflict_text(error)
            } else {
                captured.borrow().as_ref().map(label).unwrap_or_default()
            };
            status.set_label(&text);
            record.set_label(&if recording.get() {
                fl!(LANGUAGE_LOADER, "binding-recording")
            } else {
                fl!(LANGUAGE_LOADER, "binding-record")
            });
            dialog.set_response_enabled(
                "save",
                current
                    && !recording.get()
                    && captured.borrow().is_some()
                    && error.is_none()
                    && picker.selected().is_some(),
            );
        }
    });
    record.connect_clicked({
        let recording = recording.clone();
        let refresh = refresh.clone();
        move |button| {
            recording.set(true);
            button.grab_focus();
            refresh();
        }
    });
    picker.connect_changed({
        let refresh = refresh.clone();
        move || refresh()
    });
    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    keys.connect_key_pressed({
        let recording = recording.clone();
        let captured = captured.clone();
        let refresh = refresh.clone();
        let dialog = dialog.downgrade();
        move |_, key, code, modifiers| {
            if !recording.get() {
                return glib::Propagation::Proceed;
            }
            if key == gdk::Key::Escape {
                recording.set(false);
                refresh();
                return glib::Propagation::Stop;
            }
            if matches!(
                key,
                gdk::Key::Control_L
                    | gdk::Key::Control_R
                    | gdk::Key::Alt_L
                    | gdk::Key::Alt_R
                    | gdk::Key::Shift_L
                    | gdk::Key::Shift_R
                    | gdk::Key::Super_L
                    | gdk::Key::Super_R
            ) {
                return glib::Propagation::Stop;
            }
            if let Some(dialog) = dialog.upgrade() {
                *captured.borrow_mut() = chord(&dialog, key, code, modifiers);
            }
            recording.set(false);
            refresh();
            glib::Propagation::Stop
        }
    });
    dialog.add_controller(keys);
    dialog.connect_response(None, {
        let nav = nav.clone();
        let picker = picker.clone();
        let captured = captured.clone();
        let original = original.clone();
        move |_, response| {
            if response != "save" || recording.get() || !nav.registry_is_current() {
                return;
            }
            let Some(shortcut) = captured.borrow().clone() else {
                return;
            };
            if conflict(&nav.state.borrow(), &shortcut, original.as_ref()).is_some() {
                return;
            }
            let Some(action) = picker.selected() else {
                return;
            };
            let binding = ActionBinding {
                shortcut,
                action: action.clone(),
                enabled: original.as_ref().is_none_or(|binding| binding.enabled),
            };
            persist_registry(&nav, "action shortcuts", |registry| {
                if let Some(original) = &original {
                    let Some(index) = registry
                        .action_bindings
                        .iter()
                        .position(|binding| binding == original)
                    else {
                        return false;
                    };
                    registry.action_bindings[index] = binding;
                } else {
                    registry.action_bindings.push(binding);
                }
                true
            });
        }
    });
    refresh();
    dialog.present();
    glib::timeout_add_local(Duration::from_millis(250), {
        let dialog = dialog.downgrade();
        move || {
            if dialog.upgrade().is_none_or(|d| !d.is_visible()) {
                return glib::ControlFlow::Break;
            }
            refresh();
            glib::ControlFlow::Continue
        }
    });
}

#[cfg(test)]
mod tests;
