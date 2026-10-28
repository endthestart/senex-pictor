//! The Compare screen (design board 5): two renderings of the same stack, both on the GPU.
//! Space swaps them in place with nothing in between; S splits the view with a draggable line;
//! the right side shows regions chosen from the scene as 200% crops of both, each with an
//! A / same / B verdict saved in the project folder; the left side tunes each look live.

mod regions;
mod verdict;
mod view;

use crate::display::Display;
use crate::gpu::canvas::CanvasGpu;
use crate::gpu::look::Looks;
use crate::project::Project;
use crate::scene::LookParams;
use regions::Region;
use verdict::{Choice, Verdicts};
use view::Zoom;

pub struct Compare {
    device: wgpu::Device,
    queue: wgpu::Queue,
    looks: Looks,
    params: [LookParams; 2],
    defaults: [LookParams; 2],
    names: [&'static str; 2],
    size: [u32; 2],
    display: Display,
    swap: bool,
    split: Option<f32>,
    zoom: Zoom,
    centre: [f32; 2],
    regions: Vec<Region>,
    verdicts: Verdicts,
    pub name: String,
    pub facts: Vec<String>,
    recompute_ms: Option<f64>,
    pub error: Option<String>,
}

impl Compare {
    /// Uploads `project` to the GPU (at `mp` megapixels if given), renders both looks and
    /// registers the canvas.
    pub fn new(
        rs: &eframe::egui_wgpu::RenderState,
        display: Display,
        project: Project,
        mp: Option<f64>,
    ) -> star_photos::Result<Compare> {
        let scene = &project.scene;
        let size = crate::scene::output_size(scene, mp);
        let (device, queue) = (rs.device.clone(), rs.queue.clone());
        let looks = Looks::new(&device, &queue, scene, size)?;
        for (k, p) in scene.looks.iter().enumerate() {
            looks.apply(&device, &queue, k, p)?;
        }
        rs.renderer
            .write()
            .callback_resources
            .insert(CanvasGpu::new(&device, rs.target_format, &looks.outputs));
        let k = [
            size[0] as f32 / scene.width as f32,
            size[1] as f32 / scene.height as f32,
        ];
        let mut regions = regions::choose(scene);
        for r in &mut regions {
            r.centre = [r.centre[0] * k[0], r.centre[1] * k[1]];
        }
        let verdicts = Verdicts::load(project.dir.as_deref(), scene.names[0], scene.names[1])?;
        Ok(Compare {
            device,
            queue,
            looks,
            params: scene.looks,
            defaults: scene.looks,
            names: scene.names,
            size,
            display,
            swap: false,
            split: None,
            zoom: Zoom::Fit,
            centre: [0.0; 2],
            regions,
            verdicts,
            name: project.name,
            facts: project.facts,
            recompute_ms: None,
            error: None,
        })
    }

    pub fn keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.swap = !self.swap;
            }
            if i.key_pressed(egui::Key::S) {
                self.split = if self.split.is_some() {
                    None
                } else {
                    Some(0.5)
                };
            }
            if i.key_pressed(egui::Key::F) {
                self.zoom = Zoom::Fit;
            }
            for (key, z) in [(egui::Key::Num1, 1.0), (egui::Key::Num2, 2.0)] {
                if i.key_pressed(key) {
                    self.zoom = Zoom::Factor(z);
                }
            }
        });
    }

    /// A scripted step `t` (0–1) of a benchmark run: a third each at fit, 100% and 200%, panning
    /// across, swapping every 15 frames. Returns the phase.
    pub fn script(&mut self, t: f64, frame: usize) -> &'static str {
        let sweep = ((t * 3.0).fract() as f32 * std::f32::consts::TAU).sin() * 0.4 + 0.5;
        if frame.is_multiple_of(15) {
            self.swap = !self.swap;
        }
        let (zoom, name) = match (t * 3.0) as usize {
            0 => (Zoom::Fit, "fit"),
            1 => (Zoom::Factor(1.0), "100%"),
            _ => (Zoom::Factor(2.0), "200%"),
        };
        self.zoom = zoom;
        self.centre = [self.size[0] as f32 * sweep, self.size[1] as f32 * 0.5];
        name
    }

    /// Sets look `k`'s sky stretch and recomputes it on the GPU.
    fn restretch(&mut self, k: usize, stretch: f32) {
        self.params[k] = self.params[k].stretched(stretch);
        match self
            .looks
            .apply(&self.device, &self.queue, k, &self.params[k])
        {
            Ok(t) => self.recompute_ms = Some(t * 1e3),
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    pub fn looks_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Looks");
        for k in 0..2 {
            let mut stretch = self.params[k].stretch;
            ui.label(format!("{} · {}", ["A", "B"][k], self.names[k]));
            ui.horizontal(|ui| {
                let slider = egui::Slider::new(&mut stretch, 0.5..=30.0).text("sky stretch");
                if ui.add(slider).changed() {
                    self.restretch(k, stretch);
                }
                if ui.button("Reset").clicked() {
                    self.restretch(k, self.defaults[k].stretch);
                }
            });
        }
        if let Some(ms) = self.recompute_ms {
            ui.weak(format!("look recomputed on the GPU in {ms:.0} ms"));
        }
        ui.separator();
        ui.weak(
            "Space swap · S split · F fit · 1 = 100% · 2 = 200% · drag to pan · scroll to zoom",
        );
        ui.separator();
        for line in &self.display.lines {
            ui.weak(line);
        }
    }

    pub fn regions_panel(&mut self, ui: &mut egui::Ui) {
        let [a, same, b] = self.verdicts.tally();
        ui.heading("Where to look");
        ui.label(format!("A better: {a} · same: {same} · B better: {b}"));
        ui.weak(if self.verdicts.saved() {
            "saved in compare.json in the project folder"
        } else {
            "not saved (no project folder)"
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            for r in 0..self.regions.len() {
                ui.separator();
                ui.strong(self.regions[r].name);
                ui.weak(self.regions[r].look_for);
                if self.crops(ui, r) {
                    self.centre = self.regions[r].centre;
                    self.zoom = Zoom::Factor(2.0);
                }
                let name = self.regions[r].name;
                ui.horizontal(|ui| {
                    for (choice, label) in [
                        (Choice::First, "A better"),
                        (Choice::Same, "Same"),
                        (Choice::Second, "B better"),
                    ] {
                        let on = self.verdicts.get(name) == Some(choice);
                        if ui.selectable_label(on, label).clicked()
                            && let Err(e) = self.verdicts.set(name, choice)
                        {
                            self.error = Some(e.to_string());
                        }
                    }
                });
            }
        });
    }
}
