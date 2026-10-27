//! The foreground of a whole-sequence composite: the long-exposure ground over the frames not
//! lit by local light, each moved back by its foreground turn, and the plain mean of the
//! unlit frames nearest the reference (for unstable pixels and thinly covered sky).

use super::composite::WINDOW_HALF;
use super::ground::{self, Combine, Source};
use crate::budget::Budget;
use crate::flow::displacement::{Tracked, foreground_displacement};
use crate::flow::rotation::solve3;
use crate::sky::Camera;
use crate::store::Store;
use crate::{Result, error};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Report {
    pub frames: usize,
    /// Frames moved back into place (their foreground drifted more than `ground::STILL_PX`).
    pub moved: usize,
    pub widest_shift_px: f64,
    /// Per moved frame, its corners' median motion and its turn's widest shift over the
    /// ground (px).
    pub shifts_px: Vec<(String, f64, f64)>,
    pub lit_left_out: Vec<String>,
    /// Frames whose foreground could not be tracked; they are left out.
    pub lost: Vec<String>,
    pub rejected_share: f64,
}

pub struct Foreground {
    pub ground: Vec<[f32; 3]>,
    pub window: Vec<[f32; 3]>,
    pub report: Report,
}

/// Every frame's foreground tracked from `reference` over `ground` (matte ≤ 0.5), with its
/// camera turn.
pub fn track(
    store: &Store,
    ground: &[bool],
    camera: &Camera,
    reference: usize,
    budget: &Budget,
) -> Result<Tracked> {
    let all: Vec<usize> = (0..store.manifest.frames.len()).collect();
    foreground_displacement(store, ground, reference, &all, Some(camera), budget)
}

/// How far the foreground drifted (px): a quadratic in time fitted to every frame's median
/// corner motion, at its largest over the sequence. A tripod sagging moves the foreground
/// smoothly; branches and reflections swaying in the wind scatter the medians frame to frame
/// without a trend (BWCA: 1–6 px, no trend), and the fit leaves that scatter out.
pub fn drift(tracked: &Tracked, times: &[f64]) -> f64 {
    let points: Vec<(f64, [f64; 2])> = tracked
        .fields
        .iter()
        .map(|(i, d)| (times[*i], d.median_px))
        .collect();
    let (lo, hi) = points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.0), hi.max(p.0))
        });
    let span = (hi - lo).max(1.0);
    let terms = |t: f64| {
        let u = 2.0 * (t - lo) / span - 1.0;
        [1.0, u, u * u]
    };
    let fits: Option<Vec<[f64; 3]>> = (0..2)
        .map(|k| {
            let mut a = [[0f64; 4]; 3];
            for (t, m) in &points {
                let x = terms(*t);
                for i in 0..3 {
                    for j in 0..3 {
                        a[i][j] += x[i] * x[j];
                    }
                    a[i][3] += x[i] * m[k];
                }
            }
            solve3(a)
        })
        .collect();
    match fits {
        Some(c) => points
            .iter()
            .map(|(t, _)| {
                let x = terms(*t);
                let at = |c: &[f64; 3]| c.iter().zip(&x).map(|(a, b)| a * b).sum::<f64>();
                at(&c[0]).hypot(at(&c[1]))
            })
            .fold(0.0, f64::max),
        None => points
            .iter()
            .map(|(_, m)| m[0].hypot(m[1]))
            .fold(0.0, f64::max),
    }
}

/// The ground and window of `store` from the frames not `lit`: as shot without `tracked`,
/// otherwise the tracked frames, each moved to the reference by its turn when it drifted.
#[allow(clippy::too_many_arguments)]
pub fn foreground(
    store: &Store,
    flat: &[f32],
    ground_mask: &[bool],
    camera: &Camera,
    reference: usize,
    tracked: Option<&Tracked>,
    lit: &[usize],
    budget: &Budget,
) -> Result<Foreground> {
    let m = &store.manifest;
    let w = m.width;
    let usable: Vec<Source> = match tracked {
        None => (0..m.frames.len())
            .filter(|i| !lit.contains(i))
            .map(|frame| Source { frame, moved: None })
            .collect(),
        Some(t) => t
            .fields
            .iter()
            .filter(|(i, d)| !lit.contains(i) && d.turn.is_some())
            .map(|(i, d)| ground::moved(*i, d))
            .collect(),
    };
    if usable.is_empty() {
        return Err(error(
            "every frame is lit or untracked; no foreground to stack",
        ));
    }
    let mut nearest = usable.clone();
    nearest.sort_by_key(|s| (s.frame.abs_diff(reference), s.frame));
    nearest.truncate(2 * WINDOW_HALF + 1);
    let (ground, kept, valid) =
        ground::stack(store, flat, camera, &usable, Combine::Clipped, budget)?;
    let (window, _, _) = ground::stack(store, flat, camera, &nearest, Combine::Mean, budget)?;
    let median = |i: usize| {
        tracked
            .and_then(|t| t.fields.iter().find(|(j, _)| *j == i))
            .map_or(0.0, |(_, d)| d.median_px[0].hypot(d.median_px[1]))
    };
    let shifts: Vec<(String, f64, f64)> = usable
        .iter()
        .filter_map(|s| s.moved.map(|t| (s.frame, t)))
        .map(|(i, t)| {
            let wide = ground::widest(t, camera, ground_mask, w);
            (m.frames[i].name.clone(), median(i), wide)
        })
        .collect();
    let widest = shifts.iter().map(|s| s.2).fold(0.0, f64::max);
    let names = |frames: &mut dyn Iterator<Item = usize>| -> Vec<String> {
        frames.map(|i| m.frames[i].name.clone()).collect()
    };
    let report = Report {
        frames: usable.len(),
        moved: usable.iter().filter(|s| s.moved.is_some()).count(),
        widest_shift_px: widest,
        shifts_px: shifts,
        lit_left_out: names(&mut lit.iter().copied()),
        lost: names(&mut tracked.map_or(&[][..], |t| &t.lost).iter().copied()),
        rejected_share: 1.0
            - kept.iter().map(|&k| f64::from(k)).sum::<f64>()
                / valid.iter().map(|&v| f64::from(v)).sum::<f64>().max(1.0),
    };
    Ok(Foreground {
        ground,
        window,
        report,
    })
}
