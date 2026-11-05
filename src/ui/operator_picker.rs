//! Scoped action selection with independent, temporary source inspection.

use super::{
    action_catalog::{self, CatalogAction, Entry},
    navigation::NavigationContext,
};
use crate::{
    domain::operator_action::{ActionTarget, OperatorAction},
    infra::i18n::LANGUAGE_LOADER,
};
use adw::prelude::*;
use gtk4::{Box as GtkBox, Button, DropDown, Label, Orientation};
use i18n_embed_fl::fl;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(super) struct ActionPicker {
    nav: NavigationContext,
    action: glib::WeakRef<DropDown>,
    input: glib::WeakRef<DropDown>,
    notice: glib::WeakRef<Label>,
    retry: glib::WeakRef<Button>,
    choices: RefCell<Vec<Entry>>,
    inputs: RefCell<Vec<ActionTarget>>,
    inspection: Cell<Option<u64>>,
    updating: Cell<bool>,
}

fn label(text: &str) -> Label {
    Label::builder().label(text).wrap(true).xalign(0.0).build()
}

fn dropdown(name: &str, title: &str) -> DropDown {
    let dropdown = DropDown::builder()
        .enable_search(true)
        .hexpand(true)
        .build();
    dropdown.set_widget_name(name);
    dropdown.update_property(&[gtk4::accessible::Property::Label(title)]);
    let factory = gtk4::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        item.downcast_ref::<gtk4::ListItem>()
            .unwrap()
            .set_child(Some(&label("")));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
        if let (Some(value), Some(label)) = (
            item.item().and_downcast::<gtk4::StringObject>(),
            item.child().and_downcast::<Label>(),
        ) {
            label.set_label(&value.string());
        }
    });
    dropdown.set_factory(Some(&factory));
    dropdown.set_list_factory(Some(&factory));
    dropdown
}

fn popup_open(widget: &impl IsA<gtk4::Widget>) -> bool {
    let mut child = widget.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        if widget.is::<gtk4::Popover>() && widget.is_visible() || popup_open(&widget) {
            return true;
        }
    }
    false
}

impl ActionPicker {
    pub fn new(nav: &NavigationContext, original: Option<&OperatorAction>) -> (GtkBox, Rc<Self>) {
        let root = GtkBox::new(Orientation::Vertical, 8);
        let input_title = fl!(LANGUAGE_LOADER, "action-inspect-input");
        root.append(&label(&input_title));
        let input = dropdown("action-inspect-input", &input_title);
        root.append(&input);
        let notice = label(&fl!(LANGUAGE_LOADER, "action-inspect-help"));
        notice.set_widget_name("action-inspect-status");
        notice.add_css_class("dim-label");
        root.append(&notice);
        let retry = Button::new();
        retry.set_child(Some(&label(&fl!(LANGUAGE_LOADER, "control-refresh"))));
        retry.set_widget_name("action-inspect-refresh");
        root.append(&retry);
        let title = fl!(LANGUAGE_LOADER, "binding-action");
        root.append(&label(&title));
        let action = dropdown("binding-action", &title);
        root.append(&action);
        let picker = Rc::new(Self {
            nav: nav.scoped().registry_scoped(),
            action: action.downgrade(),
            input: input.downgrade(),
            notice: notice.downgrade(),
            retry: retry.downgrade(),
            choices: RefCell::new(Vec::new()),
            inputs: RefCell::new(Vec::new()),
            inspection: Cell::new(None),
            updating: Cell::new(false),
        });
        if let Some(original) = original {
            picker
                .choices
                .borrow_mut()
                .push(action_catalog::binding_entry(&nav.state.borrow(), original));
            action.set_selected(0);
        }
        // Seed the original selection before the first model is installed.
        picker.refresh_with_selection(original.cloned());
        input.connect_selected_notify({
            let picker = picker.clone();
            move |input| {
                if picker.updating.get() {
                    return;
                }
                picker.release();
                if !picker.current() {
                    return;
                }
                let target = input
                    .selected()
                    .checked_sub(1)
                    .and_then(|index| picker.inputs.borrow().get(index as usize).cloned());
                if let Some(target) = target {
                    let token = picker
                        .nav
                        .state
                        .borrow_mut()
                        .workspace
                        .inspect_operator_input(target);
                    picker.inspection.set(Some(token));
                    let idle = {
                        let state = picker.nav.state.borrow();
                        !state.workspace.loading && !state.workspace.pending
                    };
                    if idle {
                        super::workspace::refresh(&picker.nav);
                    }
                }
                // The next dialog render adds newly observed actions. Do not
                // replace a GTK selection model from inside its notification.
            }
        });
        root.connect_unmap({
            let picker = picker.clone();
            move |_| picker.release()
        });
        retry.connect_clicked({
            let picker = picker.clone();
            move |_| {
                let idle = {
                    let state = picker.nav.state.borrow();
                    !state.workspace.loading && !state.workspace.pending
                };
                if picker.current() && idle {
                    super::workspace::refresh(&picker.nav);
                    picker.refresh();
                }
            }
        });
        let needs_read = {
            let state = nav.state.borrow();
            (!state
                .workspace
                .input_catalog
                .as_ref()
                .is_some_and(Result::is_ok)
                || !state.workspace.can_control())
                && !state.workspace.loading
                && !state.workspace.pending
        };
        if needs_read && picker.current() {
            super::workspace::refresh(&picker.nav);
            picker.refresh();
        }
        (root, picker)
    }

    fn current(&self) -> bool {
        self.nav.is_current() && self.nav.registry_is_current()
    }

    fn release(&self) {
        if let Some(token) = self.inspection.take() {
            self.nav
                .state
                .borrow_mut()
                .workspace
                .release_operator_inspection(token);
        }
    }

    pub fn selected(&self) -> Option<OperatorAction> {
        let action = self.action.upgrade()?;
        match &self
            .choices
            .borrow()
            .get(action.selected() as usize)?
            .action
        {
            CatalogAction::Operator(action) => Some(action.clone()),
            _ => None,
        }
    }

    pub fn refresh(&self) {
        self.refresh_with_selection(self.selected());
    }

    pub fn connect_changed(&self, callback: impl Fn() + 'static) {
        if let Some(action) = self.action.upgrade() {
            action.connect_selected_notify(move |_| callback());
        }
    }

    fn refresh_with_selection(&self, selected: Option<OperatorAction>) {
        if self.updating.get() {
            return;
        }
        let (Some(action), Some(input), Some(notice)) = (
            self.action.upgrade(),
            self.input.upgrade(),
            self.notice.upgrade(),
        ) else {
            return;
        };
        let current = self.current();
        input.set_sensitive(current);
        action.set_sensitive(current);
        if let Some(retry) = self.retry.upgrade() {
            let state = self.nav.state.borrow();
            retry.set_sensitive(current && !state.workspace.loading && !state.workspace.pending);
        }
        if !current {
            self.release();
            notice.set_label(&fl!(LANGUAGE_LOADER, "palette-scope-changed"));
            return;
        }
        let state = self.nav.state.borrow();
        let next_inputs: Vec<_> = state
            .workspace
            .input_catalog
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|inputs| inputs.iter().map(|input| input.target.clone()).collect())
            // Keep the user's inspection through a failed list read so the
            // normal refresh can recover it. The action catalog still rejects
            // all input actions until an authoritative inventory succeeds.
            .unwrap_or_else(|| self.inputs.borrow().clone());
        let mut choices = action_catalog::binding_entries(&state);
        if let Some(selected) = &selected {
            if !choices
                .iter()
                .any(|entry| entry.action == CatalogAction::Operator(selected.clone()))
            {
                choices.insert(0, action_catalog::binding_entry(&state, selected));
            }
        }
        let selected_index = selected
            .as_ref()
            .and_then(|selected| {
                choices
                    .iter()
                    .position(|entry| entry.action == CatalogAction::Operator(selected.clone()))
            })
            .unwrap_or(0) as u32;
        notice.set_label(&if state
            .workspace
            .input_catalog
            .as_ref()
            .is_some_and(Result::is_err)
        {
            fl!(LANGUAGE_LOADER, "palette-unavailable")
        } else if self.inspection.get().is_some()
            && (state.workspace.loading || state.workspace.dirty)
        {
            fl!(LANGUAGE_LOADER, "control-loading")
        } else {
            fl!(LANGUAGE_LOADER, "action-inspect-help")
        });
        drop(state);
        self.updating.set(true);
        if *self.inputs.borrow() != next_inputs && !popup_open(&input) || input.model().is_none() {
            let selected = input
                .selected()
                .checked_sub(1)
                .and_then(|i| self.inputs.borrow().get(i as usize).cloned());
            let index = selected
                .as_ref()
                .and_then(|target| {
                    next_inputs
                        .iter()
                        .position(|input| target.matches(&input.name, input.uuid.as_deref()))
                })
                .map_or(0, |i| i as u32 + 1);
            let mut names = vec![fl!(LANGUAGE_LOADER, "action-inspect-none")];
            names.extend(next_inputs.iter().map(|input| input.name.clone()));
            *self.inputs.borrow_mut() = next_inputs;
            input.set_model(Some(&super::string_list(&names)));
            input.set_selected(index);
            if index == 0 {
                self.release();
            }
        }
        let labels: Vec<_> = choices.iter().map(|entry| entry.label.clone()).collect();
        let previous: Vec<_> = self
            .choices
            .borrow()
            .iter()
            .map(|entry| entry.label.clone())
            .collect();
        if (labels != previous || *self.choices.borrow() != choices) && !popup_open(&action)
            || action.model().is_none()
        {
            *self.choices.borrow_mut() = choices;
            action.set_model(Some(&super::string_list(&labels)));
            action.set_selected(selected_index);
        }
        self.updating.set(false);
    }
}

impl Drop for ActionPicker {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_support::{native_test, navigation, pump};
    use crate::{
        controller::state::Page,
        domain::{
            control::{MediaState, MediaStatus},
            operator_input::{InputIdentity, InputObservation},
        },
    };
    use std::time::Duration;

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_picker_inspects_independently_preserves_selection_and_releases_scope() {
        native_test("ui::operator_picker::tests::native_picker_inspects_independently_preserves_selection_and_releases_scope", || {
            let (nav, _stack, _runtime, _events) = navigation();
            *nav.state.borrow_mut() = action_catalog::tests::connected();
            let target = ActionTarget { name: "Clip <b>One</b>".into(), uuid: Some("clip-1".into()) };
            {
                let mut state = nav.state.borrow_mut();
                state.current_page = Page::Settings;
                state.workspace.selected_scene = Some("Editor scene".into());
                state.workspace.selected_input = Some("Editor input".into());
                state.workspace.loading = true; // Hold a read while the inspector changes its target.
                state.workspace.input_catalog.as_mut().unwrap().as_mut().unwrap().push(InputIdentity { target: target.clone(), kind: "ffmpeg_source".into() });
            }
            let (root, picker) = ActionPicker::new(&nav, None);
            let window = adw::Window::new(); window.set_content(Some(&root)); window.present();
            pump(Duration::from_millis(50));
            let selected = picker.selected().unwrap();
            picker.input.upgrade().unwrap().set_selected(2);
            let token = picker.inspection.get().unwrap();
            {
                let state = nav.state.borrow();
                assert_eq!(state.workspace.operator_inspections, vec![(token, target.clone())]);
                assert!(state.workspace.dirty, "a pending read must leave the new inspection requested");
                assert_eq!(state.workspace.selected_input.as_deref(), Some("Editor input"));
                assert_eq!(state.workspace.selected_scene.as_deref(), Some("Editor scene"));
            }
            {
                let mut state = nav.state.borrow_mut();
                state.workspace.loading = false;
                state.workspace.dirty = false;
                state.workspace.operator_inputs.push(InputObservation { target: target.clone(), media: Some(Ok(MediaStatus { state: MediaState::Playing, cursor_ms: Some(0), duration_ms: Some(1000) })), filters: Some(Ok(Vec::new())) });
                state.available_requests.push("TriggerMediaInputAction".into());
            }
            picker.refresh();
            assert_eq!(picker.selected(), Some(selected), "new results do not change the user's chosen action");
            let restart = OperatorAction::RestartMedia { target };
            let index = picker.choices.borrow().iter().position(|entry| entry.action == CatalogAction::Operator(restart.clone()) && entry.available).unwrap();
            picker.action.upgrade().unwrap().set_selected(index as u32);
            assert_eq!(picker.selected(), Some(restart.clone()));
            picker.refresh(); assert_eq!(picker.selected(), Some(restart));
            let catalog = nav.state.borrow().workspace.input_catalog.clone();
            nav.state.borrow_mut().workspace.input_catalog = Some(Err("temporarily unavailable".into()));
            picker.refresh();
            assert_eq!(picker.input.upgrade().unwrap().selected(), 2);
            assert_eq!(picker.inspection.get(), Some(token), "a read failure must retain inspection for recovery");
            assert!(!picker.choices.borrow()[picker.action.upgrade().unwrap().selected() as usize].available);
            nav.state.borrow_mut().workspace.input_catalog = catalog;
            picker.refresh();
            assert!(action_catalog::execute(&nav, &CatalogAction::Operator(picker.selected().unwrap())));
            assert!(matches!(nav.state.borrow().workspace.active_action, Some(crate::domain::control::ControlAction::Media { .. })));
            assert_eq!(nav.state.borrow().workspace.selected_input.as_deref(), Some("Editor input"));
            window.close(); pump(Duration::from_millis(50));
            assert!(nav.state.borrow().workspace.operator_inspections.is_empty());
            let weak = Rc::downgrade(&picker); drop((picker, root, window));
            pump(Duration::from_millis(50)); assert!(weak.upgrade().is_none());

            let (root, picker) = ActionPicker::new(&nav, None);
            let window = adw::Window::new(); window.set_content(Some(&root)); window.present();
            nav.state.borrow_mut().workspace.loading = true;
            picker.input.upgrade().unwrap().set_selected(1);
            assert_eq!(nav.state.borrow().workspace.operator_inspections.len(), 1);
            nav.state.borrow_mut().show_registry.generation += 1;
            picker.refresh();
            assert!(nav.state.borrow().workspace.operator_inspections.is_empty());
            assert!(!picker.input.upgrade().unwrap().is_sensitive());
            window.close();
        });
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_picker_can_retry_failed_initial_inventory_without_an_inspection() {
        native_test("ui::operator_picker::tests::native_picker_can_retry_failed_initial_inventory_without_an_inspection", || {
            let (nav, _stack, _runtime, _events) = navigation();
            *nav.state.borrow_mut() = action_catalog::tests::connected();
            nav.state.borrow_mut().current_page = Page::Settings;
            nav.state.borrow_mut().workspace.input_catalog = Some(Err("unavailable".into()));
            let initial = nav.state.borrow().workspace.sequence;
            let (root, picker) = ActionPicker::new(&nav, None);
            assert_eq!(nav.state.borrow().workspace.sequence, initial + 1, "opening after a failed catalog retries the read");
            assert!(!picker.retry.upgrade().unwrap().is_sensitive());
            assert!(nav.state.borrow().workspace.operator_inspections.is_empty());
            // An unsuccessful read enables an explicit retry even though no
            // source could be selected and no background inspector exists.
            nav.state.borrow_mut().workspace.loading = false;
            picker.refresh();
            let retry = picker.retry.upgrade().unwrap();
            assert!(retry.is_sensitive()); retry.emit_clicked();
            assert_eq!(nav.state.borrow().workspace.sequence, initial + 2);
            assert!(!retry.is_sensitive());
            nav.state.borrow_mut().workspace.loading = false;
            nav.state.borrow_mut().show_registry.generation += 1;
            picker.refresh();
            assert!(!retry.is_sensitive()); retry.emit_clicked();
            assert_eq!(nav.state.borrow().workspace.sequence, initial + 2, "retired callbacks cannot refresh another scope");
            drop(root);
        });
    }
}
