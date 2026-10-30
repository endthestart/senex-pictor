//! Image files: display renderings as 8-bit JPEG (optionally reduced) or 16-bit TIFF, and the
//! linear camera-RGB composite as a 32-bit float TIFF for further editing.

use crate::{Result, error};
use std::io::{BufWriter, Write};
use std::path::Path;
use tiff::encoder::{TiffEncoder, colortype};
use tiff::tags::Tag;

fn check(pixels: usize, w: usize, h: usize) -> Result<()> {
    if pixels != w * h || w == 0 || h == 0 {
        return Err(error("image size does not match its pixels"));
    }
    Ok(())
}

/// Area-average reduction so the longer side is at most `side`.
pub fn reduce(rgb: &[[f32; 3]], w: usize, h: usize, side: usize) -> (Vec<[f32; 3]>, usize, usize) {
    let factor = w.max(h).div_ceil(side.max(1)).max(1);
    if factor == 1 {
        return (rgb.to_vec(), w, h);
    }
    let (nw, nh) = (w / factor, h / factor);
    let out = (0..nw * nh)
        .map(|p| {
            let (x0, y0) = ((p % nw) * factor, (p / nw) * factor);
            let mut s = [0f32; 3];
            for y in y0..y0 + factor {
                for x in x0..x0 + factor {
                    for k in 0..3 {
                        s[k] += rgb[y * w + x][k];
                    }
                }
            }
            s.map(|v| v / (factor * factor) as f32)
        })
        .collect();
    (out, nw, nh)
}

pub fn jpeg(path: &Path, display: &[[f32; 3]], w: usize, h: usize, quality: u8) -> Result<()> {
    check(display.len(), w, h)?;
    let bytes: Vec<u8> = display
        .iter()
        .flat_map(|c| c.map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8))
        .collect();
    let encoder = jpeg_encoder::Encoder::new_file(path, quality)?;
    encoder.encode(
        &bytes,
        u16::try_from(w)?,
        u16::try_from(h)?,
        jpeg_encoder::ColorType::Rgb,
    )?;
    Ok(())
}

pub fn tiff16(
    path: &Path,
    display: &[[f32; 3]],
    w: usize,
    h: usize,
    description: &str,
) -> Result<()> {
    check(display.len(), w, h)?;
    let data: Vec<u16> = display
        .iter()
        .flat_map(|c| c.map(|v| (v.clamp(0.0, 1.0) * 65535.0 + 0.5) as u16))
        .collect();
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(&file);
    let mut encoder = TiffEncoder::new(&mut writer)?;
    let mut image = encoder.new_image::<colortype::RGB16>(u32::try_from(w)?, u32::try_from(h)?)?;
    image
        .encoder()
        .write_tag(Tag::ImageDescription, description)?;
    image.encoder().write_tag(Tag::Software, "Pictor")?;
    image.write_data(&data)?;
    writer.flush()?;
    file.sync_all()?;
    Ok(())
}

/// Linear camera RGB (ADU above black, flat-fielded) as 32-bit float; non-finite values are
/// written as 0.
pub fn linear(path: &Path, rgb: &[[f32; 3]], w: usize, h: usize, description: &str) -> Result<()> {
    check(rgb.len(), w, h)?;
    let data: Vec<f32> = rgb
        .iter()
        .flat_map(|c| c.map(|v| if v.is_finite() { v } else { 0.0 }))
        .collect();
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(&file);
    let mut encoder = TiffEncoder::new(&mut writer)?;
    let mut image =
        encoder.new_image::<colortype::RGB32Float>(u32::try_from(w)?, u32::try_from(h)?)?;
    image
        .encoder()
        .write_tag(Tag::ImageDescription, description)?;
    image.encoder().write_tag(Tag::Software, "Pictor")?;
    image.write_data(&data)?;
    writer.flush()?;
    file.sync_all()?;
    Ok(())
}
