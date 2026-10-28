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

/// The built-in colour matrix for a camera, if there is one.
pub fn built_in_matrix(make: &str, model: &str) -> Result<Option<Matrix>> {
    let known: Cameras = serde_json::from_str(include_str!("../data/colour-matrices.json"))?;
    Ok(known
        .cameras
        .into_iter()
        .find(|c| c.make == make && c.model == model)
        .map(|c| c.xyz_to_camera))
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
    built_in_matrix(make, model)?.ok_or_else(|| {
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
    /// The display JPEG's longest side; no JPEG when absent.
    pub jpeg: Option<usize>,
    /// The display image as a 16-bit TIFF at full size.
    pub tiff: bool,
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

/// A stored result with everything its rendering needs: the image, the frame it belongs to,
/// the sky weight and clear sky, glow, flat field and the camera's colour transform.
pub struct Prepared {
    pub image: Vec<[f32; 3]>,
    pub frame: usize,
    pub alpha: Vec<f32>,
    pub clear: Vec<bool>,
    pub glow: Vec<[f32; 3]>,
    pub flat: Vec<f32>,
    pub wb: [f32; 3],
    pub matrix: Matrix,
    pub white: f32,
}

impl Prepared {
    pub fn scene(&self) -> Scene<'_> {
        Scene {
            alpha: &self.alpha,
            clear: &self.clear,
            glow: &self.glow,
            flat: &self.flat,
            wb: self.wb,
            matrix: self.matrix,
            white: self.white,
        }
    }
}

/// The `planes` result of `store` (`final`, `hero`, `moment-…`) prepared for rendering, with
/// the colour matrix in `matrix` or the camera's built-in one.
pub fn prepare(store: &Store, planes: &str, matrix: Option<&Path>) -> Result<Prepared> {
    let (w, h) = (store.manifest.width, store.manifest.height);
    let result = planes::load(&store.dir.join(format!("{planes}.planes")))?;
    let image = rgb_of(&result, "")?;
    let hero = planes == "hero" || planes.starts_with("hero-");
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
            serde_json::from_slice(&std::fs::read(store.dir.join(format!("{planes}.json")))?)?;
        named(
            report["hero"]
                .as_str()
                .ok_or_else(|| error(format!("{planes}.json names no hero")))?,
        )?
    } else if let Some(centre) = planes.strip_prefix("moment-") {
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
    let xyz_to_camera = xyz_to_camera(store, matrix)?;
    Ok(Prepared {
        image,
        frame,
        alpha,
        clear,
        glow,
        flat,
        wb: store.manifest.frames[frame].wb,
        matrix: render::camera_to_srgb(&xyz_to_camera)?,
        white: store.manifest.frames[frame].scale_adu,
    })
}

pub fn render(store: &Store, o: &Options) -> Result<Report> {
    let h = store.manifest.height;
    let p = prepare(store, o.planes, o.matrix)?;
    let (frame, flat, clear, image) = (p.frame, &p.flat, &p.clear, &p.image);
    let w = store.manifest.width;
    let scene = p.scene();
    let single: Vec<[f32; 3]> = store
        .rgb_rows(frame, 0, h)?
        .as_chunks::<3>()
        .0
        .iter()
        .zip(flat)
        .map(|(c, f)| c.map(|v| v / f))
        .collect();
    let mut images: Vec<&[[f32; 3]]> = vec![image];
    if o.single {
        images.push(&single);
    }
    let lin = render::linear(image, &scene, &o.look);
    let (black, scale) = render::levels(&lin, clear, &o.look)?;
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
        if let Some(side) = o.jpeg {
            let (small, sw, sh) = export::reduce(display, w, h, side);
            export::jpeg(Path::new(&format!("{name}.jpg")), &small, sw, sh, 92)?;
            files.push(format!("{name}.jpg"));
        }
        if o.tiff {
            export::tiff16(
                Path::new(&format!("{name}.tif")),
                display,
                w,
                h,
                &description,
            )?;
            files.push(format!("{name}.tif"));
        }
    }
    if o.linear {
        export::linear(
            Path::new(&format!("{stem}-linear.tif")),
            image,
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
