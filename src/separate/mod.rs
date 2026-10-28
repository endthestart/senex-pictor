//! Sky and foreground separation of a registered store (ADR 0009): temporal statistics with
//! sky-coherent transits, the matte, camera-fixed glow and per-frame occlusion.

pub mod cells;
pub mod filters;
pub mod glow;
pub mod grid;
pub mod matte;
pub mod occlusion;
pub mod order;
pub mod stats;
pub mod velocity;

use crate::budget::Budget;
use crate::lens::{self, Vignetting};
use crate::sky::SkyModel;
use crate::store::{Store, planes};
use crate::{Result, error};
use serde::Serialize;

/// The lensfun vignetting field of the store's lens (or `lens`, for files that do not
/// record it), focal length and aperture.
pub fn flat(store: &Store, lens: Option<&str>) -> Result<(Vec<f32>, Vignetting)> {
    let m = &store.manifest;
    let f = m.frames.first().ok_or_else(|| error("empty store"))?;
    let lens_name = lens.or(f.lens.as_deref()).ok_or_else(|| {
        error("no lens model recorded; name the lens (--lens) so vignetting can be corrected")
    })?;
    let (focal, aperture) = f
        .focal_mm
        .zip(f.aperture)
        .ok_or_else(|| error("no focal length or aperture recorded; vignetting needs them"))?;
    let v = lens::for_camera(m.make.as_deref(), lens_name, focal, aperture)?;
    Ok((lens::field(m.width, m.height, v.k), v))
}

pub fn model(store: &Store) -> Result<SkyModel> {
    Ok(serde_json::from_slice(
        &std::fs::read(store.dir.join("model.json")).map_err(|e| {
            error(format!(
                "{}: register the store first ({e})",
                store.dir.display()
            ))
        })?,
    )?)
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub vignetting: Vignetting,
    pub unresolvable_pole_pixels: usize,
    pub matte: matte::MatteReport,
    pub glow_iteration_rms_change_adu: Vec<f64>,
    pub occluded_share_percentiles: [f64; 4],
    pub seconds: [f64; 4],
}

/// Separates the store in place: `stats.planes`, `matte.planes`, `glow.planes`, `occ/` and
/// `separate.json`. `labels` are optional segments (one id per pixel).
pub fn separate(
    store: &Store,
    labels: Option<&[u32]>,
    lens: Option<&str>,
    budget: &Budget,
) -> Result<Report> {
    let (w, h, n) = (
        store.manifest.width,
        store.manifest.height,
        store.manifest.frames.len(),
    );
    let clock = std::time::Instant::now();
    let model = model(store)?;
    let (flat, vignetting) = flat(store, lens)?;
    let s = stats::temporal_stats(store, &model, 0..n, budget)?;
    planes::save(
        &store.dir.join("stats.planes"),
        w,
        h,
        &[
            ("median", &s.median),
            ("low", &s.low),
            ("sigma", &s.sigma),
            ("excess", &s.excess),
            ("transit", &s.transit),
            ("scatter", &s.scatter),
            ("resolvable", &planes::bools(&s.resolvable)),
        ],
    )?;
    let t_stats = clock.elapsed().as_secs_f64();
    budget.progress.step("Drawing the sky and ground", 0);
    let m = matte::matte(&s, n, labels)?;
    planes::save(
        &store.dir.join("matte.planes"),
        w,
        h,
        &[
            ("alpha", &m.alpha),
            ("seen", &planes::bools(&m.seen)),
            ("unstable", &planes::bools(&m.unstable)),
        ],
    )?;
    let t_matte = clock.elapsed().as_secs_f64() - t_stats;
    let (medians, _) = glow::block_medians(store, &m.alpha, &flat, None, budget)?;
    let (glow, _, changes) = glow::decompose(&medians, &model, &vec![true; n])?;
    save_glow(store, &glow)?;
    let t_glow = clock.elapsed().as_secs_f64() - t_stats - t_matte;
    let mut shares = occlusion::build(store, &glow, &m.alpha, &flat, budget)?;
    shares.sort_by(f64::total_cmp);
    let q = |p: f64| crate::sky::report::percentile(&shares, p).unwrap_or(0.0);
    let report = Report {
        vignetting,
        unresolvable_pole_pixels: s.resolvable.iter().filter(|&&r| !r).count(),
        matte: m.report,
        glow_iteration_rms_change_adu: changes,
        occluded_share_percentiles: [q(10.0), q(50.0), q(90.0), q(100.0)],
        seconds: [
            t_stats,
            t_matte,
            t_glow,
            clock.elapsed().as_secs_f64() - t_stats - t_matte - t_glow,
        ],
    };
    std::fs::write(
        store.dir.join("separate.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}

/// Glow grids as planes named "{frame}.{channel}".
pub fn save_glow(store: &Store, glow: &[glow::Grid]) -> Result<()> {
    let first = glow.first().ok_or_else(|| error("no glow"))?;
    let channels: Vec<(String, Vec<f32>)> = glow
        .iter()
        .enumerate()
        .flat_map(|(i, g)| {
            (0..3).map(move |k| (format!("{i}.{k}"), g.rgb.iter().map(|c| c[k]).collect()))
        })
        .collect();
    let named: Vec<(&str, &[f32])> = channels
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_slice()))
        .collect();
    planes::save(
        &store.dir.join("glow.planes"),
        first.columns,
        first.rows,
        &named,
    )
}

pub fn load_glow(store: &Store) -> Result<Vec<glow::Grid>> {
    let p = planes::load(&store.dir.join("glow.planes"))?;
    (0..store.manifest.frames.len())
        .map(|i| {
            let c: Vec<&[f32]> = (0..3)
                .map(|k| p.get(&format!("{i}.{k}")))
                .collect::<Result<_>>()?;
            Ok(glow::Grid {
                columns: p.width,
                rows: p.height,
                rgb: (0..p.width * p.height)
                    .map(|q| [c[0][q], c[1][q], c[2][q]])
                    .collect(),
            })
        })
        .collect()
}
