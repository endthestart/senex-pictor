//! The Look & export step (design board 6): the look shown as A, chosen from the presets and
//! tuned with a few guided controls (one "More settings" level), recomputed on the GPU as they
//! move; a check that the foreground does not render brighter than the sky; and export of the
//! editable linear master, the display image as a 16-bit TIFF, or a JPEG.

mod controls;
mod export;

use crate::job::Job;
use crate::project::Run;
use crate::theme::{GOOD, MUTED, WARN};
use crate::viewer::{Canvas, Viewer};
use crate::widgets::{self, Badge, card, section};
use controls::{BLACK, Control, SATURATION, SHADOWS, STRETCH, WHITE};
use egui::{RichText, Ui};
use export::Format;
use half::f16;
use pictor_engine::render::{self, Look};

const PRESETS: [(&str, &str, Look); 2] = [
    (
        "Natural",
        "Foreground as shot, sky stretched",
        render::NATURAL,
    ),
    (
        "Dark",
        "Deeper blacks, gentle stretch, glow kept",
        render::DARK,
    ),
];

pub struct LookStep {
    pub canvas: Canvas,
    more: bool,
    format: Format,
    /// A look waiting for a slider's release before its levels are measured again.
    pending: Option<Look>,
    guard: Option<(f32, f32)>,
    export: Option<Job<Vec<String>>>,
    exported: Vec<String>,
    error: Option<String>,
}

impl Default for LookStep {
    fn default() -> LookStep {
        LookStep {
            canvas: Canvas::single(false),
            more: false,
            format: Format::Master,
            pending: None,
            guard: None,
            export: None,
            exported: Vec::new(),
            error: None,
        }
    }
}

/// Median display luminance of the ground and of the clear sky under rendering `k`'s look,
/// from a sample of the scene, as the look shader computes it.
fn guard(viewer: &Viewer, k: usize) -> Option<(f32, f32)> {
    let s = &viewer.scene;
    let p = &viewer.params[k];
    let at = |plane: &[u16], q: usize, c: usize| f16::from_bits(plane[4 * q + c]).to_f32();
    let encode = |v: f32| render::encode(v.clamp(0.0, 1.0));
    let (mut ground, mut sky) = (Vec::new(), Vec::new());
    for q in (0..s.clear.len()).step_by(61) {
        let a = at(&s.base, q, 3);
        let c: [f32; 3] =
            std::array::from_fn(|i| at(&s.base, q, i) - p.remove_glow * at(&s.glow, q, i));
        let d: [f32; 3] = std::array::from_fn(|i| c[i] - p.black[i]);
        let l = 0.2126 * d[0] + 0.7152 * d[1] + 0.0722 * d[2];
        let shown: [f32; 3] = std::array::from_fn(|i| {
            let sv = l + p.saturation * (d[i] - l);
            let y = (p.stretch * (sv / p.scale).max(0.0)).asinh() / p.norm;
            a * encode(y.max(0.0).powf(p.shadows)) + (1.0 - a) * encode(c[i] / p.white)
        });
        let lum = 0.2126 * shown[0] + 0.7152 * shown[1] + 0.0722 * shown[2];
        if a <= 0.02 {
            ground.push(lum);
        } else if s.clear[q] {
            sky.push(lum);
        }
    }
    let median = |v: &mut Vec<f32>| {
        let n = v.len();
        (n > 0).then(|| *v.select_nth_unstable_by(n / 2, f32::total_cmp).1)
    };
    Some((median(&mut ground)?, median(&mut sky)?))
}

impl LookStep {
    pub fn exporting(&self) -> bool {
        self.export.is_some()
    }

    pub fn detail(&self, viewer: &Viewer) -> String {
        if self.exported.is_empty() {
            viewer.names[0].clone()
        } else {
            format!("{} · exported", viewer.names[0])
        }
    }

    pub fn show(&mut self, ui: &mut Ui, viewer: &mut Viewer) {
        viewer.canvas(ui, &mut self.canvas);
    }

    fn apply(&mut self, viewer: &mut Viewer, look: Look) {
        if let Err(e) = viewer.set_look(0, look) {
            self.error = Some(e.to_string());
        }
        if !PRESETS.iter().any(|(_, _, p)| *p == look) {
            viewer.names[0] = "Custom".into();
        }
        self.guard = guard(viewer, 0);
    }

    fn slider(&mut self, ui: &mut Ui, viewer: &mut Viewer, c: Control) {
        let mut v = (c.get)(&self.pending.unwrap_or(viewer.look[0]));
        ui.label(c.label);
        ui.spacing_mut().slider_width = ui.available_width() - 64.0;
        let r = ui.add(egui::Slider::new(&mut v, c.range).max_decimals(2));
        if r.changed() {
            let mut look = self.pending.unwrap_or(viewer.look[0]);
            (c.set)(&mut look, v);
            if c.relevel && r.dragged() {
                self.pending = Some(look);
            } else {
                self.pending = None;
                self.apply(viewer, look);
            }
        }
        if r.drag_stopped()
            && let Some(look) = self.pending.take()
        {
            self.apply(viewer, look);
        }
    }

    pub fn panel(
        &mut self,
        ui: &mut Ui,
        viewer: &mut Viewer,
        run: Option<&Run>,
        result: Option<&str>,
    ) {
        ui.spacing_mut().item_spacing.y = 10.0;
        if self.guard.is_none() {
            self.guard = guard(viewer, 0);
        }
        card(ui, |ui| {
            section(ui, "Look");
            for (name, note, look) in PRESETS {
                let on = viewer.names[0] == name;
                if widgets::choice(ui, on, name, note).clicked() && !on {
                    self.apply(viewer, look);
                    viewer.names[0] = name.into();
                }
            }
            if !PRESETS.iter().any(|(n, _, _)| *n == viewer.names[0]) {
                widgets::choice(ui, true, &viewer.names[0], "Your changes to a preset");
            }
        });
        let l = self.pending.unwrap_or(viewer.look[0]);
        card(ui, |ui| {
            section(ui, "Sky");
            for c in [BLACK, STRETCH, SATURATION] {
                self.slider(ui, viewer, c);
            }
            ui.label(RichText::new("Foreground: as shot").color(MUTED));
            let label = if self.more {
                "Fewer settings"
            } else {
                "More settings"
            };
            if ui.link(label).clicked() {
                self.more = !self.more;
            }
            if self.more {
                for c in [SHADOWS, WHITE] {
                    self.slider(ui, viewer, c);
                }
                let mut remove = l.remove_glow;
                if ui
                    .checkbox(&mut remove, "Remove the camera-fixed glow")
                    .changed()
                {
                    self.apply(
                        viewer,
                        Look {
                            remove_glow: remove,
                            ..l
                        },
                    );
                }
            }
            if let Some(ms) = viewer.recompute_ms {
                ui.label(
                    RichText::new(format!("Recomputed on the GPU in {ms:.0} ms"))
                        .size(12.0)
                        .color(MUTED),
                );
            }
        });
        if let Some((ground, sky)) = self.guard {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    section(ui, "Realism");
                    widgets::badge(ui, "Proposal", Badge::Quiet);
                });
                if ground > sky {
                    ui.label(RichText::new("The foreground renders brighter than the sky behind it; that reads as artificial at night.").color(WARN));
                } else {
                    ui.label(
                        RichText::new("The foreground stays darker than the sky, as at night.")
                            .color(GOOD),
                    );
                }
            });
        }
        self.export_card(ui, viewer, run, result);
        if let Some(e) = &self.error {
            ui.colored_label(WARN, e);
        }
    }
}
