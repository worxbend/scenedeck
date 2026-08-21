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
use crate::domain::scene::SceneInventory;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::registry::{
    self as registry_storage, parse_scene_accent, read_registry_yaml_from_path, scene_accent_hex,
    write_registry, write_registry_yaml_to_path, SceneEntry, SceneRegistry,
};
use crate::ui::navigation::NavigationContext;
use crate::ui::persist::persist_registry;
use crate::ui::widgets::icon_picker;
use crate::ui::{index_of, string_list};
use i18n_embed_fl::fl;

// ── Role index helpers ────────────────────────────────────────────────────────

// The role dropdown lists "Unassigned" first and then `SceneRole::ALL`, so a
// row's index is one more than the role's position. That offset used to be
// spelled out as two seven-arm tables, which had to be kept in step with each
// other, with `SceneRole::ALL`, and with the model built at the call site.
// Below it is stated once in each direction.

/// Dropdown row for a scene's current role. Row 0 is "Unassigned".
fn role_row(role: Option<SceneRole>) -> u32 {
    role.map_or(0, |role| index_of(&SceneRole::ALL, role) + 1)
}

/// The role a dropdown row selects, or `None` for "Unassigned".
///
/// Row 0 and anything past the end of the list — including the `u32::MAX` GTK
/// reports when nothing is selected — mean no role, matching what the table
/// this replaced did with its `_` arm.
fn role_at(row: u32) -> Option<SceneRole> {
    row.checked_sub(1)
        .and_then(|index| SceneRole::ALL.get(index as usize).copied())
}

// ── Public entry point ────────────────────────────────────────────────────────

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    crate::ui::rebuildable_page("inventory-page", false, move |container| {
        populate(container, &nav);
    })
}

/// Refill the page in place.
///
/// Kept as its own function, unlike the Graph page, because `populate` wires
/// it to the drag-reorder handler and the registry-import completion, both of
/// which need to rebuild from inside `populate` — where the refresh callback
/// that `rebuildable_page` returns is not reachable.
fn rebuild(container: &GtkBox, nav: &NavigationContext) {
    crate::ui::clear_children(container);
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
        let role_model = string_list(&role_labels);

        let combo_row = ComboRow::builder()
            .title(scene.name.as_str())
            .subtitle(subtitle)
            .model(&role_model)
            .selected(role_row(current_role))
            .build();
        combo_row.add_css_class("scenedeck-combo-row");

        let drag_handle =
            install_scene_reordering(&combo_row, &scene.id, &inventory_scene_ids, nav, container);

        let (accent_box, clear_accent_button) = build_accent_controls(&scene.id, &registry, nav);

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

        let icon_picker = icon_picker::build(
            registry.scene_icon(&scene.id),
            &fl!(LANGUAGE_LOADER, "inventory-scene-icon-tooltip"),
            icon_picker::PickerDisplay::CurrentIcon,
            {
                let scene_id = scene.id.clone();
                let nav = nav.clone();
                move |icon| set_scene_icon(&nav, &scene_id, icon.as_deref())
            },
        );
        combo_row.add_prefix(&icon_picker);

        combo_row.add_suffix(&drag_handle);
        combo_row.add_suffix(&accent_box);
        combo_row.add_suffix(&clear_accent_button);

        scenes_group.add(&combo_row);
    }

    page.add(&scenes_group);

    if let Some(stale_group) = build_stale_group(&inventory, &registry, nav) {
        page.add(&stale_group);
    }

    container.append(&page);
}

/// The accent-colour picker for one scene row, and its clear button.
///
/// The two are built together because they drive each other: choosing a colour
/// enables the clear button, and clearing puts an empty picker back in place.
fn build_accent_controls(
    scene_id: &str,
    registry: &SceneRegistry,
    nav: &NavigationContext,
) -> (GtkBox, Button) {
    let current_accent = registry
        .scenes
        .get(scene_id)
        .and_then(|entry| entry.accent_color.as_deref())
        .map(str::to_owned);

    let clear_accent_button = Button::builder()
        .icon_name("edit-clear-symbolic")
        .tooltip_text("Clear scene accent color")
        .valign(Align::Center)
        .sensitive(
            registry
                .scenes
                .get(scene_id)
                .and_then(|entry| entry.accent_color.as_ref())
                .is_some(),
        )
        .build();
    clear_accent_button.add_css_class("flat");

    let accent_box = GtkBox::new(Orientation::Horizontal, 0);
    let accent_button = build_accent_button(
        scene_id,
        current_accent.as_deref(),
        nav,
        &clear_accent_button,
    );
    accent_box.append(&accent_button);

    clear_accent_button.connect_clicked({
        let scene_id = scene_id.to_string();
        let nav = nav.clone();
        let clear_accent_button = clear_accent_button.clone();
        let accent_box = accent_box.clone();
        move |_| {
            if set_scene_accent(&nav, &scene_id, None) {
                if let Some(previous_picker) = accent_box.first_child() {
                    accent_box.remove(&previous_picker);
                }
                let unset_picker = build_accent_button(&scene_id, None, &nav, &clear_accent_button);
                accent_box.append(&unset_picker);
                clear_accent_button.set_sensitive(false);
            }
        }
    });

    (accent_box, clear_accent_button)
}

/// Make one scene row draggable, and a drop target for the others.
///
/// Returns the drag handle for the caller to place; the drop target is
/// attached to the whole row, so a scene can be dropped anywhere on it.
/// Whether it lands before or after depends on which half of the row the
/// pointer is in when it is released.
fn install_scene_reordering(
    combo_row: &ComboRow,
    scene_id: &str,
    inventory_scene_ids: &[String],
    nav: &NavigationContext,
    container: &GtkBox,
) -> Image {
    let drag_handle = Image::from_icon_name("list-drag-handle-symbolic");
    drag_handle.set_tooltip_text(Some("Drag to reorder scene"));
    drag_handle.set_valign(Align::Center);
    drag_handle.add_css_class("dim-label");

    let drag_source = DragSource::builder().actions(gdk::DragAction::MOVE).build();
    drag_source.connect_prepare({
        let scene_id = scene_id.to_string();
        move |_, _, _| Some(gdk::ContentProvider::for_value(&scene_id.to_value()))
    });
    drag_handle.add_controller(drag_source);

    let drop_target = DropTarget::new(String::static_type(), gdk::DragAction::MOVE);
    drop_target.connect_drop({
        let target_scene_id = scene_id.to_string();
        let inventory_scene_ids = inventory_scene_ids.to_vec();
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

    drag_handle
}

/// Build the group listing registry entries whose scene is gone from OBS.
///
/// Returns `None` when nothing is stale, so the caller adds no empty group.
///
/// An entry goes stale when a scene is renamed or deleted in OBS: SceneDeck
/// keys its registry by scene name, so the old name is left holding a role,
/// an accent, and an icon that no longer point at anything.
fn build_stale_group(
    inventory: &SceneInventory,
    registry: &SceneRegistry,
    nav: &NavigationContext,
) -> Option<PreferencesGroup> {
    let obs_ids: HashSet<&str> = inventory.scenes.iter().map(|s| s.id.as_str()).collect();

    let mut stale: Vec<(String, SceneEntry)> = registry
        .scenes
        .iter()
        .filter(|(name, _)| !obs_ids.contains(name.as_str()))
        .map(|(n, e)| (n.clone(), e.clone()))
        .collect();
    if stale.is_empty() {
        return None;
    }
    stale.sort_by(|(a, _), (b, _)| a.cmp(b));

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

    Some(stale_group)
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

    persist_registry(nav, "scene order", |registry| {
        let order = registry.ordered_scene_ids(inventory_scene_ids.iter().map(String::as_str));
        let Some(order) =
            reordered_scene_ids(order, source_scene_id, target_scene_id, insert_after)
        else {
            return false;
        };
        registry.set_scene_order(order)
    })
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
    let new_role = role_at(row.selected());
    // Both directions go through the registry: it owns when an entry is
    // created and when clearing the last piece of metadata removes it.
    persist_registry(nav, "scene role", |registry| {
        registry.set_scene_role(scene_id, new_role)
    });

    let subtitle = new_role
        .map(SceneRole::description)
        .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "inventory-no-role-assigned"));
    row.set_subtitle(&subtitle);
}

fn handle_stale_entry_remove(entry_name: &str, stale_row: &ActionRow, nav: &NavigationContext) {
    persist_registry(nav, "stale entry removal", |registry| {
        registry.scenes.remove(entry_name).is_some()
    });

    stale_row.set_visible(false);
}

/// Persist one scene's icon and mirror it into the cached registry.
///
/// Failures are logged rather than surfaced: the picker has already moved, and
/// the next Inventory rebuild reads back from the registry either way.
fn set_scene_icon(nav: &NavigationContext, scene_id: &str, icon: Option<&str>) {
    let owned_id = scene_id.to_string();
    let owned_icon = icon.map(str::to_string);
    crate::ui::persist::persist_registry_field(
        nav,
        "scene icon",
        |registry| registry.set_scene_icon(scene_id, icon),
        move || registry_storage::set_scene_icon(&owned_id, owned_icon.as_deref()),
    );
}

/// Save a scene's accent colour, and report whether anything changed.
///
/// The rule for what an accent change does to the entry — create it, update
/// it, or remove an entry that is now empty — belongs to `SceneRegistry`. This
/// only mirrors the result into the cached snapshot and writes it out.
fn set_scene_accent(nav: &NavigationContext, scene_id: &str, accent_color: Option<String>) -> bool {
    persist_registry(nav, "scene accent", |registry| {
        registry.set_scene_accent(scene_id, accent_color)
    })
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
