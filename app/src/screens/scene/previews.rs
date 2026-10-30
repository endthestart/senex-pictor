//! Each treatment's preview: its rendered JPEG, read and reduced in the background.

use super::SceneStep;
use crate::job::Job;
use egui::{ColorImage, TextureHandle, TextureOptions};
use std::path::Path;

/// Long side of a treatment's preview.
const PREVIEW_SIDE: u32 = 1600;

fn decode(path: &Path) -> pictor_engine::Result<ColorImage> {
    let image = image::open(path)?
        .thumbnail(PREVIEW_SIDE, PREVIEW_SIDE)
        .into_rgb8();
    let (w, h) = image.dimensions();
    Ok(ColorImage::from_rgb(
        [w as usize, h as usize],
        image.as_raw(),
    ))
}

impl SceneStep {
    /// Keeps previews once they have been read.
    pub fn poll_preview(&mut self, ctx: &egui::Context) {
        let mut pending = Vec::new();
        for (p, job) in std::mem::take(&mut self.loading) {
            match job.poll() {
                None => pending.push((p, job)),
                Some(Ok(image)) => {
                    let name = p.display().to_string();
                    let t = ctx.load_texture(name, image, TextureOptions::LINEAR);
                    self.previews.push((p, t));
                }
                Some(Err(e)) => self.error = Some(format!("{}: {e}", p.display())),
            }
        }
        self.loading = pending;
    }

    pub fn loading(&self) -> bool {
        !self.loading.is_empty()
    }

    /// The JPEG at `path` as a texture, read in the background the first time; none while
    /// it is read or when it does not exist.
    pub(super) fn preview(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        self.poll_preview(ctx);
        if let Some((_, t)) = self.previews.iter().find(|(p, _)| p == path) {
            return Some(t.clone());
        }
        if !self.loading.iter().any(|(p, _)| p == path) && path.exists() {
            let p = path.to_path_buf();
            let job = Job::start("Reading a preview", {
                let p = p.clone();
                move |_| decode(&p)
            });
            self.loading.push((p, job));
        }
        None
    }

    /// Forgets the preview of `path` (it was made again).
    pub fn refresh(&mut self, path: &Path) {
        self.previews.retain(|(p, _)| p != path);
    }
}
