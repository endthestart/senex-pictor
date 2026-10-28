//! Camera-fixed sky glow (atmosphere, light pollution) apart from sky-fixed light.
//!
//! Frame i at camera pixel x sees G_i(x) + S(sky position). Block medians of clear sky are
//! fitted by alternation: S is the median of the frames with their glow removed, moved to the
//! reference; each G_i is the smoothed remainder in camera coordinates. Stacking then uses
//! F_i(x_i(q)) − G_i(x_i(q)) + G_ref(q).

pub use super::cells::Grid;
use super::filters::nearest_known;
use super::grid::gaussian;
use crate::budget::Budget;
use crate::sky::SkyModel;
use crate::sky::model::{apply, transpose};
use crate::stars::similarity::Point;
use crate::store::Store;
use crate::{Result, error};
use rayon::prelude::*;

pub const BLOCK: usize = 32;
pub const CLEAR: f32 = 0.98;
const FILL: f64 = 0.9;
pub const ITERATIONS: usize = 4;
const SMOOTH_CELLS: f64 = 2.0;
/// Clear-sky pixel indices kept per cell, per pixel.
const MEMBER_BYTES_PER_PIXEL: u64 = 8;
/// One frame read for its block medians, per pixel: the RGB (f16 and f32) and the mask.
const FRAME_BYTES_PER_PIXEL: u64 = 20;

fn median(v: &mut [f32]) -> f32 {
    super::stats::median(v)
}

/// Per frame the median RGB (divided by `flat`) of clear-sky pixels in each BLOCK cell that
/// is at least FILL clear; with `exclude` (a per-frame bit mask kind, such as `hero-occ`)
/// each frame also leaves out its own masked pixels. Returns the grids and the cells usable
/// before any exclusion.
pub fn block_medians(
    store: &Store,
    alpha: &[f32],
    flat: &[f32],
    exclude: Option<&str>,
    budget: &Budget,
) -> Result<(Vec<Grid>, Vec<bool>)> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    let (gw, gh) = (w / BLOCK, h / BLOCK);
    let enough = FILL * (BLOCK * BLOCK) as f64;
    let members: Vec<Vec<usize>> = (0..gw * gh)
        .map(|c| {
            let (i, j) = (c % gw, c / gw);
            (j * BLOCK..(j + 1) * BLOCK)
                .flat_map(|y| (i * BLOCK..(i + 1) * BLOCK).map(move |x| y * w + x))
                .filter(|&p| alpha[p] >= CLEAR)
                .collect()
        })
        .collect();
    let usable: Vec<bool> = members.iter().map(|m| m.len() as f64 >= enough).collect();
    let pool = budget.pool(
        "glow block medians",
        Budget::held(w * h) + MEMBER_BYTES_PER_PIXEL * (w * h) as u64,
        FRAME_BYTES_PER_PIXEL * (w * h) as u64,
    )?;
    budget
        .progress
        .step("Measuring glow", store.manifest.frames.len());
    let grids = pool.install(|| {
        (0..store.manifest.frames.len())
            .into_par_iter()
            .map(|i| {
                budget.progress.advance()?;
                let rgb = store.rgb_rows(i, 0, h)?;
                let masked = match exclude {
                    Some(kind) => Some(store.bit_rows(kind, i, 0, h)?),
                    None => None,
                };
                let mut buf = Vec::with_capacity(BLOCK * BLOCK);
                let cells = members
                    .iter()
                    .zip(&usable)
                    .map(|(m, ok)| {
                        let kept: Vec<usize> = match &masked {
                            Some(bits) => m.iter().copied().filter(|&p| !bits[p]).collect(),
                            None => m.clone(),
                        };
                        if !ok || (kept.len() as f64) < enough {
                            return [f32::NAN; 3];
                        }
                        std::array::from_fn(|k| {
                            buf.clear();
                            buf.extend(kept.iter().map(|&p| rgb[3 * p + k] / flat[p]));
                            median(&mut buf)
                        })
                    })
                    .collect();
                Ok(Grid {
                    columns: gw,
                    rows: gh,
                    rgb: cells,
                })
            })
            .collect::<Result<Vec<_>>>()
    })?;
    Ok((grids, usable))
}

/// Normalised Gaussian smoothing of the known cells; cells far from data take the nearest
/// smoothed value.
fn smooth(g: &Grid) -> Result<Grid> {
    let (c, r) = (g.columns, g.rows);
    let valid: Vec<f64> = g
        .rgb
        .iter()
        .map(|v| f64::from(u8::from(v[0].is_finite())))
        .collect();
    let weight = gaussian(&valid, c, r, SMOOTH_CELLS, false);
    let channels: Vec<Vec<f64>> = (0..3)
        .map(|k| {
            let v: Vec<f64> = g
                .rgb
                .iter()
                .map(|x| {
                    if x[0].is_finite() {
                        f64::from(x[k])
                    } else {
                        0.0
                    }
                })
                .collect();
            gaussian(&v, c, r, SMOOTH_CELLS, false)
        })
        .collect();
    let known: Vec<bool> = weight.iter().map(|&w| w > 0.05).collect();
    let nearest =
        nearest_known(&known, c, r).ok_or_else(|| error("glow: a frame has no clear-sky cells"))?;
    let rgb = nearest
        .iter()
        .map(|&q| std::array::from_fn(|k| (channels[k][q] / weight[q].max(1e-9)) as f32))
        .collect();
    Ok(Grid {
        columns: c,
        rows: r,
        rgb,
    })
}

/// Per-frame glow, the sky light on the reference grid and the RMS change of the glow at
/// each iteration, from the `used` frames (others get no glow).
pub fn decompose(
    medians: &[Grid],
    model: &SkyModel,
    used: &[bool],
) -> Result<(Vec<Grid>, Grid, Vec<f64>)> {
    if used.len() != medians.len() {
        return Err(error("glow: frame selection differs from the frames"));
    }
    let first = medians.first().ok_or_else(|| error("glow: no frames"))?;
    let centres = first.centres();
    let map = |r: &crate::sky::model::Matrix| -> Vec<Point> {
        centres
            .iter()
            .map(|p| {
                model
                    .camera
                    .pixel(apply(r, model.camera.ray(*p)))
                    .unwrap_or([f64::NAN; 2])
            })
            .collect()
    };
    let to_frame: Vec<Vec<Point>> = (0..medians.len())
        .into_par_iter()
        .map(|i| map(&model.rotation(i)))
        .collect();
    let to_reference: Vec<Vec<Point>> = (0..medians.len())
        .into_par_iter()
        .map(|i| map(&transpose(&model.rotation(i))))
        .collect();
    let minus = |a: &Grid, b: &Grid| Grid {
        columns: a.columns,
        rows: a.rows,
        rgb: a
            .rgb
            .iter()
            .zip(&b.rgb)
            .map(|(x, y)| std::array::from_fn(|k| x[k] - y[k]))
            .collect(),
    };
    let warp = |g: &Grid, points: &[Point]| Grid {
        columns: g.columns,
        rows: g.rows,
        rgb: points.iter().map(|p| g.sample(*p)).collect(),
    };
    let mut glow: Vec<Grid> = medians
        .iter()
        .map(|m| Grid {
            columns: m.columns,
            rows: m.rows,
            rgb: vec![[0.0; 3]; m.rgb.len()],
        })
        .collect();
    let mut changes = Vec::with_capacity(ITERATIONS);
    let mut sky = first.clone();
    for _ in 0..ITERATIONS {
        let warped: Vec<Grid> = (0..medians.len())
            .into_par_iter()
            .filter(|&i| used[i])
            .map(|i| warp(&minus(&medians[i], &glow[i]), &to_frame[i]))
            .collect();
        sky.rgb = (0..first.rgb.len())
            .map(|c| {
                std::array::from_fn(|k| {
                    let mut v: Vec<f32> = warped
                        .iter()
                        .map(|g| g.rgb[c][k])
                        .filter(|v| v.is_finite())
                        .collect();
                    if v.is_empty() {
                        f32::NAN
                    } else {
                        median(&mut v)
                    }
                })
            })
            .collect();
        let next: Vec<Grid> = (0..medians.len())
            .into_par_iter()
            .map(|i| {
                if !used[i] {
                    return Ok(Grid {
                        columns: first.columns,
                        rows: first.rows,
                        rgb: vec![[f32::NAN; 3]; first.rgb.len()],
                    });
                }
                smooth(&minus(&medians[i], &warp(&sky, &to_reference[i])))
            })
            .collect::<Result<_>>()?;
        let (mut sum, mut count) = (0f64, 0usize);
        for (a, b) in next.iter().zip(&glow) {
            for (x, y) in a.rgb.iter().zip(&b.rgb) {
                for k in 0..3 {
                    let d = f64::from(x[k] - y[k]);
                    if d.is_finite() {
                        sum += d * d;
                        count += 1;
                    }
                }
            }
        }
        changes.push((sum / count.max(1) as f64).sqrt());
        glow = next;
    }
    Ok((glow, sky, changes))
}
