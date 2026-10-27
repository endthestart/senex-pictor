//! The spike's window: the canvas (drag to pan, scroll to zoom, F fit, 1 = 100%, 2 = 200%,
//! Space swaps the two renderings), the stretch of the shown rendering recomputed on the GPU,
//! EDR test swatches, and frame statistics. `--bench-window` drives the same view by script.

use crate::canvas::{self, CanvasGpu, Draw, MODE_ENCODE, MODE_SOLID, View};
use crate::look::Looks;
use crate::source::LookParams;
use crate::stats;
use std::time::Instant;

pub struct Setup {
    pub looks: Looks,
    pub params: [LookParams; 2],
    pub names: [&'static str; 2],
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub size: [u32; 2],
    pub adapter: String,
    pub bench_seconds: Option<f64>,
}

#[derive(Clone, Copy, PartialEq)]
enum Zoom {
    Fit,
    Factor(f32),
}

pub struct SpikeApp {
    s: Setup,
    showing: usize,
    zoom: Zoom,
    centre: [f32; 2],
    mode: u32,
    scale: f32,
    info: Vec<String>,
    edr: bool,
    intervals: Vec<(&'static str, f64)>,
    last: Option<Instant>,
    start: Instant,
    swaps: usize,
    recompute_ms: Option<f64>,
    error: Option<String>,
}

impl SpikeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, s: Setup) -> Result<SpikeApp, String> {
        let rs = cc
            .wgpu_render_state
            .as_ref()
            .ok_or("eframe did not start with wgpu")?;
        let format = rs.target_format;
        let gpu = CanvasGpu::new(&rs.device, format, &s.looks.outputs);
        rs.renderer.write().callback_resources.insert(gpu);
        let scale = rs.renderer.read().output_scale();
        let mode = if format == wgpu::TextureFormat::Rgba16Float || format.is_srgb() {
            0
        } else {
            MODE_ENCODE
        };
        let shared = rs.device == s.device;
        let info = vec![
            format!("GPU: {}", s.adapter),
            format!("window device shared with compute: {shared}"),
            format!("surface: {format:?}, {:?}", rs.target_color_space),
            format!("display HDR: {:?}", rs.display_hdr_info.read().clone()),
            format!("paper-white scale: {scale}"),
            format!(
                "renderings: 2 × {}×{} ({:.1} MP) on the GPU",
                s.size[0],
                s.size[1],
                f64::from(s.size[0] * s.size[1]) / 1e6
            ),
        ];
        for line in &info {
            eprintln!("spike: {line}");
        }
        Ok(SpikeApp {
            s,
            showing: 0,
            zoom: Zoom::Fit,
            centre: [0.0; 2],
            mode,
            scale,
            info,
            edr: false,
            intervals: Vec::new(),
            last: None,
            start: Instant::now(),
            swaps: 0,
            recompute_ms: None,
            error: None,
        })
    }

    fn image(&self) -> [f32; 2] {
        [self.s.size[0] as f32, self.s.size[1] as f32]
    }

    fn zoom_factor(&self, size: [f32; 2]) -> f32 {
        let img = self.image();
        match self.zoom {
            Zoom::Fit => (size[0] / img[0]).min(size[1] / img[1]),
            Zoom::Factor(z) => z,
        }
    }

    /// The scripted phase at `t` seconds of a `total`-second window benchmark.
    fn script(&mut self, t: f64, total: f64, frame: usize) -> &'static str {
        let img = self.image();
        let phase = t / total;
        let sweep = ((phase * 3.0).fract() as f32 * std::f32::consts::TAU).sin() * 0.4 + 0.5;
        if frame.is_multiple_of(15) {
            self.showing ^= 1;
            self.swaps += 1;
        }
        let (zoom, name) = match (phase * 3.0) as usize {
            0 => (Zoom::Fit, "fit"),
            1 => (Zoom::Factor(1.0), "100%"),
            _ => (Zoom::Factor(2.0), "200%"),
        };
        self.zoom = zoom;
        self.centre = [img[0] * sweep, img[1] * 0.5];
        name
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Canvas spike");
        for line in &self.info {
            ui.label(line);
        }
        ui.separator();
        ui.label(format!(
            "showing {} (Space swaps)",
            self.s.names[self.showing]
        ));
        let k = self.showing;
        let mut stretch = self.s.params[k].stretch;
        if ui
            .add(egui::Slider::new(&mut stretch, 0.5..=30.0).text("sky stretch"))
            .changed()
        {
            self.s.params[k] = self.s.params[k].stretched(stretch);
            match self
                .s
                .looks
                .apply(&self.s.device, &self.s.queue, k, &self.s.params[k])
            {
                Ok(t) => self.recompute_ms = Some(t * 1e3),
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        if let Some(ms) = self.recompute_ms {
            ui.label(format!("look recomputed on the GPU in {ms:.0} ms"));
        }
        ui.checkbox(&mut self.edr, "EDR test swatches (1×, 2×, 4× paper white)");
        ui.separator();
        let recent: Vec<f64> = self
            .intervals
            .iter()
            .rev()
            .take(240)
            .map(|(_, v)| *v)
            .collect();
        ui.label(format!("frame interval {}", stats::summary_ms(&recent)));
        ui.label(format!(
            "swaps: {} (drawn in the frame of the key press)",
            self.swaps
        ));
        if let Some(e) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, e);
        }
    }
}

impl eframe::App for SpikeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        let mut phase = "interactive";
        if let Some(total) = self.s.bench_seconds {
            let t = now.duration_since(self.start).as_secs_f64();
            if t > total {
                if self.s.bench_seconds.take().is_some() {
                    let report = stats::by_phase(&self.intervals);
                    println!(
                        "{}",
                        serde_json::json!({"window": report, "info": self.info, "swaps": self.swaps})
                    );
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            phase = self.script(t, total, self.intervals.len());
        }
        if let Some(last) = self.last.replace(now) {
            self.intervals
                .push((phase, now.duration_since(last).as_secs_f64()));
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.showing ^= 1;
                self.swaps += 1;
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
        egui::Panel::left("controls")
            .min_size(320.0)
            .show(ui, |ui| self.controls(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let response = ui.interact(rect, ui.id().with("canvas"), egui::Sense::drag());
                let ppp = ctx.pixels_per_point();
                let size = canvas::viewport_px(rect, ppp);
                if self.centre == [0.0; 2] {
                    self.centre = self.image().map(|v| v / 2.0);
                }
                let z = self.zoom_factor(size);
                let drag = response.drag_delta() * ppp / z;
                self.centre = [self.centre[0] - drag.x, self.centre[1] - drag.y];
                let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 && response.hovered() {
                    self.zoom = Zoom::Factor((z * (scroll / 200.0).exp()).clamp(0.02, 8.0));
                }
                let z = self.zoom_factor(size);
                let (lo, hi) = canvas::place(size, self.image(), z, self.centre);
                let view = View {
                    lo,
                    hi,
                    scale: self.scale,
                    mode: self.mode,
                    ..Default::default()
                };
                let draw = Draw {
                    view,
                    slot: 0,
                    rendering: self.showing,
                    nearest: z >= 2.0,
                };
                ui.painter().add(canvas::shape(rect, draw));
                if self.edr {
                    for (i, level) in [1.0f32, 2.0, 4.0].into_iter().enumerate() {
                        let r = egui::Rect::from_min_size(
                            rect.min + egui::vec2(20.0 + 70.0 * i as f32, 20.0),
                            egui::vec2(60.0, 60.0),
                        );
                        let solid = View {
                            lo: [-1.0, -1.0],
                            hi: [1.0, 1.0],
                            solid: [level, level, level, 1.0],
                            scale: self.scale,
                            mode: self.mode | MODE_SOLID,
                            ..Default::default()
                        };
                        ui.painter().add(canvas::shape(
                            r,
                            Draw {
                                view: solid,
                                slot: 1 + i as u64,
                                rendering: 0,
                                nearest: false,
                            },
                        ));
                    }
                }
            });
        ctx.request_repaint();
    }
}
