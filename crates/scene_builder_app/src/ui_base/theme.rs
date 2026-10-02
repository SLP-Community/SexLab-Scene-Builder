use super::visual_style::style;

pub fn apply(ctx: &egui::Context, dark: bool) {
    ctx.set_style(style(dark));
}
