//! Registration quality: residuals of held-out stars by field zone, and how far the fitted
//! angles depart from the sidereal rate.

use super::drift::free_rotation;
use super::fit::angle;
use super::model::{SIDEREAL_RAD_PER_S, SkyModel, rotation_vector, vector_of};
use crate::Result;
use crate::stars::similarity::Random;
use crate::stars::{Star, Tracks};
use serde::Serialize;

pub const HELD_OUT_SHARE: f64 = 0.3;
const ZONES: [(&str, f64, f64); 3] = [
    ("center", 0.0, 0.35),
    ("mid", 0.35, 0.7),
    ("outer", 0.7, 1.01),
];

#[derive(Debug, Serialize)]
pub struct Zone {
    pub zone: &'static str,
    pub stars: usize,
    pub rms_px: Option<f64>,
    pub median_px: Option<f64>,
    pub p95_px: Option<f64>,
}

/// Linear-interpolated percentile of sorted values (numpy's default).
pub fn percentile(sorted: &[f64], q: f64) -> Option<f64> {
    let last = sorted.len().checked_sub(1)?;
    let rank = q / 100.0 * last as f64;
    let (lo, hi) = (rank.floor() as usize, rank.ceil() as usize);
    Some(sorted[lo] + (sorted[hi] - sorted[lo]) * (rank - lo as f64))
}

/// Residuals of a random HELD_OUT_SHARE of `frame`'s stars when its angle (or free rotation)
/// is refit on the others.
pub fn held_out(
    model: &SkyModel,
    stars: &[Vec<Star>],
    tracks: &Tracks,
    frame: usize,
    seed: u64,
) -> Result<Vec<Zone>> {
    let pairs = tracks.correspondences(stars, model.reference, frame);
    let mut random = Random::new(seed);
    let test: Vec<bool> = pairs
        .iter()
        .map(|_| (random.next_u64() >> 11) as f64 / (1u64 << 53) as f64 <= HELD_OUT_SHARE)
        .collect();
    let train: Vec<_> = pairs
        .iter()
        .zip(&test)
        .filter(|(_, t)| !**t)
        .map(|(p, _)| *p)
        .collect();
    let mut refit = model.clone();
    match &mut refit.rotations {
        Some(r) => {
            r[frame] = rotation_vector(free_rotation(&model.camera, &train, vector_of(&r[frame]))?)
        }
        None => {
            refit.angles[frame] = angle(&model.camera, model.axis, &train, model.angles[frame])?
        }
    }
    let (c, m) = (model.camera.centre(), model.camera.scale());
    let mut zones: Vec<(Vec<f64>, &'static str, f64, f64)> = ZONES
        .iter()
        .map(|&(z, lo, hi)| (Vec::new(), z, lo, hi))
        .collect();
    for ((a, b), _) in pairs.iter().zip(&test).filter(|(_, t)| **t) {
        let Some(p) = refit.predict(frame, *a) else {
            continue;
        };
        let rho = (b[0] - c[0]).hypot(b[1] - c[1]) / m;
        if let Some(z) = zones.iter_mut().find(|z| rho >= z.2 && rho < z.3) {
            z.0.push((p[0] - b[0]).hypot(p[1] - b[1]));
        }
    }
    Ok(zones
        .into_iter()
        .map(|(mut e, zone, _, _)| {
            e.sort_by(f64::total_cmp);
            let rms = (!e.is_empty())
                .then(|| (e.iter().map(|v| v * v).sum::<f64>() / e.len() as f64).sqrt());
            Zone {
                zone,
                stars: e.len(),
                rms_px: rms,
                median_px: percentile(&e, 50.0),
                p95_px: percentile(&e, 95.0),
            }
        })
        .collect())
}

/// Largest and RMS departure of the fitted angles from the sidereal rate, in degrees.
pub fn sidereal_departure(model: &SkyModel, times: &[f64]) -> (f64, f64) {
    let d: Vec<f64> = model
        .angles
        .iter()
        .zip(times)
        .map(|(a, t)| (a - SIDEREAL_RAD_PER_S * (t - times[model.reference])).to_degrees())
        .collect();
    let max = d.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    (
        max,
        (d.iter().map(|v| v * v).sum::<f64>() / d.len().max(1) as f64).sqrt(),
    )
}
