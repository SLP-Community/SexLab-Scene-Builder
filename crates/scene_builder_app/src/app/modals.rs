use super::{ExportConfirm, PendingAction, SceneBuilderApp, REPO_URL};
use crate::io;
use egui::{Context, RichText};
use scene_builder_core::project::NanoID;

impl SceneBuilderApp {
    pub(super) fn modals(&mut self, ctx: &Context) {
        if self.show_close_confirm {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("There are unsaved changes. Continue and discard them?");
                    ui.horizontal(|ui| {
                        if ui.button("Discard").clicked() {
                            self.show_close_confirm = false;
                            self.ws.dirty = false;
                            if let Some(action) = self.pending_after_confirm.take() {
                                match action {
                                    PendingAction::Quit => {
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                    other => self.run_pending(other),
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_close_confirm = false;
                            self.pending_after_confirm = None;
                        }
                    });
                });
        }

        if let Some(confirm) = self.export_confirm.take() {
            let fnis_mod = self.ws.package.fnis_mod_name();
            let mut keep = Some(confirm);
            match keep.as_mut().unwrap() {
                ExportConfirm::Tip { kind, dont_show } => {
                    let kind = *kind;
                    let mut decided: Option<bool> = None;
                    egui::Window::new("Animation clips for Pandora")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.set_max_width(460.0);
                            ui.label(format!(
                                "Export writes into a subfolder named {fnis_mod} under the folder you pick.\n\n\
                                 It writes AnimLists, Behavior files, and registry data — not your .hkx animation clips.\n\n\
                                 Copy your animation HKX files into:\n\
                                 meshes/actors/<race>/animations/{fnis_mod}/\n\n\
                                 For humans that is usually:\n\
                                 meshes/actors/character/animations/{fnis_mod}/\n\n\
                                 Pandora only plays clips that live in the folder the Behavior references."
                            ));
                            ui.add_space(6.0);
                            ui.checkbox(dont_show, "Don't show this tip again on export");
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if ui.button("Continue").clicked() {
                                    decided = Some(true);
                                }
                                if ui.button("Cancel").clicked() {
                                    decided = Some(false);
                                }
                            });
                        });
                    if let Some(proceed) = decided {
                        let dont_show = matches!(
                            keep.as_ref(),
                            Some(ExportConfirm::Tip {
                                dont_show: true,
                                ..
                            })
                        );
                        if dont_show {
                            self.prefs.hide_export_clip_tip = true;
                            self.prefs.save();
                        }
                        keep = None;
                        if proceed {
                            io::spawn_export(self.dialog_tx.clone(), kind);
                        }
                    }
                }
                ExportConfirm::Merge {
                    path,
                    kind,
                    dont_show,
                } => {
                    let kind = *kind;
                    let path = path.clone();
                    let mut decided: Option<bool> = None;
                    egui::Window::new("Export merge")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.set_max_width(460.0);
                            ui.label(format!(
                                "Export writes into a subfolder named {fnis_mod} and soft-merges with anything already there.\n\n\
                                 Matching files are overwritten. Other files (such as .hkx animation clips) are kept.\n\n\
                                 Continue?"
                            ));
                            ui.add_space(6.0);
                            ui.checkbox(dont_show, "Don't warn about export overwrites again");
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if ui.button("Continue").clicked() {
                                    decided = Some(true);
                                }
                                if ui.button("Cancel").clicked() {
                                    decided = Some(false);
                                }
                            });
                        });
                    if let Some(proceed) = decided {
                        let dont_show = matches!(
                            keep.as_ref(),
                            Some(ExportConfirm::Merge {
                                dont_show: true,
                                ..
                            })
                        );
                        if dont_show {
                            self.prefs.hide_export_merge_warn = true;
                            self.prefs.save();
                        }
                        keep = None;
                        if proceed {
                            self.start_export(path, kind);
                        }
                    }
                }
            }
            self.export_confirm = keep;
        }

        if let Some(scene_id) = self.confirm_delete_scene.clone() {
            let name = self
                .ws
                .package
                .get_scene(&scene_id)
                .map(|s| {
                    if s.name.is_empty() {
                        s.id.0.clone()
                    } else {
                        s.name.clone()
                    }
                })
                .unwrap_or_default();
            egui::Window::new("Delete scene")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Delete \"{name}\"? This cannot be undone."));
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new("Delete").color(egui::Color32::RED))
                            .clicked()
                        {
                            self.ws.package.discard_scene(&scene_id);
                            if self.ws.selected_scene.as_ref() == Some(&scene_id) {
                                self.ws.selected_scene = None;
                                self.graph.selected = None;
                            }
                            self.mark_dirty();
                            self.confirm_delete_scene = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_delete_scene = None;
                        }
                    });
                });
        }

        if self.confirm_clear_canvas {
            egui::Window::new("Clear canvas")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Remove all stages from this scene? This can be undone.");
                    ui.horizontal(|ui| {
                        if ui.button("Clear").clicked() {
                            self.confirm_clear_canvas = false;
                            if let Some(id) = self.ws.selected_scene.clone() {
                                if let Some(scene) = self.ws.package.get_scene_mut(&id) {
                                    self.graph.push_undo(scene);
                                    scene.stages.clear();
                                    scene.graph.clear();
                                    scene.root = NanoID::new_nanoid();
                                    self.graph.selected = None;
                                    self.mark_dirty();
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_clear_canvas = false;
                        }
                    });
                });
        }

        if let Some(stage_id) = self.clone_to.clone() {
            let mut close = false;
            let mut target: Option<NanoID> = None;
            let src_n = self
                .ws
                .selected_scene
                .as_ref()
                .and_then(|sid| self.ws.package.get_scene(sid))
                .and_then(|s| s.get_stage(&stage_id))
                .map(|s| s.positions.len())
                .unwrap_or(0);
            egui::Window::new("Clone stage to…")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.clone_to_search)
                            .hint_text("Search scenes"),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "This stage has {src_n} position(s). The target scene will use that count."
                        ))
                        .small()
                        .weak(),
                    );
                    ui.add_space(4.0);
                    let needle = self.clone_to_search.to_lowercase();
                    let mut rows: Vec<(NanoID, String, usize)> = self
                        .ws
                        .package
                        .scenes
                        .iter()
                        .filter(|(id, _)| Some(*id) != self.ws.selected_scene.as_ref())
                        .map(|(id, scene)| {
                            let name = if scene.name.is_empty() {
                                id.0.clone()
                            } else {
                                scene.name.clone()
                            };
                            (id.clone(), name, scene.positions.len())
                        })
                        .filter(|(_, name, _)| {
                            needle.is_empty() || name.to_lowercase().contains(&needle)
                        })
                        .collect();
                    rows.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .show(ui, |ui| {
                            for (id, name, n_pos) in &rows {
                                let label = format!("{name}  ·  {n_pos} pos");
                                if ui.selectable_label(false, label).clicked() {
                                    target = Some(id.clone());
                                }
                            }
                        });
                    ui.add_space(4.0);
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            if let Some(to_scene) = target {
                if let Some(from_scene) = self.ws.selected_scene.clone() {
                    self.clone_stage_to_scene(ctx, &stage_id, &from_scene, &to_scene);
                }
                close = true;
            }
            if close {
                self.clone_to = None;
            }
        }

        if self.show_about {
            egui::Window::new("About SexLab Scene Builder")
                .collapsible(false)
                .resizable(false)
                .open(&mut self.show_about)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "SexLab Scene Builder {}",
                        env!("CARGO_PKG_VERSION")
                    ));
                    ui.label("Apache-2.0 — Scrab and contributors");
                    if ui.link(REPO_URL).clicked() {
                        let _ = open::that(REPO_URL);
                    }
                    ui.separator();
                    ui.label("Third-party: serde-hkx (MIT OR Apache-2.0) for Behavior.hkx packing");
                });
        }

        crate::ui_define::show_job_progress(ctx, &self.job);
    }
}
