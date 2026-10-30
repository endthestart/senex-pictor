//! Controls for choosing: a toggle button, joined segment buttons, choice cards, colour
//! legends and key hints.

use super::strong;
use crate::theme::{self, ACCENT, CARD, GROUND, LINE, MUTED, RAISED, TEXT, radius, role};
use egui::text::LayoutJob;
use egui::{
    Align, Color32, CornerRadius, FontId, Frame, Margin, Response, RichText, Sense, Stroke,
    TextFormat, Ui, vec2,
};

/// A button that is on or off, filled with the accent when on.
pub fn toggle(ui: &mut Ui, on: bool, text: &str, key: Option<&str>) -> Response {
    let (fill, color, stroke) = if on {
        (ACCENT, GROUND, ACCENT)
    } else {
        (CARD, TEXT, LINE)
    };
    let mut job = LayoutJob::default();
    let fmt = |c: Color32, f: FontId| TextFormat {
        font_id: f,
        color: c,
        valign: Align::Center,
        ..Default::default()
    };
    job.append(text, 0.0, fmt(color, FontId::proportional(13.5)));
    if let Some(k) = key {
        job.append(
            k,
            8.0,
            fmt(color.gamma_multiply(0.7), FontId::monospace(11.5)),
        );
    }
    ui.add(
        egui::Button::new(job)
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(CornerRadius::same(radius::CONTROL)),
    )
}

/// One choice of several, as a row of joined buttons; returns the one clicked.
pub fn segmented(ui: &mut Ui, options: &[&str], selected: Option<usize>) -> Option<usize> {
    let mut clicked = None;
    Frame::new()
        .fill(theme::CHROME)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(radius::BUTTON))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (k, option) in options.iter().enumerate() {
                    let on = selected == Some(k);
                    let text =
                        RichText::new(*option)
                            .size(13.0)
                            .color(if on { GROUND } else { TEXT });
                    let button = egui::Button::new(if on {
                        text.font(theme::semibold(13.0))
                    } else {
                        text
                    })
                    .fill(if on { ACCENT } else { Color32::TRANSPARENT })
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(radius::CONTROL));
                    if ui.add(button).clicked() {
                        clicked = Some(k);
                    }
                }
            });
        });
    clicked
}

/// A small colour square and what it marks.
pub fn legend(ui: &mut Ui, color: Color32, text: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(2), color);
    ui.label(
        RichText::new(text)
            .font(role::SECONDARY.font())
            .color(MUTED),
    );
}

/// A choice shown as a card: a title and a note, outlined in the accent when chosen.
pub fn choice(ui: &mut Ui, on: bool, title: &str, note: &str) -> Response {
    let frame = Frame::new()
        .fill(if on { RAISED } else { theme::CHROME })
        .stroke(Stroke::new(
            if on { 2.0 } else { 1.0 },
            if on { ACCENT } else { LINE },
        ))
        .corner_radius(CornerRadius::same(radius::CARD))
        .inner_margin(Margin::symmetric(12, 10));
    let response = frame
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(strong(title));
                ui.label(RichText::new(note).size(12.5).color(MUTED));
            });
        })
        .response;
    response
        .interact(Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Keys and what they do, two to a row.
pub fn key_hints(ui: &mut Ui, hints: &[(&str, &str)]) {
    egui::Grid::new(ui.next_auto_id())
        .num_columns(4)
        .spacing(vec2(8.0, 6.0))
        .show(ui, |ui| {
            for (k, (name, what)) in hints.iter().enumerate() {
                key(ui, name);
                ui.label(RichText::new(*what).size(12.5).color(MUTED));
                if k % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

/// A key name in a small box.
pub fn key(ui: &mut Ui, name: &str) {
    Frame::new()
        .stroke(Stroke::new(1.0, theme::LINE_STRONG))
        .corner_radius(CornerRadius::same(radius::KEY))
        .inner_margin(Margin::symmetric(5, 0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(name)
                    .monospace()
                    .size(role::NOTE.size)
                    .color(MUTED),
            )
        });
}
