//! Per-frame occlusion of sky pixels (ADR 0009). Moving foreground or dark cloud can cover a
//! sky pixel in single frames: its 5×5-smoothed, glow-corrected luminance falls more than
//! DARK_SIGMA robust σ below its own history. Local light that is not sky is the mirror case:
//! far above the history over a region larger than any star. Flagged pixels are dilated by
//! the resampling kernel's reach and kept as one bit per frame and pixel.

use super::glow::Grid;
use super::stats::median;
use crate::budget::Budget;
use crate::store::{Store, file};
use crate::{Result, error};
use rayon::prelude::*;

const SMOOTH: usize = 5;
pub const DARK_SIGMA: f32 = 5.0;
pub const BRIGHT_SIGMA: f32 = 8.0;
pub const BRIGHT_SIZE: usize = 15;
pub const DILATE: usize = 4;
const HALO: usize = SMOOTH / 2 + DILATE + BRIGHT_SIZE;
const STRIP: usize = 32;
pub const MIN_ALPHA: f32 = 0.5;
/// Per pixel of a strip besides the frames' values: the median and spread, one frame's
/// smoothing, bright and flagged masks and their morphology.
const WORK_BYTES_PER_PIXEL: u64 = 40;

/// Box mean of size SMOOTH with edges replicated, per plane of `rows`×`w` (running sums).
fn smooth(v: &[f32], w: usize, rows: usize) -> Vec<f32> {
    let r = SMOOTH / 2;
    let line = |get: &dyn Fn(usize) -> f32, n: usize, out: &mut dyn FnMut(usize, f32)| {
        let at = |i: isize| get(i.clamp(0, n as isize - 1) as usize);
        let mut sum: f32 = (-(r as isize)..=r as isize).map(at).sum();
        for i in 0..n {
            out(i, sum / SMOOTH as f32);
            sum += at(i as isize + r as isize + 1) - at(i as isize - r as isize);
        }
    };
    let mut horizontal = vec![0f32; w * rows];
    for y in 0..rows {
        line(&|x| v[y * w + x], w, &mut |x, s| horizontal[y * w + x] = s);
    }
    let mut out = vec![0f32; w * rows];
    for x in 0..w {
        line(&|y| horizontal[y * w + x], rows, &mut |y, s| {
            out[y * w + x] = s
        });
    }
    out
}

/// Square erosion or dilation of radius `r` with outside pixels counted as background
/// (running counts of set pixels along rows, then columns).
fn square(mask: &[bool], w: usize, rows: usize, r: usize, grow: bool) -> Vec<bool> {
    let pass = |m: &[bool], along_x: bool| -> Vec<bool> {
        let (n, lines) = if along_x { (w, rows) } else { (rows, w) };
        let index = |line: usize, i: usize| if along_x { line * w + i } else { i * w + line };
        let mut out = vec![false; w * rows];
        for line in 0..lines {
            let mut count = (0..=r.min(n - 1)).filter(|&i| m[index(line, i)]).count();
            for i in 0..n {
                let full = i >= r && i + r < n;
                out[index(line, i)] = if grow {
                    count > 0
                } else {
                    full && count == 2 * r + 1
                };
                if i + r + 1 < n && m[index(line, i + r + 1)] {
                    count += 1;
                }
                if i >= r && m[index(line, i - r)] {
                    count -= 1;
                }
            }
        }
        out
    };
    pass(&pass(mask, true), false)
}

/// Writes `occ/{index}.occ` bit masks; returns each frame's occluded share of sky pixels.
pub fn build(
    store: &Store,
    glow: &[Grid],
    alpha: &[f32],
    flat: &[f32],
    budget: &Budget,
) -> Result<Vec<f64>> {
    let (w, h, n) = (
        store.manifest.width,
        store.manifest.height,
        store.manifest.frames.len(),
    );
    if glow.len() != n {
        return Err(error("occlusion: glow and frames differ in count"));
    }
    let sky: Vec<bool> = alpha.iter().map(|&a| a >= MIN_ALPHA).collect();
    let dir = store.dir.join("occ");
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    let row_bytes = file::bit_row(w);
    let paths: Vec<_> = (0..n).map(|i| dir.join(format!("{i:05}.occ"))).collect();
    for path in &paths {
        file::create_zeroed(path, (h * row_bytes) as u64)?;
    }
    let rows = (STRIP + 2 * HALO) as u64;
    let each = rows * w as u64 * (4 * n as u64 + WORK_BYTES_PER_PIXEL) + (n * STRIP * w) as u64;
    let pool = budget.pool("occlusion", Budget::held(w * h), each)?;
    budget
        .progress
        .step("Finding where the ground hides the sky", h.div_ceil(STRIP));
    let counts: Vec<Vec<usize>> = pool.install(|| {
        (0..h)
            .step_by(STRIP)
            .collect::<Vec<_>>()
            .par_iter()
            .map(|&y0| {
                budget.progress.advance()?;
                let masks = strip(store, glow, &sky, flat, y0, (y0 + STRIP).min(h))?;
                masks
                    .iter()
                    .zip(&paths)
                    .map(|(mask, path)| {
                        file::write_at(path, (y0 * row_bytes) as u64, &file::pack_bits(mask, w))?;
                        Ok(mask
                            .iter()
                            .zip(&sky[y0 * w..])
                            .filter(|(m, s)| **m && **s)
                            .count())
                    })
                    .collect::<Result<Vec<usize>>>()
            })
            .collect::<Result<_>>()
    })?;
    for path in &paths {
        file::sync(path)?;
    }
    let total = sky.iter().filter(|&&s| s).count().max(1) as f64;
    Ok((0..n)
        .map(|i| counts.iter().map(|c| c[i]).sum::<usize>() as f64 / total)
        .collect())
}

fn strip(
    store: &Store,
    glow: &[Grid],
    sky: &[bool],
    flat: &[f32],
    y0: usize,
    y1: usize,
) -> Result<Vec<Vec<bool>>> {
    let (w, h, n) = (store.manifest.width, store.manifest.height, glow.len());
    let (ya, yb) = (y0.saturating_sub(HALO), (y1 + HALO).min(h));
    let rows = yb - ya;
    let plane = rows * w;
    let mut values = Vec::with_capacity(n * plane);
    for (i, g) in glow.iter().enumerate() {
        let rgb = store.rgb_rows(i, ya, yb)?;
        let lum: Vec<f32> = rgb
            .as_chunks::<3>()
            .0
            .iter()
            .enumerate()
            .map(|(p, c)| (c[0] + c[1] + c[2]) / flat[ya * w + p])
            .collect();
        let s = smooth(&lum, w, rows);
        let glow = g.luminance_rows(ya, yb, w);
        values.extend(s.iter().zip(&glow).map(|(v, g)| v - g));
    }
    let (mut centre, mut sigma) = (vec![0f32; plane], vec![0f32; plane]);
    let mut buf = vec![0f32; n];
    for p in 0..plane {
        for (f, b) in buf.iter_mut().enumerate() {
            *b = values[f * plane + p];
        }
        let m = median(&mut buf);
        for b in buf.iter_mut() {
            *b = (*b - m).abs();
        }
        centre[p] = m;
        sigma[p] = (1.4826 * median(&mut buf)).max(1e-3);
    }
    let core = (y0 - ya) * w..(y1 - ya) * w;
    let r = BRIGHT_SIZE / 2;
    Ok((0..n)
        .map(|f| {
            let v = &values[f * plane..(f + 1) * plane];
            let in_sky = |p: usize| sky[ya * w + p];
            let bright: Vec<bool> = (0..plane)
                .map(|p| v[p] > centre[p] + BRIGHT_SIGMA * sigma[p])
                .collect();
            let bright = square(&square(&bright, w, rows, r, false), w, rows, r, true);
            let flagged: Vec<bool> = (0..plane)
                .map(|p| in_sky(p) && (v[p] < centre[p] - DARK_SIGMA * sigma[p] || bright[p]))
                .collect();
            square(&flagged, w, rows, DILATE, true)[core.clone()].to_vec()
        })
        .collect())
}
