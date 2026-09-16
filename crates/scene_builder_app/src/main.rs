// Hide the console window on Windows GUI builds. Use View → Show console (or --console)
// to attach one for log output.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod app_prefs;
#[cfg(windows)]
mod console_win;
mod furniture;
mod graph;
mod graph_layout;
mod io;
mod jobs;
mod positions;
mod stage_editor;
mod tags;
mod toasts;
mod ui_base;
mod ui_define;
mod workspace;

use app::SceneBuilderApp;
use log::LevelFilter;
use std::fs::OpenOptions;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let want_console = std::env::args().any(|a| a == "--console" || a == "-c");
    let mut prefs = app_prefs::Prefs::load();
    if want_console {
        prefs.show_console = true;
    }

    #[cfg(windows)]
    if prefs.show_console {
        let _ = console_win::show();
    }

    init_logging(prefs.show_console);

    let mut viewport = prefs
        .apply_viewport(egui::ViewportBuilder::default().with_title(SceneBuilderApp::APP_TITLE));

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        SceneBuilderApp::APP_TITLE,
        options,
        Box::new(move |cc| {
            ui_base::configure_fonts(&cc.egui_ctx);
            prefs.theme.apply(&cc.egui_ctx);
            Ok(Box::new(SceneBuilderApp::new(prefs)))
        }),
    )
}

fn init_logging(also_console: bool) {
    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(LevelFilter::Info);

    // Always keep a log file (useful when the console is hidden).
    if let Some(path) = log_file_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
            dispatch = dispatch.chain(file);
        }
    }

    #[cfg(windows)]
    {
        // Always chain — ConsoleWriter no-ops until View → Show console / --console.
        let _ = also_console;
        dispatch = dispatch
            .chain(Box::new(console_win::console_writer()) as Box<dyn std::io::Write + Send>);
    }

    #[cfg(not(windows))]
    {
        let _ = also_console;
        dispatch = dispatch.chain(std::io::stdout());
    }

    let _ = dispatch.apply();
}

fn log_file_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("SexLabSceneBuilder").join("scene_builder.log"))
}

fn load_icon() -> Option<egui::IconData> {
    // Embedded so shipped .exe does not depend on CWD.
    const EMBEDDED: &[u8] = include_bytes!("../assets/app-icon-256.png");
    if let Ok(img) = image::load_from_memory(EMBEDDED) {
        let rgba = img.into_rgba8();
        let (w, h) = rgba.dimensions();
        return Some(egui::IconData {
            rgba: rgba.into_raw(),
            width: w,
            height: h,
        });
    }
    None
}
