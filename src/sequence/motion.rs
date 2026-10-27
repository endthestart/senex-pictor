//! Frame-to-frame motion for the sequence check (ADR 0010): the foreground's shift, tracked
//! on star-free half-resolution luminance of its lower half, and the stars' similarity step,
//! from triangle matching of the brightest stars of the upper half.

use crate::calibration::Mosaic;
use crate::flow::{self, Gray, Pyramid};
use crate::stars::similarity::{Point, Random};
use crate::stars::{detect, triangles};
use serde::Serialize;

/// Log scale of the feature image (ADU of a 2×2 site sum).
const FEATURE_SCALE: f32 = 40.0;
const CORNERS: usize = 1500;
const CORNER_SPACING: f64 = 10.0;
const ROUND_TRIP_PX: f64 = 0.5;
const MIN_TRACKED: usize = 20;
const STAR_COUNT: usize = 300;
const MIN_STARS: usize = 10;

/// What one frame contributes to motion: its feature image and brightest upper-half stars,
/// both at half resolution.
pub struct Look {
    features: Gray,
    stars: Vec<Point>,
}

/// Per frame (from the second on) the motion from the previous frame, in full-resolution px.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Steps {
    pub foreground: Vec<Option<[f64; 2]>>,
    /// Star translation (x, y) and rotation (degrees).
    pub sky: Vec<Option<[f64; 3]>>,
}

impl Steps {
    /// A sequence without any motion measured or needed (tests, single frames).
    pub fn still(frames: usize) -> Steps {
        Steps {
            foreground: vec![Some([0.0, 0.0]); frames],
            sky: vec![Some([0.0, 0.0, 0.0]); frames],
        }
    }
}

/// The look of a RAW mosaic's default crop: 2×2 site sums above black.
pub fn look(mosaic: &Mosaic, crop: (usize, usize, usize, usize)) -> crate::Result<Look> {
    let (cx, cy, cw, ch) = crop;
    let above: Vec<f32> = (0..cw * ch)
        .map(|i| {
            let (x, y) = (cx + i % cw, cy + i / cw);
            mosaic.data[y * mosaic.width + x] - mosaic.black.at(x, y, 0, 1)
        })
        .collect();
    let (half, w, h) = flow::half_sums(&above, cw, ch);
    let upper = &half[..w * (h / 2)];
    let found = detect(upper, &vec![false; upper.len()], w, h / 2)?;
    let mut order: Vec<usize> = (0..found.len()).collect();
    order.sort_by(|&a, &b| found[b].flux.total_cmp(&found[a].flux));
    let stars = order
        .iter()
        .take(STAR_COUNT)
        .map(|&i| [found[i].x, found[i].y])
        .collect();
    Ok(Look {
        features: flow::features(&half, w, h, FEATURE_SCALE),
        stars,
    })
}

/// Median shift of lower-half corners from `a` to `b` (full-resolution px), or None when
/// fewer than MIN_TRACKED corners survive the round trip.
pub fn foreground_step(a: &Look, b: &Look) -> Option<[f64; 2]> {
    let (w, h) = (a.features.width, a.features.height);
    let allowed: Vec<bool> = (0..w * h).map(|i| i / w >= h / 2).collect();
    let corners = flow::corners(&a.features, &allowed, CORNERS, CORNER_SPACING);
    let (pa, pb) = (Pyramid::new(&a.features), Pyramid::new(&b.features));
    let moved: Vec<[f64; 2]> = flow::follow_checked(&pa, &pb, &corners, None, ROUND_TRIP_PX)
        .iter()
        .zip(&corners)
        .filter_map(|(f, c)| f.map(|f| [f[0] - c[0], f[1] - c[1]]))
        .collect();
    if moved.len() < MIN_TRACKED {
        return None;
    }
    let median = |k: usize| {
        let mut v: Vec<f64> = moved.iter().map(|m| m[k]).collect();
        v.sort_by(f64::total_cmp);
        let n = v.len();
        if n % 2 == 1 {
            v[n / 2]
        } else {
            (v[n / 2 - 1] + v[n / 2]) / 2.0
        }
    };
    Some([2.0 * median(0), 2.0 * median(1)])
}

/// The stars' similarity step from `a` to `b`: translation (full-resolution px), rotation (°).
pub fn sky_step(a: &Look, b: &Look, random: &mut Random) -> Option<[f64; 3]> {
    if a.stars.len() < MIN_STARS || b.stars.len() < MIN_STARS {
        return None;
    }
    let t = triangles::find(&a.stars, &b.stars, random)?;
    Some([2.0 * t.t[0], 2.0 * t.t[1], t.b.atan2(t.a).to_degrees()])
}
