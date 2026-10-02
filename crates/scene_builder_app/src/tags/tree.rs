//! Tag editor: search/create plus framed SFW / NSFW / Custom groups.
//! Selected tags are highlighted in-place (no separate “selected” chip list).

use super::presets::{TAGS_NSFW, TAGS_SFW};
use crate::ui_base;
use egui::{Color32, RichText, Stroke, TextEdit};

#[derive(Debug, Clone)]
pub struct TagTreeState {
    pub search: String,
    /// (old name, draft) while renaming a saved custom tag.
    renaming: Option<(String, String)>,
    /// Request focus into the inline rename field once.
    rename_needs_focus: bool,
}

impl Default for TagTreeState {
    fn default() -> Self {
        Self {
            search: String::new(),
            renaming: None,
            rename_needs_focus: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TagTreeResult {
    pub tags_changed: bool,
    pub custom_changed: bool,
}

fn tag_key(tag: &str) -> String {
    tag.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn preset_canonical(tag: &str) -> Option<&'static str> {
    let key = tag_key(tag);
    TAGS_SFW
        .iter()
        .chain(TAGS_NSFW.iter())
        .find(|p| tag_key(p) == key)
        .copied()
}

#[derive(Clone, Copy)]
enum ChipColor {
    Cyan,
    Volcano,
    Purple,
}

impl ChipColor {
    fn accent(self, dark: bool) -> Color32 {
        match (self, dark) {
            (ChipColor::Cyan, false) => Color32::from_rgb(0x00, 0x6d, 0x75),
            (ChipColor::Cyan, true) => Color32::from_rgb(0x5a, 0x9e, 0x9a),
            (ChipColor::Volcano, false) => Color32::from_rgb(0xad, 0x21, 0x02),
            (ChipColor::Volcano, true) => Color32::from_rgb(0xc9, 0x7a, 0x5c),
            (ChipColor::Purple, false) => Color32::from_rgb(0x39, 0x10, 0x85),
            (ChipColor::Purple, true) => Color32::from_rgb(0x9a, 0x7a, 0xb8),
        }
    }

    fn wash(self, dark: bool) -> Color32 {
        match (self, dark) {
            (ChipColor::Cyan, false) => Color32::from_rgb(0xef, 0xf5, 0xf5),
            (ChipColor::Cyan, true) => Color32::from_rgb(0x18, 0x1f, 0x1f),
            (ChipColor::Volcano, false) => Color32::from_rgb(0xf7, 0xf2, 0xef),
            (ChipColor::Volcano, true) => Color32::from_rgb(0x20, 0x1a, 0x18),
            (ChipColor::Purple, false) => Color32::from_rgb(0xf3, 0xf0, 0xf6),
            (ChipColor::Purple, true) => Color32::from_rgb(0x1c, 0x1a, 0x22),
        }
    }

    fn selected_fill(self, dark: bool) -> Color32 {
        match (self, dark) {
            (ChipColor::Cyan, false) => Color32::from_rgb(0x08, 0x97, 0x9c),
            (ChipColor::Cyan, true) => Color32::from_rgb(0x1a, 0x8a, 0x86),
            (ChipColor::Volcano, false) => Color32::from_rgb(0xd4, 0x38, 0x0d),
            (ChipColor::Volcano, true) => Color32::from_rgb(0xb8, 0x4a, 0x2a),
            (ChipColor::Purple, false) => Color32::from_rgb(0x53, 0x1d, 0xab),
            (ChipColor::Purple, true) => Color32::from_rgb(0x6b, 0x45, 0x9a),
        }
    }
}

fn contains_key(list: &[String], tag: &str) -> bool {
    let key = tag_key(tag);
    list.iter().any(|t| tag_key(t) == key)
}

fn add_tag(tags: &mut Vec<String>, custom: &mut Vec<String>, raw: &str) -> TagTreeResult {
    let trimmed = raw.trim();
    let mut result = TagTreeResult::default();
    if trimmed.is_empty() {
        return result;
    }
    let canonical = preset_canonical(trimmed)
        .map(str::to_string)
        .unwrap_or_else(|| trimmed.to_string());
    if !contains_key(tags, &canonical) {
        tags.push(canonical.clone());
        result.tags_changed = true;
    }
    if preset_canonical(&canonical).is_none() && !contains_key(custom, &canonical) {
        custom.push(canonical);
        custom.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        result.custom_changed = true;
    }
    result
}

fn truncate_tag_label(ui: &egui::Ui, tag: &str, max_width: f32, font_id: &egui::FontId) -> String {
    let max_width = max_width.max(12.0);
    let measure = |s: &str| {
        ui.fonts(|f| {
            f.layout_no_wrap(s.to_owned(), font_id.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    if measure(tag) <= max_width {
        return tag.to_owned();
    }
    let chars: Vec<char> = tag.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    let mut best = String::from("…");
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let candidate: String = chars[..mid].iter().collect::<String>() + "…";
        if measure(&candidate) <= max_width {
            best = candidate;
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    best
}

pub fn tag_tree_ui(
    ui: &mut egui::Ui,
    id_salt: &str,
    state: &mut TagTreeState,
    tags: &mut Vec<String>,
    custom_tags: &mut Vec<String>,
) -> TagTreeResult {
    let dark = ui.visuals().dark_mode;
    let mut result = TagTreeResult::default();
    let panel_w = ui.available_width().max(0.0).min(ui.max_rect().width());
    // Pin the whole tree to panel_w so a wide child cannot expand SidePanel
    // (egui allocate_ui grows the parent when content overflows).
    // Do not set_clip_rect here: a height-0 allocate gives a zero-height clip
    // that hides every chip while frames still paint.
    ui.allocate_ui_with_layout(
        egui::vec2(panel_w, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_max_width(panel_w);
            ui.set_min_width(0.0);
            result = tag_tree_ui_inner(ui, id_salt, state, tags, custom_tags, panel_w, dark);
        },
    );
    result
}

fn tag_tree_ui_inner(
    ui: &mut egui::Ui,
    id_salt: &str,
    state: &mut TagTreeState,
    tags: &mut Vec<String>,
    custom_tags: &mut Vec<String>,
    panel_w: f32,
    dark: bool,
) -> TagTreeResult {
    let mut result = TagTreeResult::default();

    let search_w = ui.available_width().min(panel_w);
    let search_resp = ui.add(
        TextEdit::singleline(&mut state.search)
            .hint_text("Search or create tags")
            .desired_width(search_w)
            .id_salt((id_salt, "tag_search")),
    );
    let mut commit = false;
    if state.search.contains(',') {
        commit = true;
        state.search.retain(|c| c != ',');
    }
    if search_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        commit = true;
    }
    if commit && !state.search.trim().is_empty() {
        let r = add_tag(tags, custom_tags, &state.search.clone());
        result.tags_changed |= r.tags_changed;
        result.custom_changed |= r.custom_changed;
        state.search.clear();
        search_resp.request_focus();
    }

    ui.add_space(6.0);

    let filter = state.search.trim().to_lowercase();
    let matches = |tag: &str| filter.is_empty() || tag.to_lowercase().contains(&filter);

    let mut custom_display: Vec<String> =
        custom_tags.iter().filter(|t| matches(t)).cloned().collect();
    for t in tags.iter() {
        if preset_canonical(t).is_none() && matches(t) && !contains_key(&custom_display, t) {
            custom_display.push(t.clone());
        }
    }
    custom_display.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    let show_custom = filter.is_empty() || !custom_display.is_empty();

    for (group, presets, chip) in [
        ("SFW", TAGS_SFW, ChipColor::Cyan),
        ("NSFW", TAGS_NSFW, ChipColor::Volcano),
    ] {
        let visible: Vec<&&str> = presets.iter().filter(|t| matches(t)).collect();
        if visible.is_empty() {
            continue;
        }
        group_frame(ui, panel_w, group, chip, dark, |ui, inner_w| {
            chip_wrap_row(ui, inner_w, |ui| {
                for tag in visible {
                    let on = contains_key(tags, tag);
                    let chip_max = ui.available_width().min(inner_w).max(16.0);
                    if preset_chip(ui, tag, on, chip, dark, chip_max).clicked() {
                        if on {
                            let key = tag_key(tag);
                            tags.retain(|t| tag_key(t) != key);
                        } else {
                            tags.push((*tag).to_string());
                        }
                        result.tags_changed = true;
                    }
                }
            });
        });
        ui.add_space(6.0);
    }

    if show_custom {
        let chip = ChipColor::Purple;
        group_frame(ui, panel_w, "Custom", chip, dark, |ui, inner_w| {
            if custom_display.is_empty() {
                ui.label(
                    RichText::new("Create tags above — they appear here.")
                        .weak()
                        .small(),
                );
                return;
            }
            let mut remove_custom: Option<String> = None;
            let mut finish_rename: Option<bool> = None; // Some(true)=commit, Some(false)=cancel
            chip_wrap_row(ui, inner_w, |ui| {
                for tag in &custom_display {
                    let on = contains_key(tags, tag);
                    let editing = state
                        .renaming
                        .as_ref()
                        .is_some_and(|(old, _)| tag_key(old) == tag_key(tag));

                    let mut toggle = false;
                    let mut start_rename = false;
                    let mut remove = false;

                    let pill_resp = custom_pill(
                        ui,
                        id_salt,
                        tag,
                        on,
                        chip,
                        dark,
                        inner_w,
                        editing,
                        state.renaming.as_mut().map(|(_, d)| d),
                        &mut finish_rename,
                        &mut state.rename_needs_focus,
                    );
                    match pill_resp {
                        CustomPillAction::Toggle => toggle = true,
                        CustomPillAction::StartRename => start_rename = true,
                        CustomPillAction::Remove => remove = true,
                        CustomPillAction::None => {}
                    }

                    if toggle {
                        if on {
                            let key = tag_key(tag);
                            tags.retain(|t| tag_key(t) != key);
                        } else {
                            tags.push(tag.clone());
                        }
                        result.tags_changed = true;
                    }
                    if start_rename {
                        state.renaming = Some((tag.clone(), tag.clone()));
                        state.rename_needs_focus = true;
                    }
                    if remove {
                        remove_custom = Some(tag.clone());
                    }
                }
            });

            if let Some(commit) = finish_rename {
                if let Some((old, draft)) = state.renaming.take() {
                    if commit {
                        let new_name = draft.trim().to_string();
                        let valid = !new_name.is_empty()
                            && preset_canonical(&new_name).is_none()
                            && (tag_key(&new_name) == tag_key(&old)
                                || !contains_key(custom_tags, &new_name));
                        if valid {
                            let old_key = tag_key(&old);
                            for t in custom_tags.iter_mut() {
                                if tag_key(t) == old_key {
                                    *t = new_name.clone();
                                    result.custom_changed = true;
                                }
                            }
                            for t in tags.iter_mut() {
                                if tag_key(t) == old_key {
                                    *t = new_name.clone();
                                    result.tags_changed = true;
                                }
                            }
                        } else {
                            // Invalid — keep editing.
                            state.renaming = Some((old, draft));
                        }
                    }
                }
            }

            if let Some(tag) = remove_custom {
                let key = tag_key(&tag);
                custom_tags.retain(|t| tag_key(t) != key);
                let before = tags.len();
                tags.retain(|t| tag_key(t) != key);
                result.custom_changed = true;
                result.tags_changed |= tags.len() != before;
                if state
                    .renaming
                    .as_ref()
                    .is_some_and(|(old, _)| tag_key(old) == key)
                {
                    state.renaming = None;
                }
            }
        });
        ui.add_space(4.0);
    }

    result
}

/// Wrapping chip row with a real wrap width.
fn chip_wrap_row(ui: &mut egui::Ui, inner_w: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(inner_w.max(1.0), 0.0),
        egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
        |ui| {
            ui.set_max_width(inner_w.max(1.0));
            ui.spacing_mut().item_spacing =
                egui::vec2(ui_base::TAG_CHIP_GAP, ui_base::TAG_CHIP_GAP);
            add_contents(ui);
        },
    );
}

enum CustomPillAction {
    None,
    Toggle,
    StartRename,
    Remove,
}

/// Custom tag pill with edit/delete inside the chip; rename types into the pill.
fn custom_pill(
    ui: &mut egui::Ui,
    id_salt: &str,
    tag: &str,
    selected: bool,
    chip: ChipColor,
    dark: bool,
    inner_w: f32,
    editing: bool,
    draft: Option<&mut String>,
    finish_rename: &mut Option<bool>,
    rename_needs_focus: &mut bool,
) -> CustomPillAction {
    let accent = chip.accent(dark);
    let (fill, text_color, stroke) = if selected && !editing {
        (chip.selected_fill(dark), Color32::WHITE, accent)
    } else {
        let fg = if dark {
            Color32::from_gray(230)
        } else {
            Color32::from_gray(35)
        };
        let fill = if dark {
            Color32::from_rgba_unmultiplied(255, 255, 255, 14)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 180)
        };
        (
            fill,
            fg,
            accent.gamma_multiply(if dark { 0.5 } else { 0.35 }),
        )
    };

    let mut action = CustomPillAction::None;
    const BTN: f32 = 16.0;
    const GAP: f32 = 2.0;
    let action_w = BTN * 2.0 + GAP;
    let font = egui::FontId::new(12.0, egui::FontFamily::Proportional);
    let remaining = ui.available_width();
    // Leftover on this wrap-row is too small: consume it so the pill starts on
    // the next line instead of overflowing the Custom frame.
    if remaining < 48.0 && remaining + 1.0 < inner_w {
        ui.allocate_exact_size(egui::vec2(remaining.max(0.0), 0.0), egui::Sense::hover());
    }
    let max_pill = ui.available_width().min(inner_w).max(16.0);

    let measure_src = if editing {
        draft.as_ref().map(|d| d.as_str()).unwrap_or(tag)
    } else {
        tag
    };
    let label_budget = (max_pill - 12.0 - action_w).max(16.0);
    let display = truncate_tag_label(ui, measure_src, label_budget, &font);
    let text_w = ui.fonts(|f| {
        f.layout_no_wrap(display.clone(), font.clone(), Color32::WHITE)
            .size()
            .x
    });
    // Exact size like SFW/NSFW chips. Frame + small_button overflowed the group.
    let pill_w = if editing {
        max_pill
    } else {
        (text_w + 12.0 + action_w).clamp(48.0, max_pill)
    };
    let pill_h = ui.spacing().interact_size.y.max(22.0);

    let (rect, _) = ui.allocate_exact_size(egui::vec2(pill_w, pill_h), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return action;
    }

    ui.painter().rect(
        rect,
        4.0,
        fill,
        Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );

    let inner = rect.shrink2(egui::vec2(6.0, 2.0));
    let btn_y = inner.center().y - BTN * 0.5;
    let del_rect =
        egui::Rect::from_min_size(egui::pos2(inner.right() - BTN, btn_y), egui::vec2(BTN, BTN));
    let edit_rect = egui::Rect::from_min_size(
        egui::pos2(del_rect.left() - GAP - BTN, btn_y),
        egui::vec2(BTN, BTN),
    );
    let text_rect =
        egui::Rect::from_min_max(inner.min, egui::pos2(edit_rect.left() - 2.0, inner.max.y));
    let id = ui.id().with(id_salt).with("custom_pill").with(tag);

    if editing {
        if let Some(draft) = draft {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
            child.set_clip_rect(text_rect.intersect(ui.clip_rect()));
            let output = TextEdit::singleline(draft)
                .desired_width(text_rect.width().max(16.0))
                .frame(false)
                .text_color(text_color)
                .id_salt((id_salt, "inline_rename", tag))
                .show(&mut child);
            if *rename_needs_focus {
                output.response.request_focus();
                if let Some(mut ts) = TextEdit::load_state(ui.ctx(), output.response.id) {
                    let range = egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(draft.chars().count()),
                    );
                    ts.cursor.set_char_range(Some(range));
                    ts.store(ui.ctx(), output.response.id);
                }
                *rename_needs_focus = false;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                *finish_rename = Some(true);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                *finish_rename = Some(false);
            }
        }
        let ok = ui
            .interact(edit_rect, id.with("ok"), egui::Sense::click())
            .on_hover_text("Save name");
        let cancel = ui
            .interact(del_rect, id.with("cancel"), egui::Sense::click())
            .on_hover_text("Cancel");
        ui.painter().text(
            edit_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✓",
            font.clone(),
            accent,
        );
        ui.painter().text(
            del_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✕",
            font,
            Color32::from_rgb(0xcf, 0x13, 0x22),
        );
        if ok.clicked() {
            *finish_rename = Some(true);
        }
        if cancel.clicked() {
            *finish_rename = Some(false);
        }
    } else {
        let galley = ui.fonts(|f| f.layout_no_wrap(display.clone(), font.clone(), text_color));
        let text_pos = egui::pos2(
            text_rect.left(),
            text_rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, text_color);
        let label = ui.interact(text_rect, id.with("label"), egui::Sense::click());
        if display != tag {
            label.clone().on_hover_text(tag);
        }
        if label.clicked() {
            action = CustomPillAction::Toggle;
        }
        let edit = ui
            .interact(edit_rect, id.with("edit"), egui::Sense::click())
            .on_hover_text("Rename");
        let del = ui
            .interact(del_rect, id.with("del"), egui::Sense::click())
            .on_hover_text("Remove saved tag");
        ui.painter().text(
            edit_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✎",
            font.clone(),
            text_color,
        );
        ui.painter().text(
            del_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✕",
            font,
            Color32::from_rgb(0xcf, 0x13, 0x22),
        );
        if edit.clicked() {
            action = CustomPillAction::StartRename;
        }
        if del.clicked() {
            action = CustomPillAction::Remove;
        }
    }

    action
}

fn group_frame(
    ui: &mut egui::Ui,
    panel_w: f32,
    title: &str,
    chip: ChipColor,
    dark: bool,
    add_contents: impl FnOnce(&mut egui::Ui, f32),
) {
    let accent = chip.accent(dark);
    let panel = chip.wash(dark);
    let outer_w = panel_w.min(ui.available_width()).max(0.0);
    ui.allocate_ui_with_layout(
        egui::vec2(outer_w.max(1.0), 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_min_width(outer_w);
            ui.set_max_width(outer_w);
            egui::Frame::new()
                .fill(panel)
                .stroke(Stroke::new(
                    1.0,
                    accent.gamma_multiply(if dark { 0.55 } else { 0.35 }),
                ))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(ui_base::TAG_FRAME_PAD as i8))
                .show(ui, |ui| {
                    let inner_w = (outer_w - ui_base::TAG_FRAME_PAD * 2.0)
                        .max(0.0)
                        .min(ui.available_width());
                    ui.set_min_width(inner_w);
                    ui.set_max_width(inner_w);
                    ui.label(RichText::new(title).color(accent).size(13.0));
                    ui.add_space(4.0);
                    add_contents(ui, inner_w);
                    ui.add_space(2.0);
                });
        },
    );
}

fn preset_chip(
    ui: &mut egui::Ui,
    tag: &str,
    selected: bool,
    chip: ChipColor,
    dark: bool,
    max_width: f32,
) -> egui::Response {
    let accent = chip.accent(dark);
    let (fill, text, stroke) = if selected {
        (chip.selected_fill(dark), Color32::WHITE, accent)
    } else {
        let fg = if dark {
            Color32::from_gray(230)
        } else {
            Color32::from_gray(35)
        };
        let fill = if dark {
            Color32::from_rgba_unmultiplied(255, 255, 255, 14)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 180)
        };
        (
            fill,
            fg,
            accent.gamma_multiply(if dark { 0.5 } else { 0.35 }),
        )
    };
    let max_width = max_width.max(24.0);
    let label_max = (max_width - 12.0).max(12.0);
    let font = egui::FontId::new(12.0, egui::FontFamily::Proportional);
    let label = truncate_tag_label(ui, tag, label_max, &font);
    let truncated = label != tag;

    let text_w = ui.fonts(|f| {
        f.layout_no_wrap(label.clone(), font.clone(), Color32::WHITE)
            .size()
            .x
    });
    let chip_w = (text_w + 12.0).clamp(24.0, max_width);
    let chip_h = 22.0;
    let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(chip_w, chip_h), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = if resp.hovered() {
            ui.visuals().widgets.hovered
        } else {
            ui.visuals().widgets.inactive
        };
        let _ = visuals;
        ui.painter().rect(
            rect,
            4.0,
            fill,
            Stroke::new(1.0, stroke),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &label,
            font,
            text,
        );
    }
    if truncated {
        resp = resp.on_hover_text(tag);
    }
    resp
}
