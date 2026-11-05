//! Workspace-bound search controls shared by Inventory and Live.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use i18n_embed_fl::fl;

use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::registry::SceneFilter;
use crate::ui::{navigation::NavigationContext, persist::persist_registry};

struct Controls {
    nav: NavigationContext,
    search: glib::WeakRef<gtk4::SearchEntry>,
    favorites: glib::WeakRef<gtk4::CheckButton>,
    last_filter: RefCell<SceneFilter>,
    menu: glib::WeakRef<gtk4::MenuButton>,
    last_decks: RefCell<Option<std::collections::BTreeMap<String, SceneFilter>>>,
    synchronizing: Cell<bool>,
    changed: Rc<dyn Fn()>,
}

impl Controls {
    fn refresh(self: &Rc<Self>) {
        let filter = self.nav.state.borrow().registry.scene_filter.clone();
        // An unrelated readback must not overwrite a not-yet-debounced draft.
        if *self.last_filter.borrow() != filter {
            self.sync_inputs(filter);
        }
        let decks = self.nav.state.borrow().registry.decks.clone();
        if self.last_decks.borrow().as_ref() != Some(&decks) {
            self.rebuild_menu(&decks);
            *self.last_decks.borrow_mut() = Some(decks);
        }
    }

    fn sync_inputs(&self, filter: SceneFilter) {
        let (Some(search), Some(favorites)) = (self.search.upgrade(), self.favorites.upgrade())
        else {
            return;
        };
        self.synchronizing.set(true);
        if search.text().as_str() != filter.query {
            search.set_text(&filter.query);
        }
        favorites.set_active(filter.favorites_only);
        *self.last_filter.borrow_mut() = filter;
        self.synchronizing.set(false);
    }

    fn apply(self: &Rc<Self>, filter: SceneFilter) {
        if self.synchronizing.get() {
            return;
        }
        persist_registry(&self.nav, "scene view", |registry| {
            if registry.scene_filter == filter {
                return false;
            }
            registry.scene_filter = filter;
            true
        });
        if self.nav.registry_is_current() {
            let filter = self.nav.state.borrow().registry.scene_filter.clone();
            self.sync_inputs(filter);
        }
        self.refresh();
        (self.changed)();
    }

    fn apply_inputs(self: &Rc<Self>) {
        if let (Some(search), Some(favorites)) = (self.search.upgrade(), self.favorites.upgrade()) {
            self.apply(SceneFilter {
                query: search.text().into(),
                favorites_only: favorites.is_active(),
            });
        }
    }
    fn rebuild_menu(self: &Rc<Self>, decks: &std::collections::BTreeMap<String, SceneFilter>) {
        let Some(menu) = self.menu.upgrade() else {
            return;
        };
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        content.set_margin_top(8);
        content.set_margin_bottom(8);
        content.set_margin_start(8);
        content.set_margin_end(8);
        let popover = gtk4::Popover::new();
        if decks.is_empty() {
            content.append(&literal_label(&fl!(LANGUAGE_LOADER, "deck-none")));
        }
        for (name, filter) in decks {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            let select = gtk4::Button::builder()
                .child(&literal_label(name))
                .hexpand(true)
                .build();
            select.set_widget_name(&format!("deck-select-{name}"));
            select.connect_clicked({
                let controls = self.clone();
                let filter = filter.clone();
                let name = name.clone();
                let popover = popover.downgrade();
                move |_| {
                    if controls.nav.state.borrow().registry.decks.get(&name) != Some(&filter) {
                        return;
                    }
                    if let Some(popover) = popover.upgrade() {
                        popover.popdown();
                    }
                    controls.apply(filter.clone());
                }
            });
            row.append(&select);
            let remove = icon_button(
                "list-remove-symbolic",
                &fl!(LANGUAGE_LOADER, "deck-remove", name = name.as_str()),
            );
            remove.set_widget_name(&format!("deck-remove-{name}"));
            remove.connect_clicked({
                let controls = self.clone();
                let name = name.clone();
                let filter = filter.clone();
                let popover = popover.downgrade();
                move |_| {
                    if let Some(popover) = popover.upgrade() {
                        popover.popdown();
                    }
                    persist_registry(&controls.nav, "remove deck", |registry| {
                        if registry.decks.get(&name) == Some(&filter) {
                            registry.decks.remove(&name).is_some()
                        } else {
                            false
                        }
                    });
                    let controls = controls.clone();
                    glib::idle_add_local_once(move || {
                        controls.refresh();
                        (controls.changed)();
                    });
                }
            });
            row.append(&remove);
            content.append(&row);
        }
        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .max_content_height(360)
            .propagate_natural_height(true)
            .child(&content)
            .build();
        popover.set_child(Some(&scroll));
        menu.set_popover(Some(&popover));
    }

    fn save_dialog(self: &Rc<Self>, button: &gtk4::Button) {
        if !self.nav.registry_is_current() {
            return;
        }
        // Capture a search draft even if its debounce has not fired yet.
        self.apply_inputs();
        let filter = self.nav.state.borrow().registry.scene_filter.clone();
        let parent = button.root().and_downcast::<gtk4::Window>();
        let dialog = adw::MessageDialog::new(
            parent.as_ref(),
            Some(&fl!(LANGUAGE_LOADER, "deck-save-title")),
            Some(&fl!(LANGUAGE_LOADER, "deck-save-help")),
        );
        dialog.set_widget_name("deck-save-dialog");
        dialog.add_response("cancel", &fl!(LANGUAGE_LOADER, "deck-cancel"));
        dialog.add_response("save", &fl!(LANGUAGE_LOADER, "deck-save"));
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        dialog.set_response_enabled("save", false);
        let name = gtk4::Entry::new();
        name.set_widget_name("deck-name");
        name.set_width_chars(1);
        name.update_property(&[gtk4::accessible::Property::Label(&fl!(
            LANGUAGE_LOADER,
            "deck-name"
        ))]);
        dialog.set_extra_child(Some(&name));
        name.connect_changed({
            let dialog = dialog.downgrade();
            let nav = self.nav.clone();
            move |name| {
                if let Some(dialog) = dialog.upgrade() {
                    dialog.set_response_enabled(
                        "save",
                        nav.registry_is_current()
                            && nav.state.borrow().registry.can_save_deck(&name.text()),
                    );
                }
            }
        });
        name.connect_activate({
            let dialog = dialog.downgrade();
            move |_| {
                if let Some(dialog) = dialog.upgrade() {
                    if dialog.is_response_enabled("save") {
                        dialog.response("save");
                    }
                }
            }
        });
        dialog.connect_response(Some("save"), {
            let name = name.downgrade();
            let controls = self.clone();
            move |_, _| {
                if let Some(name) = name.upgrade() {
                    persist_registry(&controls.nav, "save deck", |registry| {
                        registry.save_deck(&name.text(), filter.clone())
                    });
                    controls.refresh();
                    (controls.changed)();
                }
            }
        });
        dialog.present();
        name.grab_focus();
    }
}

pub(crate) fn build(nav: &NavigationContext, changed: impl Fn() + 'static) -> gtk4::Box {
    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    root.set_widget_name("scene-deck-controls");
    let filter = nav.state.borrow().registry.scene_filter.clone();
    let search = gtk4::SearchEntry::builder()
        .placeholder_text(fl!(LANGUAGE_LOADER, "deck-search-placeholder"))
        .search_delay(150)
        .hexpand(true)
        .build();
    search.set_widget_name("deck-search");
    search.set_width_chars(1);
    search.set_text(&filter.query);
    search.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "deck-search-placeholder"
    ))]);
    root.append(&search);
    let actions = gtk4::FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(4)
        .column_spacing(8)
        .row_spacing(4)
        .build();
    let favorites = gtk4::CheckButton::with_label(&fl!(LANGUAGE_LOADER, "deck-favorites-only"));
    favorites.set_widget_name("deck-favorites");
    favorites.set_active(filter.favorites_only);
    if let Some(label) = favorites.child().and_downcast::<gtk4::Label>() {
        label.set_wrap(true);
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        label.set_width_chars(1);
    }
    add_action(&actions, &favorites);
    let menu = gtk4::MenuButton::builder()
        .label(fl!(LANGUAGE_LOADER, "deck-menu"))
        .build();
    menu.set_widget_name("deck-menu");
    add_action(&actions, &menu);
    let save = icon_button(
        "list-add-symbolic",
        &fl!(LANGUAGE_LOADER, "deck-save-title"),
    );
    save.set_widget_name("deck-save");
    add_action(&actions, &save);
    let reset = icon_button("edit-clear-symbolic", &fl!(LANGUAGE_LOADER, "deck-reset"));
    reset.set_widget_name("deck-reset");
    add_action(&actions, &reset);
    root.append(&actions);
    let controls = Rc::new(Controls {
        nav: nav.registry_scoped(),
        search: search.downgrade(),
        favorites: favorites.downgrade(),
        last_filter: RefCell::new(filter),
        menu: menu.downgrade(),
        last_decks: RefCell::new(None),
        synchronizing: Cell::new(false),
        changed: Rc::new(changed),
    });
    search.connect_search_changed({
        let controls = controls.clone();
        move |_| controls.apply_inputs()
    });
    favorites.connect_toggled({
        let controls = controls.clone();
        move |_| controls.apply_inputs()
    });
    reset.connect_clicked({
        let controls = controls.clone();
        move |_| controls.apply(SceneFilter::default())
    });
    save.connect_clicked({
        let controls = controls.clone();
        move |button| controls.save_dialog(button)
    });
    controls.refresh();
    let group = gio::SimpleActionGroup::new();
    let sync = gio::SimpleAction::new("sync", None);
    sync.connect_activate(move |_, _| controls.refresh());
    group.add_action(&sync);
    root.insert_action_group("deck", Some(&group));
    root
}

pub(crate) fn sync(widget: &gtk4::Widget) {
    widget
        .activate_action("deck.sync", None)
        .expect("deck controls sync action");
}

fn literal_label(text: &str) -> gtk4::Label {
    gtk4::Label::builder()
        .label(text)
        .use_markup(false)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .max_width_chars(24)
        .xalign(0.0)
        .build()
}

fn icon_button(icon: &str, label: &str) -> gtk4::Button {
    let button = gtk4::Button::builder()
        .icon_name(icon)
        .tooltip_text(label)
        .build();
    button.update_property(&[gtk4::accessible::Property::Label(label)]);
    button
}

fn add_action(flow: &gtk4::FlowBox, widget: &impl IsA<gtk4::Widget>) {
    crate::ui::insert_compact_flow_child(flow, widget);
    if let Some(wrapper) = widget.parent() {
        wrapper.set_focusable(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(root: &gtk4::Widget, name: &str) -> gtk4::Widget {
        fn search(root: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
            if root.widget_name() == name {
                return Some(root.clone());
            }
            let mut child = root.first_child();
            while let Some(widget) = child {
                if let Some(found) = search(&widget, name) {
                    return Some(found);
                }
                child = widget.next_sibling();
            }
            None
        }
        search(root, name).unwrap_or_else(|| panic!("missing {name}"))
    }

    fn dialog() -> adw::MessageDialog {
        gtk4::Window::list_toplevels()
            .into_iter()
            .find(|widget| widget.widget_name() == "deck-save-dialog" && widget.is_visible())
            .unwrap()
            .downcast()
            .unwrap()
    }

    #[test]
    #[ignore = "requires a disposable GTK display and isolated registry storage"]
    fn native_saved_decks_capture_drafts_and_reject_stale_selection_and_dialogs() {
        crate::ui::test_support::native_test(
            "ui::widgets::deck_controls::tests::native_saved_decks_capture_drafts_and_reject_stale_selection_and_dialogs",
            || {
                use crate::ui::test_support::{navigation, pump, pump_until};
                use std::time::Duration;
                let (nav, _stack, _runtime, _events) = navigation();
                let root = build(&nav, || {});
                let window = gtk4::Window::builder().default_width(360).child(&root).build();
                window.present();
                pump(Duration::from_millis(80));
                let search = find(root.upcast_ref(), "deck-search").downcast::<gtk4::SearchEntry>().unwrap();
                let save = find(root.upcast_ref(), "deck-save").downcast::<gtk4::Button>().unwrap();
                let reset = find(root.upcast_ref(), "deck-reset").downcast::<gtk4::Button>().unwrap();
                // Reset must also clear a draft before the debounce fires.
                search.set_text("uncommitted draft");
                reset.emit_clicked();
                assert_eq!(search.text(), "");
                search.set_text("podcast guest");
                save.emit_clicked();
                let save_dialog = dialog();
                assert!(!save_dialog.is_response_enabled("save"));
                let name = save_dialog.extra_child().unwrap().downcast::<gtk4::Entry>().unwrap();
                let deck = "<b>Podcast & guest</b>";
                name.set_text(deck);
                assert!(save_dialog.is_response_enabled("save"));
                save_dialog.response("save");
                pump(Duration::from_millis(80));
                assert_eq!(nav.state.borrow().registry.decks[deck].query, "podcast guest");
                save.emit_clicked();
                let duplicate = dialog();
                duplicate.extra_child().unwrap().downcast::<gtk4::Entry>().unwrap().set_text(" <B>PODCAST & GUEST</B> ");
                assert!(!duplicate.is_response_enabled("save"));
                duplicate.response("cancel");
                reset.emit_clicked();
                let menu = find(root.upcast_ref(), "deck-menu").downcast::<gtk4::MenuButton>().unwrap();
                menu.popup();
                pump(Duration::from_millis(80));
                let popup = menu.popover().unwrap();
                let select = find(popup.upcast_ref(), &format!("deck-select-{deck}")).downcast::<gtk4::Button>().unwrap();
                let label = select.child().unwrap().downcast::<gtk4::Label>().unwrap();
                assert_eq!(label.text(), deck);
                assert!(!label.uses_markup());
                select.emit_clicked();
                assert_eq!(search.text(), "podcast guest");
                let remove = find(popup.upcast_ref(), &format!("deck-remove-{deck}")).downcast::<gtk4::Button>().unwrap();
                remove.emit_clicked();
                pump(Duration::from_millis(40));
                assert!(!nav.state.borrow().registry.decks.contains_key(deck));
                // An old menu item cannot select or delete a newly created view.
                persist_registry(&nav, "replacement view", |registry| registry.save_deck(deck, SceneFilter { query: "replacement".into(), favorites_only: true }));
                sync(root.upcast_ref());
                reset.emit_clicked();
                select.emit_clicked();
                remove.emit_clicked();
                assert_eq!(nav.state.borrow().registry.scene_filter, SceneFilter::default());
                assert_eq!(nav.state.borrow().registry.decks[deck].query, "replacement");
                save.emit_clicked();
                let stale = dialog();
                stale.extra_child().unwrap().downcast::<gtk4::Entry>().unwrap().set_text("Wrong workspace");
                nav.state.borrow_mut().show_registry.generation += 1;
                stale.response("save");
                assert!(!nav.state.borrow().registry.decks.contains_key("Wrong workspace"));
                assert!(pump_until(Duration::from_secs(5), || crate::ui::persist::registry_save_status() == crate::ui::persist::RegistrySaveStatus::Saved));
                assert_eq!(crate::storage::registry::read_registry().decks[deck].query, "replacement");
                window.set_child(gtk4::Widget::NONE);
                window.destroy();
            },
        );
    }
}
