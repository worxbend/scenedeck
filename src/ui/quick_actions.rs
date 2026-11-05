//! Workspace-owned Live controls with explicit intent and observed state.

use super::{
    action_catalog::{self, Entry},
    navigation::NavigationContext,
    persist::persist_registry,
};
use crate::{domain::operator_action::OperatorPin, infra::i18n::LANGUAGE_LOADER};
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, MenuButton, Orientation};
use i18n_embed_fl::fl;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
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

struct Card {
    button: Button,
    menu: MenuButton,
    label: Label,
    state: Label,
    displayed: Rc<RefCell<(Entry, Option<String>)>>,
}

/// Rebuild only for changed pins/ownership; ordinary observations keep widgets.
pub(crate) fn build(nav: NavigationContext) -> (GtkBox, Rc<dyn Fn()>) {
    let root = GtkBox::new(Orientation::Vertical, 6);
    root.set_widget_name("quick-actions");
    let header = GtkBox::new(Orientation::Horizontal, 8);
    let title = text(&fl!(LANGUAGE_LOADER, "quick-title"));
    title.add_css_class("heading");
    title.set_hexpand(true);
    header.append(&title);
    let add = Button::from_icon_name("list-add-symbolic");
    add.set_widget_name("quick-add");
    let caption = fl!(LANGUAGE_LOADER, "quick-add");
    add.set_tooltip_text(Some(&caption));
    add.update_property(&[gtk4::accessible::Property::Label(&caption)]);
    header.append(&add);
    root.append(&header);
    let notice = text("");
    notice.set_widget_name("quick-notice");
    notice.add_css_class("dim-label");
    root.append(&notice);
    let rows = GtkBox::new(Orientation::Vertical, 0);
    root.append(&rows);
    let signature = RefCell::new(None);
    let cards: RefCell<Vec<Card>> = RefCell::new(Vec::new());
    let focus_after_move = Rc::new(Cell::new(None));
    let refresh: Rc<dyn Fn()> = Rc::new({
        let rows = rows.downgrade();
        let add = add.downgrade();
        let notice = notice.downgrade();
        let nav = nav.clone();
        let focus_after_move = focus_after_move.clone();
        move || {
            let (Some(rows), Some(add), Some(notice)) =
                (rows.upgrade(), add.upgrade(), notice.upgrade())
            else {
                return;
            };
            let current = nav.is_current() && nav.registry_is_current();
            add.set_sensitive(current);
            let (owner, pins, entries, statuses) = {
                let state = nav.state.borrow();
                let pins = state.registry.quick_actions.clone();
                let entries: Vec<_> = pins
                    .iter()
                    .map(|pin| action_catalog::binding_entry(&state, &pin.action))
                    .collect();
                let statuses: Vec<_> = pins
                    .iter()
                    .map(|pin| {
                        if !pin.enabled {
                            fl!(LANGUAGE_LOADER, "quick-disabled")
                        } else {
                            fl!(
                                LANGUAGE_LOADER,
                                "quick-observed",
                                state = action_catalog::observed_label(&state, &pin.action)
                                    .unwrap_or_else(|| fl!(
                                        LANGUAGE_LOADER,
                                        "output-state-unknown"
                                    ))
                            )
                        }
                    })
                    .collect();
                (
                    (
                        state.workspace.epoch,
                        state.show_registry.generation,
                        state.show_registry.status.clone(),
                        state.workflow_scope(),
                        state.config.language,
                        current,
                    ),
                    pins,
                    entries,
                    statuses,
                )
            };
            notice.set_visible(!current || pins.is_empty());
            notice.set_label(&if !current {
                fl!(LANGUAGE_LOADER, "palette-scope-changed")
            } else {
                fl!(LANGUAGE_LOADER, "quick-empty")
            });
            let changed = signature.borrow().as_ref() != Some(&(owner.clone(), pins.clone()));
            if changed {
                cards.borrow_mut().clear();
                super::rebuild_page(&rows, |rows| {
                    let flow = gtk4::FlowBox::builder()
                        .selection_mode(gtk4::SelectionMode::None)
                        .min_children_per_line(1)
                        .max_children_per_line(4)
                        .column_spacing(8)
                        .row_spacing(8)
                        .homogeneous(true)
                        .build();
                    rows.append(&flow);
                    for (index, entry) in entries.iter().enumerate() {
                        let card = card(
                            &nav.scoped().registry_scoped(),
                            index,
                            &pins,
                            entry.clone(),
                            &notice,
                            focus_after_move.clone(),
                        );
                        let row = GtkBox::new(Orientation::Horizontal, 0);
                        row.append(&card.button);
                        row.append(&card.menu);
                        super::insert_compact_flow_child(&flow, &row);
                        cards.borrow_mut().push(card);
                    }
                });
                *signature.borrow_mut() = Some((owner, pins.clone()));
                if let Some(index) = focus_after_move.take() {
                    if let Some(card) = cards.borrow().get(index) {
                        let menu = card.menu.downgrade();
                        glib::idle_add_local_once(move || {
                            if let Some(menu) = menu.upgrade() {
                                menu.grab_focus();
                            }
                        });
                    }
                }
            }
            for (((card, pin), entry), status) in
                cards.borrow().iter().zip(&pins).zip(entries).zip(statuses)
            {
                card.label.set_label(&entry.label);
                card.state.set_label(&status);
                card.button
                    .set_sensitive(current && pin.enabled && entry.available);
                card.menu.set_sensitive(current);
                card.button
                    .update_property(&[gtk4::accessible::Property::Label(&format!(
                        "{} · {status}",
                        entry.label
                    ))]);
                card.button.set_tooltip_text(
                    (!entry.available)
                        .then(|| fl!(LANGUAGE_LOADER, "palette-unavailable"))
                        .as_deref(),
                );
                let observed = action_catalog::observed_label(&nav.state.borrow(), &pin.action);
                *card.displayed.borrow_mut() = (entry, observed);
            }
        }
    });
    add.connect_clicked({
        let nav = nav.clone();
        move |button| {
            if nav.registry_is_current() {
                if let Some(parent) = button.root().and_downcast::<gtk4::Window>() {
                    picker(&parent, &nav, None);
                }
            }
        }
    });
    refresh();
    (root, refresh)
}

fn card(
    nav: &NavigationContext,
    index: usize,
    pins: &[OperatorPin],
    entry: Entry,
    notice: &Label,
    focus: Rc<Cell<Option<usize>>>,
) -> Card {
    let button = Button::new();
    button.set_hexpand(true);
    button.set_widget_name(&format!("quick-action-{index}"));
    let content = GtkBox::new(Orientation::Vertical, 3);
    let label = text(&entry.label);
    let state = text("");
    state.add_css_class("caption");
    state.set_widget_name(&format!("quick-state-{index}"));
    content.append(&label);
    content.append(&state);
    button.set_child(Some(&content));
    let displayed = Rc::new(RefCell::new((entry, None)));
    button.connect_clicked({
        let nav = nav.clone();
        let expected = pins.to_vec();
        let displayed = displayed.clone();
        let notice = notice.downgrade();
        move |_| {
            if !nav.is_current() || !nav.registry_is_current() {
                return;
            }
            let shown = displayed.borrow().clone();
            let admitted = {
                let state = nav.state.borrow();
                let current = action_catalog::binding_entry(&state, &expected[index].action);
                state.registry.quick_actions == expected
                    && expected[index].enabled
                    && current.available
                    && current.label == shown.0.label
                    && action_catalog::observed_label(&state, &expected[index].action) == shown.1
            };
            if admitted && action_catalog::execute(&nav, &shown.0.action) {
                return;
            }
            if let Some(notice) = notice.upgrade() {
                notice.set_label(&fl!(LANGUAGE_LOADER, "palette-unavailable"));
                notice.set_visible(true);
            }
            super::workspace::refresh(&nav);
        }
    });
    let menu = MenuButton::builder()
        .icon_name("view-more-symbolic")
        .valign(gtk4::Align::Start)
        .build();
    menu.set_widget_name(&format!("quick-menu-{index}"));
    let caption = fl!(LANGUAGE_LOADER, "quick-manage");
    menu.set_tooltip_text(Some(&caption));
    menu.update_property(&[gtk4::accessible::Property::Label(&caption)]);
    let popover = gtk4::Popover::new();
    let controls = GtkBox::new(Orientation::Vertical, 6);
    let enabled = gtk4::CheckButton::with_label(&fl!(LANGUAGE_LOADER, "binding-enabled"));
    enabled.set_widget_name(&format!("quick-enabled-{index}"));
    enabled.set_active(pins[index].enabled);
    enabled.connect_toggled({
        let nav = nav.clone();
        let expected = pins.to_vec();
        move |toggle| {
            let value = toggle.is_active();
            persist_registry(&nav, "quick actions", |registry| {
                if registry.quick_actions != expected
                    || registry.quick_actions[index].enabled == value
                {
                    return false;
                }
                registry.quick_actions[index].enabled = value;
                true
            });
        }
    });
    controls.append(&enabled);
    for (key, name, operation) in [
        ("quick-edit", "edit", 0),
        ("quick-up", "up", -1),
        ("quick-down", "down", 1),
        ("quick-remove", "remove", 2),
    ] {
        let title = match key {
            "quick-edit" => fl!(LANGUAGE_LOADER, "quick-edit"),
            "quick-up" => fl!(LANGUAGE_LOADER, "quick-up"),
            "quick-down" => fl!(LANGUAGE_LOADER, "quick-down"),
            _ => fl!(LANGUAGE_LOADER, "quick-remove"),
        };
        let control = Button::new();
        control.set_child(Some(&text(&title)));
        control.set_widget_name(&format!("quick-{name}-{index}"));
        control.set_sensitive(
            !(operation == -1 && index == 0 || operation == 1 && index + 1 == pins.len()),
        );
        control.connect_clicked({
            let nav = nav.clone();
            let expected = pins.to_vec();
            let popover = popover.downgrade();
            let focus = focus.clone();
            move |button| {
                if !nav.registry_is_current()
                    || nav.state.borrow().registry.quick_actions != expected
                {
                    return;
                }
                let parent = button.root().and_downcast::<gtk4::Window>();
                if let Some(popover) = popover.upgrade() {
                    popover.popdown();
                }
                if operation == 0 {
                    if let Some(parent) = parent {
                        picker(&parent, &nav, Some((index, expected.clone())));
                    }
                    return;
                }
                let destination = if operation == -1 {
                    index.checked_sub(1)
                } else {
                    Some(index + 1)
                };
                let changed = persist_registry(&nav, "quick actions", |registry| {
                    if registry.quick_actions != expected {
                        return false;
                    }
                    if operation == 2 {
                        registry.quick_actions.remove(index);
                        return true;
                    }
                    let Some(destination) =
                        destination.filter(|destination| *destination < expected.len())
                    else {
                        return false;
                    };
                    registry.quick_actions.swap(index, destination);
                    true
                });
                if changed {
                    focus.set(Some(if operation == 2 {
                        index.min(expected.len().saturating_sub(2))
                    } else {
                        destination.unwrap()
                    }));
                }
            }
        });
        controls.append(&control);
    }
    popover.set_child(Some(&controls));
    menu.set_popover(Some(&popover));
    Card {
        button,
        menu,
        label,
        state,
        displayed,
    }
}

fn picker(
    parent: &gtk4::Window,
    nav: &NavigationContext,
    original: Option<(usize, Vec<OperatorPin>)>,
) {
    let nav = nav.scoped().registry_scoped();
    let dialog = adw::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .heading(fl!(LANGUAGE_LOADER, "quick-add"))
        .body(fl!(LANGUAGE_LOADER, "quick-description"))
        .build();
    dialog.set_widget_name("quick-picker");
    dialog.add_responses(&[
        ("cancel", &fl!(LANGUAGE_LOADER, "deck-cancel")),
        ("save", &fl!(LANGUAGE_LOADER, "deck-save")),
    ]);
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("save"));
    let action = original.as_ref().map(|(index, pins)| &pins[*index].action);
    let (content, picker) = super::operator_picker::ActionPicker::new(&nav, action);
    dialog.set_extra_child(Some(&content));
    let refresh: Rc<dyn Fn()> = Rc::new({
        let picker = picker.clone();
        let dialog = dialog.downgrade();
        let nav = nav.clone();
        let original = original.clone();
        move || {
            picker.refresh();
            if let Some(dialog) = dialog.upgrade() {
                dialog.set_response_enabled(
                    "save",
                    nav.is_current()
                        && nav.registry_is_current()
                        && picker.selected().is_some()
                        && original.as_ref().is_none_or(|(_, expected)| {
                            nav.state.borrow().registry.quick_actions == *expected
                        }),
                );
            }
        }
    });
    picker.connect_changed({
        let refresh = refresh.clone();
        move || refresh()
    });
    dialog.connect_response(None, {
        let nav = nav.clone();
        move |_, response| {
            if response != "save" || !nav.is_current() || !nav.registry_is_current() {
                return;
            }
            let Some(action) = picker.selected() else {
                return;
            };
            persist_registry(&nav, "quick actions", |registry| {
                if let Some((index, expected)) = &original {
                    if registry.quick_actions != *expected {
                        return false;
                    }
                    registry.quick_actions[*index].action = action;
                } else {
                    registry.quick_actions.push(OperatorPin {
                        action,
                        enabled: true,
                    });
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
            if dialog.upgrade().is_none_or(|dialog| !dialog.is_visible()) {
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
    use crate::{
        domain::{
            control::ControlAction,
            operator_action::{ActionTarget, OperatorAction},
        },
        ui::{
            action_catalog::CatalogAction,
            test_support::{native_test, navigation, pump, pump_until},
        },
    };
    use std::time::Instant;

    fn find(root: &impl IsA<gtk4::Widget>, name: &str) -> gtk4::Widget {
        fn search(root: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
            if root.widget_name() == name {
                return Some(root.clone());
            }
            let mut child = root.first_child();
            while let Some(widget) = child {
                child = widget.next_sibling();
                if let Some(found) = search(&widget, name) {
                    return Some(found);
                }
            }
            None
        }
        search(root.as_ref(), name).unwrap_or_else(|| panic!("missing {name}"))
    }
    fn dialog() -> adw::MessageDialog {
        gtk4::Window::list_toplevels()
            .into_iter()
            .find_map(|window| {
                window
                    .downcast::<adw::MessageDialog>()
                    .ok()
                    .filter(|dialog| dialog.widget_name() == "quick-picker" && dialog.is_visible())
            })
            .unwrap()
    }
    fn choose(dialog: &adw::MessageDialog, nav: &NavigationContext, action: &OperatorAction) {
        let index = action_catalog::binding_entries(&nav.state.borrow())
            .iter()
            .position(|entry| entry.action == CatalogAction::Operator(action.clone()))
            .unwrap();
        find(dialog, "binding-action")
            .downcast::<gtk4::DropDown>()
            .unwrap()
            .set_selected(index as u32);
    }
    fn mute() -> OperatorAction {
        OperatorAction::SetMuted {
            target: ActionTarget {
                name: "Microphone".into(),
                uuid: Some("input-1".into()),
            },
            muted: true,
        }
    }
    fn idle(nav: &NavigationContext) {
        let mut state = nav.state.borrow_mut();
        state.workspace.pending = false;
        state.workspace.loading = false;
        state.workspace.active_action = None;
        state.workspace.dirty = false;
        state.workspace.error = None;
        state.workspace.observed_at = Some(Instant::now());
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_quick_actions_pin_persist_observe_and_retain_widgets_during_reads() {
        native_test("ui::quick_actions::tests::native_quick_actions_pin_persist_observe_and_retain_widgets_during_reads", || {
            let (nav, _stack, _runtime, _events) = navigation();
            *nav.state.borrow_mut() = action_catalog::tests::connected();
            let (root, refresh) = build(nav.clone()); let window = adw::Window::new(); window.set_content(Some(&root)); window.present();
            for action in [mute(), OperatorAction::SaveReplay] {
                find(&root, "quick-add").downcast::<Button>().unwrap().emit_clicked();
                let dialog = dialog(); choose(&dialog, &nav, &action); assert!(dialog.is_response_enabled("save")); dialog.response("save"); dialog.close(); refresh();
            }
            let target = nav.state.borrow().show_registry.target.clone();
            assert!(pump_until(Duration::from_secs(2), || target.read().is_ok_and(|registry| registry.quick_actions == nav.state.borrow().registry.quick_actions)));
            let button = find(&root, "quick-action-0").downcast::<Button>().unwrap();
            let status = find(&root, "quick-state-0").downcast::<Label>().unwrap();
            assert!(status.label().contains(&fl!(LANGUAGE_LOADER, "quick-unmuted")));
            assert!(button.is_sensitive()); button.grab_focus(); button.emit_clicked();
            assert_eq!(nav.state.borrow().workspace.active_action, Some(ControlAction::SetInputMuted { input: "Microphone".into(), muted: true }));
            refresh(); assert!(!button.is_sensitive());
            assert!(status.label().contains(&fl!(LANGUAGE_LOADER, "output-state-unknown")), "pending commands are not observed outcomes");
            idle(&nav); nav.state.borrow_mut().workspace.audio[0].muted = true; refresh();
            assert_eq!(find(&root, "quick-action-0"), button.clone().upcast::<gtk4::Widget>());
            assert!(status.label().contains(&fl!(LANGUAGE_LOADER, "quick-muted")));
            let menu = find(&root, "quick-menu-0").downcast::<MenuButton>().unwrap(); menu.popup(); pump(Duration::from_millis(30));
            nav.state.borrow_mut().workspace.audio[0].muted = false; refresh();
            assert!(menu.popover().unwrap().is_visible(), "ordinary readback preserves an open control menu");
            nav.state.borrow_mut().workspace.input_catalog = Some(Err("lost inventory".into())); refresh(); assert!(!button.is_sensitive());
            assert!(status.label().contains(&fl!(LANGUAGE_LOADER, "output-state-unknown")));
            nav.state.borrow_mut().show_registry.generation += 1;
            button.emit_clicked(); assert!(nav.state.borrow().workspace.active_action.is_none());
            let weak = root.downgrade(); window.close(); drop((root, button, status, menu, refresh, window)); pump(Duration::from_millis(300)); assert!(weak.upgrade().is_none());
        });
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_quick_actions_reorder_edit_disable_remove_and_fence_old_callbacks() {
        native_test("ui::quick_actions::tests::native_quick_actions_reorder_edit_disable_remove_and_fence_old_callbacks", || {
            let (nav, _stack, _runtime, _events) = navigation(); *nav.state.borrow_mut() = action_catalog::tests::connected();
            nav.state.borrow_mut().registry.quick_actions = vec![OperatorPin { action: mute(), enabled: true }, OperatorPin { action: OperatorAction::SaveReplay, enabled: false }];
            gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
            let (root, refresh) = build(nav.clone()); let window = adw::Window::builder().default_width(360).default_height(600).build(); window.set_content(Some(&root)); window.present(); pump(Duration::from_millis(80));
            assert!(window.width() <= 360, "quick actions must wrap at large text sizes: {}", window.width());
            let original = nav.state.borrow().registry.quick_actions.clone();
            let stale_remove = find(&root, "quick-remove-0").downcast::<Button>().unwrap();
            let stale_action = find(&root, "quick-action-0").downcast::<Button>().unwrap();
            find(&root, "quick-menu-0").downcast::<MenuButton>().unwrap().popup(); pump(Duration::from_millis(20));
            find(&root, "quick-down-0").downcast::<Button>().unwrap().emit_clicked(); refresh(); pump(Duration::from_millis(30));
            assert_eq!(nav.state.borrow().registry.quick_actions, vec![original[1].clone(), original[0].clone()]);
            let menu = find(&root, "quick-menu-1");
            let focused = gtk4::prelude::GtkWindowExt::focus(&window).unwrap();
            assert!(focused == menu || focused.is_ancestor(&menu), "focus should follow the moved menu, got {}", focused.widget_name());
            stale_remove.emit_clicked(); stale_action.emit_clicked(); assert_eq!(nav.state.borrow().registry.quick_actions.len(), 2); assert!(nav.state.borrow().workspace.active_action.is_none());
            idle(&nav); refresh();
            assert!(!find(&root, "quick-action-0").is_sensitive());
            find(&root, "quick-enabled-0").downcast::<gtk4::CheckButton>().unwrap().set_active(true); refresh();
            assert!(nav.state.borrow().registry.quick_actions[0].enabled);
            find(&root, "quick-edit-0").downcast::<Button>().unwrap().emit_clicked();
            let edit = dialog(); choose(&edit, &nav, &mute()); edit.response("save"); edit.close(); refresh();
            assert_eq!(nav.state.borrow().registry.quick_actions[0].action, mute());
            find(&root, "quick-remove-1").downcast::<Button>().unwrap().emit_clicked(); refresh();
            assert_eq!(nav.state.borrow().registry.quick_actions.len(), 1);
            find(&root, "quick-up-0").downcast::<Button>().unwrap().emit_clicked(); assert_eq!(nav.state.borrow().registry.quick_actions.len(), 1);
            find(&root, "quick-edit-0").downcast::<Button>().unwrap().emit_clicked(); let retired = dialog();
            let before = nav.state.borrow().registry.quick_actions.clone(); nav.state.borrow_mut().show_registry.generation += 1;
            retired.response("save"); retired.close(); assert_eq!(nav.state.borrow().registry.quick_actions, before);
            window.close();
        });
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_quick_toggle_rejects_changed_displayed_state_and_waits_for_readback() {
        native_test("ui::quick_actions::tests::native_quick_toggle_rejects_changed_displayed_state_and_waits_for_readback", || {
            let (nav, _stack, _runtime, _events) = navigation(); *nav.state.borrow_mut() = action_catalog::tests::connected();
            nav.state.borrow_mut().registry.quick_actions = vec![OperatorPin { action: OperatorAction::ToggleMuted { target: ActionTarget { name: "Microphone".into(), uuid: Some("input-1".into()) } }, enabled: true }];
            let (root, refresh) = build(nav.clone()); let window = adw::Window::new(); window.set_content(Some(&root)); window.present();
            let button = find(&root, "quick-action-0").downcast::<Button>().unwrap();
            nav.state.borrow_mut().workspace.audio[0].muted = true;
            button.emit_clicked(); assert!(nav.state.borrow().workspace.active_action.is_none(), "changed displayed state needs refresh before applying the opposite value");
            idle(&nav); refresh(); button.emit_clicked();
            assert_eq!(nav.state.borrow().workspace.active_action, Some(ControlAction::SetInputMuted { input: "Microphone".into(), muted: false }));
            let sequence = nav.state.borrow().workspace.sequence;
            button.emit_clicked(); assert_eq!(nav.state.borrow().workspace.sequence, sequence, "pending mutations cannot be toggled again");
            refresh(); assert!(!button.is_sensitive());
            idle(&nav); nav.state.borrow_mut().workspace.audio[0].muted = false; refresh(); button.emit_clicked();
            assert_eq!(nav.state.borrow().workspace.active_action, Some(ControlAction::SetInputMuted { input: "Microphone".into(), muted: true }));
            window.close();
        });
    }
}
