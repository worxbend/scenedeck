//! Inventory page — all scenes grouped by role.  Phase 5 implementation.

use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use adw::{prelude::*, ActionRow, ComboRow, PreferencesGroup, PreferencesPage, StatusPage};
use gtk4::{
    gdk, Align, Box as GtkBox, Button, ColorButton, DragSource, DropTarget, FileChooserAction,
    FileChooserNative, FileFilter, Image, Orientation, ResponseType,
};

use crate::domain::role::SceneRole;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::registry::{
    parse_scene_accent, read_registry_yaml_from_path, scene_accent_hex, write_registry,
    write_registry_yaml_to_path, SceneEntry,
};
use crate::ui::navigation::NavigationContext;
use i18n_embed_fl::fl;

// ── Role index helpers ────────────────────────────────────────────────────────

fn role_to_index(role: Option<SceneRole>) -> u32 {
    match role {
        None => 0,
        Some(SceneRole::Primary) => 1,
        Some(SceneRole::Secondary) => 2,
        Some(SceneRole::Module) => 3,
        Some(SceneRole::Raw) => 4,
        Some(SceneRole::Debug) => 5,
        Some(SceneRole::Archive) => 6,
    }
}

fn index_to_role(idx: u32) -> Option<SceneRole> {
    match idx {
        1 => Some(SceneRole::Primary),
        2 => Some(SceneRole::Secondary),
        3 => Some(SceneRole::Module),
        4 => Some(SceneRole::Raw),
        5 => Some(SceneRole::Debug),
        6 => Some(SceneRole::Archive),
        _ => None,
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .build();
    container.add_css_class("app-page");
    container.add_css_class("inventory-page");

    populate(&container, &nav);

    let refresh_fn: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        let container = container.clone();
        move || rebuild(&container, &nav)
    });

    container.connect_map({
        let refresh = refresh_fn.clone();
        move |_| refresh()
    });

    (container.upcast(), refresh_fn)
}

fn rebuild(container: &GtkBox, nav: &NavigationContext) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    populate(container, nav);
}

// ── Page population ───────────────────────────────────────────────────────────

fn populate(container: &GtkBox, nav: &NavigationContext) {
    let inventory = nav.state.borrow().scene_inventory.clone();

    // Empty state — OBS not yet connected or no scenes.
    if inventory.scenes.is_empty() {
        let empty = StatusPage::builder()
            .icon_name("view-list-symbolic")
            .title(fl!(LANGUAGE_LOADER, "inventory-empty-state-title"))
            .description(fl!(LANGUAGE_LOADER, "inventory-empty-state-description"))
            .build();
        container.append(&empty);
        return;
    }

    let registry = nav.state.borrow().registry.clone();

    let page = PreferencesPage::builder()
        .title(fl!(LANGUAGE_LOADER, "inventory-page-title"))
        .vexpand(true)
        .build();
    page.add_css_class("app-preferences-page");

    // ── OBS Scenes group ──────────────────────────────────────────────────────

    let scenes_group = PreferencesGroup::builder()
        .title(fl!(LANGUAGE_LOADER, "inventory-scenes-group-title"))
        .description(fl!(LANGUAGE_LOADER, "inventory-scenes-group-description"))
        .build();

    let yaml_row = build_yaml_actions_row(container, nav);
    scenes_group.add(&yaml_row);

    let ordered_scene_ids =
        registry.ordered_scene_ids(inventory.scenes.iter().map(|scene| scene.id.as_str()));
    let inventory_scene_ids: Vec<String> = inventory
        .scenes
        .iter()
        .map(|scene| scene.id.clone())
        .collect();

    for scene in ordered_scene_ids
        .iter()
        .filter_map(|scene_id| inventory.scenes.iter().find(|scene| &scene.id == scene_id))
    {
        // Look up the current role from the registry (source of truth).
        let current_role = registry.scenes.get(&scene.id).and_then(|e| e.role);

        let subtitle = current_role
            .map(SceneRole::description)
            .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "inventory-no-role-assigned"));

        let mut role_labels: Vec<String> = vec![SceneRole::unassigned_label()];
        role_labels.extend(SceneRole::ALL.iter().map(|r| r.label()));
        let role_label_refs: Vec<&str> = role_labels.iter().map(String::as_str).collect();
        let role_model = gtk4::StringList::new(&role_label_refs);

        let combo_row = ComboRow::builder()
            .title(scene.name.as_str())
            .subtitle(subtitle)
            .model(&role_model)
            .selected(role_to_index(current_role))
            .build();
        combo_row.add_css_class("scenedeck-combo-row");

        let drag_handle = Image::from_icon_name("list-drag-handle-symbolic");
        drag_handle.set_tooltip_text(Some("Drag to reorder scene"));
        drag_handle.set_valign(Align::Center);
        drag_handle.add_css_class("dim-label");

        let drag_source = DragSource::builder().actions(gdk::DragAction::MOVE).build();
        drag_source.connect_prepare({
            let scene_id = scene.id.clone();
            move |_, _, _| Some(gdk::ContentProvider::for_value(&scene_id.to_value()))
        });
        drag_handle.add_controller(drag_source);

        let drop_target = DropTarget::new(String::static_type(), gdk::DragAction::MOVE);
        drop_target.connect_drop({
            let target_scene_id = scene.id.clone();
            let inventory_scene_ids = inventory_scene_ids.clone();
            let nav = nav.clone();
            let container = container.clone();
            move |target, value, _, y| {
                let Ok(source_scene_id) = value.get::<String>() else {
                    return false;
                };
                let insert_after = target
                    .widget()
                    .is_some_and(|widget| y >= f64::from(widget.height()) / 2.0);
                if !reorder_scenes(
                    &nav,
                    &inventory_scene_ids,
                    &source_scene_id,
                    &target_scene_id,
                    insert_after,
                ) {
                    return false;
                }
                glib::idle_add_local_once({
                    let nav = nav.clone();
                    let container = container.clone();
                    move || rebuild(&container, &nav)
                });
                true
            }
        });
        combo_row.add_controller(drop_target);

        let current_accent = registry
            .scenes
            .get(&scene.id)
            .and_then(|entry| entry.accent_color.as_deref())
            .map(str::to_owned);

        let clear_accent_button = Button::builder()
            .icon_name("edit-clear-symbolic")
            .tooltip_text("Clear scene accent color")
            .valign(Align::Center)
            .sensitive(
                registry
                    .scenes
                    .get(&scene.id)
                    .and_then(|entry| entry.accent_color.as_ref())
                    .is_some(),
            )
            .build();
        clear_accent_button.add_css_class("flat");

        let accent_box = GtkBox::new(Orientation::Horizontal, 0);
        let accent_button = build_accent_button(
            &scene.id,
            current_accent.as_deref(),
            nav,
            &clear_accent_button,
        );
        accent_box.append(&accent_button);

        combo_row.connect_selected_notify({
            let scene_id = scene.id.clone();
            let nav = nav.clone();
            let clear_accent_button = clear_accent_button.clone();
            move |row| {
                handle_scene_role_change(row, &scene_id, &nav);
                clear_accent_button.set_sensitive(
                    nav.state
                        .borrow()
                        .registry
                        .scenes
                        .get(&scene_id)
                        .and_then(|entry| entry.accent_color.as_ref())
                        .is_some(),
                );
            }
        });

        clear_accent_button.connect_clicked({
            let scene_id = scene.id.clone();
            let nav = nav.clone();
            let clear_accent_button = clear_accent_button.clone();
            let accent_box = accent_box.clone();
            move |_| {
                if set_scene_accent(&nav, &scene_id, None) {
                    if let Some(previous_picker) = accent_box.first_child() {
                        accent_box.remove(&previous_picker);
                    }
                    let unset_picker =
                        build_accent_button(&scene_id, None, &nav, &clear_accent_button);
                    accent_box.append(&unset_picker);
                    clear_accent_button.set_sensitive(false);
                }
            }
        });

        combo_row.add_suffix(&drag_handle);
        combo_row.add_suffix(&accent_box);
        combo_row.add_suffix(&clear_accent_button);

        scenes_group.add(&combo_row);
    }

    page.add(&scenes_group);

    // ── Stale registry entries ────────────────────────────────────────────────

    let obs_ids: HashSet<&str> = inventory.scenes.iter().map(|s| s.id.as_str()).collect();

    let mut stale: Vec<(String, SceneEntry)> = registry
        .scenes
        .iter()
        .filter(|(name, _)| !obs_ids.contains(name.as_str()))
        .map(|(n, e)| (n.clone(), e.clone()))
        .collect();
    stale.sort_by(|(a, _), (b, _)| a.cmp(b));

    if !stale.is_empty() {
        let stale_group = PreferencesGroup::builder()
            .title(fl!(LANGUAGE_LOADER, "inventory-stale-group-title"))
            .description(fl!(LANGUAGE_LOADER, "inventory-stale-group-description"))
            .build();

        for (entry_name, entry) in stale {
            let stale_row = adw::ActionRow::builder()
                .title(entry_name.as_str())
                .subtitle(SceneRole::label_or_unassigned(entry.role))
                .build();

            let remove_btn = Button::builder()
                .icon_name("list-remove-symbolic")
                .tooltip_text(fl!(LANGUAGE_LOADER, "inventory-remove-stale-tooltip"))
                .valign(Align::Center)
                .build();
            remove_btn.add_css_class("flat");
            remove_btn.add_css_class("destructive-action");

            remove_btn.connect_clicked({
                let entry_name = entry_name.clone();
                let stale_row = stale_row.clone();
                let nav = nav.clone();
                move |_| handle_stale_entry_remove(entry_name.as_str(), &stale_row, &nav)
            });

            stale_row.add_suffix(&remove_btn);
            stale_group.add(&stale_row);
        }

        page.add(&stale_group);
    }

    container.append(&page);
}

fn reorder_scenes(
    nav: &NavigationContext,
    inventory_scene_ids: &[String],
    source_scene_id: &str,
    target_scene_id: &str,
    insert_after: bool,
) -> bool {
    if source_scene_id == target_scene_id {
        return false;
    }

    let registry = {
        let mut state = nav.state.borrow_mut();
        let order = state
            .registry
            .ordered_scene_ids(inventory_scene_ids.iter().map(String::as_str));
        let Some(order) =
            reordered_scene_ids(order, source_scene_id, target_scene_id, insert_after)
        else {
            return false;
        };
        if !state.registry.set_scene_order(order) {
            return false;
        }
        state.registry.clone()
    };

    crate::ui::background_io::run(
        move || write_registry(&registry),
        |result| {
            if let Err(error) = result {
                tracing::warn!(%error, "failed to persist scene order");
            }
        },
    );
    true
}

fn reordered_scene_ids(
    mut order: Vec<String>,
    source_scene_id: &str,
    target_scene_id: &str,
    insert_after: bool,
) -> Option<Vec<String>> {
    if source_scene_id == target_scene_id {
        return None;
    }
    let source_index = order.iter().position(|id| id == source_scene_id)?;
    order.remove(source_index);
    let target_index = order.iter().position(|id| id == target_scene_id)?;
    order.insert(
        target_index + usize::from(insert_after),
        source_scene_id.to_string(),
    );
    Some(order)
}

fn build_accent_button(
    scene_id: &str,
    accent: Option<&str>,
    nav: &NavigationContext,
    clear_accent_button: &Button,
) -> ColorButton {
    let button = ColorButton::new();
    button.set_title("Scene accent color");
    button.set_tooltip_text(Some("Choose scene accent color"));
    button.set_use_alpha(false);
    button.set_valign(Align::Center);
    if let Some((red, green, blue)) = accent.and_then(parse_scene_accent) {
        button.set_rgba(&gtk4::gdk::RGBA::new(
            f32::from(red) / 255.0,
            f32::from(green) / 255.0,
            f32::from(blue) / 255.0,
            1.0,
        ));
    }

    button.connect_rgba_notify({
        let scene_id = scene_id.to_string();
        let nav = nav.clone();
        let clear_accent_button = clear_accent_button.clone();
        move |button| {
            let rgba = button.rgba();
            let accent = scene_accent_hex(rgba.red(), rgba.green(), rgba.blue());
            if set_scene_accent(&nav, &scene_id, Some(accent)) {
                clear_accent_button.set_sensitive(true);
            }
        }
    });

    button
}

fn build_yaml_actions_row(container: &GtkBox, nav: &NavigationContext) -> ActionRow {
    let row = ActionRow::builder()
        .title(fl!(LANGUAGE_LOADER, "inventory-yaml-row-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "inventory-yaml-row-subtitle"))
        .build();

    let export_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "inventory-export-button-label"))
        .icon_name("document-save-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "inventory-export-tooltip"))
        .valign(Align::Center)
        .build();
    export_btn.add_css_class("flat");

    let import_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "inventory-import-button-label"))
        .icon_name("document-open-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "inventory-import-tooltip"))
        .valign(Align::Center)
        .build();
    import_btn.add_css_class("flat");

    export_btn.connect_clicked({
        let row = row.clone();
        let nav = nav.clone();
        move |button| handle_export_click(button, &row, &nav)
    });

    import_btn.connect_clicked({
        let row = row.clone();
        let container = container.clone();
        let nav = nav.clone();
        move |button| handle_import_click(button, &row, &container, &nav)
    });

    row.add_suffix(&export_btn);
    row.add_suffix(&import_btn);
    row
}

fn handle_scene_role_change(row: &ComboRow, scene_id: &str, nav: &NavigationContext) {
    let new_role = index_to_role(row.selected());
    let registry = {
        let mut state = nav.state.borrow_mut();
        match new_role {
            Some(role) => {
                state
                    .registry
                    .scenes
                    .entry(scene_id.to_string())
                    .and_modify(|e| e.role = Some(role))
                    .or_insert_with(|| SceneEntry {
                        role: Some(role),
                        tags: Vec::new(),
                        protected: false,
                        accent_color: None,
                    });
            }
            None => {
                state.registry.set_scene_role(scene_id, None);
            }
        }
        state.registry.clone()
    };
    let scene_id = scene_id.to_string();
    crate::ui::background_io::run(
        move || write_registry(&registry),
        move |result| {
            if let Err(error) = result {
                tracing::warn!(%error, scene = scene_id, "failed to write registry");
            }
        },
    );
    let subtitle = new_role
        .map(SceneRole::description)
        .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "inventory-no-role-assigned"));
    row.set_subtitle(&subtitle);
}

fn handle_stale_entry_remove(entry_name: &str, stale_row: &ActionRow, nav: &NavigationContext) {
    let registry = {
        let mut state = nav.state.borrow_mut();
        state.registry.scenes.remove(entry_name);
        state.registry.clone()
    };
    crate::ui::background_io::run(
        move || write_registry(&registry),
        |result| {
            if let Err(error) = result {
                tracing::warn!(%error, "failed to remove stale registry entry");
            }
        },
    );
    stale_row.set_visible(false);
}

fn set_scene_accent(nav: &NavigationContext, scene_id: &str, accent_color: Option<String>) -> bool {
    let registry = {
        let mut state = nav.state.borrow_mut();
        if !state.registry.scenes.contains_key(scene_id) && accent_color.is_some() {
            state.registry.scenes.insert(
                scene_id.to_string(),
                SceneEntry {
                    role: None,
                    tags: Vec::new(),
                    protected: false,
                    accent_color: None,
                },
            );
        }
        let Some(entry) = state.registry.scenes.get_mut(scene_id) else {
            return false;
        };
        if entry.accent_color == accent_color {
            return false;
        }
        entry.accent_color = accent_color;
        if entry.role.is_none()
            && entry.accent_color.is_none()
            && entry.tags.is_empty()
            && !entry.protected
        {
            state.registry.scenes.remove(scene_id);
        }
        state.registry.clone()
    };
    let scene_id = scene_id.to_string();
    crate::ui::background_io::run(
        move || write_registry(&registry),
        move |result| {
            if let Err(error) = result {
                tracing::warn!(%error, scene = scene_id, "failed to save scene accent");
            }
        },
    );
    true
}

fn handle_export_click(button: &Button, status_row: &ActionRow, nav: &NavigationContext) {
    show_export_dialog(button, status_row, nav);
}

fn handle_import_click(
    button: &Button,
    status_row: &ActionRow,
    container: &GtkBox,
    nav: &NavigationContext,
) {
    show_import_dialog(button, status_row, container, nav);
}

fn show_export_dialog(button: &Button, status_row: &ActionRow, nav: &NavigationContext) {
    let dialog = FileChooserNative::new(
        Some(&fl!(LANGUAGE_LOADER, "inventory-export-dialog-title")),
        parent_window(button).as_ref(),
        FileChooserAction::Save,
        Some(&fl!(LANGUAGE_LOADER, "inventory-export-button-label")),
        Some(&fl!(LANGUAGE_LOADER, "inventory-dialog-cancel-label")),
    );
    dialog.set_modal(true);
    dialog.set_current_name("scenedeck-registry.yaml");
    dialog.set_filter(&yaml_file_filter());

    let status_row = status_row.clone();
    let registry = nav.state.borrow().registry.clone();
    dialog.run_async(move |dialog, response| {
        if response == ResponseType::Accept {
            match dialog.file().and_then(|file| file.path()) {
                Some(path) => {
                    let path = ensure_yaml_extension(path);
                    let status_row = status_row.clone();
                    let display_path = path.display().to_string();
                    crate::ui::background_io::run(
                        move || write_registry_yaml_to_path(&path, &registry),
                        move |result| match result {
                            Ok(()) => status_row.set_subtitle(&fl!(
                                LANGUAGE_LOADER,
                                "inventory-export-success",
                                path = display_path
                            )),
                            Err(error) => status_row.set_subtitle(&fl!(
                                LANGUAGE_LOADER,
                                "inventory-export-error",
                                error = error.to_string()
                            )),
                        },
                    );
                }
                None => status_row.set_subtitle(&fl!(LANGUAGE_LOADER, "inventory-export-no-file")),
            }
        }
        dialog.destroy();
    });
}

fn show_import_dialog(
    button: &Button,
    status_row: &ActionRow,
    container: &GtkBox,
    nav: &NavigationContext,
) {
    let dialog = FileChooserNative::new(
        Some(&fl!(LANGUAGE_LOADER, "inventory-import-dialog-title")),
        parent_window(button).as_ref(),
        FileChooserAction::Open,
        Some(&fl!(LANGUAGE_LOADER, "inventory-import-button-label")),
        Some(&fl!(LANGUAGE_LOADER, "inventory-dialog-cancel-label")),
    );
    dialog.set_modal(true);
    dialog.set_filter(&yaml_file_filter());

    let status_row = status_row.clone();
    let container = container.clone();
    let nav = nav.clone();
    dialog.run_async(move |dialog, response| {
        if response == ResponseType::Accept {
            match dialog.file().and_then(|file| file.path()) {
                Some(path) => {
                    let status_row = status_row.clone();
                    let container = container.clone();
                    let nav = nav.clone();
                    crate::ui::background_io::run(
                        move || {
                            read_registry_yaml_from_path(&path).and_then(|registry| {
                                write_registry(&registry)?;
                                Ok(registry)
                            })
                        },
                        move |result| match result {
                            Ok(registry) => {
                                nav.state.borrow_mut().registry = registry;
                                rebuild(&container, &nav);
                            }
                            Err(error) => status_row.set_subtitle(&fl!(
                                LANGUAGE_LOADER,
                                "inventory-import-error",
                                error = error.to_string()
                            )),
                        },
                    );
                }
                None => status_row.set_subtitle(&fl!(LANGUAGE_LOADER, "inventory-import-no-file")),
            }
        }
        dialog.destroy();
    });
}

fn yaml_file_filter() -> FileFilter {
    let filter = FileFilter::new();
    filter.set_name(Some(&fl!(LANGUAGE_LOADER, "inventory-yaml-filter-name")));
    filter.add_pattern("*.yaml");
    filter.add_pattern("*.yml");
    filter
}

fn parent_window(button: &Button) -> Option<gtk4::Window> {
    button
        .root()
        .and_then(|root| root.downcast::<gtk4::Window>().ok())
}

fn ensure_yaml_extension(mut path: PathBuf) -> PathBuf {
    let has_yaml_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
        .unwrap_or(false);

    if !has_yaml_extension {
        path.set_extension("yaml");
    }

    path
}

#[cfg(test)]
mod tests {
    use super::reordered_scene_ids;

    fn order() -> Vec<String> {
        ["One", "Two", "Three", "Four"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn dragging_to_top_half_inserts_before_target() {
        assert_eq!(
            reordered_scene_ids(order(), "Four", "Two", false).unwrap(),
            ["One", "Four", "Two", "Three"]
        );
    }

    #[test]
    fn dragging_to_bottom_half_inserts_after_target() {
        assert_eq!(
            reordered_scene_ids(order(), "One", "Three", true).unwrap(),
            ["Two", "Three", "One", "Four"]
        );
    }

    #[test]
    fn dropping_scene_on_itself_does_not_reorder() {
        assert!(reordered_scene_ids(order(), "Two", "Two", false).is_none());
    }
}
