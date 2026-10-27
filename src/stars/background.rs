//! Sky background and noise in meshes, after Source Extractor (sep is the oracle; written
//! from the Source Extractor documentation and black-box tests of sep): each mesh is clipped
//! at ±3σ about its median until stable, its level is the mode estimate 2.5·median − 1.5·mean
//! and its noise the clipped σ; the meshes are median-filtered 3×3 and natural cubic splines
//! through the mesh centres interpolate the level.

use crate::{Result, error};

pub const MESH: usize = 64;
const CLIP_SIGMA: f64 = 3.0;
const CLIP_ROUNDS: usize = 100;

pub struct Background {
    columns: usize,
    rows: usize,
    level: Vec<f64>,
    /// Median of the mesh noise: the unit of the detection threshold.
    pub rms: f64,
}

impl Background {
    pub fn estimate(image: &[f32], width: usize, height: usize) -> Result<Background> {
        if width == 0 || height == 0 || image.len() != width * height {
            return Err(error("background: image size does not match its samples"));
        }
        let (columns, rows) = (width.div_ceil(MESH), height.div_ceil(MESH));
        let mut level = Vec::with_capacity(columns * rows);
        let mut noise = Vec::with_capacity(columns * rows);
        let mut values = Vec::with_capacity(MESH * MESH);
        for j in 0..rows {
            for i in 0..columns {
                values.clear();
                for y in j * MESH..((j + 1) * MESH).min(height) {
                    let row = &image[y * width..(y + 1) * width];
                    values.extend(
                        row[i * MESH..((i + 1) * MESH).min(width)]
                            .iter()
                            .filter(|v| v.is_finite())
                            .map(|&v| f64::from(v)),
                    );
                }
                let (l, s) = clipped(&mut values)
                    .ok_or_else(|| error(format!("background mesh {i},{j} has no samples")))?;
                level.push(l);
                noise.push(s);
            }
        }
        let level = median3(&level, columns, rows);
        let mut noise = median3(&noise, columns, rows);
        let mid = noise.len() / 2;
        let rms = *noise.select_nth_unstable_by(mid, f64::total_cmp).1;
        Ok(Background {
            columns,
            rows,
            level,
            rms,
        })
    }

    /// `image` minus the interpolated background level.
    pub fn subtract(&self, image: &[f32], width: usize, height: usize) -> Vec<f32> {
        let mut rows = vec![0.0; height * self.columns];
        for i in 0..self.columns {
            let knots: Vec<f64> = (0..self.rows)
                .map(|j| self.level[j * self.columns + i])
                .collect();
            let bend = natural(&knots);
            for y in 0..height {
                rows[y * self.columns + i] = spline(&knots, &bend, position(y));
            }
        }
        let mut out = vec![0f32; width * height];
        for ((knots, pixels), samples) in rows
            .chunks(self.columns)
            .zip(out.chunks_mut(width))
            .zip(image.chunks(width))
        {
            let bend = natural(knots);
            for (x, (o, v)) in pixels.iter_mut().zip(samples).enumerate() {
                *o = (f64::from(*v) - spline(knots, &bend, position(x))) as f32;
            }
        }
        out
    }
}

/// Pixel index to mesh-centre coordinates (mesh i's centre is at i).
fn position(pixel: usize) -> f64 {
    (pixel as f64 - (MESH as f64 - 1.0) / 2.0) / MESH as f64
}

/// Mode estimate and σ of `values` after iterative ±3σ clipping about the median.
fn clipped(values: &mut [f64]) -> Option<(f64, f64)> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable_by(f64::total_cmp);
    let origin = values[values.len() / 2];
    let mut sums = Vec::with_capacity(values.len() + 1);
    sums.push((0.0, 0.0));
    for v in values.iter() {
        let (s, q) = sums[sums.len() - 1];
        let d = v - origin;
        sums.push((s + d, q + d * d));
    }
    let stats = |lo: usize, hi: usize| {
        let n = (hi - lo) as f64;
        let mean = (sums[hi].0 - sums[lo].0) / n;
        let var = ((sums[hi].1 - sums[lo].1) / n - mean * mean).max(0.0);
        let m = hi - lo;
        let median = if m % 2 == 1 {
            values[lo + m / 2]
        } else {
            (values[lo + m / 2 - 1] + values[lo + m / 2]) / 2.0
        };
        (median, mean + origin, var.sqrt())
    };
    let (mut lo, mut hi) = (0, values.len());
    let (mut median, mut mean, mut sigma) = stats(lo, hi);
    for _ in 0..CLIP_ROUNDS {
        let a = values.partition_point(|v| *v <= median - CLIP_SIGMA * sigma);
        let b = values.partition_point(|v| *v < median + CLIP_SIGMA * sigma);
        if b <= a || (a, b) == (lo, hi) {
            break;
        }
        (lo, hi) = (a, b);
        (median, mean, sigma) = stats(lo, hi);
    }
    Some((2.5 * median - 1.5 * mean, sigma))
}

/// 3×3 median of a mesh grid, edges replicated.
fn median3(grid: &[f64], columns: usize, rows: usize) -> Vec<f64> {
    let at = |i: isize, j: isize| {
        let i = i.clamp(0, columns as isize - 1) as usize;
        let j = j.clamp(0, rows as isize - 1) as usize;
        grid[j * columns + i]
    };
    let mut out = Vec::with_capacity(grid.len());
    for j in 0..rows as isize {
        for i in 0..columns as isize {
            let mut window: Vec<f64> = (-1..=1)
                .flat_map(|dj| (-1..=1).map(move |di| (di, dj)))
                .map(|(di, dj)| at(i + di, j + dj))
                .collect();
            out.push(*window.select_nth_unstable_by(4, f64::total_cmp).1);
        }
    }
    out
}

/// Second derivatives of the natural cubic spline through unit-spaced `knots`.
fn natural(knots: &[f64]) -> Vec<f64> {
    let n = knots.len();
    let mut bend = vec![0.0; n];
    if n < 3 {
        return bend;
    }
    let mut diag = vec![4.0; n];
    let mut rhs: Vec<f64> = (0..n)
        .map(|i| {
            if i == 0 || i == n - 1 {
                0.0
            } else {
                6.0 * (knots[i + 1] - 2.0 * knots[i] + knots[i - 1])
            }
        })
        .collect();
    for i in 2..n - 1 {
        let m = 1.0 / diag[i - 1];
        diag[i] -= m;
        rhs[i] -= m * rhs[i - 1];
    }
    for i in (1..n - 1).rev() {
        let next = if i + 1 < n - 1 { bend[i + 1] } else { 0.0 };
        bend[i] = (rhs[i] - next) / diag[i];
    }
    bend
}

fn spline(knots: &[f64], bend: &[f64], t: f64) -> f64 {
    let n = knots.len();
    if n == 1 {
        return knots[0];
    }
    let i = (t.floor() as isize).clamp(0, n as isize - 2) as usize;
    let u = t - i as f64;
    let v = 1.0 - u;
    v * knots[i]
        + u * knots[i + 1]
        + ((v * v * v - v) * bend[i] + (u * u * u - u) * bend[i + 1]) / 6.0
}
