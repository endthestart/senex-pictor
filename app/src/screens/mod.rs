//! The steps of an opened result: what each screen shows, and each step's state in the step
//! list. Screens on the viewer (Sky & ground, Compare, Look & export) put the canvas in the
//! middle and their controls on the right; the others scroll.

pub mod frames;
pub mod look;
pub mod scene;
pub mod sky;
mod summary;

use crate::session::Session;
use crate::shell::{State, Step, StepRow};
use egui::{Frame, Margin, Ui};

/// Width of the controls beside the canvas.
const SIDE: f32 = 380.0;

fn scrolling(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::CentralPanel::default()
        .frame(Frame::NONE.inner_margin(Margin::symmetric(28, 24)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, add);
        });
}

/// The controls beside the canvas; call before `canvas`.
fn side(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Panel::right("controls")
        .exact_size(SIDE)
        .resizable(false)
        .frame(
            Frame::NONE
                .fill(crate::theme::GROUND)
                .inner_margin(Margin::symmetric(14, 14)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, add);
        });
}

fn canvas(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::CentralPanel::default()
        .frame(Frame::NONE)
        .show(ui, add);
}

pub fn show(ui: &mut Ui, ctx: &egui::Context, s: &mut Session, step: Step) {
    s.poll(ctx);
    match step {
        Step::Import => scrolling(ui, |ui| summary::show(ui, s)),
        Step::Frames => {
            frames::keys(ctx, s);
            scrolling(ui, |ui| frames::show(ui, s));
        }
        Step::Sky => {
            sky::keys(ctx, s);
            s.viewer.zoom_keys(ctx);
            side(ui, |ui| s.sky.panel(ui, &mut s.viewer, s.run.as_ref()));
            canvas(ui, |ui| s.sky.show(ui, &mut s.viewer));
        }
        Step::Scene => scrolling(ui, |ui| scene::show(ui, s)),
        Step::Compare => {
            s.compare.keys(ctx);
            s.viewer.zoom_keys(ctx);
            side(ui, |ui| s.compare.panel(ui, &mut s.viewer));
            canvas(ui, |ui| s.compare.show(ui, &mut s.viewer));
        }
        Step::Look => {
            s.viewer.zoom_keys(ctx);
            side(ui, |ui| {
                s.look
                    .panel(ui, &mut s.viewer, s.run.as_ref(), s.result.as_deref())
            });
            canvas(ui, |ui| s.look.show(ui, &mut s.viewer));
        }
    }
}

pub fn rows(s: &Session) -> Vec<StepRow> {
    let row = |step, state, detail: String| StepRow {
        step,
        state,
        detail,
    };
    let Some(run) = &s.run else {
        let none = |step| row(step, State::Unavailable, "Needs a processed folder".into());
        return vec![
            row(Step::Import, State::Done, "Opened without a run".into()),
            none(Step::Frames),
            none(Step::Sky),
            none(Step::Scene),
            row(Step::Compare, State::Done, s.compare_detail()),
            row(Step::Look, State::Done, s.look.detail(&s.viewer)),
        ];
    };
    vec![
        row(
            Step::Import,
            State::Done,
            format!("{} frames · run {}", run.report.frames, run.report.run + 1),
        ),
        frames::row(run),
        sky::row(run),
        row(Step::Scene, State::Done, scene::detail(s)),
        row(Step::Compare, State::Done, s.compare_detail()),
        row(Step::Look, State::Done, s.look.detail(&s.viewer)),
    ]
}
