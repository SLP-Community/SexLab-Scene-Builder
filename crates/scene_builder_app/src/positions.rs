//! Bottom strip: actor slots shared by every stage in the scene.

use crate::ui_base;
use egui::{Color32, RichText};
use scene_builder_core::project::position::Position;
use scene_builder_core::project::position_info::PositionInfo;
use scene_builder_core::project::scene::Scene;

const MAX_POSITIONS: usize = 5;

/// Returns `(dirty, inner_content_height)` so the panel can size to the cards.
pub fn show(ui: &mut egui::Ui, scene: &mut Scene, race_keys: &[String]) -> (bool, f32) {
    ensure_scene_positions(scene);

    let top = ui.next_widget_position().y;
    crate::ui_define::fill_width(ui);
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Scene Positions").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            crate::ui_define::info_tip(ui, "Position data shared between all stages in the scene.");
            let at_cap = scene.positions.len() >= MAX_POSITIONS;
            let add = ui
                .add_enabled(!at_cap, egui::Button::new("Add Position").small())
                .on_hover_text(if at_cap {
                    "Maximum of 5 positions."
                } else {
                    "Add an actor slot to this scene and every stage."
                });
            if add.clicked() {
                changed |= add_scene_position(scene);
            }
        });
    });

    if scene.positions.is_empty() {
        ui.label(RichText::new("No positions yet — use Add Position above.").weak());
        let inner_h = (ui.next_widget_position().y - top).max(1.0);
        return (changed, inner_h);
    }

    let budget = ui.available_width().floor().max(0.0);
    ui.set_max_width(budget);

    let n = scene.positions.len().clamp(1, MAX_POSITIONS);
    let gap = if n <= 2 {
        12.0
    } else if n <= 3 {
        10.0
    } else {
        ui_base::SPACE
    };
    // Own the gaps; default item_spacing would stack on top and shove the last card out.
    let card_w = ((budget - gap * n.saturating_sub(1) as f32) / n as f32)
        .floor()
        .max(1.0);

    let roomy = card_w >= 240.0;
    let pad = if card_w >= 280.0 {
        10.0
    } else if card_w >= 200.0 {
        8.0
    } else {
        6.0
    };

    let mut remove_at: Option<usize> = None;
    ui.allocate_ui_with_layout(
        egui::vec2(budget.max(1.0), 0.0),
        egui::Layout::left_to_right(egui::Align::Min),
        |ui| {
            ui.set_max_width(budget);
            ui.spacing_mut().item_spacing.x = 0.0;
            for idx in 0..n {
                ui.allocate_ui_with_layout(
                    egui::vec2(card_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_max_width(card_w);
                        egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::same(pad as i8))
                            .show(ui, |ui| {
                                ui.set_max_width(ui.available_width());
                                let info = &mut scene.positions[idx];

                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("Position {}", idx + 1))
                                            .small()
                                            .weak(),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let icon = RichText::new("✕")
                                                .strong()
                                                .color(Color32::from_rgb(0xE7, 0x4C, 0x3C));
                                            if ui
                                                .add(egui::Button::new(icon).small().frame(false))
                                                .on_hover_text("Remove this position")
                                                .clicked()
                                            {
                                                remove_at = Some(idx);
                                            }
                                        },
                                    );
                                });

                                let is_human = info.race == "Human";
                                let combo_w = ui.available_width().max(24.0);

                                egui::ComboBox::from_id_salt(("position_race", idx))
                                    .width(combo_w)
                                    .selected_text(info.race.clone())
                                    .show_ui(ui, |ui| {
                                        for key in race_keys {
                                            if ui.selectable_label(&info.race == key, key).clicked()
                                            {
                                                info.race = key.clone();
                                                if key != "Human" {
                                                    info.sex.futa = false;
                                                    info.vampire = false;
                                                }
                                                changed = true;
                                            }
                                        }
                                    });

                                ui.add_space(ui_base::SPACE_SM);
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                                    changed |= crate::ui_define::sex_flags(ui, &mut info.sex, is_human);
                                });

                                ui.add_space(ui_base::SPACE_XS);
                                ui.separator();
                                ui.add_space(ui_base::SPACE_XS);

                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                                    changed |= crate::ui_define::state_flags(
                                        ui,
                                        &mut info.submissive,
                                        &mut info.vampire,
                                        &mut info.dead,
                                        is_human,
                                        roomy,
                                    );
                                });

                                ui.add_space(ui_base::SPACE_SM);
                                ui.horizontal(|ui| {
                                    ui.label("Scale").on_hover_text(
                                        "Actor scale factor used by SexLab for this position (typically 1.0).",
                                    );
                                    let h = ui.spacing().interact_size.y;
                                    let w = ui.available_width().max(24.0);
                                    changed |= ui
                                        .add_sized(
                                            [w, h],
                                            egui::DragValue::new(&mut info.scale)
                                                .speed(0.01)
                                                .range(0.01..=2.0)
                                                .fixed_decimals(2),
                                        )
                                        .changed();
                                });
                            });
                    },
                );
                if idx + 1 < n {
                    ui.add_space(gap);
                }
            }
        },
    );

    if let Some(idx) = remove_at {
        changed |= remove_scene_position(scene, idx);
    }

    let inner_h = (ui.next_widget_position().y - top).max(1.0);
    (changed, inner_h)
}

/// Height range for the Scene Positions strip.
///
/// `min` hugs measured content so cards are never clipped. `default` restores the
/// last user-resized height (clamped into range).
pub fn panel_height_range(avail_h: f32, prefs_h: f32, needed: f32) -> (f32, f32, f32) {
    let abs_max = ui_base::POSITIONS_PANEL_ABS_MAX.min(avail_h * ui_base::POSITIONS_PANEL_MAX_FRAC);
    let max_h = abs_max.max(80.0);
    let floor = ui_base::POSITIONS_HEADER_H + ui_base::POSITIONS_PANEL_CHROME;
    let min_h = needed.clamp(floor, max_h);
    let default_h = prefs_h.clamp(min_h, max_h);
    (min_h, default_h, max_h)
}

/// Older / imported packs may only store actors on stages.
fn ensure_scene_positions(scene: &mut Scene) {
    if !scene.positions.is_empty() {
        return;
    }
    let Some(stage) = scene.stages.first() else {
        return;
    };
    if stage.positions.is_empty() {
        return;
    }
    scene.positions = stage
        .positions
        .iter()
        .map(|p| p.extract_position_info())
        .collect();
}

fn add_scene_position(scene: &mut Scene) -> bool {
    if scene.positions.len() >= MAX_POSITIONS {
        return false;
    }
    scene.positions.push(PositionInfo::default());
    for stage in &mut scene.stages {
        stage.positions.push(Position::new(None));
    }
    true
}

/// Pad or truncate the scene (and every stage) to `n` actor slots.
pub fn adopt_scene_position_count(scene: &mut Scene, n: usize) {
    let n = n.clamp(1, MAX_POSITIONS);
    while scene.positions.len() < n {
        scene.positions.push(PositionInfo::default());
    }
    if scene.positions.len() > n {
        scene.positions.truncate(n);
    }
    let slots = scene.positions.clone();
    for stage in &mut scene.stages {
        fit_stage_positions(stage, &slots);
    }
}

fn fit_stage_positions(
    stage: &mut scene_builder_core::project::stage::Stage,
    slots: &[PositionInfo],
) {
    let n = slots.len().max(1);
    if stage.positions.len() > n {
        stage.positions.truncate(n);
        return;
    }
    while stage.positions.len() < n {
        let i = stage.positions.len();
        let mut pos = Position::new(None);
        if let Some(info) = slots.get(i) {
            apply_position_info(&mut pos, info);
        }
        stage.positions.push(pos);
    }
}

fn apply_position_info(pos: &mut Position, info: &PositionInfo) {
    pos.sex = info.sex.clone();
    pos.race = info.race.clone();
    pos.scale = info.scale;
    pos.extra.submissive = info.submissive;
    pos.extra.vampire = info.vampire;
    pos.extra.dead = info.dead;
    pos.add_cum = info.add_cum;
}

fn remove_scene_position(scene: &mut Scene, idx: usize) -> bool {
    if idx >= scene.positions.len() {
        return false;
    }
    scene.positions.remove(idx);
    for stage in &mut scene.stages {
        if idx < stage.positions.len() {
            stage.positions.remove(idx);
        }
    }
    true
}
