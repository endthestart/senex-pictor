//! Detections and tracks on disk: magic, frame count, track count, then per frame the
//! detection count and per detection x, y, flux (f64), saturated (u8) and track (u32).

use super::{Star, Tracks};
use crate::store::file;
use crate::{Result, error};
use std::path::Path;

const MAGIC: &[u8; 8] = b"SPSTARS1";

pub fn save(path: &Path, stars: &[Vec<Star>], tracks: &Tracks) -> Result<()> {
    if stars.len() != tracks.of.len() {
        return Err(error("stars and tracks differ in frame count"));
    }
    let mut out = MAGIC.to_vec();
    out.extend((stars.len() as u32).to_le_bytes());
    out.extend((tracks.count as u64).to_le_bytes());
    for (list, ids) in stars.iter().zip(&tracks.of) {
        out.extend((list.len() as u32).to_le_bytes());
        for (s, t) in list.iter().zip(ids) {
            out.extend(s.x.to_le_bytes());
            out.extend(s.y.to_le_bytes());
            out.extend(s.flux.to_le_bytes());
            out.push(u8::from(s.saturated));
            out.extend(t.to_le_bytes());
        }
    }
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    file::create(path, &out)
}

pub fn load(path: &Path) -> Result<(Vec<Vec<Star>>, Tracks)> {
    let bytes = std::fs::read(path)?;
    let mut at = 0;
    let mut take = |n: usize| -> Result<&[u8]> {
        let piece = bytes
            .get(at..at + n)
            .ok_or_else(|| error(format!("{} is truncated", path.display())))?;
        at += n;
        Ok(piece)
    };
    if take(8)? != MAGIC {
        return Err(error(format!("{} is not a star file", path.display())));
    }
    let u32_of = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let f64_of = |b: &[u8]| f64::from_le_bytes(std::array::from_fn(|i| b[i]));
    let frames = u32_of(take(4)?) as usize;
    let count = u64::from_le_bytes(std::array::from_fn(|i| bytes[12 + i]));
    take(8)?;
    let (mut stars, mut of) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
    for _ in 0..frames {
        let n = u32_of(take(4)?) as usize;
        let (mut list, mut ids) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for _ in 0..n {
            let r = take(29)?;
            list.push(Star {
                x: f64_of(&r[0..8]),
                y: f64_of(&r[8..16]),
                flux: f64_of(&r[16..24]),
                saturated: r[24] != 0,
            });
            let t = u32_of(&r[25..29]);
            if u64::from(t) >= count {
                return Err(error(format!("{}: track id out of range", path.display())));
            }
            ids.push(t);
        }
        stars.push(list);
        of.push(ids);
    }
    Ok((
        stars,
        Tracks {
            of,
            count: count as usize,
        },
    ))
}
