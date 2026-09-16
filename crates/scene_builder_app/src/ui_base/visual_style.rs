use super::color_structs::RADIUS;
use super::colors::{
    accent, accent_hover, border, code_bg, faint_bg, hover_fill, hyperlink, input_bg, panel_alt,
    panel_bg, shell_bg, text_muted, text_strong,
};
use egui::style::WidgetVisuals;
use egui::{Color32, CornerRadius, FontFamily, FontId, Shadow, Stroke, Style, TextStyle, Visuals};

fn widget(bg: Color32, weak_bg: Color32, stroke: Color32, fg: Color32) -> WidgetVisuals {
    WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: weak_bg,
        bg_stroke: Stroke::new(1.0, stroke),
        corner_radius: CornerRadius::same(RADIUS),
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 0.0,
    }
}

pub fn visuals(dark: bool) -> Visuals {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let shell = shell_bg(dark);
    let panel = panel_bg(dark);
    let alt = panel_alt(dark);
    let acc = accent(dark);
    let acc_h = accent_hover(dark);
    let fg = text_strong(dark);
    let muted = text_muted(dark);
    let bd = border(dark);
    let hover = hover_fill(dark);
    let inp = input_bg(dark);

    v.dark_mode = dark;
    v.override_text_color = Some(fg);
    v.hyperlink_color = hyperlink(dark);
    v.warn_fg_color = Color32::from_rgb(0xfa, 0xad, 0x14);
    v.error_fg_color = Color32::from_rgb(0xff, 0x4d, 0x4f);
    v.window_fill = alt;
    v.panel_fill = panel;
    v.extreme_bg_color = shell;
    v.faint_bg_color = faint_bg(dark);
    v.code_bg_color = code_bg(dark);
    v.window_stroke = Stroke::new(1.0, bd);
    v.window_corner_radius = CornerRadius::same(RADIUS);
    v.menu_corner_radius = CornerRadius::same(RADIUS);
    v.window_shadow = if dark {
        Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(120),
        }
    } else {
        Shadow {
            offset: [0, 2],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(40),
        }
    };
    v.popup_shadow = v.window_shadow;
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(acc.r(), acc.g(), acc.b(), 64);
    v.selection.stroke = Stroke::new(1.0, acc);

    v.widgets.noninteractive = widget(panel, shell, bd, muted);
    v.widgets.inactive = widget(inp, panel, bd, fg);
    v.widgets.hovered = widget(hover, hover, acc_h, fg);
    v.widgets.active = widget(
        Color32::from_rgba_unmultiplied(acc.r(), acc.g(), acc.b(), 48),
        hover,
        acc,
        fg,
    );
    v.widgets.open = widget(alt, alt, acc, fg);

    v.text_cursor.stroke = Stroke::new(2.0, acc);
    v
}

pub fn style(dark: bool) -> Style {
    let mut style = Style {
        visuals: visuals(dark),
        ..Default::default()
    };

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    style.spacing.indent = 18.0;
    style.spacing.window_margin = egui::Margin::same(10);
    style.spacing.menu_margin = egui::Margin::same(4);
    style.interaction.selectable_labels = false;

    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(10.5, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(12.5, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(12.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(15.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.5, FontFamily::Monospace),
    );

    style
}
