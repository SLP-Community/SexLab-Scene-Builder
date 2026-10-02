use super::{GraphAction, GraphView, BTN_DANGER};
use crate::ui_define::{toolbar_glyph_button, toolbar_icon_button, ToolbarIcon};
use scene_builder_core::project::scene::Scene;

impl GraphView {
    pub fn toolbar_ui(&mut self, ui: &mut egui::Ui, scene: &mut Scene) -> GraphAction {
        let mut action = GraphAction::None;
        let mut dirty = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if toolbar_glyph_button(ui, "↺", "Undo", None, !self.undo_stack.is_empty()).clicked()
                && self.undo(scene)
            {
                dirty = true;
            }
            if toolbar_glyph_button(ui, "↻", "Redo", None, !self.redo_stack.is_empty()).clicked()
                && self.redo(scene)
            {
                dirty = true;
            }
            ui.separator();
            if toolbar_icon_button(ui, ToolbarIcon::Center, "Center content").clicked() {
                self.center_view(scene);
            }
            if toolbar_icon_button(ui, ToolbarIcon::Fit, "Fit to screen").clicked() {
                if self.last_canvas_rect.width() >= 32.0 && self.last_canvas_rect.height() >= 32.0 {
                    self.fit_view(self.last_canvas_rect, scene);
                } else {
                    self.request_fit();
                }
            }
            if toolbar_icon_button(ui, ToolbarIcon::Arrange, "Arrange stages").clicked() {
                self.push_undo(scene);
                action = GraphAction::Arrange;
            }
            if toolbar_glyph_button(
                ui,
                if self.locked { "📌" } else { "✋" },
                "Lock canvas (disables panning)",
                None,
                true,
            )
            .clicked()
            {
                self.locked = !self.locked;
            }
            ui.separator();
            if toolbar_glyph_button(ui, "−", "Zoom out", None, true).clicked() {
                self.zoom_by(0.8);
            }
            if toolbar_glyph_button(ui, "+", "Zoom in", None, true).clicked() {
                self.zoom_by(1.2);
            }
            ui.separator();
            if toolbar_glyph_button(ui, "✕", "Clear canvas", Some(BTN_DANGER), true).clicked() {
                action = GraphAction::ClearCanvas;
            }
        });
        if dirty && matches!(action, GraphAction::None) {
            action = GraphAction::Dirty;
        }
        action
    }
}
