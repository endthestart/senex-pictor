//! Sky and foreground separation of a registered store (ADR 0009): temporal statistics with
//! sky-coherent transits, the matte (with the owner's corrections laid over), camera-fixed glow
//! and per-frame occlusion. After corrections the matte onward is drawn again from the saved
//! statistics (`rematte`).

pub mod cells;
pub mod corrections;
pub mod evidence;
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
    /// Pixels the owner painted as sky or ground.
    pub corrected_pixels: usize,
    pub glow_iteration_rms_change_adu: Vec<f64>,
    pub occluded_share_percentiles: [f64; 4],
    pub seconds: [f64; 4],
}

const STATS: &str = "stats.planes";
const STATS_PLANES: [&str; 7] = [
    "low",
    "sigma",
    "transit",
    "scatter",
    "resolvable",
    "red",
    "blue",
];

fn save_stats(store: &Store, s: &stats::Stats) -> Result<()> {
    let resolvable = planes::bools(&s.resolvable);
    let values: [&[f32]; 7] = [
        &s.low,
        &s.sigma,
        &s.transit,
        &s.scatter,
        &resolvable,
        &s.red,
        &s.blue,
    ];
    let named: Vec<(&str, &[f32])> = STATS_PLANES.into_iter().zip(values).collect();
    planes::save(&store.dir.join(STATS), s.width, s.height, &named)
}

fn load_stats(store: &Store) -> Result<stats::Stats> {
    let p = planes::load(&store.dir.join(STATS))?;
    let get = |name: &str| -> Result<Vec<f32>> { Ok(p.get(name)?.to_vec()) };
    Ok(stats::Stats {
        width: p.width,
        height: p.height,
        low: get("low")?,
        sigma: get("sigma")?,
        transit: get("transit")?,
        scatter: get("scatter")?,
        resolvable: p.get("resolvable")?.iter().map(|&v| v > 0.5).collect(),
        red: get("red")?,
        blue: get("blue")?,
    })
}

/// Separates the store in place: `stats.planes`, `matte.planes`, `glow.planes`, `occ/` and
/// `separate.json`. `labels` are optional segments (one id per pixel).
pub fn separate(
    store: &Store,
    labels: Option<&[u32]>,
    lens: Option<&str>,
    budget: &Budget,
) -> Result<Report> {
    let clock = std::time::Instant::now();
    let model = model(store)?;
    let field = flat(store, lens)?;
    let n = store.manifest.frames.len();
    let s = stats::temporal_stats(store, &model, 0..n, budget)?;
    save_stats(store, &s)?;
    from_stats(store, &model, &s, labels, field, clock, budget)
}

/// The matte, glow and occlusion drawn again from the saved statistics, after corrections.
pub fn rematte(
    store: &Store,
    labels: Option<&[u32]>,
    lens: Option<&str>,
    budget: &Budget,
) -> Result<Report> {
    let clock = std::time::Instant::now();
    let field = flat(store, lens)?;
    let s = load_stats(store)?;
    from_stats(store, &model(store)?, &s, labels, field, clock, budget)
}

fn from_stats(
    store: &Store,
    model: &SkyModel,
    s: &stats::Stats,
    labels: Option<&[u32]>,
    (flat, vignetting): (Vec<f32>, Vignetting),
    clock: std::time::Instant,
    budget: &Budget,
) -> Result<Report> {
    let (w, h, n) = (
        store.manifest.width,
        store.manifest.height,
        store.manifest.frames.len(),
    );
    let t_stats = clock.elapsed().as_secs_f64();
    budget.progress.step("Drawing the sky and ground", 0);
    let mut m = matte::matte(s, n, labels)?;
    let corrected = corrections::correct(store, &mut m.alpha)?;
    planes::save(
        &store.dir.join("matte.planes"),
        w,
        h,
        &[
            ("alpha", &m.alpha),
            ("seen", &planes::bools(&m.seen)),
            ("unstable", &planes::bools(&m.unstable)),
            ("lit", &planes::bools(&m.lit)),
        ],
    )?;
    let t_matte = clock.elapsed().as_secs_f64() - t_stats;
    let (medians, _) = glow::block_medians(store, &m.alpha, &flat, None, budget)?;
    let (glow, _, changes) = glow::decompose(&medians, model, &vec![true; n])?;
    save_glow(store, &glow)?;
    let t_glow = clock.elapsed().as_secs_f64() - t_stats - t_matte;
    let mut shares = occlusion::build(store, &glow, &m.alpha, &flat, budget)?;
    shares.sort_by(f64::total_cmp);
    let q = |p: f64| crate::sky::report::percentile(&shares, p).unwrap_or(0.0);
    let report = Report {
        vignetting,
        unresolvable_pole_pixels: s.resolvable.iter().filter(|&&r| !r).count(),
        matte: m.report,
        corrected_pixels: corrected,
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
