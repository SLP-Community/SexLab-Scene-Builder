use super::draw::{
    dist_to_polyline, draw_edge_path, draw_mesh_grid, graph_bg, layout_nodes, snapshot_of,
};
use super::{
    ensure_graph_node, GraphAction, GraphView, NodeButton, BTN_DANGER, DRAG_THRESHOLD,
    EDGE_HIT_DIST, NODE_H, NODE_W,
};
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use scene_builder_core::project::scene::Scene;
use scene_builder_core::project::NanoID;

impl GraphView {
    pub fn ui(&mut self, ui: &mut egui::Ui, scene: &mut Scene) -> GraphAction {
        let mut action = GraphAction::None;
        let mut dirty = false;
        let (response, mut painter) =
            ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        if self.last_canvas_rect != Rect::NOTHING
            && self.last_canvas_rect.width() > 1.0
            && self.zoom > 0.0
        {
            let old_c = self.last_canvas_rect.center();
            let new_c = rect.center();
            let shift = old_c - new_c;
            if shift.length_sq() > 0.01 {
                self.pan += shift / self.zoom;
            }
        }
        self.last_canvas_rect = rect;
        painter.rect_filled(rect, 0.0, graph_bg(ui.visuals().dark_mode));
        painter.set_clip_rect(rect.intersect(ui.clip_rect()));

        if self.needs_fit {
            // Wait until the canvas has a real size (first frames after import can be tiny).
            if rect.width() >= 120.0 && rect.height() >= 80.0 {
                self.fit_view(rect, scene);
                self.needs_fit = false;
            }
        }

        draw_mesh_grid(&painter, rect, self.pan, self.zoom, ui.visuals().dark_mode);

        let dark = ui.visuals().dark_mode;
        let mut positions = layout_nodes(scene);

        let hover_pos = response.hover_pos();
        let pointer_pos = response.interact_pointer_pos();

        let node_rects: Vec<(NanoID, Rect)> = scene
            .stages
            .iter()
            .filter_map(|s| {
                positions.get(&s.id).map(|p| {
                    let tl = self.world_to_screen(rect, *p);
                    let br = self.world_to_screen(rect, *p + Vec2::new(NODE_W, NODE_H));
                    (s.id.clone(), Rect::from_min_max(tl, br))
                })
            })
            .collect();
        let topmost_node_at = |pos: Pos2| -> Option<NanoID> {
            node_rects
                .iter()
                .rev()
                .find(|(_, r)| r.contains(pos))
                .map(|(id, _)| id.clone())
        };

        let edge_color = if dark {
            Color32::from_gray(205)
        } else {
            Color32::BLACK
        };
        let edge_stroke = Stroke::new(1.6 * self.zoom.clamp(0.6, 1.6), edge_color);
        let mut edge_paths: Vec<(NanoID, NanoID, Vec<Pos2>)> = Vec::new();
        for (id, node) in &scene.graph {
            let Some(from) = positions.get(id).copied() else {
                continue;
            };
            for dest in &node.dest {
                let Some(to) = positions.get(dest).copied() else {
                    continue;
                };
                let path = self.edge_screen_path(rect, from, to);
                draw_edge_path(&painter, &path, edge_stroke, self.zoom);
                edge_paths.push((id.clone(), dest.clone(), path));
            }
        }

        if let (Some(from_id), Some(pointer)) = (&self.connect_drag, hover_pos) {
            if let Some(from) = positions.get(from_id).copied() {
                let start =
                    self.world_to_screen(rect, from + Vec2::new(NODE_W + 9.0, NODE_H * 0.5));
                draw_edge_path(&painter, &[start, pointer], edge_stroke, self.zoom);
            }
        }

        let mut opened_node_menu = false;
        if response.secondary_clicked() {
            if let Some(pos) = pointer_pos {
                if let Some(id) = topmost_node_at(pos) {
                    self.node_menu = Some((id, pos));
                    opened_node_menu = true;
                } else {
                    self.node_menu = None;
                    let hit = edge_paths
                        .iter()
                        .find(|(_, _, path)| dist_to_polyline(pos, path) <= EDGE_HIT_DIST)
                        .map(|(a, b, _)| (a.clone(), b.clone()));
                    if let Some((from, to)) = hit {
                        self.push_undo(scene);
                        if let Some(node) = scene.graph.get_mut(&from) {
                            node.dest.retain(|d| d != &to);
                        }
                        dirty = true;
                    }
                }
            }
        }

        let mut clicked: Option<NanoID> = None;
        let mut double: Option<NanoID> = None;
        let mut button_hit: Option<(NanoID, NodeButton)> = None;
        let mut hover_button = false;

        for stage in &scene.stages {
            let Some(pos) = positions.get(&stage.id).copied() else {
                continue;
            };
            let node_rect = {
                let tl = self.world_to_screen(rect, pos);
                let br = self.world_to_screen(rect, pos + Vec2::new(NODE_W, NODE_H));
                Rect::from_min_max(tl, br)
            };
            if !rect.intersects(node_rect.expand(60.0 * self.zoom)) {
                continue;
            }
            let is_root = scene.root == stage.id;
            let hovered = hover_pos.map(|p| node_rect.contains(p)).unwrap_or(false);
            let outgoing = scene
                .graph
                .get(&stage.id)
                .map(|n| n.dest.len())
                .unwrap_or(0);

            let buttons =
                self.draw_node(ui, &painter, stage, node_rect, is_root, hovered, outgoing);

            if hovered {
                if let Some(p) = hover_pos {
                    for (r, b) in &buttons {
                        if r.contains(p) {
                            hover_button = true;
                            if response.clicked() {
                                button_hit = Some((stage.id.clone(), *b));
                            }
                        }
                    }
                }
            }

            if let Some(p) = pointer_pos {
                if node_rect.contains(p) && button_hit.is_none() && !hover_button {
                    if response.double_clicked() {
                        double = Some(stage.id.clone());
                    } else if response.clicked() {
                        clicked = Some(stage.id.clone());
                    }
                }
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(p) = pointer_pos {
                let port_hit = scene.stages.iter().rev().find_map(|s| {
                    let pos = positions.get(&s.id)?;
                    let port = self.port_screen_rect(rect, *pos);
                    port.contains(p).then(|| s.id.clone())
                });
                if let Some(id) = port_hit {
                    self.connect_drag = Some(id);
                    self.pending_node = None;
                    self.panning_bg = false;
                } else if hover_button {
                    self.pending_node = None;
                    self.panning_bg = false;
                } else if let Some(id) = topmost_node_at(p) {
                    self.pending_node = Some(id.clone());
                    self.drag_accum = Vec2::ZERO;
                    self.drag_snapshot = Some(snapshot_of(scene));
                    self.selected = Some(id);
                    self.panning_bg = false;
                } else if !self.locked {
                    self.panning_bg = true;
                    self.pending_node = None;
                    self.dragging_node = None;
                }
            }
        }
        if response.drag_started_by(egui::PointerButton::Middle) {
            if !self.locked {
                self.panning_bg = true;
                self.pending_node = None;
            }
        }
        if response.drag_started_by(egui::PointerButton::Secondary) {
            let over_node = pointer_pos.and_then(|p| topmost_node_at(p)).is_some();
            if !self.locked && !over_node {
                self.panning_bg = true;
                self.pending_node = None;
            }
        }

        let primary_drag = response.dragged_by(egui::PointerButton::Primary);
        let mid_right_drag = response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary);

        if primary_drag || mid_right_drag {
            ui.ctx().request_repaint();
            let delta = response.drag_delta();

            if self.connect_drag.is_some() {
            } else if self.panning_bg && !self.locked {
                self.pan += delta / self.zoom;
            } else if let Some(id) = self.pending_node.clone() {
                self.drag_accum += delta;
                if self.drag_accum.length() >= DRAG_THRESHOLD {
                    self.dragging_node = Some(id);
                    self.pending_node = None;
                    let world_delta = self.drag_accum / self.zoom;
                    if let Some(node) = scene.graph.get_mut(self.dragging_node.as_ref().unwrap()) {
                        node.x += world_delta.x;
                        node.y += world_delta.y;
                        dirty = true;
                    }
                    self.drag_accum = Vec2::ZERO;
                }
            } else if let Some(id) = self.dragging_node.clone() {
                if primary_drag {
                    let world_delta = delta / self.zoom;
                    if let Some(pos) = positions.get_mut(&id) {
                        *pos += world_delta;
                    }
                    if let Some(node) = scene.graph.get_mut(&id) {
                        node.x += world_delta.x;
                        node.y += world_delta.y;
                        dirty = true;
                    } else if let Some(pos) = positions.get(&id).copied() {
                        ensure_graph_node(scene, &id, 0);
                        if let Some(node) = scene.graph.get_mut(&id) {
                            node.x = pos.x;
                            node.y = pos.y;
                            dirty = true;
                        }
                    }
                }
            }
        }

        if response.drag_stopped() {
            if let Some(from) = self.connect_drag.take() {
                if let Some(p) = hover_pos.or(pointer_pos) {
                    if let Some(target) = topmost_node_at(p) {
                        if target != from {
                            self.push_undo(scene);
                            ensure_graph_node(scene, &from, 0);
                            if let Some(node) = scene.graph.get_mut(&from) {
                                if !node.dest.iter().any(|d| d == &target) {
                                    node.dest.push(target.clone());
                                    dirty = true;
                                }
                            }
                            ensure_graph_node(scene, &target, 0);
                        }
                    }
                }
            }
            if dirty {
                if let Some(snap) = self.drag_snapshot.take() {
                    self.undo_stack.push(snap);
                    self.redo_stack.clear();
                }
            }
            self.dragging_node = None;
            self.pending_node = None;
            self.panning_bg = false;
            self.drag_accum = Vec2::ZERO;
            self.drag_snapshot = None;
        }

        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                let factor = (scroll * 0.00105).exp();
                if let Some(pointer) = hover_pos {
                    self.zoom_at(rect, pointer, factor);
                } else {
                    self.zoom_by(factor);
                }
                ui.ctx().request_repaint();
            }
        }

        if let Some((id, btn)) = button_hit {
            self.selected = Some(id.clone());
            action = match btn {
                NodeButton::Edit => GraphAction::OpenEditor(id),
                NodeButton::Clone => GraphAction::CloneStage(id),
                NodeButton::CloneTo => GraphAction::CloneStageTo(id),
                NodeButton::Root => GraphAction::SetRoot(id),
                NodeButton::Delete => GraphAction::DeleteStage(id),
            };
        } else if let Some(id) = double {
            self.selected = Some(id.clone());
            action = GraphAction::OpenEditor(id);
        } else if let Some(id) = clicked {
            self.selected = Some(id.clone());
            action = GraphAction::Select(id);
        } else if response.clicked()
            && self.dragging_node.is_none()
            && self.pending_node.is_none()
            && !self.panning_bg
            && !hover_button
        {
            if let Some(pos) = pointer_pos {
                if topmost_node_at(pos).is_none() {
                    self.selected = None;
                }
            }
        }

        if let Some((id, pos)) = self.node_menu.clone() {
            let mut picked: Option<GraphAction> = None;
            let mut close = false;
            let mut remove_links = false;
            let popup = egui::Area::new(egui::Id::new("stage_node_ctx"))
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .constrain(true)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(160.0);
                        if ui.button("Edit").clicked() {
                            picked = Some(GraphAction::OpenEditor(id.clone()));
                        }
                        if ui.button("Clone").clicked() {
                            picked = Some(GraphAction::CloneStage(id.clone()));
                        }
                        if ui.button("Clone to…").clicked() {
                            picked = Some(GraphAction::CloneStageTo(id.clone()));
                        }
                        if ui.button("Mark as root").clicked() {
                            picked = Some(GraphAction::SetRoot(id.clone()));
                        }
                        if ui.button("Remove connections").clicked() {
                            remove_links = true;
                        }
                        ui.separator();
                        if ui
                            .add(egui::Button::new(RichText::new("Delete").color(BTN_DANGER)))
                            .clicked()
                        {
                            picked = Some(GraphAction::DeleteStage(id.clone()));
                        }
                    });
                });
            if remove_links {
                self.push_undo(scene);
                if let Some(node) = scene.graph.get_mut(&id) {
                    node.dest.clear();
                }
                for node in scene.graph.values_mut() {
                    node.dest.retain(|d| d != &id);
                }
                dirty = true;
                close = true;
            }
            if picked.is_some() {
                close = true;
            }
            // The RMB that opened the menu is "elsewhere" on this frame.
            if popup.response.clicked_elsewhere() && !opened_node_menu {
                close = true;
            }
            if let Some(act) = picked {
                self.selected = Some(id);
                action = act;
            }
            if close {
                self.node_menu = None;
            }
        }

        if dirty {
            match action {
                GraphAction::OpenEditor(_)
                | GraphAction::SetRoot(_)
                | GraphAction::DeleteStage(_)
                | GraphAction::CloneStage(_)
                | GraphAction::CloneStageTo(_)
                | GraphAction::ClearCanvas
                | GraphAction::Arrange => {}
                _ => action = GraphAction::Dirty,
            }
        }

        action
    }
}
