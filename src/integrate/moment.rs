//! Moment composites for dynamic skies (aurora, moving cloud). They are camera-fixed and
//! change within minutes, so a long stack smears them; a moment composite keeps them as they
//! were around one frame: the temporal median of a short window (stars move between those
//! frames and drop out of the median), plus the stars of the deep sky stack (the stack minus
//! its own smooth background). The foreground is the full-sequence ground stack.

use crate::budget::Budget;
use crate::separate::filters::median;
use crate::separate::grid::resize;
use crate::separate::stats;
use crate::store::{Store, planes};
use crate::{Result, error};
use rayon::prelude::*;
use serde::Serialize;

/// Owner's choice for aurora: nine frames (the centre and four either side).
pub const HALF: usize = 4;
/// Scale of a star in px: the background is a median over about this size.
pub const STAR_SCALE_PX: usize = 15;
/// Window stars brighter than this many robust σ above its background are removed so they
/// are not counted twice.
const STILL_STAR_SIGMA: f32 = 3.0;
/// One channel's working set per pixel: the window median, the star layers and their
/// backgrounds.
const CHANNEL_BYTES_PER_PIXEL: u64 = 40;

#[derive(Debug, Serialize)]
pub struct Report {
    pub centre: String,
    pub frames: usize,
    pub seconds_spanned: f64,
}

/// Point sources of one channel: the plane minus its median background (median over
/// STAR_SCALE_PX/2 + 1 px at half resolution), where it exceeds `k` robust σ (and 0).
fn star_layer(plane: &[f32], w: usize, h: usize, k: f32) -> Vec<f32> {
    let mut finite: Vec<f32> = plane.iter().copied().filter(|v| v.is_finite()).collect();
    let fill = if finite.is_empty() {
        0.0
    } else {
        stats::median(&mut finite)
    };
    let filled: Vec<f32> = plane
        .iter()
        .map(|&v| if v.is_finite() { v } else { fill })
        .collect();
    let (hw, hh) = (w / 2, h / 2);
    let half: Vec<f32> = (0..hw * hh)
        .map(|p| {
            let (x, y) = (2 * (p % hw), 2 * (p / hw));
            (filled[y * w + x]
                + filled[y * w + x + 1]
                + filled[(y + 1) * w + x]
                + filled[(y + 1) * w + x + 1])
                / 4.0
        })
        .collect();
    let smooth = median(&half, hw, hh, STAR_SCALE_PX / 2 + 1);
    let wide: Vec<f64> = smooth.iter().map(|&v| f64::from(v)).collect();
    let background = resize(&wide, hw, hh, w, h);
    let residual: Vec<f32> = filled.iter().zip(&background).map(|(a, b)| a - b).collect();
    let mut sorted = residual.clone();
    let centre = stats::median(&mut sorted);
    for v in sorted.iter_mut() {
        *v = (*v - centre).abs();
    }
    let floor = (k * 1.4826 * stats::median(&mut sorted)).max(0.0);
    residual
        .iter()
        .map(|&r| if r > floor { r } else { 0.0 })
        .collect()
}

/// Writes `moment-{centre name}.planes`: alpha·(starless window median + deep stars) +
/// (1 − alpha)·ground.
/// Whether the store holds the whole-sequence stack a moment is made from (a drifting run is
/// composed around one frame instead and has none).
pub fn possible(store: &Store) -> bool {
    ["sky.planes", "ground.planes"]
        .iter()
        .all(|f| store.dir.join(f).exists())
}

pub fn moment(store: &Store, centre: usize, half: usize, budget: &Budget) -> Result<Report> {
    let m = &store.manifest;
    let (w, h, n) = (m.width, m.height, m.frames.len());
    if centre >= n {
        return Err(error("moment: centre frame outside the store"));
    }
    if !possible(store) {
        return Err(error(
            "a moment needs the whole-sequence stack (sky.planes and ground.planes); this \
             store has none (run the stack stage, or the run was composed around one frame)",
        ));
    }
    let flat = super::recorded_flat(store)?;
    let alpha = planes::load(&store.dir.join("matte.planes"))?;
    let alpha = alpha.get("alpha")?;
    let sky = planes::load(&store.dir.join("sky.planes"))?;
    let ground = planes::load(&store.dir.join("ground.planes"))?;
    let (lo, hi) = (centre.saturating_sub(half), (centre + half + 1).min(n));
    let window: Vec<Vec<f32>> = (lo..hi)
        .map(|i| store.rgb_rows(i, 0, h))
        .collect::<Result<_>>()?;
    let pixels = (w * h) as u64;
    let held = Budget::held(w * h) + 12 * window.len() as u64 * pixels;
    let pool = budget.pool("the moment", held, CHANNEL_BYTES_PER_PIXEL * pixels)?;
    budget.progress.step("Building the moment", 3);
    let channels: Vec<Vec<f32>> = pool.install(|| {
        (0..3usize)
            .into_par_iter()
            .map(|k| -> Result<Vec<f32>> {
                budget.progress.advance()?;
                let mut buf = vec![0f32; window.len()];
                let still: Vec<f32> = (0..w * h)
                    .map(|p| {
                        for (b, f) in buf.iter_mut().zip(&window) {
                            *b = f[3 * p + k];
                        }
                        stats::median(&mut buf) / flat[p]
                    })
                    .collect();
                let lingering = star_layer(&still, w, h, STILL_STAR_SIGMA);
                let name = ["r", "g", "b"][k];
                let deep = star_layer(sky.get(name)?, w, h, 0.0);
                let ground = ground.get(name)?;
                Ok((0..w * h)
                    .map(|p| {
                        alpha[p] * (still[p] - lingering[p] + deep[p])
                            + (1.0 - alpha[p]) * ground[p]
                    })
                    .collect())
            })
            .collect::<Result<_>>()
    })?;
    let name = m.frames[centre]
        .name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_string();
    planes::save(
        &store.dir.join(format!("moment-{name}.planes")),
        w,
        h,
        &[
            ("r", &channels[0]),
            ("g", &channels[1]),
            ("b", &channels[2]),
        ],
    )?;
    Ok(Report {
        centre: m.frames[centre].name.clone(),
        frames: hi - lo,
        seconds_spanned: m.frames[hi - 1].time_s - m.frames[lo].time_s,
    })
}
