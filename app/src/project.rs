//! What a folder or file the user opens holds, and loading a result for the viewer: a folder
//! of RAW files (to check and process), an output folder whose run stopped (to resume), a
//! processed output folder or a bare store (to review), or a scene file (to view elsewhere).

use crate::scene::{self, Scene};
use crate::synthetic;
use pictor_engine::process::Report;
use pictor_engine::sequence::Survey;
use pictor_engine::store::Store;
use pictor_engine::{Result, error, input};
use std::path::{Path, PathBuf};

pub enum Kind {
    Raw(PathBuf),
    /// An output folder and the report of its unfinished run.
    Interrupted(PathBuf, Box<Report>),
    Processed(PathBuf),
    Store(PathBuf),
    SceneFile(PathBuf),
}

pub fn classify(path: &Path) -> Result<Kind> {
    if path.is_file() {
        return if path.extension().is_some_and(|x| x == "scene") {
            Ok(Kind::SceneFile(path.into()))
        } else {
            Err(error(format!(
                "{} is a file; open a folder of RAW files, an output folder or a .scene file",
                path.display()
            )))
        };
    }
    if let Some(report) = Report::load(path).map_err(|e| {
        error(format!(
            "{}: its process.json is from another version of Pictor ({e}); process the \
             RAW folder again",
            path.display()
        ))
    })? {
        return Ok(if report.complete {
            Kind::Processed(path.into())
        } else {
            Kind::Interrupted(path.into(), Box::new(report))
        });
    }
    if path.join("manifest.json").exists() {
        return Ok(Kind::Store(path.into()));
    }
    if !input::raw_files(path)?.is_empty() {
        return Ok(Kind::Raw(path.into()));
    }
    Err(error(format!(
        "{} holds no RAW files and no Pictor results",
        path.display()
    )))
}

/// A processed output folder: its report, the sequence check and the store.
pub struct Run {
    pub out: PathBuf,
    pub report: Report,
    pub survey: Survey,
    pub store: Store,
}

impl Run {
    pub fn open(out: &Path) -> Result<Run> {
        let report = Report::load(out)?
            .ok_or_else(|| error(format!("{} has no process.json", out.display())))?;
        Ok(Run {
            out: out.into(),
            survey: Survey::load(out)?,
            store: Store::open(&out.join("store"))?,
            report,
        })
    }

    /// Results in the store: `final`, `hero`, `hero-…`, `moment-…`.
    pub fn results(&self) -> Result<Vec<String>> {
        let mut names: Vec<String> = std::fs::read_dir(&self.store.dir)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let stem = name.strip_suffix(".planes")?.to_string();
                let result = stem == "final"
                    || stem == "hero"
                    || stem.starts_with("hero-")
                    || stem.starts_with("moment-");
                result.then_some(stem)
            })
            .collect();
        names.sort();
        Ok(names)
    }
}

/// What the viewer opens: a result's scene, with the run it came from when there is one.
pub struct Loaded {
    pub name: String,
    pub run: Option<Run>,
    /// The result's planes name in the store, when it comes from one.
    pub result: Option<String>,
    pub scene: Scene,
}

/// "maplewood-2025-09-26 / process / final": the last three parts of `path`.
pub fn name(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .rev()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.into_iter().rev().collect::<Vec<_>>().join(" / ")
}

/// A processed folder's result `result` (its own when none is given).
pub fn load_run(out: &Path, result: Option<String>) -> Result<Loaded> {
    let run = Run::open(out)?;
    let result = match result.or_else(|| run.report.result.clone()) {
        Some(r) => r,
        None => return Err(error(format!("{} names no result", out.display()))),
    };
    let scene = scene::load(&run.store.dir, &result)?;
    Ok(Loaded {
        name: name(out),
        run: Some(run),
        result: Some(result),
        scene,
    })
}

/// A bare store's stacked result.
pub fn load_store(dir: &Path) -> Result<Loaded> {
    let planes = if dir.join("final.planes").exists() {
        "final"
    } else if dir.join("hero.planes").exists() {
        "hero"
    } else {
        return Err(error(
            "this store has no stacked result yet; run process or stack first",
        ));
    };
    Ok(Loaded {
        name: name(dir),
        run: None,
        result: Some(planes.into()),
        scene: scene::load(dir, planes)?,
    })
}

pub fn load_scene_file(path: &Path) -> Result<Loaded> {
    Ok(Loaded {
        name: name(path),
        run: None,
        result: None,
        scene: scene::open(path)?,
    })
}

pub fn demo(size: [u32; 2]) -> Result<Loaded> {
    Ok(Loaded {
        name: "Demo scene (synthetic)".into(),
        run: None,
        result: None,
        scene: synthetic::scene(size[0], size[1])?,
    })
}
