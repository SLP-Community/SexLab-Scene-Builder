use crate::graph::NodeButton;
use crate::ui_base::SCENE_NODE_TEXT;
use crate::ui_define::icons::draw_ctrl_icon;
use egui::{Color32, FontId, Pos2, Rect};
use scene_builder_core::project::NanoID;

pub fn draw(
    ui: &egui::Ui,
    painter: &egui::Painter,
    stage_id: &NanoID,
    node_rect: Rect,
    icon_y: f32,
    z: f32,
    icon_hit: f32,
    root_font_px: f32,
    danger: Color32,
) -> Vec<(Rect, NodeButton)> {
    let pad = 6.0 * z;
    let gap = 3.0 * z;
    let btn_size = icon_hit * z;
    let root_font = FontId::proportional(root_font_px * z);
    let mut root_w = painter
        .layout_no_wrap("Root".to_string(), root_font.clone(), Color32::BLACK)
        .size()
        .x
        + 6.0 * z;
    root_w = root_w.max(btn_size);

    let strip_w = btn_size * 4.0 + root_w + gap * 4.0;
    let max_strip = (node_rect.width() - pad * 2.0).max(btn_size);
    let (btn_size, root_w, root_font) = if strip_w > max_strip {
        let s = max_strip / strip_w;
        (
            (btn_size * s).max(4.0),
            (root_w * s).max(4.0),
            FontId::proportional((root_font_px * z * s).max(5.0)),
        )
    } else {
        (btn_size, root_w, root_font)
    };
    let strip_clip = node_rect.intersect(painter.clip_rect());
    let icon_painter = painter.with_clip_rect(strip_clip);

    let entries: [(NodeButton, &str, Color32); 5] = [
        (NodeButton::Edit, "Edit", SCENE_NODE_TEXT),
        (NodeButton::Clone, "Clone", SCENE_NODE_TEXT),
        (NodeButton::CloneTo, "Clone to…", SCENE_NODE_TEXT),
        (NodeButton::Root, "Mark as root", SCENE_NODE_TEXT),
        (NodeButton::Delete, "Delete", danger),
    ];
    let mut buttons = Vec::new();
    let mut right = node_rect.right() - pad;
    for (btn, tip, color) in entries.iter().rev() {
        let w = if *btn == NodeButton::Root {
            root_w
        } else {
            btn_size
        };
        let h = btn_size;
        let btn_rect = Rect::from_min_max(
            Pos2::new(right - w, icon_y - h * 0.5),
            Pos2::new(right, icon_y + h * 0.5),
        );
        let over = ui
            .ctx()
            .pointer_hover_pos()
            .map(|p| btn_rect.contains(p))
            .unwrap_or(false);
        if over {
            icon_painter.rect_filled(btn_rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, 20));
            egui::show_tooltip_at_pointer(
                ui.ctx(),
                ui.layer_id(),
                egui::Id::new(("node_btn_tip", stage_id.0.as_str(), *tip)),
                |ui| {
                    ui.label(*tip);
                },
            );
        }
        match btn {
            NodeButton::Root => {
                icon_painter.text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Root",
                    root_font.clone(),
                    *color,
                );
            }
            other => draw_ctrl_icon(&icon_painter, *other, btn_rect.shrink(2.0), *color),
        }
        buttons.push((btn_rect, *btn));
        right -= w + gap;
    }
    buttons
}
