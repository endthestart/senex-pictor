//! HDR output as a gain-map JPEG (Ultra HDR 1.0 and ISO 21496-1 metadata, laid out as
//! libultrahdr writes it): the SDR rendering as an ordinary JPEG every viewer shows, followed
//! by a full-size gain map that HDR displays apply to lift what the HDR rendering holds above
//! paper white (the brightest stars), as far as each display can. Stars are a pixel or two
//! across, so a coarser map would spread their gain into the sky around them.

use crate::render::encode;
use crate::{Result, error};
use jpeg_encoder::{ColorType, Encoder};

const OFFSET: f32 = 1.0 / 64.0;
const XMP_ID: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const ISO_ID: &[u8] = b"urn:iso:std:iso:ts:21496:-1\0";
const MPF_ID: &[u8] = b"MPF\0";
const DENOMINATOR: u32 = 1_000_000;
const WEIGHTS: [f32; 3] = [0.2126, 0.7152, 0.0722];
/// The smallest boost range written, so an image without HDR highlights still has a valid map.
const MIN_RANGE_LOG2: f32 = 0.01;

fn luminance(c: &[f32; 3]) -> f32 {
    c[0] * WEIGHTS[0] + c[1] * WEIGHTS[1] + c[2] * WEIGHTS[2]
}

/// The map of log2 HDR-over-SDR luminance (with the format's offsets), normalised to its range
/// and quantised; with that range (log2).
struct GainMap {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    min_log2: f32,
    max_log2: f32,
}

fn gain_map(sdr: &[[f32; 3]], hdr: &[[f32; 3]], w: usize, h: usize) -> GainMap {
    let logs: Vec<f32> = sdr
        .iter()
        .zip(hdr)
        .map(|(s, d)| ((luminance(d).max(0.0) + OFFSET) / (luminance(s).max(0.0) + OFFSET)).log2())
        .collect();
    let min_log2 = logs.iter().copied().fold(0.0, f32::min);
    let max_log2 = logs
        .iter()
        .copied()
        .fold(f32::MIN, f32::max)
        .max(min_log2 + MIN_RANGE_LOG2);
    let pixels = logs
        .iter()
        .map(|l| (((l - min_log2) / (max_log2 - min_log2)).clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
        .collect();
    GainMap {
        pixels,
        width: w,
        height: h,
        min_log2,
        max_log2,
    }
}

fn jpeg(
    pixels: &[u8],
    (w, h): (usize, usize),
    colour: ColorType,
    quality: u8,
    segments: Vec<(u8, Vec<u8>)>,
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = Encoder::new(&mut out, quality);
    for (nr, data) in segments {
        encoder.add_app_segment(nr, data)?;
    }
    encoder.encode(pixels, u16::try_from(w)?, u16::try_from(h)?, colour)?;
    Ok(out)
}

fn with_id(id: &[u8], body: &[u8]) -> Vec<u8> {
    [id, body].concat()
}

fn xmp_primary(map_bytes: usize) -> Vec<u8> {
    with_id(
        XMP_ID,
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Pictor"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/" hdrgm:Version="1.0"><Container:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><Container:Item Item:Semantic="Primary" Item:Mime="image/jpeg"/></rdf:li><rdf:li rdf:parseType="Resource"><Container:Item Item:Semantic="GainMap" Item:Mime="image/jpeg" Item:Length="{map_bytes}"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#
        )
        .as_bytes(),
    )
}

fn xmp_map(m: &GainMap) -> Vec<u8> {
    let (lo, hi) = (m.min_log2, m.max_log2);
    with_id(
        XMP_ID,
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Pictor"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/" hdrgm:Version="1.0" hdrgm:GainMapMin="{lo}" hdrgm:GainMapMax="{hi}" hdrgm:Gamma="1" hdrgm:OffsetSDR="{OFFSET}" hdrgm:OffsetHDR="{OFFSET}" hdrgm:HDRCapacityMin="0" hdrgm:HDRCapacityMax="{hi}" hdrgm:BaseRenditionIsHDR="False"/></rdf:RDF></x:xmpmeta>"#
        )
        .as_bytes(),
    )
}

/// The ISO 21496-1 version record the primary image carries.
fn iso_version() -> Vec<u8> {
    with_id(ISO_ID, &[0, 0, 0, 0])
}

/// The ISO 21496-1 metadata of a single-channel map in the base colour space (big-endian
/// fractions; headrooms and boosts in log2).
fn iso_map(m: &GainMap) -> Vec<u8> {
    let signed = |v: f32| ((v * DENOMINATOR as f32).round() as i32).to_be_bytes();
    let unsigned = |v: f32| ((v * DENOMINATOR as f32).round() as u32).to_be_bytes();
    let d = DENOMINATOR.to_be_bytes();
    let mut b = vec![0, 0, 0, 0, 1 << 6];
    for part in [
        unsigned(0.0),
        d,
        unsigned(m.max_log2),
        d,
        signed(m.min_log2),
        d,
        signed(m.max_log2),
        d,
        unsigned(1.0),
        d,
        signed(OFFSET),
        d,
        signed(OFFSET),
        d,
    ] {
        b.extend(part);
    }
    with_id(ISO_ID, &b)
}

/// The MPF index of the primary (at offset 0) and the gain map (`map_offset` bytes past the
/// MPF header), big-endian.
fn mpf(primary_bytes: u32, map_bytes: u32, map_offset: u32) -> Vec<u8> {
    let mut b = MPF_ID.to_vec();
    b.extend(b"MM\0\x2a");
    b.extend(8u32.to_be_bytes());
    b.extend(3u16.to_be_bytes());
    let tag = |b: &mut Vec<u8>, id: u16, kind: u16, count: u32, value: [u8; 4]| {
        b.extend(id.to_be_bytes());
        b.extend(kind.to_be_bytes());
        b.extend(count.to_be_bytes());
        b.extend(value);
    };
    tag(&mut b, 0xB000, 7, 4, *b"0100");
    tag(&mut b, 0xB001, 4, 1, 2u32.to_be_bytes());
    tag(&mut b, 0xB002, 7, 32, 50u32.to_be_bytes());
    b.extend(0u32.to_be_bytes());
    for (attribute, size, offset) in [(0x030000u32, primary_bytes, 0), (0, map_bytes, map_offset)] {
        b.extend(attribute.to_be_bytes());
        b.extend(size.to_be_bytes());
        b.extend(offset.to_be_bytes());
        b.extend([0u8; 4]);
    }
    b
}

/// The gain-map JPEG of linear display renderings `sdr` (0..1) and `hdr` (0..headroom).
pub fn gain_map_jpeg(
    sdr: &[[f32; 3]],
    hdr: &[[f32; 3]],
    (w, h): (usize, usize),
    quality: u8,
) -> Result<Vec<u8>> {
    if sdr.len() != w * h || hdr.len() != w * h {
        return Err(error("HDR output: renderings differ from the stated size"));
    }
    let map = gain_map(sdr, hdr, w, h);
    let map_jpeg = jpeg(
        &map.pixels,
        (map.width, map.height),
        ColorType::Luma,
        quality,
        vec![(1, xmp_map(&map)), (2, iso_map(&map))],
    )?;
    let base: Vec<u8> = sdr
        .iter()
        .flat_map(|c| c.map(|v| (encode(v) * 255.0 + 0.5) as u8))
        .collect();
    let placeholder = mpf(0, 0, 0);
    let mut primary = jpeg(
        &base,
        (w, h),
        ColorType::Rgb,
        quality,
        vec![
            (1, xmp_primary(map_jpeg.len())),
            (2, iso_version()),
            (2, placeholder.clone()),
        ],
    )?;
    let at = primary
        .windows(placeholder.len())
        .position(|win| win == placeholder.as_slice())
        .ok_or_else(|| error("HDR output: the MPF index went missing"))?;
    let header = (at + MPF_ID.len()) as u32;
    let total = u32::try_from(primary.len())?;
    let index = mpf(total, u32::try_from(map_jpeg.len())?, total - header);
    primary[at..at + index.len()].copy_from_slice(&index);
    primary.extend(map_jpeg);
    Ok(primary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mpf_index_points_at_the_gain_map_and_both_carry_metadata() -> Result<()> {
        let (w, h) = (64, 48);
        let sdr: Vec<[f32; 3]> = (0..w * h).map(|p| [(p % w) as f32 / w as f32; 3]).collect();
        let hdr: Vec<[f32; 3]> = sdr
            .iter()
            .enumerate()
            .map(|(p, c)| if p == 10 * w + 20 { [3.0; 3] } else { *c })
            .collect();
        let file = gain_map_jpeg(&sdr, &hdr, (w, h), 90)?;
        let find = |needle: &[u8], from: usize| {
            file[from..]
                .windows(needle.len())
                .position(|win| win == needle)
                .map(|p| p + from)
        };
        let at = find(MPF_ID, 0).ok_or("no MPF index")?;
        let header = at + MPF_ID.len();
        let u32_at =
            |i: usize| u32::from_be_bytes([file[i], file[i + 1], file[i + 2], file[i + 3]]);
        let entries = header + 50;
        let (primary, map, offset) = (
            u32_at(entries + 4),
            u32_at(entries + 20),
            u32_at(entries + 24),
        );
        let start = header + offset as usize;
        assert_eq!(
            primary as usize, start,
            "the primary ends where the gain map starts"
        );
        assert_eq!(
            start + map as usize,
            file.len(),
            "the gain map runs to the end"
        );
        assert_eq!(
            &file[start..start + 2],
            &[0xFF, 0xD8],
            "the gain map is a JPEG"
        );
        assert!(find(b"hdrgm:GainMapMax", start).is_some() && find(ISO_ID, start).is_some());
        assert!(find(b"Item:Semantic=\"GainMap\"", 0).is_some_and(|p| p < start));
        Ok(())
    }
}
