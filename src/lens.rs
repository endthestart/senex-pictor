//! Lens vignetting from the lensfun database (data CC-BY-SA 3.0), read in place.
//!
//! lensfun's "pa" model: V(r) = 1 + k1 r² + k2 r⁴ + k3 r⁶, with r measured from the image
//! centre and 1 at the corner. A flat field divides each sample by V at its sensor position.
//! Profiles are the starting point until vignetting is measured from the stars themselves.

use crate::{Result, error};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Where lensfun's XML database is looked for when `STAR_PHOTOS_LENSFUN` is unset.
const KNOWN_DATABASES: &[&str] = &[
    "/Applications/darktable.app/Contents/Resources/share/lensfun/version_1",
    "/opt/homebrew/share/lensfun/version_1",
    "/usr/local/share/lensfun/version_1",
    "/usr/share/lensfun/version_1",
];

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Vignetting {
    pub lens: String,
    pub focal_mm: f64,
    pub aperture: f64,
    pub k: [f64; 3],
    pub database: PathBuf,
}

/// The lensfun database directory: `STAR_PHOTOS_LENSFUN`, else the first known location.
pub fn database() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("STAR_PHOTOS_LENSFUN") {
        let dir = PathBuf::from(dir);
        return if dir.is_dir() {
            Ok(dir)
        } else {
            Err(error(format!(
                "STAR_PHOTOS_LENSFUN is not a directory: {}",
                dir.display()
            )))
        };
    }
    KNOWN_DATABASES
        .iter()
        .map(PathBuf::from)
        .find(|dir| dir.is_dir())
        .ok_or_else(|| {
            error(format!(
                "no lensfun database found; set STAR_PHOTOS_LENSFUN (looked in {})",
                KNOWN_DATABASES.join(", ")
            ))
        })
}

/// Lens names compare without case, repeated spaces or a leading maker word that one side
/// carries and the other omits ("Nikon AF Nikkor 24mm f/2.8D" against "AF Nikkor 24mm f/2.8D").
pub fn same_lens(a: &str, b: &str, maker: &str) -> bool {
    let norm = |s: &str| {
        let words: Vec<String> = s.split_whitespace().map(str::to_lowercase).collect();
        let maker = maker.to_lowercase();
        let start = usize::from(words.len() > 1 && words[0] == maker);
        words[start..].join(" ")
    };
    norm(a) == norm(b)
}

/// The calibrated "pa" profile for `lens` at `focal_mm` (within 0.5 mm) whose aperture is
/// nearest to `aperture` in stops. Fails when the database has no such entry.
pub fn find(
    database: &Path,
    maker: &str,
    lens: &str,
    focal_mm: f64,
    aperture: f64,
) -> Result<Vignetting> {
    let mut best: Option<(f64, Vignetting)> = None;
    let mut files: Vec<PathBuf> = std::fs::read_dir(database)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "xml"))
        .collect();
    files.sort();
    for file in files {
        for entry in vignetting_entries(&std::fs::read_to_string(&file)?)? {
            if !entry.models.iter().any(|m| same_lens(m, lens, maker))
                || (entry.focal_mm - focal_mm).abs() > 0.5
            {
                continue;
            }
            let stops = (entry.aperture / aperture).log2().abs();
            if best.as_ref().is_none_or(|(s, _)| stops < *s) {
                let found = Vignetting {
                    lens: lens.to_string(),
                    focal_mm: entry.focal_mm,
                    aperture: entry.aperture,
                    k: entry.k,
                    database: database.to_path_buf(),
                };
                best = Some((stops, found));
            }
        }
    }
    best.map(|(_, v)| v).ok_or_else(|| {
        error(format!(
            "lensfun has no vignetting profile for {lens} at {focal_mm} mm"
        ))
    })
}

/// V(x, y) for an image of `width × height` pixels, row-major, 1 at the centre.
pub fn field(width: usize, height: usize, k: [f64; 3]) -> Vec<f32> {
    let (cx, cy) = ((width as f64 - 1.0) / 2.0, (height as f64 - 1.0) / 2.0);
    let corner = cx * cx + cy * cy;
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let r2 = ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)) / corner;
            (1.0 + k[0] * r2 + k[1] * r2 * r2 + k[2] * r2 * r2 * r2) as f32
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub models: Vec<String>,
    pub focal_mm: f64,
    pub aperture: f64,
    pub k: [f64; 3],
}

/// Every "pa" vignetting entry of one lensfun XML file, with its lens's model names.
pub fn vignetting_entries(xml: &str) -> Result<Vec<Entry>> {
    let mut reader = Reader::from_str(xml);
    let (mut models, mut entries) = (Vec::new(), Vec::new());
    let mut in_model = false;
    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == b"lens" => models.clear(),
            Event::Start(e) if e.name().as_ref() == b"model" => in_model = true,
            Event::End(e) if e.name().as_ref() == b"model" => in_model = false,
            Event::Text(t) if in_model => models.push(t.decode()?.trim().to_string()),
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"vignetting" => {
                if attribute(&e, "model")?.as_deref() == Some("pa") {
                    let number = |name: &str| -> Result<f64> {
                        attribute(&e, name)?
                            .ok_or_else(|| {
                                error(format!("lensfun vignetting entry without {name}"))
                            })?
                            .parse::<f64>()
                            .map_err(|_| {
                                error(format!("lensfun vignetting {name} is not a number"))
                            })
                    };
                    entries.push(Entry {
                        models: models.clone(),
                        focal_mm: number("focal")?,
                        aperture: number("aperture")?,
                        k: [number("k1")?, number("k2")?, number("k3")?],
                    });
                }
            }
            Event::Eof => return Ok(entries),
            _ => {}
        }
    }
}

fn attribute(e: &BytesStart, name: &str) -> Result<Option<String>> {
    for a in e.attributes() {
        let a = a?;
        if a.key.as_ref() == name.as_bytes() {
            return Ok(Some(
                a.normalized_value(quick_xml::XmlVersion::Explicit1_0)?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}
