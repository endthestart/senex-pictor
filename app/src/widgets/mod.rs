//! The design boards' building blocks: cards, section labels, stat tiles, badges, buttons,
//! check rows with drawn icons and progress bars; choosing controls are in `controls`.

mod controls;

pub use controls::{choice, key, key_hints, legend, segmented, toggle};

use crate::theme::{self, ACCENT, CARD, GOOD, GROUND, LINE, MUTED, RAISED, TEXT, WARN};
use egui::text::LayoutJob;
use egui::{
    Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, Response, RichText, Sense, Stroke,
    TextFormat, Ui, Vec2, vec2,
};

pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A small uppercase label over a card's contents.
pub fn section(ui: &mut Ui, text: &str) {
    let mut job = LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        TextFormat {
            font_id: FontId::proportional(12.0),
            color: MUTED,
            extra_letter_spacing: 0.8,
            ..Default::default()
        },
    );
    ui.label(job);
}

pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(theme::semibold(24.0)));
}

pub fn strong(text: impl Into<String>) -> RichText {
    RichText::new(text).font(theme::semibold(14.0))
}

pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(MUTED)
}

pub fn mono(text: impl Into<String>) -> RichText {
    RichText::new(text).monospace().color(MUTED)
}

/// A labelled number, as in the boards' counts row.
pub fn tile(ui: &mut Ui, label: &str, value: &str, color: Color32, width: f32) {
    Frame::new()
        .fill(theme::CHROME)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(width - 24.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(label).size(12.0).color(MUTED));
                ui.label(
                    RichText::new(value)
                        .font(theme::semibold(18.0))
                        .color(color),
                );
            });
        });
}

/// Tiles laid out evenly across the available width.
pub fn tiles(ui: &mut Ui, items: &[(&str, String, Color32)]) {
    let gap = 8.0;
    let n = items.len().max(1) as f32;
    let width = ((ui.available_width() - gap * (n - 1.0)) / n).max(80.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (label, value, color) in items {
            tile(ui, label, value, *color, width);
        }
    });
}

#[derive(Clone, Copy)]
pub enum Badge {
    /// Filled with the accent: recommended, current.
    Strong,
    /// Outlined, muted.
    Quiet,
}

pub fn badge(ui: &mut Ui, text: &str, kind: Badge) -> Response {
    let (fill, stroke, color) = match kind {
        Badge::Strong => (ACCENT, ACCENT, GROUND),
        Badge::Quiet => (Color32::TRANSPARENT, LINE, MUTED),
    };
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(9, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(theme::semibold(11.5)).color(color))
        })
        .response
}

fn button(ui: &mut Ui, text: &str, fill: Color32, color: Color32, stroke: Color32) -> Response {
    let label = RichText::new(text).font(theme::semibold(14.0)).color(color);
    ui.add(
        egui::Button::new(label)
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(0.0, 38.0)),
    )
}

pub fn primary(ui: &mut Ui, text: &str) -> Response {
    button(ui, text, ACCENT, GROUND, ACCENT)
}

pub fn secondary(ui: &mut Ui, text: &str) -> Response {
    button(ui, text, Color32::TRANSPARENT, TEXT, LINE)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Done,
    Warn,
    Waiting,
    Current,
    Failed,
    Star,
}

/// Draws `icon` in a square of `size` points.
pub fn icon(ui: &mut Ui, icon: Icon, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_icon(ui.painter(), rect, icon);
    response
}

pub fn paint_icon(p: &egui::Painter, rect: Rect, icon: Icon) {
    let c = rect.center();
    let s = rect.width() / 16.0;
    let at = |x: f32, y: f32| Pos2::new(rect.min.x + x * s, rect.min.y + y * s);
    match icon {
        Icon::Done => {
            let stroke = Stroke::new(1.8 * s, GOOD);
            p.line_segment([at(3.0, 8.5), at(6.0, 11.5)], stroke);
            p.line_segment([at(6.0, 11.5), at(13.0, 4.5)], stroke);
        }
        Icon::Warn => {
            let stroke = Stroke::new(1.5 * s, WARN);
            let tri = [at(8.0, 2.5), at(14.0, 13.5), at(2.0, 13.5)];
            p.add(egui::Shape::closed_line(tri.to_vec(), stroke));
            p.line_segment([at(8.0, 6.5), at(8.0, 9.7)], stroke);
            p.circle_filled(at(8.0, 11.7), 0.9 * s, WARN);
        }
        Icon::Waiting => {
            p.circle_stroke(c, 5.0 * s, Stroke::new(1.4 * s, MUTED));
        }
        Icon::Current => {
            p.circle_filled(c, 4.0 * s, ACCENT);
        }
        Icon::Failed => {
            let stroke = Stroke::new(1.8 * s, theme::BAD);
            p.line_segment([at(4.0, 4.0), at(12.0, 12.0)], stroke);
            p.line_segment([at(12.0, 4.0), at(4.0, 12.0)], stroke);
        }
        Icon::Star => {
            let r = rect.width() / 2.0;
            let points: Vec<Pos2> = (0..8)
                .map(|k| {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_2;
                    let d = if k % 2 == 0 { r * 0.9 } else { r * 0.3 };
                    c + vec2(a.cos(), a.sin()) * d
                })
                .collect();
            p.add(egui::Shape::closed_line(points, Stroke::new(1.6, ACCENT)));
        }
    }
}

/// An icon, a title and lines of detail, as in "Before you start".
pub fn check_row(ui: &mut Ui, icon_kind: Icon, title: &str, lines: &[(String, Color32)]) {
    ui.horizontal_top(|ui| {
        icon(ui, icon_kind, 18.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(strong(title));
            for (line, color) in lines {
                ui.label(RichText::new(line).color(*color));
            }
        });
    });
}

/// A thin bar `fraction` full.
pub fn bar(ui: &mut Ui, fraction: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let r = CornerRadius::same((height / 2.0) as u8);
    ui.painter().rect_filled(rect, r, RAISED);
    let mut full = rect;
    full.set_width(rect.width() * fraction.clamp(0.0, 1.0));
    ui.painter().rect_filled(full, r, ACCENT);
}

/// "1 h 12 min", "4 min 10 s", "35 s".
pub fn duration(seconds: f64) -> String {
    let s = seconds.max(0.0).round() as u64;
    match s {
        0..60 => format!("{s} s"),
        60..3600 => format!("{} min {} s", s / 60, s % 60),
        _ => format!("{} h {} min", s / 3600, (s % 3600) / 60),
    }
}

/// "42 GB", "880 MB".
pub fn bytes(b: f64) -> String {
    if b >= 1e9 {
        format!("{:.0} GB", b / 1e9)
    } else {
        format!("{:.0} MB", b / 1e6)
    }
}

/// A pixel count: "1.2 MP", or "45,300 px" below a megapixel.
pub fn pixels(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1} MP", n as f64 / 1e6)
    } else {
        let digits = n.to_string();
        let mut out = String::new();
        for (k, c) in digits.chars().enumerate() {
            if k > 0 && (digits.len() - k).is_multiple_of(3) {
                out.push(',');
            }
            out.push(c);
        }
        format!("{out} px")
    }
}
