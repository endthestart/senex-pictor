//! Drawing the viewer's renderings: the main canvas (pan, zoom, an optional split line, A/B
//! tags, the sky and ground overlay) and fixed-zoom crops, every one a paint callback on the
//! same two GPU-resident renderings.

use super::Viewer;
use crate::gpu::canvas::{self, Draw, MODE_OVERLAY, View};
use egui::{Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, vec2};

#[derive(Clone, Copy, PartialEq)]
pub enum Zoom {
    Fit,
    Factor(f32),
}

/// How a screen shows the viewer.
#[derive(Clone, Copy)]
pub struct Canvas {
    /// Shows rendering B (or, with a split, B on the left).
    pub swap: bool,
    /// Where the split line sits (share of the width); none shows one rendering.
    pub split: Option<f32>,
    pub overlay: bool,
    /// A/B name tags.
    pub tags: bool,
    /// A brush paints with the primary button; the others (or Space) pan.
    pub paint: bool,
    /// Uniform slot (0 for the main canvas).
    pub slot: u64,
}

impl Canvas {
    pub fn single(swap: bool) -> Canvas {
        Canvas {
            swap,
            split: None,
            overlay: false,
            tags: false,
            paint: false,
            slot: 0,
        }
    }
}

fn tag(painter: &egui::Painter, pos: egui::Pos2, align: Align2, text: String) {
    let galley = painter.layout_no_wrap(text, FontId::proportional(14.0), Color32::WHITE);
    let at = align.anchor_size(pos, galley.size());
    painter.rect_filled(
        at.expand2(vec2(8.0, 4.0)),
        6.0,
        Color32::from_black_alpha(160),
    );
    painter.galley(at.min, galley, Color32::WHITE);
}

impl Viewer {
    fn label(&self, k: usize) -> String {
        format!("{} · {}", ["A", "B"][k], self.key(k))
    }

    /// The canvas over the whole of `ui`'s area: drag to pan, scroll to zoom, drag the split
    /// line when there is one.
    pub fn canvas(&mut self, ui: &mut Ui, c: &mut Canvas) -> Response {
        let rect = ui.max_rect();
        let ctx = ui.ctx().clone();
        let ppp = ctx.pixels_per_point();
        let size = canvas::viewport_px(rect, ppp);
        let response = ui.interact(rect, ui.id().with("canvas"), Sense::click_and_drag());
        if let Some(f) = c.split {
            let x = rect.min.x + f * rect.width();
            let handle = Rect::from_min_max(
                egui::pos2(x - 6.0, rect.min.y),
                egui::pos2(x + 6.0, rect.max.y),
            );
            let line = ui.interact(handle, ui.id().with("split"), Sense::drag());
            if line.dragged() {
                let x = line
                    .interact_pointer_pos()
                    .map_or(handle.center().x, |p| p.x);
                c.split = Some(((x - rect.min.x) / rect.width()).clamp(0.02, 0.98));
            }
            if line.hovered() || line.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }
        let z = self.zoom_factor(size);
        let space = ctx.input(|i| i.key_down(egui::Key::Space));
        let pan = if c.paint {
            response.dragged_by(egui::PointerButton::Secondary)
                || response.dragged_by(egui::PointerButton::Middle)
                || (space && response.dragged())
        } else {
            response.dragged()
        };
        if pan {
            let d = response.drag_delta() * ppp / z;
            self.centre = [self.centre[0] - d.x, self.centre[1] - d.y];
        }
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 && response.hovered() {
            self.zoom = Zoom::Factor((z * (scroll / 200.0).exp()).clamp(0.02, 8.0));
        }
        let z = self.zoom_factor(size);
        let (lo, hi) = canvas::place(size, self.image(), z, self.centre);
        let mut mode = self.display.mode;
        if c.overlay {
            mode |= MODE_OVERLAY;
        }
        let mut view = View::image(lo, hi, self.display.scale, mode, c.swap);
        if let Some(f) = c.split {
            view.split = ((rect.min.x + f * rect.width()) * ppp).round();
        }
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        painter.add(canvas::shape(
            rect,
            Draw {
                view,
                slot: c.slot,
                nearest: z >= 2.0,
            },
        ));
        let shown = usize::from(c.swap);
        match (c.split, c.tags) {
            (None, true) => tag(
                painter,
                rect.left_top() + vec2(16.0, 14.0),
                Align2::LEFT_TOP,
                self.label(shown),
            ),
            (Some(f), _) => {
                let x = rect.min.x + f * rect.width();
                painter.line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    Stroke::new(2.0, Color32::from_white_alpha(200)),
                );
                tag(
                    painter,
                    egui::pos2(x - 12.0, rect.min.y + 14.0),
                    Align2::RIGHT_TOP,
                    self.label(shown),
                );
                tag(
                    painter,
                    egui::pos2(x + 12.0, rect.min.y + 14.0),
                    Align2::LEFT_TOP,
                    self.label(1 - shown),
                );
            }
            (None, false) => {}
        }
        tag(
            painter,
            rect.right_bottom() - vec2(16.0, 14.0),
            Align2::RIGHT_BOTTOM,
            format!("{:.0}%", z * 100.0),
        );
        response
    }

    /// Physical px per rendering px, and where the rendering's top-left corner falls (physical
    /// px from the canvas's), on a canvas over `rect`.
    fn placement(&self, rect: Rect, ppp: f32) -> (f32, [f32; 2]) {
        let size = canvas::viewport_px(rect, ppp);
        let z = self.zoom_factor(size);
        (z, canvas::origin(size, z, self.centre))
    }

    fn scene_per_rendering(&self) -> [f32; 2] {
        [
            self.scene.width as f32 / self.size[0] as f32,
            self.scene.height as f32 / self.size[1] as f32,
        ]
    }

    /// The scene pixel under screen point `pos` of a canvas over `rect`.
    pub fn scene_at(&self, rect: Rect, ppp: f32, pos: egui::Pos2) -> [f32; 2] {
        let (z, o) = self.placement(rect, ppp);
        let k = self.scene_per_rendering();
        [
            ((pos.x - rect.min.x) * ppp - o[0]) / z * k[0],
            ((pos.y - rect.min.y) * ppp - o[1]) / z * k[1],
        ]
    }

    /// The screen point of scene pixel `p` on a canvas over `rect`.
    pub fn screen_at(&self, rect: Rect, ppp: f32, p: [f32; 2]) -> egui::Pos2 {
        let (z, o) = self.placement(rect, ppp);
        let k = self.scene_per_rendering();
        egui::pos2(
            rect.min.x + (o[0] + p[0] / k[0] * z) / ppp,
            rect.min.y + (o[1] + p[1] / k[1] * z) / ppp,
        )
    }

    /// Screen points per scene pixel on a canvas over `rect`.
    pub fn points_per_scene(&self, rect: Rect, ppp: f32) -> f32 {
        self.placement(rect, ppp).0 / self.scene_per_rendering()[0] / ppp
    }

    /// Rendering `k` around `centre` (rendering px) at `zoom` physical px per px, in a square
    /// of `points`, drawn with uniform `slot`.
    pub fn crop(
        &self,
        ui: &mut Ui,
        centre: [f32; 2],
        zoom: f32,
        k: usize,
        slot: u64,
        points: f32,
    ) -> Response {
        let ppp = ui.ctx().pixels_per_point();
        let (rect, response) = ui.allocate_exact_size(vec2(points, points), Sense::click());
        let size = canvas::viewport_px(rect, ppp);
        let (lo, hi) = canvas::place(size, self.image(), zoom, centre);
        let view = View::image(lo, hi, self.display.scale, self.display.mode, k == 1);
        ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
        ui.painter().add(canvas::shape(
            rect,
            Draw {
                view,
                slot,
                nearest: true,
            },
        ));
        response
    }
}
