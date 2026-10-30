//! A finished run done again from a later stage: from its matte after the owner corrected it
//! (`corrections.json` in the store; separation draws the matte, glow and occlusion again from
//! the saved statistics), or from its composite with moving foreground blurred or sharp. The
//! run is reopened at that stage and the rest follows, rendered with the run's own look.

use super::{Options, Report, journal, process};
use crate::budget::Budget;
use crate::render::Look;
use crate::sequence::Survey;
use crate::{Result, error, gpu::Gpu};
use std::path::Path;

/// What a finished run does again.
#[derive(Clone, Copy, Debug)]
pub enum Redo {
    /// The matte onward, with the store's corrections.
    Matte,
    /// The composite onward, with moving foreground sharp (from the reference minutes) or
    /// blurred (the long exposure).
    Composite { sharp_moving: bool },
}

/// The stages that make a run's composite, one of which every finished run has.
const COMPOSITES: [&str; 3] = ["stack", "composite around the reference", "hero"];

pub fn redo(out: &Path, what: Redo, gpu: Option<&Gpu>, budget: &Budget) -> Result<Report> {
    let before =
        Report::load(out)?.ok_or_else(|| error(format!("{} holds no run", out.display())))?;
    let look: Look = before
        .stage("render")
        .map(|s| serde_json::from_value(s.report["look"].clone()))
        .transpose()?
        .ok_or_else(|| error(format!("the run in {} was never rendered", out.display())))?;
    let survey = Survey::load(out)?;
    let r = match what {
        Redo::Matte => journal::reopen(out, "separate", |_| {})?,
        Redo::Composite { sharp_moving } => {
            let from = COMPOSITES
                .into_iter()
                .find(|s| before.stage(s).is_some())
                .ok_or_else(|| error(format!("the run in {} has no composite", out.display())))?;
            journal::reopen(out, from, |r| r.sharp_moving = sharp_moving)?
        }
    };
    process(
        &Options {
            out,
            run: Some(r.run),
            first: Some(&r.first),
            last: Some(&r.last),
            hero: r.hero.as_deref(),
            lens: r.lens.as_deref(),
            labels: None,
            gpu,
            matrix: None,
            look,
            rematte: matches!(what, Redo::Matte),
            sharp_moving: r.sharp_moving,
        },
        &survey,
        budget,
    )
}
