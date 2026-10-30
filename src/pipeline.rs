use crate::{Result, error};
use lcc_raw::{Method, RawImage, Rgb32f};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Development {
    pub stage: &'static str,
    pub demosaic: &'static str,
    pub width: usize,
    pub height: usize,
    pub orientation: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub sensor: [usize; 2],
    pub active_area: lcc_raw::Rect,
    pub default_crop: lcc_raw::Rect,
    pub black: lcc_raw::BlackLevel,
    pub white: Vec<f32>,
    pub wb_multipliers_not_applied: Option<[f32; 3]>,
    pub color_data_not_applied: lcc_raw::ColorData,
    pub deferred_post_demosaic_opcodes: Vec<lcc_raw::Opcode>,
    pub exposure_seconds: Option<f64>,
    pub iso: Option<u32>,
    pub minimum: [f32; 3],
    pub maximum: [f32; 3],
    pub limitations: Vec<&'static str>,
}

pub fn develop(raw: &RawImage) -> Result<(Rgb32f, Development)> {
    if raw.cpp != 1 && raw.cpp != 3 {
        return Err(error(
            "this diagnostic requires single-plane or three-channel RAW data",
        ));
    }
    check_sensor(raw)?;
    let normalized = raw.normalized()?;
    if normalized.data.iter().any(|v| !v.is_finite()) {
        return Err(error("normalization produced non-finite samples"));
    }
    let image = lcc_raw::demosaic(&normalized, Method::Ahd);
    drop(normalized);
    let crop = raw.develop_crop(image.width, image.height);
    let image = image
        .into_crop(crop.x, crop.y, crop.width, crop.height)
        .oriented(raw.orientation);
    let mut minimum = [f32::INFINITY; 3];
    let mut maximum = [f32::NEG_INFINITY; 3];
    if image.data.is_empty() {
        return Err(error("empty developed image"));
    }
    for pixel in &image.data {
        for c in 0..3 {
            if !pixel[c].is_finite() {
                return Err(error("non-finite demosaic output"));
            }
            minimum[c] = minimum[c].min(pixel[c]);
            maximum[c] = maximum[c].max(pixel[c]);
        }
    }
    let report = Development {
        stage: "scene-linear camera RGB diagnostic; not a public color-managed master",
        demosaic: "AHD (upstream implementation; stellar fidelity not yet validated)",
        width: image.width,
        height: image.height,
        orientation: "applied to pixels; TIFF top-left".into(),
        make: raw.metadata.make.clone(),
        model: raw.metadata.model.clone(),
        sensor: [raw.width, raw.height],
        active_area: raw.active_area,
        default_crop: raw.crop,
        black: raw.black.clone(),
        white: raw.white.clone(),
        wb_multipliers_not_applied: raw.wb_multipliers,
        color_data_not_applied: raw.color.clone(),
        deferred_post_demosaic_opcodes: raw.opcodes.list3.clone(),
        exposure_seconds: raw.metadata.exposure_time,
        iso: raw.metadata.iso,
        minimum,
        maximum,
        limitations: vec![
            "no dark/flat/bias calibration",
            "no white balance or color transform",
            "no denoising, sharpening, tone mapping or hot-pixel repair",
            "post-demosaic opcodes are recorded but not applied",
            "camera/compression validation incomplete; no GPU processing",
        ],
    };
    Ok((image, report))
}

/// Rejects sensor data whose levels, samples or pending sensor-stage processing are unreviewed.
/// Whether every active site's black (as `BlackLevel::at` sums it) is finite and below the white
/// level, with the black of each column worked out once per row of the black pattern.
fn blacks_below_white(raw: &RawImage) -> bool {
    let (b, a, cpp) = (&raw.black, raw.active_area, raw.cpp);
    let (r, c) = (b.repeat_rows.max(1), b.repeat_cols.max(1));
    (0..cpp).all(|s| {
        let white = raw.white_at(s);
        let columns: Vec<Vec<f32>> = (0..r)
            .map(|pr| {
                (0..a.width)
                    .map(|x| {
                        let base = b
                            .values
                            .get((pr * c + x % c) * cpp + s)
                            .or_else(|| b.values.first())
                            .copied()
                            .unwrap_or(0.0);
                        base + b.delta_h.get(x).copied().unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();
        (0..a.height).all(|y| {
            let dv = b.delta_v.get(y).copied().unwrap_or(0.0);
            columns[y % r].iter().all(|&column| {
                let black = column + dv;
                black.is_finite() && white > black
            })
        })
    })
}

pub(crate) fn check_sensor(raw: &RawImage) -> Result<()> {
    raw.validate()?;
    if raw.pixel_aspect != 1.0 {
        return Err(error(
            "non-square sensor pixels require unvalidated resampling",
        ));
    }
    if !raw.opcodes.list1.is_empty() || !raw.opcodes.list2.is_empty() {
        return Err(error(
            "sensor-stage RAW opcodes require review before scientific processing",
        ));
    }
    if let lcc_raw::RawData::F32(samples) = &raw.data
        && samples.iter().any(|v| !v.is_finite())
    {
        return Err(error("sensor contains non-finite samples"));
    }
    if raw.white.is_empty() || raw.white.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err(error("invalid or missing sensor white levels"));
    }
    if raw.black.values.is_empty() {
        return Err(error("missing sensor black levels"));
    }
    if !blacks_below_white(raw) {
        return Err(error("invalid black level or non-positive sensor range"));
    }
    Ok(())
}
