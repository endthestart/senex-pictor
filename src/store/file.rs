//! Little-endian sample files read at offsets, so large sequences stay on disk and every
//! worker reads only the rows or windows it needs. No memory maps: the crate forbids `unsafe`.

use crate::Result;
use half::f16;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Fills `buf` from `offset` in `file`.
pub fn read_at(file: &File, offset: u64, buf: &mut [u8]) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_exact_at(buf, offset)?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done = 0;
        while done < buf.len() {
            let n = file.seek_read(&mut buf[done..], offset + done as u64)?;
            if n == 0 {
                return Err(crate::error("unexpected end of sample file"));
            }
            done += n;
        }
    }
    Ok(())
}

/// Writes `bytes` to a new file at `path` (fails if it exists).
pub fn create(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = File::options().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Creates `path` as `len` zero bytes (fails if it exists), to be filled by `write_at`.
pub fn create_zeroed(path: &Path, len: u64) -> Result<()> {
    File::options()
        .write(true)
        .create_new(true)
        .open(path)?
        .set_len(len)?;
    Ok(())
}

/// Writes `bytes` at `offset` of the existing file `path` (`sync` it once filled).
pub fn write_at(path: &Path, offset: u64, bytes: &[u8]) -> Result<()> {
    let file = File::options().write(true).open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.write_all_at(bytes, offset)?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done = 0;
        while done < bytes.len() {
            done += file.seek_write(&bytes[done..], offset + done as u64)?;
        }
    }
    Ok(())
}

pub fn sync(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

pub fn f16_bytes(values: impl Iterator<Item = f32>) -> Vec<u8> {
    values
        .flat_map(|v| f16::from_f32(v).to_le_bytes())
        .collect()
}

pub fn f16_values(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| f16::from_le_bytes(*b).to_f32())
        .collect()
}

pub fn u16_bytes(values: impl Iterator<Item = u16>) -> Vec<u8> {
    values.flat_map(u16::to_le_bytes).collect()
}

pub fn u16_values(bytes: &[u8]) -> Vec<u16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect()
}

/// Bytes per row of a packed bit mask `width` pixels wide (most significant bit first).
pub fn bit_row(width: usize) -> usize {
    width.div_ceil(8)
}

pub fn pack_bits(mask: &[bool], width: usize) -> Vec<u8> {
    mask.chunks(width)
        .flat_map(|row| {
            row.chunks(8).map(|byte| {
                byte.iter()
                    .enumerate()
                    .fold(0u8, |acc, (i, &on)| acc | (u8::from(on) << (7 - i)))
            })
        })
        .collect()
}

pub fn unpack_bits(bytes: &[u8], width: usize) -> Vec<bool> {
    bytes
        .chunks(bit_row(width))
        .flat_map(|row| (0..width).map(move |x| row[x / 8] & (0x80 >> (x % 8)) != 0))
        .collect()
}
