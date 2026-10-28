//! A small linear thumbnail of each frame, made while the sequence check reads it, so the
//! app can show every frame of a folder before anything is processed: camera RGB above black,
//! scaled to the white level, as-shot white balance applied, box-averaged per colour.

use crate::calibration::Mosaic;
use crate::{Result, error};
use half::f16;

/// Longest side of a thumbnail, px.
pub const SIDE: usize = 192;

#[derive(Clone, Debug)]
pub struct Thumb {
    pub width: usize,
    pub height: usize,
    /// RGB as 16-bit float bits, row by row.
    pub rgb: Vec<u16>,
}

/// The thumbnail size for a crop of `w` × `h`.
pub fn size(w: usize, h: usize) -> (usize, usize) {
    let scale = SIDE as f64 / w.max(h) as f64;
    let side = |v: usize| ((v as f64 * scale).round() as usize).max(1);
    (side(w), side(h))
}

/// The thumbnail of `mosaic`'s `crop` (x, y, width, height) with white balance `wb`.
pub fn make(mosaic: &Mosaic, crop: (usize, usize, usize, usize), wb: [f32; 3]) -> Result<Thumb> {
    let (cx, cy, cw, ch) = crop;
    let (tw, th) = size(cw, ch);
    let mut sums = vec![[0f64; 3]; tw * th];
    let mut counts = vec![[0u32; 3]; tw * th];
    for y in 0..ch {
        let row = (y * th / ch) * tw;
        for x in 0..cw {
            let (ax, ay) = (cx + x, cy + y);
            let k = usize::from(mosaic.cfa.color_at(ax, ay));
            let black = mosaic.black.at(ax, ay, 0, 1);
            let cell = row + x * tw / cw;
            sums[cell][k] += f64::from(mosaic.data[ay * mosaic.width + ax] - black);
            counts[cell][k] += 1;
        }
    }
    let range = f64::from(mosaic.white) - f64::from(mosaic.black.at(cx, cy, 0, 1));
    if range <= 0.0 {
        return Err(error("white level at or below black"));
    }
    let mut rgb = Vec::with_capacity(3 * tw * th);
    for (s, n) in sums.iter().zip(&counts) {
        for k in 0..3 {
            if n[k] == 0 {
                return Err(error("a thumbnail cell has no samples of one colour"));
            }
            let v = s[k] / f64::from(n[k]) / range * f64::from(wb[k]);
            rgb.push(f16::from_f64(v).to_bits());
        }
    }
    Ok(Thumb {
        width: tw,
        height: th,
        rgb,
    })
}
