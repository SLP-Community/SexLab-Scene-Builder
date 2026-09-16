use egui::Color32;

pub fn shell_bg() -> Color32 {
    Color32::from_rgb(0xf5, 0xf5, 0xf5)
}

pub fn panel_bg() -> Color32 {
    Color32::WHITE
}

pub fn panel_alt() -> Color32 {
    Color32::WHITE
}

pub fn accent() -> Color32 {
    Color32::from_rgb(0x16, 0x77, 0xff)
}

pub fn accent_hover() -> Color32 {
    Color32::from_rgb(0x40, 0x90, 0xff)
}

pub fn text_strong() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 224)
}

pub fn text_muted() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 115)
}

pub fn border() -> Color32 {
    Color32::from_rgba_unmultiplied(33, 35, 48, 71)
}

pub fn border_strong() -> Color32 {
    Color32::from_rgba_unmultiplied(33, 35, 48, 230)
}

pub fn hover_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 15)
}

pub fn input_bg() -> Color32 {
    Color32::WHITE
}

pub fn hyperlink() -> Color32 {
    accent()
}

pub fn faint_bg() -> Color32 {
    Color32::from_rgb(0xf0, 0xf0, 0xf0)
}

pub fn code_bg() -> Color32 {
    Color32::from_rgb(0xfa, 0xfa, 0xfa)
}
