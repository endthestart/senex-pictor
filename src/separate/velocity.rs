//! How fast the sky moves across the image, and how many frames apart a transit can be
//! confirmed.

use super::stats::{CELL, MIN_SHIFT_PX, STEPS};
use crate::sky::SkyModel;
use crate::sky::model::{apply, rotation};

const VELOCITY_DELTA: f64 = 1e-4;

/// Displacement per radian of sky rotation at the centre of every CELL×CELL cell.
pub fn sky_velocity(model: &SkyModel, width: usize, height: usize) -> Vec<[f32; 2]> {
    let (gw, gh) = (width.div_ceil(CELL), height.div_ceil(CELL));
    let turn = rotation(model.axis, VELOCITY_DELTA);
    (0..gw * gh)
        .map(|c| {
            let p = [
                (CELL / 2 + CELL * (c % gw)) as f64,
                (CELL / 2 + CELL * (c / gw)) as f64,
            ];
            match model.camera.pixel(apply(&turn, model.camera.ray(p))) {
                Some(q) => [
                    ((q[0] - p[0]) / VELOCITY_DELTA) as f32,
                    ((q[1] - p[1]) / VELOCITY_DELTA) as f32,
                ],
                None => [0.0, 0.0],
            }
        })
        .collect()
}

/// Per cell, the smallest frame step from STEPS that moves the sky MIN_SHIFT_PX (0: none).
pub fn frame_steps(velocity: &[[f32; 2]], angles: &[f64]) -> Vec<usize> {
    let mut d: Vec<f64> = angles.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    d.sort_by(f64::total_cmp);
    let typical = match d.len() {
        0 => 0.0,
        n if n % 2 == 1 => d[n / 2],
        n => (d[n / 2 - 1] + d[n / 2]) / 2.0,
    };
    velocity
        .iter()
        .map(|v| {
            let per_frame = f64::from(v[0].hypot(v[1])) * typical;
            STEPS
                .iter()
                .copied()
                .find(|&k| k as f64 * per_frame >= MIN_SHIFT_PX)
                .unwrap_or(0)
        })
        .collect()
}
