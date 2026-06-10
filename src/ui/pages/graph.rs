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
    Align, Box as GtkBox, Button, DrawingArea, GestureDrag, Label, Orientation, PolicyType,
    ScrolledWindow,
};

use crate::domain::graph::{EdgeStatus, SceneGraph};
use crate::domain::role::SceneRole;
use crate::domain::scene::SceneInventory;
use crate::services::graph_service::classify_edge;
use crate::storage::registry::read_registry;
use crate::ui::navigation::NavigationContext;

const NODE_WIDTH: f64 = 190.0;
const NODE_HEIGHT: f64 = 74.0;
const NODE_RADIUS: f64 = 8.0;
const LAYER_GAP: f64 = 270.0;
const ROW_GAP: f64 = 124.0;
const CANVAS_PADDING: f64 = 88.0;
const GRID_STEP: f64 = 48.0;

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
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

    let model = Rc::new(RefCell::new(GraphModel::build(&graph, &inventory)));

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
        .label("Scene Dependencies")
        .xalign(0.0)
        .hexpand(true)
        .valign(Align::Center)
        .build();
    title.add_css_class("heading");
    header.append(&title);

    let reset_btn = Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Reset graph layout")
        .valign(Align::Center)
        .build();
    header.append(&reset_btn);
    page.append(&header);

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
        let canvas = canvas.clone();
        let graph = graph.clone();
        let inventory = inventory.clone();
        move |_| {
            let updated = GraphModel::build(&graph, &inventory);
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
        let canvas = canvas.clone();
        let target = target.clone();
        let start_node_x = start_node_x.clone();
        let start_node_y = start_node_y.clone();
        move |_, offset_x, offset_y| {
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
    content_width: f64,
    content_height: f64,
}

impl GraphModel {
    fn build(graph: &SceneGraph, inventory: &SceneInventory) -> Self {
        let registry = read_registry();
        let role_of =
            |scene_id: &str| -> Option<SceneRole> { registry.scenes.get(scene_id).map(|e| e.role) };

        let mut node_ids = BTreeSet::new();
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
            let role = role_of(&id);

            nodes.push(GraphNode {
                label: id,
                role,
                x: CANVAS_PADDING + layer as f64 * LAYER_GAP,
                y: CANVAS_PADDING + *row as f64 * ROW_GAP,
            });

            *row += 1;
        }

        let graph_policy = registry.rules.policy();
        let edges = edge_pairs
            .into_iter()
            .map(|(from, to)| GraphEdge {
                from,
                to,
                status: classify_edge(nodes[from].role, nodes[to].role, &graph_policy),
            })
            .collect();

        Self {
            nodes,
            edges,
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

    let mut memo = vec![None; node_count];
    let mut visiting = vec![false; node_count];
    (0..node_count)
        .map(|idx| node_depth(idx, &incoming, &mut memo, &mut visiting))
        .collect()
}

fn node_depth(
    idx: usize,
    incoming: &[Vec<usize>],
    memo: &mut [Option<usize>],
    visiting: &mut [bool],
) -> usize {
    if let Some(depth) = memo[idx] {
        return depth;
    }
    if visiting[idx] {
        return 0;
    }

    visiting[idx] = true;
    let depth = incoming[idx]
        .iter()
        .map(|&parent| node_depth(parent, incoming, memo, visiting) + 1)
        .max()
        .unwrap_or(0);
    visiting[idx] = false;
    memo[idx] = Some(depth);
    depth
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
    let role = node.role.map(SceneRole::label).unwrap_or("Unassigned");
    ctx.move_to(node.x + 20.0, node.y + 52.0);
    let _ = ctx.show_text(role);
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

    let mut trimmed = text.to_string();
    while !trimmed.is_empty() {
        trimmed.pop();
        let candidate = format!("{trimmed}...");
        if text_width(ctx, &candidate) <= max_width {
            return candidate;
        }
    }

    "...".to_string()
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
