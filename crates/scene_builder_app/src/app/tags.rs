use super::SceneBuilderApp;
use crate::furniture::{furniture_label, FURNITURE_GROUPS};
use crate::tags::tag_tree_ui;
use crate::ui_base;
use egui::RichText;
use scene_builder_core::project::NanoID;

impl SceneBuilderApp {
    /// Right column: Scene Tags (fills remaining height, scrolls) + Furniture (pinned).
    pub(super) fn tags_furniture_panel(&mut self, ui: &mut egui::Ui) {
        let Some(scene_id) = self.ws.selected_scene.clone() else {
            return;
        };
        crate::ui_define::fill_width(ui);
        let avail = ui.available_rect_before_wrap();
        let panel_w = ui_base::finite_or(avail.width(), 200.0);
        let avail_h = ui_base::finite_or(avail.height(), 400.0);
        if panel_w <= 0.0 || avail_h <= 0.0 {
            return;
        }

        let furni_id = ui.id().with("furniture_h");
        let tags_min = ui_base::TAGS_SCROLL_MIN_H;
        let prev_furni = ui
            .ctx()
            .data(|d| d.get_temp::<f32>(furni_id))
            .filter(|h| h.is_finite())
            .unwrap_or(168.0);
        let furni_h = prev_furni.clamp(1.0, (avail_h - tags_min).max(1.0));
        let split_y = (avail.max.y - furni_h).max(avail.min.y + tags_min);
        let tags_rect = egui::Rect::from_min_max(avail.min, egui::pos2(avail.max.x, split_y));
        let furni_rect = egui::Rect::from_min_max(egui::pos2(avail.min.x, split_y), avail.max);

        {
            let mut tags_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("scene_tags_block")
                    .max_rect(tags_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            tags_ui.set_clip_rect(tags_rect.intersect(ui.clip_rect()));
            tags_ui.set_max_width(panel_w);
            crate::ui_define::fill_width(&mut tags_ui);

            let mut copy_to_stages = false;
            tags_ui.label(RichText::new("Scene Tags").strong());
            tags_ui.horizontal_wrapped(|ui| {
                ui.set_max_width(panel_w);
                let has_stages = self
                    .ws
                    .package
                    .get_scene(&scene_id)
                    .map(|s| !s.stages.is_empty())
                    .unwrap_or(false);
                if ui
                    .add_enabled(has_stages, egui::Button::new("Copy").small())
                    .on_hover_text("Copy scene tags onto every stage (replaces each stage's tags).")
                    .clicked()
                {
                    copy_to_stages = true;
                }
                crate::ui_define::info_tip(
                    ui,
                    "Tags which are shared between all stages in the scene.",
                );
            });

            egui::ScrollArea::vertical()
                .id_salt("scene_tags_scroll")
                .auto_shrink([false, false])
                .hscroll(false)
                .show(&mut tags_ui, |ui| {
                    ui.set_max_width(panel_w);

                    let mut tags_changed = false;
                    let mut custom_changed = false;
                    if let Some(scene) = self.ws.package.get_scene_mut(&scene_id) {
                        let result = tag_tree_ui(
                            ui,
                            "scene_tags",
                            &mut self.tag_tree_state,
                            &mut scene.tags,
                            &mut self.prefs.custom_tags,
                        );
                        tags_changed = result.tags_changed;
                        custom_changed = result.custom_changed;
                    }
                    if copy_to_stages {
                        if let Some(scene) = self.ws.package.get_scene_mut(&scene_id) {
                            let copied = scene.tags.clone();
                            for stage in &mut scene.stages {
                                stage.tags = copied.clone();
                            }
                        }
                        self.mark_dirty();
                    }
                    if tags_changed {
                        self.mark_dirty();
                    }
                    if custom_changed {
                        self.prefs.save();
                    }
                });
        }

        {
            let mut furni_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("furniture_block")
                    .max_rect(furni_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            furni_ui.set_clip_rect(furni_rect.intersect(ui.clip_rect()));
            furni_ui.set_max_width(panel_w);
            crate::ui_define::fill_width(&mut furni_ui);
            furni_ui.horizontal_wrapped(|ui| {
                crate::ui_define::fill_width(ui);
                ui.label(RichText::new("Furniture").strong());
                crate::ui_define::info_tip(ui, "Furniture settings for the scene.");
            });
            self.furniture_section(&mut furni_ui, &scene_id);
            furni_ui.add_space(4.0);
            let used = furni_ui.min_rect().height();
            if used.is_finite() {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(furni_id, used.max(1.0)));
            }
        }

        ui.allocate_rect(avail, egui::Sense::hover());
    }

    pub(super) fn furniture_section(&mut self, ui: &mut egui::Ui, scene_id: &NanoID) {
        let mut furni_changed = false;
        if let Some(scene) = self.ws.package.get_scene_mut(scene_id) {
            let furniture = &mut scene.furniture;
            let selected_label = {
                let names: Vec<&str> = furniture
                    .furni_types
                    .iter()
                    .map(|t| furniture_label(t))
                    .collect();
                if names.is_empty() {
                    "None".to_string()
                } else {
                    names.join(", ")
                }
            };
            egui::ComboBox::from_id_salt("furniture_select")
                .width(ui_base::finite_or(ui.available_width(), 120.0))
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    let mut none_on = furniture.furni_types.iter().any(|t| t == "None");
                    if ui.checkbox(&mut none_on, "None").changed() {
                        furniture.furni_types = vec!["None".into()];
                        furni_changed = true;
                    }
                    for group in FURNITURE_GROUPS {
                        ui.label(RichText::new(group.label).small());
                        for (label, value) in group.options {
                            let mut on = furniture.furni_types.iter().any(|t| t == value);
                            if ui.checkbox(&mut on, *label).changed() {
                                if on {
                                    furniture.furni_types.retain(|t| t != "None");
                                    furniture.furni_types.push((*value).to_string());
                                    furniture.allow_bed = false;
                                } else {
                                    furniture.furni_types.retain(|t| t != value);
                                    if furniture.furni_types.is_empty() {
                                        furniture.furni_types = vec!["None".into()];
                                    }
                                }
                                furni_changed = true;
                            }
                        }
                    }
                });

            let none_selected = furniture.furni_types.iter().any(|t| t == "None");
            let mut allow_bed = furniture.allow_bed;
            if ui
                .add_enabled(
                    none_selected,
                    egui::Checkbox::new(&mut allow_bed, "Allow Bed"),
                )
                .changed()
            {
                furniture.allow_bed = allow_bed;
                furni_changed = true;
            }
            let mut private = scene.private;
            if ui.checkbox(&mut private, "Private").changed() {
                scene.private = private;
                furni_changed = true;
            }

            ui.add_space(4.0);
            // Avoid ui.columns — it expands the parent when column content
            // exceeds the soft max (was blowing the right panel to ~2k px).
            egui::Grid::new("furniture_offset_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .min_col_width(((ui.available_width() - 8.0) / 2.0).max(40.0))
                .show(ui, |ui| {
                    let offset = &mut scene.furniture.offset;
                    let fields: [(&str, &mut f32, Option<std::ops::RangeInclusive<f32>>); 4] = [
                        ("X", &mut offset.x, None),
                        ("Y", &mut offset.y, None),
                        ("Z", &mut offset.z, None),
                        ("°", &mut offset.r, Some(0.0..=359.9_f32)),
                    ];
                    for (i, (label, value, clamp)) in fields.into_iter().enumerate() {
                        let mut drag = egui::DragValue::new(value).speed(0.1).fixed_decimals(1);
                        if let Some(range) = clamp {
                            drag = drag.range(range);
                        }
                        if crate::ui_define::labeled_drag(ui, label, drag).changed() {
                            furni_changed = true;
                        }
                        if i % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
        }
        if furni_changed {
            self.mark_dirty();
        }
    }
}
