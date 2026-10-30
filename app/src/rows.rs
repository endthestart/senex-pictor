//! What the frame says about the app's state: the header's name and chips, each step's state
//! in a word or two, and the status line.

use crate::app::{Pictor, Workspace};
use crate::import::Import;
use crate::session::Session;
use crate::shell::{State, Step, StepRow};
use crate::widgets::Icon;

pub fn header(app: &Pictor) -> (String, Vec<String>) {
    let name = match &app.workspace {
        Workspace::Empty | Workspace::Loading(_) => String::new(),
        Workspace::Import(i) => crate::project::name(&i.raw),
        Workspace::Open(s) => s.name.clone(),
    };
    (name, app.chips.clone())
}

pub fn status(app: &Pictor) -> (Option<Icon>, String) {
    match &app.workspace {
        Workspace::Empty => (None, "Open a folder of RAW files to begin".into()),
        Workspace::Loading(job) => (None, format!("{}…", job.label)),
        Workspace::Import(i) => (None, i.status()),
        Workspace::Open(s) => session_status(s),
    }
}

fn session_status(s: &Session) -> (Option<Icon>, String) {
    match &s.run {
        Some(run) => {
            let total: f64 = run.report.stages.iter().map(|st| st.seconds).sum();
            (
                Some(Icon::Done),
                format!(
                    "Processed {} frames in {} · {}",
                    run.report.frames,
                    crate::widgets::duration(total),
                    run.out.display()
                ),
            )
        }
        None => (Some(Icon::Done), format!("Showing {}", s.name)),
    }
}

fn row(step: Step, state: State, detail: impl Into<String>) -> StepRow {
    StepRow {
        step,
        state,
        detail: detail.into(),
    }
}

fn import_row(i: &Import) -> StepRow {
    let (state, detail) = if i.checking() {
        (State::Waiting, "Checking the folder".to_string())
    } else if i.processing() {
        (State::Waiting, "Processing".to_string())
    } else if i.interrupted.is_some() {
        (State::Attention, "Stopped; can resume".to_string())
    } else if i.survey.is_some() {
        (
            State::Waiting,
            format!("{} frames chosen", i.selected().len()),
        )
    } else {
        (State::Attention, "Could not check".to_string())
    };
    row(Step::Import, state, detail)
}

fn waiting(step: Step, detail: &str) -> StepRow {
    row(step, State::Unavailable, detail)
}

pub fn steps(app: &Pictor) -> Vec<StepRow> {
    let later = |first: StepRow, detail: &str| {
        let mut rows = vec![first];
        rows.extend(Step::ALL[1..].iter().map(|&s| waiting(s, detail)));
        rows
    };
    match &app.workspace {
        Workspace::Empty => later(
            row(Step::Import, State::Waiting, "Choose a folder"),
            "Not run yet",
        ),
        Workspace::Loading(_) => later(row(Step::Import, State::Waiting, "Opening"), "Opening"),
        Workspace::Import(i) => later(import_row(i), "Not run yet"),
        Workspace::Open(s) => crate::screens::rows(s),
    }
}
