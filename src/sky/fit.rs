//! Fitting the sky model to tracked stars: a joint fit of focal length, radial terms (unless
//! fixed from a lens profile), axis and angles on a subset of frames, then each frame's angle
//! with the shared parameters fixed.

use super::camera::{Camera, Vec3};
use super::drift::free_rotation;
use super::joint::Joint;
use super::model::{SIDEREAL_RAD_PER_S, SkyModel, rotation};
use super::residuals::{Pairs, REJECT_ROUNDS, distances, project, rejection_limit, within};
use super::solve::{Jacobian, Problem, minimise, step_for};
use crate::stars::similarity::Point;
use crate::stars::{Star, Tracks};
use crate::{Result, error};
use rayon::prelude::*;

/// Starting camera; with `fix_distortion` k1 and k2 stay as given (a lens profile).
#[derive(Clone, Copy, Debug)]
pub struct Start {
    pub f: f64,
    pub k1: f64,
    pub k2: f64,
    pub fix_distortion: bool,
}

/// One frame's angle about a fixed axis.
struct Angle<'a> {
    camera: Camera,
    axis: Vec3,
    rays: &'a [Vec3],
    targets: &'a [Point],
}

impl Problem for Angle<'_> {
    fn residuals(&self, x: &[f64]) -> Result<Vec<f64>> {
        let mut out = Vec::with_capacity(2 * self.targets.len());
        project(
            &self.camera,
            &rotation(self.axis, x[0]),
            self.rays,
            self.targets,
            &mut out,
        );
        Ok(out)
    }

    fn jacobian(&self, x: &[f64]) -> Result<Jacobian> {
        let h = step_for(x[0]);
        let (ru, rd) = (self.residuals(&[x[0] + h])?, self.residuals(&[x[0] - h])?);
        Ok(Jacobian {
            shared: 1,
            dense: ru
                .iter()
                .zip(&rd)
                .map(|(u, d)| (u - d) / (2.0 * h))
                .collect(),
            block: vec![None; ru.len()],
        })
    }
}

/// The angle of a frame about `axis` best fitting `pairs` (outliers rejected), starting
/// from `start`.
pub fn angle(camera: &Camera, axis: Vec3, pairs: &[(Point, Point)], start: f64) -> Result<f64> {
    let solve = |kept: &[(Point, Point)], start: f64| -> Result<f64> {
        let rays: Vec<Vec3> = kept.iter().map(|(a, _)| camera.ray(*a)).collect();
        let targets: Vec<Point> = kept.iter().map(|(_, b)| *b).collect();
        let problem = Angle {
            camera: *camera,
            axis,
            rays: &rays,
            targets: &targets,
        };
        Ok(minimise(&problem, vec![start])?[0])
    };
    let mut kept = pairs.to_vec();
    let mut x = solve(&kept, start)?;
    for _ in 0..REJECT_ROUNDS {
        let d = distances(camera, &rotation(axis, x), pairs);
        let next = within(pairs, &d, rejection_limit(&d));
        if next == kept {
            break;
        }
        kept = next;
        x = solve(&kept, x)?;
    }
    Ok(x)
}

pub fn fit(
    stars: &[Vec<Star>],
    tracks: &Tracks,
    times: &[f64],
    reference: usize,
    subset: &[usize],
    size: (usize, usize),
    start: Start,
) -> Result<SkyModel> {
    let pairs: Vec<Pairs> = subset
        .iter()
        .map(|&i| tracks.correspondences(stars, reference, i))
        .collect();
    let far = (0..subset.len())
        .max_by(|&a, &b| {
            let d = |j: usize| (times[subset[j]] - times[reference]).abs();
            d(a).total_cmp(&d(b))
        })
        .ok_or_else(|| error("the sky fit needs frames besides the reference"))?;
    let camera = Camera {
        width: size.0,
        height: size.1,
        f: start.f,
        k1: start.k1,
        k2: start.k2,
    };
    let turn = free_rotation(&camera, &pairs[far], [0.0; 3])?;
    let length = (turn[0] * turn[0] + turn[1] * turn[1] + turn[2] * turn[2]).sqrt();
    if length == 0.0 {
        return Err(error(
            "the stars did not move between the reference and the farthest frame",
        ));
    }
    let sign = (times[subset[far]] - times[reference]).signum() / length;
    let axis = turn.map(|v| v * sign);
    let mut x0 = vec![start.f];
    if !start.fix_distortion {
        x0.extend([start.k1, start.k2]);
    }
    x0.extend([axis[2].clamp(-1.0, 1.0).acos(), axis[1].atan2(axis[0])]);
    x0.extend(
        subset
            .iter()
            .map(|&i| SIDEREAL_RAD_PER_S * (times[i] - times[reference])),
    );
    let mut kept = pairs.clone();
    let mut x = minimise(&Joint::new(&kept, size, start), x0)?;
    for _ in 0..REJECT_ROUNDS {
        let j = Joint::new(&kept, size, start);
        let (camera, axis) = j.unpack(&x);
        let s = j.shared();
        let d: Vec<Vec<f64>> = pairs
            .iter()
            .enumerate()
            .map(|(k, p)| distances(&camera, &rotation(axis, x[s + k]), p))
            .collect();
        let limit = rejection_limit(&d.concat());
        let next: Vec<Pairs> = pairs
            .iter()
            .zip(&d)
            .map(|(p, d)| within(p, d, limit))
            .collect();
        if next == kept {
            break;
        }
        kept = next;
        x = minimise(&Joint::new(&kept, size, start), x)?;
    }
    let (camera, axis) = Joint::new(&kept, size, start).unpack(&x);
    let angles: Vec<f64> = (0..times.len())
        .into_par_iter()
        .map(|i| {
            if i == reference {
                return Ok(0.0);
            }
            let pairs = tracks.correspondences(stars, reference, i);
            angle(
                &camera,
                axis,
                &pairs,
                SIDEREAL_RAD_PER_S * (times[i] - times[reference]),
            )
            .map_err(|e| error(format!("frame {i}: {e}")))
        })
        .collect::<Result<_>>()?;
    Ok(SkyModel {
        camera,
        axis,
        angles,
        reference,
        rotations: None,
        chroma: super::chroma::Chroma::NONE,
    })
}
