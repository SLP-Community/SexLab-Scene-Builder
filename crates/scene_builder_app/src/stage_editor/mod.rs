use egui::{Color32, Key, Modal, RichText, Stroke, TextEdit};
use scene_builder_core::project::position::Position;
use scene_builder_core::project::position_info::PositionInfo;
use scene_builder_core::project::stage::Stage;
use scene_builder_core::project::NanoID;
use scene_builder_core::racekeys::get_race_keys_string;

use crate::tags::SLAL_SOUNDS;
use crate::tags::{tag_tree_ui, TagTreeState};

mod positions;
use positions::positions_section;

pub(super) const MAX_POSITIONS: usize = 5;

#[derive(Debug, Clone)]
pub struct StageEditorState {
    pub scene_id: NanoID,
    pub draft: Stage,
    pub positions_info: Vec<PositionInfo>,
    pub open: bool,
    pub race_keys: Vec<String>,
    pub active_tab: usize,
    pub error: Option<String>,
    /// Per-position Basic (`true`) vs Sequence (`false`) animation mode.
    pub basic_anim: Vec<bool>,
    pub new_pos_tag: String,
    pub race_filter: String,
    pub tag_state: TagTreeState,
    /// Set when the persisted "Yours" tag list was modified (app saves prefs).
    pub custom_tags_changed: bool,
}

impl StageEditorState {
    pub fn new(scene_id: NanoID, stage: Stage, positions_info: Vec<PositionInfo>) -> Self {
        let mut race_keys = get_race_keys_string();
        race_keys.sort();
        let n = stage.positions.len().max(positions_info.len()).max(1);
        let mut infos = positions_info;
        while infos.len() < n {
            infos.push(PositionInfo::default());
        }
        infos.truncate(n);
        let mut draft = stage;
        while draft.positions.len() < n {
            draft.positions.push(Position::new(None));
        }
        draft.positions.truncate(n);

        let basic_anim: Vec<bool> = draft.positions.iter().map(|p| p.event.len() <= 1).collect();

        Self {
            scene_id,
            draft,
            positions_info: infos,
            open: true,
            race_keys,
            active_tab: 0,
            error: None,
            basic_anim,
            new_pos_tag: String::new(),
            race_filter: String::new(),
            tag_state: TagTreeState::default(),
            custom_tags_changed: false,
        }
    }

    fn sync_lengths(&mut self) {
        let n = self
            .draft
            .positions
            .len()
            .max(self.positions_info.len())
            .max(1);
        while self.draft.positions.len() < n {
            self.draft.positions.push(Position::new(None));
        }
        while self.positions_info.len() < n {
            self.positions_info.push(PositionInfo::default());
        }
        while self.basic_anim.len() < n {
            self.basic_anim.push(true);
        }
        self.draft.positions.truncate(n);
        self.positions_info.truncate(n);
        self.basic_anim.truncate(n);
        if self.active_tab >= n {
            self.active_tab = n.saturating_sub(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageEditorAction {
    None,
    Save,
    Cancel,
}

pub fn show_stage_editor(
    ctx: &egui::Context,
    state: &mut StageEditorState,
    custom_tags: &mut Vec<String>,
) -> StageEditorAction {
    if !state.open {
        return StageEditorAction::None;
    }

    let screen = ctx.screen_rect();
    let margin = 32.0;
    let max_w = (screen.width() - margin * 2.0).max(400.0);
    let max_h = (screen.height() - margin * 2.0).max(320.0);
    let target_w = (screen.width() * 0.72).clamp(640.0_f32.min(max_w), max_w);
    let target_h = (screen.height() * 0.88).clamp(480.0_f32.min(max_h), max_h);

    // Pin Area size every frame so a prior tag-tree overflow can't leave a giant modal.
    let modal_id = egui::Id::new(("stage_editor_modal", state.draft.id.0.as_str()));
    let modal = Modal::new(modal_id)
        .backdrop_color(egui::Color32::from_black_alpha(110))
        .frame(
            egui::Frame::popup(&ctx.style())
                .inner_margin(egui::Margin::same(12))
                .corner_radius(8.0),
        );

    let response = modal.show(ctx, |ui| {
        ui.set_width(target_w);
        ui.set_min_width(target_w);
        ui.set_max_width(target_w);
        ui.set_max_height(target_h);
        editor_form(ui, state, custom_tags)
    });

    // Backdrop click does not cancel — require Save / Cancel / Esc (matches old deliberate close).
    response.inner
}

fn try_validate_and_save(state: &mut StageEditorState) -> StageEditorAction {
    for (i, pos) in state.draft.positions.iter().enumerate() {
        let event0 = pos.event.first().map(|s| s.trim()).unwrap_or("");
        if event0.is_empty() {
            state.error = Some(format!(
                "Position {} is missing its behavior file (.hkx)",
                i + 1
            ));
            return StageEditorAction::None;
        }
    }
    for (i, info) in state.positions_info.iter().enumerate() {
        if !info.sex.male && !info.sex.female && !info.sex.futa {
            state.error = Some(format!(
                "Position {} has no sex assigned. Every position needs at least one sex.",
                i + 1
            ));
            return StageEditorAction::None;
        }
    }
    for pos in &mut state.draft.positions {
        pos.anim_obj = normalize_anim_obj(&pos.anim_obj);
    }
    state.error = None;
    StageEditorAction::Save
}

fn normalize_anim_obj(raw: &str) -> String {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn editor_form(
    ui: &mut egui::Ui,
    state: &mut StageEditorState,
    custom_tags: &mut Vec<String>,
) -> StageEditorAction {
    let mut action = StageEditorAction::None;
    let mut multiline_focused = false;

    if ui.input(|i| i.key_pressed(Key::Escape)) {
        return StageEditorAction::Cancel;
    }

    // Ctrl/Cmd+Enter always saves; plain Enter saves unless a multiline has focus.
    let ctrl_enter =
        ui.input(|i| i.key_pressed(Key::Enter) && (i.modifiers.command || i.modifiers.ctrl));
    let plain_enter = ui.input(|i| {
        i.key_pressed(Key::Enter)
            && !i.modifiers.shift
            && !i.modifiers.command
            && !i.modifiers.ctrl
            && !i.modifiers.alt
    });

    // Name | Save/Cancel in separate strips (same crowding bug as graph header).
    let full = ui.available_rect_before_wrap();
    let row_h = ui.spacing().interact_size.y.max(28.0);
    let right_w = 150.0;
    let left_w = (full.width() - right_w).max(120.0);
    let left_rect = egui::Rect::from_min_size(full.min, egui::vec2(left_w, row_h));
    let right_rect = egui::Rect::from_min_size(
        egui::pos2(left_rect.max.x, full.min.y),
        egui::vec2(full.width() - left_w, row_h),
    );

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(left_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_clip_rect(left_rect);
            let name_edit = TextEdit::singleline(&mut state.draft.name)
                .frame(false)
                .font(egui::TextStyle::Heading)
                .hint_text("Stage Name")
                .id_salt(("stage_name", state.draft.id.0.as_str()))
                .desired_width((ui.available_width() - 8.0).max(60.0));
            let output = name_edit.show(ui);
            if output.response.gained_focus() && output.response.clicked() {
                if let Some(mut text_state) = TextEdit::load_state(ui.ctx(), output.response.id) {
                    let range = egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(state.draft.name.chars().count()),
                    );
                    text_state.cursor.set_char_range(Some(range));
                    text_state.store(ui.ctx(), output.response.id);
                }
            }
        },
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        |ui| {
            ui.set_clip_rect(right_rect);
            if ui.button("Cancel").clicked() {
                action = StageEditorAction::Cancel;
            }
            let accent = crate::ui_base::accent(ui.visuals().dark_mode);
            if ui
                .add(
                    egui::Button::new(RichText::new("Save").color(egui::Color32::WHITE))
                        .fill(accent),
                )
                .clicked()
            {
                action = try_validate_and_save(state);
            }
        },
    );
    let _ = ui.allocate_rect(
        egui::Rect::from_min_size(full.min, egui::vec2(full.width(), row_h)),
        egui::Sense::hover(),
    );

    if let Some(err) = &state.error {
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), err);
    }

    ui.separator();

    let body_w = ui.available_width().max(1.0);
    egui::ScrollArea::vertical()
        .id_salt("stage_editor_scroll")
        .auto_shrink([false, true])
        .hscroll(false)
        .max_width(body_w)
        .show(ui, |ui| {
            ui.set_width(body_w);
            ui.set_max_width(body_w);
            let dark = ui.visuals().dark_mode;
        let (tags_accent, tags_fill) = if dark {
            (
                Color32::from_rgb(0x5a, 0x9e, 0x9a),
                Color32::from_rgba_unmultiplied(0x13, 0xc2, 0xc2, 22),
            )
        } else {
            (
                Color32::from_rgb(0x00, 0x6d, 0x75),
                Color32::from_rgb(0xe6, 0xff, 0xfb),
            )
        };
        let (pos_accent, pos_fill) = if dark {
            (
                Color32::from_rgb(0xc9, 0x96, 0x3c),
                Color32::from_rgba_unmultiplied(0xfa, 0xad, 0x14, 22),
            )
        } else {
            (
                Color32::from_rgb(0xad, 0x4e, 0x00),
                Color32::from_rgb(0xff, 0xf7, 0xe6),
            )
        };
        let (extra_accent, extra_fill) = if dark {
            (
                Color32::from_rgb(0x9a, 0x7a, 0xb8),
                Color32::from_rgba_unmultiplied(0x72, 0x2e, 0xd1, 22),
            )
        } else {
            (
                Color32::from_rgb(0x39, 0x10, 0x85),
                Color32::from_rgb(0xf9, 0xf0, 0xff),
            )
        };

        section_card(ui, "Tags", tags_accent, tags_fill, |ui| {
            let result = tag_tree_ui(
                ui,
                "stage_tags",
                &mut state.tag_state,
                &mut state.draft.tags,
                custom_tags,
            );
            if result.custom_changed {
                state.custom_tags_changed = true;
            }
            // Leave room so the Custom frame stroke isn't clipped by the card.
            ui.add_space(4.0);
        });

        ui.add_space(8.0);
        section_card(ui, "Positions", pos_accent, pos_fill, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width().max(1.0), 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    positions_section(ui, state);
                },
            );
        });

        ui.add_space(8.0);
        section_card(ui, "Extra", extra_accent, extra_fill, |ui| {
            even_columns(ui, 3, |i, ui| match i {
                0 => {
                    ui.horizontal(|ui| {
                        ui.label("Navigation");
                        crate::ui_define::info_tip(
                            ui,
                            "In-game label when the player picks among several next stages. Unused on a linear path.",
                        );
                    });
                    let nav = TextEdit::multiline(&mut state.draft.extra.nav_text)
                        .desired_rows(3)
                        .desired_width(ui.available_width())
                        .char_limit(100);
                    let nav_resp = ui.add(nav);
                    if nav_resp.has_focus() {
                        multiline_focused = true;
                    }
                    ui.label(
                        RichText::new(format!(
                            "{}/100",
                            state.draft.extra.nav_text.chars().count()
                        ))
                        .weak()
                        .small(),
                    );
                }
                1 => {
                    ui.horizontal(|ui| {
                        ui.label("Fixed Duration");
                        crate::ui_define::info_tip(
                            ui,
                            "Duration of an animation that should only play once (does not loop).",
                        );
                    });
                    let h = ui.spacing().interact_size.y;
                    let w = ui.available_width().max(40.0);
                    ui.add_sized(
                        [w, h],
                        egui::DragValue::new(&mut state.draft.extra.fixed_len)
                            .speed(10.0)
                            .range(0.0..=f32::MAX)
                            .max_decimals(0)
                            .suffix(" ms"),
                    );
                }
                _ => {
                    ui.horizontal(|ui| {
                        ui.label("Sound (SLAL only)");
                        crate::ui_define::info_tip(
                            ui,
                            "Classic SLAL sound category for this stage (not used by SLSB/.slr). First non-empty stage sets the animation default; differing stages become per-stage overrides.",
                        );
                    });
                    let current = if state.draft.extra.sound.is_empty() {
                        "Unset"
                    } else {
                        state.draft.extra.sound.as_str()
                    };
                    egui::ComboBox::from_id_salt("slal_sound")
                        .width(ui.available_width().max(40.0))
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(state.draft.extra.sound.is_empty(), "Unset")
                                .clicked()
                            {
                                state.draft.extra.sound.clear();
                            }
                            for s in SLAL_SOUNDS.iter().filter(|s| !s.is_empty()) {
                                ui.selectable_value(
                                    &mut state.draft.extra.sound,
                                    (*s).to_string(),
                                    *s,
                                );
                            }
                        });
                }
            });
        });
    });

    if matches!(action, StageEditorAction::None) {
        if ctrl_enter || (plain_enter && !multiline_focused) {
            action = try_validate_and_save(state);
        }
    }

    action
}

/// Soft hue-wash card; title uses a muted accent, body keeps theme text colors.
fn section_card(
    ui: &mut egui::Ui,
    title: &str,
    accent: Color32,
    fill: Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let w = ui.available_width().max(1.0);
    ui.set_max_width(w);
    let stroke = Stroke::new(
        1.0,
        accent.gamma_multiply(if ui.visuals().dark_mode { 0.55 } else { 0.35 }),
    );
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_max_width((w - 20.0).max(1.0));
            egui::CollapsingHeader::new(RichText::new(title).strong().color(accent))
                .id_salt(("section", title))
                .default_open(true)
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width());
                    add_contents(ui);
                });
        });
}

pub(super) fn even_columns(ui: &mut egui::Ui, n: usize, mut add: impl FnMut(usize, &mut egui::Ui)) {
    if n == 0 {
        return;
    }
    let gap = 8.0;
    let n_f = n as f32;
    let total_w = ui.available_width();
    let col_w = ((total_w - gap * (n_f - 1.0)) / n_f).max(80.0);
    // Height 0 — do not use horizontal_top; that allocates available_height
    // (the modal viewport) and stretches the Positions card.
    ui.allocate_ui_with_layout(
        egui::vec2(total_w.max(1.0), 0.0),
        egui::Layout::left_to_right(egui::Align::Min),
        |ui| {
            ui.set_max_width(total_w);
            ui.spacing_mut().item_spacing.x = gap;
            for i in 0..n {
                ui.allocate_ui_with_layout(
                    egui::vec2(col_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(col_w);
                        ui.set_max_width(col_w);
                        add(i, ui);
                    },
                );
            }
        },
    );
}
