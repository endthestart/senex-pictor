//! "Frames to process": the chosen run as a strip of thumbnails with the frames to process
//! between two handles, overexposed and lit frames marked beneath, and the count, time span and
//! disk need of the choice.

use super::Import;
use crate::theme::{ACCENT, BAD, CHROME, MUTED, WARN};
use crate::widgets::{self, card, mono, section};
use egui::{Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

const STRIP_HEIGHT: f32 = 64.0;
const TICKS: f32 = 6.0;

impl Import {
    pub(super) fn frames_card(&mut self, ui: &mut Ui, locked: bool) {
        let (Some(survey), Some((a, b))) = (self.survey.clone(), self.run_range()) else {
            return;
        };
        let Some(run) = self.chosen_run().cloned() else {
            return;
        };
        card(ui, |ui| {
            let selected = self.selected();
            let span = survey.captures[self.last].time_s - survey.captures[self.first].time_s;
            ui.horizontal(|ui| {
                section(ui, "Frames to process");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(widgets::muted(format!(
                        "{} frames · {} of sky",
                        selected.len(),
                        widgets::duration(span)
                    )));
                });
            });
            ui.add_space(4.0);
            let width = ui.available_width();
            let (rect, response) = ui.allocate_exact_size(
                vec2(width, STRIP_HEIGHT + TICKS + 4.0),
                if locked {
                    Sense::hover()
                } else {
                    Sense::click_and_drag()
                },
            );
            let strip = Rect::from_min_size(rect.min, vec2(width, STRIP_HEIGHT));
            let n = b - a + 1;
            let x_of = |i: usize| strip.min.x + (i - a) as f32 / n as f32 * width;
            let painter = ui.painter_at(rect);
            painter.rect_filled(strip, CornerRadius::same(6), CHROME);
            if let Some(thumbs) = self.thumbs.as_mut() {
                let cell = STRIP_HEIGHT * thumbs.aspect(a);
                let slots = ((width / cell).floor() as usize).clamp(1, n);
                let w = width / slots as f32;
                for j in 0..slots {
                    let i = a + j * n / slots;
                    let r = Rect::from_min_size(
                        pos2(strip.min.x + j as f32 * w, strip.min.y),
                        vec2(w, STRIP_HEIGHT),
                    );
                    if let Some(t) = thumbs.texture(ui.ctx(), i) {
                        painter.image(
                            t.id(),
                            r.shrink(1.0),
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }
            }
            let (x0, x1) = (x_of(self.first), x_of(self.last) + width / n as f32);
            let dim = Color32::from_black_alpha(170);
            painter.rect_filled(
                Rect::from_min_max(strip.min, pos2(x0, strip.max.y)),
                0.0,
                dim,
            );
            painter.rect_filled(
                Rect::from_min_max(pos2(x1, strip.min.y), strip.max),
                0.0,
                dim,
            );
            let chosen = Rect::from_min_max(pos2(x0, strip.min.y), pos2(x1, strip.max.y));
            painter.rect_stroke(
                chosen,
                CornerRadius::same(4),
                Stroke::new(2.0, ACCENT),
                StrokeKind::Inside,
            );
            for x in [x0, x1] {
                let grip = Rect::from_center_size(pos2(x, strip.center().y), vec2(6.0, 26.0));
                painter.rect_filled(grip, CornerRadius::same(3), ACCENT);
            }
            let ticks = strip.max.y + 4.0;
            for i in a..=b {
                let name = &survey.captures[i].name;
                let color = if run.overexposed.contains(name) {
                    BAD
                } else if run.lit.contains(name) {
                    WARN
                } else {
                    continue;
                };
                let x = x_of(i);
                painter.rect_filled(
                    Rect::from_min_size(pos2(x, ticks), vec2((width / n as f32).max(2.0), TICKS)),
                    0.0,
                    color,
                );
            }
            if !locked
                && let Some(p) = response.interact_pointer_pos()
                && (response.dragged() || response.clicked())
            {
                let i = (a + (((p.x - strip.min.x) / width) * n as f32).floor().max(0.0) as usize)
                    .min(b);
                let near_first = (p.x - x0).abs() <= (p.x - x1).abs();
                if near_first {
                    self.set_range(i.min(self.last), self.last);
                } else {
                    self.set_range(self.first, i.max(self.first));
                }
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(mono(format!("First {}", survey.captures[self.first].name)));
                ui.label(mono(format!("Last {}", survey.captures[self.last].name)));
                if !run.overexposed.is_empty() {
                    widgets::legend(ui, BAD, "overexposed");
                }
                if !run.lit.is_empty() {
                    widgets::legend(ui, WARN, "lit");
                }
                let recommended = (
                    super::Import::index(&survey, &run.use_first),
                    super::Import::index(&survey, &run.use_last),
                );
                if let (Some(f), Some(l)) = recommended
                    && (f, l) != (self.first, self.last)
                    && !locked
                    && ui.link("Use the recommended frames").clicked()
                {
                    self.set_range(f, l);
                }
            });
            if !locked {
                ui.label(
                    egui::RichText::new("Drag the handles to choose the frames.")
                        .color(MUTED)
                        .size(12.0),
                );
            }
        });
    }
}
