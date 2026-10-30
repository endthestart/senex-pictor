//! Frame thumbnails from the sequence check (linear camera RGB, white-balanced, as a share of
//! white) shown with one display stretch for the whole folder, measured on a typical frame, so
//! a lit or overexposed frame looks brighter than its neighbours as it is. Textures are made
//! when first drawn.

use egui::{ColorImage, TextureHandle, TextureOptions};
use half::f16;
use pictor_engine::sequence::Thumb;

/// Stretch strength of the asinh curve.
const STRETCH: f32 = 10.0;

#[derive(Clone, Copy)]
pub struct Stretch {
    black: [f32; 3],
    span: f32,
}

fn percentile(values: &mut [f32], p: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let k = ((values.len() - 1) as f32 * p / 100.0).round() as usize;
    *values.select_nth_unstable_by(k, f32::total_cmp).1
}

impl Stretch {
    /// Black at each channel's 2nd percentile of `rgb`, white at its luminance's 99.7th.
    pub fn of(rgb: &[f32]) -> Stretch {
        let channel = |k: usize| -> Vec<f32> { rgb.iter().skip(k).step_by(3).copied().collect() };
        let black = [0, 1, 2].map(|k| percentile(&mut channel(k), 2.0));
        let mut lum: Vec<f32> = rgb
            .as_chunks::<3>()
            .0
            .iter()
            .map(|c| 0.25 * (c[0] - black[0]) + 0.5 * (c[1] - black[1]) + 0.25 * (c[2] - black[2]))
            .collect();
        let span = percentile(&mut lum, 99.7).max(1e-6);
        Stretch { black, span }
    }

    pub fn image(&self, width: usize, height: usize, rgb: &[f32]) -> ColorImage {
        let norm = STRETCH.asinh();
        let bytes: Vec<u8> = rgb
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|c| {
                [0, 1, 2].map(|k| {
                    let t = ((c[k] - self.black[k]) / self.span).max(0.0);
                    ((STRETCH * t).asinh() / norm * 255.0).clamp(0.0, 255.0) as u8
                })
            })
            .collect();
        ColorImage::from_rgb([width, height], &bytes)
    }
}

pub fn linear(t: &Thumb) -> Vec<f32> {
    t.rgb.iter().map(|&b| f16::from_bits(b).to_f32()).collect()
}

pub struct Thumbs {
    pub stretch: Stretch,
    images: Vec<Thumb>,
    textures: Vec<Option<TextureHandle>>,
}

impl Thumbs {
    /// The thumbnails `images`, stretched as frame `typical` needs.
    pub fn new(images: Vec<Thumb>, typical: usize) -> Thumbs {
        let stretch = images.get(typical).map_or(
            Stretch {
                black: [0.0; 3],
                span: 1.0,
            },
            |t| Stretch::of(&linear(t)),
        );
        let textures = vec![None; images.len()];
        Thumbs {
            stretch,
            images,
            textures,
        }
    }

    /// Width over height of frame `i`'s thumbnail.
    pub fn aspect(&self, i: usize) -> f32 {
        self.images
            .get(i)
            .map_or(1.5, |t| t.width as f32 / t.height.max(1) as f32)
    }

    pub fn texture(&mut self, ctx: &egui::Context, i: usize) -> Option<TextureHandle> {
        let t = self.images.get(i)?;
        let slot = self.textures.get_mut(i)?;
        if slot.is_none() {
            let image = self.stretch.image(t.width, t.height, &linear(t));
            *slot = Some(ctx.load_texture(format!("thumb {i}"), image, TextureOptions::LINEAR));
        }
        slot.clone()
    }
}
