//! The scene the looks are applied to: a stored result prepared by the review renderer, as
//! linear sRGB divided by sensor white with the sky weight in alpha, the sky-weighted glow
//! the same way, and each look's levels measured on the CPU exactly as the renderer does.

use half::f16;
use star_photos::render::{self, Look};
use star_photos::review;
use star_photos::store::Store;
use star_photos::{Result, error};
use std::path::Path;

/// One look's parameters as the compute shader reads them (`look.wgsl` `Look`).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LookParams {
    pub black: [f32; 4],
    pub scale: f32,
    pub stretch: f32,
    pub saturation: f32,
    pub shadows: f32,
    pub white: f32,
    pub remove_glow: f32,
    pub norm: f32,
    pub pad: f32,
}

impl LookParams {
    fn of(look: &Look, black: [f32; 3], scale: f32, white: f32) -> LookParams {
        LookParams {
            black: [black[0] / white, black[1] / white, black[2] / white, 0.0],
            scale: scale / white,
            stretch: look.stretch,
            saturation: look.saturation,
            shadows: look.shadows,
            white: 1.0,
            remove_glow: if look.remove_glow { 1.0 } else { 0.0 },
            norm: look.stretch.asinh(),
            pad: 0.0,
        }
    }

    /// With the stretch changed (and its normaliser).
    pub fn stretched(self, stretch: f32) -> LookParams {
        LookParams {
            stretch,
            norm: stretch.asinh(),
            ..self
        }
    }
}

pub struct Scene {
    pub width: u32,
    pub height: u32,
    /// RGBA half floats: linear sRGB / white, sky weight.
    pub base: Vec<u16>,
    /// RGBA half floats: sky weight × glow, linear sRGB / white.
    pub glow: Vec<u16>,
    pub looks: [LookParams; 2],
    pub names: [&'static str; 2],
}

fn halves(rgb: &[[f32; 3]], fourth: impl Fn(usize) -> f32, white: f32) -> Vec<u16> {
    rgb.iter()
        .enumerate()
        .flat_map(|(p, c)| {
            [c[0] / white, c[1] / white, c[2] / white, fourth(p)]
                .map(|v| f16::from_f32(v).to_bits())
        })
        .collect()
}

/// The scene from linear sRGB `base` and sky-weighted `glow`, with each look's levels measured
/// over `clear` sky.
pub fn assemble(
    width: u32,
    height: u32,
    base: &[[f32; 3]],
    glow: &[[f32; 3]],
    alpha: &[f32],
    clear: &[bool],
    white: f32,
) -> Result<Scene> {
    let looks = [render::NATURAL, render::DARK].map(|look| -> Result<LookParams> {
        let shown: Vec<[f32; 3]> = if look.remove_glow {
            base.iter()
                .zip(glow)
                .map(|(b, g)| std::array::from_fn(|k| b[k] - g[k]))
                .collect()
        } else {
            base.to_vec()
        };
        let (black, scale) = render::levels(&shown, clear, &look)?;
        Ok(LookParams::of(&look, black, scale, white))
    });
    let [natural, dark] = looks;
    Ok(Scene {
        width,
        height,
        base: halves(base, |q| alpha[q], white),
        glow: halves(glow, |_| 1.0, white),
        looks: [natural?, dark?],
        names: ["natural", "dark"],
    })
}

/// The `planes` result of the store at `dir`, with the natural and dark looks.
pub fn load(dir: &Path, planes: &str) -> Result<Scene> {
    let store = Store::open(dir)?;
    let p = review::prepare(&store, planes, None)?;
    let scene = p.scene();
    let keep = Look {
        remove_glow: false,
        ..render::NATURAL
    };
    let base = render::linear(&p.image, &scene, &keep);
    let glow: Vec<[f32; 3]> = render::linear(&p.glow, &scene, &keep)
        .iter()
        .zip(&p.alpha)
        .map(|(g, a)| g.map(|v| v * a))
        .collect();
    assemble(
        store.manifest.width as u32,
        store.manifest.height as u32,
        &base,
        &glow,
        &p.alpha,
        &p.clear,
        p.white,
    )
}

/// The size to render `scene` at: its own, or `mp` megapixels at its aspect ratio (a stand-in
/// for a larger sensor).
pub fn output_size(scene: &Scene, mp: Option<f64>) -> [u32; 2] {
    match mp {
        None => [scene.width, scene.height],
        Some(mp) => {
            let (w, h) = (f64::from(scene.width), f64::from(scene.height));
            let k = (mp * 1e6 / (w * h)).sqrt();
            [(w * k).round() as u32, (h * k).round() as u32]
        }
    }
}

const MAGIC: &[u8; 8] = b"SPSCENE1";

/// Writes the scene to `path`: magic, width, height, both looks, then the two RGBA half-float
/// planes (little-endian), so it can be viewed on a machine without the store.
pub fn save(scene: &Scene, path: &Path) -> Result<()> {
    let mut bytes = Vec::with_capacity(16 + 2 * 48 + 4 * (scene.base.len() + scene.glow.len()));
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&scene.width.to_le_bytes());
    bytes.extend_from_slice(&scene.height.to_le_bytes());
    for look in &scene.looks {
        bytes.extend_from_slice(bytemuck::bytes_of(look));
    }
    for plane in [&scene.base, &scene.glow] {
        bytes.extend(plane.iter().flat_map(|v| v.to_le_bytes()));
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn open(path: &Path) -> Result<Scene> {
    let bytes = std::fs::read(path)?;
    let look_size = std::mem::size_of::<LookParams>();
    let header = 16 + 2 * look_size;
    if bytes.len() < header || &bytes[..8] != MAGIC {
        return Err(error(format!(
            "{} is not a spike scene file",
            path.display()
        )));
    }
    let word =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let (width, height) = (word(8), word(12));
    let n = 4 * (width as usize) * (height as usize);
    if bytes.len() != header + 4 * n {
        return Err(error("the scene file is truncated"));
    }
    let look = |k: usize| -> LookParams {
        bytemuck::pod_read_unaligned(&bytes[16 + k * look_size..16 + (k + 1) * look_size])
    };
    let plane = |at: usize| -> Vec<u16> {
        bytes[at..at + 2 * n]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b))
            .collect()
    };
    Ok(Scene {
        width,
        height,
        base: plane(header),
        glow: plane(header + 2 * n),
        looks: [look(0), look(1)],
        names: ["natural", "dark"],
    })
}
