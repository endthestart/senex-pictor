//! The Import step once a result is open: what was processed, where it came from and went,
//! and how long each stage took.

use crate::import::stage_label;
use crate::session::Session;
use crate::theme::MUTED;
use crate::widgets::{self, Icon, card, duration, mono, section, strong};
use egui::{RichText, Ui};

pub fn show(ui: &mut Ui, s: &mut Session) {
    widgets::heading(ui, "Import");
    ui.add_space(8.0);
    ui.spacing_mut().item_spacing.y = 12.0;
    let Some(run) = &s.run else {
        card(ui, |ui| {
            section(ui, "Opened");
            ui.label(strong(&s.name));
            ui.label(widgets::muted(
                "This was opened without its run (a store, a scene file or the demo scene), so \
                 Frames, Sky & ground and Scene have nothing to show. Open a processed folder \
                 for those.",
            ));
        });
        return;
    };
    let r = &run.report;
    card(ui, |ui| {
        section(ui, "Processed");
        ui.label(strong(format!(
            "{} frames of run {}, {} to {}",
            r.frames,
            r.run + 1,
            r.first,
            r.last
        )));
        if !r.overexposed_left_out.is_empty() {
            ui.label(widgets::muted(format!(
                "{} overexposed frames left out",
                r.overexposed_left_out.len()
            )));
        }
        ui.add_space(4.0);
        ui.label(widgets::muted("RAW files"));
        ui.label(mono(r.raw.display().to_string()));
        ui.label(widgets::muted("Output folder"));
        ui.label(mono(run.out.display().to_string()));
        ui.label(widgets::muted(format!(
            "{} budget, {} threads, {}",
            widgets::bytes(r.budget.bytes as f64),
            r.budget.threads,
            r.device
        )));
    });
    card(ui, |ui| {
        section(ui, "Stages");
        let total: f64 = r.stages.iter().map(|st| st.seconds).sum();
        for st in &r.stages {
            ui.horizontal(|ui| {
                widgets::icon(ui, Icon::Done, 16.0);
                ui.label(stage_label(&st.name));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(duration(st.seconds))
                            .monospace()
                            .size(12.0)
                            .color(MUTED),
                    );
                });
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(strong("In all"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(duration(total)).monospace().size(12.0));
            });
        });
    });
    if !r.image.is_empty() {
        card(ui, |ui| {
            section(ui, "Rendered");
            for f in &r.image {
                ui.label(mono(f));
            }
        });
    }
}
