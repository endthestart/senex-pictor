//! Free per-frame rotations (ADR 0010): a drifting camera adds its own turn to the sky's, so
//! each frame keeps the full rotation its stars show relative to the reference.

use super::camera::{Camera, Vec3};
use super::model::{IDENTITY, Matrix, SkyModel, multiply, rotation_vector, transpose, vector_of};
use super::residuals::{REJECT_ROUNDS, distances, project, rejection_limit, within};
use super::solve::{Jacobian, Problem, minimise, step_for};
use crate::stars::similarity::Point;
use crate::stars::{Star, Tracks};
use crate::{Result, error};
use rayon::prelude::*;

/// Fewest shared stars a frame needs for its own rotation.
pub const MIN_STARS: usize = 10;

struct Free<'a> {
    camera: Camera,
    rays: &'a [Vec3],
    targets: &'a [Point],
}

impl Problem for Free<'_> {
    fn residuals(&self, x: &[f64]) -> Result<Vec<f64>> {
        let mut out = Vec::with_capacity(2 * self.targets.len());
        project(
            &self.camera,
            &rotation_vector([x[0], x[1], x[2]]),
            self.rays,
            self.targets,
            &mut out,
        );
        Ok(out)
    }

    fn jacobian(&self, x: &[f64]) -> Result<Jacobian> {
        let rows = 2 * self.targets.len();
        let mut dense = vec![0.0; rows * 3];
        for k in 0..3 {
            let h = step_for(x[k]).min(1e-5);
            let (mut up, mut down) = (x.to_vec(), x.to_vec());
            up[k] += h;
            down[k] -= h;
            let (ru, rd) = (self.residuals(&up)?, self.residuals(&down)?);
            for i in 0..rows {
                dense[i * 3 + k] = (ru[i] - rd[i]) / (2.0 * h);
            }
        }
        Ok(Jacobian {
            shared: 3,
            dense,
            block: vec![None; rows],
        })
    }
}

/// Rotation vector taking the reference rays of `pairs` onto their targets (outliers
/// rejected as in the fixed-axis fit).
pub fn free_rotation(camera: &Camera, pairs: &[(Point, Point)], start: Vec3) -> Result<Vec3> {
    let solve = |kept: &[(Point, Point)], start: Vec3| -> Result<Vec3> {
        if kept.len() < MIN_STARS {
            return Err(error(format!(
                "only {} stars are shared with the reference",
                kept.len()
            )));
        }
        let rays: Vec<Vec3> = kept.iter().map(|(a, _)| camera.ray(*a)).collect();
        let targets: Vec<Point> = kept.iter().map(|(_, b)| *b).collect();
        let problem = Free {
            camera: *camera,
            rays: &rays,
            targets: &targets,
        };
        let x = minimise(&problem, start.to_vec())?;
        Ok([x[0], x[1], x[2]])
    };
    let mut kept = pairs.to_vec();
    let mut v = solve(&kept, start)?;
    for _ in 0..REJECT_ROUNDS {
        let d = distances(camera, &rotation_vector(v), pairs);
        let next = within(pairs, &d, rejection_limit(&d));
        if next == kept {
            break;
        }
        kept = next;
        v = solve(&kept, v)?;
    }
    Ok(v)
}

/// `model` re-referenced to `reference` with every frame's rotation fitted to its stars,
/// started from the fixed-axis rotation.
pub fn free_rotations(
    model: &SkyModel,
    stars: &[Vec<Star>],
    tracks: &Tracks,
    reference: usize,
) -> Result<SkyModel> {
    let back = transpose(&model.rotation(reference));
    let rotations: Vec<Matrix> = (0..model.angles.len())
        .into_par_iter()
        .map(|j| {
            if j == reference {
                return Ok(IDENTITY);
            }
            let start = vector_of(&multiply(&model.rotation(j), &back));
            let pairs = tracks.correspondences(stars, reference, j);
            free_rotation(&model.camera, &pairs, start)
                .map(rotation_vector)
                .map_err(|e| error(format!("frame {j}: {e}")))
        })
        .collect::<Result<_>>()?;
    Ok(SkyModel {
        reference,
        rotations: Some(rotations),
        angles: model
            .angles
            .iter()
            .map(|a| a - model.angles[reference])
            .collect(),
        ..model.clone()
    })
}
