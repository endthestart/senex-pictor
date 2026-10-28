//! Sequence check, run before any processing (ADR 0010): split a folder into runs at gaps,
//! setting changes and camera moves, flag overexposed and lit frames, find twilight ramps,
//! report drift and recommend the frames to process. A camera move needs two witnesses: a
//! foreground step that stands out from the run's step scatter (or cannot be tracked) and a
//! break in the otherwise smooth star motion (wind moves foliage, never the stars); lit and
//! overexposed frames are not move witnesses.

use super::capture::Capture;
use super::motion::Steps;
use crate::{Result, error};
use serde::{Deserialize, Serialize};

/// A gap longer than this many typical intervals (and at least MIN_GAP_S) starts a new run.
pub const GAP_FACTOR: f64 = 3.0;
pub const MIN_GAP_S: f64 = 60.0;
/// A frame with more of its crop saturated than this is overexposed.
pub const OVEREXPOSED_SHARE: f64 = 0.02;
/// A frame whose foreground is this much brighter or redder than its darker neighbours is lit.
pub const LIT_FACTOR: f64 = 1.3;
pub const LIT_WINDOW: usize = 7;
/// A run whose smoothed sky level climbs or falls steadily by more than this factor is a
/// twilight ramp; its darkest stretch within this factor of the minimum is recommended.
pub const RAMP_FACTOR: f64 = 2.0;
pub const RAMP_STEADY: f64 = 0.9;
const RAMP_HALF_WINDOW: usize = 2;
/// A foreground step farther than MOVE_SIGMA scatters (and MOVE_FLOOR_PX) from the run's
/// median step is a move candidate.
pub const MOVE_FLOOR_PX: f64 = 1.5;
pub const MOVE_SIGMA: f64 = 4.0;
/// Star motion breaks when a step departs from the median of up to SKY_WINDOW neighbouring
/// steps either side by more than these.
pub const SKY_BREAK_PX: f64 = 1.5;
pub const SKY_BREAK_DEG: f64 = 0.02;
const SKY_WINDOW: usize = 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Twilight {
    /// Sky level at the brighter end of the run over the darker end (smoothed).
    pub brightening: f64,
    pub dark_first: String,
    pub dark_last: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub first: String,
    pub last: String,
    pub frames: usize,
    pub starts_after: String,
    pub overexposed: Vec<String>,
    pub lit: Vec<String>,
    pub twilight: Option<Twilight>,
    /// The frames recommended from this run (first, last, count).
    pub use_first: String,
    pub use_last: String,
    pub use_frames: usize,
    /// Robust scatter of the foreground steps and their sum over the run (full-res px).
    pub step_scatter_px: f64,
    pub drift_px: [f64; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub interval_s: f64,
    pub runs: Vec<Run>,
    pub recommended: usize,
}

pub fn check(captures: &[Capture], steps: &Steps) -> Result<Report> {
    if captures.is_empty() {
        return Err(error("no frames to check"));
    }
    if steps.foreground.len() != captures.len() || steps.sky.len() != captures.len() {
        return Err(error("motion steps and frames differ in count"));
    }
    let gaps: Vec<f64> = captures
        .windows(2)
        .map(|w| w[1].time_s - w[0].time_s)
        .collect();
    let mut sorted = gaps.clone();
    sorted.sort_by(f64::total_cmp);
    let interval = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0);
    let mut starts = vec![(0, "start")];
    for (i, gap) in gaps.iter().enumerate() {
        if captures[i + 1].settings != captures[i].settings {
            starts.push((i + 1, "settings change"));
        } else if *gap > (GAP_FACTOR * interval).max(MIN_GAP_S) || *gap < 0.0 {
            starts.push((i + 1, "time gap"));
        }
    }
    let ends: Vec<usize> = starts
        .iter()
        .skip(1)
        .map(|s| s.0)
        .chain([captures.len()])
        .collect();
    let mut pieces = Vec::new();
    for (&(a, why), b) in starts.iter().zip(ends) {
        let cuts = moves(captures, steps, a, b);
        let edges: Vec<usize> = [a]
            .into_iter()
            .chain(cuts.iter().copied())
            .chain([b])
            .collect();
        for (k, w) in edges.windows(2).enumerate() {
            pieces.push((w[0], w[1], if k == 0 { why } else { "camera move" }));
        }
    }
    let runs: Vec<Run> = pieces
        .iter()
        .map(|&(a, b, why)| run(captures, steps, a, b, why))
        .collect();
    let recommended = runs
        .iter()
        .enumerate()
        .max_by_key(|(_, r)| r.use_frames)
        .map(|(i, _)| i)
        .ok_or_else(|| error("no runs"))?;
    Ok(Report {
        interval_s: interval,
        runs,
        recommended,
    })
}

fn median_of(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    match v.len() {
        0 => 0.0,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// Median foreground step of frames a+1..b and the robust scatter about it.
fn step_stats(steps: &Steps, a: usize, b: usize) -> ([f64; 2], f64) {
    let tracked: Vec<[f64; 2]> = (a + 1..b).filter_map(|i| steps.foreground[i]).collect();
    let centre = [0, 1].map(|k| median_of(tracked.iter().map(|s| s[k]).collect()));
    let spread = 1.4826
        * median_of(
            tracked
                .iter()
                .map(|s| (s[0] - centre[0]).hypot(s[1] - centre[1]))
                .collect(),
        );
    (centre, spread)
}

/// Frames of a..b where the camera moved (both witnesses), excluding lit and overexposed
/// frames and the frames after them.
fn moves(c: &[Capture], steps: &Steps, a: usize, b: usize) -> Vec<usize> {
    let usable: Vec<usize> = (a..b)
        .filter(|&i| c[i].saturated_share <= OVEREXPOSED_SHARE)
        .collect();
    let lit = lit(c, &usable);
    let unreliable = |i: usize| {
        [i, i - 1]
            .iter()
            .any(|&j| c[j].saturated_share > OVEREXPOSED_SHARE || lit.contains(&j))
    };
    let (centre, spread) = step_stats(steps, a, b);
    let limit = MOVE_FLOOR_PX.max(MOVE_SIGMA * spread);
    (a + 1..b)
        .filter(|&i| !unreliable(i))
        .filter(|&i| {
            steps.foreground[i].is_none_or(|s| (s[0] - centre[0]).hypot(s[1] - centre[1]) > limit)
        })
        .filter(|&i| sky_break(steps, i, a, b))
        .collect()
}

/// Whether the star step into frame i departs from its neighbours' median, or is unmeasured.
fn sky_break(steps: &Steps, i: usize, a: usize, b: usize) -> bool {
    let Some(me) = steps.sky[i] else {
        return true;
    };
    let near: Vec<[f64; 3]> = ((a + 1).max(i.saturating_sub(SKY_WINDOW))
        ..b.min(i + SKY_WINDOW + 1))
        .filter(|&j| j != i)
        .filter_map(|j| steps.sky[j])
        .collect();
    if near.len() < 2 {
        return true;
    }
    let typical = [0, 1, 2].map(|k| median_of(near.iter().map(|s| s[k]).collect()));
    (me[0] - typical[0]).hypot(me[1] - typical[1]) > SKY_BREAK_PX
        || (me[2] - typical[2]).abs() > SKY_BREAK_DEG
}

fn run(c: &[Capture], steps: &Steps, a: usize, b: usize, starts_after: &str) -> Run {
    let names = |idx: &[usize]| idx.iter().map(|&i| c[i].name.clone()).collect::<Vec<_>>();
    let over: Vec<usize> = (a..b)
        .filter(|&i| c[i].saturated_share > OVEREXPOSED_SHARE)
        .collect();
    let usable: Vec<usize> = (a..b).filter(|i| !over.contains(i)).collect();
    let twilight = ramp(c, &usable);
    let chosen: Vec<usize> = match &twilight {
        Some((_, range)) => range.clone(),
        None => usable.clone(),
    };
    let pick = |f: fn(&[usize]) -> Option<&usize>| {
        f(&chosen).map_or_else(String::new, |&i| c[i].name.clone())
    };
    Run {
        first: c[a].name.clone(),
        last: c[b - 1].name.clone(),
        frames: b - a,
        starts_after: starts_after.into(),
        overexposed: names(&over),
        lit: names(&lit(c, &usable)),
        twilight: twilight.map(|(brightening, range)| Twilight {
            brightening,
            dark_first: c[range[0]].name.clone(),
            dark_last: c[range[range.len() - 1]].name.clone(),
        }),
        use_first: pick(<[usize]>::first),
        use_last: pick(<[usize]>::last),
        use_frames: chosen.len(),
        step_scatter_px: step_stats(steps, a, b).1,
        drift_px: (a + 1..b)
            .filter_map(|i| steps.foreground[i])
            .fold([0.0, 0.0], |d, s| [d[0] + s[0], d[1] + s[1]]),
    }
}

/// Frames whose lower half is brighter or redder than the lower quartile of their usable
/// neighbours within LIT_WINDOW (clusters of lit frames and slow twilight stay honest).
fn lit(c: &[Capture], usable: &[usize]) -> Vec<usize> {
    let quartile = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 4]
    };
    (0..usable.len())
        .filter(|&k| {
            let near: Vec<usize> = (k.saturating_sub(LIT_WINDOW)
                ..(k + LIT_WINDOW + 1).min(usable.len()))
                .filter(|&j| j != k)
                .map(|j| usable[j])
                .collect();
            if near.is_empty() {
                return false;
            }
            let g = |i: usize| c[i].ground_rgb;
            let base: [f64; 3] =
                std::array::from_fn(|ch| quartile(near.iter().map(|&j| g(j)[ch]).collect()));
            let me = g(usable[k]);
            let bright = me.iter().sum::<f64>() / base.iter().sum::<f64>().max(1e-9);
            let red = (me[0] / me[1].max(1e-9)) / (base[0] / base[1].max(1e-9)).max(1e-9);
            bright > LIT_FACTOR || red > LIT_FACTOR
        })
        .map(|k| usable[k])
        .collect()
}

/// A steady climb or fall of the smoothed sky level by more than RAMP_FACTOR: the factor
/// and the contiguous darkest stretch within RAMP_FACTOR of the minimum.
fn ramp(c: &[Capture], usable: &[usize]) -> Option<(f64, Vec<usize>)> {
    if usable.len() < 2 * RAMP_HALF_WINDOW + 2 {
        return None;
    }
    let smooth: Vec<f64> = (0..usable.len())
        .map(|k| {
            let mut v: Vec<f64> = (k.saturating_sub(RAMP_HALF_WINDOW)
                ..(k + RAMP_HALF_WINDOW + 1).min(usable.len()))
                .map(|j| c[usable[j]].sky_level.max(1e-9))
                .collect();
            v.sort_by(f64::total_cmp);
            v[v.len() / 2]
        })
        .collect();
    let (first, last) = (smooth[0], smooth[smooth.len() - 1]);
    let rising = last > first;
    let change = if rising { last / first } else { first / last };
    let steps: Vec<f64> = smooth
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d != 0.0)
        .collect();
    let steady =
        steps.iter().filter(|d| (**d > 0.0) == rising).count() as f64 / steps.len().max(1) as f64;
    if change <= RAMP_FACTOR || steady < RAMP_STEADY {
        return None;
    }
    let lowest = smooth.iter().copied().fold(f64::INFINITY, f64::min);
    let dark: Vec<usize> = (0..usable.len())
        .filter(|&k| smooth[k] <= RAMP_FACTOR * lowest)
        .collect();
    let (lo, hi) = if rising {
        (0, dark.last().copied()?)
    } else {
        (dark.first().copied()?, usable.len() - 1)
    };
    Some((change, usable[lo..=hi].to_vec()))
}
