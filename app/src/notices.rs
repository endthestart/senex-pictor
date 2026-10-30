//! Third-Party Notices (the Pictor menu, or Settings): every component Pictor ships and its
//! licence, written by `tools/notices.sh` (cargo-about).

use crate::widgets::{self, muted};
use egui::{Key, Modifiers, RichText, ScrollArea, TextStyle};

const TEXT: &str = include_str!("../assets/notices.txt");

#[derive(Default)]
pub struct Notices {
    open: bool,
    lines: Vec<&'static str>,
}

impl Notices {
    pub fn open(&mut self) {
        if self.lines.is_empty() {
            self.lines = TEXT.lines().collect();
        }
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let mut close = false;
        let height = ctx.content_rect().height() * 0.6;
        let modal = widgets::dialog("notices").show(ctx, |ui| {
            ui.set_width(720.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            widgets::heading(ui, "Third-Party Notices");
            ui.label(muted(
                "Pictor is MIT OR Apache-2.0. It ships these components under their own licences.",
            ));
            let row = ui.text_style_height(&TextStyle::Monospace);
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ScrollArea::both()
                    .max_height(height)
                    .auto_shrink([false, true])
                    .show_rows(ui, row, self.lines.len(), |ui, range| {
                        for line in &self.lines[range] {
                            ui.add(egui::Label::new(RichText::new(*line).monospace()).extend());
                        }
                    });
            });
            widgets::dialog_buttons(ui, |ui| {
                let enter = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                if widgets::primary(ui, "Close").clicked() || enter {
                    close = true;
                }
            });
        });
        if close || modal.should_close() {
            self.open = false;
        }
    }
}
