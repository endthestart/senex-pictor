//! Pinhole camera with a DNG-style radial term (kr0 = 1) about the image centre (ADR 0008).

use crate::stars::similarity::Point;
use serde::{Deserialize, Serialize};

pub type Vec3 = [f64; 3];

const UNDISTORT_ROUNDS: usize = 30;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub width: usize,
    pub height: usize,
    /// Focal length in pixels.
    pub f: f64,
    pub k1: f64,
    pub k2: f64,
}

impl Camera {
    pub fn centre(&self) -> Point {
        [
            (self.width as f64 - 1.0) / 2.0,
            (self.height as f64 - 1.0) / 2.0,
        ]
    }

    /// Distance from the centre to the farthest corner: radius 1 of the radial terms.
    pub fn scale(&self) -> f64 {
        let c = self.centre();
        c[0].hypot(c[1])
    }

    pub fn distort(&self, u: Point) -> Point {
        let (c, m) = (self.centre(), self.scale());
        let (dx, dy) = (u[0] - c[0], u[1] - c[1]);
        let r2 = (dx * dx + dy * dy) / (m * m);
        let g = 1.0 + self.k1 * r2 + self.k2 * r2 * r2;
        [c[0] + dx * g, c[1] + dy * g]
    }

    pub fn undistort(&self, p: Point) -> Point {
        let mut u = p;
        for _ in 0..UNDISTORT_ROUNDS {
            let d = self.distort(u);
            u = [u[0] + p[0] - d[0], u[1] + p[1] - d[1]];
        }
        u
    }

    /// Unit ray through pixel `p`.
    pub fn ray(&self, p: Point) -> Vec3 {
        let (u, c) = (self.undistort(p), self.centre());
        let v = [(u[0] - c[0]) / self.f, (u[1] - c[1]) / self.f, 1.0];
        let n = (v[0] * v[0] + v[1] * v[1] + 1.0).sqrt();
        [v[0] / n, v[1] / n, v[2] / n]
    }

    /// Pixel seen along `v`, or None behind the camera.
    pub fn pixel(&self, v: Vec3) -> Option<Point> {
        if v[2] <= 0.0 {
            return None;
        }
        let c = self.centre();
        Some(self.distort([c[0] + self.f * v[0] / v[2], c[1] + self.f * v[1] / v[2]]))
    }
}
