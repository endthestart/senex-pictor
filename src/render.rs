//! Renderings: as-shot white balance and the camera's colour matrix to linear sRGB, part of
//! the lens falloff put back. The sky (by its matte) is stretched: the camera-fixed glow
//! optionally removed, levels shared across a comparison (black at the colour of the darker
//! sky, white at a luminance percentile), an asinh stretch and a shadow curve. The foreground
//! keeps the exposure it was shot with: sensor white is display white, sRGB-encoded.

use crate::{Result, error};
use serde::{Deserialize, Serialize};

pub type Matrix = [[f64; 3]; 3];

const SRGB_TO_XYZ: Matrix = [
    [0.412_456_4, 0.357_576_1, 0.180_437_5],
    [0.212_672_9, 0.715_152_2, 0.072_175_0],
    [0.019_333_9, 0.119_192_0, 0.950_304_1],
];
const WEIGHTS: [f32; 3] = [0.2126, 0.7152, 0.0722];
/// Share (as a power of the flat field) of the lens falloff put back, so corrected corners
/// do not show their amplified noise.
pub const VIGNETTE_KEPT: f32 = 0.4;

/// A rendering look.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub remove_glow: bool,
    pub black_percentile: f64,
    pub white_percentile: f64,
    pub stretch: f32,
    pub saturation: f32,
    pub shadows: f32,
}

pub const NATURAL: Look = Look {
    remove_glow: true,
    black_percentile: 30.0,
    white_percentile: 99.9,
    stretch: 12.0,
    saturation: 1.2,
    shadows: 1.4,
};

/// Bright skies (aurora, town glow) keep their glow, deep blacks, a gentle stretch.
pub const DARK: Look = Look {
    remove_glow: false,
    black_percentile: 75.0,
    white_percentile: 99.95,
    stretch: 3.0,
    saturation: 1.4,
    shadows: 2.0,
};

fn invert(m: &Matrix) -> Option<Matrix> {
    let c = |i: usize, j: usize| {
        m[(i + 1) % 3][(j + 1) % 3] * m[(i + 2) % 3][(j + 2) % 3]
            - m[(i + 1) % 3][(j + 2) % 3] * m[(i + 2) % 3][(j + 1) % 3]
    };
    let det: f64 = (0..3).map(|j| m[0][j] * c(0, j)).sum();
    (det.abs() > 1e-12).then(|| std::array::from_fn(|i| std::array::from_fn(|j| c(j, i) / det)))
}

/// White-balanced camera RGB → linear sRGB with rows summing to 1, from an XYZ → camera
/// matrix (Adobe ColorMatrix convention).
pub fn camera_to_srgb(xyz_to_camera: &Matrix) -> Result<Matrix> {
    let mut srgb_to_camera: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .map(|k| xyz_to_camera[i][k] * SRGB_TO_XYZ[k][j])
                .sum()
        })
    });
    for row in srgb_to_camera.iter_mut() {
        let s: f64 = row.iter().sum();
        if s.abs() < 1e-12 {
            return Err(error("colour matrix row sums to zero"));
        }
        row.iter_mut().for_each(|v| *v /= s);
    }
    invert(&srgb_to_camera).ok_or_else(|| error("colour matrix is singular"))
}

pub fn encode(v: f32) -> f32 {
    encode_to(v, 1.0)
}

/// The sRGB curve extended past 1 for light up to `top` times paper white.
fn encode_to(v: f32, top: f32) -> f32 {
    let a = v.clamp(0.0, top);
    if a <= 0.003_130_8 {
        12.92 * a
    } else {
        1.055 * a.powf(1.0 / 2.4) - 0.055
    }
}

pub fn decode(v: f32) -> f32 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

pub struct Scene<'a> {
    /// Weight of the stretched sky rendering per pixel (the matte).
    pub alpha: &'a [f32],
    /// Clear sky that sets the levels (without light seen in one frame only).
    pub clear: &'a [bool],
    pub glow: &'a [[f32; 3]],
    pub flat: &'a [f32],
    pub wb: [f32; 3],
    pub matrix: Matrix,
    /// Sensor white in the image's units (ADU above black).
    pub white: f32,
}

/// Linear sRGB of a flat-fielded camera-RGB image under `look`.
pub fn linear(image: &[[f32; 3]], scene: &Scene, look: &Look) -> Vec<[f32; 3]> {
    let m = scene.matrix.map(|r| r.map(|v| v as f32));
    let lowest = scene.wb.iter().copied().fold(f32::INFINITY, f32::min);
    image
        .iter()
        .enumerate()
        .map(|(p, c)| {
            let a = scene.alpha[p];
            let keep = scene.flat[p].powf(VIGNETTE_KEPT);
            // Every colour clips at the sensor white; after white balance the lowest of those
            // levels is as far as all three are known, so clipped light stays neutral.
            let clip = scene.white / scene.flat[p] * lowest;
            let v: [f32; 3] = std::array::from_fn(|k| {
                let x = if c[k].is_finite() { c[k] } else { 0.0 };
                let g = if look.remove_glow && scene.glow[p][k].is_finite() {
                    a * scene.glow[p][k]
                } else {
                    0.0
                };
                ((x * scene.wb[k]).min(clip) - g * scene.wb[k]) * keep
            });
            std::array::from_fn(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
        })
        .collect()
}

/// Black colour (median colour of the `clear` sky up to the black percentile in luminance)
/// and white scale.
pub fn levels(rgb: &[[f32; 3]], clear: &[bool], look: &Look) -> Result<([f32; 3], f32)> {
    let sky: Vec<[f32; 3]> = rgb
        .iter()
        .zip(clear)
        .filter(|(_, c)| **c)
        .map(|(c, _)| *c)
        .collect();
    if sky.is_empty() {
        return Err(error("render: no clear sky to set levels"));
    }
    let luminance = |c: &[f32; 3]| c[0] * WEIGHTS[0] + c[1] * WEIGHTS[1] + c[2] * WEIGHTS[2];
    let mut level: Vec<f32> = sky.iter().map(luminance).collect();
    let cut = crate::separate::stats::percentile(&mut level.clone(), look.black_percentile);
    let dark: Vec<&[f32; 3]> = sky.iter().filter(|c| luminance(c) <= cut).collect();
    let black: [f32; 3] = std::array::from_fn(|k| {
        crate::separate::stats::median(&mut dark.iter().map(|c| c[k]).collect::<Vec<_>>())
    });
    let b = luminance(&black);
    for l in level.iter_mut() {
        *l -= b;
    }
    let scale = crate::separate::stats::percentile(&mut level, look.white_percentile);
    if scale <= 0.0 {
        return Err(error("render: the sky has no range above its black level"));
    }
    Ok((black, scale))
}

/// A colour `d` of luminance `l` with its saturation raised by `saturation`, never below
/// `d / saturation`: extrapolating from luminance would take the weak colour of a bright,
/// strongly coloured star below zero and leave a cyan or magenta core after the stretch.
pub fn saturate(d: f32, l: f32, saturation: f32) -> f32 {
    (l + saturation * (d - l)).max(d / saturation)
}

/// Display values in 0..=1 (sRGB-encoded): black removed, saturation, asinh stretch, shadow curve.
pub fn finish(rgb: &[[f32; 3]], black: [f32; 3], scale: f32, look: &Look) -> Vec<[f32; 3]> {
    finish_to(rgb, black, scale, look, 1.0)
}

/// `finish` with light kept up to `top` times paper white.
fn finish_to(
    rgb: &[[f32; 3]],
    black: [f32; 3],
    scale: f32,
    look: &Look,
    top: f32,
) -> Vec<[f32; 3]> {
    let norm = look.stretch.asinh();
    rgb.iter()
        .map(|c| {
            let d: [f32; 3] = std::array::from_fn(|k| c[k] - black[k]);
            let l = d[0] * WEIGHTS[0] + d[1] * WEIGHTS[1] + d[2] * WEIGHTS[2];
            std::array::from_fn(|k| {
                let s = saturate(d[k], l, look.saturation);
                let y = (look.stretch * (s / scale).max(0.0)).asinh() / norm;
                encode_to(y.powf(look.shadows), top)
            })
        })
        .collect()
}

/// The stretched sky and the as-shot foreground of linear sRGB `rgb`, blended by the scene's
/// sky weight in display values.
fn display(
    rgb: &[[f32; 3]],
    scene: &Scene,
    black: [f32; 3],
    scale: f32,
    look: &Look,
    top: f32,
) -> Vec<[f32; 3]> {
    finish_to(rgb, black, scale, look, top)
        .iter()
        .zip(rgb)
        .zip(scene.alpha)
        .map(|((sky, c), &a)| {
            std::array::from_fn(|k| a * sky[k] + (1.0 - a) * encode(c[k] / scene.white))
        })
        .collect()
}

/// Renderings of `images` with the levels of the first, so a stack and a single frame are
/// judged on the same black and white points.
pub fn compare(images: &[&[[f32; 3]]], scene: &Scene, look: &Look) -> Result<Vec<Vec<[f32; 3]>>> {
    if !(scene.white.is_finite() && scene.white > 0.0) {
        return Err(error("render: the sensor white must be positive"));
    }
    let rgbs: Vec<Vec<[f32; 3]>> = images.iter().map(|i| linear(i, scene, look)).collect();
    let first = rgbs.first().ok_or_else(|| error("render: no images"))?;
    let (black, scale) = levels(first, scene.clear, look)?;
    Ok(rgbs
        .iter()
        .map(|r| display(r, scene, black, scale, look, 1.0))
        .collect())
}

/// Linear display light of `image` for SDR (0..=1) and for an HDR display (stretched sky up to
/// `headroom` times paper white; the same where the SDR rendering is not clipped).
pub fn hdr(
    image: &[[f32; 3]],
    scene: &Scene,
    look: &Look,
    headroom: f32,
) -> Result<[Vec<[f32; 3]>; 2]> {
    if !(scene.white.is_finite() && scene.white > 0.0) {
        return Err(error("render: the sensor white must be positive"));
    }
    let rgb = linear(image, scene, look);
    let (black, scale) = levels(&rgb, scene.clear, look)?;
    Ok([1.0, headroom].map(|top| {
        display(&rgb, scene, black, scale, look, top)
            .iter()
            .map(|c| c.map(decode))
            .collect()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bright_star_cores_keep_every_colour() -> Result<()> {
        let xyz_to_camera = crate::review::built_in_matrix("NIKON CORPORATION", "NIKON D700")?
            .ok_or_else(|| error("no D700 matrix"))?;
        let (white, flat) = (16383.0, 0.8);
        let scene = Scene {
            alpha: &[1.0],
            clear: &[true],
            glow: &[[0.0; 3]],
            flat: &[flat],
            wb: [1.890_625, 1.0, 2.148_437_5],
            matrix: camera_to_srgb(&xyz_to_camera)?,
            white,
        };
        // A clipped star core as the stack fills it: sensor white in every colour.
        let rgb = linear(&[[white / flat; 3]], &scene, &NATURAL)[0];
        let (lo, hi) = rgb
            .iter()
            .fold((f32::MAX, 0f32), |(l, h), &v| (l.min(v), h.max(v)));
        assert!(hi - lo < 1e-3 * hi, "{rgb:?}");
        // A bright, strongly coloured star: its weak colour stays visible after the boost.
        let shown = finish(&[[500.0, 6000.0, 9000.0]], [0.0; 3], 812.0, &NATURAL)[0];
        assert!(shown[0] > 0.5, "{shown:?}");
        Ok(())
    }
}
