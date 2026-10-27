//! Foreground displacement under camera drift (ADR 0010): per frame, a smooth field
//! (quadratic in x, y) fitted robustly to the motion of silhouette corners tracked from the
//! reference frame. It widens each frame's sky-sample margin and moves the foreground back
//! into place in the long-exposure ground.

use super::image::{Gray, features, half_sums};
use super::quadratic::{fit, median, terms};
use super::rotation::{self, Turn};
use super::track::{Pyramid, corners, follow_checked};
use crate::budget::Budget;
use crate::sky::Camera;
use crate::stars::similarity::Point;
use crate::store::Store;
use crate::{Result, error};
use rayon::prelude::*;

const SCALE: f32 = 20.0;
const CORNERS: usize = 3000;
const SPACING: f64 = 12.0;
const ROUND_TRIP_PX: f64 = 0.5;
const MIN_TRACKED: usize = 20;
const EDGE_OUT: usize = 4;
const EDGE_IN: usize = 12;
/// One frame's features per pixel: the RGB read (f16 and f32), its sum, half-resolution
/// sums, the filtered features and their pyramid with gradients.
const FEATURE_BYTES_PER_PIXEL: u64 = 34;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Displacement {
    /// x and y displacement coefficients of [1, u, v, u², uv, v²], u = x/(W/2) − 1, v = y/(W/2) − 1.
    pub coefficients: [[f64; 6]; 2],
    pub residual_p90_px: f64,
    pub corners: usize,
    /// Median motion of the tracked corners (full-resolution px): the frame's drift, robust to
    /// swaying foliage.
    pub median_px: [f64; 2],
    /// The camera turn that moves the same corners, when a camera model was given.
    pub turn: Option<Turn>,
}

impl Displacement {
    pub fn at(&self, x: f64, y: f64, width: usize) -> [f64; 2] {
        let t = terms(x, y, width as f64 / 2.0);
        self.coefficients
            .map(|c| c.iter().zip(&t).map(|(a, b)| a * b).sum())
    }
}

fn frame_features(store: &Store, i: usize) -> Result<Gray> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    let (sums, hw, hh) = half_sums(&store.luminance(i)?, w, h);
    let means: Vec<f32> = sums.iter().map(|s| s / 4.0).collect();
    Ok(features(&means, hw, hh, SCALE))
}

/// Foreground displacements from the reference frame: those tracked and the frames whose
/// foreground could not be tracked (too few corners survived).
#[derive(Debug, Default)]
pub struct Tracked {
    pub fields: Vec<(usize, Displacement)>,
    pub lost: Vec<usize>,
}

/// Displacements from `reference` for `frames` (full-resolution px); `ground` marks
/// foreground pixels (alpha ≤ 0.5) whose silhouettes carry the corners.
pub fn foreground_displacement(
    store: &Store,
    ground: &[bool],
    reference: usize,
    frames: &[usize],
    camera: Option<&Camera>,
    budget: &Budget,
) -> Result<Tracked> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    let target = frame_features(store, reference)?;
    let (hw, hh) = (target.width, target.height);
    let half: Vec<bool> = (0..hw * hh)
        .map(|p| ground[(2 * (p / hw)) * w + 2 * (p % hw)])
        .collect();
    let grown = square(&half, hw, hh, EDGE_OUT, true);
    let shrunk = square(&half, hw, hh, EDGE_IN, false);
    let allowed: Vec<bool> = grown.iter().zip(&shrunk).map(|(g, s)| *g && !*s).collect();
    let points = corners(&target, &allowed, CORNERS, SPACING);
    let full: Vec<Point> = points
        .iter()
        .map(|p| [2.0 * p[0] + 0.5, 2.0 * p[1] + 0.5])
        .collect();
    let pyramid = Pyramid::new(&target);
    let mut order: Vec<usize> = frames.iter().copied().filter(|&i| i != reference).collect();
    order.sort_by_key(|&i| (i.abs_diff(reference), i));
    let workers = budget.workers(
        "foreground tracking",
        Budget::held(w * h),
        FEATURE_BYTES_PER_PIXEL * (w * h) as u64,
    )?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()?;
    let mut lost = Vec::new();
    let mut shifts: Vec<(usize, [f64; 2])> = vec![(reference, [0.0, 0.0])];
    let mut out = vec![(
        reference,
        Displacement {
            coefficients: [[0.0; 6]; 2],
            residual_p90_px: 0.0,
            corners: points.len(),
            median_px: [0.0; 2],
            turn: camera.map(|_| Turn {
                matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                residual_p90_px: 0.0,
            }),
        },
    )];
    for chunk in order.chunks(workers) {
        let pyramids: Vec<Pyramid> = pool.install(|| {
            chunk
                .par_iter()
                .map(|&i| Ok(Pyramid::new(&frame_features(store, i)?)))
                .collect::<Result<_>>()
        })?;
        for (&i, image) in chunk.iter().zip(&pyramids) {
            let (_, shift) = *shifts
                .iter()
                .min_by_key(|(j, _)| j.abs_diff(i))
                .ok_or_else(|| error("no tracked frame"))?;
            let start: Vec<Point> = points
                .iter()
                .map(|p| [p[0] + shift[0], p[1] + shift[1]])
                .collect();
            let found = follow_checked(&pyramid, image, &points, Some(&start), ROUND_TRIP_PX);
            let pairs: Vec<(Point, Point)> = found
                .iter()
                .zip(&points)
                .zip(&full)
                .filter_map(|((f, p), q)| f.map(|f| (*q, [f[0] - p[0], f[1] - p[1]])))
                .collect();
            if pairs.len() < MIN_TRACKED {
                lost.push(i);
                continue;
            }
            let moved: Vec<Point> = pairs
                .iter()
                .map(|(_, m)| [2.0 * m[0], 2.0 * m[1]])
                .collect();
            let at: Vec<Point> = pairs.iter().map(|(q, _)| *q).collect();
            let half = [
                median(pairs.iter().map(|(_, m)| m[0]).collect()),
                median(pairs.iter().map(|(_, m)| m[1]).collect()),
            ];
            shifts.push((i, half));
            let median_px = [2.0 * half[0], 2.0 * half[1]];
            let turn = match camera {
                None => None,
                Some(c) => {
                    let to: Vec<Point> = at
                        .iter()
                        .zip(&moved)
                        .map(|(a, m)| [a[0] + m[0], a[1] + m[1]])
                        .collect();
                    match rotation::fit(c, &at, &to) {
                        Some(t) => Some(t),
                        None => {
                            lost.push(i);
                            continue;
                        }
                    }
                }
            };
            match fit(&at, &moved, w) {
                Ok(field) => out.push((
                    i,
                    Displacement {
                        turn,
                        median_px,
                        ..field
                    },
                )),
                Err(_) => lost.push(i),
            }
        }
    }
    Ok(Tracked { fields: out, lost })
}

/// Square dilation (`grow`) or erosion of radius `r`; pixels beyond the border never shrink
/// or grow the mask (OpenCV's default border).
fn square(mask: &[bool], w: usize, h: usize, r: usize, grow: bool) -> Vec<bool> {
    let pass = |m: &[bool], along_x: bool| -> Vec<bool> {
        (0..w * h)
            .map(|p| {
                let (x, y) = (p % w, p / w);
                let (c, n) = if along_x { (x, w) } else { (y, h) };
                let range = c.saturating_sub(r)..=(c + r).min(n - 1);
                let hit = |k: usize| if along_x { m[y * w + k] } else { m[k * w + x] };
                if grow {
                    range.into_iter().any(hit)
                } else {
                    range.into_iter().all(hit)
                }
            })
            .collect()
    };
    pass(&pass(mask, true), false)
}
