//! Graph page — scene dependency visualisation.
//!
//! Renders the OBS scene dependency graph as a scrollable canvas.  Each scene
//! is drawn as a node and each nested-scene dependency is drawn as a directed
//! edge.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::f64::consts::PI;
use std::rc::Rc;

use adw::{prelude::*, StatusPage};
use gtk4::cairo::{Context, FontSlant, FontWeight};
use gtk4::{
    Align, Box as GtkBox, Button, DrawingArea, GestureDrag, Image, Label, Orientation, PolicyType,
    ScrolledWindow,
};
use i18n_embed_fl::fl;

use crate::domain::graph::{EdgeStatus, SceneGraph};
use crate::domain::registry::SceneRegistrySnapshot;
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::services::graph_service::classify_edge;
use crate::ui::navigation::NavigationContext;

const NODE_WIDTH: f64 = 190.0;
const NODE_HEIGHT: f64 = 74.0;
const NODE_RADIUS: f64 = 8.0;
const LAYER_GAP: f64 = 270.0;
const ROW_GAP: f64 = 124.0;
const CANVAS_PADDING: f64 = 88.0;
const GRID_STEP: f64 = 48.0;

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let read_nav = nav.clone();
    crate::ui::stateful_page(
        "graph-page",
        true,
        move || {
            let state = read_nav.state.borrow();
            (
                state.workspace.epoch,
                state.config.language,
                state
                    .scene_inventory
                    .scenes
                    .iter()
                    .map(|s| s.id.clone())
                    .collect::<Vec<_>>(),
                state.scene_graph.edges.clone(),
                state.registry.snapshot(),
                state.topology_status.clone(),
            )
        },
        move |container| {
            populate(container, &nav);
        },
    )
}

fn populate(container: &GtkBox, nav: &NavigationContext) {
    let (graph, inventory, topology_status) = {
        let state = nav.state.borrow();
        (
            state.scene_graph.clone(),
            state.scene_inventory.clone(),
            state.topology_status.clone(),
        )
    };

    if let Some((title, description)) = topology_status.notice() {
        let unavailable = StatusPage::builder()
            .icon_name("view-grid-symbolic")
            .title(&title)
            .description(glib::markup_escape_text(&description))
            .build();
        container.append(&unavailable);
        return;
    }

    if graph.edges.is_empty() && inventory.scenes.is_empty() {
        let empty = StatusPage::builder()
            .icon_name("view-grid-symbolic")
            .title(fl!(LANGUAGE_LOADER, "topology-empty-title"))
            .description(fl!(LANGUAGE_LOADER, "topology-empty-description"))
            .build();
        container.append(&empty);
        return;
    }

    let registry = nav.state.borrow().registry.snapshot();
    let model = GraphModel::build(&graph, &inventory, &registry);
    let edge_summary = model.edge_summary;
    let model = Rc::new(RefCell::new(model));

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .vexpand(true)
        .hexpand(true)
        .build();

    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .hexpand(true)
        .build();

    let title = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "graph-page-title"))
        .xalign(0.0)
        .hexpand(true)
        .valign(Align::Center)
        .build();
    title.add_css_class("heading");
    header.append(&title);
    header.append(&build_edge_summary(edge_summary));

    let reset_btn = Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(fl!(LANGUAGE_LOADER, "graph-reset-tooltip"))
        .valign(Align::Center)
        .build();
    header.append(&reset_btn);
    page.append(&header);
    let relationships = gtk4::Expander::builder()
        .label(fl!(LANGUAGE_LOADER, "graph-relationship-list"))
        .expanded(false)
        .build();
    let list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();
    list.add_css_class("boxed-list");
    let mut scenes = BTreeSet::new();
    scenes.extend(inventory.scenes.iter().map(|scene| scene.id.clone()));
    for (parent, children) in &graph.edges {
        scenes.insert(parent.clone());
        scenes.extend(children.iter().cloned());
    }
    for parent in scenes {
        let children = graph.children(&parent);
        let row = adw::ExpanderRow::builder().use_markup(false).build();
        row.set_title(&parent);
        if children.is_empty() {
            row.set_subtitle(&fl!(LANGUAGE_LOADER, "graph-no-dependencies"));
            row.set_enable_expansion(false);
        }
        for child in children {
            let status = classify_edge(
                registry.scene_role(&parent),
                registry.scene_role(child),
                registry.graph_policy(),
            );
            let edge = adw::ActionRow::builder()
                .use_markup(false)
                .focusable(true)
                .build();
            edge.set_title(child);
            edge.set_subtitle(&status.summary_label());
            let icon = Image::from_icon_name(status.icon_name());
            icon.add_css_class(status.css_class());
            edge.add_prefix(&icon);
            row.add_row(&edge);
        }
        list.append(&row);
    }
    let list_scroll = ScrolledWindow::builder()
        .max_content_height(260)
        .propagate_natural_height(true)
        .hscrollbar_policy(PolicyType::Never)
        .child(&list)
        .build();
    relationships.set_child(Some(&list_scroll));
    page.append(&relationships);

    let canvas = DrawingArea::builder()
        .content_width(model.borrow().content_width as i32)
        .content_height(model.borrow().content_height as i32)
        .vexpand(true)
        .hexpand(true)
        .build();
    canvas.add_css_class("graph-canvas");
    canvas.set_focusable(true);
    canvas.set_draw_func({
        let model = model.clone();
        move |_, ctx, width, height| {
            draw_canvas(ctx, width, height, &model.borrow());
        }
    });

    reset_btn.connect_clicked({
        let model = model.clone();
        let canvas = canvas.downgrade();
        let graph = graph.clone();
        let inventory = inventory.clone();
        let registry = registry.clone();
        move |_| {
            let Some(canvas) = canvas.upgrade() else {
                return;
            };
            let updated = GraphModel::build(&graph, &inventory, &registry);
            canvas.set_content_width(updated.content_width as i32);
            canvas.set_content_height(updated.content_height as i32);
            model.replace(updated);
            canvas.queue_draw();
        }
    });

    install_drag_controller(&canvas, &model);

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&canvas)
        .build();
    page.append(&scroll);
    container.append(&page);
}

fn install_drag_controller(canvas: &DrawingArea, model: &Rc<RefCell<GraphModel>>) {
    let drag = GestureDrag::new();
    drag.set_button(0);

    let target = Rc::new(RefCell::new(None));
    let start_node_x = Rc::new(Cell::new(0.0));
    let start_node_y = Rc::new(Cell::new(0.0));

    drag.connect_drag_begin({
        let model = model.clone();
        let target = target.clone();
        let start_node_x = start_node_x.clone();
        let start_node_y = start_node_y.clone();
        move |_, x, y| {
            let hit = model.borrow().hit_test(x, y);
            target.replace(hit);

            if let Some(node_idx) = hit {
                let node = &model.borrow().nodes[node_idx];
                start_node_x.set(node.x);
                start_node_y.set(node.y);
            }
        }
    });

    drag.connect_drag_update({
        let model = model.clone();
        let canvas = canvas.downgrade();
        let target = target.clone();
        let start_node_x = start_node_x.clone();
        let start_node_y = start_node_y.clone();
        move |_, offset_x, offset_y| {
            let Some(canvas) = canvas.upgrade() else {
                return;
            };
            if let Some(node_idx) = *target.borrow() {
                let mut graph_model = model.borrow_mut();
                let max_x = (graph_model.content_width - NODE_WIDTH).max(0.0);
                let max_y = (graph_model.content_height - NODE_HEIGHT).max(0.0);

                if let Some(node) = graph_model.nodes.get_mut(node_idx) {
                    node.x = (start_node_x.get() + offset_x).clamp(0.0, max_x);
                    node.y = (start_node_y.get() + offset_y).clamp(0.0, max_y);
                }
                canvas.queue_draw();
            }
        }
    });

    canvas.add_controller(drag);
}

#[derive(Debug, Clone)]
struct GraphModel {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    edge_summary: EdgeSummary,
    content_width: f64,
    content_height: f64,
}

impl GraphModel {
    fn build(
        graph: &SceneGraph,
        inventory: &SceneInventory,
        registry: &SceneRegistrySnapshot,
    ) -> Self {
        let mut node_ids: BTreeSet<String> = inventory
            .scenes
            .iter()
            .map(|scene| scene.id.clone())
            .collect();
        for (parent, children) in &graph.edges {
            node_ids.insert(parent.clone());
            for child in children {
                node_ids.insert(child.clone());
            }
        }

        let order = scene_order(inventory);
        let mut ids: Vec<String> = node_ids.into_iter().collect();
        ids.sort_by(|a, b| {
            order
                .get(a)
                .unwrap_or(&usize::MAX)
                .cmp(order.get(b).unwrap_or(&usize::MAX))
                .then_with(|| a.cmp(b))
        });

        let index_by_id: HashMap<String, usize> = ids
            .iter()
            .enumerate()
            .map(|(idx, id)| (id.clone(), idx))
            .collect();

        let mut edge_pairs = BTreeSet::new();
        for (parent, children) in &graph.edges {
            for child in children {
                if let (Some(&from), Some(&to)) = (index_by_id.get(parent), index_by_id.get(child))
                {
                    edge_pairs.insert((from, to));
                }
            }
        }

        let layers = compute_layers(ids.len(), &edge_pairs);
        let mut layer_rows: HashMap<usize, usize> = HashMap::new();
        let mut max_layer = 0;
        let mut max_row = 0;

        let mut nodes = Vec::with_capacity(ids.len());
        for (idx, id) in ids.into_iter().enumerate() {
            let layer = layers[idx];
            let row = layer_rows.entry(layer).or_insert(0);
            max_layer = max_layer.max(layer);
            max_row = max_row.max(*row);
            let role = registry.scene_role(&id);

            nodes.push(GraphNode {
                label: id,
                role,
                x: CANVAS_PADDING + layer as f64 * LAYER_GAP,
                y: CANVAS_PADDING + *row as f64 * ROW_GAP,
            });

            *row += 1;
        }

        let mut edge_summary = EdgeSummary::default();
        let edges = edge_pairs
            .into_iter()
            .map(|(from, to)| {
                let status =
                    classify_edge(nodes[from].role, nodes[to].role, registry.graph_policy());
                edge_summary.record(status);
                GraphEdge { from, to, status }
            })
            .collect();

        Self {
            nodes,
            edges,
            edge_summary,
            content_width: CANVAS_PADDING * 2.0 + NODE_WIDTH + max_layer as f64 * LAYER_GAP,
            content_height: CANVAS_PADDING * 2.0 + NODE_HEIGHT + max_row as f64 * ROW_GAP,
        }
    }

    fn hit_test(&self, x: f64, y: f64) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, node)| node.contains(x, y))
            .map(|(idx, _)| idx)
    }
}

#[derive(Debug, Clone)]
struct GraphNode {
    label: String,
    role: Option<SceneRole>,
    x: f64,
    y: f64,
}

impl GraphNode {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + NODE_WIDTH && y >= self.y && y <= self.y + NODE_HEIGHT
    }

    fn center(&self) -> (f64, f64) {
        (self.x + NODE_WIDTH / 2.0, self.y + NODE_HEIGHT / 2.0)
    }
}

#[derive(Debug, Clone)]
struct GraphEdge {
    from: usize,
    to: usize,
    status: EdgeStatus,
}

#[derive(Debug, Default, Clone, Copy)]
struct EdgeSummary {
    ok: usize,
    warning: usize,
    forbidden: usize,
}

impl EdgeSummary {
    fn record(&mut self, status: EdgeStatus) {
        match status {
            EdgeStatus::Ok => self.ok += 1,
            EdgeStatus::Warning => self.warning += 1,
            EdgeStatus::Forbidden => self.forbidden += 1,
        }
    }

    fn count(self, status: EdgeStatus) -> usize {
        match status {
            EdgeStatus::Ok => self.ok,
            EdgeStatus::Warning => self.warning,
            EdgeStatus::Forbidden => self.forbidden,
        }
    }
}

fn build_edge_summary(summary: EdgeSummary) -> GtkBox {
    let root = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .valign(Align::Center)
        .build();
    root.add_css_class("graph-status-summary");

    for status in EdgeStatus::SUMMARY_ORDER {
        root.append(&build_edge_summary_label(status, summary.count(status)));
    }

    root
}

fn build_edge_summary_label(status: EdgeStatus, count: usize) -> GtkBox {
    let root = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .valign(Align::Center)
        .build();
    root.set_tooltip_text(Some(&status.summary_tooltip()));

    let icon = Image::from_icon_name(status.icon_name());
    icon.add_css_class(status.css_class());
    root.append(&icon);

    let summary = Label::builder()
        .label(fl!(
            LANGUAGE_LOADER,
            "graph-edge-summary-count",
            label = status.summary_label(),
            count = count
        ))
        .valign(Align::Center)
        .build();
    summary.add_css_class("caption");
    summary.add_css_class(status.css_class());
    root.append(&summary);

    root
}

fn scene_order(inventory: &SceneInventory) -> HashMap<String, usize> {
    inventory
        .scenes
        .iter()
        .enumerate()
        .map(|(idx, scene)| (scene.id.clone(), idx))
        .collect()
}

fn compute_layers(node_count: usize, edges: &BTreeSet<(usize, usize)>) -> Vec<usize> {
    let mut incoming = vec![Vec::new(); node_count];
    for &(from, to) in edges {
        incoming[to].push(from);
    }

    // Iterative longest-path-over-parents with an explicit stack. The
    // recursive version this replaced walked one frame per edge along the
    // deepest dependency chain, which a pathologically nested OBS scene graph
    // could overflow; page layout has no business putting a recursion depth
    // limit on user data.
    let mut memo: Vec<Option<usize>> = vec![None; node_count];
    let mut on_stack = vec![false; node_count];

    for start in 0..node_count {
        if memo[start].is_some() {
            continue;
        }
        let mut stack = vec![start];
        while let Some(&idx) = stack.last() {
            if memo[idx].is_some() {
                stack.pop();
                continue;
            }
            on_stack[idx] = true;

            let mut unresolved = None;
            let mut depth = 0;
            for &parent in &incoming[idx] {
                if let Some(parent_depth) = memo[parent] {
                    depth = depth.max(parent_depth + 1);
                } else if on_stack[parent] {
                    // Cycle: the in-flight ancestor contributes depth zero,
                    // exactly as the recursive guard did.
                    depth = depth.max(1);
                } else {
                    unresolved = Some(parent);
                }
            }

            match unresolved {
                // Resolve one parent deeper; `idx` is revisited once the
                // stack above it unwinds.
                Some(parent) => stack.push(parent),
                None => {
                    memo[idx] = Some(depth);
                    on_stack[idx] = false;
                    stack.pop();
                }
            }
        }
    }

    memo.into_iter().map(|depth| depth.unwrap_or(0)).collect()
}

fn draw_canvas(ctx: &Context, width: i32, height: i32, model: &GraphModel) {
    let palette = Palette::for_current_theme();
    let width = width as f64;
    let height = height as f64;

    set_rgb(ctx, palette.background);
    ctx.rectangle(0.0, 0.0, width, height);
    let _ = ctx.fill();

    draw_grid(ctx, width, height, palette.grid);

    for edge in &model.edges {
        draw_edge(
            ctx,
            &model.nodes[edge.from],
            &model.nodes[edge.to],
            edge.status,
        );
    }

    for node in &model.nodes {
        draw_node(ctx, node, &palette);
    }
}

fn draw_grid(ctx: &Context, width: f64, height: f64, color: Color) {
    set_rgb(ctx, color);
    ctx.set_line_width(1.0);

    let left = 0.0;
    let top = 0.0;
    let right = width;
    let bottom = height;

    let mut x = (left / GRID_STEP).floor() * GRID_STEP;
    while x <= right {
        ctx.move_to(x, top);
        ctx.line_to(x, bottom);
        x += GRID_STEP;
    }

    let mut y = (top / GRID_STEP).floor() * GRID_STEP;
    while y <= bottom {
        ctx.move_to(left, y);
        ctx.line_to(right, y);
        y += GRID_STEP;
    }

    let _ = ctx.stroke();
}

fn draw_edge(ctx: &Context, from: &GraphNode, to: &GraphNode, status: EdgeStatus) {
    let (start, end) = edge_points(from, to);
    let color = match status {
        EdgeStatus::Ok => Color(0.20, 0.52, 0.89),
        EdgeStatus::Warning => Color(0.88, 0.58, 0.05),
        EdgeStatus::Forbidden => Color(0.75, 0.11, 0.16),
    };

    set_rgb(ctx, color);
    ctx.set_line_width(2.6);
    ctx.move_to(start.0, start.1);

    let mid_x = (start.0 + end.0) / 2.0;
    ctx.curve_to(mid_x, start.1, mid_x, end.1, end.0, end.1);
    let _ = ctx.stroke();

    let angle = (end.1 - start.1).atan2(end.0 - start.0);
    draw_arrow(ctx, end, angle, color);
}

fn draw_arrow(ctx: &Context, tip: (f64, f64), angle: f64, color: Color) {
    let size = 10.0;
    let left = (
        tip.0 - size * (angle - PI / 6.0).cos(),
        tip.1 - size * (angle - PI / 6.0).sin(),
    );
    let right = (
        tip.0 - size * (angle + PI / 6.0).cos(),
        tip.1 - size * (angle + PI / 6.0).sin(),
    );

    set_rgb(ctx, color);
    ctx.move_to(tip.0, tip.1);
    ctx.line_to(left.0, left.1);
    ctx.line_to(right.0, right.1);
    ctx.close_path();
    let _ = ctx.fill();
}

fn edge_points(from: &GraphNode, to: &GraphNode) -> ((f64, f64), (f64, f64)) {
    let (from_x, from_y) = from.center();
    let (to_x, to_y) = to.center();
    let dx = to_x - from_x;
    let dy = to_y - from_y;

    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return ((from_x, from_y), (to_x, to_y));
    }

    let start_scale = rect_edge_scale(dx, dy);
    let end_scale = rect_edge_scale(-dx, -dy);

    (
        (from_x + dx * start_scale, from_y + dy * start_scale),
        (to_x - dx * end_scale, to_y - dy * end_scale),
    )
}

fn rect_edge_scale(dx: f64, dy: f64) -> f64 {
    let x_scale = if dx.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        NODE_WIDTH / 2.0 / dx.abs()
    };
    let y_scale = if dy.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        NODE_HEIGHT / 2.0 / dy.abs()
    };
    x_scale.min(y_scale)
}

fn draw_node(ctx: &Context, node: &GraphNode, palette: &Palette) {
    rounded_rect(ctx, node.x, node.y, NODE_WIDTH, NODE_HEIGHT, NODE_RADIUS);
    set_rgb(ctx, palette.node_fill);
    let _ = ctx.fill_preserve();
    set_rgb(ctx, palette.node_border);
    ctx.set_line_width(1.2);
    let _ = ctx.stroke();

    let accent = role_color(node.role);
    rounded_rect(ctx, node.x, node.y, 7.0, NODE_HEIGHT, NODE_RADIUS);
    set_rgb(ctx, accent);
    let _ = ctx.fill();

    ctx.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    ctx.set_font_size(14.0);
    set_rgb(ctx, palette.text);
    let label = fitted_text(ctx, &node.label, NODE_WIDTH - 34.0);
    ctx.move_to(node.x + 20.0, node.y + 29.0);
    let _ = ctx.show_text(&label);

    ctx.select_font_face("Sans", FontSlant::Normal, FontWeight::Normal);
    ctx.set_font_size(12.0);
    set_rgb(ctx, palette.muted_text);
    let role = SceneRole::label_or_unassigned(node.role);
    ctx.move_to(node.x + 20.0, node.y + 52.0);
    let _ = ctx.show_text(&role);
}

fn rounded_rect(ctx: &Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let right = x + width;
    let bottom = y + height;

    ctx.new_sub_path();
    ctx.arc(right - radius, y + radius, radius, -PI / 2.0, 0.0);
    ctx.arc(right - radius, bottom - radius, radius, 0.0, PI / 2.0);
    ctx.arc(x + radius, bottom - radius, radius, PI / 2.0, PI);
    ctx.arc(x + radius, y + radius, radius, PI, PI * 1.5);
    ctx.close_path();
}

fn fitted_text(ctx: &Context, text: &str, max_width: f64) -> String {
    if text_width(ctx, text) <= max_width {
        return text.to_string();
    }

    // Truncate with the same ellipsis the rest of the UI's Fluent strings
    // use, rather than three ASCII dots.
    let mut trimmed = text.to_string();
    while !trimmed.is_empty() {
        trimmed.pop();
        let candidate = format!("{trimmed}…");
        if text_width(ctx, &candidate) <= max_width {
            return candidate;
        }
    }

    "…".to_string()
}

fn text_width(ctx: &Context, text: &str) -> f64 {
    ctx.text_extents(text)
        .map(|extents| extents.width())
        .unwrap_or(0.0)
}

fn role_color(role: Option<SceneRole>) -> Color {
    match role {
        Some(SceneRole::Primary) => Color(0.20, 0.52, 0.89),
        Some(SceneRole::Secondary) => Color(0.13, 0.55, 0.42),
        Some(SceneRole::Module) => Color(0.52, 0.37, 0.76),
        Some(SceneRole::Raw) => Color(0.88, 0.58, 0.05),
        Some(SceneRole::Debug) => Color(0.75, 0.11, 0.16),
        Some(SceneRole::Archive) => Color(0.44, 0.50, 0.56),
        None => Color(0.44, 0.50, 0.56),
    }
}

fn set_rgb(ctx: &Context, color: Color) {
    ctx.set_source_rgb(color.0, color.1, color.2);
}

#[derive(Clone, Copy)]
struct Color(f64, f64, f64);

struct Palette {
    background: Color,
    grid: Color,
    node_fill: Color,
    node_border: Color,
    text: Color,
    muted_text: Color,
}

impl Palette {
    fn for_current_theme() -> Self {
        if adw::StyleManager::default().is_dark() {
            Self {
                background: Color(0.10, 0.11, 0.13),
                grid: Color(0.17, 0.18, 0.21),
                node_fill: Color(0.15, 0.16, 0.19),
                node_border: Color(0.33, 0.36, 0.40),
                text: Color(0.93, 0.94, 0.96),
                muted_text: Color(0.66, 0.69, 0.74),
            }
        } else {
            Self {
                background: Color(0.96, 0.97, 0.98),
                grid: Color(0.86, 0.88, 0.90),
                node_fill: Color(1.0, 1.0, 1.0),
                node_border: Color(0.76, 0.79, 0.83),
                text: Color(0.12, 0.14, 0.18),
                muted_text: Color(0.42, 0.46, 0.52),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::state::ObsStatus;
    use crate::domain::registry::SceneMetadata;
    use crate::domain::scene::Scene;

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_graph_distinguishes_missing_empty_failed_and_reset_topology() {
        crate::ui::test_support::native_test(
            "ui::pages::graph::tests::native_graph_distinguishes_missing_empty_failed_and_reset_topology",
            || {
                let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
                let container = GtkBox::new(Orientation::Vertical, 0);
                nav.state.borrow_mut().scene_inventory = inventory(["Main"]);
                populate(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Scene topology not observed"));
                assert!(!page_text(container.upcast_ref()).contains("No nested scene dependencies"));
                nav.state.borrow_mut().scene_inventory = inventory([]);
                nav.state.borrow_mut().set_topology_ready(SceneGraph::default());
                crate::ui::clear_children(&container);
                populate(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("No scenes observed"));
                {
                    let mut state = nav.state.borrow_mut();
                    state.scene_inventory = inventory(["Main"]);
                    state.set_topology_ready(SceneGraph::default());
                }
                crate::ui::clear_children(&container);
                populate(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Scene Dependencies"));
                nav.state.borrow_mut().set_topology_failed("Group <b>& read failed".into());
                crate::ui::clear_children(&container);
                populate(&container, &nav);
                let text = page_text(container.upcast_ref());
                assert!(text.contains("Scene topology could not be read"));
                assert!(text.contains("Group <b>& read failed"));
                assert!(!text.contains("No nested scene dependencies"));
                nav.state.borrow_mut().set_topology_loading();
                crate::ui::clear_children(&container);
                populate(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Reading scene topology"));
                nav.state.borrow_mut().set_topology_ready(SceneGraph::default());
                crate::ui::clear_children(&container);
                populate(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Scene Dependencies"));
                nav.state.borrow_mut().invalidate_collection_runtime();
                crate::ui::clear_children(&container);
                populate(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Scene topology not observed"));
                nav.state.borrow_mut().reset_obs_session(ObsStatus::Disconnected);
                crate::ui::clear_children(&container);
                populate(&container, &nav);
                assert!(page_text(container.upcast_ref()).contains("Scene topology not observed"));
            },
        );
    }

    fn page_text(widget: &gtk4::Widget) -> String {
        let mut text = widget
            .downcast_ref::<Label>()
            .map(|label| label.text().to_string())
            .unwrap_or_default();
        let mut child = widget.first_child();
        while let Some(widget) = child {
            text.push_str(&page_text(&widget));
            child = widget.next_sibling();
        }
        text
    }

    #[test]
    #[ignore = "requires a native GTK display"]
    fn native_graph_gesture_releases_its_canvas_and_model() {
        crate::ui::test_support::native_test(
            "ui::pages::graph::tests::native_graph_gesture_releases_its_canvas_and_model",
            || {
                let model = Rc::new(RefCell::new(GraphModel::build(
                    &SceneGraph::default(),
                    &inventory(["Main"]),
                    &registry([]),
                )));
                let weak_model = Rc::downgrade(&model);
                let canvas = DrawingArea::new();
                canvas.set_draw_func({
                    let model = model.clone();
                    move |_, ctx, width, height| draw_canvas(ctx, width, height, &model.borrow())
                });
                install_drag_controller(&canvas, &model);
                let weak_canvas = canvas.downgrade();
                drop(model);
                drop(canvas);
                assert!(weak_canvas.upgrade().is_none());
                assert!(weak_model.upgrade().is_none());
            },
        );
    }

    #[test]
    fn graph_model_counts_edge_statuses_from_registry_policy() {
        let graph = SceneGraph {
            edges: HashMap::from([(
                "Main".to_string(),
                vec!["Module".to_string(), "Raw".to_string()],
            )]),
        };
        let inventory = inventory(["Main", "Module", "Raw"]);
        let registry = registry([
            ("Main", SceneRole::Primary),
            ("Module", SceneRole::Module),
            ("Raw", SceneRole::Raw),
        ]);

        let model = GraphModel::build(&graph, &inventory, &registry);

        assert_eq!(model.edge_summary.count(EdgeStatus::Ok), 1);
        assert_eq!(model.edge_summary.count(EdgeStatus::Warning), 1);
        assert_eq!(model.edge_summary.count(EdgeStatus::Forbidden), 0);
    }

    #[test]
    fn graph_model_keeps_isolated_scenes_with_no_edges() {
        let model = GraphModel::build(
            &SceneGraph::default(),
            &inventory(["Solo", "Other"]),
            &registry([]),
        );
        assert_eq!(
            model
                .nodes
                .iter()
                .map(|node| node.label.as_str())
                .collect::<Vec<_>>(),
            ["Solo", "Other"]
        );
        assert!(model.edges.is_empty());
        assert!(model.content_height.is_finite());
    }

    #[test]
    fn compute_layers_handles_cycles_with_finite_depths() {
        let layers = compute_layers(2, &BTreeSet::from([(0, 1), (1, 0)]));

        assert_eq!(layers.len(), 2);
        assert!(layers.into_iter().all(|layer| layer <= 2));
    }

    fn inventory<const N: usize>(ids: [&str; N]) -> SceneInventory {
        SceneInventory {
            scenes: ids
                .into_iter()
                .map(|id| Scene { id: id.to_string() })
                .collect(),
            current_id: None,
            previous_id: None,
        }
    }

    fn registry<const N: usize>(roles: [(&str, SceneRole); N]) -> SceneRegistrySnapshot {
        SceneRegistrySnapshot::new(
            roles
                .into_iter()
                .map(|(scene_id, role)| {
                    (
                        scene_id.to_string(),
                        SceneMetadata {
                            role,
                            tags: Vec::new(),
                            protected: false,
                        },
                    )
                })
                .collect(),
            Default::default(),
        )
    }
}
