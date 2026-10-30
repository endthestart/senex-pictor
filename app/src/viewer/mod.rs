//! A result on the GPU, shared by every screen that shows it: the scene uploaded once, two
//! renderings (look A and look B) recomputed there when a look changes, and one view (zoom and
//! centre) so moving between Compare, Sky & ground and Look & export keeps the place.

mod canvas;

pub use canvas::{Canvas, Zoom};

use crate::display::Display;
use crate::gpu::canvas::CanvasGpu;
use crate::gpu::look::Looks;
use crate::scene::{self, LookParams, Scene};
use pictor_engine::Result;
use pictor_engine::render::{self, Look};

pub struct Viewer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    looks: Looks,
    pub scene: Scene,
    /// Each rendering's look and its GPU parameters.
    pub look: [Look; 2],
    pub params: [LookParams; 2],
    pub names: [String; 2],
    /// Rendering size (px).
    pub size: [u32; 2],
    pub display: Display,
    pub zoom: Zoom,
    pub centre: [f32; 2],
    pub recompute_ms: Option<f64>,
}

impl Viewer {
    /// Uploads `scene` (at `mp` megapixels if given), renders the natural look as A and the
    /// dark look as B, and registers the canvas with the window's renderer.
    pub fn new(
        rs: &eframe::egui_wgpu::RenderState,
        display: Display,
        scene: Scene,
        mp: Option<f64>,
    ) -> Result<Viewer> {
        let size = scene::output_size(&scene, mp);
        let (device, queue) = (rs.device.clone(), rs.queue.clone());
        let looks = Looks::new(&device, &queue, &scene, size)?;
        for (k, p) in scene.looks.iter().enumerate() {
            looks.apply(&device, &queue, k, p)?;
        }
        rs.renderer
            .write()
            .callback_resources
            .insert(CanvasGpu::new(&device, rs.target_format, &looks.outputs));
        Ok(Viewer {
            device,
            queue,
            looks,
            params: scene.looks,
            look: [render::NATURAL, render::DARK],
            names: ["Natural".into(), "Dark".into()],
            scene,
            size,
            display,
            zoom: Zoom::Fit,
            centre: [size[0] as f32 / 2.0, size[1] as f32 / 2.0],
            recompute_ms: None,
        })
    }

    /// Scene pixel `p` in rendering pixels.
    pub fn rendering_point(&self, p: [f32; 2]) -> [f32; 2] {
        [
            p[0] * self.size[0] as f32 / self.scene.width as f32,
            p[1] * self.size[1] as f32 / self.scene.height as f32,
        ]
    }

    fn apply(&mut self, k: usize) -> Result<()> {
        let t = self
            .looks
            .apply(&self.device, &self.queue, k, &self.params[k])?;
        self.recompute_ms = Some(t * 1e3);
        Ok(())
    }

    /// Sets rendering `k`'s look and recomputes it. Levels are measured again only when what
    /// they depend on changed (black and white points, glow removal).
    pub fn set_look(&mut self, k: usize, look: Look) -> Result<()> {
        let old = self.look[k];
        let relevel = old.black_percentile != look.black_percentile
            || old.white_percentile != look.white_percentile
            || old.remove_glow != look.remove_glow;
        self.params[k] = if relevel {
            scene::params(&self.scene, &look)?
        } else {
            LookParams {
                saturation: look.saturation,
                shadows: look.shadows,
                ..self.params[k].stretched(look.stretch)
            }
        };
        self.look[k] = look;
        self.apply(k)
    }

    /// Puts `look` (named `name`) in rendering `k`.
    pub fn set_named(&mut self, k: usize, name: &str, look: Look) -> Result<()> {
        self.names[k] = name.into();
        self.set_look(k, look)
    }

    pub fn image(&self) -> [f32; 2] {
        [self.size[0] as f32, self.size[1] as f32]
    }

    /// Physical px per rendering px in a viewport of `size` physical px.
    pub fn zoom_factor(&self, size: [f32; 2]) -> f32 {
        let img = self.image();
        match self.zoom {
            Zoom::Fit => (size[0] / img[0]).min(size[1] / img[1]),
            Zoom::Factor(z) => z,
        }
    }

    /// F fits, 1 and 2 zoom to 100% and 200%.
    pub fn zoom_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        ctx.input(|i| {
            if i.modifiers.command {
                return;
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
}
