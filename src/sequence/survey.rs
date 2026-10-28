//! A folder's sequence check, kept beside a run so the run can start or resume from it and
//! the app can show every frame before anything is processed: the RAW files, every capture,
//! the check's report and each frame's thumbnail (`survey.json` and `thumbs.f16`).

use super::{Capture, Report, Thumb, capture_all, check};
use crate::budget::Budget;
use crate::store::file;
use crate::{Result, error, input};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

const FILE: &str = "survey.json";
const THUMBS: &str = "thumbs.f16";

pub struct Survey {
    pub raw: PathBuf,
    pub paths: Vec<PathBuf>,
    pub captures: Vec<Capture>,
    pub check: Report,
    pub thumbs: Vec<Thumb>,
    pub seconds: f64,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    raw: PathBuf,
    captures: Vec<Capture>,
    check: Report,
    /// Width and height of each thumbnail, in frame order.
    thumbs: Vec<[usize; 2]>,
    seconds: f64,
}

impl Survey {
    /// Checks every RAW file in `raw`.
    pub fn of(raw: &Path, budget: &Budget) -> Result<Survey> {
        let clock = Instant::now();
        let paths = input::raw_files(raw)?;
        let c = capture_all(&paths, budget)?;
        let check = check(&c.captures, &c.steps)?;
        Ok(Survey {
            raw: raw.to_path_buf(),
            paths,
            captures: c.captures,
            check,
            thumbs: c.thumbs,
            seconds: clock.elapsed().as_secs_f64(),
        })
    }

    pub fn saved_in(dir: &Path) -> bool {
        dir.join(FILE).exists()
    }

    /// Writes the survey into `dir` (its files must not exist yet).
    pub fn save(&self, dir: &Path) -> Result<()> {
        let pixels = self.thumbs.iter().flat_map(|t| t.rgb.iter().copied());
        file::create(&dir.join(THUMBS), &file::u16_bytes(pixels))?;
        let saved = Saved {
            raw: self.raw.clone(),
            captures: self.captures.clone(),
            check: self.check.clone(),
            thumbs: self.thumbs.iter().map(|t| [t.width, t.height]).collect(),
            seconds: self.seconds,
        };
        file::create(&dir.join(FILE), &serde_json::to_vec_pretty(&saved)?)
    }

    /// The survey saved in `dir`.
    pub fn load(dir: &Path) -> Result<Survey> {
        let saved: Saved = serde_json::from_slice(&std::fs::read(dir.join(FILE))?)?;
        if saved.thumbs.len() != saved.captures.len() {
            return Err(error(format!(
                "{}: thumbnails and captures differ in count",
                dir.join(FILE).display()
            )));
        }
        let values = file::u16_values(&std::fs::read(dir.join(THUMBS))?);
        let mut thumbs = Vec::with_capacity(saved.thumbs.len());
        let mut at = 0;
        for &[width, height] in &saved.thumbs {
            let len = 3 * width * height;
            let rgb = values
                .get(at..at + len)
                .ok_or_else(|| error(format!("{} is shorter than its index", THUMBS)))?
                .to_vec();
            thumbs.push(Thumb { width, height, rgb });
            at += len;
        }
        Ok(Survey {
            paths: saved
                .captures
                .iter()
                .map(|c| saved.raw.join(&c.name))
                .collect(),
            raw: saved.raw,
            captures: saved.captures,
            check: saved.check,
            thumbs,
            seconds: saved.seconds,
        })
    }
}
