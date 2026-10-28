//! Foreground stacks (ADR 0009, 0010). The long-exposure ground is a robust temporal mean in
//! which each frame is resampled through its foreground turn (the camera rotation that moved
//! its tracked foreground corners), so camera drift moves the foreground back into place
//! instead of smearing it; frames whose foreground stayed within STILL_PX are used as they
//! are. Also the plain mean and the median of a few frames.

use super::combine::luminance_clipped;
use crate::budget::Budget;
use crate::flow::displacement::Displacement;
use crate::flow::rotation::Turn;
use crate::separate::stats::median;
use crate::sky::Camera;
use crate::sky::camera::Vec3;
use crate::sky::model::apply;
use crate::store::Store;
use crate::{Result, error};
use rayon::prelude::*;

const STRIP: usize = 16;
/// Per frame pixel of a strip: its RGB as read (f16 and f32) and as sampled.
const BYTES_PER_SAMPLE: usize = 30;
/// A frame whose corners moved less than this (px, median) is not moved.
pub const STILL_PX: f64 = 0.5;
/// Spacing of the points where a displacement is checked against STILL_PX.
const PROBE_STEP: usize = 16;

/// One frame of a foreground stack and its foreground turn from the reference, if it moved.
#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub frame: usize,
    pub moved: Option<&'a Turn>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Combine {
    /// Mean of the frames whose luminance survives σ-clipping.
    Clipped,
    Mean,
}

/// Per pixel the stacked RGB (divided by the flat field), kept and valid frame counts.
pub type Ground = (Vec<[f32; 3]>, Vec<u32>, Vec<u32>);

/// The largest move (px) of `turn` over `ground`, probed every PROBE_STEP px.
pub fn widest(turn: &Turn, camera: &Camera, ground: &[bool], w: usize) -> f64 {
    let h = ground.len() / w;
    (0..h)
        .step_by(PROBE_STEP)
        .flat_map(|y| (0..w).step_by(PROBE_STEP).map(move |x| (x, y)))
        .filter(|&(x, y)| ground[y * w + x])
        .map(|(x, y)| {
            let p = [x as f64, y as f64];
            turn.at(camera, p)
                .map_or(f64::INFINITY, |q| (q[0] - p[0]).hypot(q[1] - p[1]))
        })
        .fold(0.0, f64::max)
}

/// Frame `frame`, moved by its camera turn when its corners' median motion exceeds STILL_PX.
pub fn moved(frame: usize, displacement: &Displacement) -> Source<'_> {
    let [x, y] = displacement.median_px;
    Source {
        frame,
        moved: displacement.turn.as_ref().filter(|_| x.hypot(y) > STILL_PX),
    }
}

/// `sources` stacked per pixel by `combine`.
pub fn stack(
    store: &Store,
    flat: &[f32],
    camera: &Camera,
    sources: &[Source],
    combine: Combine,
    budget: &Budget,
) -> Result<Ground> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    if sources.is_empty() {
        return Err(error("foreground stack: no frames"));
    }
    let each = (sources.len() * STRIP * w * BYTES_PER_SAMPLE) as u64;
    let pool = budget.pool("the ground stack", Budget::held(w * h), each)?;
    budget
        .progress
        .step("Stacking the foreground", h.div_ceil(STRIP));
    let strips: Vec<Vec<([f32; 3], u32, u32)>> = pool.install(|| {
        (0..h)
            .step_by(STRIP)
            .collect::<Vec<_>>()
            .par_iter()
            .map(|&y0| {
                budget.progress.advance()?;
                strip(
                    store,
                    flat,
                    camera,
                    sources,
                    combine,
                    y0,
                    (y0 + STRIP).min(h),
                )
            })
            .collect::<Result<_>>()
    })?;
    let mut out = (
        Vec::with_capacity(w * h),
        Vec::with_capacity(w * h),
        Vec::with_capacity(w * h),
    );
    for (m, k, v) in strips.into_iter().flatten() {
        out.0.push(m);
        out.1.push(k);
        out.2.push(v);
    }
    Ok(out)
}

fn strip(
    store: &Store,
    flat: &[f32],
    camera: &Camera,
    sources: &[Source],
    combine: Combine,
    y0: usize,
    y1: usize,
) -> Result<Vec<([f32; 3], u32, u32)>> {
    let w = store.manifest.width;
    let pixels = (y1 - y0) * w;
    let rays: Vec<Vec3> = if sources.iter().any(|s| s.moved.is_some()) {
        (y0 * w..y1 * w)
            .map(|p| camera.ray([(p % w) as f64, (p / w) as f64]))
            .collect()
    } else {
        Vec::new()
    };
    let frames: Vec<Vec<[f32; 3]>> = sources
        .iter()
        .map(|s| match s.moved {
            None => Ok(store.rgb_rows(s.frame, y0, y1)?.as_chunks::<3>().0.to_vec()),
            Some(t) => resampled(store, flat, camera, &rays, s.frame, t, y0),
        })
        .collect::<Result<_>>()?;
    let mut samples = vec![[0f32; 3]; frames.len()];
    Ok((0..pixels)
        .map(|p| {
            for (s, f) in samples.iter_mut().zip(&frames) {
                *s = f[p];
            }
            let f = flat[y0 * w + p];
            let (value, kept, valid) = match combine {
                Combine::Clipped => luminance_clipped(&samples),
                Combine::Mean => mean(&samples),
            };
            (value.map(|v| v / f), kept, valid)
        })
        .collect())
}

fn mean(samples: &[[f32; 3]]) -> ([f32; 3], u32, u32) {
    let valid: Vec<&[f32; 3]> = samples
        .iter()
        .filter(|c| c.iter().all(|v| v.is_finite()))
        .collect();
    let count = valid.len() as u32;
    let mut sum = [0f64; 3];
    for c in &valid {
        for k in 0..3 {
            sum[k] += f64::from(c[k]);
        }
    }
    let value = if count > 0 {
        sum.map(|s| (s / f64::from(count)) as f32)
    } else {
        [f32::NAN; 3]
    };
    (value, count, count)
}

/// The output pixels with reference `rays` (from row y0) resampled (bilinear) from frame `i`
/// where `turn` moved them, scaled to the output pixel's flat field; NaN where the moved
/// point is outside the frame.
fn resampled(
    store: &Store,
    flat: &[f32],
    camera: &Camera,
    rays: &[Vec3],
    i: usize,
    turn: &Turn,
    y0: usize,
) -> Result<Vec<[f32; 3]>> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    let at: Vec<[f64; 2]> = rays
        .iter()
        .map(|r| {
            camera
                .pixel(apply(&turn.matrix, *r))
                .unwrap_or([f64::NAN, f64::NAN])
        })
        .collect();
    let (lo, hi) = at
        .iter()
        .filter(|q| q[1].is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), q| {
            (lo.min(q[1]), hi.max(q[1]))
        });
    if lo > hi {
        return Ok(vec![[f32::NAN; 3]; rays.len()]);
    }
    let ra = (lo.floor().max(0.0) as usize).min(h - 1);
    let rb = ((hi.floor() + 2.0).max(0.0) as usize).clamp(ra + 1, h);
    let rows = store.rgb_rows(i, ra, rb)?;
    Ok(at
        .iter()
        .enumerate()
        .map(|(k, q)| {
            let (x, y) = (q[0], q[1]);
            if !(x >= 0.0 && y >= 0.0 && x <= (w - 1) as f64 && y <= (h - 1) as f64) {
                return [f32::NAN; 3];
            }
            let (x0, yy) = (x.floor() as usize, y.floor() as usize);
            let (x1, y1) = ((x0 + 1).min(w - 1), (yy + 1).min(h - 1));
            let (fx, fy) = ((x - x0 as f64) as f32, (y - yy as f64) as f32);
            let v = |xi: usize, yi: usize, c: usize| rows[3 * ((yi - ra) * w + xi) + c];
            let source = flat[y.round() as usize * w + x.round() as usize];
            let scale = flat[y0 * w + k] / source;
            std::array::from_fn(|c| {
                let top = v(x0, yy, c) * (1.0 - fx) + v(x1, yy, c) * fx;
                let bottom = v(x0, y1, c) * (1.0 - fx) + v(x1, y1, c) * fx;
                (top * (1.0 - fy) + bottom * fy) * scale
            })
        })
        .collect())
}

/// Per-pixel median of the frames in `range` (camera RGB divided by `flat`).
pub fn median_of(
    store: &Store,
    flat: &[f32],
    range: std::ops::Range<usize>,
    budget: &Budget,
) -> Result<Vec<[f32; 3]>> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    let k = range.len();
    if k == 0 || range.end > store.manifest.frames.len() {
        return Err(error("median: frame range outside the store"));
    }
    let each = (k * STRIP * w * BYTES_PER_SAMPLE) as u64;
    let pool = budget.pool("the window median", Budget::held(w * h), each)?;
    budget.progress.step(
        "Stacking the foreground around one frame",
        h.div_ceil(STRIP),
    );
    let strips: Vec<Vec<[f32; 3]>> = pool.install(|| {
        (0..h)
            .step_by(STRIP)
            .collect::<Vec<_>>()
            .par_iter()
            .map(|&y0| {
                budget.progress.advance()?;
                let y1 = (y0 + STRIP).min(h);
                let rows: Vec<Vec<f32>> = range
                    .clone()
                    .map(|i| store.rgb_rows(i, y0, y1))
                    .collect::<Result<_>>()?;
                let mut buf = vec![0f32; k];
                Ok((0..(y1 - y0) * w)
                    .map(|p| {
                        std::array::from_fn(|c| {
                            for (b, r) in buf.iter_mut().zip(&rows) {
                                *b = r[3 * p + c];
                            }
                            median(&mut buf) / flat[y0 * w + p]
                        })
                    })
                    .collect())
            })
            .collect::<Result<_>>()
    })?;
    Ok(strips.concat())
}
