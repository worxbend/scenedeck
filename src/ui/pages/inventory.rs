//! Inventory page — all scenes grouped by role.  Phase 5 implementation.

use std::collections::HashSet;
use std::rc::Rc;

use adw::{prelude::*, ComboRow, PreferencesGroup, PreferencesPage, StatusPage};
use gtk4::{Align, Box as GtkBox, Button, Orientation};

use crate::domain::role::SceneRole;
use crate::storage::registry::{read_registry, write_registry, SceneEntry};
use crate::ui::navigation::NavigationContext;

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

    populate(&container, &nav);

    let refresh_fn: Rc<dyn Fn()> = Rc::new({
        let nav = nav.clone();
        let container = container.clone();
        move || {
            while let Some(child) = container.first_child() {
                container.remove(&child);
            }
            populate(&container, &nav);
        }
    });

    container.connect_map({
        let refresh = refresh_fn.clone();
        move |_| refresh()
    });

    (container.upcast(), refresh_fn)
}

// ── Page population ───────────────────────────────────────────────────────────

fn populate(container: &GtkBox, nav: &NavigationContext) {
    let inventory = nav.state.borrow().scene_inventory.clone();

    // Empty state — OBS not yet connected or no scenes.
    if inventory.scenes.is_empty() {
        let empty = StatusPage::builder()
            .icon_name("view-list-symbolic")
            .title("No Scenes")
            .description("Connect to OBS to load the scene list.")
            .build();
        container.append(&empty);
        return;
    }

    let registry = read_registry();

    let page = PreferencesPage::builder()
        .title("Inventory")
        .vexpand(true)
        .build();

    // ── OBS Scenes group ──────────────────────────────────────────────────────

    let scenes_group = PreferencesGroup::builder()
        .title("OBS Scenes")
        .description("Assign roles to control which scenes appear on the Live page.")
        .build();

    for scene in &inventory.scenes {
        // Look up the current role from the registry (source of truth).
        let current_role = registry.scenes.get(&scene.id).map(|e| e.role);

        let subtitle = current_role
            .map(SceneRole::description)
            .unwrap_or("No role assigned");

        let role_model = gtk4::StringList::new(&[
            "Unassigned",
            "Primary",
            "Secondary",
            "Module",
            "Raw",
            "Debug",
            "Archive",
        ]);

        let combo_row = ComboRow::builder()
            .title(scene.name.as_str())
            .subtitle(subtitle)
            .model(&role_model)
            .selected(role_to_index(current_role))
            .build();

        combo_row.connect_selected_notify({
            let scene_id = scene.id.clone();
            move |row| {
                let new_role = index_to_role(row.selected());
                let mut reg = read_registry();
                match new_role {
                    Some(role) => {
                        reg.scenes
                            .entry(scene_id.clone())
                            .and_modify(|e| e.role = role)
                            .or_insert_with(|| SceneEntry {
                                role,
                                tags: Vec::new(),
                                protected: false,
                            });
                    }
                    None => {
                        reg.scenes.remove(&scene_id);
                    }
                }
                if let Err(e) = write_registry(&reg) {
                    tracing::warn!(%e, "failed to write registry");
                }
                // Update subtitle to reflect the new role description.
                let subtitle = new_role
                    .map(SceneRole::description)
                    .unwrap_or("No role assigned");
                row.set_subtitle(subtitle);
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
            .title("Stale Registry Entries")
            .description("These scenes are in your local registry but no longer exist in OBS.")
            .build();

        for (entry_name, entry) in stale {
            let stale_row = adw::ActionRow::builder()
                .title(entry_name.as_str())
                .subtitle(entry.role.label())
                .build();

            let remove_btn = Button::builder()
                .icon_name("list-remove-symbolic")
                .tooltip_text("Remove stale entry")
                .valign(Align::Center)
                .build();
            remove_btn.add_css_class("flat");
            remove_btn.add_css_class("destructive-action");

            remove_btn.connect_clicked({
                let entry_name = entry_name.clone();
                let stale_row = stale_row.clone();
                move |_| {
                    let mut reg = read_registry();
                    reg.scenes.remove(&entry_name);
                    let _ = write_registry(&reg);
                    stale_row.set_visible(false);
                }
            });

            stale_row.add_suffix(&remove_btn);
            stale_group.add(&stale_row);
        }

        page.add(&stale_group);
    }

    container.append(&page);
}
