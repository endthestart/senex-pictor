//! Star detection after Source Extractor (sep is the oracle): background subtraction, the
//! normalised 3×3 smoothing filter, 8-connected pixels of the smoothed image above
//! THRESHOLD_SIGMA times the background noise, shape cuts on the smoothed moments, then
//! windowed (Gaussian-weighted) centroids on the unsmoothed signal. Blends, objects touching
//! the border and degenerate shapes are dropped, as the prototype drops sep's flagged objects.

use super::background::Background;
use crate::{Result, error};
use serde::{Deserialize, Serialize};

pub const THRESHOLD_SIGMA: f64 = 8.0;
pub const MIN_AREA: usize = 4;
pub const MAX_AREA: usize = 400;
pub const MAX_ELONGATION: f64 = 3.0;
/// Thresholds between detection level and peak tried when looking for blended objects, and
/// the share of the object's smoothed flux a separate peak must hold.
const DEBLEND_LEVELS: usize = 32;
const DEBLEND_CONTRAST: f64 = 0.005;
/// Second moments with a determinant below this (a one-pixel-wide object) are degenerate.
const SINGULAR: f64 = 1.0 / 144.0;
const WINDOW_SIGMA: f64 = 1.0;
const WINDOW_RADIUS: f64 = 4.0 * WINDOW_SIGMA;
const WINDOW_SUBPIXELS: usize = 11;
const WINDOW_ROUNDS: usize = 16;
const WINDOW_STEP: f64 = 2e-4;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Star {
    pub x: f64,
    pub y: f64,
    pub flux: f64,
    pub saturated: bool,
}

/// Stars in `image` (pixel centres at integer coordinates); `saturated` marks clipped pixels.
pub fn detect(image: &[f32], saturated: &[bool], width: usize, height: usize) -> Result<Vec<Star>> {
    if saturated.len() != image.len() {
        return Err(error("saturation mask and image differ in size"));
    }
    let background = Background::estimate(image, width, height)?;
    if !background.rms.is_finite() || background.rms < 0.0 {
        return Err(error("the frame's background noise is not a number"));
    }
    if background.rms == 0.0 {
        // Most meshes are constant: the frame is clipped or blank and shows no stars.
        return Ok(Vec::new());
    }
    let signal = background.subtract(image, width, height);
    let smooth = smoothed(&signal, width, height);
    let threshold = (THRESHOLD_SIGMA * background.rms) as f32;
    let mut seen = vec![false; image.len()];
    let mut pixels = Vec::new();
    let mut stars = Vec::new();
    for start in 0..image.len() {
        if seen[start] || smooth[start] <= threshold {
            continue;
        }
        component(
            start,
            &smooth,
            threshold,
            width,
            height,
            &mut seen,
            &mut pixels,
        );
        let Some((x0, y0, flux)) = measure(&pixels, &signal, &smooth, threshold, width, height)
        else {
            continue;
        };
        let Some((x, y)) = windowed(&signal, width, height, x0, y0) else {
            continue;
        };
        let near = |v: f64, n: usize| (v.round().max(0.0) as usize).min(n - 1);
        stars.push(Star {
            x,
            y,
            flux,
            saturated: saturated[near(y, height) * width + near(x, width)],
        });
    }
    Ok(stars)
}

/// Convolution with [[1,2,1],[2,4,2],[1,2,1]]/16; samples outside the image count as zero.
fn smoothed(signal: &[f32], width: usize, height: usize) -> Vec<f32> {
    const K: [[f32; 3]; 3] = [[1.0, 2.0, 1.0], [2.0, 4.0, 2.0], [1.0, 2.0, 1.0]];
    let mut out = vec![0f32; signal.len()];
    for y in 0..height {
        for x in 0..width {
            let mut s = 0.0;
            for (dy, row) in K.iter().enumerate() {
                let Some(yy) = (y + dy).checked_sub(1).filter(|v| *v < height) else {
                    continue;
                };
                for (dx, k) in row.iter().enumerate() {
                    if let Some(xx) = (x + dx).checked_sub(1).filter(|v| *v < width) {
                        s += k * signal[yy * width + xx];
                    }
                }
            }
            out[y * width + x] = s / 16.0;
        }
    }
    out
}

fn neighbours(p: usize, width: usize, height: usize) -> impl Iterator<Item = usize> {
    let (x, y) = ((p % width) as isize, (p / width) as isize);
    (-1..=1)
        .flat_map(move |dy| (-1..=1).map(move |dx| (x + dx, y + dy)))
        .filter(move |&(xx, yy)| {
            (xx, yy) != (x, y) && xx >= 0 && yy >= 0 && xx < width as isize && yy < height as isize
        })
        .map(move |(xx, yy)| yy as usize * width + xx as usize)
}

/// The 8-connected pixels above `threshold` reached from `start`.
fn component(
    start: usize,
    smooth: &[f32],
    threshold: f32,
    width: usize,
    height: usize,
    seen: &mut [bool],
    pixels: &mut Vec<usize>,
) {
    pixels.clear();
    pixels.push(start);
    seen[start] = true;
    let mut next = 0;
    while next < pixels.len() {
        let p = pixels[next];
        next += 1;
        for q in neighbours(p, width, height) {
            if !seen[q] && smooth[q] > threshold {
                seen[q] = true;
                pixels.push(q);
            }
        }
    }
}

/// Barycentre and flux of a usable object, or None for objects outside the area limits,
/// touching the border, degenerate, too elongated or blended.
fn measure(
    pixels: &[usize],
    signal: &[f32],
    smooth: &[f32],
    threshold: f32,
    width: usize,
    height: usize,
) -> Option<(f64, f64, f64)> {
    if pixels.len() < MIN_AREA || pixels.len() >= MAX_AREA {
        return None;
    }
    let xy = |p: usize| (p % width, p / width);
    if pixels.iter().any(|&p| {
        let (x, y) = xy(p);
        x == 0 || y == 0 || x == width - 1 || y == height - 1
    }) {
        return None;
    }
    let (ox, oy) = xy(pixels[0]);
    let (mut s, mut sx, mut sy, mut sxx, mut syy, mut sxy, mut flux) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for &p in pixels {
        let (x, y) = xy(p);
        let (dx, dy) = (x as f64 - ox as f64, y as f64 - oy as f64);
        let v = f64::from(smooth[p]);
        s += v;
        sx += v * dx;
        sy += v * dy;
        sxx += v * dx * dx;
        syy += v * dy * dy;
        sxy += v * dx * dy;
        flux += f64::from(signal[p]);
    }
    let (mx, my) = (sx / s, sy / s);
    let (x2, y2, xy2) = (sxx / s - mx * mx, syy / s - my * my, sxy / s - mx * my);
    if x2 * y2 - xy2 * xy2 < SINGULAR {
        return None;
    }
    let spread = (((x2 - y2) / 2.0).powi(2) + xy2 * xy2).sqrt();
    let (a2, b2) = ((x2 + y2) / 2.0 + spread, (x2 + y2) / 2.0 - spread);
    if b2 <= 0.0 || a2.sqrt() >= MAX_ELONGATION * b2.sqrt() {
        return None;
    }
    if blended(pixels, smooth, threshold, width) {
        return None;
    }
    Some((ox as f64 + mx, oy as f64 + my, flux))
}

/// Whether the object splits into two peaks at some threshold between detection and peak,
/// each of at least MIN_AREA pixels and holding, above that threshold, DEBLEND_CONTRAST of
/// the object's smoothed flux.
fn blended(pixels: &[usize], smooth: &[f32], threshold: f32, width: usize) -> bool {
    let (x0, y0) = pixels.iter().fold((usize::MAX, usize::MAX), |(a, b), &p| {
        (a.min(p % width), b.min(p / width))
    });
    let (x1, y1) = pixels
        .iter()
        .fold((0, 0), |(a, b), &p| (a.max(p % width), b.max(p / width)));
    let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
    let mut local = vec![f32::NEG_INFINITY; w * h];
    for &p in pixels {
        local[(p / width - y0) * w + p % width - x0] = smooth[p];
    }
    let peak = local.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let total: f64 = pixels.iter().map(|&p| f64::from(smooth[p])).sum();
    let ratio = f64::from(peak) / f64::from(threshold);
    let mut seen = vec![false; w * h];
    let mut part = Vec::new();
    for k in 1..DEBLEND_LEVELS {
        let level = (f64::from(threshold) * ratio.powf(k as f64 / DEBLEND_LEVELS as f64)) as f32;
        seen.iter_mut().for_each(|s| *s = false);
        let mut peaks = 0;
        for start in 0..local.len() {
            if seen[start] || local[start] <= level {
                continue;
            }
            component(start, &local, level, w, h, &mut seen, &mut part);
            let above: f64 = part.iter().map(|&p| f64::from(local[p] - level)).sum();
            if part.len() >= MIN_AREA && above > DEBLEND_CONTRAST * total {
                peaks += 1;
                if peaks >= 2 {
                    return true;
                }
            }
        }
    }
    false
}

/// Windowed centroid (Source Extractor XWIN/YWIN): iterate x ← x + 2·Σw·I·(xᵢ − x) / Σw·I with
/// Gaussian weights of σ WINDOW_SIGMA inside a circle of WINDOW_RADIUS (edge pixels by their
/// subsampled overlap with the circle).
fn windowed(signal: &[f32], width: usize, height: usize, x0: f64, y0: f64) -> Option<(f64, f64)> {
    let (mut x, mut y) = (x0, y0);
    let reach = WINDOW_RADIUS + 1.0;
    for _ in 0..WINDOW_ROUNDS {
        let (mut sw, mut sdx, mut sdy) = (0.0, 0.0, 0.0);
        let ys =
            (y - reach).floor().max(0.0) as usize..=((y + reach).ceil() as usize).min(height - 1);
        for py in ys {
            let xs = (x - reach).floor().max(0.0) as usize
                ..=((x + reach).ceil() as usize).min(width - 1);
            for px in xs {
                let (dx, dy) = (px as f64 - x, py as f64 - y);
                let inside = overlap(dx, dy);
                if inside == 0.0 {
                    continue;
                }
                let w = inside
                    * (-(dx * dx + dy * dy) / (2.0 * WINDOW_SIGMA * WINDOW_SIGMA)).exp()
                    * f64::from(signal[py * width + px]);
                sw += w;
                sdx += w * dx;
                sdy += w * dy;
            }
        }
        if sw <= 0.0 {
            return None;
        }
        let (step_x, step_y) = (2.0 * sdx / sw, 2.0 * sdy / sw);
        x += step_x;
        y += step_y;
        if step_x * step_x + step_y * step_y < WINDOW_STEP * WINDOW_STEP {
            break;
        }
    }
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// Share of the pixel centred at (dx, dy) from the window centre inside the window circle.
fn overlap(dx: f64, dy: f64) -> f64 {
    let d = (dx * dx + dy * dy).sqrt();
    if d < WINDOW_RADIUS - std::f64::consts::FRAC_1_SQRT_2 {
        return 1.0;
    }
    if d > WINDOW_RADIUS + std::f64::consts::FRAC_1_SQRT_2 {
        return 0.0;
    }
    let n = WINDOW_SUBPIXELS;
    let step = 1.0 / n as f64;
    let mut inside = 0;
    for j in 0..n {
        for i in 0..n {
            let sx = dx - 0.5 + (i as f64 + 0.5) * step;
            let sy = dy - 0.5 + (j as f64 + 0.5) * step;
            inside += usize::from(sx * sx + sy * sy < WINDOW_RADIUS * WINDOW_RADIUS);
        }
    }
    inside as f64 / (n * n) as f64
}
