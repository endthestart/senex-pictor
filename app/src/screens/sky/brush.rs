//! Correcting the matte by hand: brushes that paint sky or ground over the result in scene
//! (store) pixels, with undo. The strokes are drawn as the engine applies them, rasterised into
//! one translucent layer, so what shows is what Apply paints. Move (V) pans; Sky (S) and Ground
//! (G) paint with the primary button while the others, or Space, pan; [ and ] size the brush.

use crate::viewer::Viewer;
use egui::{
    Color32, ColorImage, Key, PointerButton, Rect, Response, Stroke as Line, TextureHandle, Ui,
    pos2,
};
use pictor_engine::separate::corrections::{Stroke, force};

const SKY_PAINT: Color32 = Color32::from_rgba_premultiplied(30, 56, 96, 110);
const GROUND_PAINT: Color32 = Color32::from_rgba_premultiplied(120, 56, 52, 110);
pub const RADIUS_RANGE: std::ops::RangeInclusive<f32> = 4.0..=400.0;
/// Longest side of the drawn layer, px.
const LAYER_SIDE: usize = 1600;

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Move,
    Sky,
    Ground,
}

pub struct Brush {
    pub tool: Tool,
    /// Brush radius in scene pixels.
    pub radius: f32,
    pub strokes: Vec<Stroke>,
    /// The strokes the store holds.
    pub saved: Vec<Stroke>,
    drawing: bool,
    /// The strokes rasterised, with a hash of what they were drawn from.
    layer: Option<(u64, TextureHandle)>,
}

impl Brush {
    pub fn new(saved: Vec<Stroke>) -> Brush {
        Brush {
            tool: Tool::Move,
            radius: 40.0,
            strokes: saved.clone(),
            saved,
            drawing: false,
            layer: None,
        }
    }

    /// Whether the strokes differ from those the store holds.
    pub fn changed(&self) -> bool {
        self.strokes != self.saved
    }

    pub fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        ctx.input(|i| {
            if i.modifiers.command {
                if i.key_pressed(Key::Z) {
                    self.strokes.pop();
                }
                return;
            }
            for (key, tool) in [
                (Key::V, Tool::Move),
                (Key::S, Tool::Sky),
                (Key::G, Tool::Ground),
            ] {
                if i.key_pressed(key) {
                    self.tool = tool;
                }
            }
            let resize = |r: f32, f: f32| (r * f).clamp(*RADIUS_RANGE.start(), *RADIUS_RANGE.end());
            if i.key_pressed(Key::OpenBracket) {
                self.radius = resize(self.radius, 0.8);
            }
            if i.key_pressed(Key::CloseBracket) {
                self.radius = resize(self.radius, 1.25);
            }
        });
    }

    /// Paints with the pointer on the canvas `response`, then draws every stroke and the brush.
    pub fn paint(&mut self, ui: &Ui, response: &Response, viewer: &Viewer) {
        let ctx = ui.ctx();
        let (rect, ppp) = (response.rect, ctx.pixels_per_point());
        let sky = match self.tool {
            Tool::Move => None,
            Tool::Sky => Some(true),
            Tool::Ground => Some(false),
        };
        let space = ctx.input(|i| i.key_down(Key::Space));
        if let Some(sky) = sky
            && !space
        {
            let at = |p: egui::Pos2| viewer.scene_at(rect, ppp, p);
            if response.drag_started_by(PointerButton::Primary)
                && let Some(origin) = ctx.input(|i| i.pointer.press_origin())
            {
                self.strokes.push(Stroke {
                    sky,
                    radius: self.radius,
                    points: vec![at(origin)],
                });
                self.drawing = true;
            }
            if self.drawing
                && response.dragged_by(PointerButton::Primary)
                && let (Some(pos), Some(stroke)) =
                    (response.interact_pointer_pos(), self.strokes.last_mut())
            {
                let p = at(pos);
                let last = stroke.points[stroke.points.len() - 1];
                if (p[0] - last[0]).hypot(p[1] - last[1]) >= stroke.radius / 4.0 {
                    stroke.points.push(p);
                }
            }
            if response.drag_stopped() {
                self.drawing = false;
            }
            if response.clicked_by(PointerButton::Primary)
                && let Some(pos) = response.interact_pointer_pos()
            {
                self.strokes.push(Stroke {
                    sky,
                    radius: self.radius,
                    points: vec![at(pos)],
                });
            }
        }
        let painter = ui.painter_at(rect);
        let scale = viewer.points_per_scene(rect, ppp);
        if let Some(texture) = self.layer(ctx, viewer) {
            let (w, h) = (viewer.scene.width as f32, viewer.scene.height as f32);
            let shown = Rect::from_min_max(
                viewer.screen_at(rect, ppp, [0.0, 0.0]),
                viewer.screen_at(rect, ppp, [w, h]),
            );
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            painter.image(texture.id(), shown, uv, Color32::WHITE);
        }
        if sky.is_some()
            && let Some(pos) = response.hover_pos()
        {
            painter.circle_stroke(pos, self.radius * scale, Line::new(1.5, Color32::WHITE));
            painter.circle_stroke(
                pos,
                self.radius * scale + 1.5,
                Line::new(1.0, Color32::BLACK),
            );
        }
    }

    /// The strokes rasterised at most LAYER_SIDE px on the long side, redrawn when they changed;
    /// none without strokes.
    fn layer(&mut self, ctx: &egui::Context, viewer: &Viewer) -> Option<TextureHandle> {
        if self.strokes.is_empty() {
            return None;
        }
        let drawn = {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            for s in &self.strokes {
                (s.sky, s.radius.to_bits()).hash(&mut hash);
                for p in &s.points {
                    (p[0].to_bits(), p[1].to_bits()).hash(&mut hash);
                }
            }
            hash.finish()
        };
        if let Some((from, texture)) = &self.layer
            && *from == drawn
        {
            return Some(texture.clone());
        }
        let (w, h) = (viewer.scene.width as usize, viewer.scene.height as usize);
        let step = w.max(h).div_ceil(LAYER_SIDE).max(1);
        let (lw, lh) = (w.div_ceil(step), h.div_ceil(step));
        let k = step as f32;
        let scaled: Vec<Stroke> = self
            .strokes
            .iter()
            .map(|s| Stroke {
                sky: s.sky,
                radius: s.radius / k,
                points: s.points.iter().map(|p| [p[0] / k, p[1] / k]).collect(),
            })
            .collect();
        let pixels = force(&scaled, lw, lh)
            .into_iter()
            .map(|f| match f {
                1 => SKY_PAINT,
                -1 => GROUND_PAINT,
                _ => Color32::TRANSPARENT,
            })
            .collect();
        let image = ColorImage::new([lw, lh], pixels);
        let texture = match self.layer.take() {
            Some((_, mut t)) => {
                t.set(image, egui::TextureOptions::LINEAR);
                t
            }
            None => ctx.load_texture("corrections", image, egui::TextureOptions::LINEAR),
        };
        self.layer = Some((drawn, texture.clone()));
        Some(texture)
    }
}
