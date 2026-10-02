//! Bottom-left toast notifications.

use egui::{Color32, RichText};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Warning,
}

impl ToastKind {
    fn glyph(self) -> &'static str {
        match self {
            ToastKind::Success => "✔",
            ToastKind::Error => "✖",
            ToastKind::Warning => "⚠",
        }
    }

    fn color(self) -> Color32 {
        match self {
            ToastKind::Success => Color32::from_rgb(0x52, 0xc4, 0x1a),
            ToastKind::Error => Color32::from_rgb(0xff, 0x4d, 0x4f),
            ToastKind::Warning => Color32::from_rgb(0xfa, 0xad, 0x14),
        }
    }
}

#[derive(Debug, Clone)]
struct Toast {
    kind: ToastKind,
    title: String,
    description: String,
    expires_at: f64,
}

#[derive(Debug, Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn push(&mut self, ctx: &egui::Context, kind: ToastKind, title: &str, description: &str) {
        let now = ctx.input(|i| i.time);
        let ttl = match kind {
            ToastKind::Error => 8.0,
            _ => 5.0,
        };
        self.items.push(Toast {
            kind,
            title: title.to_string(),
            description: description.to_string(),
            expires_at: now + ttl,
        });
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        if self.items.is_empty() {
            return;
        }
        let now = ctx.input(|i| i.time);
        self.items.retain(|t| t.expires_at > now);
        if self.items.is_empty() {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));

        let mut dismiss: Option<usize> = None;
        egui::Area::new(egui::Id::new("toast_stack"))
            .anchor(egui::Align2::LEFT_BOTTOM, [12.0, -32.0])
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ctx, |ui| {
                for (i, toast) in self.items.iter().enumerate() {
                    egui::Frame::popup(ui.style())
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_max_width(340.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(toast.kind.glyph())
                                        .color(toast.kind.color())
                                        .size(15.0),
                                );
                                ui.label(RichText::new(&toast.title).strong());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("✕").clicked() {
                                            dismiss = Some(i);
                                        }
                                    },
                                );
                            });
                            if !toast.description.is_empty() {
                                ui.label(&toast.description);
                            }
                        });
                }
            });
        if let Some(i) = dismiss {
            self.items.remove(i);
        }
    }
}
