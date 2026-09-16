use super::{even_columns, StageEditorState, MAX_POSITIONS};
use egui::{Color32, RichText, Stroke, TextEdit};
use scene_builder_core::project::define::Stripping;
use scene_builder_core::project::position::Position;
use scene_builder_core::project::position_info::PositionInfo;

pub(super) fn positions_section(ui: &mut egui::Ui, state: &mut StageEditorState) {
    crate::ui_define::fill_width(ui);
    state.sync_lengths();
    let n = state.draft.positions.len();

    let mut close_tab: Option<usize> = None;
    ui.horizontal(|ui| {
        for i in 0..n {
            let active = state.active_tab == i;
            let stroke_color = if active {
                crate::ui_base::accent(ui.visuals().dark_mode)
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke.color
            };
            egui::Frame::group(ui.style())
                .stroke(egui::Stroke::new(1.0, stroke_color))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if ui
                        .selectable_label(active, format!("Position {}", i + 1))
                        .clicked()
                    {
                        state.active_tab = i;
                    }
                    if n > 1
                        && ui
                            .add(
                                egui::Button::new(RichText::new("✕").size(11.0))
                                    .frame(false)
                                    .small(),
                            )
                            .on_hover_text("Remove position")
                            .clicked()
                    {
                        close_tab = Some(i);
                    }
                });
        }
        if n < MAX_POSITIONS && ui.button("+").on_hover_text("Add position").clicked() {
            state.draft.positions.push(Position::new(None));
            state.positions_info.push(PositionInfo::default());
            state.basic_anim.push(true);
            state.active_tab = state.draft.positions.len() - 1;
        }
    });
    if let Some(idx) = close_tab {
        state.draft.positions.remove(idx);
        state.positions_info.remove(idx);
        state.basic_anim.remove(idx);
        state.active_tab = state
            .active_tab
            .min(state.draft.positions.len().saturating_sub(1));
    }

    state.sync_lengths();
    let tab = state
        .active_tab
        .min(state.draft.positions.len().saturating_sub(1));
    state.active_tab = tab;

    let race_keys = state.race_keys.clone();
    egui::Frame::new()
        .fill(if ui.visuals().dark_mode {
            Color32::from_rgba_unmultiplied(255, 255, 255, 6)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 4)
        })
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            crate::ui_define::fill_width(ui);
            let race_filter = &mut state.race_filter;
            let new_pos_tag = &mut state.new_pos_tag;
            let basic = &mut state.basic_anim[tab];
            let pos = &mut state.draft.positions[tab];
            let info = &mut state.positions_info[tab];
            position_form(ui, pos, info, basic, &race_keys, race_filter, new_pos_tag);
        });
}

fn position_form(
    ui: &mut egui::Ui,
    pos: &mut Position,
    info: &mut PositionInfo,
    basic_anim: &mut bool,
    race_keys: &[String],
    race_filter: &mut String,
    new_pos_tag: &mut String,
) {
    crate::ui_define::fill_width(ui);

    even_columns(ui, 3, |i, ui| match i {
        0 => {
            ui.label("Race");
            ui.add(
                TextEdit::singleline(race_filter)
                    .hint_text("Filter…")
                    .desired_width(f32::INFINITY),
            );
            let filter = race_filter.to_lowercase();
            let display = if info.race.is_empty() {
                "Human"
            } else {
                info.race.as_str()
            };
            let prev_race = info.race.clone();
            egui::ComboBox::from_id_salt(("race", ui.id()))
                .selected_text(display)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for key in race_keys {
                        if !filter.is_empty() && !key.to_lowercase().contains(&filter) {
                            continue;
                        }
                        ui.selectable_value(&mut info.race, key.clone(), key);
                    }
                });
            if info.race != prev_race && info.race != "Human" {
                info.sex.futa = false;
            }
            if info.race.is_empty() {
                info.race = "Human".into();
            }
        }
        1 => {
            ui.label("Sex");
            let futa_enabled = info.race == "Human";
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                crate::ui_define::sex_flags(ui, &mut info.sex, futa_enabled);
            });
        }
        _ => {
            ui.label("SOS Angle");
            let mut schlong = pos.schlong as i32;
            if crate::ui_define::labeled_drag(
                ui,
                "SOS",
                egui::DragValue::new(&mut schlong).range(-9..=9).speed(1.0),
            )
            .changed()
            {
                pos.schlong = schlong.clamp(-9, 9) as i8;
            }
        }
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        if ui
            .checkbox(
                basic_anim,
                if *basic_anim {
                    "Animation (Basic)"
                } else {
                    "Animation (Sequence)"
                },
            )
            .changed()
            && *basic_anim
        {
            if let Some(first) = pos.event.first().cloned() {
                pos.event = vec![first];
            }
        }
    });

    ensure_event0(pos);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(".hkx");
        ui.add(
            TextEdit::singleline(&mut pos.event[0])
                .hint_text("Behavior file")
                .desired_width(ui.available_width()),
        );
    });

    if !*basic_anim {
        let mut remove_at: Option<usize> = None;
        for i in 1..pos.event.len() {
            ui.horizontal(|ui| {
                ui.label("+");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").clicked() {
                        remove_at = Some(i);
                    }
                    ui.label(".hkx");
                    ui.add(
                        TextEdit::singleline(&mut pos.event[i]).desired_width(ui.available_width()),
                    );
                });
            });
        }
        if let Some(i) = remove_at {
            pos.event.remove(i);
        }
        if ui.button("Add event").clicked() {
            pos.event.push(String::new());
        }
    }

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    ui.label("Anim Object");
    ui.add(
        TextEdit::singleline(&mut pos.anim_obj)
            .hint_text("Editor ID(s), comma/space separated")
            .desired_width(f32::INFINITY),
    );

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    even_columns(ui, 4, |i, ui| match i {
        0 => {
            ui.label("Data");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                crate::ui_define::state_flags(
                    ui,
                    &mut info.submissive,
                    &mut info.vampire,
                    &mut info.dead,
                    info.race == "Human",
                    true,
                );
            });
            ui.checkbox(&mut pos.climax, "Climax");
            ui.label("Tags");
            let mut remove_tag: Option<usize> = None;
            for (i, tag) in pos.tags.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(tag);
                    if ui.small_button("✕").clicked() {
                        remove_tag = Some(i);
                    }
                });
            }
            if let Some(i) = remove_tag {
                pos.tags.remove(i);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let add_clicked = ui.small_button("+").clicked();
                ui.add(
                    TextEdit::singleline(new_pos_tag)
                        .hint_text("tag")
                        .desired_width(ui.available_width().max(40.0))
                        .id_salt(("pos_tag", ui.id())),
                );
                if add_clicked {
                    let t = new_pos_tag.trim().to_string();
                    if !t.is_empty() && !pos.tags.iter().any(|e| e.eq_ignore_ascii_case(&t)) {
                        pos.tags.push(t);
                        new_pos_tag.clear();
                    }
                }
            });
        }
        1 => {
            ui.label("Offset");
            for (label, val, clamp) in [
                ("X", &mut pos.offset.x, None),
                ("Y", &mut pos.offset.y, None),
                ("Z", &mut pos.offset.z, None),
                ("R", &mut pos.offset.r, Some(0.0..=359.9_f32)),
            ] {
                let mut drag = egui::DragValue::new(val).speed(0.1).min_decimals(1);
                if let Some(range) = clamp {
                    drag = drag.range(range);
                }
                crate::ui_define::labeled_drag(ui, label, drag);
            }
        }
        2 => {
            ui.label("Scale");
            let h = ui.spacing().interact_size.y;
            let w = ui.available_width();
            ui.add_sized(
                [w, h],
                egui::DragValue::new(&mut info.scale)
                    .speed(0.01)
                    .range(0.01..=2.0)
                    .min_decimals(2),
            );
        }
        _ => {
            ui.label("Stripping");
            stripping_ui(ui, &mut pos.strip_data);
        }
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    ui.label("SLAL compatibility");
    ui.horizontal(|ui| {
        ui.checkbox(&mut pos.open_mouth, "Open Mouth");
        ui.checkbox(&mut pos.silent, "Silent");
        ui.checkbox(&mut pos.strap_on, "Strap-on");
    });
}

fn ensure_event0(pos: &mut Position) {
    if pos.event.is_empty() {
        pos.event.push(String::new());
    }
}

fn stripping_ui(ui: &mut egui::Ui, s: &mut Stripping) {
    if ui.checkbox(&mut s.default, "Default").changed() && s.default {
        s.everything = false;
        s.nothing = false;
        s.helmet = false;
        s.gloves = false;
        s.boots = false;
    }
    if ui.checkbox(&mut s.everything, "Everything").changed() && s.everything {
        s.default = false;
        s.nothing = false;
        s.helmet = false;
        s.gloves = false;
        s.boots = false;
    }
    if ui.checkbox(&mut s.nothing, "Nothing").changed() && s.nothing {
        s.default = false;
        s.everything = false;
        s.helmet = false;
        s.gloves = false;
        s.boots = false;
    }
    ui.horizontal_wrapped(|ui| {
        let mut any = false;
        any |= ui.checkbox(&mut s.helmet, "Helmet").changed();
        any |= ui.checkbox(&mut s.gloves, "Gloves").changed();
        any |= ui.checkbox(&mut s.boots, "Boots").changed();
        if any && (s.helmet || s.gloves || s.boots) {
            s.default = false;
            s.everything = false;
            s.nothing = false;
        }
    });
}
