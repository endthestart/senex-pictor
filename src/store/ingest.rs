//! Ingest: develop each RAW of a sequence into a new store.

use super::file;
use super::frames::{FrameInfo, Manifest, Store, frame_path, seconds};
use crate::budget::Budget;
use crate::calibration::Mosaic;
use crate::{Result, error, input, pipeline};
use lcc_raw::Rect;
use std::path::{Path, PathBuf};

/// Working set of one frame being developed, per sensor pixel: the RAW, its CFA samples,
/// the developed RGB and the encoded files.
const DEVELOP_BYTES_PER_PIXEL: u64 = 56;

/// Develops each RAW into a new store at `dir` (which must not exist). Frames must share
/// sensor geometry, crop, CFA layout and orientation; anything else fails.
pub fn ingest(paths: &[PathBuf], dir: &Path, budget: &Budget) -> Result<Store> {
    if dir.exists() {
        return Err(error(format!(
            "store directory already exists: {}",
            dir.display()
        )));
    }
    for kind in ["rgb", "cfa", "sat"] {
        std::fs::create_dir_all(dir.join(kind))?;
    }
    let first = paths.first().ok_or_else(|| error("no frames to ingest"))?;
    let [w, h] = input::inspect(first)?.sensor;
    let pool = budget.pool("ingest", 0, DEVELOP_BYTES_PER_PIXEL * (w * h) as u64)?;
    let results: Vec<Result<(FrameInfo, Manifest)>> = pool.install(|| {
        use rayon::prelude::*;
        paths
            .par_iter()
            .enumerate()
            .map(|(i, path)| frame(path, dir, i))
            .collect()
    });
    let mut manifest: Option<Manifest> = None;
    for result in results {
        let (info, shape) = result?;
        match &mut manifest {
            None => {
                manifest = Some(Manifest {
                    frames: vec![info],
                    ..shape
                })
            }
            Some(m)
                if Manifest {
                    frames: m.frames.clone(),
                    ..shape
                } == *m =>
            {
                m.frames.push(info)
            }
            Some(_) => {
                return Err(error(format!(
                    "{} differs in geometry, CFA or camera from the first frame",
                    info.name
                )));
            }
        }
    }
    let manifest = manifest.ok_or_else(|| error("no frames to ingest"))?;
    file::create(
        &dir.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(Store {
        dir: dir.to_path_buf(),
        manifest,
    })
}

fn frame(path: &Path, dir: &Path, index: usize) -> Result<(FrameInfo, Manifest)> {
    let (raw, identity) = input::decode(path)?;
    if !format!("{:?}", raw.orientation).eq_ignore_ascii_case("normal") {
        return Err(error(format!(
            "{}: rotated frames are not supported yet",
            identity.filename
        )));
    }
    let (image, _) = pipeline::develop(&raw)?;
    let mosaic = Mosaic::from_raw(&raw)?;
    let crop = raw.develop_crop(mosaic.width, mosaic.height);
    if (image.width, image.height) != (crop.width, crop.height) {
        return Err(error("developed image and CFA crop differ in size"));
    }
    let black_mean = raw.black.values.iter().sum::<f32>() / raw.black.values.len() as f32;
    let scale = raw.white_at(0) - black_mean;
    file::create(
        &frame_path(dir, "rgb", index),
        &file::f16_bytes(
            image
                .data
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .map(|v| v * scale),
        ),
    )?;
    let site = |x: usize, y: usize| mosaic.data[(crop.y + y) * mosaic.width + crop.x + x];
    let mut cfa = Vec::with_capacity(crop.width * crop.height);
    let mut saturated = Vec::with_capacity(crop.width * crop.height);
    for y in 0..crop.height {
        for x in 0..crop.width {
            let v = site(x, y);
            if v.fract() != 0.0 || !(0.0..=65535.0).contains(&v) {
                return Err(error(format!(
                    "{}: CFA samples are not 16-bit integers",
                    identity.filename
                )));
            }
            cfa.push(v as u16);
            saturated.push(v >= mosaic.white);
        }
    }
    file::create(
        &frame_path(dir, "cfa", index),
        &file::u16_bytes(cfa.into_iter()),
    )?;
    file::create(
        &frame_path(dir, "sat", index),
        &file::pack_bits(&saturated, crop.width),
    )?;
    let pattern = mosaic.cfa.shifted(crop.x, crop.y);
    let black = (0..pattern.height)
        .flat_map(|y| (0..pattern.width).map(move |x| (x, y)))
        .map(|(x, y)| mosaic.black.at(crop.x + x, crop.y + y, 0, 1))
        .collect();
    let m = &raw.metadata;
    let wb = raw
        .wb_multipliers
        .ok_or_else(|| error(format!("{}: no as-shot white balance", identity.filename)))?;
    let info = FrameInfo {
        name: identity.filename.clone(),
        sha256: identity.sha256,
        time_s: m
            .capture_time
            .as_ref()
            .map(seconds)
            .ok_or_else(|| error(format!("{}: no capture time", identity.filename)))?,
        exposure_s: m.exposure_time,
        iso: m.iso,
        aperture: m.f_number,
        focal_mm: m.focal_length,
        lens: m.lens_model.clone(),
        wb: [wb[0] / wb[1], 1.0, wb[2] / wb[1]],
        scale_adu: scale,
        black,
        white: mosaic.white,
        saturated_pixels: saturated.iter().filter(|&&s| s).count(),
    };
    let a = raw.active_area;
    let shape = Manifest {
        decoder: crate::DECODER_REVISION.into(),
        make: m.make.clone(),
        model: m.model.clone(),
        width: crop.width,
        height: crop.height,
        crop: Rect::new(a.x + crop.x, a.y + crop.y, crop.width, crop.height),
        cfa_width: pattern.width,
        cfa_height: pattern.height,
        cfa: pattern.pattern,
        frames: Vec::new(),
    };
    Ok((info, shape))
}
