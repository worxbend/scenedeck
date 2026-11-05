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
mod tests {
    use super::*;
    use crate::domain::operator_action::OperatorAction;
    use crate::ui::action_catalog::CatalogAction;
    use crate::ui::test_support::{native_test, navigation, pump};

    fn binding(key: &str) -> ActionBinding {
        ActionBinding {
            shortcut: ActionShortcut {
                key: key.into(),
                modifiers: Modifiers::NONE,
            },
            action: OperatorAction::SaveReplay,
            enabled: true,
        }
    }

    #[test]
    fn only_unique_enabled_nonconflicting_bindings_can_match() {
        let mut state = action_catalog::tests::connected();
        let original = binding("F8");
        state.registry.action_bindings.push(original.clone());
        assert_eq!(matching(&state, &original.shortcut), Some(original.clone()));
        state.registry.action_bindings.push(original.clone());
        assert!(matching(&state, &original.shortcut).is_none());
        assert_eq!(
            conflict(&state, &original.shortcut, Some(&original)),
            Some(ShortcutConflict::Duplicate)
        );
        state.registry.action_bindings.pop();
        state.registry.action_bindings[0].enabled = false;
        assert!(matching(&state, &original.shortcut).is_none());
        state.registry.action_bindings = vec![ActionBinding {
            shortcut: ActionShortcut {
                key: "1".into(),
                modifiers: Modifiers::new(true, false, false, false),
            },
            ..original
        }];
        assert!(matching(&state, &state.registry.action_bindings[0].shortcut).is_none());
    }

    fn find(widget: &gtk4::Widget, name: &str) -> gtk4::Widget {
        fn search(widget: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
            if widget.widget_name() == name {
                return Some(widget.clone());
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                child = widget.next_sibling();
                if let Some(found) = search(&widget, name) {
                    return Some(found);
                }
            }
            None
        }
        search(widget, name).unwrap_or_else(|| panic!("missing {name}"))
    }

    fn dialog() -> adw::MessageDialog {
        gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|w| {
                w.downcast::<adw::MessageDialog>()
                    .ok()
                    .filter(|d| d.widget_name() == "binding-editor" && d.is_visible())
            })
            .unwrap()
    }

    fn key_controller(window: &impl IsA<gtk4::Widget>) -> gtk4::EventControllerKey {
        let controllers = window.observe_controllers();
        (0..controllers.n_items())
            .filter_map(|i| controllers.item(i))
            .find_map(|object| object.downcast().ok())
            .unwrap()
    }

    fn record(dialog: &adw::MessageDialog, key: gdk::Key, modifiers: gdk::ModifierType) {
        find(dialog.upcast_ref(), "binding-record")
            .downcast::<Button>()
            .unwrap()
            .emit_clicked();
        assert!(
            key_controller(dialog).emit_by_name::<bool>("key-pressed", &[&key, &0u32, &modifiers])
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_binding_editor_records_conflicts_persists_and_rejects_retired_scopes() {
        native_test("ui::action_shortcuts::tests::native_binding_editor_records_conflicts_persists_and_rejects_retired_scopes", || {
            let (nav, _stack, _runtime, _events) = navigation();
            *nav.state.borrow_mut() = action_catalog::tests::connected();
            let parent = adw::Window::new(); parent.present();
            editor(parent.upcast_ref(), &nav, None);
            let first = dialog();
            record(&first, gdk::Key::k, gdk::ModifierType::CONTROL_MASK);
            assert!(!first.is_response_enabled("save"));
            record(&first, gdk::Key::F8, gdk::ModifierType::empty());
            assert!(first.is_response_enabled("save"));
            first.response("save"); first.close();
            assert_eq!(nav.state.borrow().registry.action_bindings.len(), 1);
            let saved = nav.state.borrow().registry.action_bindings[0].clone();
            assert_eq!(saved.shortcut.key, "F8");
            assert!(saved.enabled);
            editor(parent.upcast_ref(), &nav, None);
            let second = dialog();
            record(&second, gdk::Key::F8, gdk::ModifierType::empty());
            assert!(!second.is_response_enabled("save"));
            second.response("save"); second.close();
            assert_eq!(nav.state.borrow().registry.action_bindings.len(), 1, "forced response must revalidate conflicts");
            editor(parent.upcast_ref(), &nav, Some(saved.clone()));
            let third = dialog();
            record(&third, gdk::Key::F9, gdk::ModifierType::empty());
            assert!(third.is_response_enabled("save"));
            nav.state.borrow_mut().show_registry.generation += 1;
            third.response("save"); third.close();
            assert_eq!(nav.state.borrow().registry.action_bindings.as_slice(), std::slice::from_ref(&saved));
            let target = nav.state.borrow().show_registry.target.clone();
            assert!(crate::ui::test_support::pump_until(Duration::from_secs(2), || target.read().is_ok_and(|r| r.action_bindings == [saved.clone()])));
            let weak = third.downgrade();
            drop((first, second, third));
            pump(Duration::from_millis(280));
            assert!(weak.upgrade().is_none());
            parent.close();
        });
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_binding_manager_edit_remove_and_large_text_layout() {
        native_test(
            "ui::action_shortcuts::tests::native_binding_manager_edit_remove_and_large_text_layout",
            || {
                let (nav, _stack, _runtime, _events) = navigation();
                *nav.state.borrow_mut() = action_catalog::tests::connected();
                nav.state
                    .borrow_mut()
                    .registry
                    .action_bindings
                    .push(binding("F8"));
                gtk4::Settings::default()
                    .unwrap()
                    .set_gtk_font_name(Some("Sans 22"));
                let parent = adw::Window::new();
                parent.present();
                show(parent.upcast_ref(), &nav);
                let manager = gtk4::Window::list_toplevels()
                    .into_iter()
                    .find_map(|w| {
                        w.downcast::<adw::Window>()
                            .ok()
                            .filter(|w| w.widget_name() == "binding-manager")
                    })
                    .unwrap();
                manager.set_default_size(360, 480);
                pump(Duration::from_millis(100));
                assert!(
                    manager.width() <= 360,
                    "manager too wide: {}",
                    manager.width()
                );
                find(manager.upcast_ref(), "binding-edit-0")
                    .downcast::<Button>()
                    .unwrap()
                    .emit_clicked();
                let edit = dialog();
                record(&edit, gdk::Key::F9, gdk::ModifierType::empty());
                edit.response("save");
                edit.close();
                pump(Duration::from_millis(280));
                assert_eq!(
                    nav.state.borrow().registry.action_bindings[0].shortcut.key,
                    "F9"
                );
                let stale_remove = find(manager.upcast_ref(), "binding-remove-0")
                    .downcast::<Button>()
                    .unwrap();
                stale_remove.emit_clicked();
                pump(Duration::from_millis(280));
                assert!(nav.state.borrow().registry.action_bindings.is_empty());
                nav.state
                    .borrow_mut()
                    .registry
                    .action_bindings
                    .push(binding("F10"));
                stale_remove.emit_clicked();
                assert_eq!(
                    nav.state.borrow().registry.action_bindings[0].shortcut.key,
                    "F10"
                );
                let weak = manager.downgrade();
                manager.close();
                drop((manager, edit, stale_remove));
                pump(Duration::from_millis(280));
                assert!(weak.upgrade().is_none());
                parent.close();
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_custom_keys_work_across_pages_suppress_typing_and_repeats() {
        native_test("ui::action_shortcuts::tests::native_custom_keys_work_across_pages_suppress_typing_and_repeats", || {
            let (nav, _stack, _runtime, _events) = navigation();
            *nav.state.borrow_mut() = action_catalog::tests::connected();
            let scene = action_catalog::entries(&nav.state.borrow()).into_iter().find_map(|entry| match entry.action { CatalogAction::Operator(action @ OperatorAction::Scene { .. }) => Some(action), _ => None }).unwrap();
            nav.state.borrow_mut().registry.action_bindings.push(ActionBinding { action: scene, ..binding("F8") });
            nav.state.borrow_mut().current_page = crate::controller::state::Page::Settings;
            nav.state.borrow_mut().registry.scene_filter.query = "hide everything".into();
            let window = adw::ApplicationWindow::builder().build();
            let root = GtkBox::new(Orientation::Vertical, 8); let button = Button::with_label("Focus"); let entry = gtk4::Entry::new(); root.append(&button); root.append(&entry); window.set_content(Some(&root));
            let live = Rc::new(crate::ui::pages::live::build(nav.clone())); let toast = adw::ToastOverlay::new();
            crate::ui::hotkeys::install(&window, &nav, live.clone(), &toast);
            window.present(); pump(Duration::from_millis(70)); button.grab_focus();
            let keys = key_controller(&window);
            let press = || keys.emit_by_name::<bool>("key-pressed", &[&gdk::Key::F8, &0u32, &gdk::ModifierType::empty()]);
            let release = || keys.emit_by_name::<()>("key-released", &[&gdk::Key::F8, &0u32, &gdk::ModifierType::empty()]);
            assert!(press());
            let sequence = nav.state.borrow().workspace.sequence;
            assert!(nav.state.borrow().workspace.active_action.is_some());
            assert!(press()); assert_eq!(nav.state.borrow().workspace.sequence, sequence, "repeat cannot dispatch again");
            release(); entry.grab_focus(); assert!(!press()); assert_eq!(nav.state.borrow().workspace.sequence, sequence);
            button.grab_focus(); release();
            nav.state.borrow_mut().registry.action_bindings[0].enabled = false;
            assert!(!press());
            let lower = chord(&window, gdk::Key::M, 0, gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK).unwrap();
            assert_eq!(lower.key, "m"); assert!(lower.modifiers.shift);
            assert!(chord(&window, gdk::Key::m, 0, gdk::ModifierType::META_MASK).is_none());
            window.close();
        });
    }
}
