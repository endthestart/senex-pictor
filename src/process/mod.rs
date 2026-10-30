//! One command from a folder of RAW files to a finished image: check the sequence, ingest the
//! recommended run (overexposed frames left out), register, separate, stack (or build a hero
//! composite around a chosen frame) and render, every stage within one memory budget. The
//! working store, the sequence check and every stage's report stay in the output folder; a run
//! that was cancelled or failed resumes there.

mod correct;
pub mod disk;
pub mod journal;

use crate::budget::Budget;
use crate::integrate::{self, cfa, foreground, hero};
use crate::render::Look;
use crate::sequence::Survey;
use crate::store::planes;
use crate::{Result, error, gpu::Gpu, register, review, separate, store};
pub use correct::correct;
use journal::Journal;
pub use journal::{Report, Stage};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct Options<'a> {
    pub out: &'a Path,
    /// A run of the sequence check; its recommended run when absent.
    pub run: Option<usize>,
    /// The first and last frames (file names) to use within the run; its recommended frames
    /// when absent.
    pub first: Option<&'a str>,
    pub last: Option<&'a str>,
    /// A frame (file name) to build a hero composite around instead of the whole-sequence
    /// composite.
    pub hero: Option<&'a str>,
    pub lens: Option<&'a str>,
    pub labels: Option<&'a [u32]>,
    pub gpu: Option<&'a Gpu>,
    pub matrix: Option<&'a Path>,
    pub look: Look,
    /// Separation draws the matte onward again from its saved statistics (after corrections).
    pub rematte: bool,
}

/// Foreground moving more than this (px) over the sequence puts trees in different places in
/// different frames, so the whole-sequence matte no longer fits any one of them; the
/// composite is then built around the reference frame instead (`hero::Foreground::LongExposure`).
pub const DRIFT_PX: f64 = 4.0;

const CHECK: &str = "sequence check";
const STACK: &str = "stack";
const AROUND: &str = "composite around the reference";
const HERO: &str = "hero";

/// Each stage's usual share of a run: seconds per RAW file in the folder (the check) or per
/// frame processed, measured on an 8-thread Intel Mac with 24 MP frames; rendering is about
/// constant.
pub fn plan(files: usize, frames: usize, hero: bool) -> Vec<(&'static str, f64)> {
    let f = frames as f64;
    let mut stages = vec![
        (CHECK, 0.45 * files as f64),
        ("ingest", 1.4 * f),
        ("register", 0.2 * f),
        ("separate", 0.7 * f),
    ];
    if hero {
        stages.push((HERO, 0.6 * f));
    } else {
        stages.push(("foreground drift", 0.4 * f));
        stages.push((STACK, 0.6 * f));
    }
    stages.push(("render", 10.0));
    stages
}

#[derive(Serialize)]
struct Drift {
    widest_shift_px: f64,
    /// Each tracked frame's median foreground motion from the reference (px).
    frames: Vec<(String, f64)>,
    lost: Vec<String>,
}

/// How far the foreground moved from the reference frame (the largest median corner motion).
fn drift(store: &store::Store, budget: &Budget) -> Result<Drift> {
    let model = separate::model(store)?;
    let alpha = planes::load(&store.dir.join("matte.planes"))?;
    let ground: Vec<bool> = alpha.get("alpha")?.iter().map(|&a| a <= 0.5).collect();
    let tracked = foreground::track(store, &ground, &model.camera, model.reference, budget)?;
    let name = |i: usize| store.manifest.frames[i].name.clone();
    let mut frames: Vec<(String, f64)> = tracked
        .fields
        .iter()
        .map(|(i, d)| (name(*i), d.median_px[0].hypot(d.median_px[1])))
        .collect();
    frames.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(Drift {
        widest_shift_px: foreground::drift(&tracked, &register::times(store)),
        frames,
        lost: tracked.lost.iter().map(|&i| name(i)).collect(),
    })
}

/// A composite named `name` around frame `hero` over every frame, with `foreground`.
pub fn around<'b>(
    o: &Options<'b>,
    hero: usize,
    foreground: hero::Foreground<'b>,
    name: &'b str,
) -> hero::Options<'b> {
    hero::Options {
        hero,
        foreground,
        first: 0,
        labels: o.labels,
        lens: o.lens,
        kernel: cfa::KERNEL,
        gpu: o.gpu,
        name,
    }
}

/// The name of the result composed around `frame` (a file name).
pub fn hero_name(frame: &str) -> String {
    format!("hero-{}", frame.split('.').next().unwrap_or(frame))
}

/// The survey of `raw` kept in `out` by an interrupted run, or a new one.
pub fn survey_for(raw: &Path, out: &Path, budget: &Budget) -> Result<Survey> {
    if Survey::saved_in(out) {
        let s = Survey::load(out)?;
        if s.raw != raw {
            return Err(error(format!(
                "{} holds a run of {}, not {}",
                out.display(),
                s.raw.display(),
                raw.display()
            )));
        }
        return Ok(s);
    }
    Survey::of(raw, budget)
}

pub fn process(o: &Options, survey: &Survey, budget: &Budget) -> Result<Report> {
    let check = &survey.check;
    let index = o.run.unwrap_or(check.recommended);
    let run = check.runs.get(index).ok_or_else(|| {
        error(format!(
            "the sequence check found {} runs; --run {index} is not one",
            check.runs.len()
        ))
    })?;
    let (first, last) = (
        o.first.unwrap_or(&run.use_first),
        o.last.unwrap_or(&run.use_last),
    );
    let over = |p: &PathBuf| {
        run.overexposed
            .iter()
            .any(|n| p.file_name().is_some_and(|f| f == n.as_str()))
    };
    let chosen = disk::names_between(&survey.paths, first, last)?;
    let left_out: Vec<String> = chosen
        .iter()
        .filter(|p| over(p))
        .filter_map(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()))
        .collect();
    let selected: Vec<PathBuf> = chosen.into_iter().filter(|p| !over(p)).collect();
    if let Some(name) = o.hero
        && !selected
            .iter()
            .any(|p| p.file_name().is_some_and(|f| f == name))
    {
        return Err(error(format!(
            "the hero frame {name} is not among the frames of run {index}"
        )));
    }
    budget
        .progress
        .plan(&plan(survey.paths.len(), selected.len(), o.hero.is_some()));
    std::fs::create_dir_all(o.out)?;
    let fresh = Report {
        complete: false,
        raw: survey.raw.clone(),
        budget: budget.clone(),
        device: o.gpu.map_or_else(|| "CPU".into(), |g| g.name.clone()),
        run: index,
        first: first.to_string(),
        last: last.to_string(),
        hero: o.hero.map(str::to_string),
        lens: o.lens.map(str::to_string),
        frames: selected.len(),
        overexposed_left_out: left_out,
        stages: Vec::new(),
        running: None,
        result: None,
        image: Vec::new(),
    };
    let report = journal::continue_in(o.out, fresh)?;
    if report.stages.is_empty() && report.running.is_none() {
        disk::room_for(o.out, &selected)?;
    }
    if !Survey::saved_in(o.out) {
        survey.save(o.out)?;
    }
    let mut log = Journal::open(o.out, report, budget.progress.clone())?;
    log.done(CHECK, survey.seconds, check)?;
    log.record("ingest", || {
        let s = store::ingest(&selected, &o.out.join("store"), budget)?;
        let m = &s.manifest;
        Ok(serde_json::json!({"frames": m.frames.len(), "width": m.width, "height": m.height}))
    })?;
    let store = store::Store::open(&o.out.join("store"))?;
    log.record("register", || register::register(&store, None, budget))?;
    log.record("separate", || {
        if o.rematte {
            separate::rematte(&store, o.labels, o.lens, budget)
        } else {
            separate::separate(&store, o.labels, o.lens, budget)
        }
    })?;
    let frame = |name: &str| {
        store
            .manifest
            .frames
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| error(format!("{name} is not among the processed frames")))
    };
    let lit: Vec<usize> = run.lit.iter().filter_map(|n| frame(n).ok()).collect();
    let planes = match o.hero {
        Some(name) => {
            let result = hero_name(name);
            let options = around(o, frame(name)?, hero::Foreground::Frame, &result);
            log.record(HERO, || hero::hero(&store, &options, budget))?;
            result
        }
        None => {
            let drift = log.record("foreground drift", || drift(&store, budget))?;
            let shift = drift["widest_shift_px"]
                .as_f64()
                .ok_or_else(|| error("the drift report has no widest shift"))?;
            if shift > DRIFT_PX {
                budget.progress.rename(STACK, AROUND);
                let reference = separate::model(&store)?.reference;
                let options = around(
                    o,
                    reference,
                    hero::Foreground::LongExposure { lit: &lit },
                    "hero",
                );
                log.record(AROUND, || hero::hero(&store, &options, budget))?;
                "hero".to_string()
            } else {
                let options = integrate::Options {
                    kernel: cfa::KERNEL,
                    gpu: o.gpu,
                    lit: &lit,
                };
                log.record(STACK, || integrate::stack(&store, &options, budget))?;
                "final".to_string()
            }
        }
    };
    let stem = o.out.join(match o.hero {
        Some(_) => planes.as_str(),
        None => "final",
    });
    let rendered = log.record("render", || {
        review::render(
            &store,
            &review::Options {
                planes: &planes,
                look: o.look,
                matrix: o.matrix,
                out: &stem,
                jpeg: Some(review::SIDE),
                tiff: true,
                single: false,
                linear: false,
                hdr: None,
            },
        )
    })?;
    log.report.image = serde_json::from_value(rendered["files"].clone())?;
    log.report.result = Some(planes);
    log.report.complete = true;
    log.save()?;
    Ok(log.report)
}
