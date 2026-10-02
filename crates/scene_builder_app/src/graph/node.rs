use super::draw::truncate_to_width;
use super::{
    GraphView, NodeButton, StatusKind, BTN_DANGER, FIXED_LEN_CYAN, FIXED_LEN_PINK, HEADER_H,
    ICON_FIXED, ICON_HIT, ICON_ORGASM, ICON_START, ICON_WARN, NAME_PX, PORT_FILL, ROOT_BORDER,
    ROOT_FONT_PX,
};
use crate::ui_base::{SCENE_NODE_BG, SCENE_NODE_CONNECT, SCENE_NODE_TEXT};
use crate::ui_define::{draw_node_header_buttons, draw_status_icon};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use scene_builder_core::project::stage::Stage;

impl GraphView {
    /// Draws the node card and returns header button rects (only populated when hovered).
    pub(super) fn draw_node(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        stage: &Stage,
        node_rect: Rect,
        is_root: bool,
        hovered: bool,
        outgoing: usize,
    ) -> Vec<(Rect, NodeButton)> {
        let z = self.zoom;
        let selected = self.selected.as_ref() == Some(&stage.id);
        let connect_src = self.connect_drag.as_ref() == Some(&stage.id);
        let has_climax = stage.positions.iter().any(|p| p.climax);
        let fixed_len = stage.extra.fixed_len;
        let missing_nav = outgoing > 1 && stage.extra.nav_text.trim().is_empty();

        let fill = if fixed_len > 0.0 {
            if fixed_len < 50.0 {
                FIXED_LEN_PINK
            } else {
                FIXED_LEN_CYAN
            }
        } else {
            SCENE_NODE_BG
        };

        let port_stroke_color = if is_root { ROOT_BORDER } else { Color32::BLACK };
        {
            let base_x = node_rect.right() - 1.0 * z;
            let cy = node_rect.center().y;
            let p1 = Pos2::new(base_x, cy - 40.0 * z);
            let p2 = Pos2::new(base_x + 10.0 * z, cy);
            let p3 = Pos2::new(base_x, cy + 40.0 * z);
            painter.add(egui::Shape::convex_polygon(
                vec![p1, p2, p3],
                if connect_src {
                    SCENE_NODE_CONNECT
                } else {
                    PORT_FILL
                },
                Stroke::new(1.0 * z.clamp(0.5, 1.5), port_stroke_color),
            ));
        }

        let border_color = if is_root {
            ROOT_BORDER
        } else {
            crate::ui_base::border_strong(false)
        };
        let rounding = 6.0 * z.clamp(0.5, 1.5);
        painter.rect_filled(node_rect, rounding, fill);
        let line_w = 2.0 * z.clamp(0.5, 1.5);
        painter.rect_stroke(
            node_rect,
            rounding,
            Stroke::new(line_w, border_color),
            egui::StrokeKind::Inside,
        );
        painter.rect_stroke(
            node_rect.shrink(2.0 * line_w),
            (rounding - 2.0 * line_w).max(0.0),
            Stroke::new(line_w, border_color),
            egui::StrokeKind::Inside,
        );
        if selected {
            painter.rect_stroke(
                node_rect.expand(2.0),
                rounding,
                Stroke::new(1.5, crate::ui_base::accent(false)),
                egui::StrokeKind::Outside,
            );
        }

        let header_bottom = node_rect.top() + HEADER_H * z;
        painter.line_segment(
            [
                Pos2::new(node_rect.left() + 3.0 * line_w, header_bottom),
                Pos2::new(node_rect.right() - 3.0 * line_w, header_bottom),
            ],
            Stroke::new(2.0 * z.clamp(0.5, 1.2), Color32::BLACK),
        );

        let header_h = (HEADER_H * z).max(1.0);
        let icon_y = node_rect.top() + header_h * 0.5;
        let mut icon_x = node_rect.left() + 10.0 * z;
        let mut status: Vec<(StatusKind, Color32, &str)> = Vec::new();
        if is_root {
            status.push((StatusKind::Start, ICON_START, "Start Animation"));
        }
        if has_climax {
            status.push((StatusKind::Orgasm, ICON_ORGASM, "Orgasm Stage"));
        }
        if missing_nav {
            status.push((
                StatusKind::Warn,
                ICON_WARN,
                "Missing choice label for this branch",
            ));
        }
        if fixed_len > 0.0 {
            status.push((StatusKind::Fixed, ICON_FIXED, "Fixed Length"));
        }
        let icon_draw = ICON_HIT * z;
        for (kind, color, tip) in &status {
            let icon_rect = Rect::from_center_size(
                Pos2::new(icon_x + icon_draw * 0.5, icon_y),
                Vec2::splat(icon_draw),
            );
            draw_status_icon(painter, *kind, icon_rect, *color);
            if let Some(p) = ui.ctx().pointer_hover_pos() {
                if icon_rect.expand(2.0).contains(p) {
                    egui::show_tooltip_at_pointer(
                        ui.ctx(),
                        ui.layer_id(),
                        egui::Id::new(("node_status_tip", stage.id.0.as_str(), *tip)),
                        |ui| {
                            ui.label(*tip);
                        },
                    );
                }
            }
            icon_x = icon_rect.right() + 6.0 * z.clamp(0.5, 1.2);
        }

        let mut buttons = Vec::new();
        if hovered {
            buttons = draw_node_header_buttons(
                ui,
                painter,
                &stage.id,
                node_rect,
                icon_y,
                z,
                ICON_HIT,
                ROOT_FONT_PX,
                BTN_DANGER,
            );
        }

        let label = if stage.name.is_empty() {
            "Untitled".to_string()
        } else {
            stage.name.clone()
        };
        let pad = 8.0 * z;
        let body_h = (node_rect.bottom() - header_bottom).max(1.0);
        let name_font = egui::FontId::proportional(NAME_PX * z);
        let max_w = (node_rect.width() - pad).max(8.0);
        let text = truncate_to_width(painter, &label, &name_font, max_w);
        let name_area_center = Pos2::new(node_rect.center().x, header_bottom + body_h * 0.5);
        let name_clip = Rect::from_min_max(
            Pos2::new(node_rect.left() + pad * 0.5, header_bottom),
            node_rect.right_bottom(),
        )
        .intersect(painter.clip_rect());
        painter.with_clip_rect(name_clip).text(
            name_area_center,
            egui::Align2::CENTER_CENTER,
            text,
            name_font,
            SCENE_NODE_TEXT,
        );

        buttons
    }
}
