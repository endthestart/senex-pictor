//! Coarse grids over an image: block medians with bilinear lookup, nearest-value filling,
//! Gaussian smoothing and bilinear resizing (scipy.ndimage and OpenCV semantics, the
//! prototype's oracles).

use super::filters::nearest_known;

/// Block size of each frame's large-scale brightness trend.
pub const TREND_BLOCK: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct BlockGrid {
    pub columns: usize,
    pub rows: usize,
    pub block: usize,
    pub values: Vec<f32>,
}

impl BlockGrid {
    /// Medians of whole `block`×`block` blocks (a partial last row or column is dropped).
    pub fn medians(image: &[f32], width: usize, height: usize, block: usize) -> BlockGrid {
        let (columns, rows) = (width / block, height / block);
        let mut values = Vec::with_capacity(columns * rows);
        let mut buf = Vec::with_capacity(block * block);
        for j in 0..rows {
            for i in 0..columns {
                buf.clear();
                for y in j * block..(j + 1) * block {
                    buf.extend_from_slice(
                        &image[y * width + i * block..y * width + (i + 1) * block],
                    );
                }
                values.push(super::stats::median(&mut buf));
            }
        }
        BlockGrid {
            columns,
            rows,
            block,
            values,
        }
    }

    /// Bilinear value at pixel (x, y), block centres at (k + 0.5)·block − 0.5, clamped.
    pub fn at(&self, x: usize, y: usize) -> f32 {
        let b = self.block as f64;
        let gx = ((x as f64 + 0.5) / b - 0.5).clamp(0.0, (self.columns - 1) as f64);
        let gy = ((y as f64 + 0.5) / b - 0.5).clamp(0.0, (self.rows - 1) as f64);
        let (x0, y0) = (gx.floor() as usize, gy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.columns - 1), (y0 + 1).min(self.rows - 1));
        let (fx, fy) = (gx - x0 as f64, gy - y0 as f64);
        let v = |i: usize, j: usize| f64::from(self.values[j * self.columns + i]);
        let top = v(x0, y0) * (1.0 - fx) + v(x1, y0) * fx;
        let bottom = v(x0, y1) * (1.0 - fx) + v(x1, y1) * fx;
        (top * (1.0 - fy) + bottom * fy) as f32
    }
}

/// Cells without a value take their nearest valued cell's (None when no cell has one).
pub fn fill_nearest(values: &[Option<f64>], columns: usize, rows: usize) -> Option<Vec<f64>> {
    let known: Vec<bool> = values.iter().map(Option::is_some).collect();
    let nearest = nearest_known(&known, columns, rows)?;
    nearest.iter().map(|&k| values[k]).collect()
}

/// scipy.ndimage.gaussian_filter (truncate 4); `nearest` replicates edges, else reflects.
pub fn gaussian(grid: &[f64], columns: usize, rows: usize, sigma: f64, nearest: bool) -> Vec<f64> {
    let radius = (4.0 * sigma + 0.5) as isize;
    let mut kernel: Vec<f64> = (-radius..=radius)
        .map(|k| (-(k * k) as f64 / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f64 = kernel.iter().sum();
    kernel.iter_mut().for_each(|k| *k /= total);
    let index = |i: isize, n: usize| -> usize {
        let n = n as isize;
        if nearest {
            return i.clamp(0, n - 1) as usize;
        }
        let mut i = i;
        loop {
            if i < 0 {
                i = -i - 1;
            } else if i >= n {
                i = 2 * n - i - 1;
            } else {
                return i as usize;
            }
        }
    };
    let pass = |src: &[f64], along_x: bool| -> Vec<f64> {
        (0..columns * rows)
            .map(|p| {
                let (x, y) = ((p % columns) as isize, (p / columns) as isize);
                kernel
                    .iter()
                    .enumerate()
                    .map(|(k, w)| {
                        let d = k as isize - radius;
                        let q = if along_x {
                            y as usize * columns + index(x + d, columns)
                        } else {
                            index(y + d, rows) * columns + x as usize
                        };
                        w * src[q]
                    })
                    .sum()
            })
            .collect()
    };
    pass(&pass(grid, true), false)
}

/// OpenCV INTER_LINEAR resize of a `columns`×`rows` grid to `width`×`height`.
pub fn resize(grid: &[f64], columns: usize, rows: usize, width: usize, height: usize) -> Vec<f32> {
    let axis = |n_out: usize, n_in: usize| -> Vec<(usize, usize, f64)> {
        let scale = n_in as f64 / n_out as f64;
        (0..n_out)
            .map(|d| {
                let s = (d as f64 + 0.5) * scale - 0.5;
                let s0 = s.floor();
                let (mut i0, mut f) = (s0 as isize, s - s0);
                if i0 < 0 {
                    (i0, f) = (0, 0.0);
                }
                if i0 as usize >= n_in - 1 {
                    (i0, f) = (n_in as isize - 1, 0.0);
                }
                let i0 = i0 as usize;
                (i0, (i0 + 1).min(n_in - 1), f)
            })
            .collect()
    };
    let (xs, ys) = (axis(width, columns), axis(height, rows));
    let mut out = Vec::with_capacity(width * height);
    for &(y0, y1, fy) in &ys {
        for &(x0, x1, fx) in &xs {
            let v = |i: usize, j: usize| grid[j * columns + i];
            let top = v(x0, y0) * (1.0 - fx) + v(x1, y0) * fx;
            let bottom = v(x0, y1) * (1.0 - fx) + v(x1, y1) * fx;
            out.push((top * (1.0 - fy) + bottom * fy) as f32);
        }
    }
    out
}
