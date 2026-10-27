//! A sequence on disk: per frame the developed camera RGB in ADU above black (f16), the
//! default-crop CFA samples as stored (u16, black included) and a packed saturation mask,
//! with one JSON manifest. Pixel (x, y) of every file is the same sensor site.

use super::file::{self, read_at};
use crate::{Result, error};
use lcc_raw::Rect;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameInfo {
    pub name: String,
    pub sha256: String,
    /// Seconds on the camera clock (only differences are meaningful).
    pub time_s: f64,
    pub exposure_s: Option<f64>,
    pub iso: Option<u32>,
    pub aperture: Option<f64>,
    pub focal_mm: Option<f64>,
    pub lens: Option<String>,
    /// As-shot white balance, green = 1.
    pub wb: [f32; 3],
    /// ADU per developed unit: white minus mean black.
    pub scale_adu: f32,
    /// Black per site of the CFA repeat at the crop origin, row-major.
    pub black: Vec<f32>,
    pub white: f32,
    pub saturated_pixels: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub decoder: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub width: usize,
    pub height: usize,
    /// Crop in full sensor coordinates (active-area origin plus default crop).
    pub crop: Rect,
    /// CFA repeat anchored at the crop origin: `cfa_width × cfa_height` colour indices.
    pub cfa_width: usize,
    pub cfa_height: usize,
    pub cfa: Vec<u8>,
    pub frames: Vec<FrameInfo>,
}

pub struct Store {
    pub dir: PathBuf,
    pub manifest: Manifest,
}

pub(super) fn frame_path(dir: &Path, kind: &str, index: usize) -> PathBuf {
    dir.join(kind).join(format!("{index:05}.{kind}"))
}

impl Store {
    pub fn open(dir: &Path) -> Result<Store> {
        let manifest = serde_json::from_slice(&std::fs::read(dir.join("manifest.json"))?)?;
        Ok(Store {
            dir: dir.to_path_buf(),
            manifest,
        })
    }

    fn rows(
        &self,
        kind: &str,
        index: usize,
        y0: usize,
        y1: usize,
        row_bytes: usize,
    ) -> Result<Vec<u8>> {
        let m = &self.manifest;
        if index >= m.frames.len() || y0 > y1 || y1 > m.height {
            return Err(error(format!(
                "rows {y0}..{y1} of frame {index} are outside the store"
            )));
        }
        let mut buf = vec![0u8; (y1 - y0) * row_bytes];
        read_at(
            &File::open(frame_path(&self.dir, kind, index))?,
            (y0 * row_bytes) as u64,
            &mut buf,
        )?;
        Ok(buf)
    }

    /// Developed camera RGB of rows `y0..y1`, interleaved, in ADU above black.
    pub fn rgb_rows(&self, index: usize, y0: usize, y1: usize) -> Result<Vec<f32>> {
        Ok(file::f16_values(&self.rows(
            "rgb",
            index,
            y0,
            y1,
            self.manifest.width * 6,
        )?))
    }

    /// R + G + B of the whole frame, the image stars are detected on.
    pub fn luminance(&self, index: usize) -> Result<Vec<f32>> {
        Ok(self
            .rgb_rows(index, 0, self.manifest.height)?
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| p[0] + p[1] + p[2])
            .collect())
    }

    /// CFA samples of rows `y0..y1` as stored (black included).
    pub fn cfa_rows(&self, index: usize, y0: usize, y1: usize) -> Result<Vec<u16>> {
        Ok(file::u16_values(&self.rows(
            "cfa",
            index,
            y0,
            y1,
            self.manifest.width * 2,
        )?))
    }

    /// Saturated sites of rows `y0..y1`.
    pub fn saturated_rows(&self, index: usize, y0: usize, y1: usize) -> Result<Vec<bool>> {
        self.bit_rows("sat", index, y0, y1)
    }

    /// Sky pixels occluded in frame `index` (written by separation) for rows `y0..y1`.
    pub fn occluded_rows(&self, index: usize, y0: usize, y1: usize) -> Result<Vec<bool>> {
        self.bit_rows("occ", index, y0, y1)
    }

    /// Rows `y0..y1` of the bit masks `kind/{index}.kind`, packed as stored.
    pub fn packed_rows(&self, kind: &str, index: usize, y0: usize, y1: usize) -> Result<Vec<u8>> {
        self.rows(kind, index, y0, y1, file::bit_row(self.manifest.width))
    }

    /// Rows `y0..y1` of the bit masks `kind/{index}.kind` (sat, occ, hero-occ).
    pub fn bit_rows(&self, kind: &str, index: usize, y0: usize, y1: usize) -> Result<Vec<bool>> {
        let width = self.manifest.width;
        Ok(file::unpack_bits(
            &self.rows(kind, index, y0, y1, file::bit_row(width))?,
            width,
        ))
    }
}

/// Seconds since 1970-01-01 on the camera's clock (time zone ignored).
pub fn seconds(t: &lcc_meta::DateTime) -> f64 {
    let (y, m) = if t.month <= 2 {
        (i64::from(t.year) - 1, i64::from(t.month) + 9)
    } else {
        (i64::from(t.year), i64::from(t.month) - 3)
    };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + i64::from(t.day) - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    (days * 86_400 + i64::from(t.hour) * 3600 + i64::from(t.minute) * 60 + i64::from(t.second))
        as f64
        + f64::from(t.millis) / 1000.0
}
