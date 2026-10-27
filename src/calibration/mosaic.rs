use super::Capture;
use crate::{Result, error, pipeline::check_sensor};
use lcc_raw::{BlackLevel, Cfa, RawImage};

/// Active-area CFA samples in linear ADU with the pedestal retained (ADR 0006).
#[derive(Clone, Debug)]
pub struct Mosaic {
    pub width: usize,
    pub height: usize,
    /// Anchored at (0, 0) of this mosaic.
    pub cfa: Cfa,
    pub data: Vec<f32>,
    /// Metadata black, at active-area coordinates.
    pub black: BlackLevel,
    pub white: f32,
    pub capture: Capture,
}

impl Mosaic {
    pub fn from_raw(raw: &RawImage) -> Result<Mosaic> {
        if raw.cpp != 1 {
            return Err(error("calibration requires single-plane CFA data"));
        }
        check_sensor(raw)?;
        let cfa = raw
            .cfa
            .as_ref()
            .ok_or_else(|| error("calibration requires a CFA pattern"))?
            .shifted(raw.active_area.x, raw.active_area.y);
        let a = raw.active_area;
        let data = (a.y..a.y + a.height)
            .flat_map(|y| (a.x..a.x + a.width).map(move |x| raw.data.get(y * raw.width + x)))
            .collect();
        let mosaic = Mosaic {
            width: a.width,
            height: a.height,
            capture: Capture::from_raw(raw, &cfa),
            cfa,
            data,
            black: raw.black.clone(),
            white: raw.white_at(0),
        };
        mosaic.validate()?;
        Ok(mosaic)
    }

    pub fn validate(&self) -> Result<()> {
        let sites = self
            .width
            .checked_mul(self.height)
            .filter(|&n| n > 0)
            .ok_or_else(|| error("mosaic has no sites"))?;
        if self.data.len() != sites {
            return Err(error("mosaic sample count does not match its dimensions"));
        }
        if self.cfa.width == 0
            || self.cfa.height == 0
            || self.cfa.pattern.len() != self.cfa.width * self.cfa.height
            || self.cfa.pattern.iter().any(|&c| c > 2)
            || (0..3).any(|c| !self.cfa.pattern.contains(&c))
            || self.width < self.cfa.width
            || self.height < self.cfa.height
        {
            return Err(error("CFA pattern must hold red, green and blue indices"));
        }
        if self.data.iter().any(|v| !v.is_finite()) {
            return Err(error("mosaic contains non-finite samples"));
        }
        let black = &self.black;
        if !black.delta_h.is_empty() || !black.delta_v.is_empty() {
            return Err(error(
                "per-row/column black deltas are not reviewed for calibration",
            ));
        }
        if black.values.len() != black.repeat_rows.max(1) * black.repeat_cols.max(1) {
            return Err(error("black level pattern does not match its repeat size"));
        }
        if !self.white.is_finite()
            || black
                .values
                .iter()
                .any(|b| !b.is_finite() || *b >= self.white)
        {
            return Err(error("invalid black level or non-positive sensor range"));
        }
        if self.capture.size != [self.width, self.height] {
            return Err(error("capture size does not match mosaic dimensions"));
        }
        Ok(())
    }

    pub(crate) fn sites(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn color(&self, site: usize) -> usize {
        usize::from(self.cfa.color_at(site % self.width, site / self.width))
    }

    /// Stored value minus this frame's own black: per-frame pedestal drift is removed.
    pub(crate) fn above_black(&self, site: usize) -> f32 {
        self.data[site] - self.black.at(site % self.width, site / self.width, 0, 1)
    }

    /// White minus the maximum black of the plane (the DNG rescale denominator).
    pub(crate) fn range(&self) -> f32 {
        self.white - self.black.values.iter().copied().fold(f32::MIN, f32::max)
    }
}
