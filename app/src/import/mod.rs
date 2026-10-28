//! The Import step (design board 1): a RAW folder is checked in the background, its runs and
//! flags shown with thumbnails, the frames to process chosen, the lens, colour, memory and
//! disk checked, and the run started, cancelled and resumed here.

mod checks;
mod frames;
mod lookups;
mod running;
mod runs;
mod view;

pub use checks::Checks;
pub use lookups::Lookups;
pub use running::stage_label;

use crate::job::Job;
use crate::thumbs::Thumbs;
use star_photos::budget::Budget;
use star_photos::process::{self, Report};
use star_photos::sequence::{Run, Survey};
use star_photos::{Result, error, render};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Import {
    pub raw: PathBuf,
    pub survey: Option<Arc<Survey>>,
    checking: Option<Job<Survey>>,
    thumbs: Option<Thumbs>,
    /// The chosen run and its first and last frames (indices into the survey's files).
    pub run: usize,
    pub first: usize,
    pub last: usize,
    pub out: PathBuf,
    /// A lens to use when the files record none.
    pub lens: Option<String>,
    checks: Option<Checks>,
    lookups: Lookups,
    processing: Option<Job<Report>>,
    /// The report of a run in `out` that stopped before finishing.
    pub interrupted: Option<Report>,
    pub error: Option<String>,
    show_short_runs: bool,
}

/// Where a RAW folder's run goes unless another folder is chosen: `~/Pictures/Star Photos/`
/// and the folder's name (its parent's when it is called "raw"), numbered when taken.
pub fn default_out(raw: &Path) -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| error("HOME is not set; choose an output folder"))?;
    let named = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    let mut name = named(raw).unwrap_or_else(|| "photos".into());
    if name.eq_ignore_ascii_case("raw") {
        name = raw.parent().and_then(named).unwrap_or(name);
    }
    let base = PathBuf::from(home).join("Pictures").join("Star Photos");
    let mut out = base.join(&name);
    let mut k = 2;
    while out.exists() {
        out = base.join(format!("{name} {k}"));
        k += 1;
    }
    Ok(out)
}

impl Import {
    /// Checks `raw` in the background.
    pub fn raw(raw: PathBuf) -> Import {
        let (out, error) = match default_out(&raw) {
            Ok(o) => (o, None),
            Err(e) => (raw.join("star-photos"), Some(e.to_string())),
        };
        let folder = raw.clone();
        Import {
            checking: Some(Job::start("Checking the folder", move |progress| {
                let mut budget = Budget::from_free()?;
                budget.progress = progress;
                Survey::of(&folder, &budget)
            })),
            raw,
            survey: None,
            thumbs: None,
            run: 0,
            first: 0,
            last: 0,
            out,
            lens: None,
            checks: None,
            lookups: Lookups::default(),
            processing: None,
            interrupted: None,
            error,
            show_short_runs: false,
        }
    }

    /// The run in `out` that stopped, ready to resume with its frames.
    pub fn interrupted(out: PathBuf, report: Report) -> Result<Import> {
        let survey = Survey::load(&out)?;
        let index = |name: &str| {
            survey
                .paths
                .iter()
                .position(|p| p.file_name().is_some_and(|f| f == name))
                .ok_or_else(|| error(format!("{name} is not in the checked folder")))
        };
        let (first, last) = (index(&report.first)?, index(&report.last)?);
        let mut import = Import {
            raw: survey.raw.clone(),
            survey: None,
            checking: None,
            thumbs: None,
            run: report.run,
            first,
            last,
            out,
            lens: report.lens.clone(),
            checks: None,
            lookups: Lookups::default(),
            processing: None,
            interrupted: Some(report),
            error: None,
            show_short_runs: false,
        };
        import.surveyed(survey);
        Ok(import)
    }

    fn surveyed(&mut self, survey: Survey) {
        let typical = self
            .recommended_range(&survey)
            .map_or(0, |(a, b)| (a + b) / 2);
        self.thumbs = Some(Thumbs::new(survey.thumbs.clone(), typical));
        if self.interrupted.is_none() {
            self.run = survey.check.recommended;
            if let Some((a, b)) = self.recommended_range(&survey) {
                (self.first, self.last) = (a, b);
            }
        }
        self.survey = Some(Arc::new(survey));
        self.checks = None;
    }

    fn index(survey: &Survey, name: &str) -> Option<usize> {
        survey
            .paths
            .iter()
            .position(|p| p.file_name().is_some_and(|f| f == name))
    }

    /// The recommended frames of the chosen run, as indices.
    fn recommended_range(&self, survey: &Survey) -> Option<(usize, usize)> {
        let run = survey.check.runs.get(survey.check.recommended)?;
        Some((
            Self::index(survey, &run.use_first)?,
            Self::index(survey, &run.use_last)?,
        ))
    }

    pub fn chosen_run(&self) -> Option<&Run> {
        self.survey.as_ref()?.check.runs.get(self.run)
    }

    /// The run's first and last frames, as indices.
    pub fn run_range(&self) -> Option<(usize, usize)> {
        let survey = self.survey.as_ref()?;
        let run = self.chosen_run()?;
        Some((
            Self::index(survey, &run.first)?,
            Self::index(survey, &run.last)?,
        ))
    }

    /// Chooses run `k` with its recommended frames.
    pub fn choose_run(&mut self, k: usize) {
        let Some(survey) = self.survey.clone() else {
            return;
        };
        let Some(run) = survey.check.runs.get(k) else {
            return;
        };
        if let (Some(a), Some(b)) = (
            Self::index(&survey, &run.use_first),
            Self::index(&survey, &run.use_last),
        ) {
            (self.run, self.first, self.last) = (k, a, b);
            self.checks = None;
        }
    }

    pub fn set_range(&mut self, first: usize, last: usize) {
        if (first, last) != (self.first, self.last) && first <= last {
            (self.first, self.last) = (first, last);
            self.checks = None;
        }
    }

    /// The frames that will be processed: the chosen range without overexposed frames.
    pub fn selected(&self) -> Vec<usize> {
        let (Some(survey), Some(run)) = (&self.survey, self.chosen_run()) else {
            return Vec::new();
        };
        (self.first..=self.last.min(survey.paths.len().saturating_sub(1)))
            .filter(|&i| !run.overexposed.contains(&survey.captures[i].name))
            .collect()
    }

    pub fn processing(&self) -> bool {
        self.processing.is_some()
    }

    pub fn checking(&self) -> bool {
        self.checking.is_some()
    }

    /// Starts (or resumes) the run.
    pub fn start(&mut self) {
        let Some(survey) = self.survey.clone() else {
            return;
        };
        let name = |i: usize| survey.captures.get(i).map(|c| c.name.clone());
        let (Some(first), Some(last)) = (name(self.first), name(self.last)) else {
            return;
        };
        let (out, run, lens) = (self.out.clone(), self.run, self.lens.clone());
        self.error = None;
        self.processing = Some(Job::start("Processing", move |progress| {
            let mut budget = Budget::from_free()?;
            budget.progress = progress;
            process::process(
                &process::Options {
                    out: &out,
                    run: Some(run),
                    first: Some(&first),
                    last: Some(&last),
                    hero: None,
                    lens: lens.as_deref(),
                    labels: None,
                    gpu: None,
                    matrix: None,
                    look: render::NATURAL,
                },
                &survey,
                &budget,
            )
        }));
    }

    pub fn cancel(&self) {
        if let Some(job) = &self.processing {
            job.cancel();
        }
    }

    /// Advances background work; returns the output folder once a run has finished.
    pub fn poll(&mut self) -> Option<PathBuf> {
        if let Some(result) = self.checking.as_ref().and_then(Job::poll) {
            self.checking = None;
            match result {
                Ok(s) => self.surveyed(s),
                Err(e) => self.error = Some(format!("Checking the folder failed: {e}")),
            }
        }
        let result = self.processing.as_ref().and_then(Job::poll)?;
        let cancelled = self.processing.as_ref().is_some_and(Job::cancelled);
        self.processing = None;
        match result {
            Ok(_) => return Some(self.out.clone()),
            Err(_) if cancelled => self.error = None,
            Err(e) => self.error = Some(format!("Processing stopped: {e}")),
        }
        self.interrupted = Report::load(&self.out).ok().flatten();
        None
    }
}
