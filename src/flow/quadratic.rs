//! The quadratic displacement field of a frame's foreground: coefficients of
//! [1, u, v, u², uv, v²] for x and y, fitted to the tracked corners by least squares with
//! ROUNDS rounds of rejecting corners beyond 3 robust σ (at least 1 px).

use super::displacement::Displacement;
use crate::stars::similarity::Point;
use crate::{Result, error};

const ROUNDS: usize = 3;

pub(super) fn terms(x: f64, y: f64, scale: f64) -> [f64; 6] {
    let (u, v) = (x / scale - 1.0, y / scale - 1.0);
    [1.0, u, v, u * u, u * v, v * v]
}

/// Least squares for 6 coefficients by normal equations.
fn solve6(rows: &[[f64; 6]], targets: &[f64]) -> Option<[f64; 6]> {
    let mut a = [[0f64; 7]; 6];
    for (r, t) in rows.iter().zip(targets) {
        for i in 0..6 {
            for j in 0..6 {
                a[i][j] += r[i] * r[j];
            }
            a[i][6] += r[i] * t;
        }
    }
    for c in 0..6 {
        let pivot = (c..6).max_by(|&x, &y| a[x][c].abs().total_cmp(&a[y][c].abs()))?;
        a.swap(c, pivot);
        if a[c][c].abs() < 1e-12 {
            return None;
        }
        for r in 0..6 {
            if r != c {
                let (f, pivot) = (a[r][c] / a[c][c], a[c]);
                for (x, p) in a[r].iter_mut().zip(pivot).skip(c) {
                    *x -= f * p;
                }
            }
        }
    }
    Some(std::array::from_fn(|i| a[i][6] / a[i][i]))
}

pub(super) fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

pub(super) fn fit(full: &[Point], moved: &[Point], width: usize) -> Result<Displacement> {
    let design: Vec<[f64; 6]> = full
        .iter()
        .map(|p| terms(p[0], p[1], width as f64 / 2.0))
        .collect();
    let mut keep = vec![true; moved.len()];
    let mut coefficients = [[0f64; 6]; 2];
    let mut residual = Vec::new();
    for _ in 0..ROUNDS {
        let rows: Vec<[f64; 6]> = design
            .iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(d, _)| *d)
            .collect();
        for (k, c) in coefficients.iter_mut().enumerate() {
            let t: Vec<f64> = moved
                .iter()
                .zip(&keep)
                .filter(|(_, k)| **k)
                .map(|(m, _)| m[k])
                .collect();
            *c = solve6(&rows, &t)
                .ok_or_else(|| error("foreground displacement: degenerate corner layout"))?;
        }
        residual = design
            .iter()
            .zip(moved)
            .map(|(d, m)| {
                let f: [f64; 2] = coefficients.map(|c| c.iter().zip(d).map(|(a, b)| a * b).sum());
                (f[0] - m[0]).hypot(f[1] - m[1])
            })
            .collect();
        let typical = median(
            residual
                .iter()
                .zip(&keep)
                .filter(|(_, k)| **k)
                .map(|(r, _)| *r)
                .collect(),
        );
        let limit = (3.0 * 1.4826 * typical).max(1.0);
        keep = residual.iter().map(|r| *r < limit).collect();
    }
    let mut kept: Vec<f64> = residual
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(r, _)| *r)
        .collect();
    kept.sort_by(f64::total_cmp);
    Ok(Displacement {
        coefficients,
        residual_p90_px: crate::sky::report::percentile(&kept, 90.0).unwrap_or(0.0),
        corners: moved.len(),
        median_px: [0.0; 2],
        turn: None,
    })
}
