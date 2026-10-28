//! Room on disk for a run's store, and the frames between two names.

use crate::{Result, error, input};
use std::path::{Path, PathBuf};

/// Bytes a store keeps per frame pixel (RGB as 16-bit floats, CFA samples, saturation and
/// occlusion bits) and per pixel of the whole sequence (statistics, matte, stacks).
pub const STORE_BYTES_PER_FRAME_PIXEL: f64 = 8.4;
pub const STORE_BYTES_PER_PIXEL: f64 = 220.0;

/// Bytes the store of `frames` frames of `pixels` pixels needs.
pub fn store_bytes(pixels: usize, frames: usize) -> f64 {
    pixels as f64 * (frames as f64 * STORE_BYTES_PER_FRAME_PIXEL + STORE_BYTES_PER_PIXEL)
}

/// Bytes free on the volume that holds `path` (or would hold it, for a folder not made yet).
pub fn free_bytes(path: &Path) -> Result<f64> {
    let existing = path
        .ancestors()
        .find(|p| p.exists())
        .ok_or_else(|| error(format!("no existing folder above {}", path.display())))?
        .canonicalize()?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .filter(|d| existing.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .ok_or_else(|| error(format!("no volume found for {}", existing.display())))?;
    Ok(disk.available_space() as f64)
}

/// Fails unless the volume holding `out` has room for the store of `frames`.
pub fn room_for(out: &Path, frames: &[PathBuf]) -> Result<()> {
    let first = frames.first().ok_or_else(|| error("no frames selected"))?;
    let [w, h] = input::inspect(first)?.sensor;
    let need = store_bytes(w * h, frames.len());
    let free = free_bytes(out)?;
    if need > free {
        return Err(error(format!(
            "{} frames need about {:.0} GB in {} and its volume has {:.0} GB free; \
             choose fewer frames (--first, --last) or another output folder",
            frames.len(),
            need / 1e9,
            out.display(),
            free / 1e9
        )));
    }
    Ok(())
}

pub fn names_between(paths: &[PathBuf], first: &str, last: &str) -> Result<Vec<PathBuf>> {
    let index = |name: &str| {
        paths
            .iter()
            .position(|p| p.file_name().is_some_and(|f| f == name))
            .ok_or_else(|| error(format!("{name} is not in the folder")))
    };
    let (a, b) = (index(first)?, index(last)?);
    if a > b {
        return Err(error(format!("{first} comes after {last}")));
    }
    Ok(paths[a..=b].to_vec())
}
