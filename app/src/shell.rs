//! The window's frame from the design boards: a header with the project's name and what the
//! run uses, the step list on the left (each step with its state in a word or two, never
//! locked), and a status line at the bottom.

use crate::theme::{self, CHROME, LINE, MUTED, RAISED, TEXT, WARN};
use crate::widgets::{self, Icon, paint_icon};
use egui::{CornerRadius, Frame, Margin, Rect, RichText, Sense, Stroke, Ui, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Import,
    Frames,
    Sky,
    Scene,
    Compare,
    Look,
}

impl Step {
    pub const ALL: [Step; 6] = [
        Step::Import,
        Step::Frames,
        Step::Sky,
        Step::Scene,
        Step::Compare,
        Step::Look,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Step::Import => "Import",
            Step::Frames => "Frames",
            Step::Sky => "Sky & ground",
            Step::Scene => "Scene",
            Step::Compare => "Compare",
            Step::Look => "Look & export",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    Done,
    Attention,
    Waiting,
    /// Needs something that is not there yet; still shown, not clickable.
    Unavailable,
}

pub struct StepRow {
    pub step: Step,
    pub state: State,
    pub detail: String,
}

pub fn header(ui: &mut Ui, name: &str, chips: &[String]) {
    Frame::new()
        .fill(CHROME)
        .inner_margin(Margin::symmetric(20, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                widgets::icon(ui, Icon::Star, 20.0);
                ui.label(RichText::new("Pictor").font(theme::semibold(15.0)));
                if !name.is_empty() {
                    ui.label(RichText::new(name).color(MUTED));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for chip in chips.iter().rev() {
                        Frame::new()
                            .stroke(Stroke::new(1.0, LINE))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                ui.label(RichText::new(chip).monospace().size(12.0).color(MUTED))
                            });
                    }
                });
            });
        });
}

/// The step list; returns the step clicked.
pub fn rail(ui: &mut Ui, rows: &[StepRow], current: Step) -> Option<Step> {
    let mut clicked = None;
    ui.add_space(8.0);
    for (k, row) in rows.iter().enumerate() {
        let on = row.step == current;
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(vec2(width, 52.0), Sense::click());
        let usable = row.state != State::Unavailable;
        let p = ui.painter();
        if on {
            p.rect_filled(rect, CornerRadius::same(8), RAISED);
        } else if response.hovered() && usable {
            p.rect_filled(rect, CornerRadius::same(8), RAISED.gamma_multiply(0.5));
        }
        let icon = match (on, row.state) {
            (true, _) => Icon::Current,
            (false, State::Done) => Icon::Done,
            (false, State::Attention) => Icon::Warn,
            (false, _) => Icon::Waiting,
        };
        let icon_rect = Rect::from_min_size(rect.min + vec2(12.0, 12.0), vec2(16.0, 16.0));
        paint_icon(p, icon_rect, icon);
        let title_color = if usable || on { TEXT } else { MUTED };
        p.text(
            rect.min + vec2(38.0, 9.0),
            egui::Align2::LEFT_TOP,
            row.step.title(),
            if on {
                theme::semibold(14.0)
            } else {
                egui::FontId::proportional(14.0)
            },
            title_color,
        );
        let detail_color = if row.state == State::Attention {
            WARN
        } else {
            MUTED
        };
        p.text(
            rect.min + vec2(38.0, 29.0),
            egui::Align2::LEFT_TOP,
            &row.detail,
            egui::FontId::proportional(12.0),
            detail_color,
        );
        if response.clicked() && usable {
            clicked = Some(row.step);
        }
        if usable && let Some(key) = egui::Key::from_name(&(k + 1).to_string()) {
            let shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key);
            let text = ui.ctx().format_shortcut(&shortcut);
            response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(text);
        }
        ui.add_space(4.0);
    }
    clicked
}

pub fn footer(ui: &mut Ui, icon: Option<Icon>, text: &str) {
    Frame::new()
        .fill(CHROME)
        .inner_margin(Margin::symmetric(20, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some(i) = icon {
                    widgets::icon(ui, i, 16.0);
                }
                ui.label(RichText::new(text).size(13.0).color(MUTED));
            });
        });
}
