//! A run in progress (stages with their times, the running step's count, time left, Cancel)
//! or stopped (what finished, Resume).

use super::Import;
use crate::job::Job;
use crate::theme::{MUTED, WARN};
use crate::widgets::{self, Icon, card, duration, section, strong};
use egui::{RichText, Ui};
use pictor_engine::progress::{Snapshot, StageState};

/// The engine's stage names, for people.
pub fn stage_label(name: &str) -> &str {
    match name {
        "sequence check" => "Check the sequence",
        "ingest" => "Develop the frames",
        "register" => "Align the sky",
        "separate" => "Separate sky and ground",
        "foreground drift" => "Measure foreground drift",
        "stack" => "Stack sky and ground",
        "composite around the reference" => "Compose around the middle frame",
        "hero" => "Compose the hero frame",
        "render" => "Render",
        other => other,
    }
}

pub fn status_line(s: &Snapshot) -> String {
    let stage = s
        .stages
        .iter()
        .find(|v| matches!(v.state, StageState::Running { .. }))
        .map_or("Starting", |v| stage_label(&v.name));
    let left = s
        .remaining_s
        .map_or(String::new(), |r| format!(" · about {} left", duration(r)));
    format!("Processing · {stage} · {:.0}%{left}", s.fraction * 100.0)
}

fn stages(ui: &mut Ui, s: &Snapshot) {
    for v in &s.stages {
        let (icon, time) = match v.state {
            StageState::Done { seconds } => (Icon::Done, duration(seconds)),
            StageState::Resumed { seconds } => {
                (Icon::Done, format!("{}, earlier", duration(seconds)))
            }
            StageState::Running { since } => {
                (Icon::Current, duration(since.elapsed().as_secs_f64()))
            }
            StageState::Waiting => (Icon::Waiting, String::new()),
        };
        ui.horizontal(|ui| {
            widgets::icon(ui, icon, 16.0);
            let label = RichText::new(stage_label(&v.name));
            ui.label(if icon == Icon::Waiting {
                label.color(MUTED)
            } else {
                label
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(time).monospace().size(12.0).color(MUTED));
            });
        });
    }
}

/// A background run's card: progress, each stage, the current step, Cancel and `note`;
/// whether Cancel was clicked.
pub fn job_card<T: Send + 'static>(ui: &mut Ui, title: &str, job: &Job<T>, note: &str) -> bool {
    let s = job.snapshot();
    let elapsed = job.started.elapsed().as_secs_f64();
    let cancelling = job.cancelled();
    let mut cancel = false;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            section(ui, title);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(widgets::muted(format!("{} so far", duration(elapsed))));
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{:.0}%", s.fraction * 100.0))
                    .font(crate::theme::semibold(28.0)),
            );
            if let Some(r) = s.remaining_s {
                ui.label(widgets::muted(format!("about {} left", duration(r))));
            }
        });
        widgets::bar(ui, s.fraction as f32, 6.0);
        ui.add_space(8.0);
        stages(ui, &s);
        if let Some(step) = &s.step {
            ui.add_space(6.0);
            let count = if s.total > 0 {
                format!(" · {} of {}", s.done, s.total)
            } else {
                String::new()
            };
            ui.label(widgets::muted(format!("{step}{count}")));
            if s.total > 0 {
                widgets::bar(ui, s.done as f32 / s.total as f32, 4.0);
            }
        }
        ui.add_space(10.0);
        if cancelling {
            ui.label(RichText::new("Stopping at the next frame…").color(WARN));
        } else {
            cancel = widgets::secondary(ui, "Cancel").clicked();
        }
        ui.label(widgets::muted(note));
    });
    cancel
}

impl Import {
    /// The running run's card, with Cancel.
    pub(super) fn run_card(&mut self, ui: &mut Ui) {
        if let Some(job) = &self.processing
            && job_card(
                ui,
                "Processing",
                job,
                "Cancelling keeps what is done; the run can resume later from this folder.",
            )
        {
            self.cancel();
        }
    }

    pub(super) fn interrupted_card(&mut self, ui: &mut Ui) -> bool {
        let Some(r) = &self.interrupted else {
            return false;
        };
        let mut resume = false;
        card(ui, |ui| {
            section(ui, "Run stopped");
            let at = r
                .running
                .as_ref()
                .map_or("between stages".to_string(), |s| {
                    format!("during “{}”", stage_label(&s.stage))
                });
            ui.label(strong(format!("Stopped {at}")));
            for s in &r.stages {
                ui.horizontal(|ui| {
                    widgets::icon(ui, Icon::Done, 16.0);
                    ui.label(stage_label(&s.name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(duration(s.seconds))
                                .monospace()
                                .size(12.0)
                                .color(MUTED),
                        );
                    });
                });
            }
            ui.add_space(8.0);
            ui.label(widgets::muted(
                "Resuming keeps the finished stages and runs the rest with the same frames.",
            ));
            ui.add_space(6.0);
            resume = widgets::primary(ui, "Resume").clicked();
        });
        resume
    }
}

impl Import {
    /// A line for the footer.
    pub fn status(&self) -> String {
        if let Some(job) = &self.checking {
            let s = job.snapshot();
            return format!(
                "Checking the folder · {} of {} frames read",
                s.done, s.total
            );
        }
        if let Some(job) = &self.processing {
            let s = job.snapshot();
            return status_line(&s);
        }
        if let Some(r) = &self.interrupted {
            return match &r.running {
                Some(stage) => format!("The run stopped during {}; it can resume", stage.stage),
                None => "The run stopped between stages; it can resume".into(),
            };
        }
        match &self.survey {
            Some(s) => format!(
                "Checked {} frames in {}",
                s.paths.len(),
                crate::widgets::duration(s.seconds)
            ),
            None => String::new(),
        }
    }
}
