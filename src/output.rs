use crate::{Result, error, input::Identity, pipeline::Development};
use lcc_raw::Rgb32f;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use tiff::encoder::{TiffEncoder, colortype::RGB32Float};
use tiff::tags::Tag;

#[derive(Serialize)]
struct Report<'a> {
    schema: u32,
    source: &'a Identity,
    development: &'a Development,
    decoder_revision: &'static str,
}

pub fn save(
    dir: &Path,
    image: &Rgb32f,
    source: &Identity,
    development: &Development,
) -> Result<()> {
    std::fs::create_dir(dir)?;
    let result = write(dir, image, source, development);
    if result.is_ok() {
        std::fs::remove_file(dir.join("INCOMPLETE"))?;
    }
    result
}

fn create(path: &Path) -> Result<File> {
    Ok(OpenOptions::new().write(true).create_new(true).open(path)?)
}

fn write(dir: &Path, image: &Rgb32f, source: &Identity, development: &Development) -> Result<()> {
    create(&dir.join("INCOMPLETE"))?.sync_all()?;
    let report = Report {
        schema: 1,
        source,
        development,
        decoder_revision: crate::DECODER_REVISION,
    };
    let json = serde_json::to_string_pretty(&report)?;
    let file = create(&dir.join("camera-linear.tif"))?;
    let mut writer = BufWriter::new(&file);
    let mut encoder = TiffEncoder::new(&mut writer)?;
    let mut frame = encoder
        .new_image::<RGB32Float>(u32::try_from(image.width)?, u32::try_from(image.height)?)?;
    frame
        .encoder()
        .write_tag(Tag::ImageDescription, json.as_str())?;
    frame
        .encoder()
        .write_tag(Tag::Software, "Pictor RAW diagnostic 0.1.0")?;
    frame.encoder().write_tag(Tag::Orientation, 1u16)?;
    let flat: &[f32] = image.data.as_flattened();
    if flat.len()
        != image
            .width
            .checked_mul(image.height)
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(|| error("output dimensions overflow"))?
    {
        return Err(error("pixel count does not match output dimensions"));
    }
    frame.write_data(flat)?;
    writer.flush()?;
    file.sync_all()?;
    let file = create(&dir.join("report.json"))?;
    serde_json::to_writer_pretty(&file, &report)?;
    file.sync_all()?;
    Ok(())
}
