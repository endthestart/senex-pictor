use super::combine::{Estimator, combine};
use super::master::{Master, Role, by_color, check_members};
use super::matching::{Capture, Pairing, Warning, compare};
use super::mosaic::Mosaic;
use crate::{Result, error};
use lcc_raw::Cfa;
use serde::Serialize;

/// Relative pixel response: each color's median is 1 (ADR 0006).
#[derive(Clone, Debug)]
pub struct FlatField {
    pub capture: Capture,
    pub width: usize,
    pub height: usize,
    pub cfa: Cfa,
    pub frames: usize,
    pub estimator: Estimator,
    /// Flat-dark or bias master subtracted, if any.
    pub dark_role: Option<Role>,
    /// Sites whose response is below this, non-finite, or saturated in any flat are invalid.
    pub minimum: f32,
    pub data: Vec<f32>,
    pub valid: Vec<bool>,
    /// Standard error of each site's response, in response units.
    pub standard_error: Vec<f32>,
    pub summary: FlatSummary,
    pub warnings: Vec<Warning>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FlatSummary {
    /// Per-frame median of (flat − black − flat-dark) by color (R, G, B), ADU.
    pub frame_levels: Vec<[f64; 3]>,
    pub standard_error_by_color: [f32; 3],
    pub invalid_sites: usize,
}

pub fn build_flat(
    frames: &[Mosaic],
    dark: Option<&Master>,
    estimator: Estimator,
    minimum: f32,
) -> Result<FlatField> {
    if !(minimum > 0.0 && minimum < 1.0) {
        return Err(error("flat minimum must lie between 0 and 1"));
    }
    let first = frames.first().ok_or_else(|| error("no flat frames"))?;
    let mut warnings = check_members(frames, Pairing::FlatFrames)?;
    if let Some(dark) = dark {
        let pairing = match dark.role {
            Role::FlatDark => Pairing::FlatDarkForFlat,
            Role::Bias => Pairing::BiasForFlat,
            Role::Dark => {
                return Err(error(
                    "flats need a flat-dark or bias master, not a light dark",
                ));
            }
        };
        if dark.cfa != first.cfa || dark.data.len() != first.sites() {
            return Err(error("flat-dark geometry differs from the flats"));
        }
        for (index, frame) in frames.iter().enumerate() {
            for warning in compare(&dark.capture, &frame.capture, pairing)
                .map_err(|e| error(format!("flat {index}: {e}")))?
            {
                if !warnings.contains(&warning) {
                    warnings.push(warning);
                }
            }
        }
    } else {
        warnings.push(Warning {
            subject: "flat dark",
            detail:
                "no flat-dark or bias master; flat dark signal and offset structure uncorrected"
                    .into(),
        });
    }
    let flat_dark = |site: usize| dark.map_or(0.0, |d| d.data[site]);
    let mut frame_levels = Vec::with_capacity(frames.len());
    for (index, frame) in frames.iter().enumerate() {
        let levels = by_color(
            frame,
            (0..frame.sites()).map(|s| frame.above_black(s) - flat_dark(s)),
        );
        if levels.iter().any(|l| !l.is_finite() || *l <= 0.0) {
            return Err(error(format!(
                "flat {index} has no positive signal in a color"
            )));
        }
        frame_levels.push(levels);
    }
    let inverse: Vec<[f32; 3]> = frame_levels
        .iter()
        .map(|l| l.map(|v| (1.0 / v) as f32))
        .collect();
    let combined = combine(frames.len(), first.sites(), estimator, |frame, site| {
        let color = first.color(site);
        (frames[frame].above_black(site) - flat_dark(site)) * inverse[frame][color]
    })?;
    let levels = by_color(first, combined.data.iter().copied());
    if levels.iter().any(|l| !l.is_finite() || *l <= 0.0) {
        return Err(error("combined flat has no positive response in a color"));
    }
    let scale = |site: usize| (1.0 / levels[first.color(site)]) as f32;
    let data: Vec<f32> = combined
        .data
        .iter()
        .enumerate()
        .map(|(s, v)| v * scale(s))
        .collect();
    let standard_error: Vec<f32> = combined
        .standard_error
        .iter()
        .enumerate()
        .map(|(s, v)| v * scale(s))
        .collect();
    let valid: Vec<bool> = data
        .iter()
        .enumerate()
        .map(|(s, v)| v.is_finite() && *v >= minimum && frames.iter().all(|f| f.data[s] < f.white))
        .collect();
    let summary = FlatSummary {
        frame_levels,
        standard_error_by_color: by_color(first, standard_error.iter().copied()).map(|v| v as f32),
        invalid_sites: valid.iter().filter(|v| !**v).count(),
    };
    Ok(FlatField {
        capture: first.capture.clone(),
        width: first.width,
        height: first.height,
        cfa: first.cfa.clone(),
        frames: frames.len(),
        estimator,
        dark_role: dark.map(|d| d.role),
        minimum,
        data,
        valid,
        standard_error,
        summary,
        warnings,
    })
}
