//! Graph canvas with manual pan/zoom in screen space (crisp text at any zoom).
//! Custom edge engine: rounded orthogonal routing, hit-testing, arrowheads.
//! Nodes allocate screen-space interact rects so future in-card controls can
//! be added without a transform-layer rewrite.

mod canvas;
mod draw;
mod node;
mod toolbar;

pub use draw::ensure_graph_node;

use egui::{Color32, Pos2, Rect, Vec2};
use scene_builder_core::project::define::Node;
use scene_builder_core::project::scene::Scene;
use scene_builder_core::project::stage::Stage;
use scene_builder_core::project::NanoID;

use draw::{apply_snapshot, content_bounds_from, layout_nodes, rounded_polyline, snapshot_of};

pub const NODE_W: f32 = 300.0;
pub const NODE_H: f32 = 140.0;
pub(super) const HEADER_H: f32 = 44.0;
pub(super) const ROOT_BORDER: Color32 = Color32::from_rgb(0, 88, 0);
pub(super) const FIXED_LEN_PINK: Color32 = Color32::from_rgb(255, 175, 175);
pub(super) const FIXED_LEN_CYAN: Color32 = Color32::from_rgb(175, 235, 255);
pub(super) const ICON_START: Color32 = Color32::from_rgb(17, 175, 17);
pub(super) const ICON_ORGASM: Color32 = Color32::from_rgb(255, 20, 147);
pub(super) const ICON_WARN: Color32 = Color32::from_rgb(255, 0, 0);
pub(super) const ICON_FIXED: Color32 = Color32::from_rgb(0, 191, 255);
pub(super) const BTN_DANGER: Color32 = Color32::from_rgb(0xcf, 0x13, 0x22);
// rgba(201, 225, 195, 0.3) premultiplied (from_rgba_unmultiplied is not const).
pub(super) const PORT_FILL: Color32 = Color32::from_rgba_premultiplied(61, 68, 59, 77);
/// Soft major grid spacing in world units (no dense minors).
pub(super) const GRID_STEP: f32 = 90.0;
pub(super) const DRAG_THRESHOLD: f32 = 4.0;
pub(super) const EDGE_HIT_DIST: f32 = 6.0;
pub(super) const ZOOM_MIN: f32 = 0.25;
pub(super) const ZOOM_MAX: f32 = 5.0;
/// Status / hover control size in world pixels (scales with zoom).
pub(super) const ICON_HIT: f32 = 22.0;
/// Stage name size in world pixels.
pub(super) const NAME_PX: f32 = 18.0;
pub(super) const ROOT_FONT_PX: f32 = 12.0;

#[derive(Debug, Clone)]
pub struct GraphSnapshot {
    pub stages: Vec<Stage>,
    pub nodes: Vec<(NanoID, Node)>,
    pub root: NanoID,
}

#[derive(Debug, Clone)]
pub struct GraphView {
    pub pan: Vec2,
    pub zoom: f32,
    pub selected: Option<NanoID>,
    pub locked: bool,
    pub(super) needs_fit: bool,
    /// Last graph canvas rect — used by the header toolbar Fit action.
    pub(super) last_canvas_rect: Rect,
    pub(super) undo_stack: Vec<GraphSnapshot>,
    pub(super) redo_stack: Vec<GraphSnapshot>,
    /// Port-drag connect in progress (source node).
    pub(super) connect_drag: Option<NanoID>,
    pub(super) dragging_node: Option<NanoID>,
    /// Node under pointer awaiting drag threshold before move.
    pub(super) pending_node: Option<NanoID>,
    pub(super) drag_accum: Vec2,
    pub(super) panning_bg: bool,
    pub(super) drag_snapshot: Option<GraphSnapshot>,
    /// Right-click menu on a stage node (id + screen pos).
    pub(super) node_menu: Option<(NanoID, Pos2)>,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            selected: None,
            locked: false,
            needs_fit: true,
            last_canvas_rect: Rect::NOTHING,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            connect_drag: None,
            dragging_node: None,
            pending_node: None,
            drag_accum: Vec2::ZERO,
            panning_bg: false,
            drag_snapshot: None,
            node_menu: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphAction {
    None,
    Select(NanoID),
    OpenEditor(NanoID),
    SetRoot(NanoID),
    DeleteStage(NanoID),
    CloneStage(NanoID),
    CloneStageTo(NanoID),
    Arrange,
    /// User asked to clear the canvas (app confirms before wiping).
    ClearCanvas,
    /// Scene graph mutated (edge or node position).
    Dirty,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeButton {
    Edit,
    Clone,
    CloneTo,
    Root,
    Delete,
}

#[derive(Clone, Copy)]
pub(crate) enum StatusKind {
    Start,
    Orgasm,
    Warn,
    Fixed,
}

impl GraphView {
    pub fn push_undo(&mut self, scene: &Scene) {
        self.undo_stack.push(snapshot_of(scene));
        self.redo_stack.clear();
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self, scene: &mut Scene) -> bool {
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(snapshot_of(scene));
        apply_snapshot(scene, &prev);
        true
    }

    pub fn redo(&mut self, scene: &mut Scene) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(snapshot_of(scene));
        apply_snapshot(scene, &next);
        true
    }

    /// Re-fit the view on the next frame (e.g. after switching scenes).
    pub fn request_fit(&mut self) {
        self.needs_fit = true;
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
    }

    /// Zoom keeping the world point under `screen_pos` fixed.
    pub fn zoom_at(&mut self, rect: Rect, screen_pos: Pos2, factor: f32) {
        let world = self.screen_to_world(rect, screen_pos);
        self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        self.pan = (screen_pos - rect.center()) / self.zoom - world.to_vec2();
    }

    pub fn center_view(&mut self, scene: &Scene) {
        let positions = layout_nodes(scene);
        if positions.is_empty() {
            self.pan = Vec2::ZERO;
            return;
        }
        let Some(bounds) = content_bounds_from(&positions) else {
            return;
        };
        self.pan = -bounds.center().to_vec2();
    }

    pub fn fit_view(&mut self, rect: Rect, scene: &Scene) {
        let positions = layout_nodes(scene);
        if positions.is_empty() {
            self.pan = Vec2::ZERO;
            self.zoom = 1.0;
            return;
        }
        let Some(bounds) = content_bounds_from(&positions) else {
            return;
        };
        let content_w = bounds.width().max(1.0);
        let content_h = bounds.height().max(1.0);
        let pad = 48.0;
        let avail_w = (rect.width() - pad * 2.0).max(1.0);
        let avail_h = (rect.height() - pad * 2.0).max(1.0);
        // Zoom so content fills the canvas (may zoom in for sparse scenes).
        self.zoom = (avail_w / content_w)
            .min(avail_h / content_h)
            .clamp(ZOOM_MIN, ZOOM_MAX.min(2.5));
        self.pan = -bounds.center().to_vec2();
    }

    fn port_screen_rect(&self, rect: Rect, world_pos: Pos2) -> Rect {
        let base = self.world_to_screen(rect, world_pos + Vec2::new(NODE_W - 1.0, NODE_H * 0.5));
        Rect::from_min_max(
            Pos2::new(base.x - 4.0 * self.zoom, base.y - 40.0 * self.zoom),
            Pos2::new(base.x + 12.0 * self.zoom, base.y + 40.0 * self.zoom),
        )
    }

    /// Orthogonal edge path from `from`'s out-port to `to`'s in-port, in screen space.
    fn edge_screen_path(&self, rect: Rect, from: Pos2, to: Pos2) -> Vec<Pos2> {
        let start = from + Vec2::new(NODE_W + 9.0, NODE_H * 0.5);
        let end = to + Vec2::new(0.0, NODE_H * 0.5);
        let pad = 24.0;

        let world: Vec<Pos2> = if end.x >= start.x + pad {
            let mid_x = (start.x + end.x) * 0.5;
            vec![
                start,
                Pos2::new(mid_x, start.y),
                Pos2::new(mid_x, end.y),
                end,
            ]
        } else {
            let from_bottom = from.y + NODE_H;
            let to_bottom = to.y + NODE_H;
            let corridor_y = if to.y > from_bottom + pad {
                (from_bottom + to.y) * 0.5
            } else if from.y > to_bottom + pad {
                (to_bottom + from.y) * 0.5
            } else {
                from_bottom.max(to_bottom) + pad * 1.5
            };
            vec![
                start,
                Pos2::new(start.x + pad, start.y),
                Pos2::new(start.x + pad, corridor_y),
                Pos2::new(end.x - pad, corridor_y),
                Pos2::new(end.x - pad, end.y),
                end,
            ]
        };

        rounded_polyline(&world, 10.0)
            .into_iter()
            .map(|p| self.world_to_screen(rect, p))
            .collect()
    }

    fn world_to_screen(&self, rect: Rect, world: Pos2) -> Pos2 {
        rect.center() + (world.to_vec2() + self.pan) * self.zoom
    }

    fn screen_to_world(&self, rect: Rect, screen: Pos2) -> Pos2 {
        let v = (screen - rect.center()) / self.zoom - self.pan;
        Pos2::new(v.x, v.y)
    }
}
