//! `--screenshot FILE`: once the comparison has drawn for a moment, capture the window (the
//! patched egui-wgpu reads float surfaces back as sRGB bytes), save it as a JPEG and close. For
//! checking layouts on machines nobody is looking at.

use std::path::PathBuf;

/// Frames to draw before capturing, so crops and labels have settled.
const SETTLE_FRAMES: usize = 20;

pub struct Shot {
    path: PathBuf,
    frames: usize,
    requested: bool,
}

impl Shot {
    pub fn new(path: PathBuf) -> Shot {
        Shot {
            path,
            frames: 0,
            requested: false,
        }
    }

    /// Advances one frame of the comparison; true once the shot is saved (or failed).
    pub fn frame(&mut self, ctx: &egui::Context) -> bool {
        self.frames += 1;
        if !self.requested && self.frames >= SETTLE_FRAMES {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        let image = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = image else {
            ctx.request_repaint();
            return false;
        };
        let [w, h] = image.size;
        let display: Vec<[f32; 3]> = image
            .pixels
            .iter()
            .map(|p| [p.r(), p.g(), p.b()].map(|v| f32::from(v) / 255.0))
            .collect();
        match star_photos::export::jpeg(&self.path, &display, w, h, 90) {
            Ok(()) => eprintln!(
                "star-photos-app: screenshot saved to {}",
                self.path.display()
            ),
            Err(e) => eprintln!("star-photos-app: screenshot failed: {e}"),
        }
        true
    }
}
