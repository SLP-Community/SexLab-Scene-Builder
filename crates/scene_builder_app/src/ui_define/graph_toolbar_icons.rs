use egui::{Color32, Pos2, Rect, Stroke};

pub const TOOLBAR_BTN: f32 = 22.0;

#[derive(Clone, Copy)]
pub enum ToolbarIcon {
    Center,
    Fit,
    Arrange,
}

pub fn toolbar_icon_button(ui: &mut egui::Ui, icon: ToolbarIcon, tip: &str) -> egui::Response {
    let size = egui::vec2(TOOLBAR_BTN, TOOLBAR_BTN);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let visuals = ui.style().interact(&resp);
    ui.painter().rect(
        rect,
        2.0,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let color = visuals.fg_stroke.color;
    draw_toolbar_icon(ui.painter(), icon, rect.shrink(4.0), color);
    resp.on_hover_text(tip)
}

pub fn toolbar_glyph_button(
    ui: &mut egui::Ui,
    glyph: &str,
    tip: &str,
    color: Option<Color32>,
    enabled: bool,
) -> egui::Response {
    let size = egui::vec2(TOOLBAR_BTN, TOOLBAR_BTN);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, mut resp) = ui.allocate_exact_size(size, sense);
    if !enabled {
        resp = resp.on_disabled_hover_text(tip);
    }
    let visuals = ui.style().interact(&resp);
    ui.painter().rect(
        rect,
        2.0,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let fg = color.unwrap_or(visuals.fg_stroke.color);
    let fg = if enabled { fg } else { Color32::from_gray(130) };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(14.0),
        fg,
    );
    if enabled {
        resp.on_hover_text(tip)
    } else {
        resp
    }
}

fn draw_toolbar_icon(painter: &egui::Painter, icon: ToolbarIcon, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    let stroke = Stroke::new((s * 0.22).max(1.1), color);
    match icon {
        ToolbarIcon::Center => {
            painter.line_segment(
                [Pos2::new(c.x - s, c.y), Pos2::new(c.x - s * 0.25, c.y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + s * 0.25, c.y), Pos2::new(c.x + s, c.y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y - s), Pos2::new(c.x, c.y - s * 0.25)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y + s * 0.25), Pos2::new(c.x, c.y + s)],
                stroke,
            );
            painter.circle_stroke(c, s * 0.28, stroke);
        }
        ToolbarIcon::Fit => {
            let r = Rect::from_center_size(c, egui::vec2(s * 1.6, s * 1.2));
            painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
            let t = s * 0.35;
            for (x, y, dx, dy) in [
                (r.left(), r.top(), 1.0, 1.0),
                (r.right(), r.top(), -1.0, 1.0),
                (r.left(), r.bottom(), 1.0, -1.0),
                (r.right(), r.bottom(), -1.0, -1.0),
            ] {
                painter.line_segment([Pos2::new(x, y), Pos2::new(x + dx * t, y)], stroke);
                painter.line_segment([Pos2::new(x, y), Pos2::new(x, y + dy * t)], stroke);
            }
        }
        ToolbarIcon::Arrange => {
            let w = s * 1.1;
            let h = s * 0.45;
            let gap = s * 0.22;
            for i in 0..3 {
                let y = c.y - (h + gap) + i as f32 * (h + gap);
                let node = Rect::from_center_size(Pos2::new(c.x, y), egui::vec2(w, h));
                painter.rect_stroke(node, 1.0, stroke, egui::StrokeKind::Middle);
            }
        }
    }
}
