//! Inventory page — all scenes grouped by role.  Phase 5 implementation.

use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use adw::{prelude::*, ActionRow, ComboRow, PreferencesGroup, PreferencesPage, StatusPage};
use gtk4::{
    Align, Box as GtkBox, Button, FileChooserAction, FileChooserNative, FileFilter, Orientation,
    ResponseType,
};

use crate::domain::role::SceneRole;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::registry::{
    read_registry, read_registry_yaml_from_path, write_registry, write_registry_yaml_to_path,
    SceneEntry,
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

    let registry = read_registry();

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

    for scene in &inventory.scenes {
        // Look up the current role from the registry (source of truth).
        let current_role = registry.scenes.get(&scene.id).map(|e| e.role);

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

        combo_row.connect_selected_notify({
            let scene_id = scene.id.clone();
            move |row| {
                handle_scene_role_change(row, &scene_id);
            }
        });

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
                .subtitle(entry.role.label())
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
                move |_| handle_stale_entry_remove(entry_name.as_str(), &stale_row)
            });

            stale_row.add_suffix(&remove_btn);
            stale_group.add(&stale_row);
        }

        page.add(&stale_group);
    }

    container.append(&page);
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
        move |button| handle_export_click(button, &row)
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

fn handle_scene_role_change(row: &ComboRow, scene_id: &str) {
    let new_role = index_to_role(row.selected());
    let mut reg = read_registry();
    match new_role {
        Some(role) => {
            reg.scenes
                .entry(scene_id.to_string())
                .and_modify(|e| e.role = role)
                .or_insert_with(|| SceneEntry {
                    role,
                    tags: Vec::new(),
                    protected: false,
                });
        }
        None => {
            reg.scenes.remove(scene_id);
        }
    }

    if let Err(e) = write_registry(&reg) {
        tracing::warn!(%e, scene = scene_id, "failed to write registry");
    }
    let subtitle = new_role
        .map(SceneRole::description)
        .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "inventory-no-role-assigned"));
    row.set_subtitle(&subtitle);
}

fn handle_stale_entry_remove(entry_name: &str, stale_row: &ActionRow) {
    let mut reg = read_registry();
    reg.scenes.remove(entry_name);
    let _ = write_registry(&reg);
    stale_row.set_visible(false);
}

fn handle_export_click(button: &Button, status_row: &ActionRow) {
    show_export_dialog(button, status_row);
}

fn handle_import_click(
    button: &Button,
    status_row: &ActionRow,
    container: &GtkBox,
    nav: &NavigationContext,
) {
    show_import_dialog(button, status_row, container, nav);
}

fn show_export_dialog(button: &Button, status_row: &ActionRow) {
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
    dialog.run_async(move |dialog, response| {
        if response == ResponseType::Accept {
            match dialog.file().and_then(|file| file.path()) {
                Some(path) => {
                    let path = ensure_yaml_extension(path);
                    let registry = read_registry();
                    match write_registry_yaml_to_path(&path, &registry) {
                        Ok(()) => status_row.set_subtitle(&fl!(
                            LANGUAGE_LOADER,
                            "inventory-export-success",
                            path = path.display().to_string()
                        )),
                        Err(err) => status_row.set_subtitle(&fl!(
                            LANGUAGE_LOADER,
                            "inventory-export-error",
                            error = err.to_string()
                        )),
                    }
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
                Some(path) => match read_registry_yaml_from_path(&path)
                    .and_then(|registry| write_registry(&registry))
                {
                    Ok(()) => rebuild(&container, &nav),
                    Err(err) => status_row.set_subtitle(&fl!(
                        LANGUAGE_LOADER,
                        "inventory-import-error",
                        error = err.to_string()
                    )),
                },
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
