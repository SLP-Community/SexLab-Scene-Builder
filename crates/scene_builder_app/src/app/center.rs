use super::SceneBuilderApp;
use crate::graph::GraphAction;
use crate::graph_layout::arrange_scene;
use crate::ui_base;
use egui::RichText;
use scene_builder_core::project::NanoID;

impl SceneBuilderApp {
    pub(super) fn center_panel(&mut self, ui: &mut egui::Ui) {
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        let Some(scene_id) = self.ws.selected_scene.clone() else {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.4);
                    ui.label(RichText::new("No scene loaded :(").weak());
                    ui.add_space(ui_base::SPACE);
                    if ui.button("New Scene").clicked() {
                        self.add_blank_scene();
                    }
                });
            });
            return;
        };

        let mut open_editor: Option<NanoID> = None;
        let mut add_stage = false;
        let mut store = false;
        let mut rename: Option<String> = None;
        let mut toolbar_action = crate::graph::GraphAction::None;

        {
            let Some(scene) = self.ws.package.get_scene_mut(&scene_id) else {
                ui.label("Scene missing");
                return;
            };

            // Name | graph controls | Add Stage + Store, packed from the right
            // of the *clipped* center column so the action buttons cannot paint
            // over the tags sidebar when the leftover width is under ~616px.
            let full = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let row_h = ui.spacing().interact_size.y.max(28.0);
            let (left_rect, mid_rect, right_rect) = scene_header_strips(full, row_h);

            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(left_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.set_clip_rect(ui.clip_rect().intersect(left_rect));
                    // Always take the dirty-slot so the name field's id does not
                    // jump when ≠ appears after the first keystroke.
                    let dirty_sz = egui::vec2(22.0, 22.0);
                    let (dirty_rect, dirty_resp) =
                        ui.allocate_exact_size(dirty_sz, egui::Sense::hover());
                    if self.ws.dirty {
                        ui.painter().text(
                            dirty_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "≠",
                            egui::FontId::proportional(22.0),
                            egui::Color32::RED,
                        );
                        dirty_resp.on_hover_text("Unsaved changes");
                    }
                    let mut name = scene.name.clone();
                    let name_edit = egui::TextEdit::singleline(&mut name)
                        .frame(false)
                        .hint_text("Scene Name")
                        .font(egui::TextStyle::Heading)
                        .id_salt(("scene_name", scene_id.0.as_str()))
                        .desired_width((ui.available_width() - 8.0).max(60.0));
                    let output = name_edit.show(ui);
                    if output.response.changed() {
                        rename = Some(name.clone());
                    }
                    if output.response.gained_focus() && output.response.clicked() {
                        if let Some(mut state) =
                            egui::TextEdit::load_state(ui.ctx(), output.response.id)
                        {
                            let range = egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(name.chars().count()),
                            );
                            state.cursor.set_char_range(Some(range));
                            state.store(ui.ctx(), output.response.id);
                        }
                    }
                },
            );

            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(mid_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.set_clip_rect(ui.clip_rect().intersect(mid_rect));
                    ui.separator();
                    toolbar_action = self.graph.toolbar_ui(ui, scene);
                },
            );

            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(right_rect)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                |ui| {
                    ui.set_clip_rect(ui.clip_rect().intersect(right_rect));
                    let accent = crate::ui_base::accent(ui.visuals().dark_mode);
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Store").color(egui::Color32::WHITE))
                                .fill(accent),
                        )
                        .clicked()
                    {
                        store = true;
                    }
                    if ui.button("Add Stage").clicked() {
                        add_stage = true;
                    }
                    ui.separator();
                },
            );

            // Reserve the row; strip UIs are clipped so they must not expand width.
            let _ = ui.allocate_rect(
                egui::Rect::from_min_size(full.min, egui::vec2(full.width(), row_h)),
                egui::Sense::hover(),
            );

            ui.separator();
        }

        if let Some(name) = rename {
            if let Some(scene) = self.ws.package.get_scene_mut(&scene_id) {
                scene.name = name;
                self.mark_dirty();
            }
        }
        if store {
            self.store_scene(ui.ctx(), &scene_id);
        }

        let action = {
            let Some(scene) = self.ws.package.get_scene_mut(&scene_id) else {
                return;
            };
            egui::Frame::canvas(ui.style())
                .show(ui, |ui| self.graph.ui(ui, scene))
                .inner
        };

        let action = if !matches!(action, crate::graph::GraphAction::None) {
            action
        } else {
            toolbar_action
        };

        match action {
            GraphAction::None => {}
            GraphAction::Select(_) => {}
            GraphAction::OpenEditor(id) => {
                open_editor = Some(id);
            }
            GraphAction::CloneStage(id) => {
                self.clone_stage_in_scene(&scene_id, &id);
            }
            GraphAction::CloneStageTo(id) => {
                self.clone_to = Some(id);
                self.clone_to_search.clear();
            }
            GraphAction::ClearCanvas => {
                self.confirm_clear_canvas = true;
            }
            GraphAction::SetRoot(id) => {
                self.set_scene_root(&scene_id, &id);
            }
            GraphAction::DeleteStage(id) => {
                self.delete_stage_from_scene(&scene_id, &id);
            }
            GraphAction::Arrange => {
                if let Some(scene) = self.ws.package.get_scene_mut(&scene_id) {
                    arrange_scene(scene);
                    self.mark_dirty();
                }
            }
            GraphAction::Dirty => {
                self.mark_dirty();
            }
        }

        if add_stage {
            if let Some(id) = self.add_stage_to_scene(&scene_id) {
                open_editor = Some(id);
            }
        }
        if let Some(stage_id) = open_editor {
            self.open_stage_editor(&scene_id, &stage_id);
        }
    }
}

/// Name | toolbar | Add Stage+Store. Packed from the right of `full` so the
/// action buttons stay inside the center column instead of painting over the
/// tags sidebar when width is tight.
fn scene_header_strips(full: egui::Rect, row_h: f32) -> (egui::Rect, egui::Rect, egui::Rect) {
    const ACTIONS_W: f32 = 196.0;
    const TOOLBAR_W: f32 = 340.0;
    let w = full.width().max(0.0);
    let actions_w = ACTIONS_W.min(w);
    let toolbar_w = TOOLBAR_W.min((w - actions_w).max(0.0));
    let y0 = full.min.y;
    let y1 = y0 + row_h;
    let actions = egui::Rect::from_min_max(
        egui::pos2(full.max.x - actions_w, y0),
        egui::pos2(full.max.x, y1),
    );
    let toolbar = egui::Rect::from_min_max(
        egui::pos2(actions.min.x - toolbar_w, y0),
        egui::pos2(actions.min.x, y1),
    );
    let name = egui::Rect::from_min_max(egui::pos2(full.min.x, y0), egui::pos2(toolbar.min.x, y1));
    (name, toolbar, actions)
}
