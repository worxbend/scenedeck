//! Graph page — scene dependency visualisation.
//!
//! Shows each scene that nests other scenes as an expandable row, with one
//! child row per nested scene.  Each child is annotated with an icon showing
//! whether the dependency satisfies the role rules (`graph_service`).

use std::rc::Rc;

use adw::{prelude::*, ActionRow, ExpanderRow, PreferencesGroup, PreferencesPage, StatusPage};
use gtk4::{Box as GtkBox, Image, Orientation};

use crate::domain::role::SceneRole;
use crate::services::graph_service::classify_edge;
use crate::storage::registry::read_registry;
use crate::ui::navigation::NavigationContext;

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

fn populate(container: &GtkBox, nav: &NavigationContext) {
    let (graph, inventory) = {
        let state = nav.state.borrow();
        (state.scene_graph.clone(), state.scene_inventory.clone())
    };

    if graph.edges.is_empty() {
        let empty = StatusPage::builder()
            .icon_name("view-grid-symbolic")
            .title("No Dependencies")
            .description(
                "No scenes nest other scenes, or OBS is not connected. \
                 Connect and add nested scene sources to see the dependency graph.",
            )
            .build();
        container.append(&empty);
        return;
    }

    let registry = read_registry();
    let role_of =
        |scene_id: &str| -> Option<SceneRole> { registry.scenes.get(scene_id).map(|e| e.role) };

    let page = PreferencesPage::builder()
        .title("Graph")
        .vexpand(true)
        .build();

    let group = PreferencesGroup::builder()
        .title("Scene Dependencies")
        .description("Each scene's nested scene sources, validated against the role rules.")
        .build();

    // Stable ordering: follow the OBS scene list order where possible.
    let mut parents: Vec<&String> = graph.edges.keys().collect();
    parents.sort_by_key(|name| {
        inventory
            .scenes
            .iter()
            .position(|s| &s.id == *name)
            .unwrap_or(usize::MAX)
    });

    for parent in parents {
        let parent_role = role_of(parent);
        let subtitle = parent_role.map(SceneRole::label).unwrap_or("Unassigned");

        let expander = ExpanderRow::builder()
            .title(parent.as_str())
            .subtitle(subtitle)
            .build();

        for child in graph.children(parent) {
            let child_role = role_of(child);
            let status = classify_edge(parent_role, child_role, &registry.rules);

            let icon = Image::from_icon_name(status.icon_name());
            icon.add_css_class(status.css_class());

            let child_subtitle = child_role.map(SceneRole::label).unwrap_or("Unassigned");
            let row = ActionRow::builder()
                .title(child.as_str())
                .subtitle(child_subtitle)
                .build();
            row.add_prefix(&icon);
            expander.add_row(&row);
        }

        group.add(&expander);
    }

    page.add(&group);
    container.append(&page);
}
