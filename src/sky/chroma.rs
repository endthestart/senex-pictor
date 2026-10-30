//! Lateral chromatic aberration measured from the stars, so no lens profile is needed: each
//! colour's image is green's scaled about the image centre by 1 + a + b·r² (r = 1 at the
//! corner). Centroids on the raw colour sites of isolated, unsaturated stars (demosaicing
//! would pull red and blue toward green) give each colour's radial offset from green, and a
//! least-squares fit with 4σ rejection gives a and b.

use super::Camera;
use crate::stars::Star;
use crate::stars::similarity::Point;
use serde::{Deserialize, Serialize};

/// Fewer usable stars than this leave the aberration unmeasured.
pub const MIN_STARS: usize = 200;
const WINDOW: isize = 9;
const CORE_PX: f64 = 3.5;
const CENTROID_ROUNDS: usize = 3;
const ANNULUS_PX: [f64; 2] = [6.0, 9.0];
/// Another star nearer than this spoils a centroid.
const ISOLATION_PX: f64 = 12.0;
/// Stars between these flux percentiles: bright enough to centroid, short of saturation.
const FLUX_PERCENTILES: [f64; 2] = [60.0, 99.7];
const SIGNAL_SIGMA: f64 = 8.0;
const REJECT_SIGMA: f64 = 4.0;
const FIT_ROUNDS: usize = 3;

/// Radial scale terms (a, b) of red and blue relative to green.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chroma {
    pub red: [f64; 2],
    pub blue: [f64; 2],
}

impl Chroma {
    pub const NONE: Chroma = Chroma {
        red: [0.0; 2],
        blue: [0.0; 2],
    };

    /// Where colour `colour` (0 red, 1 green, 2 blue) images the point green images at `p`.
    pub fn place(&self, camera: &Camera, colour: usize, p: Point) -> Point {
        let [a, b] = match colour {
            0 => self.red,
            2 => self.blue,
            _ => return p,
        };
        let (c, m) = (camera.centre(), camera.scale());
        let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
        let g = 1.0 + a + b * (dx * dx + dy * dy) / (m * m);
        [c[0] + dx * g, c[1] + dy * g]
    }
}

/// What the stars showed: offsets from green at mid-radius and at the corner (px), the
/// residual σ of each fit (px), and whether enough stars measured it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub stars: usize,
    pub measured: bool,
    pub red_px: [f64; 2],
    pub blue_px: [f64; 2],
    pub residual_px: [f64; 2],
}

/// The aberration fitted to `offsets`; none (and `measured` false) under MIN_STARS stars.
pub fn fitted(offsets: &[Offset], scale: f64) -> (Chroma, Report) {
    let r: Vec<f64> = offsets.iter().map(|o| o.r).collect();
    let red: Vec<f64> = offsets.iter().map(|o| o.red).collect();
    let blue: Vec<f64> = offsets.iter().map(|o| o.blue).collect();
    let at = |ab: [f64; 2], r: f64| scale * (ab[0] * r + ab[1] * r.powi(3));
    match (fit(&r, &red, scale), fit(&r, &blue, scale)) {
        (Some((red, sr)), Some((blue, sb))) if offsets.len() >= MIN_STARS => (
            Chroma { red, blue },
            Report {
                stars: offsets.len(),
                measured: true,
                red_px: [at(red, 0.5), at(red, 1.0)],
                blue_px: [at(blue, 0.5), at(blue, 1.0)],
                residual_px: [sr, sb],
            },
        ),
        _ => (
            Chroma::NONE,
            Report {
                stars: offsets.len(),
                measured: false,
                red_px: [0.0; 2],
                blue_px: [0.0; 2],
                residual_px: [0.0; 2],
            },
        ),
    }
}

/// One star's radius (1 at the corner) and the radial offsets of red and blue from green, px.
#[derive(Clone, Copy, Debug)]
pub struct Offset {
    pub r: f64,
    pub red: f64,
    pub blue: f64,
}

/// A frame's raw colour sites: samples as stored, the CFA repeat and per-site black.
pub struct Raw<'a> {
    pub samples: &'a [u16],
    pub width: usize,
    pub height: usize,
    pub cfa: &'a [u8],
    pub cfa_width: usize,
    pub cfa_height: usize,
    pub black: &'a [f32],
    pub white: f32,
}

impl Raw<'_> {
    fn site(&self, x: usize, y: usize) -> (usize, f64) {
        let k = (y % self.cfa_height) * self.cfa_width + x % self.cfa_width;
        let v = f64::from(self.samples[y * self.width + x]);
        (usize::from(self.cfa[k]), v - f64::from(self.black[k]))
    }
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// Centroid of colour `colour` near `star`, its window re-centred on the colour's own
/// centroid (a window on green's would clip an offset colour's far side), or None when its
/// signal is weak or clipped.
fn centroid(raw: &Raw, star: Point, colour: usize) -> Option<Point> {
    let (cx, cy) = (star[0].round() as isize, star[1].round() as isize);
    let (mut ring, mut sites) = (Vec::new(), Vec::new());
    for dy in -WINDOW..=WINDOW {
        for dx in -WINDOW..=WINDOW {
            let (x, y) = ((cx + dx) as usize, (cy + dy) as usize);
            let (c, v) = raw.site(x, y);
            if c != colour {
                continue;
            }
            if (ANNULUS_PX[0]..=ANNULUS_PX[1]).contains(&((dx * dx + dy * dy) as f64).sqrt()) {
                ring.push(v);
            } else {
                let clipped = f64::from(raw.samples[y * raw.width + x]) >= f64::from(raw.white);
                sites.push((x as f64, y as f64, v, clipped));
            }
        }
    }
    if ring.len() < 8 {
        return None;
    }
    let background = median(&mut ring);
    let mut spread: Vec<f64> = ring.iter().map(|v| (v - background).abs()).collect();
    let noise = 1.4826 * median(&mut spread);
    let mut centre = star;
    let mut signal = 0.0;
    for _ in 0..CENTROID_ROUNDS {
        let (mut sx, mut sy, mut sum, mut count) = (0.0, 0.0, 0.0, 0);
        for &(x, y, v, clipped) in &sites {
            if (x - centre[0]).hypot(y - centre[1]) > CORE_PX {
                continue;
            }
            if clipped {
                return None;
            }
            let s = (v - background).max(0.0);
            sx += x * s;
            sy += y * s;
            sum += s;
            count += 1;
        }
        if count < 3 || sum <= 0.0 {
            return None;
        }
        centre = [sx / sum, sy / sum];
        signal = sum / (count as f64).sqrt();
    }
    (signal >= SIGNAL_SIGMA * noise).then_some(centre)
}

fn percentile(values: &[f64], q: f64) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    v[((q / 100.0) * (v.len() - 1) as f64).round() as usize]
}

/// Radial offsets of the isolated, unsaturated stars of one frame.
pub fn offsets(raw: &Raw, stars: &[Star], camera: &Camera) -> Vec<Offset> {
    let usable: Vec<&Star> = stars.iter().filter(|s| !s.saturated).collect();
    if usable.is_empty() {
        return Vec::new();
    }
    let fluxes: Vec<f64> = usable.iter().map(|s| s.flux).collect();
    let (lo, hi) = (
        percentile(&fluxes, FLUX_PERCENTILES[0]),
        percentile(&fluxes, FLUX_PERCENTILES[1]),
    );
    let cell = ISOLATION_PX;
    let key = |x: f64, y: f64| ((x / cell) as i64, (y / cell) as i64);
    let mut grid: std::collections::HashMap<(i64, i64), Vec<Point>> = Default::default();
    for s in stars {
        grid.entry(key(s.x, s.y)).or_default().push([s.x, s.y]);
    }
    let crowded = |s: &Star| {
        let (kx, ky) = key(s.x, s.y);
        (kx - 1..=kx + 1)
            .flat_map(|i| (ky - 1..=ky + 1).map(move |j| (i, j)))
            .filter_map(|k| grid.get(&k))
            .flatten()
            .filter(|p| (p[0] - s.x).hypot(p[1] - s.y) < ISOLATION_PX)
            .count()
            > 1
    };
    let margin = (WINDOW + 1) as f64;
    let (c, m) = (camera.centre(), camera.scale());
    usable
        .into_iter()
        .filter(|s| s.flux > lo && s.flux < hi)
        .filter(|s| {
            s.x >= margin
                && s.y >= margin
                && s.x < raw.width as f64 - margin
                && s.y < raw.height as f64 - margin
        })
        .filter(|s| !crowded(s))
        .filter_map(|s| {
            let star = [s.x, s.y];
            let [r, g, b] = [0, 1, 2].map(|k| centroid(raw, star, k));
            let (r, g, b) = (r?, g?, b?);
            let (dx, dy) = (g[0] - c[0], g[1] - c[1]);
            let radius = dx.hypot(dy);
            (radius >= 1.0).then(|| {
                let u = [dx / radius, dy / radius];
                let along = |p: Point| (p[0] - g[0]) * u[0] + (p[1] - g[1]) * u[1];
                Offset {
                    r: radius / m,
                    red: along(r),
                    blue: along(b),
                }
            })
        })
        .collect()
}

/// The fitted (a, b) of offsets `d` (px) at radii `r` with `scale` px at r = 1, and the
/// residual σ in px; 4σ outliers are dropped over FIT_ROUNDS rounds.
pub fn fit(r: &[f64], d: &[f64], scale: f64) -> Option<([f64; 2], f64)> {
    let mut keep = vec![true; r.len()];
    let mut result = None;
    for _ in 0..FIT_ROUNDS {
        let (mut s11, mut s12, mut s22, mut t1, mut t2) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for i in (0..r.len()).filter(|&i| keep[i]) {
            let (x1, x2) = (scale * r[i], scale * r[i].powi(3));
            s11 += x1 * x1;
            s12 += x1 * x2;
            s22 += x2 * x2;
            t1 += x1 * d[i];
            t2 += x2 * d[i];
        }
        let det = s11 * s22 - s12 * s12;
        if det.abs() < 1e-12 {
            return None;
        }
        let ab = [(t1 * s22 - t2 * s12) / det, (s11 * t2 - s12 * t1) / det];
        let residual: Vec<f64> = (0..r.len())
            .map(|i| d[i] - scale * (ab[0] * r[i] + ab[1] * r[i].powi(3)))
            .collect();
        let mut kept: Vec<f64> = (0..r.len())
            .filter(|&i| keep[i])
            .map(|i| residual[i].abs())
            .collect();
        let sigma = 1.4826 * median(&mut kept);
        for i in 0..r.len() {
            keep[i] = residual[i].abs() < REJECT_SIGMA * sigma;
        }
        result = Some((ab, sigma));
    }
    result
}
