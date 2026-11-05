//! Built-in colour matrices: Adobe's ColorMatrix for every camera in LibRaw's table
//! (`data/colour-matrices.json`, written by `tools/colour_matrices.py`), found by whole model
//! name so that a camera the table does not list fails instead of borrowing an older model's.

use crate::render::Matrix;
use crate::{Result, error};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
pub struct Camera {
    pub maker: String,
    pub models: Vec<String>,
    pub xyz_to_camera: Matrix,
}

#[derive(Deserialize)]
struct Cameras {
    cameras: Vec<Camera>,
}

/// LibRaw's maker for a word (or two) of a file's Make, in lower case; the first match wins.
pub const MAKERS: &[(&str, &str)] = &[
    ("agfa", "Agfa"),
    ("apple", "Apple"),
    ("broadcom", "Broadcom"),
    ("canon", "Canon"),
    ("casio", "Casio"),
    ("cine", "CINE"),
    ("contax", "Contax"),
    ("dxo", "DXO"),
    ("epson", "Epson"),
    ("fujifilm", "Fujifilm"),
    ("gitup", "GITUP"),
    ("gopro", "GoPro"),
    ("hasselblad", "Hasselblad"),
    ("htc", "HTC"),
    ("imacon", "Imacon"),
    ("kodak", "Kodak"),
    ("leaf", "Leaf"),
    ("leica", "Leica"),
    ("mamiya", "Mamiya"),
    ("micron", "Micron"),
    ("minolta", "Minolta"),
    ("motorola", "Motorola"),
    ("nikon", "Nikon"),
    ("olympus", "Olympus"),
    ("om digital", "Olympus"),
    ("panasonic", "Panasonic"),
    ("pentax", "Pentax"),
    ("phase one", "PhaseOne"),
    ("photron", "Photron"),
    ("polaroid", "Polaroid"),
    ("red", "RED"),
    ("ricoh", "Ricoh"),
    ("samsung", "Samsung"),
    ("sigma", "Sigma"),
    ("sony", "Sony"),
    ("yi", "YI"),
];

/// Every built-in matrix, read once.
pub fn table() -> Result<&'static [Camera]> {
    static TABLE: OnceLock<std::result::Result<Vec<Camera>, String>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            serde_json::from_str::<Cameras>(include_str!("../data/colour-matrices.json"))
                .map(|c| c.cameras)
                .map_err(|e| e.to_string())
        })
        .as_deref()
        .map_err(|e| error(format!("built-in colour matrices: {e}")))
}

/// Lower case, with anything but letters and digits turned into single spaces.
fn words(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// LibRaw's maker of a camera whose files say `make` and `model`.
pub fn maker(make: &str, model: &str) -> Option<&'static str> {
    let padded = format!(" {} ", words(make));
    let (_, maker) = MAKERS
        .iter()
        .find(|(key, _)| padded.contains(&format!(" {key} ")))?;
    // Pentax bodies made by Ricoh say RICOH IMAGING in Make and PENTAX in Model.
    if *maker == "Ricoh" && model.to_lowercase().starts_with("pentax") {
        return Some("Pentax");
    }
    Some(maker)
}

/// The model as LibRaw names it: without leading maker words ("NIKON Z 6_2" is "Z 6_2",
/// "FinePix S5Pro" is "S5Pro") and with "Mark" set apart ("E-M1MarkII" is "E-M1 Mark II").
pub fn model_name(make: &str, maker: &str, model: &str) -> String {
    let skip: Vec<String> = words(make)
        .split(' ')
        .map(str::to_string)
        .chain([maker.to_lowercase(), "finepix".to_string()])
        .collect();
    let mut parts: Vec<&str> = model.split_whitespace().collect();
    while parts.len() > 1 && skip.contains(&parts[0].to_lowercase()) {
        parts.remove(0);
    }
    let name = parts.join(" ");
    match name.split_once("Mark") {
        Some((head, numeral)) => format!("{} Mark {}", head.trim_end(), numeral.trim_start())
            .trim_end()
            .to_string(),
        None => name,
    }
}

/// The built-in colour matrix for a camera whose files say `make` and `model`, if LibRaw
/// lists that model by name.
pub fn built_in_matrix(make: &str, model: &str) -> Result<Option<Matrix>> {
    let Some(maker) = maker(make, model) else {
        return Ok(None);
    };
    let name = model_name(make, maker, model).to_lowercase();
    Ok(table()?
        .iter()
        .find(|c| c.maker == maker && c.models.iter().any(|m| m.to_lowercase() == name))
        .map(|c| c.xyz_to_camera))
}
