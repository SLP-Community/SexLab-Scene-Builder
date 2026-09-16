use crate::app_prefs::Prefs;
use crate::graph::{self, GraphView};
use crate::io::{self, DialogResult};
use crate::jobs::{JobEvent, JobUi};
use crate::stage_editor::{show_stage_editor, StageEditorAction, StageEditorState};
use crate::tags::TagTreeState;
use crate::toasts::Toasts;
use crate::ui_base;
use crate::workspace::Workspace;
use eframe::App;
use egui::Context;
use scene_builder_core::project::package::ExportKind;
use scene_builder_core::project::NanoID;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

mod center;
mod commands;
mod left_panel;
mod menu_bar;
mod modals;
mod scene_ops;
mod tags;

pub(super) const WIKI_URL: &str =
    "https://slp-community.github.io/SexLab-Wiki/slsb/creating-packs-using-slsb/";
pub(super) const DISCORD_URL: &str = "https://discord.gg/JPSHb4ebqj";
pub(super) const PATREON_URL: &str = "https://www.patreon.com/ScrabJoseline";
pub(super) const KOFI_URL: &str = "https://ko-fi.com/scrab";
pub(super) const KOFI_MISS_URL: &str = "https://ko-fi.com/misscorruption";
pub(super) const REPO_URL: &str = "https://github.com/SLP-Community/SexLab-Scene-Builder";

pub(super) enum PendingAction {
    New,
    Open,
    ImportSlal,
    Quit,
}

/// Pre-export confirmations (Pandora clip tip and merge warning).
pub(super) enum ExportConfirm {
    Tip {
        kind: ExportKind,
        dont_show: bool,
    },
    Merge {
        path: PathBuf,
        kind: ExportKind,
        dont_show: bool,
    },
}

pub struct SceneBuilderApp {
    pub(super) ws: Workspace,
    pub(super) prefs: Prefs,
    pub(super) graph: GraphView,
    pub(super) stage_editor: Option<StageEditorState>,
    pub(super) job: JobUi,
    pub(super) job_rx: Receiver<JobEvent>,
    pub(super) job_tx: Sender<JobEvent>,
    pub(super) dialog_rx: Receiver<DialogResult>,
    pub(super) dialog_tx: Sender<DialogResult>,
    pub(super) show_close_confirm: bool,
    pub(super) show_about: bool,
    pub(super) pending_after_confirm: Option<PendingAction>,
    pub(super) status: String,
    /// Stage awaiting a target scene in the "Clone to…" modal.
    pub(super) clone_to: Option<NanoID>,
    pub(super) clone_to_search: String,
    pub(super) scene_search: String,
    pub(super) confirm_clear_canvas: bool,
    pub(super) confirm_delete_scene: Option<NanoID>,
    pub(super) export_confirm: Option<ExportConfirm>,
    pub(super) tag_tree_state: TagTreeState,
    pub(super) race_keys: Vec<String>,
    pub(super) toasts: Toasts,
}

impl SceneBuilderApp {
    pub const APP_TITLE: &'static str = "SexLab Scene Builder";
    pub fn new(prefs: Prefs) -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let (dialog_tx, dialog_rx) = mpsc::channel();
        Self {
            ws: Workspace::new(),
            prefs,
            graph: GraphView::default(),
            stage_editor: None,
            job: JobUi::default(),
            job_rx,
            job_tx,
            dialog_rx,
            dialog_tx,
            show_close_confirm: false,
            show_about: false,
            pending_after_confirm: None,
            status: String::new(),
            clone_to: None,
            clone_to_search: String::new(),
            scene_search: String::new(),
            confirm_clear_canvas: false,
            confirm_delete_scene: None,
            export_confirm: None,
            tag_tree_state: TagTreeState::default(),
            race_keys: scene_builder_core::racekeys::get_race_keys_string(),
            toasts: Toasts::default(),
        }
    }

    fn window_title(&self) -> String {
        let name = self.ws.pack_display_name();
        if self.ws.dirty {
            format!("* {} - {}", name, Self::APP_TITLE)
        } else if self.ws.package.pack_name.is_empty() && !self.ws.has_save_path() {
            Self::APP_TITLE.to_string()
        } else {
            format!("{} - {}", name, Self::APP_TITLE)
        }
    }

    fn mark_dirty(&mut self) {
        self.ws.mark_dirty();
    }

    fn request_if_clean(&mut self, action: PendingAction) {
        if self.ws.dirty {
            self.pending_after_confirm = Some(action);
            self.show_close_confirm = true;
        } else {
            self.run_pending(action);
        }
    }

    fn run_pending(&mut self, action: PendingAction) {
        match action {
            PendingAction::New => {
                self.ws.reset();
                self.graph.selected = None;
                self.stage_editor = None;
                self.status = "New project".into();
            }
            PendingAction::Open => io::spawn_open(self.dialog_tx.clone()),
            PendingAction::ImportSlal => io::spawn_slal(self.dialog_tx.clone()),
            PendingAction::Quit => {}
        }
    }

    fn handle_stage_editor(&mut self, ctx: &Context) {
        let Some(mut editor) = self.stage_editor.take() else {
            return;
        };
        let action = show_stage_editor(ctx, &mut editor, &mut self.prefs.custom_tags);
        if editor.custom_tags_changed {
            editor.custom_tags_changed = false;
            self.prefs.save();
        }
        match action {
            StageEditorAction::None => {
                if editor.open {
                    self.stage_editor = Some(editor);
                }
            }
            StageEditorAction::Cancel => {}
            StageEditorAction::Save => {
                let scene_id = editor.scene_id.clone();
                let stage = editor.draft.clone();
                let infos = editor.positions_info.clone();
                if let Some(scene) = self.ws.package.get_scene_mut(&scene_id) {
                    if let Some(existing) = scene.get_stage_mut(&stage.id) {
                        *existing = stage;
                    } else {
                        let idx = scene.stages.len();
                        graph::ensure_graph_node(scene, &stage.id, idx);
                        scene.stages.push(stage);
                    }
                    scene.positions = infos;
                    self.mark_dirty();
                    self.status = "Stage saved".into();
                }
            }
        }
    }
}

impl App for SceneBuilderApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_channels(ctx);

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.stage_editor.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else if self.ws.dirty {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pending_after_confirm = Some(PendingAction::Quit);
                self.show_close_confirm = true;
            }
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));

        if self.stage_editor.is_none() {
            use egui::{Key, KeyboardShortcut, Modifiers};
            const SAVE_AS: KeyboardShortcut =
                KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);
            const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
            const NEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
            const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
            const EXPORT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::B);
            if ctx.input_mut(|i| i.consume_shortcut(&SAVE_AS)) {
                self.save_project(true);
            } else if ctx.input_mut(|i| i.consume_shortcut(&SAVE)) {
                self.save_project(false);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&NEW)) {
                self.request_if_clean(PendingAction::New);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&OPEN)) {
                self.request_if_clean(PendingAction::Open);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&EXPORT)) {
                self.request_export(ExportKind::Both);
            }
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            self.menu_bar(ui, ctx);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(self.ws.document_status());
                });
            });
        });

        let left_w = self.prefs.left_panel_width;
        let dark = self.prefs.theme.is_dark();
        let panel_stroke = egui::Stroke::new(1.0, crate::ui_base::border(dark));
        egui::SidePanel::left("left")
            .resizable(true)
            .default_width(left_w)
            .width_range(ui_base::LEFT_PANEL_MIN..=ui_base::LEFT_PANEL_MAX)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(crate::ui_base::panel_bg(dark))
                    .stroke(panel_stroke)
                    .inner_margin(egui::Margin::same(ui_base::PANEL_MARGIN)),
            )
            .show(ctx, |ui| {
                ui_base::constrain_panel_contents(ui);
                self.left_panel(ui);
                ui_base::claim_allocated_width(ui);
                let new_w = ui
                    .max_rect()
                    .width()
                    .clamp(ui_base::LEFT_PANEL_MIN, ui_base::LEFT_PANEL_MAX);
                if (new_w - self.prefs.left_panel_width).abs() > 1.0 {
                    self.prefs.left_panel_width = new_w;
                    self.prefs.save();
                }
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .fill(crate::ui_base::shell_bg(dark))
                    .inner_margin(egui::Margin::same(8)),
            )
            .show(ctx, |ui| {
                if let Some(scene_id) = self.ws.selected_scene.clone() {
                    let measured = ui
                        .ctx()
                        .data(|d| d.get_temp::<f32>(egui::Id::new("scene_positions_needed_h")));
                    let (min_h, default_h, max_h) = crate::positions::panel_height_range(
                        ui.available_height(),
                        self.prefs.bottom_panel_height,
                        measured.unwrap_or(ui_base::POSITIONS_PANEL_FALLBACK_H),
                    );
                    egui::TopBottomPanel::bottom("scene_positions_panel")
                        .resizable(true)
                        .default_height(default_h)
                        .height_range(min_h..=max_h)
                        .frame(
                            egui::Frame::side_top_panel(&ctx.style())
                                .fill(crate::ui_base::panel_bg(dark))
                                .stroke(panel_stroke)
                                .inner_margin(egui::Margin::same(ui_base::PANEL_MARGIN)),
                        )
                        .show_inside(ui, |ui| {
                            let (changed, inner_h) =
                                if let Some(scene) = self.ws.package.get_scene_mut(&scene_id) {
                                    crate::positions::show(ui, scene, &self.race_keys)
                                } else {
                                    (false, ui_base::POSITIONS_HEADER_H)
                                };
                            if changed {
                                self.mark_dirty();
                            }
                            let needed = inner_h + ui_base::POSITIONS_PANEL_CHROME;
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new("scene_positions_needed_h"), needed);
                            });
                            let new_h = ui.max_rect().height();
                            if new_h >= min_h
                                && (new_h - self.prefs.bottom_panel_height).abs() > 1.0
                            {
                                self.prefs.bottom_panel_height = new_h.max(min_h);
                                self.prefs.save();
                            }
                        });

                    let panel_id = egui::Id::new("tags_furniture_panel");
                    let resize_id = panel_id.with("__resize");
                    let avail = ui.available_rect_before_wrap();
                    let max_w =
                        ui_base::RIGHT_PANEL_MAX.min(avail.width().max(ui_base::RIGHT_PANEL_MIN));
                    let dragging = ui
                        .ctx()
                        .read_response(resize_id)
                        .is_some_and(|r| r.dragged());
                    let mut width = self
                        .prefs
                        .right_panel_width
                        .clamp(ui_base::RIGHT_PANEL_MIN, max_w);
                    if dragging {
                        if let Some(pointer) = ui
                            .ctx()
                            .read_response(resize_id)
                            .and_then(|r| r.interact_pointer_pos())
                        {
                            width = (avail.max.x - pointer.x)
                                .abs()
                                .clamp(ui_base::RIGHT_PANEL_MIN, max_w);
                        }
                    }

                    let mut right = egui::SidePanel::right(panel_id)
                        .resizable(true)
                        .default_width(width)
                        .frame(
                            egui::Frame::side_top_panel(&ctx.style())
                                .fill(crate::ui_base::panel_bg(dark))
                                .stroke(panel_stroke)
                                .inner_margin(egui::Margin::same(ui_base::PANEL_MARGIN)),
                        );
                    right = if dragging {
                        right.width_range(ui_base::RIGHT_PANEL_MIN..=ui_base::RIGHT_PANEL_MAX)
                    } else {
                        right.exact_width(width)
                    };
                    right.show_inside(ui, |ui| {
                        ui_base::constrain_panel_contents(ui);
                        crate::ui_define::fill_width(ui);
                        let allocated = ui.max_rect();
                        self.tags_furniture_panel(ui);
                        ui.expand_to_include_rect(allocated);
                    });

                    let mut panel_rect = avail;
                    panel_rect.min.x = panel_rect.max.x - width;
                    ui_base::persist_side_panel_rect(ui.ctx(), panel_id, panel_rect);
                    if (width - self.prefs.right_panel_width).abs() > 1.0 {
                        self.prefs.right_panel_width = width;
                        self.prefs.save();
                    }
                }

                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(4)))
                    .show_inside(ui, |ui| {
                        self.center_panel(ui);
                    });
            });

        self.handle_stage_editor(ctx);
        self.modals(ctx);
        self.toasts.ui(ctx);

        if self.prefs.capture_viewport(ctx) {
            self.prefs.save();
        }
    }
}
