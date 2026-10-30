//! The Import screen's layout: the folder, its runs and the frames to process on the left;
//! the checks, output folder and Start (or the run's progress) on the right.

use super::{Checks, Import};
use crate::theme::WARN;
use crate::widgets::{self, card, check_row, mono, section};
use egui::{RichText, Ui};

impl Import {
    pub fn show(&mut self, ui: &mut Ui) {
        widgets::heading(ui, "Import");
        ui.label(widgets::muted(
            "The folder is checked before anything is processed. Nothing is written until you start.",
        ));
        ui.add_space(12.0);
        let wide = ui.available_width() > 980.0;
        if wide {
            let total = ui.available_width();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(total * 0.58 - 12.0);
                    self.left(ui);
                });
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.set_width(total * 0.42 - 12.0);
                    self.right(ui);
                });
            });
        } else {
            self.left(ui);
            ui.add_space(12.0);
            self.right(ui);
        }
    }

    fn left(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 12.0;
        self.folder_card(ui);
        if let Some(job) = &self.checking {
            let s = job.snapshot();
            card(ui, |ui| {
                section(ui, "Checking the folder");
                ui.label(format!("Read {} of {} frames", s.done, s.total));
                let f = if s.total > 0 {
                    s.done as f32 / s.total as f32
                } else {
                    0.0
                };
                widgets::bar(ui, f, 6.0);
                ui.label(widgets::muted(
                    "Every frame is read once for its time, settings, brightness and motion.",
                ));
            });
        }
        self.runs_card(ui);
        let locked = self.processing() || self.interrupted.is_some();
        self.frames_card(ui, locked);
    }

    fn folder_card(&mut self, ui: &mut Ui) {
        card(ui, |ui| {
            section(ui, "Folder");
            ui.label(RichText::new(self.raw.display().to_string()).monospace());
            let Some(survey) = &self.survey else {
                return;
            };
            let Some(c) = self.selected().first().map(|&i| &survey.captures[i]) else {
                return;
            };
            let s = &c.settings;
            let mut parts = vec![format!("{} RAW files", survey.paths.len())];
            parts.extend(s.model.clone());
            parts.extend(s.lens.clone());
            parts.extend(s.exposure_s.map(|e| format!("{e} s")));
            parts.extend(s.iso.map(|i| format!("ISO {i}")));
            parts.extend(s.aperture.map(|a| format!("f/{a}")));
            ui.label(widgets::muted(parts.join(" · ")));
        });
    }

    fn right(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 12.0;
        if let Some(e) = &self.error {
            card(ui, |ui| {
                check_row(
                    ui,
                    widgets::Icon::Failed,
                    "Something went wrong",
                    &[(e.clone(), WARN)],
                );
            });
        }
        if self.processing() {
            self.run_card(ui);
            return;
        }
        if self.survey.is_none() {
            return;
        }
        if self.interrupted_card(ui) {
            self.start();
            return;
        }
        if self.checks.is_none() {
            let mut lookups = std::mem::take(&mut self.lookups);
            self.checks = Some(Checks::of(self, &mut lookups));
            self.lookups = lookups;
        }
        let mut lens_pick: Option<String> = None;
        if let Some(checks) = &self.checks {
            card(ui, |ui| {
                section(ui, "Before you start");
                for item in &checks.items {
                    ui.separator();
                    check_row(ui, item.icon, item.title, &item.lines);
                }
                if let Some(choices) = &checks.lens_choices {
                    ui.add_space(4.0);
                    let current = self
                        .lens
                        .clone()
                        .unwrap_or_else(|| "Choose the lens…".into());
                    egui::ComboBox::from_id_salt("lens")
                        .selected_text(current)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for name in choices {
                                if ui
                                    .selectable_label(self.lens.as_deref() == Some(name), name)
                                    .clicked()
                                {
                                    lens_pick = Some(name.clone());
                                }
                            }
                        });
                }
            });
        }
        if let Some(l) = lens_pick {
            self.lens = Some(l);
            self.checks = None;
        }
        if self.interrupted.is_some() {
            return;
        }
        card(ui, |ui| {
            section(ui, "Output folder");
            ui.label(mono(self.out.display().to_string()));
            if widgets::secondary(ui, "Choose Another…").clicked()
                && let Some(dir) = rfd::FileDialog::new().pick_folder()
            {
                self.choose_out(dir);
            }
        });
        let (estimate, blocking) = self
            .checks
            .as_ref()
            .map_or((0.0, Some("Checking…".to_string())), |c| {
                (c.estimate_s, c.blocking.clone())
            });
        let label = match &blocking {
            None => format!("Start · about {}", widgets::duration(estimate)),
            Some(why) => why.clone(),
        };
        ui.add_enabled_ui(blocking.is_none(), |ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 10.0);
            if widgets::primary(ui, &label).clicked() {
                self.start();
            }
        });
    }

    /// Uses `dir` for the run: as it is when it is empty or holds a run, else a new folder
    /// inside it named after the RAW folder.
    fn choose_out(&mut self, dir: std::path::PathBuf) {
        let empty = std::fs::read_dir(&dir).is_ok_and(|mut d| d.next().is_none());
        let ours = dir.join(pictor_engine::process::journal::FILE).exists();
        self.out = if empty || ours {
            dir
        } else {
            let name = super::default_out(&self.raw, &self.settings)
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_os_string()))
                .unwrap_or_else(|| "Pictor".into());
            dir.join(name)
        };
        self.interrupted = pictor_engine::process::Report::load(&self.out)
            .ok()
            .flatten()
            .filter(|r| !r.complete);
        self.checks = None;
    }
}
