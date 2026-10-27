//! Tracking stars from frame to frame. Each consecutive pair's motion is found afresh by
//! triangle matching of the brightest stars (gaps and uneven intervals change it; the
//! previous pair's motion is the fallback), detections are paired with mutual nearest
//! neighbours of their predicted positions, and a RANSAC similarity keeps the consistent
//! pairs, which continue their tracks.

use super::detect::Star;
use super::similarity::{Grid, Point, Random, Similarity, ransac};
use super::triangles;
use crate::{Result, error};
use serde::{Deserialize, Serialize};

/// Brightest stars per frame handed to triangle matching.
pub const BRIGHTEST: usize = 300;
pub const PAIR_RADIUS_PX: f64 = 4.0;
pub const INLIER_PX: f64 = 1.5;
const RANSAC_TRIALS: usize = 200;
const SEED: u64 = 0x5eed_57a2;

/// `of[frame][detection]` is that detection's track.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tracks {
    pub of: Vec<Vec<u32>>,
    pub count: usize,
}

fn positions(stars: &[Star]) -> Vec<Point> {
    stars.iter().map(|s| [s.x, s.y]).collect()
}

fn brightest(stars: &[Star]) -> Vec<Point> {
    let mut order: Vec<usize> = (0..stars.len()).collect();
    order.sort_by(|&a, &b| stars[b].flux.total_cmp(&stars[a].flux));
    order.truncate(BRIGHTEST);
    order.iter().map(|&i| [stars[i].x, stars[i].y]).collect()
}

pub fn track(frames: &[Vec<Star>]) -> Result<Tracks> {
    if frames.len() < 2 {
        return Err(error("tracking needs at least two frames"));
    }
    let mut random = Random::new(SEED);
    let mut step = triangles::find(&brightest(&frames[0]), &brightest(&frames[1]), &mut random)
        .ok_or_else(|| error("the first two frames share no matching star pattern"))?;
    let mut of = vec![(0..frames[0].len() as u32).collect::<Vec<_>>()];
    let mut count = frames[0].len();
    for i in 1..frames.len() {
        if let Some(s) = triangles::find(
            &brightest(&frames[i - 1]),
            &brightest(&frames[i]),
            &mut random,
        ) {
            step = s;
        }
        let (prev, cur) = (positions(&frames[i - 1]), positions(&frames[i]));
        let pairs = mutual_pairs(&prev, &cur, &step);
        let (from, to): (Vec<Point>, Vec<Point>) =
            pairs.iter().map(|&(a, b)| (prev[a], cur[b])).unzip();
        let (_, inliers) =
            ransac(&from, &to, INLIER_PX, RANSAC_TRIALS, &mut random).ok_or_else(|| {
                error(format!(
                    "frame {i}: no consistent star motion from the previous frame"
                ))
            })?;
        let mut current: Vec<Option<u32>> = vec![None; cur.len()];
        for (&(a, b), ok) in pairs.iter().zip(&inliers) {
            if *ok {
                current[b] = Some(of[i - 1][a]);
            }
        }
        of.push(
            current
                .into_iter()
                .map(|t| {
                    t.unwrap_or_else(|| {
                        count += 1;
                        (count - 1) as u32
                    })
                })
                .collect(),
        );
    }
    Ok(Tracks { of, count })
}

/// (previous, current) detection pairs whose predicted and observed positions are each
/// other's nearest neighbour within PAIR_RADIUS_PX.
fn mutual_pairs(prev: &[Point], cur: &[Point], step: &Similarity) -> Vec<(usize, usize)> {
    let predicted: Vec<Point> = prev.iter().map(|p| step.apply(*p)).collect();
    let (to_cur, to_predicted) = (
        Grid::new(cur, PAIR_RADIUS_PX),
        Grid::new(&predicted, PAIR_RADIUS_PX),
    );
    predicted
        .iter()
        .enumerate()
        .filter_map(|(a, p)| {
            let (b, _) = to_cur.nearest(*p, PAIR_RADIUS_PX)?;
            let (back, _) = to_predicted.nearest(cur[b], PAIR_RADIUS_PX)?;
            (back == a).then_some((a, b))
        })
        .collect()
}

impl Tracks {
    /// Unsaturated (reference, frame) positions of stars tracked through both, in track order.
    pub fn correspondences(
        &self,
        stars: &[Vec<Star>],
        reference: usize,
        frame: usize,
    ) -> Vec<(Point, Point)> {
        let mut in_reference = vec![u32::MAX; self.count];
        for (d, &t) in self.of[reference].iter().enumerate() {
            in_reference[t as usize] = d as u32;
        }
        let mut both: Vec<(u32, usize, usize)> = self.of[frame]
            .iter()
            .enumerate()
            .filter_map(|(d, &t)| {
                let r = in_reference[t as usize];
                (r != u32::MAX).then_some((t, r as usize, d))
            })
            .filter(|&(_, r, d)| !stars[reference][r].saturated && !stars[frame][d].saturated)
            .collect();
        both.sort_unstable();
        both.iter()
            .map(|&(_, r, d)| {
                let (a, b) = (stars[reference][r], stars[frame][d]);
                ([a.x, a.y], [b.x, b.y])
            })
            .collect()
    }
}
