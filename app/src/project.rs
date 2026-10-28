//! Opening work: a `process` output folder (its `process.json` says which composite it made) or a
//! store, prepared into a scene on a background thread so the window keeps drawing; or the
//! synthetic demo scene.

use crate::scene::{self, Scene};
use crate::synthetic;
use star_photos::{Result, error};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

pub struct Project {
    pub name: String,
    /// Where comparisons are saved; none for the demo scene.
    pub dir: Option<PathBuf>,
    pub scene: Scene,
    pub facts: Vec<String>,
}

/// The store, the planes to show, and facts about the folder.
fn resolve(folder: &Path) -> Result<(PathBuf, &'static str, Vec<String>)> {
    let report = folder.join("process.json");
    if report.exists() {
        let r: serde_json::Value = serde_json::from_slice(&std::fs::read(&report)?)?;
        let stages: Vec<&str> = r["stages"]
            .as_array()
            .map(|s| s.iter().filter_map(|x| x["name"].as_str()).collect())
            .unwrap_or_default();
        let around = stages
            .iter()
            .any(|s| *s == "hero" || *s == "composite around the reference");
        let mut facts = vec![format!("{} frames", r["frames"])];
        if let (Some(a), Some(b)) = (r["first"].as_str(), r["last"].as_str()) {
            facts.push(format!("{a} to {b}"));
        }
        facts.push(
            if around {
                "composite around one frame"
            } else {
                "whole-sequence stack"
            }
            .into(),
        );
        return Ok((
            folder.join("store"),
            if around { "hero" } else { "final" },
            facts,
        ));
    }
    if folder.join("manifest.json").exists() {
        let planes = if folder.join("final.planes").exists() {
            "final"
        } else if folder.join("hero.planes").exists() {
            "hero"
        } else {
            return Err(error(
                "this store has no stacked result yet; run process or stack first",
            ));
        };
        return Ok((
            folder.to_path_buf(),
            planes,
            vec![format!("store, {planes} result")],
        ));
    }
    Err(error(format!(
        "{} is not a Star Photos folder (no process.json or manifest.json)",
        folder.display()
    )))
}

fn name(folder: &Path) -> String {
    let parts: Vec<String> = folder
        .components()
        .rev()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.into_iter().rev().collect::<Vec<_>>().join(" / ")
}

/// Prepares `folder` (or a scene file) on a background thread.
pub fn open(folder: PathBuf) -> Receiver<Result<Project>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        if folder.is_file() {
            let _ = tx.send(from_file(&folder));
            return;
        }
        let project = resolve(&folder).and_then(|(store, planes, facts)| {
            Ok(Project {
                name: name(&folder),
                dir: Some(folder.clone()),
                scene: scene::load(&store, planes)?,
                facts,
            })
        });
        let _ = tx.send(project);
    });
    rx
}

/// The synthetic demo scene at `size`, prepared on a background thread.
pub fn demo(size: [u32; 2]) -> Receiver<Result<Project>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let project = synthetic::scene(size[0], size[1]).map(|scene| Project {
            name: "Demo scene (synthetic)".into(),
            dir: None,
            scene,
            facts: vec!["generated: stars, Milky Way, treeline".into()],
        });
        let _ = tx.send(project);
    });
    rx
}

/// A scene file, wrapped as a project.
pub fn from_file(path: &Path) -> Result<Project> {
    Ok(Project {
        name: name(path),
        dir: None,
        scene: scene::open(path)?,
        facts: vec!["scene file".into()],
    })
}
