//! The run's report, written to `process.json` before and after every stage, so a run that
//! was cancelled or failed resumes where it stopped: finished stages are kept and the stage
//! that was running is rolled back (whatever it added to the store is removed) and run again.

use crate::budget::Budget;
use crate::progress::Progress;
use crate::{Result, error};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const FILE: &str = "process.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stage {
    pub name: String,
    pub seconds: f64,
    pub report: serde_json::Value,
}

/// The stage in progress and what the store held when it started.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Running {
    pub stage: String,
    pub store_before: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub complete: bool,
    pub raw: PathBuf,
    pub budget: Budget,
    pub device: String,
    pub run: usize,
    pub first: String,
    pub last: String,
    pub hero: Option<String>,
    pub lens: Option<String>,
    pub frames: usize,
    pub overexposed_left_out: Vec<String>,
    pub stages: Vec<Stage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<Running>,
    /// The result's planes in the store (`final`, `hero`, `hero-{frame}`), once rendered.
    pub result: Option<String>,
    pub image: Vec<String>,
}

impl Report {
    /// The report in `out`, if a run has started there.
    pub fn load(out: &Path) -> Result<Option<Report>> {
        let path = out.join(FILE);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&std::fs::read(path)?)?))
    }

    pub fn stage(&self, name: &str) -> Option<&Stage> {
        self.stages.iter().find(|s| s.name == name)
    }
}

/// The report to continue in `out`: `fresh`, or that of an interrupted run of the same frames.
pub fn continue_in(out: &Path, fresh: Report) -> Result<Report> {
    match Report::load(out)? {
        None if out.join("store").exists() => Err(error(format!(
            "{} already holds a processed store; choose a new output folder",
            out.display()
        ))),
        None => Ok(fresh),
        Some(r) if r.complete => Err(error(format!(
            "{} already holds a finished run; choose a new output folder",
            out.display()
        ))),
        Some(r) => {
            let same = r.raw == fresh.raw
                && (r.run, &r.first, &r.last, &r.hero, &r.lens)
                    == (
                        fresh.run,
                        &fresh.first,
                        &fresh.last,
                        &fresh.hero,
                        &fresh.lens,
                    );
            if !same {
                return Err(error(format!(
                    "{} holds an interrupted run of other frames; resume it with the same \
                     frames or choose a new output folder",
                    out.display()
                )));
            }
            eprintln!("pictor: resuming the run in {}", out.display());
            Ok(r)
        }
    }
}

pub struct Journal<'a> {
    out: &'a Path,
    progress: Progress,
    clock: Instant,
    pub report: Report,
}

/// Every file and folder under `dir` (relative to it), sorted; none when it does not exist.
fn listing(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        if !d.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&d)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path.clone());
            }
            out.push(path.strip_prefix(dir)?.to_path_buf());
        }
    }
    out.sort();
    Ok(out)
}

impl<'a> Journal<'a> {
    /// A journal for `report` in `out`. An interrupted stage is rolled back: everything it
    /// added under `out/store` is removed, deepest first.
    pub fn open(out: &'a Path, mut report: Report, progress: Progress) -> Result<Journal<'a>> {
        if let Some(running) = report.running.take() {
            let store = out.join("store");
            let mut added: Vec<PathBuf> = listing(&store)?
                .into_iter()
                .filter(|p| running.store_before.binary_search(p).is_err())
                .collect();
            added.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
            for path in added {
                let full = store.join(path);
                if full.is_dir() {
                    std::fs::remove_dir(&full)?;
                } else {
                    std::fs::remove_file(&full)?;
                }
            }
            if running.store_before.is_empty() && store.exists() {
                std::fs::remove_dir(&store)?;
            }
            eprintln!("pictor: rolled back the interrupted {}", running.stage);
        }
        for s in &report.stages {
            progress.resumed(&s.name, s.seconds);
        }
        let journal = Journal {
            out,
            progress,
            clock: Instant::now(),
            report,
        };
        journal.save()?;
        Ok(journal)
    }

    pub fn save(&self) -> Result<()> {
        let tmp = self.out.join(format!("{FILE}.tmp"));
        std::fs::write(&tmp, serde_json::to_vec_pretty(&self.report)?)?;
        Ok(std::fs::rename(tmp, self.out.join(FILE))?)
    }

    /// A stage done before the journal (the sequence check), unless already recorded.
    pub fn done(&mut self, name: &str, seconds: f64, report: &impl Serialize) -> Result<()> {
        if self.report.stage(name).is_none() {
            self.report.stages.push(Stage {
                name: name.into(),
                seconds,
                report: serde_json::to_value(report)?,
            });
            self.progress.finish(name, seconds);
            self.save()?;
        }
        Ok(())
    }

    /// Runs stage `name` and records its report, or returns the report an earlier run of this
    /// folder recorded for it.
    pub fn record<T: Serialize>(
        &mut self,
        name: &str,
        run: impl FnOnce() -> Result<T>,
    ) -> Result<serde_json::Value> {
        if let Some(s) = self.report.stage(name) {
            return Ok(s.report.clone());
        }
        self.report.running = Some(Running {
            stage: name.into(),
            store_before: listing(&self.out.join("store"))?,
        });
        self.save()?;
        eprintln!("pictor: {name}…");
        self.progress.begin(name);
        let start = Instant::now();
        let value = run().map_err(|e| {
            if self.progress.is_cancelled() {
                e
            } else {
                error(format!("{name}: {e}"))
            }
        })?;
        let seconds = start.elapsed().as_secs_f64();
        eprintln!(
            "pictor: {name} done in {seconds:.0} s ({:.0} s so far)",
            self.clock.elapsed().as_secs_f64()
        );
        self.progress.finish(name, seconds);
        let report = serde_json::to_value(&value)?;
        self.report.stages.push(Stage {
            name: name.into(),
            seconds,
            report: report.clone(),
        });
        self.report.running = None;
        self.save()?;
        Ok(report)
    }
}
