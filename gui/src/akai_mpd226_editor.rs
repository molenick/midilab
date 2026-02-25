use eframe::egui::Vec2;

use crate::spacing::spacing;

mod render;
pub mod state;

const APP_X: f32 = spacing(0) * 130.;
const APP_Y: f32 = spacing(0) * 87.;
pub const APP_DIMENSIONS: Vec2 = Vec2 { x: APP_X, y: APP_Y };
