//! Correcting the matte: the Corrections card (tools, brush size, undo, Apply) and applying
//! the strokes, which saves them to the store and does the run again from its matte on.

use super::Sky;
use super::brush::{RADIUS_RANGE, Tool};
use crate::import::job_card;
use crate::job::Job;
use crate::project::Run;
use crate::settings::Settings;
use crate::theme::WARN;
use crate::widgets::{self, card, section};
use egui::{RichText, Ui};
use pictor_engine::process;
use pictor_engine::separate::corrections;
use pictor_engine::store::Store;

impl Sky {
    /// Saves the strokes to the store and does the run again from its matte on.
    fn apply(&mut self, run: &Run, settings: Settings) {
        let (dir, out, strokes) = (
            run.store.dir.clone(),
            run.out.clone(),
            self.brush.strokes.clone(),
        );
        self.brush.tool = Tool::Move;
        self.error = None;
        self.applying = Some(Job::start("Applying the corrections", move |progress| {
            corrections::save(&Store::open(&dir)?, &strokes)?;
            let gpu = settings.gpu()?;
            process::correct(&out, gpu.as_ref(), &settings.budget(progress)?)
        }));
    }

    pub(super) fn corrections(&mut self, ui: &mut Ui, run: &Run) {
        card(ui, |ui| {
            section(ui, "Corrections");
            let busy = self.applying.is_some();
            ui.add_enabled_ui(!busy, |ui| {
                let tools = [Tool::Move, Tool::Sky, Tool::Ground];
                let at = tools.iter().position(|&t| t == self.brush.tool);
                if let Some(k) = widgets::segmented(ui, &["Move", "Paint sky", "Paint ground"], at)
                {
                    self.brush.tool = tools[k];
                }
                ui.horizontal(|ui| {
                    ui.label("Brush");
                    ui.add(
                        egui::Slider::new(&mut self.brush.radius, RADIUS_RANGE)
                            .logarithmic(true)
                            .suffix(" px")
                            .max_decimals(0),
                    );
                });
                widgets::key_hints(
                    ui,
                    &[
                        ("V S G", "move, sky, ground"),
                        ("[ ]", "brush size"),
                        ("⌘Z", "undo"),
                    ],
                );
                ui.horizontal(|ui| {
                    let some = !self.brush.strokes.is_empty();
                    if ui.add_enabled(some, egui::Button::new("Undo")).clicked() {
                        self.brush.strokes.pop();
                    }
                    if ui
                        .add_enabled(some, egui::Button::new("Clear all"))
                        .clicked()
                    {
                        self.brush.strokes.clear();
                    }
                });
            });
            let pending = self.brush.changed();
            if pending {
                ui.label(RichText::new("Not applied yet").color(WARN));
            }
            if ui
                .add_enabled_ui(pending && !busy, |ui| {
                    widgets::primary(ui, "Apply corrections")
                })
                .inner
                .clicked()
            {
                self.apply(run, Settings::get(ui.ctx()));
            }
            ui.label(widgets::muted(
                "Applying draws the sky and ground again from the night's saved statistics, \
                 then stacks and renders the result again.",
            ));
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(WARN));
            }
        });
        if let Some(job) = &self.applying
            && job_card(
                ui,
                "Applying corrections",
                job,
                "Cancelling leaves the run to resume from Import.",
            )
        {
            job.cancel();
        }
    }
}
