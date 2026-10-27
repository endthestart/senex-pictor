//! Named float planes of one size in one file: magic, header length (u32), a JSON header
//! with width, height and names, then each plane as little-endian f32 in name order.

use super::file;
use crate::{Result, error};
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAGIC: &[u8; 8] = b"SPPLANE1";

#[derive(Serialize, Deserialize)]
struct Header {
    width: usize,
    height: usize,
    names: Vec<String>,
}

pub struct Planes {
    pub width: usize,
    pub height: usize,
    pub names: Vec<String>,
    pub data: Vec<Vec<f32>>,
}

impl Planes {
    pub fn get(&self, name: &str) -> Result<&[f32]> {
        self.names
            .iter()
            .position(|n| n == name)
            .map(|i| self.data[i].as_slice())
            .ok_or_else(|| error(format!("no plane named {name}")))
    }
}

pub fn bools(mask: &[bool]) -> Vec<f32> {
    mask.iter().map(|&m| f32::from(u8::from(m))).collect()
}

/// Writes (replacing) `path`.
pub fn save(path: &Path, width: usize, height: usize, planes: &[(&str, &[f32])]) -> Result<()> {
    if planes.iter().any(|(_, p)| p.len() != width * height) {
        return Err(error("planes differ from the stated size"));
    }
    let header = serde_json::to_vec(&Header {
        width,
        height,
        names: planes.iter().map(|(n, _)| n.to_string()).collect(),
    })?;
    let mut out = MAGIC.to_vec();
    out.extend((header.len() as u32).to_le_bytes());
    out.extend(header);
    for (_, p) in planes {
        out.extend(p.iter().flat_map(|v| v.to_le_bytes()));
    }
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    file::create(path, &out)
}

pub fn load(path: &Path) -> Result<Planes> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 12 || &bytes[..8] != MAGIC {
        return Err(error(format!("{} is not a planes file", path.display())));
    }
    let length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let header: Header = serde_json::from_slice(
        bytes
            .get(12..12 + length)
            .ok_or_else(|| error(format!("{} is truncated", path.display())))?,
    )?;
    let size = header.width * header.height * 4;
    let body = &bytes[12 + length..];
    if body.len() != size * header.names.len() {
        return Err(error(format!("{} has the wrong size", path.display())));
    }
    let data = body
        .chunks(size)
        .map(|c| {
            c.as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect()
        })
        .collect();
    Ok(Planes {
        width: header.width,
        height: header.height,
        names: header.names,
        data,
    })
}
