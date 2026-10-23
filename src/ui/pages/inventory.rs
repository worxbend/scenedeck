//! Inventory page — responsive scene list with local metadata and ordering.

use std::collections::HashSet;
use std::rc::Rc;

use adw::{prelude::*, ActionRow, PreferencesGroup, StatusPage};
use gtk4::{
    gdk, Align, Box as GtkBox, Button, ColorDialog, ColorDialogButton, DragSource, DropDown,
    DropTarget, FileDialog, FileFilter, Image, Label, MenuButton, Orientation, PolicyType, Popover,
    ScrolledWindow, SignalListItemFactory,
};

use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::storage::registry::{
    self as registry_storage, parse_scene_accent, scene_accent_hex, write_registry_yaml_to_path,
    SceneEntry, SceneRegistry,
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

#[derive(Clone, PartialEq, Eq)]
struct ScenePresentation {
    id: String,
    role: Option<SceneRole>,
    icon: Option<String>,
    accent: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
struct InventoryPresentation {
    scope: crate::domain::workflow::ScopeBinding,
    epoch: u64,
    language: crate::domain::appearance::Language,
    rows: Vec<ScenePresentation>,
    stale: Vec<(String, Option<SceneRole>)>,
}

impl InventoryPresentation {
    fn read(nav: &NavigationContext) -> Self {
        let state = nav.state.borrow();
        let ids: HashSet<&str> = state
            .scene_inventory
            .scenes
            .iter()
            .map(|scene| scene.id.as_str())
            .collect();
        let rows = state
            .registry
            .ordered_scene_ids(
                state
                    .scene_inventory
                    .scenes
                    .iter()
                    .map(|scene| scene.id.as_str()),
            )
            .into_iter()
            .map(|id| {
                let entry = state.registry.scenes.get(&id);
                ScenePresentation {
                    role: entry.and_then(|entry| entry.role),
                    icon: entry.and_then(|entry| entry.icon.clone()),
                    accent: entry.and_then(|entry| entry.accent_color.clone()),
                    id,
                }
            })
            .collect();
        let mut stale: Vec<_> = state
            .registry
            .scenes
            .iter()
            .filter(|(id, _)| !ids.contains(id.as_str()))
            .map(|(id, entry)| (id.clone(), entry.role))
            .collect();
        stale.sort_by(|left, right| left.0.cmp(&right.0));
        Self {
            scope: state.workflow_scope(),
            epoch: state.workspace.epoch,
            language: state.config.language,
            rows,
            stale,
        }
    }
}

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    container.add_css_class("app-page");
    container.add_css_class("inventory-page");
    let rendered = std::cell::RefCell::new(None::<InventoryPresentation>);
    let compact = glib::WeakRef::<adw::Breakpoint>::new();
    let refresh: Rc<dyn Fn()> = Rc::new({
        let container = container.downgrade();
        move || {
            let Some(container) = container.upgrade() else {
                return;
            };
            let next = InventoryPresentation::read(&nav);
            let previous = rendered.borrow().clone();
            if previous.as_ref() == Some(&next) {
                return;
            }
            let incremental = previous.as_ref().is_some_and(|previous| {
                previous.scope == next.scope
                    && previous.epoch == next.epoch
                    && previous.language == next.language
                    && !previous.rows.is_empty()
                    && !next.rows.is_empty()
            });
            if let Some((previous, compact)) = previous
                .as_ref()
                .zip(compact.upgrade())
                .filter(|_| incremental)
            {
                update_rows(&container, &nav, previous, &next, &compact);
            } else {
                crate::ui::rebuild_page(&container, |container| {
                    populate(container, &nav, &compact)
                });
            }
            *rendered.borrow_mut() = Some(next);
        }
    });
    // Local reorder/import/removal actions and OBS readbacks share one reconciler.
    let actions = gio::SimpleActionGroup::new();
    let action = gio::SimpleAction::new("refresh", None);
    action.connect_activate({
        let refresh = refresh.clone();
        move |_, _| refresh()
    });
    actions.add_action(&action);
    container.insert_action_group("inventory", Some(&actions));
    refresh();
    container.connect_map({
        let refresh = refresh.clone();
        move |_| refresh()
    });
    (container.upcast(), refresh)
}

fn rebuild(container: &GtkBox, _nav: &NavigationContext) {
    container
        .activate_action("inventory.refresh", None)
        .expect("Inventory refresh action");
}

fn inventory_widget(root: &impl IsA<gtk4::Widget>, name: &str) -> Option<gtk4::Widget> {
    let root = root.as_ref();
    if root.widget_name() == name {
        return Some(root.clone());
    }
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Some(found) = inventory_widget(&widget, name) {
            return Some(found);
        }
        child = widget.next_sibling();
    }
    None
}

fn update_rows(
    container: &GtkBox,
    nav: &NavigationContext,
    previous: &InventoryPresentation,
    next: &InventoryPresentation,
    compact: &adw::Breakpoint,
) {
    let list = inventory_widget(container, "inventory-scenes")
        .unwrap()
        .downcast::<gtk4::ListBox>()
        .unwrap();
    let ordered: Vec<_> = next.rows.iter().map(|row| row.id.clone()).collect();
    let order_changed = previous
        .rows
        .iter()
        .map(|row| &row.id)
        .ne(next.rows.iter().map(|row| &row.id));
    let mut existing = std::collections::HashMap::new();
    let mut child = list.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        if let Some(id) = widget.widget_name().strip_prefix("scene-order-") {
            existing.insert(
                id.to_string(),
                widget.downcast::<gtk4::ListBoxRow>().unwrap(),
            );
        }
    }
    let desired: std::collections::HashMap<_, _> =
        next.rows.iter().map(|row| (row.id.as_str(), row)).collect();
    for row in &previous.rows {
        let replace = desired
            .get(row.id.as_str())
            .is_none_or(|next| next.icon != row.icon || next.accent != row.accent);
        if replace {
            if let Some(row) = existing.remove(&row.id) {
                list.remove(&row);
            }
        }
    }
    for (position, scene) in next.rows.iter().enumerate() {
        let row = existing.remove(&scene.id);
        if let Some(row) = row {
            if row.index() != position as i32 {
                list.remove(&row);
                list.insert(&row, position as i32);
            }
            let selector = inventory_widget(&row, &format!("inventory-role-{}", scene.id))
                .unwrap()
                .downcast::<DropDown>()
                .unwrap();
            if selector.selected() != role_row(scene.role) {
                selector.set_selected(role_row(scene.role));
            }
            if order_changed {
                let options =
                    inventory_widget(&row, &format!("inventory-options-{}", scene.id)).unwrap();
                let controls = options.parent().unwrap().downcast::<GtkBox>().unwrap();
                controls.remove(&options);
                controls.append(&build_scene_options(&scene.id, &ordered, nav, container));
            }
        } else {
            list.insert(
                &build_scene_row(&scene.id, &ordered, nav, container, compact),
                position as i32,
            );
        }
    }
    if previous.stale != next.stale {
        let page = inventory_widget(container, "inventory-content")
            .unwrap()
            .downcast::<GtkBox>()
            .unwrap();
        if let Some(stale) = inventory_widget(&page, "inventory-stale") {
            page.remove(&stale);
        }
        let state = nav.state.borrow();
        let group = build_stale_group(&state.scene_inventory, &state.registry, container, nav);
        drop(state);
        if let Some(group) = group {
            page.append(&group);
        }
    }
}

// ── Page population ───────────────────────────────────────────────────────────

fn populate(
    container: &GtkBox,
    nav: &NavigationContext,
    compact_ref: &glib::WeakRef<adw::Breakpoint>,
) {
    container.append(&build_registry_save_status());
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

    let compact = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        720.0,
        adw::LengthUnit::Sp,
    ));
    compact_ref.set(Some(&compact));
    let page = GtkBox::new(Orientation::Vertical, 16);
    page.set_widget_name("inventory-content");
    page.set_margin_start(16);
    page.set_margin_end(16);
    page.set_margin_top(16);
    page.set_margin_bottom(24);
    page.append(&build_yaml_actions_row(container, nav, &compact));

    let title = inventory_label(&fl!(LANGUAGE_LOADER, "inventory-scenes-group-title"));
    title.add_css_class("heading");
    page.append(&title);
    let description = inventory_label(&fl!(LANGUAGE_LOADER, "inventory-scenes-group-description"));
    description.add_css_class("dim-label");
    page.append(&description);

    // A single native breakpoint controls the page, in text-relative units.
    // GTK's boxes retain the very same widgets/focus as rows stack; no copies,
    // reparenting, per-row timers, column sizing or config writes are involved.
    // Give the bin an explicit minimal allocation; content keeps its natural
    // height inside the vertical scrolling viewport.
    let responsive = adw::BreakpointBin::builder()
        .width_request(1)
        .height_request(1)
        .vexpand(true)
        .build();
    let ordered_scene_ids =
        registry.ordered_scene_ids(inventory.scenes.iter().map(|scene| scene.id.as_str()));
    let scenes = gtk4::ListBox::new();
    scenes.set_widget_name("inventory-scenes");
    scenes.set_selection_mode(gtk4::SelectionMode::None);
    scenes.add_css_class("boxed-list");
    scenes.add_css_class("inventory-list");
    for scene_id in &ordered_scene_ids {
        scenes.append(&build_scene_row(
            scene_id,
            &ordered_scene_ids,
            nav,
            container,
            &compact,
        ));
    }
    page.append(&scenes);
    if let Some(stale_group) = build_stale_group(&inventory, &registry, container, nav) {
        page.append(&stale_group);
    }
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vexpand(true)
        .child(&page)
        .build();
    scroll.set_widget_name("inventory-scroll");
    responsive.set_child(Some(&scroll));
    preserve_breakpoint_focus(&responsive);
    responsive.add_breakpoint(compact);
    container.append(&responsive);
}

/// BreakpointBin temporarily hides its child while changing layouts. Remember
/// the editor before that hide, rather than its transient fallback focus, and
/// restore it on the frame that makes the child visible again.
fn preserve_breakpoint_focus(bin: &adw::BreakpointBin) {
    let focused = Rc::new(glib::WeakRef::<gtk4::Widget>::new());
    let transitioning = Rc::new(std::cell::Cell::new(false));
    let subscription = Rc::new(std::cell::RefCell::new(
        None::<(glib::WeakRef<gtk4::Window>, glib::SignalHandlerId)>,
    ));
    bin.connect_map({
        let focused = focused.clone();
        let transitioning = transitioning.clone();
        let subscription = subscription.clone();
        move |bin| {
            let Some(window) = bin.root().and_downcast::<gtk4::Window>() else {
                return;
            };
            let handler = window.connect_focus_widget_notify({
                let bin = bin.downgrade();
                let focused = focused.clone();
                let transitioning = transitioning.clone();
                move |window| {
                    let Some(bin) = bin.upgrade() else {
                        return;
                    };
                    if transitioning.get()
                        || bin.child().is_some_and(|child| !child.is_child_visible())
                    {
                        return;
                    }
                    let focus = gtk4::prelude::RootExt::focus(window)
                        .filter(|widget| widget.is_ancestor(&bin));
                    focused.set(focus.as_ref());
                }
            });
            *subscription.borrow_mut() = Some((window.downgrade(), handler));
        }
    });
    bin.connect_unmap({
        let subscription = subscription.clone();
        move |_| {
            if let Some((window, handler)) = subscription.borrow_mut().take() {
                if let Some(window) = window.upgrade() {
                    window.disconnect(handler);
                }
            }
        }
    });
    bin.connect_current_breakpoint_notify(move |bin| {
        if transitioning.replace(true) {
            return;
        }
        let focused = focused.clone();
        let transitioning = transitioning.clone();
        bin.add_tick_callback(move |bin, _| {
            if bin.child().is_some_and(|child| !child.is_child_visible()) {
                return glib::ControlFlow::Continue;
            }
            if let Some(focus) = focused.upgrade() {
                focus.grab_focus();
            }
            transitioning.set(false);
            glib::ControlFlow::Break
        });
    });
}

/// Persistence is application-owned; this view observes it weakly and can be
/// torn down during a save without dropping accepted intent or retry evidence.
fn build_registry_save_status() -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 12);
    row.set_widget_name("inventory-registry-save-status");
    let label = inventory_label("");
    let retry = Button::with_label(&fl!(LANGUAGE_LOADER, "window-connect-btn-retry"));
    retry.set_widget_name("inventory-registry-save-retry");
    retry.connect_clicked(|_| {
        crate::ui::persist::retry_registry_writes();
    });
    row.append(&label);
    row.append(&retry);
    let refresh = {
        let row = row.downgrade();
        let label = label.downgrade();
        let retry = retry.downgrade();
        move || {
            let (Some(row), Some(label), Some(retry)) =
                (row.upgrade(), label.upgrade(), retry.upgrade())
            else {
                return glib::ControlFlow::Break;
            };
            match crate::ui::persist::registry_save_status() {
                crate::ui::persist::RegistrySaveStatus::Saved => row.set_visible(false),
                crate::ui::persist::RegistrySaveStatus::Saving => {
                    label.set_text(&fl!(LANGUAGE_LOADER, "live-button-working"));
                    retry.set_sensitive(false);
                    row.set_visible(true);
                }
                crate::ui::persist::RegistrySaveStatus::Failed(error) => {
                    label.set_text(&fl!(
                        LANGUAGE_LOADER,
                        "settings-failed-to-save",
                        err = error
                    ));
                    retry.set_sensitive(true);
                    row.set_visible(true);
                }
            }
            glib::ControlFlow::Continue
        }
    };
    refresh();
    glib::timeout_add_local(std::time::Duration::from_millis(100), refresh);
    row
}

/// Literal, height-for-width labels also wrap unbroken OBS names.
fn inventory_label(text: &str) -> Label {
    Label::builder()
        .label(text)
        .use_markup(false)
        .xalign(0.0)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .width_chars(1)
        .hexpand(true)
        .build()
}

fn build_scene_row(
    scene_id: &str,
    ordered: &[String],
    nav: &NavigationContext,
    container: &GtkBox,
    compact: &adw::Breakpoint,
) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::builder()
        .activatable(false)
        .focusable(true)
        .build();
    row.set_widget_name(&format!("scene-order-{scene_id}"));
    row.add_css_class("inventory-scene-row");
    let layout = GtkBox::new(Orientation::Horizontal, 12);
    layout.set_widget_name(&format!("inventory-row-layout-{scene_id}"));
    let identity = GtkBox::new(Orientation::Horizontal, 8);
    identity.set_hexpand(true);
    let drag = install_scene_reordering(&row, scene_id, nav, container);
    identity.append(&drag);
    let picker = icon_picker::build(
        nav.state.borrow().registry.scene_icon(scene_id),
        &fl!(LANGUAGE_LOADER, "inventory-scene-icon-tooltip"),
        icon_picker::PickerDisplay::CurrentIcon,
        {
            let nav = nav.clone();
            let scene_id = scene_id.to_string();
            move |icon| set_scene_icon(&nav, &scene_id, icon.as_deref())
        },
    );
    picker.set_valign(Align::Center);
    picker.set_widget_name(&format!("inventory-icon-{scene_id}"));
    identity.append(&picker);
    let text = GtkBox::new(Orientation::Vertical, 4);
    text.set_hexpand(true);
    let name = inventory_label(scene_id);
    name.set_widget_name(&format!("inventory-name-{scene_id}"));
    name.set_tooltip_text(Some(scene_id));
    name.add_css_class("heading");
    let role = nav
        .state
        .borrow()
        .registry
        .scenes
        .get(scene_id)
        .and_then(|entry| entry.role);
    let description = inventory_label(
        &role
            .map(SceneRole::description)
            .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "inventory-no-role-assigned")),
    );
    description.set_widget_name(&format!("inventory-description-{scene_id}"));
    description.add_css_class("dim-label");
    text.append(&name);
    text.append(&description);
    identity.append(&text);
    layout.append(&identity);

    let controls = GtkBox::new(Orientation::Horizontal, 8);
    controls.set_valign(Align::Center);
    let role = build_role_cell(scene_id, nav);
    role.set_hexpand(true);
    role.connect_selected_notify({
        let description = description.downgrade();
        move |dropdown| {
            if let Some(description) = description.upgrade() {
                description.set_text(
                    &role_at(dropdown.selected())
                        .map(SceneRole::description)
                        .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "inventory-no-role-assigned")),
                );
            }
        }
    });
    controls.append(&role);
    controls.append(&build_scene_options(scene_id, ordered, nav, container));
    layout.append(&controls);
    compact.add_setter(
        &layout,
        "orientation",
        Some(&Orientation::Vertical.to_value()),
    );
    row.set_child(Some(&layout));
    row
}

fn build_role_cell(scene_id: &str, nav: &NavigationContext) -> DropDown {
    let role = nav
        .state
        .borrow()
        .registry
        .scenes
        .get(scene_id)
        .and_then(|entry| entry.role);
    let mut labels = vec![SceneRole::unassigned_label()];
    labels.extend(SceneRole::ALL.iter().map(|role| role.label()));
    let dropdown = DropDown::builder()
        .model(&string_list(&labels))
        .selected(role_row(role))
        .valign(Align::Center)
        .build();
    dropdown.add_css_class("scenedeck-dropdown");
    dropdown.add_css_class("inventory-role-selector");
    dropdown.set_widget_name(&format!("inventory-role-{scene_id}"));
    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
        let label = Label::builder()
            .xalign(0.0)
            .wrap(true)
            .width_chars(1)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .build();
        item.set_child(Some(&label));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
        let label = item.child().and_downcast::<Label>().unwrap();
        let value = item.item().and_downcast::<gtk4::StringObject>().unwrap();
        label.set_label(value.string().as_str());
    });
    dropdown.set_factory(Some(&factory));
    dropdown.set_list_factory(Some(&factory));
    let description = role
        .map(SceneRole::description)
        .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "inventory-no-role-assigned"));
    dropdown.set_tooltip_text(Some(&description));
    let accessible_name = format!(
        "{} — {scene_id}",
        fl!(LANGUAGE_LOADER, "inventory-role-column")
    );
    dropdown.update_property(&[gtk4::accessible::Property::Label(&accessible_name)]);
    dropdown.connect_selected_notify({
        let scene_id = scene_id.to_string();
        let nav = nav.clone();
        move |row| handle_scene_role_change(row, &scene_id, &nav)
    });
    dropdown
}

fn build_scene_options(
    scene_id: &str,
    ordered: &[String],
    nav: &NavigationContext,
    container: &GtkBox,
) -> MenuButton {
    let label = fl!(LANGUAGE_LOADER, "inventory-scene-options", scene = scene_id);
    let button = MenuButton::builder()
        .icon_name("view-more-symbolic")
        .tooltip_text(&label)
        .build();
    button.update_property(&[gtk4::accessible::Property::Label(&label)]);
    button.set_widget_name(&format!("inventory-options-{scene_id}"));
    button.add_css_class("flat");
    let content = GtkBox::new(Orientation::Vertical, 8);
    content.add_css_class("inventory-row-options");
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    let appearance = GtkBox::new(Orientation::Horizontal, 8);
    let registry = nav.state.borrow().registry.clone();
    let (accent, clear) = build_accent_controls(scene_id, &registry, nav);
    appearance.append(&accent);
    appearance.append(&clear);
    content.append(&appearance);
    let position = ordered.iter().position(|id| id == scene_id).unwrap();
    for down in [false, true] {
        let target = if down {
            position.checked_add(1)
        } else {
            position.checked_sub(1)
        }
        .and_then(|index| ordered.get(index))
        .cloned();
        let label = if down {
            fl!(LANGUAGE_LOADER, "inventory-move-down", scene = scene_id)
        } else {
            fl!(LANGUAGE_LOADER, "inventory-move-up", scene = scene_id)
        };
        let move_label = inventory_label(&label);
        move_label.set_max_width_chars(20);
        let move_button = Button::builder()
            .child(&move_label)
            .sensitive(target.is_some())
            .build();
        move_button.set_widget_name(&format!(
            "inventory-move-{}-{scene_id}",
            if down { "down" } else { "up" }
        ));
        move_button.connect_clicked({
            let nav = nav.clone();
            let container = container.downgrade();
            let source = scene_id.to_string();
            move |_| {
                let Some(target) = target.as_ref() else {
                    return;
                };
                if !reorder_scenes(&nav, &source, target, down) {
                    return;
                }
                let nav = nav.clone();
                let container = container.clone();
                let name = format!("scene-order-{source}");
                glib::idle_add_local_once(move || {
                    if let Some(container) = container.upgrade() {
                        rebuild(&container, &nav);
                        focus_named_row(container.upcast_ref(), &name);
                    }
                });
            }
        });
        content.append(&move_button);
    }
    let popover = Popover::new();
    popover.set_child(Some(&content));
    button.set_popover(Some(&popover));
    button
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
        .tooltip_text(fl!(LANGUAGE_LOADER, "inventory-accent-clear-tooltip"))
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
        let accent_box = accent_box.downgrade();
        move |clear_accent_button| {
            let Some(accent_box) = accent_box.upgrade() else {
                return;
            };
            if set_scene_accent(&nav, &scene_id, None) {
                if let Some(previous_picker) = accent_box.first_child() {
                    accent_box.remove(&previous_picker);
                }
                let unset_picker = build_accent_button(&scene_id, None, &nav, clear_accent_button);
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
    row: &impl IsA<gtk4::Widget>,
    scene_id: &str,
    nav: &NavigationContext,
    container: &GtkBox,
) -> Image {
    let drag_handle = Image::from_icon_name("list-drag-handle-symbolic");
    drag_handle.set_tooltip_text(Some(&fl!(LANGUAGE_LOADER, "inventory-drag-tooltip")));
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
        let nav = nav.clone();
        let container = container.downgrade();
        move |target, value, _, y| {
            let Ok(source_scene_id) = value.get::<String>() else {
                return false;
            };
            let insert_after = target
                .widget()
                .is_some_and(|widget| y >= f64::from(widget.height()) / 2.0);
            if !reorder_scenes(&nav, &source_scene_id, &target_scene_id, insert_after) {
                return false;
            }
            glib::idle_add_local_once({
                let nav = nav.clone();
                let container = container.clone();
                move || {
                    if let Some(container) = container.upgrade() {
                        rebuild(&container, &nav);
                    }
                }
            });
            true
        }
    });
    row.add_controller(drop_target);

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
    container: &GtkBox,
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
        .name("inventory-stale")
        .title(fl!(LANGUAGE_LOADER, "inventory-stale-group-title"))
        .description(fl!(LANGUAGE_LOADER, "inventory-stale-group-description"))
        .build();

    for (entry_name, entry) in stale {
        let stale_row = adw::ActionRow::builder()
            .use_markup(false)
            .subtitle(SceneRole::label_or_unassigned(entry.role))
            .build();
        // Apply untrusted text only after literal rendering is established.
        stale_row.set_title(&entry_name);

        let remove_btn = Button::builder()
            .icon_name("list-remove-symbolic")
            .tooltip_text(fl!(LANGUAGE_LOADER, "inventory-remove-stale-tooltip"))
            .valign(Align::Center)
            .build();
        remove_btn.add_css_class("flat");
        remove_btn.add_css_class("destructive-action");

        remove_btn.connect_clicked({
            let entry_name = entry_name.clone();
            let container = container.downgrade();
            let nav = nav.clone();
            move |_| {
                if let Some(container) = container.upgrade() {
                    handle_stale_entry_remove(entry_name.as_str(), &container, &nav);
                }
            }
        });

        stale_row.add_suffix(&remove_btn);
        stale_group.add(&stale_row);
    }

    Some(stale_group)
}

fn focus_named_row(widget: &gtk4::Widget, name: &str) -> bool {
    if widget.widget_name() == name {
        return widget.grab_focus();
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        if focus_named_row(&current, name) {
            return true;
        }
        child = current.next_sibling();
    }
    false
}

fn reorder_scenes(
    nav: &NavigationContext,
    source_scene_id: &str,
    target_scene_id: &str,
    insert_after: bool,
) -> bool {
    if source_scene_id == target_scene_id {
        return false;
    }

    let inventory_scene_ids: Vec<_> = nav
        .state
        .borrow()
        .scene_inventory
        .scenes
        .iter()
        .map(|scene| scene.id.clone())
        .collect();
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
) -> ColorDialogButton {
    let dialog = ColorDialog::builder()
        .title(fl!(LANGUAGE_LOADER, "inventory-accent-dialog-title"))
        .with_alpha(false)
        .build();
    let button = ColorDialogButton::new(Some(dialog));
    button.set_tooltip_text(Some(&fl!(
        LANGUAGE_LOADER,
        "inventory-accent-choose-tooltip"
    )));
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
        let clear_accent_button = clear_accent_button.downgrade();
        move |button| {
            let rgba = button.rgba();
            let accent = scene_accent_hex(rgba.red(), rgba.green(), rgba.blue());
            if set_scene_accent(&nav, &scene_id, Some(accent)) {
                if let Some(clear_accent_button) = clear_accent_button.upgrade() {
                    clear_accent_button.set_sensitive(true);
                }
            }
        }
    });

    button
}

fn build_yaml_actions_row(
    container: &GtkBox,
    nav: &NavigationContext,
    compact: &adw::Breakpoint,
) -> GtkBox {
    let card = GtkBox::new(Orientation::Vertical, 0);
    card.add_css_class("inventory-yaml-card");
    let row = ActionRow::builder()
        .use_markup(false)
        .title(fl!(LANGUAGE_LOADER, "inventory-yaml-row-title"))
        .subtitle(fl!(LANGUAGE_LOADER, "inventory-yaml-row-subtitle"))
        .title_lines(0)
        .subtitle_lines(0)
        .build();

    row.set_widget_name("inventory-yaml-actions");
    let export_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "inventory-export-button-label"))
        .icon_name("document-save-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "inventory-export-tooltip"))
        .valign(Align::Center)
        .hexpand(true)
        .build();
    export_btn.add_css_class("flat");
    export_btn.set_widget_name("inventory-export");

    let import_btn = Button::builder()
        .label(fl!(LANGUAGE_LOADER, "inventory-import-button-label"))
        .icon_name("document-open-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "inventory-import-tooltip"))
        .valign(Align::Center)
        .hexpand(true)
        .build();
    import_btn.add_css_class("flat");
    import_btn.set_widget_name("inventory-import");

    export_btn.connect_clicked({
        let row = row.downgrade();
        let nav = nav.clone();
        move |button| {
            if let Some(row) = row.upgrade() {
                handle_export_click(button, &row, &nav);
            }
        }
    });

    import_btn.connect_clicked({
        let row = row.downgrade();
        let container = container.downgrade();
        let nav = nav.clone();
        move |button| {
            if let (Some(row), Some(container)) = (row.upgrade(), container.upgrade()) {
                handle_import_click(button, &row, &container, &nav);
            }
        }
    });

    // Keep file actions below their copy even at large text sizes. Their native
    // buttons remain visible instead of competing with the title's minimum.
    let status = gtk4::ListBox::new();
    status.set_selection_mode(gtk4::SelectionMode::None);
    status.append(&row);
    card.append(&status);
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_homogeneous(true);
    actions.set_margin_start(12);
    actions.set_margin_end(12);
    actions.set_margin_bottom(8);
    actions.append(&export_btn);
    actions.append(&import_btn);
    compact.add_setter(
        &actions,
        "orientation",
        Some(&Orientation::Vertical.to_value()),
    );
    card.append(&actions);
    card
}

fn handle_scene_role_change(row: &DropDown, scene_id: &str, nav: &NavigationContext) {
    let new_role = role_at(row.selected());
    if nav
        .state
        .borrow()
        .registry
        .scenes
        .get(scene_id)
        .and_then(|entry| entry.role)
        != new_role
    {
        persist_registry(nav, "scene role", |registry| {
            registry.set_scene_role(scene_id, new_role)
        });
    }
    row.set_tooltip_text(Some(
        &new_role
            .map(SceneRole::description)
            .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "inventory-no-role-assigned")),
    ));
}

fn handle_stale_entry_remove(entry_name: &str, container: &GtkBox, nav: &NavigationContext) {
    persist_registry(nav, "stale entry removal", |registry| {
        registry.scenes.remove(entry_name).is_some()
    });

    // Rebuild rather than hiding the row in place: removing the last stale
    // entry must also drop the now-empty group, which `build_stale_group`
    // already handles by returning `None`.
    rebuild(container, nav);
}

/// Persist one scene's icon and mirror it into the cached registry.
///
/// Failed writes retain dirty intent; the page status and Close/Quit decision
/// offer retry without rolling the picker or newer accepted edits backward.
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

fn yaml_dialog(title: &str, accept_label: &str) -> FileDialog {
    let filter = yaml_file_filter();
    let filters = gio::ListStore::new::<FileFilter>();
    filters.append(&filter);
    FileDialog::builder()
        .title(title)
        .accept_label(accept_label)
        .modal(true)
        .filters(&filters)
        .default_filter(&filter)
        .build()
}

fn dialog_cancelled(error: &glib::Error) -> bool {
    error.matches(gtk4::DialogError::Dismissed)
        || error.matches(gtk4::DialogError::Cancelled)
        || error.matches(gio::IOErrorEnum::Cancelled)
}

fn show_export_dialog(button: &Button, status_row: &ActionRow, nav: &NavigationContext) {
    let dialog = yaml_dialog(
        &fl!(LANGUAGE_LOADER, "inventory-export-dialog-title"),
        &fl!(LANGUAGE_LOADER, "inventory-export-button-label"),
    );
    dialog.set_initial_name(Some("scenedeck-registry.yaml"));
    let status_row = status_row.clone();
    let registry = nav.state.borrow().registry.clone();
    dialog.save(
        parent_window(button).as_ref(),
        gio::Cancellable::NONE,
        move |result| {
            let file = match result {
                Ok(file) => file,
                Err(error) if dialog_cancelled(&error) => return,
                Err(error) => {
                    status_row.set_subtitle(&fl!(
                        LANGUAGE_LOADER,
                        "inventory-export-error",
                        error = error.to_string()
                    ));
                    return;
                }
            };
            let Some(path) = file.path() else {
                status_row.set_subtitle(&fl!(LANGUAGE_LOADER, "inventory-export-no-file"));
                return;
            };
            // FileDialog confirmed this exact target, not a sibling with a YAML suffix.
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
        },
    );
}

fn show_import_dialog(
    button: &Button,
    status_row: &ActionRow,
    container: &GtkBox,
    nav: &NavigationContext,
) {
    let dialog = yaml_dialog(
        &fl!(LANGUAGE_LOADER, "inventory-import-dialog-title"),
        &fl!(LANGUAGE_LOADER, "inventory-import-button-label"),
    );
    let status_row = status_row.clone();
    let container = container.clone();
    let nav = nav.clone();
    dialog.open(
        parent_window(button).as_ref(),
        gio::Cancellable::NONE,
        move |result| {
            let file = match result {
                Ok(file) => file,
                Err(error) if dialog_cancelled(&error) => return,
                Err(error) => {
                    status_row.set_subtitle(&fl!(
                        LANGUAGE_LOADER,
                        "inventory-import-error",
                        error = error.to_string()
                    ));
                    return;
                }
            };
            let Some(path) = file.path() else {
                status_row.set_subtitle(&fl!(LANGUAGE_LOADER, "inventory-import-no-file"));
                return;
            };
            let completion_container = container.clone();
            let completion_nav = nav.clone();
            if crate::ui::persist::import_registry(&nav, path, move |result| {
                completion_container.set_sensitive(true);
                match result {
                    Ok(()) => rebuild(&completion_container, &completion_nav),
                    Err(error) => status_row.set_subtitle(&fl!(
                        LANGUAGE_LOADER,
                        "inventory-import-error",
                        error = error.to_string()
                    )),
                }
            }) {
                container.set_sensitive(false);
            }
        },
    );
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

#[cfg(test)]
mod tests {
    use super::{reordered_scene_ids, role_at, role_row};
    use crate::domain::role::SceneRole;

    #[test]
    #[ignore = "requires a disposable GTK display; no OBS or keyring access"]
    fn native_inventory_rebuild_releases_yaml_rows_and_root() {
        crate::ui::test_support::native_test(
            "ui::pages::inventory::tests::native_inventory_rebuild_releases_yaml_rows_and_root",
            || {
                use crate::ui::test_support::{navigation, pump};
                use adw::prelude::*;
                use std::time::Duration;
                let (nav, stack, _runtime, _events) = navigation();
                for id in ["Intro", "Main"] {
                    nav.state
                        .borrow_mut()
                        .scene_inventory
                        .scenes
                        .push(crate::domain::scene::Scene { id: id.into() });
                }
                nav.state
                    .borrow_mut()
                    .registry
                    .set_scene_role("Missing <b>scene</b>", Some(SceneRole::Primary));
                let (page, refresh) = super::build(nav.clone());
                stack.add_named(&page, Some("inventory"));
                let window = gtk4::Window::builder()
                    .default_width(1000)
                    .default_height(700)
                    .child(&stack)
                    .build();
                window.present();
                pump(Duration::from_millis(80));
                fn descendants(
                    widget: &gtk4::Widget,
                    tracked: &mut Vec<glib::WeakRef<gtk4::Widget>>,
                ) {
                    if widget.widget_name() == "inventory-yaml-actions"
                        || widget.has_css_class("inventory-list")
                        || widget.widget_name().starts_with("scene-order-")
                    {
                        tracked.push(widget.downgrade());
                    }
                    let mut child = widget.first_child();
                    while let Some(widget) = child {
                        child = widget.next_sibling();
                        descendants(&widget, tracked);
                    }
                }
                for _ in 0..3 {
                    let mut old = Vec::new();
                    descendants(&page, &mut old);
                    assert!(old.len() >= 4, "real mapped YAML/list/scene rows required");
                    gtk4::prelude::GtkWindowExt::set_focus(&window, gtk4::Widget::NONE);
                    // Only a real scope retirement replaces the full page now.
                    nav.state.borrow_mut().workspace.epoch += 1;
                    refresh();
                    pump(Duration::from_millis(80));
                    assert!(
                        old.iter().all(|widget| widget.upgrade().is_none()),
                        "detached Inventory descendants retained"
                    );
                }
                let weak = page.downgrade();
                window.set_child(gtk4::Widget::NONE);
                window.destroy();
                stack.remove(&page);
                drop(page);
                refresh();
                pump(Duration::from_millis(80));
                assert!(
                    weak.upgrade().is_none(),
                    "Inventory root retained by row or callback"
                );
            },
        );
    }

    #[test]
    fn every_role_round_trips_through_dropdown_including_unassigned() {
        assert_eq!(role_at(role_row(None)), None);
        for role in SceneRole::ALL {
            assert_eq!(role_at(role_row(Some(role))), Some(role));
        }
        assert_eq!(role_at(u32::MAX), None);
    }

    #[test]
    #[ignore = "requires a disposable GTK display; no OBS or keyring access"]
    fn native_unchanged_inventory_refresh_keeps_rows_and_open_popovers() {
        crate::ui::test_support::native_test(
            "ui::pages::inventory::tests::native_unchanged_inventory_refresh_keeps_rows_and_open_popovers",
            || {
                use adw::prelude::*;
                use crate::domain::scene::Scene;
                use crate::ui::test_support::{navigation, pump};
                use std::time::Duration;
                let (nav, stack, _runtime, _events) = navigation();
                nav.state.borrow_mut().scene_inventory.scenes = vec![Scene { id: "Main".into() }];
                let (page, refresh) = super::build(nav.clone());
                let row = find(&page, "scene-order-Main").unwrap();
                stack.add_named(&page, Some("inventory"));
                let window = gtk4::Window::builder().default_width(640).default_height(700).child(&stack).build();
                window.present();
                pump(Duration::from_millis(120));
                assert_eq!(find(&page, "scene-order-Main").unwrap(), row,
                    "mapping an already current page must not replace its scene rows");
                let options = find(&page, "inventory-options-Main").unwrap().downcast::<gtk4::MenuButton>().unwrap();
                options.popup();
                pump(Duration::from_millis(60));
                let popup = options.popover().unwrap();
                assert!(popup.is_mapped());
                nav.state.borrow_mut().scene_inventory.current_id = Some("Main".into());
                for _ in 0..3 {
                    refresh();
                    assert_eq!(find(&page, "scene-order-Main").unwrap(), row);
                    assert!(popup.is_mapped(), "unchanged refresh closed the scene menu");
                    assert!(row.is_mapped(), "refresh unmapped a visible scene row");
                }
                window.destroy();
                pump(Duration::from_millis(120));
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display and isolated registry storage"]
    fn native_inventory_changes_reconcile_without_replacing_the_page() {
        crate::ui::test_support::native_test("ui::pages::inventory::tests::native_inventory_changes_reconcile_without_replacing_the_page", || {
            use adw::prelude::*;
            use crate::domain::scene::Scene;
            use crate::ui::test_support::{navigation, pump};
            use std::time::Duration;
            let (nav, stack, _runtime, _events) = navigation();
            nav.state.borrow_mut().scene_inventory.scenes = vec![Scene { id: "Main".into() }, Scene { id: "Other".into() }];
            let (page, refresh) = super::build(nav.clone());
            stack.add_named(&page, Some("inventory"));
            let window = gtk4::Window::builder().default_width(640).default_height(700).child(&stack).build();
            window.present();
            pump(Duration::from_millis(100));
            let main = find(&page, "scene-order-Main").unwrap();
            let other = find(&page, "scene-order-Other").unwrap();
            let yaml = find(&page, "inventory-yaml-actions").unwrap();
            let selector = find(&page, "inventory-role-Main").unwrap().downcast::<gtk4::DropDown>().unwrap();
            nav.state.borrow_mut().registry.set_scene_role("Main", Some(SceneRole::Primary));
            refresh();
            assert_eq!(selector.selected(), super::role_row(Some(SceneRole::Primary)));
            assert_eq!(find(&page, "scene-order-Main").unwrap(), main);
            assert_eq!(find(&page, "scene-order-Other").unwrap(), other);
            nav.state.borrow_mut().scene_inventory.scenes.push(Scene { id: "New".into() });
            refresh();
            assert!(super::reorder_scenes(&nav, "New", "Main", false), "retained handlers must use current membership");
            super::rebuild(page.downcast_ref::<gtk4::Box>().unwrap(), &nav);
            assert_eq!(find(&page, "scene-order-New").unwrap().downcast::<gtk4::ListBoxRow>().unwrap().index(), 0);
            assert_eq!(find(&page, "scene-order-Main").unwrap(), main);
            nav.state.borrow_mut().registry.set_scene_icon("Other", Some("nf-md-camera"));
            refresh();
            assert_ne!(find(&page, "scene-order-Other").unwrap(), other);
            assert_eq!(find(&page, "scene-order-Main").unwrap(), main);
            nav.state.borrow_mut().registry.set_scene_role("Missing", Some(SceneRole::Archive));
            refresh();
            assert!(find(&page, "inventory-stale").is_some());
            nav.state.borrow_mut().registry.scenes.remove("Missing");
            refresh();
            assert!(find(&page, "inventory-stale").is_none());
            assert_eq!(find(&page, "inventory-yaml-actions").unwrap(), yaml);
            nav.state.borrow_mut().workspace.epoch += 1;
            refresh();
            assert_ne!(find(&page, "scene-order-Main").unwrap(), main, "scope retirement must rebind controls");
            window.destroy();
            pump(Duration::from_millis(120));
        });
    }

    #[test]
    fn inventory_uses_native_rows_without_width_configuration() {
        let source = include_str!("inventory.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for removed in [
            "ColumnView",
            "build_column_controls",
            "set_fixed_width",
            "config.inventory",
        ] {
            assert!(
                !production.contains(removed),
                "Inventory must reflow native list rows, not {removed}"
            );
        }
        assert!(
            production.contains("ListBox"),
            "Inventory needs native list rows"
        );
    }

    fn find(root: &gtk4::Widget, name: &str) -> Option<gtk4::Widget> {
        use adw::prelude::*;
        if root.widget_name() == name {
            return Some(root.clone());
        }
        let mut child = root.first_child();
        while let Some(widget) = child {
            if let Some(found) = find(&widget, name) {
                return Some(found);
            }
            child = widget.next_sibling();
        }
        None
    }

    fn assert_native_list(widget: &gtk4::Widget) {
        use adw::prelude::*;
        assert!(
            !widget.is::<gtk4::ColumnView>(),
            "Inventory must not be a table"
        );
        assert_ne!(widget.widget_name(), "inventory-column-widths");
        if let Some(row) = widget.downcast_ref::<adw::PreferencesRow>() {
            assert!(
                !row.uses_markup(),
                "YAML and stale scene text must remain literal"
            );
        }
        if let Some(scroll) = widget.downcast_ref::<gtk4::ScrolledWindow>().filter(|_| {
            widget.widget_name() == "inventory-scroll"
                || widget.widget_name() == "inventory-table-scroll"
        }) {
            assert_eq!(scroll.hscrollbar_policy(), gtk4::PolicyType::Never);
            let adjustment = scroll.hadjustment();
            assert!(
                adjustment.upper() <= adjustment.page_size() + 1.0,
                "Inventory has hidden horizontal overflow: {} > {}",
                adjustment.upper(),
                adjustment.page_size()
            );
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            assert_native_list(&widget);
            child = widget.next_sibling();
        }
    }

    fn capture(window: &gtk4::Window, name: &str) {
        use adw::prelude::*;
        let Some(directory) = std::env::var_os("SCENEDECK_INVENTORY_CAPTURE_DIR") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let snapshot = gtk4::Snapshot::new();
        gtk4::WidgetPaintable::new(Some(window)).snapshot(
            &snapshot,
            f64::from(window.width()),
            f64::from(window.height()),
        );
        let node = snapshot.to_node().expect("actual Inventory render node");
        let bounds =
            gtk4::graphene::Rect::new(0.0, 0.0, window.width() as f32, window.height() as f32);
        window
            .renderer()
            .unwrap()
            .render_texture(&node, Some(&bounds))
            .save_to_png(directory.join(format!("{name}.png")))
            .unwrap();
    }

    #[test]
    #[ignore = "requires a disposable GTK display; no OBS or keyring access"]
    fn native_inventory_rows_reflow_without_horizontal_scrolling() {
        crate::ui::test_support::native_test(
            "ui::pages::inventory::tests::native_inventory_rows_reflow_without_horizontal_scrolling",
            || {
                use adw::prelude::*;
                use crate::domain::{appearance::Language, scene::Scene};
                use crate::ui::test_support::{navigation, pump};
                use std::time::Duration;
                // Real long, malformed markup and unbroken names must remain literal.
                let scene = "<b>Studio & Hauptübertragung</b> — SehrLangerSzenennameOhneTrennzeichenFürDieDeutscheÜbertragung";
                for (language, locale, dpi) in [(Language::En, "en", 98_304), (Language::De, "de-200", 196_608)] {
                    crate::infra::i18n::init(language);
                    let settings = gtk4::Settings::default().unwrap();
                    settings.set_property("gtk-font-name", "Sans 10");
                    settings.set_property("gtk-xft-dpi", dpi);
                    for width in [360, 640, 1100] {
                        let (nav, stack, _runtime, _events) = navigation();
                        nav.state.borrow_mut().scene_inventory.scenes = vec![Scene { id: scene.into() }, Scene { id: "Main".into() }];
                        nav.state.borrow_mut().registry.set_scene_role(scene, Some(SceneRole::Primary));
                        nav.state.borrow_mut().registry.set_scene_role("Missing <b>scene & literal", Some(SceneRole::Primary));
                        // Compatibility-only persisted settings must have no effect on layout.
                        nav.state.borrow_mut().config = serde_json::from_str(r#"{"inventory":{"name_width":1000,"role_width":600}}"#).unwrap();
                        let saved = serde_json::to_value(&nav.state.borrow().config).unwrap();
                        let (page, refresh) = super::build(nav.clone());
                        stack.add_named(&page, Some("inventory"));
                        let window = gtk4::Window::builder().default_width(width).default_height(900).child(&stack).build();
                        window.add_css_class("scenedeck-root");
                        let provider = gtk4::CssProvider::new();
                        provider.load_from_data(include_str!("../../../assets/scenedeck.css"));
                        gtk4::style_context_add_provider_for_display(&gtk4::prelude::WidgetExt::display(&window), &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
                        window.present();
                        pump(Duration::from_millis(200));
                        assert!(window.width() <= width, "window minimum escaped requested {width}: {}", window.width());
                        assert_native_list(&page);
                        for name in [format!("scene-order-{scene}"), format!("inventory-name-{scene}"), format!("inventory-description-{scene}"), format!("inventory-role-{scene}"), format!("inventory-icon-{scene}"), format!("inventory-options-{scene}"), "inventory-yaml-actions".into(), "inventory-export".into(), "inventory-import".into()] {
                            let widget = find(&page, &name).unwrap_or_else(|| panic!("missing {name}"));
                            let bounds = widget.compute_bounds(&page).unwrap();
                            assert!(bounds.width() > 0.0 && bounds.height() > 0.0, "unallocated {name}");
                            assert!(bounds.x() >= 0.0 && bounds.x() + bounds.width() <= page.width() as f32 + 1.0,
                                "{locale}/{width}: {name} escaped viewport: {bounds:?}");
                        }
                        let layout = find(&page, &format!("inventory-row-layout-{scene}")).unwrap().downcast::<gtk4::Box>().unwrap();
                        assert_eq!(layout.orientation(), if width == 1100 && locale == "en" { gtk4::Orientation::Horizontal } else { gtk4::Orientation::Vertical });
                        if width == 1100 {
                            let selector = find(&page, &format!("inventory-role-{scene}")).unwrap();
                            assert!(selector.grab_focus());
                            pump(Duration::from_millis(80));
                            let focused = gtk4::prelude::RootExt::focus(&window).unwrap();
                            assert!(focused == selector || focused.is_ancestor(&selector), "role focus must be established before resizing");
                            for resized_width in [360, 1100] {
                                window.set_default_size(resized_width, 900);
                                pump(Duration::from_millis(150));
                                assert_eq!(window.width(), resized_width);
                                assert_native_list(&page);
                                assert_eq!(find(&page, &format!("inventory-role-{scene}")).unwrap(), selector,
                                    "resizing must retain the actual editing widget");
                                let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
                                assert!(focus == selector || focus.is_ancestor(&selector), "{locale}/{resized_width}: resizing lost role focus to {} ({:?}); selector bounds {:?}", focus.widget_name(), focus.type_(), selector.compute_bounds(&page));
                                assert_eq!(layout.orientation(), if resized_width == 1100 && locale == "en" {
                                    gtk4::Orientation::Horizontal
                                } else {
                                    gtk4::Orientation::Vertical
                                });
                            }
                        }
                        let title = find(&page, &format!("inventory-name-{scene}")).unwrap().downcast::<gtk4::Label>().unwrap();
                        assert_eq!(title.text(), scene);
                        assert!(!title.uses_markup());
                        for name in ["inventory-export".to_string(), format!("inventory-role-{scene}"), format!("inventory-options-{scene}")] {
                            let widget = find(&page, &name).unwrap();
                            assert!(widget.grab_focus(), "{name} not keyboard reachable");
                            refresh();
                            pump(Duration::from_millis(100));
                            let target = find(&page, &name).unwrap();
                            let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
                            assert!(focus == target || focus.is_ancestor(&target), "refresh lost {name}");
                        }
                        assert_eq!(serde_json::to_value(&nav.state.borrow().config).unwrap(), saved);
                        // Capture the top of the list rather than the focused bottom row.
                        gtk4::prelude::GtkWindowExt::set_focus(&window, gtk4::Widget::NONE);
                        find(&page, "inventory-scroll").unwrap().downcast::<gtk4::ScrolledWindow>().unwrap().vadjustment().set_value(0.0);
                        pump(Duration::from_millis(80));
                        capture(&window, &format!("inventory-{locale}-{width}"));
                        window.set_child(gtk4::Widget::NONE);
                        window.destroy();
                        gtk4::style_context_remove_provider_for_display(&gtk4::prelude::WidgetExt::display(&window), &provider);
                        drop(refresh);
                        stack.remove(&page);
                        drop(page);
                        pump(Duration::from_millis(120));
                    }
                }
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable GTK display and isolated XDG paths"]
    fn native_inventory_keyboard_reorder_preserves_focus_and_admits_edits() {
        crate::ui::test_support::native_test(
            "ui::pages::inventory::tests::native_inventory_keyboard_reorder_preserves_focus_and_admits_edits",
            || {
                use adw::prelude::*;
                use crate::domain::scene::Scene;
                use crate::ui::test_support::{navigation, pump};
                use std::time::Duration;
                let (nav, stack, _runtime, _events) = navigation();
                nav.state.borrow_mut().scene_inventory.scenes = vec![Scene { id: "Intro".into() }, Scene { id: "Main".into() }];
                let (page, _refresh) = super::build(nav.clone());
                stack.add_named(&page, Some("inventory"));
                let window = gtk4::Window::builder().default_width(360).default_height(700).child(&stack).build();
                window.present();
                pump(Duration::from_millis(100));
                let role = find(&page, "inventory-role-Main").unwrap().downcast::<gtk4::DropDown>().unwrap();
                role.set_selected(super::role_row(Some(SceneRole::Primary)));
                assert_eq!(nav.state.borrow().registry.scenes["Main"].role, Some(SceneRole::Primary));
                let description = find(&page, "inventory-description-Main").unwrap().downcast::<gtk4::Label>().unwrap();
                assert_eq!(description.text().as_str(), SceneRole::Primary.description());
                let options = find(&page, "inventory-options-Main").unwrap().downcast::<gtk4::MenuButton>().unwrap();
                options.popup();
                pump(Duration::from_millis(80));
                let up = find(options.popover().unwrap().upcast_ref(), "inventory-move-up-Main").unwrap().downcast::<gtk4::Button>().unwrap();
                assert!(up.grab_focus());
                up.emit_clicked();
                // Acceptance is synchronous, before any idle rebuild or disk publication.
                assert_eq!(nav.state.borrow().registry.ordered_scene_ids(["Intro", "Main"]), ["Main", "Intro"]);
                drop(up);
                drop(options);
                drop(role);
                drop(description);
                pump(Duration::from_millis(150));
                let target = find(&page, "scene-order-Main").unwrap();
                let focus = gtk4::prelude::RootExt::focus(&window).unwrap();
                assert!(focus == target || focus.is_ancestor(&target));
                window.destroy();
                pump(Duration::from_millis(120));
            },
        );
    }

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
    fn adjacent_keyboard_moves_use_the_same_order_as_drag() {
        assert_eq!(
            reordered_scene_ids(order(), "Two", "One", false).unwrap(),
            ["Two", "One", "Three", "Four"]
        );
        assert_eq!(
            reordered_scene_ids(order(), "Three", "Four", true).unwrap(),
            ["One", "Two", "Four", "Three"]
        );
        assert!(reordered_scene_ids(order(), "Missing", "One", false).is_none());
    }

    #[test]
    fn dropping_scene_on_itself_does_not_reorder() {
        assert!(reordered_scene_ids(order(), "Two", "Two", false).is_none());
    }
}
