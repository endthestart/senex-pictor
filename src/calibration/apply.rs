use super::combine::Estimator;
use super::flat::{FlatField, FlatSummary};
use super::master::{Master, Role};
use super::matching::{Capture, Pairing, Warning, compare};
use super::mosaic::Mosaic;
use crate::{Result, error};
use lcc_raw::Cfa;
use serde::Serialize;

/// Calibrated CFA mosaic in ADU above the light's black: (light − black − master dark) / flat.
/// No per-frame rescale: decoder white levels can be inferred per frame, which would give
/// frames of one sequence different gains.
#[derive(Clone, Debug)]
pub struct Calibrated {
    pub width: usize,
    pub height: usize,
    pub cfa: Cfa,
    /// Unclipped; negative values preserved. Invalid sites hold NaN.
    pub data: Vec<f32>,
    pub valid: Vec<bool>,
    /// Stored light value at or above the white level.
    pub saturated: Vec<bool>,
    pub report: Report,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Report {
    pub pedestal: &'static str,
    /// The light's white level above its black: its saturation headroom as decoded.
    pub white_above_black_adu: f32,
    pub dark: Option<MasterSummary>,
    pub flat: Option<FlatReport>,
    pub uncorrected: Vec<&'static str>,
    pub saturated_sites: usize,
    pub invalid_sites: usize,
    pub warnings: Vec<Warning>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MasterSummary {
    pub frames: usize,
    pub estimator: Estimator,
    pub standard_error_by_color_adu: [f32; 3],
    pub capture: Capture,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FlatReport {
    pub frames: usize,
    pub estimator: Estimator,
    pub dark_role: Option<Role>,
    pub minimum: f32,
    pub summary: FlatSummary,
    pub capture: Capture,
}

fn merge(warnings: &mut Vec<Warning>, found: impl IntoIterator<Item = Warning>) {
    for warning in found {
        if !warnings.contains(&warning) {
            warnings.push(warning);
        }
    }
}

pub fn calibrate(
    light: &Mosaic,
    dark: Option<&Master>,
    flat: Option<&FlatField>,
) -> Result<Calibrated> {
    light.validate()?;
    let mut warnings = Vec::new();
    let mut uncorrected = Vec::new();
    if let Some(dark) = dark {
        if dark.role != Role::Dark {
            return Err(error(
                "lights are calibrated with a dark master, not bias or flat-dark",
            ));
        }
        if dark.cfa != light.cfa || dark.data.len() != light.sites() {
            return Err(error("dark master geometry differs from the light"));
        }
        merge(
            &mut warnings,
            compare(&dark.capture, &light.capture, Pairing::DarkForLight)?,
        );
        merge(&mut warnings, dark.warnings.iter().cloned());
    } else {
        uncorrected.push("dark signal and offset structure (only frame black subtracted)");
    }
    if let Some(flat) = flat {
        if flat.cfa != light.cfa || flat.data.len() != light.sites() {
            return Err(error("flat field geometry differs from the light"));
        }
        merge(
            &mut warnings,
            compare(&flat.capture, &light.capture, Pairing::FlatForLight)?,
        );
        merge(&mut warnings, flat.warnings.iter().cloned());
    } else {
        uncorrected.push("vignetting and pixel response (no flat field)");
    }
    let mut data = Vec::with_capacity(light.sites());
    let mut valid = Vec::with_capacity(light.sites());
    let mut saturated = Vec::with_capacity(light.sites());
    for (site, &value) in light.data.iter().enumerate() {
        let residual = dark.map_or(0.0, |d| d.data[site]);
        let response = flat.map_or(1.0, |f| f.data[site]);
        let usable = flat.is_none_or(|f| f.valid[site]);
        let calibrated = (light.above_black(site) - residual) / response;
        let usable = usable && calibrated.is_finite();
        data.push(if usable { calibrated } else { f32::NAN });
        valid.push(usable);
        saturated.push(value >= light.white);
    }
    let report = Report {
        pedestal: if dark.is_some() {
            "frame black and master dark"
        } else {
            "frame black"
        },
        white_above_black_adu: light.range(),
        dark: dark.map(|d| MasterSummary {
            frames: d.frames,
            estimator: d.estimator,
            standard_error_by_color_adu: d.standard_error_by_color,
            capture: d.capture.clone(),
        }),
        flat: flat.map(|f| FlatReport {
            frames: f.frames,
            estimator: f.estimator,
            dark_role: f.dark_role,
            minimum: f.minimum,
            summary: f.summary.clone(),
            capture: f.capture.clone(),
        }),
        uncorrected,
        saturated_sites: saturated.iter().filter(|s| **s).count(),
        invalid_sites: valid.iter().filter(|v| !**v).count(),
        warnings,
    };
    Ok(Calibrated {
        width: light.width,
        height: light.height,
        cfa: light.cfa.clone(),
        data,
        valid,
        saturated,
        report,
    })
}
