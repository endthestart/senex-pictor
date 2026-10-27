use crate::{Result, error};
use lcc_raw::RawImage;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct Identity {
    pub filename: String,
    pub bytes: usize,
    pub sha256: String,
}

pub fn read(path: &Path) -> Result<(Vec<u8>, Identity)> {
    let data = std::fs::read(path)?;
    let identity = Identity {
        filename: path
            .file_name()
            .ok_or_else(|| error("input has no filename"))?
            .to_string_lossy()
            .into_owned(),
        bytes: data.len(),
        sha256: format!("{:x}", Sha256::digest(&data)),
    };
    Ok((data, identity))
}

#[derive(Debug, Serialize)]
pub struct Inspection {
    pub source: Identity,
    pub decoder: &'static str,
    pub format: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub sensor: [usize; 2],
    pub active_area: lcc_raw::Rect,
    pub default_crop: lcc_raw::Rect,
    pub developed_unoriented: [usize; 2],
    pub orientation: String,
    pub bits: u32,
    pub exposure_seconds: Option<f64>,
    pub iso: Option<u32>,
    pub aperture: Option<f64>,
    pub focal_length_mm: Option<f64>,
    pub wb_multipliers: Option<[f32; 3]>,
    pub embedded_color_matrix: bool,
    pub opcodes: lcc_raw::OpcodeLists,
}

pub fn inspect(path: &Path) -> Result<Inspection> {
    let (bytes, identity) = read(path)?;
    let info = lcc_raw::probe_info(&bytes)?;
    let size = info.developed_size();
    Ok(Inspection {
        source: identity,
        decoder: crate::DECODER_REVISION,
        format: format!("{:?}", info.format),
        make: info.metadata.make,
        model: info.metadata.model,
        sensor: [info.width, info.height],
        active_area: info.active_area,
        default_crop: info.crop,
        developed_unoriented: [size.0, size.1],
        orientation: format!("{:?}", info.orientation),
        bits: info.bits,
        exposure_seconds: info.metadata.exposure_time,
        iso: info.metadata.iso,
        aperture: info.metadata.f_number,
        focal_length_mm: info.metadata.focal_length,
        wb_multipliers: info.wb_multipliers,
        embedded_color_matrix: lcc_raw::color::has_matrix(&info.color),
        opcodes: info.opcodes,
    })
}

pub fn decode(path: &Path) -> Result<(RawImage, Identity)> {
    let (bytes, identity) = read(path)?;
    Ok((lcc_raw::decode(&bytes)?, identity))
}

/// Decoded sensor levels and geometry, for comparison against camera databases.
#[derive(Debug, Serialize)]
pub struct Levels {
    pub source: Identity,
    pub decoder: &'static str,
    pub make: Option<String>,
    pub model: Option<String>,
    pub iso: Option<u32>,
    pub exposure_seconds: Option<f64>,
    pub bits: u32,
    pub sensor: [usize; 2],
    pub active_area: lcc_raw::Rect,
    pub default_crop: lcc_raw::Rect,
    pub cfa: Option<String>,
    pub black: lcc_raw::BlackLevel,
    pub white: Vec<f32>,
    /// Largest stored sample inside the active area.
    pub active_maximum: f32,
}

pub fn levels(path: &Path) -> Result<Levels> {
    let (raw, source) = decode(path)?;
    let a = raw.active_area;
    let active_maximum = (a.y..a.y + a.height)
        .flat_map(|y| (a.x..a.x + a.width).map(move |x| (y * raw.width + x) * raw.cpp))
        .flat_map(|i| (0..raw.cpp).map(move |s| i + s))
        .map(|i| raw.data.get(i))
        .fold(f32::NEG_INFINITY, f32::max);
    Ok(Levels {
        source,
        decoder: crate::DECODER_REVISION,
        make: raw.metadata.make.clone(),
        model: raw.metadata.model.clone(),
        iso: raw.metadata.iso,
        exposure_seconds: raw.metadata.exposure_time,
        bits: raw.bits,
        sensor: [raw.width, raw.height],
        active_area: raw.active_area,
        default_crop: raw.crop,
        cfa: raw.cfa.as_ref().map(lcc_raw::Cfa::name),
        black: raw.black.clone(),
        white: raw.white.clone(),
        active_maximum,
    })
}

/// RAW files in `dir`, sorted by name (hidden files skipped).
pub fn raw_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    const EXTENSIONS: &[&str] = &[
        "nef", "nrw", "dng", "cr2", "cr3", "arw", "raf", "orf", "rw2", "pef",
    ];
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .filter(|p| {
            p.extension()
                .is_some_and(|x| EXTENSIONS.contains(&x.to_string_lossy().to_lowercase().as_str()))
        })
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(error(format!("no RAW files in {}", dir.display())));
    }
    Ok(paths)
}
