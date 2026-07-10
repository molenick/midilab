use eframe::egui::Vec2;

use crate::spacing::spacing;

pub mod app;
pub mod config;
pub mod fs;
pub mod message;
pub mod render;
pub(crate) mod spacing;
pub mod state;

pub use state::AkaiMpd226Editor;

const APP_X: f32 = spacing(0) * 130.;
const APP_Y: f32 = spacing(0) * 87.;
pub const APP_DIMENSIONS: Vec2 = Vec2 { x: APP_X, y: APP_Y };
