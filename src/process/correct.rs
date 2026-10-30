//! A finished run again after the owner corrected its matte (`corrections.json` in the
//! store): the run is reopened at separation, which draws the matte, glow and occlusion again
//! from the saved statistics with the corrections laid over, and the stack (or composite)
//! and render follow with the look the run was rendered with.

use super::{Options, Report, journal, process};
use crate::budget::Budget;
use crate::render::Look;
use crate::sequence::Survey;
use crate::{Result, error, gpu::Gpu};
use std::path::Path;

pub fn correct(out: &Path, gpu: Option<&Gpu>, budget: &Budget) -> Result<Report> {
    let before =
        Report::load(out)?.ok_or_else(|| error(format!("{} holds no run", out.display())))?;
    let look: Look = before
        .stage("render")
        .map(|s| serde_json::from_value(s.report["look"].clone()))
        .transpose()?
        .ok_or_else(|| error(format!("the run in {} was never rendered", out.display())))?;
    let survey = Survey::load(out)?;
    let r = journal::reopen(out, "separate")?;
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
            rematte: true,
        },
        &survey,
        budget,
    )
}
