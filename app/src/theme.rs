//! The app's dark, night-friendly look (design boards: blue-black ground, star-blue accent):
//! egui's dark visuals with the design's palette.

use egui::{Color32, Visuals};

const GROUND: Color32 = Color32::from_rgb(11, 14, 19);
const PANEL: Color32 = Color32::from_rgb(18, 24, 33);
const LINE: Color32 = Color32::from_rgb(36, 48, 64);
const STAR: Color32 = Color32::from_rgb(169, 196, 255);

pub fn apply(ctx: &egui::Context) {
    let mut v = Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = GROUND;
    v.faint_bg_color = GROUND;
    v.selection.bg_fill = STAR.gamma_multiply(0.45);
    v.selection.stroke.color = STAR;
    v.hyperlink_color = STAR;
    v.widgets.noninteractive.bg_stroke.color = LINE;
    ctx.set_visuals(v);
}
