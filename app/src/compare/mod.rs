//! The Compare step (design board 5), the hub every step returns to: two renderings of the
//! same result, both on the GPU. Space swaps them in place with nothing in between; S splits
//! the view with a draggable line; the side panel picks each side's look and shows regions
//! chosen from the scene as 200% crops of both, each with an A / same / B verdict saved in the
//! project folder.

mod regions;
mod verdict;

use crate::theme::MUTED;
use crate::viewer::{Canvas, Viewer, Zoom};
use crate::widgets::{self, card, section, strong};
use egui::{RichText, Ui};
use pictor_engine::Result;
use pictor_engine::render::{self, Look};
use regions::Region;
use std::path::PathBuf;
use verdict::{Choice, Verdicts};

/// Zoom of the region crops, physical px per rendering px.
const CROP_ZOOM: f32 = 2.0;

/// Uniform slots: 0 main, then two per crop.
const FIRST_CROP_SLOT: u64 = 1;

/// The looks a side can show.
const PRESETS: [(&str, Look); 2] = [("Natural", render::NATURAL), ("Dark", render::DARK)];

pub struct Compare {
    pub canvas: Canvas,
    regions: Vec<Region>,
    verdicts: Verdicts,
    dir: Option<PathBuf>,
    pub error: Option<String>,
}

impl Compare {
    pub fn new(viewer: &Viewer, dir: Option<PathBuf>) -> Result<Compare> {
        let verdicts = Verdicts::load(dir.as_deref(), &viewer.names[0], &viewer.names[1])?;
        Ok(Compare {
            canvas: Canvas {
                tags: true,
                ..Canvas::single(false)
            },
            regions: regions::choose(&viewer.scene),
            verdicts,
            dir,
            error: None,
        })
    }

    pub fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.canvas.swap = !self.canvas.swap;
            }
            if i.key_pressed(egui::Key::S) {
                self.canvas.split = match self.canvas.split {
                    Some(_) => None,
                    None => Some(0.5),
                };
            }
        });
    }

    /// A scripted step `t` (0–1) of a benchmark run: a third each at fit, 100% and 200%,
    /// panning across, swapping every 15 frames. Returns the phase.
    pub fn script(&mut self, viewer: &mut Viewer, t: f64, frame: usize) -> &'static str {
        let sweep = ((t * 3.0).fract() as f32 * std::f32::consts::TAU).sin() * 0.4 + 0.5;
        if frame.is_multiple_of(15) {
            self.canvas.swap = !self.canvas.swap;
        }
        let (zoom, name) = match (t * 3.0) as usize {
            0 => (Zoom::Fit, "fit"),
            1 => (Zoom::Factor(1.0), "100%"),
            _ => (Zoom::Factor(2.0), "200%"),
        };
        viewer.zoom = zoom;
        viewer.centre = [viewer.size[0] as f32 * sweep, viewer.size[1] as f32 * 0.5];
        name
    }

    pub fn show(&mut self, ui: &mut Ui, viewer: &mut Viewer) {
        viewer.canvas(ui, &mut self.canvas);
    }

    fn side(&mut self, ui: &mut Ui, viewer: &mut Viewer, k: usize) {
        ui.horizontal(|ui| {
            ui.label(strong(["A", "B"][k]));
            let names: Vec<&str> = PRESETS.iter().map(|(n, _)| *n).collect();
            let selected = names.iter().position(|n| *n == viewer.names[k]);
            if let Some(i) = widgets::segmented(ui, &names, selected)
                && Some(i) != selected
            {
                let (name, look) = PRESETS[i];
                if let Err(e) = viewer.set_named(k, name, look) {
                    self.error = Some(e.to_string());
                }
                match Verdicts::load(self.dir.as_deref(), &viewer.names[0], &viewer.names[1]) {
                    Ok(v) => self.verdicts = v,
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            if selected.is_none() {
                ui.label(RichText::new(&viewer.names[k]).color(MUTED));
            }
        });
    }

    pub fn panel(&mut self, ui: &mut Ui, viewer: &mut Viewer) {
        ui.spacing_mut().item_spacing.y = 10.0;
        card(ui, |ui| {
            section(ui, "Renderings");
            self.side(ui, viewer, 0);
            self.side(ui, viewer, 1);
            ui.add_space(4.0);
            widgets::key_hints(
                ui,
                &[
                    ("Space", "swap A and B"),
                    ("S", "split"),
                    ("F", "fit"),
                    ("1  2", "100%  200%"),
                ],
            );
            if let Some(e) = &self.error {
                ui.colored_label(crate::theme::WARN, e);
            }
        });
        let [a, same, b] = self.verdicts.tally();
        card(ui, |ui| {
            section(ui, "Where to look");
            ui.label(widgets::muted(format!(
                "A better {a} · same {same} · B better {b}{}",
                if self.verdicts.saved() {
                    ""
                } else {
                    " · not saved without a project folder"
                }
            )));
        });
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for r in 0..self.regions.len() {
                    self.region(ui, viewer, r);
                }
            });
    }

    fn region(&mut self, ui: &mut Ui, viewer: &mut Viewer, r: usize) {
        let centre = viewer.rendering_point(self.regions[r].centre);
        let name = self.regions[r].name;
        card(ui, |ui| {
            ui.label(strong(name));
            ui.label(widgets::muted(self.regions[r].look_for));
            let mut clicked = false;
            let side = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
            ui.horizontal(|ui| {
                for k in 0..2 {
                    let slot = FIRST_CROP_SLOT + 2 * r as u64 + k as u64;
                    let response = viewer.crop(ui, centre, CROP_ZOOM, k, slot, side);
                    ui.painter().text(
                        response.rect.left_top() + egui::vec2(6.0, 4.0),
                        egui::Align2::LEFT_TOP,
                        ["A", "B"][k],
                        egui::FontId::proportional(13.0),
                        egui::Color32::WHITE,
                    );
                    clicked |= response.clicked();
                }
            });
            if clicked {
                viewer.centre = centre;
                viewer.zoom = Zoom::Factor(CROP_ZOOM);
            }
            let choices = [Choice::First, Choice::Same, Choice::Second];
            let selected = choices
                .iter()
                .position(|c| self.verdicts.get(name) == Some(*c));
            if let Some(i) = widgets::segmented(ui, &["A better", "Same", "B better"], selected)
                && let Err(e) = self.verdicts.set(name, choices[i])
            {
                self.error = Some(e.to_string());
            }
        });
    }

    /// Regions judged so far.
    pub fn judged(&self) -> usize {
        self.verdicts.tally().iter().sum()
    }
}
