//! What the sequence check needs from each RAW: capture time and settings, the saturated
//! share of the default crop, the sky level (median of the upper half) and the foreground
//! colour (median of the lower half per colour), all from CFA samples above black, the frame's
//! look for motion between frames and its thumbnail.

use super::motion::{self, Look, Steps};
use super::thumb::{self, Thumb};
use crate::budget::Budget;
use crate::calibration::Mosaic;
use crate::stars::similarity::Random;
use crate::store::seconds;
use crate::{Result, error, input};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Settings that must stay fixed within one run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,
    pub exposure_s: Option<f64>,
    pub iso: Option<u32>,
    pub aperture: Option<f64>,
    pub focal_mm: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capture {
    pub name: String,
    pub time_s: f64,
    pub settings: Settings,
    pub saturated_share: f64,
    /// Median of 2×2 site sums above black in the upper half (mostly sky).
    pub sky_level: f64,
    /// Median R, G, B above black in the lower half (mostly foreground).
    pub ground_rgb: [f64; 3],
}

/// Frames decoded at once before their motion is measured.
const CHUNK: usize = 32;
/// Working set of one capture per sensor pixel: the RAW, its CFA samples above black, the
/// lower half's samples and the half-resolution look.
const CAPTURE_BYTES_PER_PIXEL: u64 = 40;
/// A kept look per sensor pixel (half-resolution features).
const LOOK_BYTES_PER_PIXEL: u64 = 1;

pub fn capture(path: &Path) -> Result<(Capture, Look, Thumb)> {
    let (raw, name) = input::decode_named(path)?;
    let mosaic = Mosaic::from_raw(&raw)?;
    let crop = raw.develop_crop(mosaic.width, mosaic.height);
    let m = &raw.metadata;
    let time_s = m
        .capture_time
        .as_ref()
        .map(seconds)
        .ok_or_else(|| error(format!("{name}: no capture time")))?;
    let at = |x: usize, y: usize| (crop.x + x, crop.y + y);
    let area = (crop.x, crop.y, crop.width, crop.height);
    let plane = mosaic.plane_above_black(area);
    let colours = mosaic.colours(area);
    let above = |x: usize, y: usize| plane[y * crop.width + x];
    let colour = |x: usize, y: usize| colours[(crop.y + y) % colours.len()][x];
    let (w, h) = (crop.width / 2 * 2, crop.height / 2 * 2);
    let mut saturated = 0usize;
    for y in 0..crop.height {
        for x in 0..crop.width {
            let (ax, ay) = at(x, y);
            saturated += usize::from(mosaic.data[ay * mosaic.width + ax] >= mosaic.white);
        }
    }
    let mut sky = Vec::with_capacity(h / 4 * (w / 2));
    for j in 0..h / 4 {
        for i in 0..w / 2 {
            let (x, y) = (2 * i, 2 * j);
            sky.push(above(x, y) + above(x + 1, y) + above(x, y + 1) + above(x + 1, y + 1));
        }
    }
    let mut ground: [Vec<f32>; 3] = Default::default();
    for y in h / 2..h {
        for x in 0..w {
            ground[usize::from(colour(x, y))].push(above(x, y));
        }
    }
    let look = motion::look(&plane, (crop.width, crop.height))?;
    let wb = raw
        .wb_multipliers
        .ok_or_else(|| error(format!("{name}: no as-shot white balance")))?;
    let range = f64::from(mosaic.white) - f64::from(mosaic.black.at(crop.x, crop.y, 0, 1));
    let thumb = thumb::make(
        &plane,
        (crop.width, crop.height),
        (&colours, crop.y),
        range,
        [wb[0] / wb[1], 1.0, wb[2] / wb[1]],
    )?;
    let capture = Capture {
        name,
        time_s,
        settings: Settings {
            make: m.make.clone(),
            model: m.model.clone(),
            lens: m.lens_model.clone(),
            exposure_s: m.exposure_time,
            iso: m.iso,
            aperture: m.f_number,
            focal_mm: m.focal_length,
        },
        saturated_share: saturated as f64 / (crop.width * crop.height) as f64,
        sky_level: median(&mut sky)?,
        ground_rgb: [
            median(&mut ground[0])?,
            median(&mut ground[1])?,
            median(&mut ground[2])?,
        ],
    };
    Ok((capture, look, thumb))
}

pub fn median(values: &mut [f32]) -> Result<f64> {
    if values.is_empty() {
        return Err(error("median of no samples"));
    }
    let mid = values.len() / 2;
    let (_, v, _) = values.select_nth_unstable_by(mid, f32::total_cmp);
    Ok(f64::from(*v))
}

/// Every frame's capture, the motion from each frame to the next, and every thumbnail.
pub struct Captured {
    pub captures: Vec<Capture>,
    pub steps: Steps,
    pub thumbs: Vec<Thumb>,
}

/// `capture` for every path, as many at a time as `budget` allows, in path order.
pub fn capture_all(paths: &[std::path::PathBuf], budget: &Budget) -> Result<Captured> {
    use rayon::prelude::*;
    let first = paths.first().ok_or_else(|| error("no RAW files"))?;
    let [w, h] = input::inspect(first)?.sensor;
    let pixels = (w * h) as u64;
    let pool = budget.pool(
        "the sequence check",
        2 * CHUNK as u64 * LOOK_BYTES_PER_PIXEL * pixels,
        CAPTURE_BYTES_PER_PIXEL * pixels,
    )?;
    budget.progress.step("Reading frames", paths.len());
    pool.install(|| {
        let mut captures = Vec::with_capacity(paths.len());
        let mut thumbs = Vec::with_capacity(paths.len());
        let mut steps = Steps::default();
        let mut previous: Option<Look> = None;
        for chunk in paths.chunks(CHUNK) {
            let read: Vec<(Capture, Look, Thumb)> = chunk
                .par_iter()
                .map(|p| {
                    let c = capture(p)?;
                    budget.progress.advance()?;
                    Ok(c)
                })
                .collect::<Result<_>>()?;
            let mut caps = Vec::with_capacity(read.len());
            let mut looks = Vec::with_capacity(read.len());
            for (c, l, t) in read {
                caps.push(c);
                looks.push(l);
                thumbs.push(t);
            }
            if previous.is_none() {
                steps.foreground.push(None);
                steps.sky.push(None);
            }
            let chain: Vec<&Look> = previous.iter().chain(looks.iter()).collect();
            let first = captures.len() + usize::from(previous.is_none());
            let measured: Vec<_> = chain
                .par_windows(2)
                .enumerate()
                .map(|(k, w)| {
                    let mut random = Random::new((first + k) as u64);
                    (
                        motion::foreground_step(w[0], w[1]),
                        motion::sky_step(w[0], w[1], &mut random),
                    )
                })
                .collect();
            for (f, s) in measured {
                steps.foreground.push(f);
                steps.sky.push(s);
            }
            captures.extend(caps);
            previous = looks.pop();
        }
        Ok(Captured {
            captures,
            steps,
            thumbs,
        })
    })
}
