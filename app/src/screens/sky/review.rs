//! Regions to check: where the colour check made light of its own colour ground (lamp-lit
//! leaves, where no star crossed), tinted over the result on request and listed largest first,
//! each zoomed to with Show. Read from the matte in the background.

use crate::job::Job;
use crate::viewer::{Viewer, Zoom};
use crate::widgets::{self, card, section};
use egui::{Color32, ColorImage, Rect, Response, TextureHandle, Ui, pos2};
use pictor_engine::separate::filters::components;
use pictor_engine::store::planes;
use std::path::Path;

const TINT: Color32 = Color32::from_rgba_premultiplied(110, 30, 110, 120);
/// Longest side of the tinted layer, px.
const LAYER_SIDE: usize = 4096;
const LISTED: usize = 8;
/// Regions under this share of the picture are left out of the list.
const MIN_SHARE: f64 = 0.0002;

pub struct Region {
    /// Left, top, right and bottom edges in scene pixels.
    pub bounds: [usize; 4],
    pub pixels: usize,
}

pub struct Found {
    layer: ColorImage,
    regions: Vec<Region>,
    lit_pixels: usize,
    total: usize,
}

/// The lit plane of `store`'s matte as a tinted layer and its largest regions.
fn find(store: &Path) -> pictor_engine::Result<Found> {
    let p = planes::load(&store.join("matte.planes"))?;
    let (w, h) = (p.width, p.height);
    let lit: Vec<bool> = p
        .get("lit")
        .map_err(|_| {
            pictor_engine::error(
                "This run was separated before Pictor kept these regions; applying corrections \
                 draws them again.",
            )
        })?
        .iter()
        .map(|&v| v > 0.5)
        .collect();
    let (sizes, label) = components(&lit, w, h);
    let mut bounds = vec![[usize::MAX, usize::MAX, 0, 0]; sizes.len()];
    for (q, &on) in lit.iter().enumerate() {
        if on {
            let (x, y, b) = (q % w, q / w, &mut bounds[label[q]]);
            *b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
        }
    }
    let least = (MIN_SHARE * (w * h) as f64) as usize;
    let mut regions: Vec<Region> = sizes
        .iter()
        .zip(bounds)
        .filter(|(n, _)| **n >= least)
        .map(|(&pixels, bounds)| Region { bounds, pixels })
        .collect();
    regions.sort_by_key(|r| std::cmp::Reverse(r.pixels));
    regions.truncate(LISTED);
    let step = w.max(h).div_ceil(LAYER_SIDE).max(1);
    let (lw, lh) = (w.div_ceil(step), h.div_ceil(step));
    let mut pixels = vec![Color32::TRANSPARENT; lw * lh];
    for (q, &on) in lit.iter().enumerate() {
        if on {
            pixels[(q / w / step) * lw + (q % w) / step] = TINT;
        }
    }
    Ok(Found {
        layer: ColorImage::new([lw, lh], pixels),
        regions,
        lit_pixels: lit.iter().filter(|&&l| l).count(),
        total: w * h,
    })
}

#[derive(Default)]
pub struct Review {
    reading: Option<Job<Found>>,
    found: Option<Found>,
    texture: Option<TextureHandle>,
    pub show: bool,
    error: Option<String>,
}

impl Review {
    /// Reads `store`'s matte in the background.
    pub fn start(store: &Path) -> Review {
        let dir = store.to_path_buf();
        Review {
            reading: Some(Job::start("Finding regions to check", move |_| find(&dir))),
            ..Default::default()
        }
    }

    pub fn reading(&self) -> bool {
        self.reading.is_some()
    }

    /// Takes the regions once read.
    pub fn poll(&mut self) {
        if let Some(done) = self.reading.as_ref().and_then(|j| j.poll()) {
            self.reading = None;
            match done {
                Ok(f) => self.found = Some(f),
                Err(e) => self.error = Some(e.to_string()),
            }
        }
    }

    /// The tinted regions over the canvas `response`, when shown.
    pub fn draw(&mut self, ui: &Ui, response: &Response, viewer: &Viewer) {
        let (Some(found), true) = (&self.found, self.show) else {
            return;
        };
        let texture = self.texture.get_or_insert_with(|| {
            ui.ctx()
                .load_texture("regions", found.layer.clone(), egui::TextureOptions::LINEAR)
        });
        let (rect, ppp) = (response.rect, ui.ctx().pixels_per_point());
        let (w, h) = (viewer.scene.width as f32, viewer.scene.height as f32);
        let shown = Rect::from_min_max(
            viewer.screen_at(rect, ppp, [0.0, 0.0]),
            viewer.screen_at(rect, ppp, [w, h]),
        );
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter_at(rect)
            .image(texture.id(), shown, uv, Color32::WHITE);
    }

    /// The "Regions to check" card.
    pub fn panel(&mut self, ui: &mut Ui, viewer: &mut Viewer) {
        card(ui, |ui| {
            section(ui, "Regions to check");
            if let Some(e) = &self.error {
                ui.label(widgets::muted(e));
                return;
            }
            let Some(found) = &self.found else {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(widgets::muted("Reading the matte…"));
                });
                return;
            };
            if found.regions.is_empty() {
                ui.label(widgets::muted(
                    "The colour check made no sizeable region ground here.",
                ));
                return;
            }
            ui.label(widgets::muted(format!(
                "Light of its own colour where no star crossed became ground: {:.1}% of the \
                 picture. Check that each is foreground; paint sky where it is not.",
                found.lit_pixels as f64 / found.total as f64 * 100.0
            )));
            if widgets::toggle(ui, self.show, "Tint them", None).clicked() {
                self.show = !self.show;
            }
            for (k, r) in found.regions.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Region {} · {}", k + 1, widgets::pixels(r.pixels)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Show").clicked() {
                            let [x0, y0, x1, y1] = r.bounds.map(|v| v as f32);
                            viewer.centre =
                                viewer.rendering_point([(x0 + x1) / 2.0, (y0 + y1) / 2.0]);
                            let span = viewer.rendering_point([x1 - x0, y1 - y0]);
                            let fit = 900.0 / span[0].max(span[1]).max(1.0);
                            viewer.zoom = Zoom::Factor(fit.clamp(0.1, 2.0));
                            self.show = true;
                        }
                    });
                });
            }
        });
    }
}
