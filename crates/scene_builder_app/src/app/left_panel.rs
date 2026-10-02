use super::SceneBuilderApp;
use egui::RichText;
use scene_builder_core::project::NanoID;

impl SceneBuilderApp {
    pub(super) fn left_panel(&mut self, ui: &mut egui::Ui) {
        crate::ui_define::fill_width(ui);
        let full = ui.available_width();
        let muted = crate::ui_base::text_muted(ui.visuals().dark_mode);
        for (value, hint) in [
            (&mut self.ws.package.pack_name, "Package Name"),
            (&mut self.ws.package.pack_author, "Author Name"),
            (&mut self.ws.package.pack_version, "Pack Version"),
        ] {
            if ui
                .add(
                    egui::TextEdit::singleline(value)
                        .hint_text(RichText::new(hint).color(muted).italics())
                        .desired_width(full),
                )
                .changed()
            {
                self.ws.dirty = true;
            }
        }

        ui.separator();

        if ui
            .add(egui::Button::new("+  New Scene").frame(false))
            .clicked()
        {
            self.add_blank_scene();
        }

        ui.add(
            egui::TextEdit::singleline(&mut self.scene_search)
                .hint_text("Search scenes")
                .desired_width(full),
        );

        let mut to_delete: Option<NanoID> = None;
        let mut to_select: Option<NanoID> = None;
        let count = self.ws.package.scenes.len();
        let header = if count > 0 {
            format!("Scenes ({count})")
        } else {
            "Scenes".to_string()
        };
        let needle = self.scene_search.trim().to_lowercase();
        let mut rows: Vec<(NanoID, String, bool)> = self
            .ws
            .package
            .scenes
            .iter()
            .map(|(id, scene)| {
                let label = if scene.name.is_empty() {
                    id.0.clone()
                } else {
                    scene.name.clone()
                };
                (id.clone(), label, scene.has_warnings)
            })
            .collect();
        if !needle.is_empty() {
            rows.retain(|(_, label, _)| label.to_lowercase().contains(&needle));
        }
        rows.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .hscroll(false)
            .show(ui, |ui| {
                crate::ui_define::fill_width(ui);
                egui::CollapsingHeader::new(header)
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            crate::ui_define::fill_width(ui);
                            for (id, label, has_warnings) in &rows {
                                let selected = self.ws.selected_scene.as_ref() == Some(id);
                                let icon = if *has_warnings {
                                    RichText::new("⚠").color(egui::Color32::RED)
                                } else {
                                    RichText::new("◆").color(egui::Color32::from_rgb(17, 175, 17))
                                };
                                ui.horizontal(|ui| {
                                    crate::ui_define::fill_width(ui);
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.label(icon);
                                    let resp = truncated_selectable(ui, selected, label)
                                        .on_hover_text(label);
                                    if resp.clicked() {
                                        to_select = Some(id.clone());
                                    }
                                    resp.context_menu(|ui| {
                                        if ui.button("Edit").clicked() {
                                            to_select = Some(id.clone());
                                            ui.close_menu();
                                        }
                                        if ui
                                            .button(
                                                RichText::new("Delete").color(egui::Color32::RED),
                                            )
                                            .clicked()
                                        {
                                            to_delete = Some(id.clone());
                                            ui.close_menu();
                                        }
                                    });
                                });
                            }
                        });
                    });
            });

        if let Some(id) = to_select {
            self.select_scene(id);
        }
        if let Some(id) = to_delete {
            self.confirm_delete_scene = Some(id);
        }
    }
}

fn truncated_selectable(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let w = ui.available_width().max(0.0);
    let h = ui.spacing().interact_size.y;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let visuals = ui.style().interact_selectable(&resp, selected);
    if selected || resp.hovered() || resp.has_focus() {
        ui.painter()
            .rect_filled(rect, visuals.corner_radius, visuals.weak_bg_fill);
    }
    let pad = ui.spacing().button_padding.x;
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        (rect.width() - pad * 2.0).max(0.0),
        egui::TextStyle::Button,
    );
    let pos = egui::pos2(rect.left() + pad, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(pos, galley, visuals.text_color());
    resp
}
