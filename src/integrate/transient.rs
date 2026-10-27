//! Transient light in a hero frame: light painting, headlights, a lamp (light that only that
//! frame saw), found against the median of the frames around it.

use crate::separate::filters::{components, dilate, uniform};
use crate::separate::glow::CLEAR;
use crate::separate::grid::gaussian;

pub const TRANSIENT_SIGMA: f32 = 20.0;
pub const TRANSIENT_MIN_PX: usize = 500;

/// Light in the hero frame only: 5×5-smoothed luminance far above the window's median, as
/// regions of at least TRANSIENT_MIN_PX pixels that lie mostly outside clear sky (the sky
/// moves under the window, so its median is no reference there), grown 3 px and feathered
/// (σ 3 px).
pub fn transient(
    hero: &[[f32; 3]],
    still: &[[f32; 3]],
    alpha: &[f32],
    noise: f32,
    w: usize,
    h: usize,
) -> Vec<f32> {
    let excess: Vec<f32> = hero
        .iter()
        .zip(still)
        .map(|(a, b)| a.iter().sum::<f32>() - b.iter().sum::<f32>())
        .collect();
    let bright: Vec<bool> = uniform(&excess, w, h, 5)
        .iter()
        .map(|&e| e > TRANSIENT_SIGMA * noise)
        .collect();
    let (sizes, label) = components(&bright, w, h);
    let mut in_sky = vec![0usize; sizes.len()];
    for p in (0..w * h).filter(|&p| bright[p] && alpha[p] >= CLEAR) {
        in_sky[label[p]] += 1;
    }
    let large: Vec<bool> = (0..w * h)
        .map(|p| {
            bright[p]
                && sizes[label[p]] >= TRANSIENT_MIN_PX
                && 2 * in_sky[label[p]] < sizes[label[p]]
        })
        .collect();
    let grown: Vec<f64> = dilate(&large, w, h, 3)
        .iter()
        .map(|&b| f64::from(u8::from(b)))
        .collect();
    gaussian(&grown, w, h, 3.0, false)
        .iter()
        .map(|&v| v as f32)
        .collect()
}
