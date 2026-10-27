//! Robust least squares with scipy's soft_l1 loss (f_scale 1): minimise Σ ρ(rᵢ²) with
//! ρ(z) = 2(√(1 + z) − 1) by Levenberg–Marquardt on the reweighted Gauss–Newton system.
//! Jacobians are block-sparse: shared columns plus at most one further column per residual.

use crate::{Result, error};

const MAX_ITERATIONS: usize = 200;
const STEP_TOLERANCE: f64 = 1e-12;
const COST_TOLERANCE: f64 = 1e-15;
const MAX_DAMPING: f64 = 1e12;

pub struct Jacobian {
    /// Columns any residual may depend on.
    pub shared: usize,
    /// Row-major derivatives on the shared columns (rows × shared).
    pub dense: Vec<f64>,
    /// Per row at most one further column (index ≥ shared) and its derivative.
    pub block: Vec<Option<(usize, f64)>>,
}

pub trait Problem {
    fn residuals(&self, x: &[f64]) -> Result<Vec<f64>>;
    fn jacobian(&self, x: &[f64]) -> Result<Jacobian>;
}

pub fn cost(r: &[f64]) -> f64 {
    r.iter().map(|v| 2.0 * ((1.0 + v * v).sqrt() - 1.0)).sum()
}

/// Central-difference step for a parameter of size |x|.
pub fn step_for(x: f64) -> f64 {
    f64::EPSILON.cbrt() * x.abs().max(1.0)
}

pub fn minimise(problem: &impl Problem, x0: Vec<f64>) -> Result<Vec<f64>> {
    let n = x0.len();
    let mut x = x0;
    let mut r = problem.residuals(&x)?;
    let mut c = cost(&r);
    if !c.is_finite() {
        return Err(error(
            "least squares: the starting point gives no finite residuals",
        ));
    }
    let mut damping = 1e-3;
    for _ in 0..MAX_ITERATIONS {
        let (h, g) = normal(&problem.jacobian(&x)?, &r, n)?;
        loop {
            let mut a = h.clone();
            for k in 0..n {
                a[k * n + k] += damping * h[k * n + k].max(1e-300);
            }
            let Some(step) = cholesky_solve(&mut a, &g, n) else {
                damping *= 10.0;
                if damping > MAX_DAMPING {
                    return Err(error("least squares: singular normal equations"));
                }
                continue;
            };
            let candidate: Vec<f64> = x.iter().zip(&step).map(|(v, s)| v - s).collect();
            let rc = problem.residuals(&candidate)?;
            let cc = cost(&rc);
            if cc.is_finite() && cc <= c {
                let small = step
                    .iter()
                    .zip(&candidate)
                    .all(|(s, v)| s.abs() <= STEP_TOLERANCE * (STEP_TOLERANCE + v.abs()));
                let flat = c - cc <= COST_TOLERANCE * c;
                (x, r, c) = (candidate, rc, cc);
                damping = (damping / 10.0).max(1e-12);
                if small || flat {
                    return Ok(x);
                }
                break;
            }
            damping *= 10.0;
            if damping > MAX_DAMPING {
                return Ok(x);
            }
        }
    }
    Err(error("least squares did not converge"))
}

/// JᵀWJ and JᵀWr with the soft_l1 weights wᵢ = 1/√(1 + rᵢ²).
fn normal(j: &Jacobian, r: &[f64], n: usize) -> Result<(Vec<f64>, Vec<f64>)> {
    let s = j.shared;
    if j.dense.len() != r.len() * s || j.block.len() != r.len() {
        return Err(error(
            "least squares: Jacobian and residuals differ in size",
        ));
    }
    let mut h = vec![0.0; n * n];
    let mut g = vec![0.0; n];
    for (i, &ri) in r.iter().enumerate() {
        let w = 1.0 / (1.0 + ri * ri).sqrt();
        let d = &j.dense[i * s..(i + 1) * s];
        for a in 0..s {
            let wa = w * d[a];
            g[a] += wa * ri;
            for b in a..s {
                h[a * n + b] += wa * d[b];
            }
        }
        if let Some((col, v)) = j.block[i] {
            if col < s || col >= n {
                return Err(error("least squares: block column out of range"));
            }
            let wv = w * v;
            g[col] += wv * ri;
            h[col * n + col] += wv * v;
            for (a, da) in d.iter().enumerate() {
                h[a * n + col] += wv * da;
            }
        }
    }
    for a in 0..n {
        for b in 0..a {
            h[a * n + b] = h[b * n + a];
        }
    }
    Ok((h, g))
}

/// Solves A x = b for symmetric positive definite A (overwritten), or None.
fn cholesky_solve(a: &mut [f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    for j in 0..n {
        let mut d = a[j * n + j];
        for k in 0..j {
            d -= a[j * n + k] * a[j * n + k];
        }
        if d <= 0.0 || !d.is_finite() {
            return None;
        }
        let d = d.sqrt();
        a[j * n + j] = d;
        for i in j + 1..n {
            let mut v = a[i * n + j];
            for k in 0..j {
                v -= a[i * n + k] * a[j * n + k];
            }
            a[i * n + j] = v / d;
        }
    }
    let mut y = b.to_vec();
    for i in 0..n {
        for k in 0..i {
            y[i] -= a[i * n + k] * y[k];
        }
        y[i] /= a[i * n + i];
    }
    for i in (0..n).rev() {
        for k in i + 1..n {
            y[i] -= a[k * n + i] * y[k];
        }
        y[i] /= a[i * n + i];
    }
    Some(y)
}
