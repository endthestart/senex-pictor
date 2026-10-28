//! Store stage commands: options parsed into each stage's call.

use star_photos::budget::Budget;
use star_photos::integrate::{self, cfa, hero, moment};
use star_photos::store::{Store, planes};
use star_photos::{Result, error, gpu::Gpu, register, render, review, separate};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;

/// `--name value` pairs from `rest`, each name one of `allowed`.
pub fn flags(rest: &[OsString], allowed: &[&str]) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    let mut items = rest.iter();
    while let Some(name) = items.next() {
        let name = name.to_string_lossy().into_owned();
        if !allowed.contains(&name.as_str()) {
            return Err(error(format!("unknown option {name}")));
        }
        let value = items
            .next()
            .ok_or_else(|| error(format!("{name} needs a value")))?;
        out.insert(name, value.to_string_lossy().into_owned());
    }
    Ok(out)
}

/// Segment labels from a planes file's "labels" plane.
fn labels_from(path: Option<&String>) -> Result<Option<Vec<u32>>> {
    path.map(|p| {
        Ok(planes::load(Path::new(p))?
            .get("labels")?
            .iter()
            .map(|&v| v as u32)
            .collect())
    })
    .transpose()
}

/// The index of frame `name` (a file name) in the store.
pub fn named(store: &Store, name: &str) -> Result<usize> {
    store
        .manifest
        .frames
        .iter()
        .position(|f| f.name == name)
        .ok_or_else(|| error(format!("{name} is not in the store")))
}

/// `--lit NAME,NAME…`: frames lit by local light, left out of a long exposure.
fn lit(store: &Store, options: &HashMap<String, String>) -> Result<Vec<usize>> {
    options.get("--lit").map_or(Ok(Vec::new()), |names| {
        names.split(',').map(|n| named(store, n)).collect()
    })
}

fn frame(store: &Store, options: &HashMap<String, String>, name: &str) -> Result<usize> {
    let wanted = options
        .get(name)
        .ok_or_else(|| error(format!("{name} FRAME_NAME is required")))?;
    named(store, wanted)
}

/// `--gpu`: no (the default), yes (the most capable adapter) or part of an adapter's name.
pub fn gpu(options: &HashMap<String, String>) -> Result<Option<Gpu>> {
    match options.get("--gpu").map(String::as_str) {
        None | Some("no") => Ok(None),
        Some("yes") => Ok(Some(Gpu::new(None)?)),
        Some(name) => Ok(Some(Gpu::new(Some(name))?)),
    }
}

fn json<T: serde::Serialize>(value: T) -> Result<serde_json::Value> {
    Ok(serde_json::to_value(value)?)
}

/// register, separate, stack, hero or moment on the store at `dir`.
pub fn stage(
    command: &str,
    dir: &Path,
    rest: &[OsString],
    budget: &Budget,
) -> Result<serde_json::Value> {
    let store = Store::open(dir)?;
    let lens = |o: &HashMap<String, String>| o.get("--lens").cloned();
    match command {
        "register" => {
            let o = flags(rest, &["--lens-from"])?;
            json(register::register(
                &store,
                o.get("--lens-from").map(Path::new),
                budget,
            )?)
        }
        "separate" => {
            let o = flags(rest, &["--labels", "--lens"])?;
            let labels = labels_from(o.get("--labels"))?;
            json(separate::separate(
                &store,
                labels.as_deref(),
                lens(&o).as_deref(),
                budget,
            )?)
        }
        "stack" => {
            let o = flags(rest, &["--sigma", "--radius", "--gpu", "--lit"])?;
            let number = |name: &str, default: f64| -> Result<f64> {
                o.get(name).map_or(Ok(default), |v| {
                    v.parse().map_err(|e| error(format!("{name}: {e}")))
                })
            };
            let kernel = cfa::Kernel {
                sigma: number("--sigma", cfa::KERNEL.sigma)?,
                radius: number("--radius", cfa::KERNEL.radius)?,
            };
            let lit = lit(&store, &o)?;
            let device = gpu(&o)?;
            let options = integrate::Options {
                kernel,
                gpu: device.as_ref(),
                lit: &lit,
            };
            json(integrate::stack(&store, &options, budget)?)
        }
        "hero" => {
            let o = flags(
                rest,
                &[
                    "--hero",
                    "--first",
                    "--labels",
                    "--lens",
                    "--gpu",
                    "--foreground",
                    "--lit",
                ],
            )?;
            let labels = labels_from(o.get("--labels"))?;
            let device = gpu(&o)?;
            let lens = lens(&o);
            let lit = lit(&store, &o)?;
            let foreground = match o.get("--foreground").map(String::as_str) {
                None | Some("frame") => hero::Foreground::Frame,
                Some("long") => hero::Foreground::LongExposure { lit: &lit },
                Some(other) => return Err(error(format!("--foreground {other}: frame or long"))),
            };
            let options = hero::Options {
                hero: frame(&store, &o, "--hero")?,
                foreground,
                first: frame(&store, &o, "--first")?,
                labels: labels.as_deref(),
                lens: lens.as_deref(),
                kernel: cfa::KERNEL,
                gpu: device.as_ref(),
                name: "hero",
            };
            json(hero::hero(&store, &options, budget)?)
        }
        "moment" => {
            let o = flags(rest, &["--centre", "--half"])?;
            let half = o.get("--half").map_or(Ok(moment::HALF), |v| v.parse())?;
            json(moment::moment(
                &store,
                frame(&store, &o, "--centre")?,
                half,
                budget,
            )?)
        }
        other => Err(error(format!("{other} is not a store stage"))),
    }
}

fn look(options: &HashMap<String, String>) -> Result<render::Look> {
    match options.get("--look").map(String::as_str) {
        None | Some("natural") => Ok(render::NATURAL),
        Some("dark") => Ok(render::DARK),
        Some(other) => Err(error(format!("unknown look {other} (natural, dark)"))),
    }
}

/// `process RAW_DIR OUT_DIR` with its options.
pub fn process(
    raw: &Path,
    out: &Path,
    rest: &[OsString],
    budget: &Budget,
) -> Result<star_photos::process::Report> {
    let o = flags(
        rest,
        &[
            "--run", "--first", "--last", "--hero", "--lens", "--labels", "--gpu", "--matrix",
            "--look",
        ],
    )?;
    let labels = labels_from(o.get("--labels"))?;
    let device = gpu(&o)?;
    let run = o
        .get("--run")
        .map(|v| v.parse().map_err(|e| error(format!("--run: {e}"))))
        .transpose()?;
    let survey = star_photos::process::survey_for(raw, out, budget)?;
    star_photos::process::process(
        &star_photos::process::Options {
            out,
            run,
            first: o.get("--first").map(String::as_str),
            last: o.get("--last").map(String::as_str),
            hero: o.get("--hero").map(String::as_str),
            lens: o.get("--lens").map(String::as_str),
            labels: labels.as_deref(),
            gpu: device.as_ref(),
            matrix: o.get("--matrix").map(Path::new),
            look: look(&o)?,
        },
        &survey,
        budget,
    )
}

pub fn render(dir: &Path, planes: &str, out: &Path, rest: &[OsString]) -> Result<review::Report> {
    let store = Store::open(dir)?;
    let o = flags(
        rest,
        &["--matrix", "--look", "--side", "--single", "--linear"],
    )?;
    let yes = |name: &str| o.get(name).is_some_and(|v| v == "yes");
    review::render(
        &store,
        &review::Options {
            planes,
            look: look(&o)?,
            matrix: o.get("--matrix").map(Path::new),
            out,
            jpeg: Some(o.get("--side").map_or(Ok(review::SIDE), |v| v.parse())?),
            tiff: true,
            single: yes("--single"),
            linear: yes("--linear"),
        },
    )
}
