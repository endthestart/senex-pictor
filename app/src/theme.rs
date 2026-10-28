//! The app's dark, night-friendly look from the design boards: a blue-black ground, raised
//! cards, a star-blue accent, green for done and amber for things to look at; Inter for text
//! (from LightCraft, SIL OFL) with a semibold face for headings.

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, Visuals};
use std::sync::Arc;

pub const GROUND: Color32 = Color32::from_rgb(13, 17, 23);
/// Header, step list and footer.
pub const CHROME: Color32 = Color32::from_rgb(17, 22, 29);
pub const CARD: Color32 = Color32::from_rgb(21, 26, 34);
/// The selected step, the recommended run, hovered cards.
pub const RAISED: Color32 = Color32::from_rgb(27, 35, 48);
pub const LINE: Color32 = Color32::from_rgb(42, 51, 64);
pub const LINE_STRONG: Color32 = Color32::from_rgb(58, 74, 99);
pub const TEXT: Color32 = Color32::from_rgb(230, 234, 240);
pub const MUTED: Color32 = Color32::from_rgb(161, 171, 184);
pub const ACCENT: Color32 = Color32::from_rgb(143, 180, 255);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(44, 74, 122);
pub const GOOD: Color32 = Color32::from_rgb(92, 194, 138);
pub const WARN: Color32 = Color32::from_rgb(240, 169, 59);
pub const BAD: Color32 = Color32::from_rgb(240, 113, 103);

pub const SEMIBOLD: &str = "semibold";

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-SemiBold".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    let defaults = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut body = vec!["Inter".to_string()];
    body.extend(defaults.iter().cloned());
    fonts.families.insert(FontFamily::Proportional, body);
    let mut strong = vec!["Inter-SemiBold".to_string()];
    strong.extend(defaults);
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD.into()), strong);
    fonts
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    let mut v = Visuals::dark();
    v.panel_fill = GROUND;
    v.window_fill = CARD;
    v.extreme_bg_color = GROUND;
    v.faint_bg_color = CHROME;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.selection.bg_fill = ACCENT_DIM;
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    v.override_text_color = Some(TEXT);
    v.slider_trailing_fill = true;
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = CARD;
    w.noninteractive.weak_bg_fill = CARD;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    // `bg_fill` draws slider rails and check boxes, `weak_bg_fill` buttons.
    for (state, rail, fill, stroke) in [
        (&mut w.inactive, LINE, CARD, LINE),
        (&mut w.hovered, LINE_STRONG, RAISED, LINE_STRONG),
        (&mut w.active, LINE_STRONG, RAISED, ACCENT),
        (&mut w.open, LINE_STRONG, RAISED, LINE_STRONG),
    ] {
        state.bg_fill = rail;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, stroke);
        state.fg_stroke = Stroke::new(1.5, TEXT);
        state.corner_radius = CornerRadius::same(6);
        state.expansion = 0.0;
    }
    w.noninteractive.corner_radius = CornerRadius::same(6);
    ctx.set_visuals_of(egui::Theme::Dark, v.clone());
    ctx.set_visuals_of(egui::Theme::Light, v);
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        s.spacing.slider_width = 220.0;
        s.text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        s.text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
        s.text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(12.0));
        s.text_styles
            .insert(egui::TextStyle::Monospace, FontId::monospace(12.5));
        s.text_styles
            .insert(egui::TextStyle::Heading, semibold(24.0));
        s.animation_time = 0.1;
    });
}
