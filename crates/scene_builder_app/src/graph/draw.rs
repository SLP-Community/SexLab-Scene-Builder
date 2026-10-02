use super::{GraphSnapshot, GRID_STEP, NODE_H, NODE_W};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use scene_builder_core::project::define::Node;
use scene_builder_core::project::scene::Scene;
use scene_builder_core::project::NanoID;
use std::collections::HashMap;

pub(super) fn snapshot_of(scene: &Scene) -> GraphSnapshot {
    GraphSnapshot {
        stages: scene.stages.clone(),
        nodes: scene
            .graph
            .iter()
            .map(|(id, node)| (id.clone(), node.clone()))
            .collect(),
        root: scene.root.clone(),
    }
}

pub(super) fn apply_snapshot(scene: &mut Scene, snap: &GraphSnapshot) {
    scene.stages = snap.stages.clone();
    scene.graph.clear();
    for (id, node) in &snap.nodes {
        scene.graph.insert(id.clone(), node.clone());
    }
    scene.root = snap.root.clone();
}

pub(super) fn content_bounds_from(positions: &HashMap<NanoID, Pos2>) -> Option<Rect> {
    if positions.is_empty() {
        return None;
    }
    let mut min = Pos2::new(f32::MAX, f32::MAX);
    let mut max = Pos2::new(f32::MIN, f32::MIN);
    for p in positions.values() {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x + NODE_W);
        max.y = max.y.max(p.y + NODE_H);
    }
    Some(Rect::from_min_max(min, max))
}

pub(super) fn truncate_to_width(
    painter: &egui::Painter,
    text: &str,
    font: &egui::FontId,
    max_w: f32,
) -> String {
    let full_w = painter
        .layout_no_wrap(text.to_string(), font.clone(), Color32::BLACK)
        .size()
        .x;
    if full_w <= max_w {
        return text.to_string();
    }
    let mut out: String = text.to_string();
    while !out.is_empty() {
        out.pop();
        let candidate = format!("{out}…");
        let w = painter
            .layout_no_wrap(candidate.clone(), font.clone(), Color32::BLACK)
            .size()
            .x;
        if w <= max_w {
            return candidate;
        }
    }
    "…".to_string()
}

pub(super) fn dist_to_polyline(p: Pos2, path: &[Pos2]) -> f32 {
    let mut best = f32::MAX;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ab = b - a;
        let len2 = ab.length_sq();
        let t = if len2 > 0.0 {
            ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let proj = a + ab * t;
        best = best.min((p - proj).length());
    }
    best
}

pub(super) fn rounded_polyline(points: &[Pos2], radius: f32) -> Vec<Pos2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut out = Vec::with_capacity(points.len() * 4);
    out.push(points[0]);
    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let p = points[i];
        let next = points[i + 1];
        let in_len = (p - prev).length();
        let out_len = (next - p).length();
        let r = radius.min(in_len * 0.5).min(out_len * 0.5);
        if r < 0.5 {
            out.push(p);
            continue;
        }
        let dir_in = (p - prev) / in_len;
        let dir_out = (next - p) / out_len;
        let a = p - dir_in * r;
        let b = p + dir_out * r;
        const STEPS: usize = 6;
        for s in 0..=STEPS {
            let t = s as f32 / STEPS as f32;
            let q1 = a + (p - a) * t;
            let q2 = p + (b - p) * t;
            out.push(q1 + (q2 - q1) * t);
        }
    }
    out.push(*points.last().unwrap());
    out
}

pub(super) fn draw_edge_path(painter: &egui::Painter, path: &[Pos2], stroke: Stroke, zoom: f32) {
    if path.len() < 2 {
        return;
    }
    painter.add(egui::Shape::line(path.to_vec(), stroke));

    let tip = *path.last().unwrap();
    let prev = path[path.len() - 2];
    let dir = tip - prev;
    if dir.length_sq() > 0.0 {
        let dir = dir.normalized();
        let side = Vec2::new(-dir.y, dir.x);
        let len = 8.0 * zoom.clamp(0.6, 1.5);
        let half_h = 3.0 * zoom.clamp(0.6, 1.5);
        let p1 = tip - dir * len + side * half_h;
        let p2 = tip - dir * len - side * half_h;
        painter.add(egui::Shape::convex_polygon(
            vec![tip, p1, p2],
            stroke.color,
            Stroke::NONE,
        ));
    }
}
pub(super) fn draw_mesh_grid(
    painter: &egui::Painter,
    rect: Rect,
    pan: Vec2,
    zoom: f32,
    dark: bool,
) {
    let mut step_world = GRID_STEP;
    while step_world * zoom < 20.0 {
        step_world *= 2.0;
        if step_world > GRID_STEP * 64.0 {
            break;
        }
    }
    let step = step_world * zoom;

    let dot = if dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 12)
    } else {
        Color32::from_rgba_unmultiplied(33, 35, 48, 14)
    };
    let major_line = if dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 10)
    } else {
        Color32::from_rgba_unmultiplied(33, 35, 48, 10)
    };

    let origin = rect.center() + pan * zoom;
    let start_x = ((rect.left() - origin.x) / step).floor() as i32 - 1;
    let end_x = ((rect.right() - origin.x) / step).ceil() as i32 + 1;
    let start_y = ((rect.top() - origin.y) / step).floor() as i32 - 1;
    let end_y = ((rect.bottom() - origin.y) / step).ceil() as i32 + 1;

    for i in start_x..=end_x {
        let x = origin.x + i as f32 * step;
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, major_line),
        );
    }
    for j in start_y..=end_y {
        let y = origin.y + j as f32 * step;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(1.0, major_line),
        );
        for i in start_x..=end_x {
            let x = origin.x + i as f32 * step;
            painter.circle_filled(Pos2::new(x, y), 1.0, dot);
        }
    }
}

/// Softer than shell/extreme bg — easier on the eyes for long graph sessions.
pub(super) fn graph_bg(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x1a, 0x1a, 0x1a)
    } else {
        Color32::from_rgb(0xeb, 0xeb, 0xed)
    }
}

pub(super) fn layout_nodes(scene: &Scene) -> HashMap<NanoID, Pos2> {
    let mut map = HashMap::new();
    for (i, stage) in scene.stages.iter().enumerate() {
        if let Some(node) = scene.graph.get(&stage.id) {
            map.insert(stage.id.clone(), Pos2::new(node.x, node.y));
            continue;
        }
        let col = (i % 4) as f32;
        let row = (i / 4) as f32;
        map.insert(
            stage.id.clone(),
            Pos2::new(40.0 + col * (NODE_W + 60.0), 40.0 + row * (NODE_H + 80.0)),
        );
    }
    map
}

pub fn ensure_graph_node(scene: &mut Scene, stage_id: &NanoID, index: usize) {
    scene.graph.entry(stage_id.clone()).or_insert_with(|| {
        let col = (index % 4) as f32;
        let row = (index / 4) as f32;
        Node {
            dest: Vec::new(),
            x: 40.0 + col * (NODE_W + 40.0),
            y: 40.0 + row * (NODE_H + 40.0),
        }
    });
}
