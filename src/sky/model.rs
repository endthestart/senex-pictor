//! Sky registration model (ADR 0008): rays rotated about one axis by a per-frame angle, or,
//! under drift, by a free rotation per frame fitted to stars (ADR 0010).

use super::camera::{Camera, Vec3};
use crate::stars::similarity::Point;
use serde::{Deserialize, Serialize};

pub type Matrix = [[f64; 3]; 3];

pub const SIDEREAL_RAD_PER_S: f64 = 2.0 * std::f64::consts::PI / 86_164.090_5;
pub const IDENTITY: Matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Rodrigues rotation by `angle` about the unit `axis`.
pub fn rotation(axis: Vec3, angle: f64) -> Matrix {
    let [x, y, z] = axis;
    let k = [[0.0, -z, y], [z, 0.0, -x], [-y, x, 0.0]];
    let (s, c) = (angle.sin(), 1.0 - angle.cos());
    let mut m = IDENTITY;
    for (i, row) in m.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            let kk: f64 = (0..3).map(|l| k[i][l] * k[l][j]).sum();
            *v += s * k[i][j] + c * kk;
        }
    }
    m
}

/// Rotation by |v| about v.
pub fn rotation_vector(v: Vec3) -> Matrix {
    let angle = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if angle < 1e-15 {
        return IDENTITY;
    }
    rotation([v[0] / angle, v[1] / angle, v[2] / angle], angle)
}

/// The rotation vector of `m` (angle below π).
pub fn vector_of(m: &Matrix) -> Vec3 {
    let w = [m[2][1] - m[1][2], m[0][2] - m[2][0], m[1][0] - m[0][1]];
    let s = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt() / 2.0;
    let c = (m[0][0] + m[1][1] + m[2][2] - 1.0) / 2.0;
    let angle = s.atan2(c);
    if s < 1e-15 {
        return [w[0] / 2.0, w[1] / 2.0, w[2] / 2.0];
    }
    let k = angle / (2.0 * s);
    [w[0] * k, w[1] * k, w[2] * k]
}

pub fn axis_from(theta: f64, phi: f64) -> Vec3 {
    [
        theta.sin() * phi.cos(),
        theta.sin() * phi.sin(),
        theta.cos(),
    ]
}

pub fn apply(m: &Matrix, v: Vec3) -> Vec3 {
    std::array::from_fn(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
}

pub fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|l| a[i][l] * b[l][j]).sum()))
}

pub fn transpose(m: &Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| m[j][i]))
}

/// Camera, rotation axis and per-frame angles relative to the reference frame; with
/// `rotations` (one per frame) the sky follows those instead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkyModel {
    pub camera: Camera,
    pub axis: Vec3,
    pub angles: Vec<f64>,
    pub reference: usize,
    pub rotations: Option<Vec<Matrix>>,
}

impl SkyModel {
    /// R with ray_frame = R · ray_reference.
    pub fn rotation(&self, frame: usize) -> Matrix {
        match &self.rotations {
            Some(r) => r[frame],
            None => rotation(self.axis, self.angles[frame]),
        }
    }

    /// Where a reference-frame pixel's sky point is seen in `frame`.
    pub fn predict(&self, frame: usize, p: Point) -> Option<Point> {
        self.camera
            .pixel(apply(&self.rotation(frame), self.camera.ray(p)))
    }
}
