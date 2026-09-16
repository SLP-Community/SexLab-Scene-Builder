use crate::ui_base::{accent, accent_hover};
use egui::Color32;

/// Compact "Info" control that shows `tip` as soon as the pointer is over it.
///
/// Uses [`egui::Response::show_tooltip_text`] (always-on path) instead of
/// `on_hover_text`, which stays suppressed for `tooltip_delay` after any
/// `ScrollArea` scroll — common in the stage editor and side panels.
pub fn info_tip(ui: &mut egui::Ui, tip: &str) {
    let resp = ui.add(
        egui::Button::new(egui::RichText::new("Info").small())
            .frame(true)
            .small()
            .min_size(egui::vec2(36.0, 18.0)),
    );
    if resp.contains_pointer() {
        resp.show_tooltip_text(tip);
    }
}

/// Pin the current UI to the parent's allocated width (avoids Frame/ScrollArea shrink-wrap).
pub fn fill_width(ui: &mut egui::Ui) {
    let w = ui.available_width();
    if w.is_finite() && w > 0.0 {
        ui.set_min_width(w);
        ui.set_max_width(w);
    }
}

/// Label + expanding numeric field on one row (fills remaining horizontal space).
pub fn labeled_drag(ui: &mut egui::Ui, label: &str, drag: egui::DragValue<'_>) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(label);
        let h = ui.spacing().interact_size.y;
        let w = ui.available_width().max(40.0);
        ui.add_sized([w, h], drag)
    })
    .inner
}

/// High-contrast radio-style chip (clear on/off vs white-on-grey native radios).
pub fn choice_chip(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    enabled: bool,
) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let acc = accent(dark);
    let (fill, text, ring, dot) = if !enabled {
        if dark {
            (
                Color32::from_rgb(0x22, 0x22, 0x22),
                Color32::from_gray(110),
                Color32::from_gray(70),
                Color32::from_gray(70),
            )
        } else {
            (
                Color32::from_gray(235),
                Color32::from_gray(140),
                Color32::from_gray(180),
                Color32::from_gray(180),
            )
        }
    } else if selected {
        (acc, Color32::WHITE, acc, Color32::WHITE)
    } else if dark {
        (
            Color32::from_rgb(0x32, 0x32, 0x32),
            Color32::from_gray(245),
            Color32::from_gray(175),
            Color32::TRANSPARENT,
        )
    } else {
        (
            Color32::WHITE,
            Color32::from_gray(25),
            Color32::from_gray(90),
            Color32::TRANSPARENT,
        )
    };

    let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
    let text_w = ui.fonts(|f| {
        f.layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    });
    let h = 24.0;
    let pad_x = 8.0;
    let radio_r = 5.5;
    let gap = 6.0;
    let w = pad_x + radio_r * 2.0 + gap + text_w + pad_x;
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(w, h), sense);
    if enabled {
        resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let stroke_w = if selected || resp.hovered() { 1.5 } else { 1.0 };
        painter.rect(
            rect,
            4.0,
            fill,
            egui::Stroke::new(
                stroke_w,
                if resp.hovered() && enabled {
                    accent_hover(dark)
                } else {
                    ring
                },
            ),
            egui::StrokeKind::Inside,
        );
        let c = egui::pos2(rect.left() + pad_x + radio_r, rect.center().y);
        painter.circle_stroke(c, radio_r, egui::Stroke::new(1.5, ring));
        if selected {
            painter.circle_filled(c, radio_r * 0.55, dot);
        }
        painter.text(
            egui::pos2(c.x + radio_r + gap, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            text,
        );
    }
    resp
}

/// Multi-select Male / Female / Futa chips — a position can allow more than one sex.
/// When `futa_enabled` is false, Futa is disabled and cleared.
pub fn sex_flags(
    ui: &mut egui::Ui,
    sex: &mut scene_builder_core::project::define::Sex,
    futa_enabled: bool,
) -> bool {
    let mut changed = false;
    if choice_chip(ui, sex.male, "Male", true).clicked() {
        sex.male = !sex.male;
        changed = true;
    }
    if choice_chip(ui, sex.female, "Female", true).clicked() {
        sex.female = !sex.female;
        changed = true;
    }
    if choice_chip(ui, sex.futa, "Futa", futa_enabled).clicked() && futa_enabled {
        sex.futa = !sex.futa;
        changed = true;
    }
    if !futa_enabled && sex.futa {
        sex.futa = false;
        changed = true;
    }
    changed
}

/// Multi-select actor state chips (same radio look as sex; flags combine freely).
pub fn state_flags(
    ui: &mut egui::Ui,
    submissive: &mut bool,
    vampire: &mut bool,
    dead: &mut bool,
    vampire_enabled: bool,
    roomy_labels: bool,
) -> bool {
    let mut changed = false;
    let sub_l = if roomy_labels { "Submissive" } else { "Sub" };
    let dead_l = if roomy_labels { "Unconscious" } else { "Uncon" };

    if choice_chip(ui, *submissive, sub_l, true)
        .on_hover_text("Passive / Taker / Bottom position.")
        .clicked()
    {
        *submissive = !*submissive;
        changed = true;
    }
    if choice_chip(ui, *vampire, "Vampire", vampire_enabled)
        .on_hover_text("Actor is a vampire.")
        .clicked()
        && vampire_enabled
    {
        *vampire = !*vampire;
        changed = true;
    }
    if !vampire_enabled && *vampire {
        *vampire = false;
        changed = true;
    }
    if choice_chip(ui, *dead, dead_l, true)
        .on_hover_text("Unconscious / dead.")
        .clicked()
    {
        *dead = !*dead;
        changed = true;
    }
    changed
}
