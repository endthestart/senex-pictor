//! One command from a folder of RAW files to a finished image: check the sequence, ingest the
//! recommended run (overexposed frames left out), register, separate, stack (or build a hero
//! composite around a chosen frame) and render, every stage within one memory budget. The
//! working store and every stage's report stay in the output folder.

use crate::budget::Budget;
use crate::integrate::{self, cfa, foreground, hero};
use crate::render::Look;
use crate::store::planes;
use crate::{Result, error, gpu::Gpu, input, register, review, separate, sequence, store};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct Options<'a> {
    pub raw: &'a Path,
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
}

#[derive(Debug, Serialize)]
pub struct Stage {
    pub name: &'static str,
    pub seconds: f64,
    pub report: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub budget: Budget,
    pub device: String,
    pub run: usize,
    pub frames: usize,
    pub first: String,
    pub last: String,
    pub overexposed_left_out: Vec<String>,
    pub stages: Vec<Stage>,
    pub image: Vec<String>,
}

struct Log {
    clock: Instant,
    stages: Vec<Stage>,
}

impl Log {
    fn record<T: Serialize>(
        &mut self,
        name: &'static str,
        run: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        eprintln!("star-photos: {name}…");
        let start = Instant::now();
        let value = run().map_err(|e| error(format!("{name}: {e}")))?;
        let seconds = start.elapsed().as_secs_f64();
        eprintln!(
            "star-photos: {name} done in {seconds:.0} s ({:.0} s so far)",
            self.clock.elapsed().as_secs_f64()
        );
        self.stages.push(Stage {
            name,
            seconds,
            report: serde_json::to_value(&value)?,
        });
        Ok(value)
    }
}

/// Foreground moving more than this (px) over the sequence puts trees in different places in
/// different frames, so the whole-sequence matte no longer fits any one of them; the
/// composite is then built around the reference frame instead (`hero::Foreground::LongExposure`).
pub const DRIFT_PX: f64 = 4.0;

#[derive(Serialize)]
struct Drift {
    widest_shift_px: f64,
}

/// How far the foreground moved from the reference frame (the largest median corner motion).
fn drift(store: &store::Store, budget: &Budget) -> Result<Drift> {
    let model = separate::model(store)?;
    let alpha = planes::load(&store.dir.join("matte.planes"))?;
    let ground: Vec<bool> = alpha.get("alpha")?.iter().map(|&a| a <= 0.5).collect();
    let tracked = foreground::track(store, &ground, &model.camera, model.reference, budget)?;
    Ok(Drift {
        widest_shift_px: foreground::drift(&tracked, &register::times(store)),
    })
}

/// A composite around frame `hero` over every frame, with `foreground`.
fn around<'b>(o: &Options<'b>, hero: usize, foreground: hero::Foreground<'b>) -> hero::Options<'b> {
    hero::Options {
        hero,
        foreground,
        first: 0,
        labels: o.labels,
        lens: o.lens,
        kernel: cfa::KERNEL,
        gpu: o.gpu,
    }
}

/// Bytes a store keeps per frame pixel (RGB as 16-bit floats, CFA samples, saturation and
/// occlusion bits) and per pixel of the whole sequence (statistics, matte, stacks).
const STORE_BYTES_PER_FRAME_PIXEL: f64 = 8.4;
const STORE_BYTES_PER_PIXEL: f64 = 220.0;

/// Fails unless the volume holding `out` has room for the store of `frames`.
fn room_for(out: &Path, frames: &[PathBuf]) -> Result<()> {
    let first = frames.first().ok_or_else(|| error("no frames selected"))?;
    let [w, h] = input::inspect(first)?.sensor;
    let pixels = (w * h) as f64;
    let need = pixels * (frames.len() as f64 * STORE_BYTES_PER_FRAME_PIXEL + STORE_BYTES_PER_PIXEL);
    let out = out.canonicalize()?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .filter(|d| out.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .ok_or_else(|| error(format!("no volume found for {}", out.display())))?;
    let free = disk.available_space() as f64;
    if need > free {
        return Err(error(format!(
            "{} frames need about {:.0} GB in {} and its volume has {:.0} GB free; \
             choose fewer frames (--first, --last) or another output folder",
            frames.len(),
            need / 1e9,
            out.display(),
            free / 1e9
        )));
    }
    Ok(())
}

fn names_between(paths: &[PathBuf], first: &str, last: &str) -> Result<Vec<PathBuf>> {
    let index = |name: &str| {
        paths
            .iter()
            .position(|p| p.file_name().is_some_and(|f| f == name))
            .ok_or_else(|| error(format!("{name} is not in the folder")))
    };
    Ok(paths[index(first)?..=index(last)?].to_vec())
}

pub fn process(o: &Options, budget: &Budget) -> Result<Report> {
    if o.out.join("store").exists() {
        return Err(error(format!(
            "{} already holds a processed store; choose a new output folder",
            o.out.display()
        )));
    }
    std::fs::create_dir_all(o.out)?;
    let mut log = Log {
        clock: Instant::now(),
        stages: Vec::new(),
    };
    let paths = input::raw_files(o.raw)?;
    let check = log.record("sequence check", || {
        let (captures, steps) = sequence::capture_all(&paths, budget)?;
        sequence::check(&captures, &steps)
    })?;
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
    let selected: Vec<PathBuf> = names_between(&paths, first, last)?
        .into_iter()
        .filter(|p| {
            !run.overexposed
                .iter()
                .any(|n| p.file_name().is_some_and(|f| f == n.as_str()))
        })
        .collect();
    if let Some(name) = o.hero
        && !selected
            .iter()
            .any(|p| p.file_name().is_some_and(|f| f == name))
    {
        return Err(error(format!(
            "the hero frame {name} is not among the frames of run {index}"
        )));
    }
    room_for(o.out, &selected)?;
    log.record("ingest", || {
        let s = store::ingest(&selected, &o.out.join("store"), budget)?;
        let m = &s.manifest;
        Ok(serde_json::json!({"frames": m.frames.len(), "width": m.width, "height": m.height}))
    })?;
    let store = store::Store::open(&o.out.join("store"))?;
    log.record("register", || register::register(&store, None, budget))?;
    log.record("separate", || {
        separate::separate(&store, o.labels, o.lens, budget)
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
            let options = around(o, frame(name)?, hero::Foreground::Frame);
            log.record("hero", || hero::hero(&store, &options, budget))?;
            "hero"
        }
        None => {
            let drift = log.record("foreground drift", || drift(&store, budget))?;
            if drift.widest_shift_px > DRIFT_PX {
                let reference = separate::model(&store)?.reference;
                let options = around(o, reference, hero::Foreground::LongExposure { lit: &lit });
                log.record("composite around the reference", || {
                    hero::hero(&store, &options, budget)
                })?;
                "hero"
            } else {
                let options = integrate::Options {
                    kernel: cfa::KERNEL,
                    gpu: o.gpu,
                    lit: &lit,
                };
                log.record("stack", || integrate::stack(&store, &options, budget))?;
                "final"
            }
        }
    };
    let stem = o.out.join(match o.hero {
        Some(name) => format!("hero-{}", name.split('.').next().unwrap_or(name)),
        None => "final".into(),
    });
    let rendered = log.record("render", || {
        review::render(
            &store,
            &review::Options {
                planes,
                look: o.look,
                matrix: o.matrix,
                out: &stem,
                side: review::SIDE,
                single: false,
                linear: false,
            },
        )
    })?;
    let report = Report {
        budget: *budget,
        device: o.gpu.map_or_else(|| "CPU".into(), |g| g.name.clone()),
        run: index,
        frames: store.manifest.frames.len(),
        first: first.to_string(),
        last: last.to_string(),
        overexposed_left_out: run.overexposed.clone(),
        stages: log.stages,
        image: rendered.files,
    };
    std::fs::write(
        o.out.join("process.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
