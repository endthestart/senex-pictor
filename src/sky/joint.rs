//! The joint fit of camera, axis and the angles of a subset of frames, with numerical
//! derivatives that reuse the reference rays when only the axis or angles change.

use super::camera::{Camera, Vec3};
use super::fit::Start;
use super::model::{axis_from, rotation};
use super::residuals::{Pairs, project};
use super::solve::{Jacobian, Problem, step_for};
use crate::Result;
use crate::stars::similarity::Point;
use rayon::prelude::*;

pub struct Joint<'a> {
    pairs: &'a [Pairs],
    width: usize,
    height: usize,
    fixed: Option<(f64, f64)>,
}

impl<'a> Joint<'a> {
    pub fn new(pairs: &'a [Pairs], size: (usize, usize), start: Start) -> Joint<'a> {
        Joint {
            pairs,
            width: size.0,
            height: size.1,
            fixed: start.fix_distortion.then_some((start.k1, start.k2)),
        }
    }

    pub fn shared(&self) -> usize {
        if self.fixed.is_some() { 3 } else { 5 }
    }

    pub fn unpack(&self, x: &[f64]) -> (Camera, Vec3) {
        let (k1, k2) = self.fixed.unwrap_or((x[1], x[2]));
        let s = self.shared();
        let camera = Camera {
            width: self.width,
            height: self.height,
            f: x[0],
            k1,
            k2,
        };
        (camera, axis_from(x[s - 2], x[s - 1]))
    }

    fn rays(&self, camera: &Camera) -> Vec<Vec<Vec3>> {
        self.pairs
            .par_iter()
            .map(|p| p.iter().map(|(a, _)| camera.ray(*a)).collect())
            .collect()
    }

    fn evaluate(&self, x: &[f64], rays: &[Vec<Vec3>]) -> Vec<f64> {
        let (camera, axis) = self.unpack(x);
        let angles = &x[self.shared()..];
        let parts: Vec<Vec<f64>> = (0..self.pairs.len())
            .into_par_iter()
            .map(|j| {
                let targets: Vec<Point> = self.pairs[j].iter().map(|(_, b)| *b).collect();
                let mut out = Vec::with_capacity(2 * targets.len());
                project(
                    &camera,
                    &rotation(axis, angles[j]),
                    &rays[j],
                    &targets,
                    &mut out,
                );
                out
            })
            .collect();
        parts.concat()
    }
}

impl Problem for Joint<'_> {
    fn residuals(&self, x: &[f64]) -> Result<Vec<f64>> {
        Ok(self.evaluate(x, &self.rays(&self.unpack(x).0)))
    }

    fn jacobian(&self, x: &[f64]) -> Result<Jacobian> {
        let s = self.shared();
        let rays = self.rays(&self.unpack(x).0);
        let rows: usize = self.pairs.iter().map(|p| 2 * p.len()).sum();
        let mut dense = vec![0.0; rows * s];
        let camera_columns = s - 2;
        for k in 0..s {
            let h = step_for(x[k]);
            let (mut up, mut down) = (x.to_vec(), x.to_vec());
            up[k] += h;
            down[k] -= h;
            let (ru, rd) = if k < camera_columns {
                (self.residuals(&up)?, self.residuals(&down)?)
            } else {
                (self.evaluate(&up, &rays), self.evaluate(&down, &rays))
            };
            for i in 0..rows {
                dense[i * s + k] = (ru[i] - rd[i]) / (2.0 * h);
            }
        }
        let (mut up, mut down) = (x.to_vec(), x.to_vec());
        let steps: Vec<f64> = x[s..].iter().map(|v| step_for(*v)).collect();
        for (j, h) in steps.iter().enumerate() {
            up[s + j] += h;
            down[s + j] -= h;
        }
        let (ru, rd) = (self.evaluate(&up, &rays), self.evaluate(&down, &rays));
        let mut block = Vec::with_capacity(rows);
        for (j, p) in self.pairs.iter().enumerate() {
            for _ in 0..2 * p.len() {
                let i = block.len();
                block.push(Some((s + j, (ru[i] - rd[i]) / (2.0 * steps[j]))));
            }
        }
        Ok(Jacobian {
            shared: s,
            dense,
            block,
        })
    }
}
