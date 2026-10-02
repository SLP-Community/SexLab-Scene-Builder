//! Named color tokens that do not change with light/dark.

use egui::Color32;

/// Classic X6 stage node fill `rgb(221, 235, 217)` — same in light and dark.
pub const SCENE_NODE_BG: Color32 = Color32::from_rgb(221, 235, 217);
pub const SCENE_NODE_TEXT: Color32 = Color32::from_rgb(0, 0, 0);
pub const SCENE_NODE_CONNECT: Color32 = Color32::from_rgb(240, 220, 160);

pub const RADIUS: u8 = 6;
