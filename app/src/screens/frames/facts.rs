//! What a run says about its frames, gathered from its report and sequence check, and a
//! larger preview of one frame read from the store.

use crate::project::Run;
use egui::ColorImage;
use pictor_engine::store::Store;
use pictor_engine::{Result, error};

/// Long side of the larger preview read from the store.
const PREVIEW_SIDE: usize = 1600;

/// What the run says about its frames.
pub struct Facts {
    pub names: Vec<String>,
    pub lit: Vec<String>,
    pub overexposed: Vec<String>,
    pub reference: Option<String>,
    pub drift: Vec<(String, f64)>,
    pub widest: Option<f64>,
    pub lost: Vec<String>,
    pub stars: Option<u64>,
    /// Whether the long exposure moved each frame back by its drift (a run composed around
    /// the reference frame); a whole-sequence stack does not.
    pub moved_back: bool,
}

/// Those of `names` between the run's first and last frames.
fn within(run: &Run, names: &[String]) -> Vec<String> {
    let at = |name: &str| run.survey.captures.iter().position(|c| c.name == name);
    let (Some(a), Some(b)) = (at(&run.report.first), at(&run.report.last)) else {
        return names.to_vec();
    };
    names
        .iter()
        .filter(|n| at(n).is_some_and(|i| (a..=b).contains(&i)))
        .cloned()
        .collect()
}

impl Facts {
    pub fn of(run: &Run) -> Facts {
        let r = &run.report;
        let check_run = run.survey.check.runs.get(r.run);
        let stage = |name: &str| r.stage(name).map(|s| &s.report);
        let drift = stage("foreground drift");
        let names_of = |v: Option<&serde_json::Value>| -> Vec<String> {
            v.and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default()
        };
        Facts {
            names: run
                .store
                .manifest
                .frames
                .iter()
                .map(|f| f.name.clone())
                .collect(),
            lit: check_run.map(|c| c.lit.clone()).unwrap_or_default(),
            overexposed: within(run, &r.overexposed_left_out),
            reference: stage("register")
                .and_then(|v| v["reference"].as_str())
                .map(str::to_string),
            drift: drift
                .and_then(|v| serde_json::from_value(v["frames"].clone()).ok())
                .unwrap_or_default(),
            widest: drift.and_then(|v| v["widest_shift_px"].as_f64()),
            lost: names_of(drift.map(|v| &v["lost"])),
            stars: stage("register").and_then(|v| v["stars_per_frame"][1].as_u64()),
            moved_back: r.stage("composite around the reference").is_some(),
        }
    }

    pub fn reference_index(&self) -> Option<usize> {
        let r = self.reference.as_ref()?;
        self.names.iter().position(|n| n == r)
    }
}

/// Frame `index` of the store, reduced to PREVIEW_SIDE, in the thumbnails' units and stretch.
pub fn preview(
    dir: &std::path::Path,
    index: usize,
    stretch: crate::thumbs::Stretch,
) -> Result<ColorImage> {
    let store = Store::open(dir)?;
    let (w, h) = (store.manifest.width, store.manifest.height);
    let info = store
        .manifest
        .frames
        .get(index)
        .ok_or_else(|| error("no such frame"))?;
    let step = w.max(h).div_ceil(PREVIEW_SIDE);
    let (pw, ph) = (w / step, h / step);
    let mut rgb = vec![0f32; 3 * pw * ph];
    for py in 0..ph {
        let rows = store.rgb_rows(index, py * step, (py + 1) * step)?;
        for px in 0..pw {
            for k in 0..3 {
                let mut sum = 0.0;
                for y in 0..step {
                    for x in 0..step {
                        sum += rows[3 * (y * w + px * step + x) + k];
                    }
                }
                rgb[3 * (py * pw + px) + k] =
                    sum / (step * step) as f32 / info.scale_adu * info.wb[k];
            }
        }
    }
    Ok(stretch.image(pw, ph, &rgb))
}
