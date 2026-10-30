//! A small linear thumbnail of each frame, made while the sequence check reads it, so the
//! app can show every frame of a folder before anything is processed: camera RGB above black,
//! scaled to the white level, as-shot white balance applied, box-averaged per colour.

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

/// The thumbnail of a crop's samples `above` black (`cw` × `ch`, row by row), whose colours
/// are `colours` per row of the CFA pattern (row `first_row` of the crop is pattern row
/// `first_row % colours.len()`), scaled by the white level's `range` above black, with white
/// balance `wb`.
pub fn make(
    above: &[f32],
    (cw, ch): (usize, usize),
    (colours, first_row): (&[Vec<u8>], usize),
    range: f64,
    wb: [f32; 3],
) -> Result<Thumb> {
    let (tw, th) = size(cw, ch);
    let mut sums = vec![[0f64; 3]; tw * th];
    let mut counts = vec![[0u32; 3]; tw * th];
    let cells: Vec<usize> = (0..cw).map(|x| x * tw / cw).collect();
    for y in 0..ch {
        let row = (y * th / ch) * tw;
        let pattern = &colours[(first_row + y) % colours.len()];
        let samples = &above[y * cw..(y + 1) * cw];
        for ((&v, &k), &cell) in samples.iter().zip(pattern).zip(&cells) {
            let (cell, k) = (row + cell, usize::from(k));
            sums[cell][k] += f64::from(v);
            counts[cell][k] += 1;
        }
    }
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
