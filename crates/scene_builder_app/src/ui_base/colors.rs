use super::{colors_dark as dark, colors_light as light};
use egui::Color32;

pub fn shell_bg(is_dark: bool) -> Color32 {
    if is_dark {
        dark::shell_bg()
    } else {
        light::shell_bg()
    }
}

pub fn panel_bg(is_dark: bool) -> Color32 {
    if is_dark {
        dark::panel_bg()
    } else {
        light::panel_bg()
    }
}

pub fn panel_alt(is_dark: bool) -> Color32 {
    if is_dark {
        dark::panel_alt()
    } else {
        light::panel_alt()
    }
}

pub fn accent(is_dark: bool) -> Color32 {
    if is_dark {
        dark::accent()
    } else {
        light::accent()
    }
}

pub fn accent_hover(is_dark: bool) -> Color32 {
    if is_dark {
        dark::accent_hover()
    } else {
        light::accent_hover()
    }
}

pub fn text_strong(is_dark: bool) -> Color32 {
    if is_dark {
        dark::text_strong()
    } else {
        light::text_strong()
    }
}

pub fn text_muted(is_dark: bool) -> Color32 {
    if is_dark {
        dark::text_muted()
    } else {
        light::text_muted()
    }
}

pub fn border(is_dark: bool) -> Color32 {
    if is_dark {
        dark::border()
    } else {
        light::border()
    }
}

pub fn border_strong(is_dark: bool) -> Color32 {
    if is_dark {
        dark::border_strong()
    } else {
        light::border_strong()
    }
}

pub fn hover_fill(is_dark: bool) -> Color32 {
    if is_dark {
        dark::hover_fill()
    } else {
        light::hover_fill()
    }
}

pub fn input_bg(is_dark: bool) -> Color32 {
    if is_dark {
        dark::input_bg()
    } else {
        light::input_bg()
    }
}

pub fn hyperlink(is_dark: bool) -> Color32 {
    if is_dark {
        dark::hyperlink()
    } else {
        light::hyperlink()
    }
}

pub fn faint_bg(is_dark: bool) -> Color32 {
    if is_dark {
        dark::faint_bg()
    } else {
        light::faint_bg()
    }
}

pub fn code_bg(is_dark: bool) -> Color32 {
    if is_dark {
        dark::code_bg()
    } else {
        light::code_bg()
    }
}
