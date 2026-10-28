//! The chosen frame: larger (read from the store), blinked against the reference with B, its
//! settings, why it was flagged and how the composite used it.

use super::Facts;
use crate::session::Session;
use crate::theme::{GOOD, WARN};
use crate::widgets::{self, card, key, mono, strong};
use egui::{RichText, Ui};

pub fn show(ui: &mut Ui, s: &mut Session, f: &Facts) {
    let i = s.frames.selected;
    let reference = f.reference_index();
    let shown = if s.frames.blink {
        reference.unwrap_or(i)
    } else {
        i
    };
    let stretch = s.thumbs.as_ref().map(|t| t.stretch);
    let ctx = ui.ctx().clone();
    let texture = s.frames.texture(&ctx, &s.run, stretch, shown);
    let Some(run) = &s.run else {
        return;
    };
    let name = &f.names[i];
    let survey_index = run.survey.captures.iter().position(|c| &c.name == name);
    card(ui, |ui| {
        let width = ui.available_width();
        let aspect = run.store.manifest.width as f32 / run.store.manifest.height as f32;
        let size = egui::vec2(width, width / aspect);
        match (
            &texture,
            survey_index.and_then(|k| s.thumbs.as_mut()?.texture(&ctx, k)),
        ) {
            (Some(t), _) => {
                ui.add(egui::Image::new(t).fit_to_exact_size(size).corner_radius(6));
            }
            (None, Some(thumb)) => {
                ui.add(
                    egui::Image::new(&thumb)
                        .fit_to_exact_size(size)
                        .corner_radius(6),
                );
            }
            (None, None) => {
                ui.allocate_space(size);
            }
        }
        ui.horizontal(|ui| {
            ui.label(mono(format!("Showing {}", f.names[shown])));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                key(ui, "B");
                if widgets::toggle(ui, s.frames.blink, "Blink against the reference", None)
                    .clicked()
                {
                    s.frames.blink = !s.frames.blink;
                }
            });
        });
    });
    card(ui, |ui| {
        ui.label(strong(name).size(16.0));
        if let Some(c) = survey_index.map(|k| &run.survey.captures[k]) {
            let st = &c.settings;
            let mut parts = Vec::new();
            parts.extend(st.exposure_s.map(|e| format!("{e} s")));
            parts.extend(st.iso.map(|v| format!("ISO {v}")));
            parts.extend(st.aperture.map(|v| format!("f/{v}")));
            parts.extend(st.focal_mm.map(|v| format!("{v:.0} mm")));
            ui.label(mono(parts.join(" · ")));
        }
        let lit = f.lit.contains(name);
        if lit {
            ui.label(RichText::new("Lit: the foreground is at least 1.3× brighter or redder than its darker neighbours").color(WARN));
        } else {
            ui.label(RichText::new("Not flagged").color(GOOD));
        }
        if Some(i) == reference {
            ui.label(widgets::muted("Reference frame: the sky is aligned to it"));
        }
        let moved = f.drift.iter().find(|(n, _)| n == name).map(|(_, d)| *d);
        let usage = if lit {
            "Hero frame: available · Long exposure: left out".to_string()
        } else if f.lost.contains(name) {
            "Long exposure: left out, its foreground could not be tracked".to_string()
        } else {
            match moved {
                Some(d) if f.moved_back && d >= 0.5 => {
                    format!("Long exposure: used, moved back {d:.1} px")
                }
                _ => "Long exposure: used".to_string(),
            }
        };
        ui.label(widgets::muted(usage));
        ui.horizontal(|ui| {
            key(ui, "←");
            key(ui, "→");
            ui.label(widgets::muted("previous and next frame"));
        });
    });
}
