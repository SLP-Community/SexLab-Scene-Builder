use crate::ui_base;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePref {
    pub fn is_dark(self) -> bool {
        match self {
            ThemePref::Light => false,
            ThemePref::Dark => true,
            ThemePref::System => dark_from_env(),
        }
    }

    pub fn apply(self, ctx: &egui::Context) {
        ui_base::apply(ctx, self.is_dark());
    }
}

fn dark_from_env() -> bool {
    std::env::var("GTK_THEME")
        .map(|t| t.to_ascii_lowercase().contains("dark"))
        .unwrap_or(false)
        || std::env::var("COLORFGBG")
            .ok()
            .and_then(|v| v.split(';').last()?.parse::<u8>().ok())
            .map(|bg| bg < 8)
            .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub theme: ThemePref,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub bottom_panel_height: f32,
    pub window_width: f32,
    pub window_height: f32,
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
    pub window_maximized: bool,
    /// Saved custom tags ("Yours" group in the tag tree).
    pub custom_tags: Vec<String>,
    /// "Don't show this tip again on export?" (Pandora clip-folder tip).
    pub hide_export_clip_tip: bool,
    /// "Don't warn about export overwrites again?"
    pub hide_export_merge_warn: bool,
    /// Show a debug console window (Windows). Toggle under View; also `--console`.
    pub show_console: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: ThemePref::System,
            left_panel_width: super::DEFAULT_LEFT_PANEL_WIDTH,
            right_panel_width: super::DEFAULT_RIGHT_PANEL_WIDTH,
            bottom_panel_height: crate::ui_base::POSITIONS_PANEL_FALLBACK_H,
            window_width: crate::ui_base::WINDOW_DEFAULT_W,
            window_height: crate::ui_base::WINDOW_DEFAULT_H,
            window_x: None,
            window_y: None,
            window_maximized: false,
            custom_tags: Vec::new(),
            hide_export_clip_tip: false,
            hide_export_merge_warn: false,
            show_console: false,
        }
    }
}

impl Prefs {
    fn path() -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("SexLabSceneBuilder").join("prefs.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, text);
        }
    }

    pub fn apply_viewport(&self, mut viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
        let w = self.window_width.max(crate::ui_base::WINDOW_MIN_W);
        let h = self.window_height.max(crate::ui_base::WINDOW_MIN_H);
        viewport = viewport
            .with_inner_size([w, h])
            .with_min_inner_size([crate::ui_base::WINDOW_MIN_W, crate::ui_base::WINDOW_MIN_H])
            .with_maximized(self.window_maximized);
        if let (Some(x), Some(y)) = (self.window_x, self.window_y) {
            viewport = viewport.with_position([x, y]);
        }
        viewport
    }

    /// Copy current OS window geometry into prefs. Returns true if anything changed.
    pub fn capture_viewport(&mut self, ctx: &egui::Context) -> bool {
        let vp = ctx.input(|i| i.viewport().clone());
        if vp.minimized == Some(true) {
            return false;
        }
        let mut changed = false;
        let maximized = vp.maximized.unwrap_or(false);
        if maximized != self.window_maximized {
            self.window_maximized = maximized;
            changed = true;
        }
        if maximized {
            return changed;
        }
        if let Some(inner) = vp.inner_rect {
            let w = inner.width().round();
            let h = inner.height().round();
            if w >= crate::ui_base::WINDOW_MIN_W && h >= crate::ui_base::WINDOW_MIN_H {
                if (w - self.window_width).abs() > 1.0 || (h - self.window_height).abs() > 1.0 {
                    self.window_width = w;
                    self.window_height = h;
                    changed = true;
                }
            }
        }
        if let Some(outer) = vp.outer_rect {
            let x = outer.min.x.round();
            let y = outer.min.y.round();
            let pos_changed = self.window_x.map(|px| (px - x).abs() > 1.0).unwrap_or(true)
                || self.window_y.map(|py| (py - y).abs() > 1.0).unwrap_or(true);
            if pos_changed {
                self.window_x = Some(x);
                self.window_y = Some(y);
                changed = true;
            }
        }
        changed
    }
}
