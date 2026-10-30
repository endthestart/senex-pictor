//! Per-pixel temporal statistics and star transits that move with the sky (ADR 0009).
//!
//! A > TRANSIT_SIGMA excess at a camera pixel is a star transit only when the same sky point,
//! looked up in a frame where the sky has moved it at least MIN_SHIFT px, shows a
//! ≥ PARTNER_SIGMA peak there while the original pixel has dropped below half that peak.
//! Each frame's large-scale brightness change (block medians) is removed first, so twilight,
//! aurora and passing cloud do not count as events.

use super::grid::{BlockGrid, TREND_BLOCK};
pub use super::order::{median, percentile};
pub use super::velocity::{frame_steps, sky_velocity};
use crate::budget::Budget;
use crate::sky::SkyModel;
use crate::store::Store;
use crate::{Result, error};
use rayon::prelude::*;

pub const TRANSIT_SIGMA: f32 = 6.0;
pub const PARTNER_SIGMA: f32 = 3.0;
pub const MIN_SHIFT_PX: f64 = 2.5;
pub const STEPS: [usize; 6] = [1, 2, 4, 8, 16, 32];
pub const CELL: usize = 16;
const HALO: usize = 8;
const STRIP: usize = 32;
/// Working set of one frame's trend per pixel: the RGB read (f16 and f32) and its sum.
const TREND_BYTES_PER_PIXEL: u64 = 24;

/// Per-pixel planes, row-major.
pub struct Stats {
    pub width: usize,
    pub height: usize,
    pub median: Vec<f32>,
    /// 20th percentile of luminance over frames.
    pub low: Vec<f32>,
    /// Robust temporal noise after removing each frame's large-scale trend.
    pub sigma: Vec<f32>,
    /// Share of frames with a > TRANSIT_SIGMA excess.
    pub excess: Vec<f32>,
    /// Share of frames with a sky-coherent transit (0 unless half the events cohere).
    pub transit: Vec<f32>,
    /// Temporal standard deviation after removing the trend.
    pub scatter: Vec<f32>,
    /// Whether the sky moves enough here for transits to be confirmed.
    pub resolvable: Vec<bool>,
    /// Shares of red and blue in the light summed over frames (camera RGB).
    pub red: Vec<f32>,
    pub blue: Vec<f32>,
}

/// Statistics over the store's `frames` (a window around a hero frame, or all of them).
pub fn temporal_stats(
    store: &Store,
    model: &SkyModel,
    frames: std::ops::Range<usize>,
    budget: &Budget,
) -> Result<Stats> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    if model.angles.len() != store.manifest.frames.len() {
        return Err(error("the sky model and the store differ in frame count"));
    }
    if frames.is_empty() || frames.end > model.angles.len() {
        return Err(error("temporal statistics: frame range outside the store"));
    }
    let angles = &model.angles[frames.clone()];
    let held = Budget::held(w * h);
    let pool = budget.pool("frame trends", held, TREND_BYTES_PER_PIXEL * (w * h) as u64)?;
    budget
        .progress
        .step("Measuring each frame's brightness", frames.len());
    let trends: Vec<BlockGrid> = pool.install(|| {
        frames
            .clone()
            .into_par_iter()
            .map(|i| {
                let grid = BlockGrid::medians(&store.luminance(i)?, w, h, TREND_BLOCK);
                budget.progress.advance()?;
                Ok(grid)
            })
            .collect::<Result<_>>()
    })?;
    let velocity = sky_velocity(model, w, h);
    let steps = frame_steps(&velocity, angles);
    let starts: Vec<usize> = (0..h).step_by(STRIP).collect();
    let context = Context {
        store,
        first: frames.start,
        angles,
        trends: &trends,
        velocity: &velocity,
        steps: &steps,
    };
    let strip_bytes = 2 * (frames.len() * (STRIP + 2 * HALO) * w * 4) as u64;
    let pool = budget.pool("temporal statistics", held, strip_bytes)?;
    budget
        .progress
        .step("Following each pixel through the night", starts.len());
    let strips: Vec<Strip> = pool.install(|| {
        starts
            .par_iter()
            .map(|&y0| {
                let s = strip(&context, y0, (y0 + STRIP).min(h))?;
                budget.progress.advance()?;
                Ok(s)
            })
            .collect::<Result<_>>()
    })?;
    let mut out = Stats {
        width: w,
        height: h,
        median: Vec::with_capacity(w * h),
        low: Vec::with_capacity(w * h),
        sigma: Vec::with_capacity(w * h),
        excess: Vec::with_capacity(w * h),
        transit: Vec::with_capacity(w * h),
        scatter: Vec::with_capacity(w * h),
        resolvable: (0..w * h)
            .map(|p| steps[(p / w / CELL) * w.div_ceil(CELL) + (p % w) / CELL] > 0)
            .collect(),
        red: Vec::with_capacity(w * h),
        blue: Vec::with_capacity(w * h),
    };
    for s in strips {
        out.median.extend(s.median);
        out.low.extend(s.low);
        out.sigma.extend(s.sigma);
        out.excess.extend(s.excess);
        out.transit.extend(s.transit);
        out.scatter.extend(s.scatter);
        for c in s.colour {
            let total = (c[0] + c[1] + c[2]).max(f32::MIN_POSITIVE);
            out.red.push(c[0] / total);
            out.blue.push(c[2] / total);
        }
    }
    Ok(out)
}

struct Strip {
    median: Vec<f32>,
    low: Vec<f32>,
    sigma: Vec<f32>,
    excess: Vec<f32>,
    transit: Vec<f32>,
    scatter: Vec<f32>,
    colour: Vec<[f32; 3]>,
}

/// What every strip shares: the store, the first frame used and that range's angles, frame
/// trends, sky velocity and frame steps per cell.
struct Context<'a> {
    store: &'a Store,
    first: usize,
    angles: &'a [f64],
    trends: &'a [BlockGrid],
    velocity: &'a [[f32; 2]],
    steps: &'a [usize],
}

fn strip(c: &Context, y0: usize, y1: usize) -> Result<Strip> {
    let Context {
        store,
        first,
        angles,
        trends,
        velocity,
        steps,
    } = *c;
    let (w, h, n) = (store.manifest.width, store.manifest.height, trends.len());
    let (ya, yb) = (y0.saturating_sub(HALO), (y1 + HALO).min(h));
    let plane = (yb - ya) * w;
    let core = (y0 - ya) * w..(y1 - ya) * w;
    let mut v = Vec::with_capacity(n * plane);
    let mut colour = vec![[0f32; 3]; core.len()];
    for i in first..first + n {
        let rgb = store.rgb_rows(i, ya, yb)?;
        let pixels = rgb.as_chunks::<3>().0;
        for (sum, p) in colour.iter_mut().zip(&pixels[core.clone()]) {
            *sum = std::array::from_fn(|k| sum[k] + p[k].max(0.0));
        }
        v.extend(pixels.iter().map(|p| p[0] + p[1] + p[2]));
    }
    let mut buf = vec![0f32; n];
    let gather = |v: &[f32], p: usize, buf: &mut [f32]| {
        for (f, b) in buf.iter_mut().enumerate() {
            *b = v[f * plane + p];
        }
    };
    let (mut med, mut low) = (
        Vec::with_capacity(core.len()),
        Vec::with_capacity(core.len()),
    );
    for p in core.clone() {
        gather(&v, p, &mut buf);
        med.push(median(&mut buf));
        low.push(percentile(&mut buf, 20.0));
    }
    for p in 0..plane {
        let (x, y) = (p % w, ya + p / w);
        for (f, b) in buf.iter_mut().enumerate() {
            *b = trends[f].at(x, y);
        }
        let typical = median(&mut buf.clone());
        for (f, b) in buf.iter().enumerate() {
            v[f * plane + p] += typical - b;
        }
    }
    let (mut sigma, mut scatter) = (
        Vec::with_capacity(core.len()),
        Vec::with_capacity(core.len()),
    );
    for p in 0..plane {
        gather(&v, p, &mut buf);
        let mean = buf.iter().map(|&x| f64::from(x)).sum::<f64>() / n as f64;
        let var = buf
            .iter()
            .map(|&x| (f64::from(x) - mean).powi(2))
            .sum::<f64>()
            / n as f64;
        let m = median(&mut buf);
        for b in buf.iter_mut() {
            *b = (*b - m).abs();
        }
        let s = 1.4826 * median(&mut buf);
        if core.contains(&p) {
            sigma.push(s);
            scatter.push(var.sqrt() as f32);
        }
        for f in 0..n {
            let q = f * plane + p;
            v[q] = (v[q] - m) / s.max(1e-3);
        }
    }
    let rows = yb - ya;
    let mut peak = vec![0f32; n * plane];
    for f in 0..n {
        for r in 0..rows {
            for x in 0..w {
                let mut m = f32::NEG_INFINITY;
                for rr in r.saturating_sub(1)..(r + 2).min(rows) {
                    for xx in x.saturating_sub(1)..(x + 2).min(w) {
                        m = m.max(v[f * plane + rr * w + xx]);
                    }
                }
                peak[f * plane + r * w + x] = m;
            }
        }
    }
    let gw = w.div_ceil(CELL);
    let (mut excess, mut transit) = (
        Vec::with_capacity(core.len()),
        Vec::with_capacity(core.len()),
    );
    for p in core {
        let (x, y) = (p % w, ya + p / w);
        let cell = (y / CELL) * gw + x / CELL;
        let (step, vel) = (steps[cell], velocity[cell]);
        let (mut events, mut hits) = (0usize, 0usize);
        for f in 0..n {
            if v[f * plane + p] <= TRANSIT_SIGMA {
                continue;
            }
            events += 1;
            let coherent = step > 0
                && [-1isize, 1].iter().any(|&sign| {
                    let other = f as isize + sign * step as isize;
                    if other < 0 || other >= n as isize {
                        return false;
                    }
                    let other = other as usize;
                    let turn = (angles[other] - angles[f]) as f32;
                    let ty = (y as f64 + f64::from(turn * vel[1])).round_ties_even();
                    let tx = (x as f64 + f64::from(turn * vel[0])).round_ties_even();
                    if ty < ya as f64 || ty >= yb as f64 || tx < 0.0 || tx >= w as f64 {
                        return false;
                    }
                    let there = peak[other * plane + (ty as usize - ya) * w + tx as usize];
                    let here = v[other * plane + p];
                    there >= PARTNER_SIGMA && here < 0.5 * there
                });
            hits += usize::from(coherent);
        }
        excess.push(events as f32 / n as f32);
        transit.push(if 2 * hits >= events {
            hits as f32 / n as f32
        } else {
            0.0
        });
    }
    Ok(Strip {
        median: med,
        low,
        sigma,
        excess,
        transit,
        scatter,
        colour,
    })
}
