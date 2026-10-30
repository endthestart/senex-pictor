//! The first screen, with nothing open: open a folder (RAW files to process, or a processed
//! output folder to review) by button or by dropping it on the window, a scene file, or the
//! demo scene.

use crate::theme::{self, ACCENT, LINE_STRONG, MUTED, WARN};
use crate::widgets::{self, Icon};
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use std::path::PathBuf;

pub enum Action {
    Open(PathBuf),
    Demo,
}

pub fn show(ui: &mut Ui, error: Option<&str>) -> Option<Action> {
    let mut action = None;
    let dropping = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() * 0.16).max(24.0));
        widgets::icon(ui, Icon::Star, 44.0);
        ui.add_space(8.0);
        ui.label(RichText::new("Pictor").font(theme::semibold(30.0)));
        ui.label(
            RichText::new("A night of sky, stacked over a sharp foreground.")
                .color(MUTED)
                .size(16.0),
        );
        ui.add_space(28.0);
        let width = ui.available_width().min(560.0);
        Frame::new()
            .stroke(Stroke::new(
                if dropping { 2.0 } else { 1.0 },
                if dropping { ACCENT } else { LINE_STRONG },
            ))
            .corner_radius(CornerRadius::same(16))
            .inner_margin(Margin::symmetric(32, 28))
            .show(ui, |ui| {
                ui.set_width(width - 64.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(if dropping {
                            "Drop to open"
                        } else {
                            "Drop a folder of RAW files here"
                        })
                        .font(theme::semibold(17.0)),
                    );
                    ui.label(widgets::muted(
                        "It is checked first; nothing is written until you start.",
                    ));
                    ui.add_space(16.0);
                    ui.spacing_mut().button_padding = egui::vec2(18.0, 10.0);
                    if widgets::primary(ui, "Open a folder…").clicked()
                        && let Some(dir) = rfd::FileDialog::new().pick_folder()
                    {
                        action = Some(Action::Open(dir));
                    }
                    ui.add_space(6.0);
                    ui.label(widgets::muted(
                        "A processed output folder opens for review, an unfinished one to resume.",
                    ));
                });
            });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            let w = 330.0;
            ui.add_space((ui.available_width() - w).max(0.0) / 2.0);
            if ui.link("Open a scene file…").clicked()
                && let Some(file) = rfd::FileDialog::new()
                    .add_filter("Pictor scene", &["scene"])
                    .pick_file()
            {
                action = Some(Action::Open(file));
            }
            ui.label(RichText::new("·").color(MUTED));
            if ui.link("Show the demo scene").clicked() {
                action = Some(Action::Demo);
            }
        });
        if let Some(e) = error {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - width).max(0.0) / 2.0);
                ui.set_max_width(width);
                widgets::icon(ui, Icon::Warn, 18.0);
                ui.label(RichText::new(e).color(WARN));
            });
        }
    });
    let dropped = ui
        .ctx()
        .input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()));
    if let Some(path) = dropped {
        action = Some(Action::Open(path));
    }
    action
}
