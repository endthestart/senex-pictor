//! The camera turn that best moves a frame's foreground corners onto where they were tracked.
//! A sagging tripod head turns the camera; distant foreground then moves as the sky would
//! under that turn, through the same lens model, so three parameters describe the whole
//! frame and nothing extrapolates away from the corners.

use crate::sky::Camera;
use crate::sky::model::{Matrix, apply, rotation_vector};
use crate::stars::similarity::Point;

const ITERATIONS: usize = 12;
const STEP: f64 = 1e-7;
const CONVERGED: f64 = 1e-10;
const ROUNDS: usize = 3;

/// A frame's foreground turn from the reference and its p90 corner residual (px).
#[derive(Clone, Debug, serde::Serialize)]
pub struct Turn {
    pub matrix: Matrix,
    pub residual_p90_px: f64,
}

impl Turn {
    /// Where the reference pixel `p` is in the turned frame.
    pub fn at(&self, camera: &Camera, p: Point) -> Option<Point> {
        camera.pixel(apply(&self.matrix, camera.ray(p)))
    }
}

fn residuals(camera: &Camera, w: [f64; 3], from: &[Point], to: &[Point]) -> Option<Vec<f64>> {
    let m = rotation_vector(w);
    let mut out = Vec::with_capacity(2 * from.len());
    for (f, t) in from.iter().zip(to) {
        let p = camera.pixel(apply(&m, camera.ray(*f)))?;
        out.extend([p[0] - t[0], p[1] - t[1]]);
    }
    Some(out)
}

/// Gauss–Newton over the rotation vector, from no turn.
fn solve(camera: &Camera, from: &[Point], to: &[Point]) -> Option<[f64; 3]> {
    let mut w = [0f64; 3];
    for _ in 0..ITERATIONS {
        let r = residuals(camera, w, from, to)?;
        let columns: Vec<Vec<f64>> = (0..3)
            .map(|k| {
                let mut s = w;
                s[k] += STEP;
                residuals(camera, s, from, to)
                    .map(|q| q.iter().zip(&r).map(|(a, b)| (a - b) / STEP).collect())
            })
            .collect::<Option<_>>()?;
        let mut a = [[0f64; 4]; 3];
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] = columns[i].iter().zip(&columns[j]).map(|(x, y)| x * y).sum();
            }
            a[i][3] = -columns[i].iter().zip(&r).map(|(x, y)| x * y).sum::<f64>();
        }
        let delta = solve3(a)?;
        for k in 0..3 {
            w[k] += delta[k];
        }
        if delta.iter().map(|d| d.abs()).fold(0.0, f64::max) < CONVERGED {
            break;
        }
    }
    Some(w)
}

/// The solution of a 3×3 system given as augmented rows.
pub(crate) fn solve3(mut a: [[f64; 4]; 3]) -> Option<[f64; 3]> {
    for c in 0..3 {
        let pivot = (c..3).max_by(|&x, &y| a[x][c].abs().total_cmp(&a[y][c].abs()))?;
        a.swap(c, pivot);
        if a[c][c].abs() < 1e-18 {
            return None;
        }
        for r in 0..3 {
            if r != c {
                let (f, row) = (a[r][c] / a[c][c], a[c]);
                for (x, p) in a[r].iter_mut().zip(row).skip(c) {
                    *x -= f * p;
                }
            }
        }
    }
    Some(std::array::from_fn(|i| a[i][3] / a[i][i]))
}

fn percentile(mut v: Vec<f64>, q: f64) -> f64 {
    v.sort_by(f64::total_cmp);
    crate::sky::report::percentile(&v, q).unwrap_or(0.0)
}

/// The turn moving `from` (reference px) to `to`, refitted ROUNDS times without corners
/// farther than 3 robust σ (at least 1 px) from it; None for a degenerate layout.
pub fn fit(camera: &Camera, from: &[Point], to: &[Point]) -> Option<Turn> {
    let mut keep = vec![true; from.len()];
    let mut distances = Vec::new();
    let mut w = [0f64; 3];
    for _ in 0..ROUNDS {
        let (f, t): (Vec<Point>, Vec<Point>) = from
            .iter()
            .zip(to)
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|((f, t), _)| (*f, *t))
            .unzip();
        if f.len() < 3 {
            return None;
        }
        w = solve(camera, &f, &t)?;
        let r = residuals(camera, w, from, to)?;
        distances = r.chunks(2).map(|d| d[0].hypot(d[1])).collect();
        let kept: Vec<f64> = distances
            .iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(d, _)| *d)
            .collect();
        let limit = (3.0 * 1.4826 * percentile(kept, 50.0)).max(1.0);
        keep = distances.iter().map(|d| *d < limit).collect();
    }
    let kept: Vec<f64> = distances
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(d, _)| *d)
        .collect();
    Some(Turn {
        matrix: rotation_vector(w),
        residual_p90_px: percentile(kept, 90.0),
    })
}
