//! "Sequence check": the folder's runs (the recommended one marked, short ones folded away)
//! and the chosen run's counts of overexposed and lit frames, twilight and drift.

use super::Import;
use crate::theme::{LINE_STRONG, MUTED, RAISED, WARN, radius};
use crate::widgets::{self, Badge, card, section, strong};
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};

fn reason(starts_after: &str) -> &str {
    match starts_after {
        "start" => "first run",
        other => other,
    }
}

/// Runs this short are folded into one line unless chosen or recommended.
const SHORT_RUN: usize = 10;

fn frames(n: usize) -> String {
    if n == 1 {
        "1 frame".into()
    } else {
        format!("{n} frames")
    }
}

impl Import {
    pub(super) fn runs_card(&mut self, ui: &mut Ui) {
        let Some(survey) = self.survey.clone() else {
            return;
        };
        let check = &survey.check;
        let locked = self.processing() || self.interrupted.is_some();
        card(ui, |ui| {
            let runs = check.runs.len();
            section(
                ui,
                &format!(
                    "Sequence check · {runs} run{}",
                    if runs == 1 { "" } else { "s" }
                ),
            );
            let short: Vec<usize> = (0..runs)
                .filter(|&k| {
                    check.runs[k].frames < SHORT_RUN && k != self.run && k != check.recommended
                })
                .collect();
            if !short.is_empty() && !self.show_short_runs {
                let total: usize = short.iter().map(|&k| check.runs[k].frames).sum();
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(widgets::muted(format!(
                        "{} short run{} ({}) before gaps or setting changes",
                        short.len(),
                        if short.len() == 1 { "" } else { "s" },
                        frames(total)
                    )));
                    if ui.link("Show").clicked() {
                        self.show_short_runs = true;
                    }
                });
            }
            for (k, run) in check.runs.iter().enumerate() {
                if short.contains(&k) && !self.show_short_runs {
                    continue;
                }
                let chosen = k == self.run;
                let frame = Frame::new()
                    .fill(if chosen {
                        RAISED
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(Stroke::new(
                        1.0,
                        if chosen {
                            LINE_STRONG
                        } else {
                            egui::Color32::TRANSPARENT
                        },
                    ))
                    .corner_radius(CornerRadius::same(radius::CARD))
                    .inner_margin(Margin::symmetric(14, 10));
                let response = frame
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                let title = format!("Run {} · {} – {}", k + 1, run.first, run.last);
                                ui.label(if chosen {
                                    strong(title)
                                } else {
                                    RichText::new(title).color(MUTED)
                                });
                                let hours = run.frames as f64 * check.interval_s;
                                ui.label(widgets::muted(format!(
                                    "{}, one every {:.0} s, about {} · after {}",
                                    frames(run.frames),
                                    check.interval_s,
                                    widgets::duration(hours),
                                    reason(&run.starts_after)
                                )));
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if k == check.recommended {
                                        widgets::badge(ui, "Recommended", Badge::Strong);
                                    }
                                },
                            );
                        });
                    })
                    .response;
                if !locked && !chosen && response.interact(egui::Sense::click()).clicked() {
                    self.choose_run(k);
                }
            }
            let Some(run) = self.chosen_run() else {
                return;
            };
            ui.add_space(4.0);
            let twilight = run.twilight.as_ref().map_or("None".to_string(), |t| {
                format!("{:.1}× brighter", t.brightening)
            });
            let drift = run.drift_px[0].hypot(run.drift_px[1]);
            widgets::tiles(
                ui,
                &[
                    (
                        "Overexposed",
                        run.overexposed.len().to_string(),
                        if run.overexposed.is_empty() {
                            crate::theme::TEXT
                        } else {
                            WARN
                        },
                    ),
                    (
                        "Lit frames",
                        run.lit.len().to_string(),
                        if run.lit.is_empty() {
                            crate::theme::TEXT
                        } else {
                            WARN
                        },
                    ),
                    ("Twilight", twilight, crate::theme::TEXT),
                    ("Camera drift", format!("{drift:.0} px"), crate::theme::TEXT),
                ],
            );
        });
    }
}
