//! Residuals of star pairs under a rotation, and rejection of pairs far off the fit.

use super::camera::{Camera, Vec3};
use super::model::{Matrix, apply};
use crate::stars::similarity::Point;

pub type Pairs = Vec<(Point, Point)>;

/// Pairs farther off the fitted model than REJECT_FACTOR times the median distance (and
/// REJECT_MIN_PX) are dropped and the fit repeated, up to REJECT_ROUNDS times: static lights
/// are tracked as stars when the sky moves less than the pairing tolerance between frames
/// (Northern Lights: 1.75 px per frame) and would pull the fit.
pub const REJECT_FACTOR: f64 = 5.0;
pub const REJECT_MIN_PX: f64 = 2.0;
pub const REJECT_ROUNDS: usize = 3;

/// Distance limit for pairs given all their distances.
pub fn rejection_limit(distances: &[f64]) -> f64 {
    let mut sorted = distances.to_vec();
    sorted.sort_by(f64::total_cmp);
    let median = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0);
    (REJECT_FACTOR * median).max(REJECT_MIN_PX)
}

/// Pixel distances of `pairs` from their predictions under rotation `r`.
pub fn distances(camera: &Camera, r: &Matrix, pairs: &[(Point, Point)]) -> Vec<f64> {
    pairs
        .iter()
        .map(|(a, b)| {
            camera
                .pixel(apply(r, camera.ray(*a)))
                .map_or(f64::INFINITY, |p| (p[0] - b[0]).hypot(p[1] - b[1]))
        })
        .collect()
}

/// `pairs` within `limit` of their predictions.
pub fn within(pairs: &[(Point, Point)], distances: &[f64], limit: f64) -> Pairs {
    pairs
        .iter()
        .zip(distances)
        .filter(|(_, d)| **d < limit)
        .map(|(p, _)| *p)
        .collect()
}

/// Residuals of `targets` against the reference `rays` turned by `r`, (x, y) per pair.
pub fn project(
    camera: &Camera,
    r: &super::model::Matrix,
    rays: &[Vec3],
    targets: &[Point],
    out: &mut Vec<f64>,
) {
    for (v, t) in rays.iter().zip(targets) {
        match camera.pixel(apply(r, *v)) {
            Some(p) => out.extend([p[0] - t[0], p[1] - t[1]]),
            None => out.extend([f64::NAN, f64::NAN]),
        }
    }
}
