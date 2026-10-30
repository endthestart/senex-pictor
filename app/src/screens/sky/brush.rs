//! Correcting the matte by hand: brushes that paint sky or ground over the result in scene
//! (store) pixels, drawn as translucent strokes, with undo. Move (V) pans; Sky (S) and Ground
//! (G) paint with the primary button while the others, or Space, pan; [ and ] size the brush.

use crate::viewer::Viewer;
use egui::{Color32, Key, PointerButton, Response, Stroke as Line, Ui};
use pictor_engine::separate::corrections::Stroke;

const SKY_PAINT: Color32 = Color32::from_rgba_premultiplied(30, 56, 96, 110);
const GROUND_PAINT: Color32 = Color32::from_rgba_premultiplied(120, 56, 52, 110);
pub const RADIUS_RANGE: std::ops::RangeInclusive<f32> = 4.0..=400.0;

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
}

impl Brush {
    pub fn new(saved: Vec<Stroke>) -> Brush {
        Brush {
            tool: Tool::Move,
            radius: 40.0,
            strokes: saved.clone(),
            saved,
            drawing: false,
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
        for s in &self.strokes {
            let colour = if s.sky { SKY_PAINT } else { GROUND_PAINT };
            let r = s.radius * scale;
            let points: Vec<egui::Pos2> = s
                .points
                .iter()
                .map(|&p| viewer.screen_at(rect, ppp, p))
                .collect();
            for pair in points.windows(2) {
                painter.line_segment([pair[0], pair[1]], Line::new(2.0 * r, colour));
            }
            for &p in &points {
                painter.circle_filled(p, r, colour);
            }
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
}
