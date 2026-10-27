use super::combine::{Estimator, combine, median};
use super::matching::{Capture, Pairing, Warning, compare};
use super::mosaic::Mosaic;
use crate::{Result, error};
use lcc_raw::Cfa;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Role {
    Dark,
    Bias,
    FlatDark,
}

/// A combined calibration frame in ADU above each frame's own black: residual pedestal
/// structure and dark signal.
#[derive(Clone, Debug)]
pub struct Master {
    pub role: Role,
    pub capture: Capture,
    pub width: usize,
    pub height: usize,
    pub cfa: Cfa,
    pub frames: usize,
    pub estimator: Estimator,
    pub data: Vec<f32>,
    /// Frames accepted at each site.
    pub accepted: Vec<u16>,
    /// Standard error of each site's estimate, ADU.
    pub standard_error: Vec<f32>,
    /// Median per-site standard error by color (R, G, B), ADU.
    pub standard_error_by_color: [f32; 3],
    pub warnings: Vec<Warning>,
}

pub fn build_master(role: Role, frames: &[Mosaic], estimator: Estimator) -> Result<Master> {
    let first = frames
        .first()
        .ok_or_else(|| error("no calibration frames"))?;
    let pairing = match role {
        Role::Bias => Pairing::BiasFrames,
        Role::Dark | Role::FlatDark => Pairing::DarkFrames,
    };
    let warnings = check_members(frames, pairing)?;
    let combined = combine(frames.len(), first.sites(), estimator, |frame, site| {
        frames[frame].above_black(site)
    })?;
    Ok(Master {
        role,
        capture: first.capture.clone(),
        width: first.width,
        height: first.height,
        cfa: first.cfa.clone(),
        frames: frames.len(),
        estimator,
        standard_error_by_color: by_color(first, combined.standard_error.iter().copied())
            .map(|v| v as f32),
        data: combined.data,
        accepted: combined.accepted,
        standard_error: combined.standard_error,
        warnings,
    })
}

/// Validates every frame and compares it with the first; returns the distinct warnings.
pub(crate) fn check_members(frames: &[Mosaic], pairing: Pairing) -> Result<Vec<Warning>> {
    let first = frames
        .first()
        .ok_or_else(|| error("no calibration frames"))?;
    let mut warnings: Vec<Warning> = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        frame.validate()?;
        if frame.cfa != first.cfa {
            return Err(error(format!("frame {index}: CFA anchoring differs")));
        }
        let found = compare(&frame.capture, &first.capture, pairing)
            .map_err(|e| error(format!("frame {index}: {e}")))?;
        for warning in found {
            if !warnings.contains(&warning) {
                warnings.push(warning);
            }
        }
    }
    Ok(warnings)
}

/// Median of `values` over each color's sites; a validated mosaic has sites of every color.
pub(crate) fn by_color(mosaic: &Mosaic, values: impl Iterator<Item = f32>) -> [f64; 3] {
    let mut channels: [Vec<f64>; 3] = Default::default();
    for (site, value) in values.enumerate() {
        channels[mosaic.color(site)].push(f64::from(value));
    }
    let mut scratch = Vec::new();
    channels.map(|c| median(&c, &mut scratch))
}
