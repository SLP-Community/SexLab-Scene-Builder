use super::{
    PendingAction, SceneBuilderApp, DISCORD_URL, KOFI_MISS_URL, KOFI_URL, PATREON_URL, WIKI_URL,
};
use crate::app_prefs::ThemePref;
use crate::io;
use egui::Context;
use scene_builder_core::project::package::ExportKind;

impl SceneBuilderApp {
    pub(super) fn menu_bar(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .add(egui::Button::new("New").shortcut_text("Ctrl+N"))
                    .clicked()
                {
                    self.request_if_clean(PendingAction::New);
                    ui.close_menu();
                }
                if ui
                    .add(egui::Button::new("Open…").shortcut_text("Ctrl+O"))
                    .clicked()
                {
                    self.request_if_clean(PendingAction::Open);
                    ui.close_menu();
                }
                if ui
                    .add(egui::Button::new("Save").shortcut_text("Ctrl+S"))
                    .clicked()
                {
                    self.save_project(false);
                    ui.close_menu();
                }
                if ui
                    .add(egui::Button::new("Save As…").shortcut_text("Ctrl+Shift+S"))
                    .clicked()
                {
                    self.save_project(true);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Import SLAL pack…").clicked() {
                    self.request_if_clean(PendingAction::ImportSlal);
                    ui.close_menu();
                }
                if ui.button("Import Offset…").clicked() {
                    io::spawn_offset(self.dialog_tx.clone());
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export SLSB…").clicked() {
                    self.request_export(ExportKind::Slsb);
                    ui.close_menu();
                }
                if ui.button("Export SLAL…").clicked() {
                    self.request_export(ExportKind::Slal);
                    ui.close_menu();
                }
                if ui
                    .add(egui::Button::new("Export Both…").shortcut_text("Ctrl+B"))
                    .clicked()
                {
                    self.request_export(ExportKind::Both);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    if self.ws.dirty {
                        self.pending_after_confirm = Some(PendingAction::Quit);
                        self.show_close_confirm = true;
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button("Tools", |ui| {
                if ui.button("Enrich SLAnim…").clicked() {
                    io::spawn_enrich_slanim(self.dialog_tx.clone());
                    ui.close_menu();
                }
                if ui.button("Enrich FNIS…").clicked() {
                    io::spawn_enrich_fnis(self.dialog_tx.clone());
                    ui.close_menu();
                }
            });
            ui.menu_button("View", |ui| {
                ui.menu_button("Theme", |ui| {
                    for (label, pref) in [
                        ("System", ThemePref::System),
                        ("Light", ThemePref::Light),
                        ("Dark", ThemePref::Dark),
                    ] {
                        if ui
                            .selectable_label(self.prefs.theme == pref, label)
                            .clicked()
                        {
                            self.prefs.theme = pref;
                            pref.apply(ctx);
                            self.prefs.save();
                            ui.close_menu();
                        }
                    }
                });
                #[cfg(windows)]
                {
                    use log::info;
                    let mut show = self.prefs.show_console;
                    if ui
                        .checkbox(&mut show, "Show console")
                        .on_hover_text("Attach a console window for log output (also: --console)")
                        .changed()
                    {
                        self.prefs.show_console = show;
                        self.prefs.save();
                        if show {
                            let _ = crate::console_win::show();
                            info!("Console enabled");
                        } else {
                            crate::console_win::hide();
                        }
                    }
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Wiki").clicked() {
                    let _ = open::that(WIKI_URL);
                    ui.close_menu();
                }
                if ui.button("About").clicked() {
                    self.show_about = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Discord").clicked() {
                    let _ = open::that(DISCORD_URL);
                    ui.close_menu();
                }
                if ui.button("Patreon").clicked() {
                    let _ = open::that(PATREON_URL);
                    ui.close_menu();
                }
                if ui.button("Ko-Fi (Scrab)").clicked() {
                    let _ = open::that(KOFI_URL);
                    ui.close_menu();
                }
                if ui.button("Ko-Fi (Miss Corruption)").clicked() {
                    let _ = open::that(KOFI_MISS_URL);
                    ui.close_menu();
                }
            });
        });
    }
}
