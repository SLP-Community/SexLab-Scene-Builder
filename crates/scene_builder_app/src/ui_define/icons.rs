use crate::graph::{NodeButton, StatusKind};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

/// Painter-drawn status glyphs — crisp at any zoom (no emoji atlas stretch).
pub fn draw_status_icon(painter: &egui::Painter, kind: StatusKind, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    match kind {
        StatusKind::Start => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - s * 0.45, c.y - s * 0.55),
                    Pos2::new(c.x + s * 0.55, c.y),
                    Pos2::new(c.x - s * 0.45, c.y + s * 0.55),
                ],
                color,
                Stroke::NONE,
            ));
        }
        StatusKind::Orgasm => {
            // Convex pieces only — PathShape fill tessellates poorly and can draw a
            // vertical "impale" seam through the cleft at some zoom levels.
            draw_heart(painter, c, s * 0.95, color);
        }
        StatusKind::Warn => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x, c.y - s * 0.75),
                    Pos2::new(c.x + s * 0.75, c.y + s * 0.65),
                    Pos2::new(c.x - s * 0.75, c.y + s * 0.65),
                ],
                color,
                Stroke::NONE,
            ));
            let bar = Color32::WHITE;
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(c.x, c.y + s * 0.05),
                    Vec2::new(s * 0.18, s * 0.55),
                ),
                1.0,
                bar,
            );
            painter.circle_filled(Pos2::new(c.x, c.y + s * 0.48), s * 0.1, bar);
        }
        StatusKind::Fixed => {
            let stroke = Stroke::new((s * 0.22).max(1.5), color);
            painter.line_segment(
                [
                    Pos2::new(c.x - s * 0.7, c.y),
                    Pos2::new(c.x + s * 0.35, c.y),
                ],
                stroke,
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x + s * 0.7, c.y),
                    Pos2::new(c.x + s * 0.15, c.y - s * 0.45),
                    Pos2::new(c.x + s * 0.15, c.y + s * 0.45),
                ],
                color,
                Stroke::NONE,
            ));
            painter.line_segment(
                [
                    Pos2::new(c.x - s * 0.7, c.y - s * 0.45),
                    Pos2::new(c.x - s * 0.7, c.y + s * 0.45),
                ],
                stroke,
            );
        }
    }
}

/// Heart from two circles + a triangle (all convex) — stable at every zoom.
fn draw_heart(painter: &egui::Painter, c: Pos2, s: f32, color: Color32) {
    let r = s * 0.48;
    let lobe_y = c.y - r * 0.22;
    painter.circle_filled(Pos2::new(c.x - r * 0.55, lobe_y), r, color);
    painter.circle_filled(Pos2::new(c.x + r * 0.55, lobe_y), r, color);
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(c.x - r * 1.12, lobe_y + r * 0.05),
            Pos2::new(c.x + r * 1.12, lobe_y + r * 0.05),
            Pos2::new(c.x, c.y + r * 1.25),
        ],
        color,
        Stroke::NONE,
    ));
}
/// Hover-control icons (Edit / Clone / CloneTo / Delete). Root is drawn as text by the caller.
pub fn draw_ctrl_icon(painter: &egui::Painter, btn: NodeButton, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    let stroke = Stroke::new((s * 0.18).max(1.25), color);
    match btn {
        NodeButton::Edit => {
            let a = Pos2::new(c.x - s * 0.55, c.y + s * 0.55);
            let b = Pos2::new(c.x + s * 0.25, c.y - s * 0.25);
            painter.line_segment([a, b], stroke);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    a,
                    Pos2::new(a.x + s * 0.22, a.y - s * 0.08),
                    Pos2::new(a.x + s * 0.08, a.y - s * 0.22),
                ],
                color,
                Stroke::NONE,
            ));
            let e0 = Pos2::new(c.x + s * 0.15, c.y - s * 0.55);
            let e1 = Pos2::new(c.x + s * 0.55, c.y - s * 0.15);
            painter.line_segment([e0, e1], stroke);
            painter.line_segment([Pos2::new(c.x + s * 0.05, c.y - s * 0.35), e0], stroke);
            painter.line_segment([Pos2::new(c.x + s * 0.35, c.y - s * 0.05), e1], stroke);
        }
        NodeButton::Clone => {
            let back = Rect::from_min_max(
                Pos2::new(c.x - s * 0.15, c.y - s * 0.55),
                Pos2::new(c.x + s * 0.55, c.y + s * 0.15),
            );
            let front = Rect::from_min_max(
                Pos2::new(c.x - s * 0.55, c.y - s * 0.15),
                Pos2::new(c.x + s * 0.15, c.y + s * 0.55),
            );
            painter.rect_stroke(back, 2.0, stroke, egui::StrokeKind::Middle);
            painter.rect_filled(front, 2.0, crate::ui_base::SCENE_NODE_BG);
            painter.rect_stroke(front, 2.0, stroke, egui::StrokeKind::Middle);
        }
        NodeButton::CloneTo => {
            let back = Rect::from_min_max(
                Pos2::new(c.x - s * 0.55, c.y - s * 0.45),
                Pos2::new(c.x + s * 0.05, c.y + s * 0.15),
            );
            let front = Rect::from_min_max(
                Pos2::new(c.x - s * 0.35, c.y - s * 0.2),
                Pos2::new(c.x + s * 0.25, c.y + s * 0.4),
            );
            painter.rect_stroke(back, 1.5, stroke, egui::StrokeKind::Middle);
            painter.rect_filled(front, 1.5, crate::ui_base::SCENE_NODE_BG);
            painter.rect_stroke(front, 1.5, stroke, egui::StrokeKind::Middle);
            let tip = Pos2::new(c.x + s * 0.65, c.y + s * 0.05);
            painter.line_segment([Pos2::new(c.x + s * 0.2, c.y + s * 0.05), tip], stroke);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    tip,
                    Pos2::new(tip.x - s * 0.28, tip.y - s * 0.22),
                    Pos2::new(tip.x - s * 0.28, tip.y + s * 0.22),
                ],
                color,
                Stroke::NONE,
            ));
        }
        NodeButton::Delete => {
            let o = s * 0.45;
            painter.line_segment(
                [Pos2::new(c.x - o, c.y - o), Pos2::new(c.x + o, c.y + o)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + o, c.y - o), Pos2::new(c.x - o, c.y + o)],
                stroke,
            );
        }
        NodeButton::Root => {}
    }
}
