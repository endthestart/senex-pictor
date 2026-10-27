//! Clearance from the foreground and the composite of sky and ground.

use crate::separate::filters::{components, distance_squared};
use crate::separate::glow::CLEAR;

/// Non-clear regions smaller than this are noise in clear sky, not obstructions.
const MIN_OBSTRUCTION: usize = 64;
/// Sky seen in under this share of frames blends toward the reference window (its own glow
/// and lighting show as arcs along the star paths otherwise).
pub const FULL_COVERAGE: f32 = 0.3;
/// Frames either side of the reference averaged into the window.
pub const WINDOW_HALF: usize = 4;

/// Distance in pixels to the nearest obstruction (a non-clear region of at least
/// MIN_OBSTRUCTION pixels) or beyond the frame border.
pub fn clearance(alpha: &[f32], w: usize, h: usize) -> Vec<f32> {
    let unclear: Vec<bool> = alpha.iter().map(|&a| a < CLEAR).collect();
    let (sizes, label) = components(&unclear, w, h);
    let obstruction: Vec<bool> = (0..w * h)
        .map(|p| unclear[p] && sizes[label[p]] >= MIN_OBSTRUCTION)
        .collect();
    let distance = distance_squared(&obstruction, w, h).map(|(d, _)| d);
    (0..w * h)
        .map(|p| {
            let (x, y) = (p % w, p / w);
            let border = x.min(w - 1 - x).min(y).min(h - 1 - y) as f32 + 1.0;
            distance
                .as_ref()
                .map_or(border, |d| (d[p].sqrt() as f32).min(border))
        })
        .collect()
}

/// alpha·sky + (1 − alpha)·ground, with sky of low `coverage` (share of frames) blended toward
/// the reference `window` and missing sky taken from it. Returns the image and the missing
/// sky pixels.
pub fn composite(
    sky: &[[f32; 3]],
    ground: &[[f32; 3]],
    alpha: &[f32],
    window: &[[f32; 3]],
    coverage: &[f32],
) -> (Vec<[f32; 3]>, Vec<bool>) {
    let missing: Vec<bool> = (0..sky.len())
        .map(|p| alpha[p] > 0.0 && !sky[p].iter().all(|v| v.is_finite()))
        .collect();
    let image = (0..sky.len())
        .map(|p| {
            let a = alpha[p];
            let weight = (coverage[p] / FULL_COVERAGE).clamp(0.0, 1.0);
            std::array::from_fn(|k| {
                let s = if missing[p] {
                    window[p][k]
                } else {
                    let v = if sky[p][k].is_finite() {
                        sky[p][k]
                    } else {
                        0.0
                    };
                    weight * v + (1.0 - weight) * window[p][k]
                };
                a * s + (1.0 - a) * ground[p][k]
            })
        })
        .collect();
    (image, missing)
}
