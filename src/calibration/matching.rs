use crate::{Result, error};
use lcc_raw::{Cfa, RawImage};
use serde::Serialize;

/// Capture properties compared before frames are combined or applied (ADR 0006).
/// Vendor fields are not exposed by the decoder; callers fill them from maker notes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Capture {
    pub make: Option<String>,
    pub model: Option<String>,
    pub size: [usize; 2],
    pub cfa: String,
    pub bits: u32,
    pub black: Vec<f32>,
    pub white: f32,
    pub iso: Option<u32>,
    pub exposure_seconds: Option<f64>,
    pub lens_model: Option<String>,
    pub focal_length_mm: Option<f64>,
    pub f_number: Option<f64>,
    pub orientation: String,
    pub shutter_mode: Option<String>,
    pub compression: Option<String>,
    pub in_camera_nr: Option<String>,
    /// Sensor or camera temperature, degrees Celsius, where the maker notes record it.
    pub temperature_c: Option<f64>,
}

impl Capture {
    pub(crate) fn from_raw(raw: &RawImage, cfa: &Cfa) -> Capture {
        let m = &raw.metadata;
        Capture {
            make: m.make.clone(),
            model: m.model.clone(),
            size: [raw.active_area.width, raw.active_area.height],
            cfa: cfa.name(),
            bits: raw.bits,
            black: raw.black.values.clone(),
            white: raw.white_at(0),
            iso: m.iso,
            exposure_seconds: m.exposure_time,
            lens_model: m.lens_model.clone(),
            focal_length_mm: m.focal_length,
            f_number: m.f_number,
            orientation: format!("{:?}", raw.orientation),
            shutter_mode: None,
            compression: None,
            in_camera_nr: None,
            temperature_c: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Warning {
    pub subject: &'static str,
    pub detail: String,
}

/// How a frame or master is used relative to the capture it is compared with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pairing {
    DarkFrames,
    BiasFrames,
    FlatFrames,
    DarkForLight,
    FlatDarkForFlat,
    BiasForFlat,
    FlatForLight,
}

#[derive(Clone, Copy)]
enum Field {
    Iso,
    Exposure,
    InCameraNr,
    Lens,
    FocalLength,
    Aperture,
}

fn required(pairing: Pairing) -> &'static [Field] {
    use Field::*;
    match pairing {
        Pairing::DarkFrames | Pairing::DarkForLight | Pairing::FlatDarkForFlat => {
            &[Iso, Exposure, InCameraNr]
        }
        Pairing::BiasFrames => &[Iso, Exposure],
        Pairing::BiasForFlat => &[Iso],
        Pairing::FlatFrames => &[Iso, Lens, FocalLength, Aperture],
        Pairing::FlatForLight => &[Lens, FocalLength, Aperture],
    }
}

fn same_number(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs())
}

fn require<T: PartialEq + std::fmt::Debug>(
    name: &str,
    a: &Option<T>,
    b: &Option<T>,
    equal: impl Fn(&T, &T) -> bool,
) -> Result<()> {
    match (a, b) {
        (Some(x), Some(y)) if equal(x, y) => Ok(()),
        (Some(x), Some(y)) => Err(error(format!("{name} differs: {x:?} vs {y:?}"))),
        _ => Err(error(format!(
            "{name} is not recorded; the match cannot be verified"
        ))),
    }
}

/// Manual lenses and telescopes record no optics, so absence on both captures is disclosed;
/// absence on one side or differing values reject.
fn optic<T: PartialEq + std::fmt::Debug>(
    name: &'static str,
    a: &Option<T>,
    b: &Option<T>,
    equal: impl Fn(&T, &T) -> bool,
    warnings: &mut Vec<Warning>,
) -> Result<()> {
    if a.is_none() && b.is_none() {
        warnings.push(Warning {
            subject: name,
            detail: "not recorded on either capture; confirm the same optics".into(),
        });
        return Ok(());
    }
    require(name, a, b, equal)
}

/// Largest accepted difference between mean black levels, as a fraction of the sensor range.
/// Each frame's own black is subtracted before combination, so per-frame drift cancels; this
/// only guards against wrong levels, such as a decoder falling back to zero.
const BLACK_TOLERANCE: f32 = 0.01;

fn mean_black(capture: &Capture) -> f32 {
    capture.black.iter().sum::<f32>() / capture.black.len() as f32
}

/// Rejects incompatible captures and returns warnings for unverified or differing properties.
pub(crate) fn compare(
    subject: &Capture,
    other: &Capture,
    pairing: Pairing,
) -> Result<Vec<Warning>> {
    let structural = subject.size == other.size
        && subject.cfa == other.cfa
        && subject.bits == other.bits
        && subject.black.len() == other.black.len();
    if !structural {
        return Err(error("sensor size, CFA, bit depth or black pattern differ"));
    }
    let (black, other_black) = (mean_black(subject), mean_black(other));
    if (black - other_black).abs() > BLACK_TOLERANCE * (subject.white - black) {
        return Err(error(format!(
            "black levels differ beyond tolerance: {black} vs {other_black}"
        )));
    }
    require("camera make", &subject.make, &other.make, |a, b| a == b)?;
    require("camera model", &subject.model, &other.model, |a, b| a == b)?;
    let mut warnings = Vec::new();
    for field in required(pairing) {
        match field {
            Field::Iso => require("ISO", &subject.iso, &other.iso, |a, b| a == b)?,
            Field::Exposure => require(
                "exposure time",
                &subject.exposure_seconds,
                &other.exposure_seconds,
                |a, b| same_number(*a, *b),
            )?,
            Field::InCameraNr => require(
                "in-camera noise reduction",
                &subject.in_camera_nr,
                &other.in_camera_nr,
                |a, b| a == b,
            )?,
            Field::Lens => optic(
                "lens",
                &subject.lens_model,
                &other.lens_model,
                |a, b| a == b,
                &mut warnings,
            )?,
            Field::FocalLength => optic(
                "focal length",
                &subject.focal_length_mm,
                &other.focal_length_mm,
                |a, b| same_number(*a, *b),
                &mut warnings,
            )?,
            Field::Aperture => optic(
                "aperture",
                &subject.f_number,
                &other.f_number,
                |a, b| same_number(*a, *b),
                &mut warnings,
            )?,
        }
    }
    let mut note = |subject: &'static str, a: &Option<String>, b: &Option<String>| match (a, b) {
        (Some(x), Some(y)) if x == y => {}
        (Some(x), Some(y)) => warnings.push(Warning {
            subject,
            detail: format!("{x} vs {y}"),
        }),
        _ => warnings.push(Warning {
            subject,
            detail: "not recorded".into(),
        }),
    };
    note("shutter mode", &subject.shutter_mode, &other.shutter_mode);
    note("compression", &subject.compression, &other.compression);
    let dark_pairing = matches!(
        pairing,
        Pairing::DarkFrames | Pairing::DarkForLight | Pairing::FlatDarkForFlat
    );
    if dark_pairing {
        match (subject.temperature_c, other.temperature_c) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => warnings.push(Warning {
                subject: "temperature",
                detail: format!("{a} °C vs {b} °C; dark signal depends on temperature"),
            }),
            _ => warnings.push(Warning {
                subject: "temperature",
                detail: "not recorded".into(),
            }),
        }
    }
    if pairing == Pairing::FlatForLight {
        if subject.iso != other.iso {
            warnings.push(Warning {
                subject: "flat ISO",
                detail: format!("{:?} vs {:?}", subject.iso, other.iso),
            });
        }
        if subject.orientation != other.orientation {
            warnings.push(Warning {
                subject: "orientation",
                detail: format!("{} vs {}", subject.orientation, other.orientation),
            });
        }
        warnings.push(Warning {
            subject: "focus",
            detail: "not recorded; flats must use the lights' focus".into(),
        });
    }
    if pairing == Pairing::BiasForFlat {
        warnings.push(Warning {
            subject: "flat dark signal",
            detail: "bias substitutes for a flat-dark; flat dark signal is uncorrected".into(),
        });
    }
    Ok(warnings)
}
