//! Named sizes for the desktop shell. Prefer these over one-off magic numbers.

/// Inner margin used on side / bottom chrome panels.
pub const PANEL_MARGIN: i8 = 10;

pub const WINDOW_DEFAULT_W: f32 = 1280.0;
pub const WINDOW_DEFAULT_H: f32 = 800.0;
pub const WINDOW_MIN_W: f32 = 800.0;
pub const WINDOW_MIN_H: f32 = 600.0;

pub const LEFT_PANEL_MIN: f32 = 180.0;
pub const LEFT_PANEL_MAX: f32 = 420.0;
pub const RIGHT_PANEL_MIN: f32 = 200.0;
pub const RIGHT_PANEL_MAX: f32 = 560.0;

/// Stop SidePanel from adopting overflowing content as its width (black gap + snap-back).
pub fn constrain_panel_contents(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.set_min_width(0.0);
    ui.set_max_width(rect.width());
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
    ui.set_clip_rect(ui.clip_rect().intersect(rect));
}

/// Make the panel report `width`, even if nested panels later shrink `max_rect`.
pub fn claim_width(ui: &mut egui::Ui, width: f32) {
    if !width.is_finite() || width <= 0.0 {
        return;
    }
    ui.set_min_width(width);
    ui.expand_to_include_x(ui.max_rect().left() + width);
}

/// Make the panel report the current `max_rect` width.
pub fn claim_allocated_width(ui: &mut egui::Ui) {
    claim_width(ui, ui.max_rect().width());
}

/// egui persists `SidePanel` size from the content frame. Nested widgets can
/// make that smaller than the drag width, so the panel snaps back next frame.
pub fn persist_side_panel_rect(ctx: &egui::Context, id: egui::Id, rect: egui::Rect) {
    ctx.data_mut(|d| {
        d.insert_persisted(id, egui::containers::panel::PanelState { rect });
    });
}

/// Keep at least this much of the tags column visible above Furniture.
pub const TAGS_SCROLL_MIN_H: f32 = 80.0;

/// Title row in the Scene Positions strip.
pub const POSITIONS_HEADER_H: f32 = 32.0;
/// First-frame fallback before content is measured.
pub const POSITIONS_PANEL_FALLBACK_H: f32 = 180.0;
/// Panel frame chrome (inner margin × 2) plus a little slack so the bar never micro-scrolls.
pub const POSITIONS_PANEL_CHROME: f32 = (PANEL_MARGIN as f32) * 2.0 + 6.0;
/// Cap so a huge window still leaves room for the graph.
pub const POSITIONS_PANEL_MAX_FRAC: f32 = 0.4;
pub const POSITIONS_PANEL_ABS_MAX: f32 = 480.0;

pub const SPACE: f32 = 8.0;
pub const SPACE_SM: f32 = 4.0;
pub const SPACE_XS: f32 = 2.0;

/// Tag group frame inner padding (egui `Margin::same`).
pub const TAG_FRAME_PAD: f32 = 8.0;
pub const TAG_CHIP_GAP: f32 = 4.0;

/// Finite, positive layout length, otherwise `fallback` (guards NaN panel sizes).
pub fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        fallback
    }
}
