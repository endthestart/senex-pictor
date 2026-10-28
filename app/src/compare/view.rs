//! The Compare canvas and crops: the main view (swap, split line, labels, pan and zoom) and the
//! region crops, every one drawn from the same two GPU-resident renderings.

use super::Compare;
use crate::gpu::canvas::{self, Draw, View};
use egui::{Align2, Color32, FontId, Rect, Sense, Stroke, vec2};

/// Zoom of the region crops, physical px per image px.
const CROP_ZOOM: f32 = 2.0;
pub const CROP_POINTS: f32 = 150.0;
/// Uniform slots: 0 main, then two per crop.
const FIRST_CROP_SLOT: u64 = 1;

#[derive(Clone, Copy, PartialEq)]
pub enum Zoom {
    Fit,
    Factor(f32),
}

impl Compare {
    fn image(&self) -> [f32; 2] {
        [self.size[0] as f32, self.size[1] as f32]
    }

    pub fn zoom_factor(&self, size: [f32; 2]) -> f32 {
        let img = self.image();
        match self.zoom {
            Zoom::Fit => (size[0] / img[0]).min(size[1] / img[1]),
            Zoom::Factor(z) => z,
        }
    }

    fn label(&self, k: usize) -> String {
        format!("{} · {}", ["A", "B"][k], self.names[k])
    }

    /// The main canvas over the panel's whole area.
    pub fn canvas(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let ctx = ui.ctx().clone();
        let ppp = ctx.pixels_per_point();
        let size = canvas::viewport_px(rect, ppp);
        let handle = self.split.map(|f| {
            let x = rect.min.x + f * rect.width();
            Rect::from_min_max(
                egui::pos2(x - 6.0, rect.min.y),
                egui::pos2(x + 6.0, rect.max.y),
            )
        });
        let response = ui.interact(rect, ui.id().with("canvas"), Sense::drag());
        if let Some(h) = handle {
            let line = ui.interact(h, ui.id().with("split"), Sense::drag());
            if line.dragged() {
                let x = line.interact_pointer_pos().map_or(h.center().x, |p| p.x);
                self.split = Some(((x - rect.min.x) / rect.width()).clamp(0.02, 0.98));
            }
        }
        if self.centre == [0.0; 2] {
            self.centre = self.image().map(|v| v / 2.0);
        }
        let z = self.zoom_factor(size);
        if response.dragged() {
            let d = response.drag_delta() * ppp / z;
            self.centre = [self.centre[0] - d.x, self.centre[1] - d.y];
        }
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 && response.hovered() {
            self.zoom = Zoom::Factor((z * (scroll / 200.0).exp()).clamp(0.02, 8.0));
        }
        let z = self.zoom_factor(size);
        let (lo, hi) = canvas::place(size, self.image(), z, self.centre);
        let mut view = View::image(lo, hi, self.display.scale, self.display.mode, self.swap);
        let painter = ui.painter();
        if let Some(f) = self.split {
            view.split = ((rect.min.x + f * rect.width()) * ppp).round();
        }
        painter.add(canvas::shape(
            rect,
            Draw {
                view,
                slot: 0,
                nearest: z >= 2.0,
            },
        ));
        let shown = usize::from(self.swap);
        let tag = |pos: egui::Pos2, align: Align2, text: String| {
            let galley = painter.layout_no_wrap(text, FontId::proportional(18.0), Color32::WHITE);
            let at = align.anchor_size(pos, galley.size());
            painter.rect_filled(at.expand(6.0), 4.0, Color32::from_black_alpha(150));
            painter.galley(at.min, galley, Color32::WHITE);
        };
        match self.split {
            None => tag(
                rect.left_top() + vec2(16.0, 14.0),
                Align2::LEFT_TOP,
                self.label(shown),
            ),
            Some(f) => {
                let x = rect.min.x + f * rect.width();
                painter.line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    Stroke::new(2.0, Color32::from_white_alpha(200)),
                );
                tag(
                    egui::pos2(x - 12.0, rect.min.y + 14.0),
                    Align2::RIGHT_TOP,
                    self.label(shown),
                );
                tag(
                    egui::pos2(x + 12.0, rect.min.y + 14.0),
                    Align2::LEFT_TOP,
                    self.label(1 - shown),
                );
            }
        }
        let zoom_text = format!("{:.0}%", z * 100.0);
        tag(
            rect.right_bottom() - vec2(16.0, 14.0),
            Align2::RIGHT_BOTTOM,
            zoom_text,
        );
    }

    /// Crops of region `r` from both renderings; returns whether one was clicked.
    pub fn crops(&self, ui: &mut egui::Ui, r: usize) -> bool {
        let ppp = ui.ctx().pixels_per_point();
        let mut clicked = false;
        ui.horizontal(|ui| {
            for k in 0..2 {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(CROP_POINTS, CROP_POINTS), Sense::click());
                let size = canvas::viewport_px(rect, ppp);
                let (lo, hi) = canvas::place(size, self.image(), CROP_ZOOM, self.regions[r].centre);
                let view = View::image(lo, hi, self.display.scale, self.display.mode, k == 1);
                let slot = FIRST_CROP_SLOT + 2 * r as u64 + k as u64;
                ui.painter().add(canvas::shape(
                    rect,
                    Draw {
                        view,
                        slot,
                        nearest: true,
                    },
                ));
                ui.painter().text(
                    rect.left_top() + vec2(6.0, 4.0),
                    Align2::LEFT_TOP,
                    ["A", "B"][k],
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );
                clicked |= response.clicked();
            }
        });
        clicked
    }
}
