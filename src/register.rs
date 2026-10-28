//! Registration of a stored sequence (ADR 0008, 0010): detect stars in every frame, track
//! them, fit the fixed-axis sky model (or reuse a lens profile), report held-out residuals,
//! and fit free per-frame rotations when the fixed axis cannot follow the stars (drift).

use crate::budget::Budget;
use crate::sky::drift::free_rotations;
use crate::sky::fit::{Start, fit};
use crate::sky::report::{Zone, held_out, sidereal_departure};
use crate::sky::{Camera, SkyModel};
use crate::stars::{self, Star, Tracks, detect, track};
use crate::store::Store;
use crate::{Result, error};
use serde::Serialize;
use std::path::Path;

/// Starting focal length assumes a 36 mm wide sensor; the fit refines it.
const SENSOR_WIDTH_MM: f64 = 36.0;
const START_K: (f64, f64) = (-0.116, 0.044);
/// Frames between joint-fit frames.
const SUBSET_STEP: usize = 10;
/// A held-out median above this in the centre zone of any reported frame means the camera
/// drifted (fixed-axis medians stay below 0.7 px on the five owner sequences; Yellowstone's
/// drifting run reaches 2.5–11 px).
pub const DRIFT_CENTRE_PX: f64 = 1.5;
const HELD_OUT_SEED: u64 = 1;
/// Working set of detection per pixel: the frame read and summed, the saturation mask, the
/// background-subtracted and filtered images and the component labels.
const DETECT_BYTES_PER_PIXEL: u64 = 48;

#[derive(Debug, Serialize)]
pub struct Report {
    pub stars_per_frame: [usize; 3],
    pub tracks: usize,
    pub distortion: String,
    pub camera: Camera,
    pub axis: [f64; 3],
    pub reference: String,
    pub angle_minus_sidereal_deg: [f64; 2],
    pub held_out: Vec<(String, Vec<Zone>)>,
    pub drift: bool,
    pub held_out_free: Vec<(String, Vec<Zone>)>,
    pub seconds: [f64; 3],
}

pub fn detect_all(store: &Store, budget: &Budget) -> Result<Vec<Vec<Star>>> {
    use rayon::prelude::*;
    let (w, h) = (store.manifest.width, store.manifest.height);
    let pool = budget.pool("star detection", 0, DETECT_BYTES_PER_PIXEL * (w * h) as u64)?;
    budget
        .progress
        .step("Finding stars", store.manifest.frames.len());
    pool.install(|| {
        (0..store.manifest.frames.len())
            .into_par_iter()
            .map(|i| {
                let stars = detect(&store.luminance(i)?, &store.saturated_rows(i, 0, h)?, w, h)?;
                budget.progress.advance()?;
                Ok(stars)
            })
            .collect()
    })
}

pub fn times(store: &Store) -> Vec<f64> {
    let f = &store.manifest.frames;
    f.iter().map(|i| i.time_s - f[0].time_s).collect()
}

fn reports(
    model: &SkyModel,
    stars: &[Vec<Star>],
    tracks: &Tracks,
    names: &[String],
) -> Result<Vec<(String, Vec<Zone>)>> {
    let n = stars.len();
    let r = model.reference;
    let mut frames = vec![
        0,
        n / 4,
        r.saturating_sub(1),
        (r + 1).min(n - 1),
        3 * n / 4,
        n - 1,
    ];
    frames.retain(|&i| i != r);
    frames.dedup();
    frames
        .into_iter()
        .map(|i| {
            Ok((
                names[i].clone(),
                held_out(model, stars, tracks, i, HELD_OUT_SEED)?,
            ))
        })
        .collect()
}

/// Registers the store in place: writes `stars.bin`, `model.json` and `register.json`.
pub fn register(store: &Store, lens_from: Option<&Path>, budget: &Budget) -> Result<Report> {
    let n = store.manifest.frames.len();
    if n < 2 {
        return Err(error("registration needs at least two frames"));
    }
    let clock = std::time::Instant::now();
    let found = detect_all(store, budget)?;
    let detected = clock.elapsed().as_secs_f64();
    budget.progress.step("Matching stars across frames", 0);
    let tracks = track(&found)?;
    let tracked = clock.elapsed().as_secs_f64() - detected;
    let reference = (n - 1) / 2;
    let size = (store.manifest.width, store.manifest.height);
    let start = match lens_from {
        Some(path) => {
            let lens: SkyModel = serde_json::from_slice(&std::fs::read(path)?)?;
            Start {
                f: lens.camera.f,
                k1: lens.camera.k1,
                k2: lens.camera.k2,
                fix_distortion: true,
            }
        }
        None => {
            let focal = store.manifest.frames[reference].focal_mm.ok_or_else(|| {
                error("no focal length to start the fit from; reuse a lens profile")
            })?;
            Start {
                f: focal * size.0 as f64 / SENSOR_WIDTH_MM,
                k1: START_K.0,
                k2: START_K.1,
                fix_distortion: false,
            }
        }
    };
    let subset: Vec<usize> = (0..n)
        .step_by(SUBSET_STEP)
        .filter(|&i| i != reference)
        .collect();
    let times = times(store);
    budget.progress.step("Fitting the sky's motion", 0);
    let fixed = fit(&found, &tracks, &times, reference, &subset, size, start)?;
    let names: Vec<String> = store
        .manifest
        .frames
        .iter()
        .map(|f| f.name.clone())
        .collect();
    let held = reports(&fixed, &found, &tracks, &names)?;
    let drift = held
        .iter()
        .flat_map(|(_, z)| z)
        .any(|z| z.zone == "center" && z.median_px.is_some_and(|m| m > DRIFT_CENTRE_PX));
    let (model, held_free) = if drift {
        let free = free_rotations(&fixed, &found, &tracks, reference)?;
        let held = reports(&free, &found, &tracks, &names)?;
        (free, held)
    } else {
        (fixed.clone(), Vec::new())
    };
    let counts = {
        let mut c: Vec<usize> = found.iter().map(Vec::len).collect();
        c.sort_unstable();
        [c[0], c[n / 2], c[n - 1]]
    };
    let (max, rms) = sidereal_departure(&fixed, &times);
    let report = Report {
        stars_per_frame: counts,
        tracks: tracks.count,
        distortion: match lens_from {
            Some(p) => format!("fixed from {}", p.display()),
            None => "fitted".into(),
        },
        camera: fixed.camera,
        axis: fixed.axis,
        reference: names[reference].clone(),
        angle_minus_sidereal_deg: [max, rms],
        held_out: held,
        drift,
        held_out_free: held_free,
        seconds: [
            detected,
            tracked,
            clock.elapsed().as_secs_f64() - detected - tracked,
        ],
    };
    stars::io::save(&store.dir.join("stars.bin"), &found, &tracks)?;
    std::fs::write(
        store.dir.join("model.json"),
        serde_json::to_vec_pretty(&model)?,
    )?;
    std::fs::write(
        store.dir.join("register.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
