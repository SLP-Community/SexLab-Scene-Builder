//! Shared visual tokens for the egui UI.

mod color_structs;
mod colors;
mod colors_dark;
mod colors_light;
mod fonts;
pub mod sizing;
mod theme;
mod visual_style;

pub use color_structs::{SCENE_NODE_BG, SCENE_NODE_CONNECT, SCENE_NODE_TEXT};
pub use colors::{accent, accent_hover, border, border_strong, panel_bg, shell_bg, text_muted};
pub use fonts::configure_fonts;
pub use sizing::*;
pub use theme::apply;
