//! Sky alpha per camera pixel (1 = clear sky) for silhouette foregrounds (ADR 0009).
//!
//! Confirmed star transits mark sky; brightness against a sky background fitted to transit
//! pixels (then refitted with featureless near-clear pixels) gives fractional coverage;
//! values within 3σ of 0 or 1 snap, as do near-clear pixels in clear neighbourhoods. Motion
//! outranks brightness (`evidence`): a part-sky pixel with no transit within the local transit
//! gap is ground (dark foliage, pale bark), as are dark segments there when segments are
//! given, and so is light of its own colour where no star crosses (leaves under a lamp).
//! Ground islands up to MAX_GROUND_ISLAND pixels (demosaic artefacts of hot pixels) become sky.

use super::evidence::{gaps, lit, near_transits};
use super::filters::{components, dilate, erode, median, open, uniform};
use super::grid::{fill_nearest, gaussian, resize};
use super::stats::Stats;
use crate::{Result, error};
use rayon::prelude::*;
use serde::Serialize;

pub const TRANSIT_MIN: f32 = 0.003;
pub const MAX_GROUND_ISLAND: usize = 16;
const BACKGROUND_CELL: usize = 16;
const UNSTABLE_FACTOR: f32 = 1.5;

pub struct Matte {
    pub alpha: Vec<f32>,
    pub seen: Vec<bool>,
    /// Mostly-ground pixels whose scatter shows moving foliage or changing light.
    pub unstable: Vec<bool>,
    /// Light of its own colour where no star crosses, made ground (`evidence::lit`).
    pub lit: Vec<bool>,
    pub report: MatteReport,
}

#[derive(Debug, Serialize)]
pub struct MatteReport {
    pub ground_level: f32,
    pub sky_clear_share: f64,
    pub ground_share: f64,
    pub edge_share: f64,
    pub transit_seen_share: f64,
    pub ground_islands_to_sky_pixels: usize,
    pub conflict_pixels: usize,
    pub unstable_pixels: usize,
    pub lit_pixels: usize,
    pub segments: usize,
}

/// Sky background from `sky` pixels: masked BACKGROUND_CELL medians, empty cells take the
/// nearest cell's value, Gaussian σ 2 cells, bilinear up to full resolution.
fn sky_background(level: &[f32], sky: &[bool], w: usize, h: usize) -> Result<Vec<f32>> {
    let c = BACKGROUND_CELL;
    let (gw, gh) = (w / c, h / c);
    let cells: Vec<Option<f64>> = (0..gw * gh)
        .into_par_iter()
        .map(|k| {
            let (i, j) = (k % gw, k / gw);
            let mut buf: Vec<f32> = (j * c..(j + 1) * c)
                .flat_map(|y| (i * c..(i + 1) * c).map(move |x| y * w + x))
                .filter(|&q| sky[q])
                .map(|q| level[q])
                .collect();
            (buf.len() >= c * c / 4).then(|| f64::from(super::stats::median(&mut buf)))
        })
        .collect();
    let filled = fill_nearest(&cells, gw, gh)
        .ok_or_else(|| error("matte: no sky cells to fit the sky background"))?;
    Ok(resize(&gaussian(&filled, gw, gh, 2.0, false), gw, gh, w, h))
}

/// Sky background, ground level, brightness alpha and its noise.
fn brightness(
    level: &[f32],
    sigma: &[f32],
    n: usize,
    sky: &[bool],
    w: usize,
    h: usize,
) -> Result<(f32, Vec<f32>, Vec<f32>)> {
    let background = sky_background(level, sky, w, h)?;
    let dark: Vec<bool> = (0..w * h)
        .into_par_iter()
        .map(|p| !sky[p] && level[p] < 0.25 * background[p])
        .collect();
    let core = erode(&dark, w, h, 3);
    let mut ground: Vec<f32> = (0..w * h)
        .into_par_iter()
        .filter(|&p| core[p])
        .map(|p| level[p])
        .collect();
    if ground.is_empty() {
        return Err(error("matte: no dark foreground to set the ground level"));
    }
    let ground_level = super::stats::median(&mut ground);
    let root = (n as f32).sqrt();
    let (alpha, noise) = (0..w * h)
        .into_par_iter()
        .map(|p| {
            let span = (background[p] - ground_level).max(1e-3);
            (
                ((level[p] - ground_level) / span).clamp(0.0, 1.0),
                1.2533 * sigma[p] / root / span,
            )
        })
        .unzip();
    Ok((ground_level, alpha, noise))
}

/// The matte from temporal statistics; `labels` (one segment id per pixel, 0 = none) are
/// optional segments.
pub fn matte(stats: &Stats, frames: usize, labels: Option<&[u32]>) -> Result<Matte> {
    let (w, h) = (stats.width, stats.height);
    let seen = open(
        &stats
            .transit
            .par_iter()
            .map(|&t| t >= TRANSIT_MIN)
            .collect::<Vec<_>>(),
        w,
        h,
    );
    let level = &stats.low;
    let (_, alpha0, _) = brightness(level, &stats.sigma, frames, &seen, w, h)?;
    let mean = uniform(level, w, h, 5);
    let squares = uniform(
        &level.par_iter().map(|v| v * v).collect::<Vec<_>>(),
        w,
        h,
        5,
    );
    let root = (frames as f32).sqrt();
    let refit: Vec<bool> = (0..w * h)
        .into_par_iter()
        .map(|p| {
            let texture = (squares[p] - mean[p] * mean[p]).max(0.0).sqrt();
            seen[p] || (alpha0[p] >= 0.85 && texture < 3.0 * 1.2533 * stats.sigma[p] / root)
        })
        .collect();
    let (ground_level, alpha, noise) = brightness(level, &stats.sigma, frames, &refit, w, h)?;
    let clear: Vec<bool> = (0..w * h)
        .into_par_iter()
        .map(|p| {
            (alpha[p] >= 1.0 - 3.0 * noise[p] && noise[p] < 0.05) || (seen[p] && alpha[p] >= 0.85)
        })
        .collect();
    let mut ground: Vec<bool> = (0..w * h)
        .into_par_iter()
        .map(|p| alpha[p] <= 3.0 * noise[p])
        .collect();
    let gap = gaps(&seen, w, h);
    let near = near_transits(gap.as_deref(), &alpha, w, h);
    let lit = lit(stats, &seen, gap.as_deref());
    ground.par_iter_mut().enumerate().for_each(|(p, g)| {
        *g |= (!near[p] && !clear[p] || lit[p]) && stats.resolvable[p];
    });
    let mut segments = 0;
    if let Some(labels) = labels {
        let count = labels.iter().copied().max().unwrap_or(0) as usize + 1;
        let (mut pixels, mut transits, mut sums) =
            (vec![0usize; count], vec![0usize; count], vec![0f64; count]);
        for p in 0..w * h {
            let l = labels[p] as usize;
            pixels[l] += 1;
            transits[l] += usize::from(seen[p]);
            sums[l] += f64::from(alpha[p]);
        }
        let dark: Vec<bool> = (0..count)
            .map(|l| {
                l > 0
                    && pixels[l] > 0
                    && (transits[l] as f64) < 0.25 * pixels[l] as f64
                    && sums[l] < 0.6 * pixels[l] as f64
            })
            .collect();
        segments = pixels.iter().skip(1).filter(|&&n| n > 0).count();
        for p in 0..w * h {
            ground[p] |= dark[labels[p] as usize] && !near[p];
        }
    }
    let mut snapped: Vec<f32> = (0..w * h)
        .into_par_iter()
        .map(|p| match (ground[p] && !seen[p], clear[p], ground[p]) {
            (true, _, _) => 0.0,
            (false, true, _) => 1.0,
            (false, false, true) => 0.0,
            _ => alpha[p],
        })
        .collect();
    let local = median(&snapped, w, h, 5);
    snapped.par_iter_mut().zip(&local).for_each(|(s, &l)| {
        if *s > 0.85 && l >= 0.98 {
            *s = 1.0;
        }
    });
    let solid: Vec<bool> = snapped.par_iter().map(|&a| a <= 0.02).collect();
    let (sizes, label) = components(&solid, w, h);
    let mut islands = 0;
    for p in 0..w * h {
        if solid[p] && sizes[label[p]] <= MAX_GROUND_ISLAND {
            snapped[p] = 1.0;
            islands += 1;
        }
    }
    let near_transit = dilate(&seen, w, h, 3);
    let unstable: Vec<bool> = (0..w * h)
        .into_par_iter()
        .map(|p| {
            snapped[p] <= 0.5
                && !near_transit[p]
                && stats.scatter[p] / stats.sigma[p].max(1e-3) > UNSTABLE_FACTOR
        })
        .collect();
    let count =
        |f: &(dyn Fn(usize) -> bool + Sync)| (0..w * h).into_par_iter().filter(|&p| f(p)).count();
    let share = |f: &(dyn Fn(usize) -> bool + Sync)| count(f) as f64 / (w * h) as f64;
    let report = MatteReport {
        ground_level,
        sky_clear_share: share(&|p| snapped[p] >= 0.98),
        ground_share: share(&|p| snapped[p] <= 0.02),
        edge_share: share(&|p| snapped[p] > 0.02 && snapped[p] < 0.98),
        transit_seen_share: share(&|p| seen[p]),
        ground_islands_to_sky_pixels: islands,
        conflict_pixels: count(&|p| seen[p] && snapped[p] < 0.2),
        unstable_pixels: count(&|p| unstable[p]),
        lit_pixels: count(&|p| lit[p]),
        segments,
    };
    Ok(Matte {
        alpha: snapped,
        seen,
        unstable,
        lit,
        report,
    })
}
