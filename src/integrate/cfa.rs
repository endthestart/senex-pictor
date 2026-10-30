//! The sky stacked from raw CFA samples (Bayer drizzle, ADR 0009): for each output pixel and
//! frame the sky model gives the sensor position that saw it, and each CFA site contributes
//! its nearest sample of that site's colour weighted by a Gaussian of the distance. Sky
//! rotation dithers the sub-pixel phase between frames, so every colour is measured at every
//! output pixel without demosaicing or resampling.

use super::combine::{Sample, weighted};
use crate::budget::Budget;
use crate::separate::glow::{CLEAR, Grid};
use crate::sky::SkyModel;
use crate::sky::model::{Matrix, apply};
use crate::store::{Store, file};
use crate::{Result, error};
use rayon::prelude::*;

/// Gaussian σ and cut-off radius of a sample's weight, in sensor pixels.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Kernel {
    pub sigma: f64,
    pub radius: f64,
}

pub const KERNEL: Kernel = Kernel {
    sigma: 0.5,
    radius: 1.0,
};
/// Samples nearer than this to anything that is not clear sky mix in foreground.
pub const CLEARANCE_PX: f32 = 1.5;
pub(crate) const BAND: usize = 128;
/// Bands are halved down to this height to fit the memory budget.
const MIN_BAND: usize = 8;
const MARGIN_ROWS: f64 = 4.0;
const PROBE_STEP: usize = 32;
/// Per output pixel and worker: the samples gathered for one pixel are small; the band's
/// frame rows dominate and are counted per band.
const PIXEL_WORKER_BYTES: u64 = 1 << 20;
/// A pixel is a clipped core where at least this share of one colour's samples was saturated;
/// fewer (a hot pixel, an aircraft light) are left out like any other outlier.
pub const CLIPPED_SHARE: f32 = 0.1;

pub struct SkyStack {
    /// Camera RGB in ADU (glow-corrected to the reference), NaN where no sample survived.
    pub rgb: Vec<[f32; 3]>,
    pub kept: Vec<[u32; 3]>,
    /// Means of the even- and odd-numbered frames' kept samples (half their difference is noise).
    pub halves: [Vec<[f32; 3]>; 2],
    /// Clipped cores: at least CLIPPED_SHARE of one colour's samples were saturated.
    pub clipped: Vec<bool>,
}

/// One frame's CFA rows y0.. and its occlusion bits for a band, packed as stored (one row of
/// `file::bit_row(width)` bytes per CFA row, most significant bit first).
pub(crate) struct Rows {
    pub(crate) y0: usize,
    pub(crate) cfa: Vec<u16>,
    pub(crate) occluded: Vec<u8>,
}

impl Rows {
    fn occluded(&self, w: usize, x: usize, row: usize) -> bool {
        self.occluded[row * file::bit_row(w) + x / 8] & (0x80 >> (x % 8)) != 0
    }
}

pub struct Inputs<'a> {
    pub store: &'a Store,
    pub model: &'a SkyModel,
    pub alpha: &'a [f32],
    pub clearance: &'a [f32],
    pub glow: &'a [Grid],
    pub flat: &'a [f32],
    pub kernel: Kernel,
    /// Bit masks of samples to leave out per frame (`occ` from separation, `hero-occ`).
    pub occlusion: &'a str,
    /// Frames that contribute.
    pub frames: &'a [bool],
}

pub fn stack(inputs: &Inputs, budget: &Budget) -> Result<SkyStack> {
    let m = &inputs.store.manifest;
    let (w, h, n) = (m.width, m.height, m.frames.len());
    if inputs.frames.len() != n {
        return Err(error(
            "the CFA stack's frame selection differs from the store",
        ));
    }
    if m.cfa_width != 2 || m.cfa_height != 2 || m.cfa.iter().any(|&c| c > 2) {
        return Err(error("the CFA stack needs a 2×2 RGB colour filter pattern"));
    }
    let rotations: Vec<Matrix> = (0..n).map(|i| inputs.model.rotation(i)).collect();
    let held = Budget::held(w * h);
    let room = budget.room(
        "the sky stack",
        held + budget.threads as u64 * PIXEL_WORKER_BYTES,
    )?;
    let mut out = SkyStack {
        rgb: Vec::with_capacity(w * h),
        kept: Vec::with_capacity(w * h),
        halves: [Vec::with_capacity(w * h), Vec::with_capacity(w * h)],
        clipped: Vec::with_capacity(w * h),
    };
    let bands = bands(inputs, &rotations, BAND, 0, room)?;
    budget.progress.step("Stacking the sky", bands.len());
    for (b0, b1, bytes) in bands {
        budget.progress.advance()?;
        let pool = budget.pool("the sky stack", held + bytes, PIXEL_WORKER_BYTES)?;
        let rows: Vec<Option<Rows>> = pool.install(|| {
            (0..n)
                .into_par_iter()
                .map(|i| band_rows(inputs, &rotations[i], i, b0, b1))
                .collect::<Result<_>>()
        })?;
        let pixels: Vec<Pixel> = pool.install(|| {
            (b0 * w..b1 * w)
                .into_par_iter()
                .map(|q| pixel(inputs, &rotations, &rows, q))
                .collect()
        });
        for p in pixels {
            out.rgb.push(p.rgb);
            out.kept.push(p.kept);
            out.halves[0].push(p.halves[0]);
            out.halves[1].push(p.halves[1]);
            out.clipped.push(p.clipped);
        }
    }
    Ok(out)
}

/// Bands of output rows, each at most `rows` tall and halved (down to MIN_BAND) until its
/// frames' rows plus `per_row` bytes per output row fit in `room`; with their bytes.
pub(crate) fn bands(
    inputs: &Inputs,
    rotations: &[Matrix],
    rows: usize,
    per_row: u64,
    room: u64,
) -> Result<Vec<(usize, usize, u64)>> {
    let (w, h) = (inputs.store.manifest.width, inputs.store.manifest.height);
    let row_bytes = (2 * w + file::bit_row(w)) as u64;
    let bytes = |b0: usize, b1: usize| -> u64 {
        let input: u64 = (0..rotations.len())
            .filter_map(|i| span(inputs, &rotations[i], i, b0, b1))
            .map(|(y0, y1)| (y1 - y0) as u64 * row_bytes)
            .sum();
        input + (b1 - b0) as u64 * per_row
    };
    let mut out = Vec::new();
    let mut b0 = 0;
    while b0 < h {
        let mut b1 = (b0 + rows).min(h);
        while bytes(b0, b1) > room && b1 - b0 > MIN_BAND {
            b1 = b0 + (b1 - b0) / 2;
        }
        let need = bytes(b0, b1);
        if need > room {
            return Err(error(format!(
                "rows {b0}..{b1} of the sky stack need {:.1} GB of frame rows; give a larger \
                 memory budget (--memory GB)",
                need as f64 / 1e9
            )));
        }
        out.push((b0, b1, need));
        b0 = b1;
    }
    Ok(out)
}

/// The rows of frame `i` that output rows b0..b1 can sample, if any.
fn span(inputs: &Inputs, r: &Matrix, i: usize, b0: usize, b1: usize) -> Option<(usize, usize)> {
    if !inputs.frames[i] {
        return None;
    }
    let (w, h) = (inputs.store.manifest.width, inputs.store.manifest.height);
    let camera = &inputs.model.camera;
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let ys = (b0..b1).step_by(PROBE_STEP).chain([b1 - 1]);
    for y in ys {
        for x in (0..w).step_by(PROBE_STEP).chain([w - 1]) {
            if let Some(p) = camera.pixel(apply(r, camera.ray([x as f64, y as f64]))) {
                lo = lo.min(p[1]);
                hi = hi.max(p[1]);
            }
        }
    }
    let y0 = (lo - MARGIN_ROWS - PROBE_STEP as f64).floor().max(0.0);
    let y1 = (hi + MARGIN_ROWS + PROBE_STEP as f64).ceil().min(h as f64);
    if y0.is_nan() || y1.is_nan() || y0 >= y1 {
        return None;
    }
    Some((y0 as usize, y1 as usize))
}

/// The rows of frame `i` that output rows b0..b1 can sample, read once per band.
pub(crate) fn band_rows(
    inputs: &Inputs,
    r: &Matrix,
    i: usize,
    b0: usize,
    b1: usize,
) -> Result<Option<Rows>> {
    let Some((y0, y1)) = span(inputs, r, i, b0, b1) else {
        return Ok(None);
    };
    Ok(Some(Rows {
        y0,
        cfa: inputs.store.cfa_rows(i, y0, y1)?,
        occluded: inputs.store.packed_rows(inputs.occlusion, i, y0, y1)?,
    }))
}

struct Pixel {
    rgb: [f32; 3],
    kept: [u32; 3],
    halves: [[f32; 3]; 2],
    clipped: bool,
}

fn pixel(inputs: &Inputs, rotations: &[Matrix], rows: &[Option<Rows>], q: usize) -> Pixel {
    let m = &inputs.store.manifest;
    let w = m.width;
    let none = Pixel {
        rgb: [f32::NAN; 3],
        kept: [0; 3],
        halves: [[f32::NAN; 3]; 2],
        clipped: false,
    };
    if inputs.alpha[q] <= 0.0 {
        return none;
    }
    let camera = &inputs.model.camera;
    let point = [(q % w) as f64, (q / w) as f64];
    let ray = camera.ray(point);
    let reference = inputs.glow[inputs.model.reference].sample(point);
    let (sigma, radius) = (inputs.kernel.sigma, inputs.kernel.radius);
    let mut samples: [Vec<Sample>; 3] = Default::default();
    let mut saturated = [0u32; 3];
    for (i, band) in rows.iter().enumerate() {
        let Some(band) = band else { continue };
        let Some(p) = camera.pixel(apply(&rotations[i], ray)) else {
            continue;
        };
        let glow = inputs.glow[i].sample(p);
        let white = m.frames[i].white;
        let places = [0, 1, 2].map(|c| inputs.model.chroma.place(camera, c, p));
        for site in 0..4 {
            let c = usize::from(m.cfa[site]);
            let p = places[c];
            let (dx, dy) = ((site % 2) as f64, (site / 2) as f64);
            let sx = dx + 2.0 * ((p[0] - dx) / 2.0).round_ties_even();
            let sy = dy + 2.0 * ((p[1] - dy) / 2.0).round_ties_even();
            let d2 = (sx - p[0]).powi(2) + (sy - p[1]).powi(2);
            if !(sx >= 0.0
                && sy >= 0.0
                && sx < w as f64
                && sy < m.height as f64
                && d2 <= radius * radius)
            {
                continue;
            }
            let (sx, sy) = (sx as usize, sy as usize);
            let s = sy * w + sx;
            if inputs.alpha[s] < CLEAR || inputs.clearance[s] < CLEARANCE_PX || sy < band.y0 {
                continue;
            }
            let local = (sy - band.y0) * w + sx;
            if local >= band.cfa.len() || band.occluded(w, sx, sy - band.y0) {
                continue;
            }
            let raw = f32::from(band.cfa[local]);
            if raw >= white {
                saturated[c] += 1;
                continue;
            }
            let value = (raw - m.frames[i].black[site]) / inputs.flat[s] - (glow[c] - reference[c]);
            if value.is_finite() {
                let weight = (-d2 / (2.0 * sigma * sigma)).exp() as f32;
                samples[c].push(Sample {
                    value,
                    weight,
                    frame: i as u32,
                });
            }
        }
    }
    let clipped = (0..3).any(|c| {
        let n = saturated[c] as f32;
        n > 0.0 && n >= CLIPPED_SHARE * (n + samples[c].len() as f32)
    });
    let mut out = Pixel { clipped, ..none };
    for (c, s) in samples.iter_mut().enumerate() {
        let (mean, kept, halves) = weighted(s);
        out.rgb[c] = mean;
        out.kept[c] = kept;
        out.halves[0][c] = halves[0];
        out.halves[1][c] = halves[1];
    }
    out
}
