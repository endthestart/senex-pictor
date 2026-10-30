//! RGB values on the BLOCK×BLOCK cell grid of the glow model, sampled bilinearly at
//! full-resolution positions.

use super::glow::BLOCK;
use crate::stars::similarity::Point;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// RGB per BLOCK cell, row-major; NaN where unknown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    pub rgb: Vec<[f32; 3]>,
}

impl Grid {
    /// `sample` at every pixel of a `w`×`h` image, row-major.
    pub fn full(&self, w: usize, h: usize) -> Vec<[f32; 3]> {
        (0..w * h)
            .into_par_iter()
            .map(|p| self.sample([(p % w) as f64, (p / w) as f64]))
            .collect()
    }

    /// Bilinear value at a full-resolution position: positions within one cell of the grid
    /// edge take the edge value, farther ones and unknown neighbourhoods are NaN.
    pub fn sample(&self, p: Point) -> [f32; 3] {
        let nan = [f32::NAN; 3];
        let b = BLOCK as f64;
        let (gx, gy) = ((p[0] - (b - 1.0) / 2.0) / b, (p[1] - (b - 1.0) / 2.0) / b);
        let (cols, rows) = (self.columns as f64, self.rows as f64);
        if !(gx >= -1.0 && gy >= -1.0 && gx <= cols && gy <= rows) {
            return nan;
        }
        let (gx, gy) = (gx.clamp(0.0, cols - 1.0), gy.clamp(0.0, rows - 1.0));
        let (x0, y0) = (gx.floor() as usize, gy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.columns - 1), (y0 + 1).min(self.rows - 1));
        let (fx, fy) = (gx - x0 as f64, gy - y0 as f64);
        let (mut value, mut norm) = ([0f64; 3], 0f64);
        for (i, j, w) in [
            (x0, y0, (1.0 - fx) * (1.0 - fy)),
            (x1, y0, fx * (1.0 - fy)),
            (x0, y1, (1.0 - fx) * fy),
            (x1, y1, fx * fy),
        ] {
            let c = self.rgb[j * self.columns + i];
            if c[0].is_finite() {
                norm += w;
                for k in 0..3 {
                    value[k] += w * f64::from(c[k]);
                }
            }
        }
        if norm <= 0.5 {
            return nan;
        }
        value.map(|v| (v / norm) as f32)
    }

    /// R + G + B of `sample` at every pixel of rows y0..y1 (width `w`), with the bilinear
    /// weights computed once per row and column.
    pub fn luminance_rows(&self, y0: usize, y1: usize, w: usize) -> Vec<f32> {
        let b = BLOCK as f64;
        let axis = |v: usize, n: usize| -> Option<(usize, usize, f64)> {
            let g = (v as f64 - (b - 1.0) / 2.0) / b;
            if !(g >= -1.0 && g <= n as f64) {
                return None;
            }
            let g = g.clamp(0.0, n as f64 - 1.0);
            let i0 = g.floor() as usize;
            Some((i0, (i0 + 1).min(n - 1), g - i0 as f64))
        };
        let xs: Vec<Option<(usize, usize, f64)>> = (0..w).map(|x| axis(x, self.columns)).collect();
        let mut out = Vec::with_capacity((y1 - y0) * w);
        for y in y0..y1 {
            let Some((j0, j1, fy)) = axis(y, self.rows) else {
                out.extend(std::iter::repeat_n(f32::NAN, w));
                continue;
            };
            for x in &xs {
                let Some((i0, i1, fx)) = *x else {
                    out.push(f32::NAN);
                    continue;
                };
                let (mut value, mut norm) = (0f64, 0f64);
                for (i, j, wt) in [
                    (i0, j0, (1.0 - fx) * (1.0 - fy)),
                    (i1, j0, fx * (1.0 - fy)),
                    (i0, j1, (1.0 - fx) * fy),
                    (i1, j1, fx * fy),
                ] {
                    let c = self.rgb[j * self.columns + i];
                    if c[0].is_finite() {
                        norm += wt;
                        value += wt * f64::from(c[0] + c[1] + c[2]);
                    }
                }
                out.push(if norm > 0.5 {
                    (value / norm) as f32
                } else {
                    f32::NAN
                });
            }
        }
        out
    }

    pub fn centres(&self) -> Vec<Point> {
        let b = BLOCK as f64;
        (0..self.columns * self.rows)
            .map(|c| {
                [
                    (c % self.columns) as f64 * b + (b - 1.0) / 2.0,
                    (c / self.columns) as f64 * b + (b - 1.0) / 2.0,
                ]
            })
            .collect()
    }
}
