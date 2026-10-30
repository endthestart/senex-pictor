//! The Frames step's two strips: each frame's foreground drift as a bar, and every processed
//! frame as a thumbnail with its flag. Clicking either chooses the frame.

use super::Facts;
use crate::theme::{ACCENT, CARD, LINE, MUTED, TEXT, WARN};
use crate::thumbs::Thumbs;
use crate::widgets::{card, legend, section};
use egui::{Color32, CornerRadius, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use pictor_engine::sequence::Survey;

const CHART_HEIGHT: f32 = 120.0;
const DRIFT_SCALE: f64 = 10.0;
const MEASURED: Color32 = Color32::from_rgb(110, 143, 204);
const THUMB_HEIGHT: f32 = 84.0;

/// The drift chart; returns the frame clicked.
pub fn drift_chart(ui: &mut Ui, f: &Facts, selected: usize) -> Option<usize> {
    let mut picked = None;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            section(ui, "Foreground drift from the reference frame, px");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                legend(ui, WARN, "lit, not in the long exposure");
                legend(ui, MEASURED, "measured");
            });
        });
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(vec2(width, CHART_HEIGHT), Sense::click());
        let n = f.names.len().max(1);
        // At least DRIFT_SCALE px high, so a still foreground's jitter looks small.
        let top = f.drift.iter().map(|(_, d)| *d).fold(DRIFT_SCALE, f64::max);
        let w = width / n as f32;
        let p = ui.painter();
        p.line_segment(
            [pos2(rect.min.x, rect.max.y), pos2(rect.max.x, rect.max.y)],
            Stroke::new(1.0, LINE),
        );
        for (i, name) in f.names.iter().enumerate() {
            let lit = f.lit.contains(name);
            let d = f.drift.iter().find(|(n, _)| n == name).map(|(_, d)| *d);
            let h = match (lit, d) {
                (true, _) => 6.0,
                (false, Some(d)) => (d / top * (CHART_HEIGHT as f64 - 10.0)).max(3.0) as f32,
                (false, None) => 0.0,
            };
            let x = rect.min.x + i as f32 * w;
            let bar = Rect::from_min_max(
                pos2(x + w * 0.1, rect.max.y - h),
                pos2(x + w * 0.9, rect.max.y),
            );
            p.rect_filled(
                bar,
                CornerRadius::same(1),
                if lit { WARN } else { MEASURED },
            );
            if i == selected {
                p.rect_stroke(
                    bar.expand(1.5),
                    CornerRadius::same(2),
                    Stroke::new(2.0, TEXT),
                    StrokeKind::Outside,
                );
            }
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            picked = Some((((pos.x - rect.min.x) / w) as usize).min(n - 1));
        }
        ui.horizontal(|ui| {
            let first = f.names.first().cloned().unwrap_or_default();
            let last = f.names.last().cloned().unwrap_or_default();
            ui.label(RichText::new(first).monospace().size(11.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(last).monospace().size(11.0).color(MUTED));
            });
        });
    });
    picked
}

/// Every processed frame as a thumbnail; returns the frame clicked.
pub fn filmstrip(
    ui: &mut Ui,
    thumbs: &mut Thumbs,
    survey: &Survey,
    f: &Facts,
    selected: usize,
    follow: bool,
) -> Option<usize> {
    let mut picked = None;
    let reference = f.reference_index();
    card(ui, |ui| {
        section(ui, &format!("Every frame · {}", f.names.len()));
        let cell_w = THUMB_HEIGHT * thumbs.aspect(0) + 12.0;
        let cell = vec2(cell_w, THUMB_HEIGHT + 30.0);
        let ctx = ui.ctx().clone();
        let scroll = egui::ScrollArea::horizontal()
            .id_salt("filmstrip")
            .auto_shrink([false, true]);
        scroll.show_viewport(ui, |ui, viewport| {
            let total = vec2(cell.x * f.names.len() as f32, cell.y);
            let (rect, response) = ui.allocate_exact_size(total, Sense::click());
            let first = (viewport.min.x / cell.x).floor().max(0.0) as usize;
            let last = ((viewport.max.x / cell.x).ceil() as usize).min(f.names.len());
            let p = ui.painter();
            for i in first..last {
                let name = &f.names[i];
                let r =
                    Rect::from_min_size(rect.min + vec2(i as f32 * cell.x, 0.0), cell).shrink(3.0);
                let on = i == selected;
                p.rect_filled(r, CornerRadius::same(8), CARD);
                p.rect_stroke(
                    r,
                    CornerRadius::same(8),
                    Stroke::new(if on { 2.0 } else { 1.0 }, if on { ACCENT } else { LINE }),
                    StrokeKind::Inside,
                );
                let image = Rect::from_min_size(
                    r.min + vec2(4.0, 4.0),
                    vec2(r.width() - 8.0, THUMB_HEIGHT - 6.0),
                );
                if let Some(k) = survey.captures.iter().position(|c| &c.name == name)
                    && let Some(t) = thumbs.texture(&ctx, k)
                {
                    p.image(
                        t.id(),
                        image,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                let short = name.split('.').next().unwrap_or(name);
                p.text(
                    pos2(r.min.x + 6.0, r.max.y - 20.0),
                    egui::Align2::LEFT_TOP,
                    short,
                    egui::FontId::monospace(11.0),
                    MUTED,
                );
                let flag = if f.lit.contains(name) {
                    Some(("Lit", WARN))
                } else if Some(i) == reference {
                    Some(("Reference", ACCENT))
                } else if f.lost.contains(name) {
                    Some(("Lost", WARN))
                } else {
                    None
                };
                if let Some((text, color)) = flag {
                    p.text(
                        pos2(r.max.x - 6.0, r.max.y - 20.0),
                        egui::Align2::RIGHT_TOP,
                        text,
                        egui::FontId::proportional(11.0),
                        color,
                    );
                }
            }
            if follow {
                let target =
                    Rect::from_min_size(rect.min + vec2(selected as f32 * cell.x, 0.0), cell);
                ui.scroll_to_rect(target, Some(egui::Align::Center));
            }
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                picked = Some((((pos.x - rect.min.x) / cell.x) as usize).min(f.names.len() - 1));
            }
        });
    });
    picked
}
