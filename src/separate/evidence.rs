//! Where motion outranks brightness in the matte: how far each pixel lies from the nearest
//! confirmed star transit against the usual gap between transits around it, and light of
//! its own colour where no star crosses (leaves under a lamp read as sky by brightness alone).

use super::filters::{dilate, distance_squared};
use super::grid::{fill_nearest, gaussian, resize};
use super::stats::{Stats, median, percentile};

pub const NEAR_RADIUS_PX: f64 = 6.0;
pub const MAX_GAP_PX: f64 = 32.0;
const GAP_BLOCK: usize = 128;
/// Red plus blue share apart from the transit-confirmed sky nearby that marks lit foreground
/// (clear sky stays within 0.1 on the five owner sequences, lamp-lit leaves exceed 0.2).
pub const LIT_COLOUR: f32 = 0.15;
/// Confirmed sky pixels a block needs to give its sky colour.
const MIN_SKY_PIXELS: usize = 64;

/// Distance in pixels from each pixel to the nearest transit; none without transits.
pub fn gaps(seen: &[bool], w: usize, h: usize) -> Option<Vec<f64>> {
    distance_squared(seen, w, h).map(|(d, _)| d.iter().map(|d| d.sqrt()).collect())
}

/// Per pixel a GAP_BLOCK grid's values (empty cells take the nearest cell's), bilinear.
fn spread(cells: &[Option<f64>], w: usize, h: usize, smooth: bool) -> Option<Vec<f32>> {
    let b = GAP_BLOCK;
    let (gw, gh) = (w.div_ceil(b), h.div_ceil(b));
    let filled = fill_nearest(cells, gw, gh)?;
    let grid = if smooth {
        gaussian(&filled, gw, gh, 1.0, true)
    } else {
        filled
    };
    let up = resize(&grid, gw, gh, gw * b, gh * b);
    Some((0..w * h).map(|p| up[(p / w) * gw * b + p % w]).collect())
}

/// Per GAP_BLOCK block, `of` the `value`s of the pixels where `keep` holds, when at least
/// `least` do.
fn cells(
    (w, h): (usize, usize),
    least: usize,
    keep: &dyn Fn(usize) -> bool,
    value: &dyn Fn(usize) -> f32,
    of: fn(&mut [f32]) -> f32,
) -> Vec<Option<f64>> {
    let b = GAP_BLOCK;
    let mut out = Vec::new();
    for j in 0..h.div_ceil(b) {
        for i in 0..w.div_ceil(b) {
            let mut v: Vec<f32> = (j * b..((j + 1) * b).min(h))
                .flat_map(|y| (i * b..((i + 1) * b).min(w)).map(move |x| y * w + x))
                .filter(|&p| keep(p))
                .map(value)
                .collect();
            out.push((v.len() >= least).then(|| f64::from(of(&mut v))));
        }
    }
    out
}

/// Whether each pixel lies within the local transit gap (the 99th-percentile gap over the
/// brightness-clear pixels of its block, log-smoothed) of a transit: everywhere where gaps
/// exceed MAX_GAP_PX or no transit was seen, so sparse skies keep their brightness matte.
pub fn near_transits(gap: Option<&[f64]>, alpha: &[f32], w: usize, h: usize) -> Vec<bool> {
    let Some(gap) = gap else {
        return vec![true; w * h];
    };
    let least = (0.05 * (GAP_BLOCK * GAP_BLOCK) as f64) as usize;
    let gaps = cells(
        (w, h),
        least,
        &|p| alpha[p] >= 0.98,
        &|p| gap[p] as f32,
        |v| percentile(v, 99.0),
    );
    let logs: Vec<Option<f64>> = gaps
        .iter()
        .map(|c| c.map(|v| v.max(NEAR_RADIUS_PX).ln()))
        .collect();
    let Some(radius) = spread(&logs, w, h, true) else {
        return vec![true; w * h];
    };
    (0..w * h)
        .map(|p| {
            let r = f64::from(radius[p]).exp();
            gap[p] <= r || r > MAX_GAP_PX
        })
        .collect()
}

/// Light of its own colour where no star crosses: a colour LIT_COLOUR or more from the
/// transit-confirmed sky of its block with no transit within NEAR_RADIUS_PX, and around such
/// cores, out to NEAR_RADIUS_PX, the pixels of that colour no transit crossed.
pub fn lit(stats: &Stats, seen: &[bool], gap: Option<&[f64]>) -> Vec<bool> {
    let (w, h) = (stats.width, stats.height);
    let Some(gap) = gap else {
        return vec![false; w * h];
    };
    let sky = |plane: &[f32]| {
        let medians = cells((w, h), MIN_SKY_PIXELS, &|p| seen[p], &|p| plane[p], median);
        spread(&medians, w, h, false)
    };
    let (Some(red), Some(blue)) = (sky(&stats.red), sky(&stats.blue)) else {
        return vec![false; w * h];
    };
    let apart: Vec<bool> = (0..w * h)
        .map(|p| (stats.red[p] - red[p]).abs() + (stats.blue[p] - blue[p]).abs() >= LIT_COLOUR)
        .collect();
    let core: Vec<bool> = (0..w * h)
        .map(|p| apart[p] && gap[p] > NEAR_RADIUS_PX)
        .collect();
    let around = dilate(&core, w, h, NEAR_RADIUS_PX as usize);
    (0..w * h)
        .map(|p| apart[p] && !seen[p] && around[p])
        .collect()
}
