//! Rendering a stored result (`final`, `hero`, `moment-…` planes) for review: the display
//! image as JPEG and 16-bit TIFF, optionally the reference frame with the same levels, and
//! the linear camera-RGB result as a float TIFF.

use crate::render::{self, Look, Matrix, Scene};
use crate::separate::glow::CLEAR;
use crate::store::{Store, planes};
use crate::{Result, error, export};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// An XYZ → camera colour matrix (Adobe ColorMatrix convention).
#[derive(Deserialize)]
struct ColourMatrix {
    xyz_to_camera: Matrix,
}

#[derive(Deserialize)]
struct Camera {
    make: String,
    model: String,
    xyz_to_camera: Matrix,
}

#[derive(Deserialize)]
struct Cameras {
    cameras: Vec<Camera>,
}

/// The colour matrix in `path`, or the built-in one for the store's camera.
fn xyz_to_camera(store: &Store, path: Option<&Path>) -> Result<Matrix> {
    if let Some(path) = path {
        let m: ColourMatrix = serde_json::from_slice(&std::fs::read(path)?)?;
        return Ok(m.xyz_to_camera);
    }
    let m = &store.manifest;
    let (make, model) = (
        m.make.as_deref().unwrap_or(""),
        m.model.as_deref().unwrap_or(""),
    );
    let known: Cameras = serde_json::from_str(include_str!("../data/colour-matrices.json"))?;
    known
        .cameras
        .into_iter()
        .find(|c| c.make == make && c.model == model)
        .map(|c| c.xyz_to_camera)
        .ok_or_else(|| {
            error(format!(
                "no built-in colour matrix for {make} {model}; give one (--matrix JSON)"
            ))
        })
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub planes: String,
    pub look: Look,
    pub frame: String,
    pub black: [f32; 3],
    pub scale: f32,
    pub files: Vec<String>,
}

/// Longest side of the JPEG unless asked otherwise.
pub const SIDE: usize = 2400;

pub struct Options<'a> {
    pub planes: &'a str,
    pub look: Look,
    /// A colour matrix file; the built-in one for the camera when absent.
    pub matrix: Option<&'a Path>,
    pub out: &'a Path,
    pub side: usize,
    pub single: bool,
    pub linear: bool,
}

fn rgb_of(p: &planes::Planes, prefix: &str) -> Result<Vec<[f32; 3]>> {
    let c: Vec<&[f32]> = ["r", "g", "b"]
        .iter()
        .map(|k| p.get(&format!("{prefix}{k}")))
        .collect::<Result<_>>()?;
    Ok((0..p.width * p.height)
        .map(|i| [c[0][i], c[1][i], c[2][i]])
        .collect())
}

pub fn render(store: &Store, o: &Options) -> Result<Report> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    let result = planes::load(&store.dir.join(format!("{}.planes", o.planes)))?;
    let image = rgb_of(&result, "")?;
    let hero = o.planes == "hero";
    let named = |name: &str| {
        store
            .manifest
            .frames
            .iter()
            .position(|f| f.name.split('.').next() == Some(name) || f.name == name)
            .ok_or_else(|| error(format!("{name} is not in the store")))
    };
    let frame = if hero {
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(store.dir.join("hero.json"))?)?;
        named(
            report["hero"]
                .as_str()
                .ok_or_else(|| error("hero.json names no hero"))?,
        )?
    } else if let Some(centre) = o.planes.strip_prefix("moment-") {
        named(centre)?
    } else {
        crate::separate::model(store)?.reference
    };
    let alpha = if hero {
        result.get("sky")?.to_vec()
    } else {
        planes::load(&store.dir.join("matte.planes"))?
            .get("alpha")?
            .to_vec()
    };
    let transient = if hero {
        result.get("transient")?.to_vec()
    } else {
        vec![0.0; w * h]
    };
    let matte = if hero {
        result.get("alpha")?.to_vec()
    } else {
        alpha.clone()
    };
    let clear: Vec<bool> = matte
        .iter()
        .zip(&transient)
        .map(|(&a, &t)| a >= CLEAR && t < 0.5)
        .collect();
    let glow = if hero {
        rgb_of(&result, "glow_")?
    } else {
        let grids = crate::separate::load_glow(store)?;
        (0..w * h)
            .map(|p| grids[frame].sample([(p % w) as f64, (p / w) as f64]))
            .collect()
    };
    let flat = crate::integrate::recorded_flat(store)?;
    let xyz_to_camera = xyz_to_camera(store, o.matrix)?;
    let scene = Scene {
        alpha: &alpha,
        clear: &clear,
        glow: &glow,
        flat: &flat,
        wb: store.manifest.frames[frame].wb,
        matrix: render::camera_to_srgb(&xyz_to_camera)?,
        white: store.manifest.frames[frame].scale_adu,
    };
    let single: Vec<[f32; 3]> = store
        .rgb_rows(frame, 0, h)?
        .as_chunks::<3>()
        .0
        .iter()
        .zip(&flat)
        .map(|(c, f)| c.map(|v| v / f))
        .collect();
    let mut images: Vec<&[[f32; 3]]> = vec![&image];
    if o.single {
        images.push(&single);
    }
    let lin = render::linear(&image, &scene, &o.look);
    let (black, scale) = render::levels(&lin, &clear, &o.look)?;
    let shown = render::compare(&images, &scene, &o.look)?;
    let mut files = Vec::new();
    let stem = o.out.to_string_lossy().into_owned();
    let description = serde_json::to_string(
        &serde_json::json!({"planes": o.planes, "look": o.look, "frame": store.manifest.frames[frame].name}),
    )?;
    for (k, display) in shown.iter().enumerate() {
        let name = if k == 0 {
            stem.clone()
        } else {
            format!("{stem}-single")
        };
        let (small, sw, sh) = export::reduce(display, w, h, o.side);
        export::jpeg(Path::new(&format!("{name}.jpg")), &small, sw, sh, 92)?;
        export::tiff16(
            Path::new(&format!("{name}.tif")),
            display,
            w,
            h,
            &description,
        )?;
        files.extend([format!("{name}.jpg"), format!("{name}.tif")]);
    }
    if o.linear {
        export::linear(
            Path::new(&format!("{stem}-linear.tif")),
            &image,
            w,
            h,
            &description,
        )?;
        files.push(format!("{stem}-linear.tif"));
    }
    Ok(Report {
        planes: o.planes.into(),
        look: o.look,
        frame: store.manifest.frames[frame].name.clone(),
        black,
        scale,
        files,
    })
}
