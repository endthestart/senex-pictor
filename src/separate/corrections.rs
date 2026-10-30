//! The owner's matte corrections: brush strokes painted as sky or ground, kept in the store
//! as `corrections.json` (store pixels) and laid over the matte wherever it is drawn, later
//! strokes over earlier ones.

use crate::store::Store;
use crate::{Result, error};
use serde::{Deserialize, Serialize};

pub const FILE: &str = "corrections.json";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    /// Paints sky (true) or ground.
    pub sky: bool,
    pub radius: f32,
    /// The brush's path in store pixels; one point is a dab.
    pub points: Vec<[f32; 2]>,
}

/// The store's strokes, none when it has no corrections.
pub fn load(store: &Store) -> Result<Vec<Stroke>> {
    let path = store.dir.join(FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&std::fs::read(&path)?)
        .map_err(|e| error(format!("{}: {e}", path.display())))
}

/// Writes `strokes` (removing the file when there are none).
pub fn save(store: &Store, strokes: &[Stroke]) -> Result<()> {
    let path = store.dir.join(FILE);
    if strokes.is_empty() {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        return Ok(());
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(strokes)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

/// Distance from `p` to the segment a–b.
fn to_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length2 = dx * dx + dy * dy;
    let t = if length2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

/// Per pixel 1 for painted sky, -1 for painted ground, 0 untouched.
pub fn force(strokes: &[Stroke], w: usize, h: usize) -> Vec<i8> {
    let mut out = vec![0i8; w * h];
    for s in strokes {
        let value = if s.sky { 1 } else { -1 };
        let segments: Vec<([f32; 2], [f32; 2])> = match s.points.as_slice() {
            [one] => vec![(*one, *one)],
            many => many.windows(2).map(|p| (p[0], p[1])).collect(),
        };
        for (a, b) in segments {
            let x0 = (a[0].min(b[0]) - s.radius).floor().max(0.0) as usize;
            let y0 = (a[1].min(b[1]) - s.radius).floor().max(0.0) as usize;
            let x1 = ((a[0].max(b[0]) + s.radius).ceil().max(0.0) as usize).min(w);
            let y1 = ((a[1].max(b[1]) + s.radius).ceil().max(0.0) as usize).min(h);
            for y in y0..y1 {
                for x in x0..x1 {
                    if to_segment([x as f32, y as f32], a, b) <= s.radius {
                        out[y * w + x] = value;
                    }
                }
            }
        }
    }
    out
}

/// `alpha` with the painted pixels set to sky (1) or ground (0); how many were painted.
pub fn apply(alpha: &mut [f32], force: &[i8]) -> usize {
    let mut painted = 0;
    for (a, &f) in alpha.iter_mut().zip(force) {
        if f != 0 {
            *a = if f > 0 { 1.0 } else { 0.0 };
            painted += 1;
        }
    }
    painted
}

/// The store's matte `alpha` with its corrections laid over; how many pixels were painted.
pub fn correct(store: &Store, alpha: &mut [f32]) -> Result<usize> {
    let strokes = load(store)?;
    if strokes.is_empty() {
        return Ok(0);
    }
    let m = &store.manifest;
    Ok(apply(alpha, &force(&strokes, m.width, m.height)))
}
