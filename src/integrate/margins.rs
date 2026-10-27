//! Per-frame sky-sample margins for a composite whose matte comes from one moment (ADR
//! 0010): each frame's foreground sits where its displacement moves the matte, so a sky
//! sample nearer than a margin to the moved foreground is left out (`hero-occ` bit masks).
//! The move is the frame's camera turn when one was fitted: a quadratic field fitted to the
//! same corners extrapolates far from them (over 1000 px at Yellowstone's sequence ends).

use crate::budget::Budget;
use crate::flow::displacement::Displacement;
use crate::flow::rotation::Turn;
use crate::sky::Camera;
use crate::store::{Store, file};
use crate::{Result, error};
use rayon::prelude::*;

/// Sky samples nearer than this to the (moved) foreground are left out.
pub const CLEARANCE_PX: f32 = 5.0;
/// One frame's mask per pixel, as computed and packed.
const MASK_BYTES_PER_PIXEL: u64 = 2;
/// Spacing (px) at which a turn's shift is evaluated; it varies slowly over the frame.
const GRID: usize = 16;

/// Writes `{kind}/{i}.{kind}` for every frame: pixels whose `distance` from the foreground,
/// moved by the frame's camera turn (its quadratic displacement when no turn was fitted), is
/// under CLEARANCE_PX plus the fit's p90 residual and 1 px. Frames without a displacement are
/// masked whole. Returns the widest shift in px.
pub fn write(
    store: &Store,
    kind: &str,
    distance: &[f32],
    fields: &[(usize, Displacement)],
    camera: &Camera,
    budget: &Budget,
) -> Result<f64> {
    let (w, h, n) = (
        store.manifest.width,
        store.manifest.height,
        store.manifest.frames.len(),
    );
    if distance.len() != w * h {
        return Err(error("margins: the distance plane differs from the frames"));
    }
    let dir = store.dir.join(kind);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    let pool = budget.pool(
        "sky-sample margins",
        Budget::held(w * h),
        MASK_BYTES_PER_PIXEL * (w * h) as u64,
    )?;
    let widest: Vec<f64> = pool.install(|| {
        (0..n)
            .into_par_iter()
            .map(|i| {
                let (mask, widest) = match fields.iter().find(|(j, _)| *j == i) {
                    None => (vec![true; w * h], 0.0),
                    Some((_, d)) => match &d.turn {
                        Some(t) => turned(distance, t, camera, w, h),
                        None => moved(distance, d, w, h),
                    },
                };
                file::create(
                    &dir.join(format!("{i:05}.{kind}")),
                    &file::pack_bits(&mask, w),
                )?;
                Ok(widest)
            })
            .collect::<Result<_>>()
    })?;
    Ok(widest.into_iter().fold(0.0, f64::max))
}

fn moved(distance: &[f32], d: &Displacement, w: usize, h: usize) -> (Vec<bool>, f64) {
    let margin = CLEARANCE_PX + d.residual_p90_px as f32 + 1.0;
    let mut widest = 0f64;
    let mask = (0..w * h)
        .map(|p| {
            let (x, y) = ((p % w) as f64, (p / w) as f64);
            let shift = d.at(x, y, w);
            widest = widest.max(shift[0].abs()).max(shift[1].abs());
            sample(distance, w, h, x - shift[0], y - shift[1]) < margin
        })
        .collect();
    (mask, widest)
}

/// The mask for a frame turned by `t`, its shift evaluated every GRID px and interpolated.
fn turned(distance: &[f32], t: &Turn, camera: &Camera, w: usize, h: usize) -> (Vec<bool>, f64) {
    let margin = CLEARANCE_PX + t.residual_p90_px as f32 + 1.0;
    let (gw, gh) = (w.div_ceil(GRID) + 1, h.div_ceil(GRID) + 1);
    let shifts: Vec<[f64; 2]> = (0..gw * gh)
        .map(|g| {
            let p = [((g % gw) * GRID) as f64, ((g / gw) * GRID) as f64];
            t.at(camera, p)
                .map_or([f64::INFINITY; 2], |q| [q[0] - p[0], q[1] - p[1]])
        })
        .collect();
    let mut widest = 0f64;
    let mask = (0..w * h)
        .map(|p| {
            let (x, y) = (p % w, p / w);
            let (i, j) = (x / GRID, y / GRID);
            let (fx, fy) = (
                (x % GRID) as f64 / GRID as f64,
                (y % GRID) as f64 / GRID as f64,
            );
            let at = |a: usize, b: usize| shifts[b * gw + a];
            let shift: [f64; 2] = std::array::from_fn(|k| {
                (at(i, j)[k] * (1.0 - fx) + at(i + 1, j)[k] * fx) * (1.0 - fy)
                    + (at(i, j + 1)[k] * (1.0 - fx) + at(i + 1, j + 1)[k] * fx) * fy
            });
            widest = widest.max(shift[0].abs()).max(shift[1].abs());
            !shift.iter().all(|s| s.is_finite())
                || sample(distance, w, h, x as f64 - shift[0], y as f64 - shift[1]) < margin
        })
        .collect();
    (mask, widest)
}

/// Bilinear sample of a plane at (x, y), zero outside.
pub fn sample(plane: &[f32], w: usize, h: usize, x: f64, y: f64) -> f32 {
    if !(x >= 0.0 && y >= 0.0 && x <= (w - 1) as f64 && y <= (h - 1) as f64) {
        return 0.0;
    }
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
    let v = |i: usize, j: usize| plane[j * w + i];
    (v(x0, y0) * (1.0 - fx) + v(x1, y0) * fx) * (1.0 - fy)
        + (v(x0, y1) * (1.0 - fx) + v(x1, y1) * fx) * fy
}
