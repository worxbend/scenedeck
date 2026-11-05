//! Searchable application commands with execution-time scope and state checks.

use std::{cell::RefCell, rc::Rc, time::Duration};

use adw::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SearchEntry};
use i18n_embed_fl::fl;

use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::{
    action_catalog::{self, Entry},
    navigation::NavigationContext,
};

/// Present one palette per parent; reopening raises the existing window.
pub(super) fn show(parent: &adw::ApplicationWindow, nav: &NavigationContext) {
    for window in gtk4::Window::list_toplevels() {
        if let Ok(window) = window.downcast::<adw::Window>() {
            if window.widget_name() == "command-palette"
                && window.transient_for().as_ref() == Some(parent.upcast_ref())
            {
                window.present();
                return;
            }
        }
    }
    build(parent, nav).present();
}

fn build(parent: &adw::ApplicationWindow, nav: &NavigationContext) -> adw::Window {
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .default_width(580)
        .default_height(520)
        .title(fl!(LANGUAGE_LOADER, "palette-title"))
        .build();
    window.set_widget_name("command-palette");
    let root = GtkBox::new(Orientation::Vertical, 8);
    root.append(&adw::HeaderBar::new());
    let search = SearchEntry::builder()
        .placeholder_text(fl!(LANGUAGE_LOADER, "palette-search"))
        .search_delay(0)
        .width_chars(1)
        .hexpand(true)
        .margin_start(12)
        .margin_end(12)
        .build();
    search.set_widget_name("palette-search");
    search.update_property(&[gtk4::accessible::Property::Label(&fl!(
        LANGUAGE_LOADER,
        "palette-search"
    ))]);
    root.append(&search);
    let notice = Label::builder()
        .wrap(true)
        .xalign(0.0)
        .margin_start(12)
        .margin_end(12)
        .build();
    notice.set_widget_name("palette-notice");
    notice.add_css_class("dim-label");
    root.append(&notice);
    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .activate_on_single_click(true)
        .build();
    list.set_widget_name("palette-results");
    list.add_css_class("navigation-sidebar");
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();
    root.append(&scroll);
    window.set_content(Some(&root));
    gtk4::prelude::GtkWindowExt::set_focus(&window, Some(&search));

    let model: Rc<RefCell<Vec<Entry>>> = Rc::new(RefCell::new(Vec::new()));
    let nav = nav.scoped().registry_scoped();
    let refresh: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        let search = search.downgrade();
        let list = list.downgrade();
        let notice = notice.downgrade();
        let model = model.clone();
        move || {
            let (Some(search), Some(list), Some(notice)) =
                (search.upgrade(), list.upgrade(), notice.upgrade())
            else {
                return;
            };
            let current = nav.is_current() && nav.registry_is_current();
            let next: Vec<_> = action_catalog::entries(&nav.state.borrow())
                .into_iter()
                .filter(|entry| entry.matches(&search.text()))
                .map(|mut entry| {
                    if matches!(entry.action, action_catalog::CatalogAction::Operator(_)) {
                        entry.available &= current;
                    }
                    entry
                })
                .collect();
            notice.set_label(&if !current {
                fl!(LANGUAGE_LOADER, "palette-scope-changed")
            } else if next.is_empty() {
                fl!(LANGUAGE_LOADER, "palette-empty")
            } else {
                fl!(LANGUAGE_LOADER, "palette-hint")
            });
            if *model.borrow() == next {
                return;
            }
            let selected = list.selected_row().and_then(|row| {
                model
                    .borrow()
                    .get(row.index() as usize)
                    .map(|entry| entry.action.clone())
            });
            let focus_in_list = list
                .root()
                .and_then(|root| root.focus())
                .is_some_and(|focus| focus.is_ancestor(&list));
            if focus_in_list {
                search.grab_focus();
            }
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            for entry in &next {
                let content = GtkBox::new(Orientation::Vertical, 3);
                content.set_margin_top(6);
                content.set_margin_bottom(6);
                let label = Label::builder()
                    .label(&entry.label)
                    .wrap(true)
                    .xalign(0.0)
                    .build();
                content.append(&label);
                if !entry.available {
                    let reason = Label::builder()
                        .label(fl!(LANGUAGE_LOADER, "palette-unavailable"))
                        .wrap(true)
                        .xalign(0.0)
                        .build();
                    reason.add_css_class("dim-label");
                    content.append(&reason);
                }
                list.append(&content);
            }
            let index = selected
                .and_then(|selected| next.iter().position(|entry| entry.action == selected))
                .unwrap_or(0);
            *model.borrow_mut() = next;
            list.select_row(list.row_at_index(index as i32).as_ref());
            if focus_in_list {
                if let Some(row) = list.selected_row() {
                    row.grab_focus();
                }
            }
        }
    });
    refresh();
    search.connect_search_changed({
        let refresh = refresh.clone();
        move |_| refresh()
    });
    let activate: Rc<dyn Fn(i32)> = Rc::new({
        let nav = nav.clone();
        let model = model.clone();
        let window = window.downgrade();
        let notice = notice.downgrade();
        let refresh = refresh.clone();
        move |index| {
            let entry = model.borrow().get(index as usize).cloned();
            // A displayed Stage action must never become Switch to Program
            // between the last render and Enter. Refresh before asking again.
            let unchanged = entry.as_ref().is_some_and(|entry| {
                action_catalog::entries(&nav.state.borrow())
                    .iter()
                    .any(|current| {
                        current.action == entry.action
                            && current.label == entry.label
                            && current.available
                    })
            });
            if unchanged && entry.is_some_and(|entry| action_catalog::execute(&nav, &entry.action))
            {
                if let Some(window) = window.upgrade() {
                    window.close();
                }
            } else {
                refresh();
                if let Some(notice) = notice.upgrade() {
                    notice.set_label(&fl!(LANGUAGE_LOADER, "palette-unavailable"));
                }
            }
        }
    });
    list.connect_row_activated({
        let activate = activate.clone();
        move |_, row| activate(row.index())
    });
    search.connect_activate({
        let list = list.downgrade();
        move |_| {
            if let Some(row) = list.upgrade().and_then(|list| list.selected_row()) {
                activate(row.index());
            }
        }
    });
    let keys = gtk4::EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    keys.connect_key_pressed({
        let window = window.downgrade();
        let list = list.downgrade();
        let search = search.downgrade();
        let scroll = scroll.downgrade();
        move |_, key, _, modifiers| {
            if !modifiers.is_empty() {
                return glib::Propagation::Proceed;
            }
            if key == gtk4::gdk::Key::Escape {
                if let Some(window) = window.upgrade() {
                    window.close();
                }
                return glib::Propagation::Stop;
            }
            let search_focused = search.upgrade().is_some_and(|search| {
                search
                    .root()
                    .and_then(|root| root.focus())
                    .is_some_and(|focus| {
                        focus == search.clone().upcast::<gtk4::Widget>()
                            || focus.is_ancestor(&search)
                    })
            });
            if !search_focused {
                return glib::Propagation::Proceed;
            }
            if let Some(list) = list.upgrade() {
                let delta = match key {
                    gtk4::gdk::Key::Down => 1,
                    gtk4::gdk::Key::Up => -1,
                    _ => return glib::Propagation::Proceed,
                };
                let index = list.selected_row().map(|r| r.index()).unwrap_or(-1);
                if let Some(row) = list.row_at_index((index + delta).max(0)) {
                    list.select_row(Some(&row));
                    if let (Some(scroll), Some(bounds)) =
                        (scroll.upgrade(), row.compute_bounds(&list))
                    {
                        let adjustment = scroll.vadjustment();
                        let top = f64::from(bounds.y());
                        let bottom = top + f64::from(bounds.height());
                        if top < adjustment.value() {
                            adjustment.set_value(top);
                        } else if bottom > adjustment.value() + adjustment.page_size() {
                            adjustment.set_value(bottom - adjustment.page_size());
                        }
                    }
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    });
    window.add_controller(keys);
    // Weak widgets prevent the refresh source from keeping a dismissed palette
    // alive. Refreshing compares presentations, preserving keyboard selection.
    glib::timeout_add_local(Duration::from_millis(250), {
        let window = window.downgrade();
        move || {
            if window.upgrade().is_none_or(|window| !window.is_visible()) {
                return glib::ControlFlow::Break;
            }
            refresh();
            glib::ControlFlow::Continue
        }
    });
    window
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::state::Page;
    use crate::ui::test_support::{native_test, navigation, pump};

    fn find(widget: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
        if widget.widget_name() == name {
            return Some(widget.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if let Some(found) = find(&widget, name) {
                return Some(found);
            }
        }
        None
    }

    fn query(search: &SearchEntry, text: &str) {
        search.set_text(text);
        search.emit_by_name::<()>("search-changed", &[]);
    }

    fn key(window: &adw::Window, key: gtk4::gdk::Key) -> bool {
        let controllers = window.observe_controllers();
        let controller = (0..controllers.n_items())
            .filter_map(|i| controllers.item(i))
            .find_map(|item| item.downcast::<gtk4::EventControllerKey>().ok())
            .unwrap();
        controller.emit_by_name::<bool>(
            "key-pressed",
            &[&key, &0u32, &gtk4::gdk::ModifierType::empty()],
        )
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_palette_search_scope_keyboard_and_lifetime() {
        native_test(
            "ui::command_palette::tests::native_palette_search_scope_keyboard_and_lifetime",
            || {
                let (nav, stack, _runtime, _events) = navigation();
                *nav.state.borrow_mut() = crate::ui::action_catalog::tests::connected();
                for page in [Page::Live, Page::Sources, Page::Settings] {
                    stack.add_named(&GtkBox::new(Orientation::Vertical, 0), Some(page.id()));
                }
                let parent = adw::ApplicationWindow::builder().build();
                parent.set_content(Some(&stack));
                parent.present();
                let window = build(&parent, &nav);
                window.present();
                pump(Duration::from_millis(60));
                let search = find(window.upcast_ref(), "palette-search")
                    .unwrap()
                    .downcast::<SearchEntry>()
                    .unwrap();
                let list = find(window.upcast_ref(), "palette-results")
                    .unwrap()
                    .downcast::<ListBox>()
                    .unwrap();
                search.grab_focus();
                assert!(key(&window, gtk4::gdk::Key::Down));
                assert_eq!(list.selected_row().unwrap().index(), 1);
                assert!(key(&window, gtk4::gdk::Key::Up));
                assert_eq!(list.selected_row().unwrap().index(), 0);
                query(&search, "INTERVIEW camera");
                assert!(list.row_at_index(1).is_none());
                let row = list.row_at_index(0).unwrap();
                let label = row
                    .child()
                    .unwrap()
                    .first_child()
                    .unwrap()
                    .downcast::<Label>()
                    .unwrap();
                assert_eq!(label.text(), "Stage Camera <b>One</b>");
                assert!(!label.uses_markup());
                pump(Duration::from_millis(270));
                assert_eq!(
                    list.row_at_index(0).unwrap(),
                    row,
                    "unchanged refresh must preserve the row"
                );
                query(&search, "no such result");
                assert!(list.row_at_index(0).is_none());
                assert!(gtk4::prelude::RootExt::focus(&window)
                    .unwrap()
                    .is_ancestor(&search));
                query(&search, "camera");
                let sequence = nav.state.borrow().workspace.sequence;
                nav.state
                    .borrow_mut()
                    .workspace
                    .snapshot
                    .as_mut()
                    .unwrap()
                    .studio
                    .enabled = Some(false);
                search.emit_by_name::<()>("activate", &[]);
                assert_eq!(
                    nav.state.borrow().workspace.sequence,
                    sequence,
                    "old Stage label cannot switch Program"
                );
                assert!(window.is_visible());
                nav.state.borrow_mut().workspace.epoch += 1;
                search.emit_by_name::<()>("activate", &[]);
                assert_eq!(
                    nav.state.borrow().workspace.sequence,
                    sequence,
                    "retired scope cannot dispatch"
                );
                query(&search, "settings");
                search.emit_by_name::<()>("activate", &[]);
                assert_eq!(nav.state.borrow().current_page, Page::Settings);
                let weak_window = window.downgrade();
                let weak_search = search.downgrade();
                drop((window, search, list, row, label));
                pump(Duration::from_millis(280));
                assert!(weak_window.upgrade().is_none());
                assert!(weak_search.upgrade().is_none());
                parent.close();
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display"]
    fn native_palette_large_text_navigation_escape_and_single_instance() {
        native_test("ui::command_palette::tests::native_palette_large_text_navigation_escape_and_single_instance", || {
            let (nav, _stack, _runtime, _events) = navigation();
            *nav.state.borrow_mut() = crate::ui::action_catalog::tests::connected();
            gtk4::Settings::default().unwrap().set_gtk_font_name(Some("Sans 22"));
            let parent = adw::ApplicationWindow::builder().build();
            parent.present();
            show(&parent, &nav);
            show(&parent, &nav);
            let windows: Vec<_> = gtk4::Window::list_toplevels().into_iter().filter_map(|w| w.downcast::<adw::Window>().ok()).filter(|w| w.widget_name() == "command-palette").collect();
            assert_eq!(windows.len(), 1);
            let window = &windows[0];
            window.set_default_size(360, 460);
            pump(Duration::from_millis(100));
            let search = find(window.upcast_ref(), "palette-search").unwrap().downcast::<SearchEntry>().unwrap();
            let list = find(window.upcast_ref(), "palette-results").unwrap().downcast::<ListBox>().unwrap();
            assert!(window.width() <= 360, "palette cannot fit 360px: {}", window.width());
            assert!(list.width() <= window.width());
            search.grab_focus();
            for _ in 0..8 { key(window, gtk4::gdk::Key::Down); }
            assert_eq!(list.selected_row().unwrap().index(), 8);
            let scroll = list.parent().unwrap().parent().unwrap().downcast::<gtk4::ScrolledWindow>().unwrap();
            assert!(scroll.vadjustment().value() > 0.0, "keyboard selection must scroll into view");
            list.selected_row().unwrap().grab_focus();
            assert!(key(window, gtk4::gdk::Key::Escape));
            assert!(!window.is_visible());
            parent.close();
        });
    }
}
