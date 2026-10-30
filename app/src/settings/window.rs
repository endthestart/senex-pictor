//! The Settings window (⌘, or the header's Settings): the GPU processing runs on, the memory
//! runs may use and where new runs go; saved when the owner clicks Save.

use super::{GpuChoice, Settings};
use crate::theme::{CARD, LINE, WARN};
use crate::widgets::{self, mono, muted, section};
use egui::{Align, Color32, ComboBox, CornerRadius, Frame, Layout, Margin, RichText, Stroke, vec2};
use pictor_engine::budget::{Budget, SHARE_OF_FREE};

const WIDTH: f32 = 480.0;
/// Space between the window's groups, more than within one so each reads as a unit.
const GROUP_GAP: f32 = 14.0;

#[derive(Default)]
pub struct Window {
    pub open: bool,
    draft: Option<Settings>,
    gpus: Vec<String>,
    installed_gb: Option<f64>,
    error: Option<String>,
}

fn gpu_label(choice: &GpuChoice) -> String {
    match choice {
        GpuChoice::Automatic => "Automatic: the most capable GPU".into(),
        GpuChoice::Off => "Off: the CPU only".into(),
        GpuChoice::Named(name) => name.clone(),
    }
}

impl Window {
    /// Opens the window on `current`.
    pub fn open(&mut self, current: Settings) {
        self.gpus = pictor_engine::gpu::Gpu::names();
        self.installed_gb = Budget::installed_gb().ok();
        self.draft = Some(current);
        self.error = None;
        self.open = true;
    }

    /// The window, while open; the settings once saved.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Settings> {
        if !self.open {
            return None;
        }
        let draft = self.draft.as_mut()?;
        let mut saved = None;
        let mut close = false;
        let frame = Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0, LINE))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::same(24));
        let modal = egui::Modal::new(egui::Id::new("settings"))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(160))
            .show(ctx, |ui| {
            ui.set_width(WIDTH);
            ui.spacing_mut().item_spacing.y = 8.0;
            widgets::heading(ui, "Settings");
            ui.add_space(GROUP_GAP);
            section(ui, "Processing GPU");
            ComboBox::from_id_salt("gpu")
                .width(ui.available_width())
                .selected_text(gpu_label(&draft.gpu))
                .show_ui(ui, |ui| {
                    let choices = std::iter::once(GpuChoice::Automatic)
                        .chain(self.gpus.iter().cloned().map(GpuChoice::Named))
                        .chain(std::iter::once(GpuChoice::Off));
                    for c in choices {
                        let label = gpu_label(&c);
                        ui.selectable_value(&mut draft.gpu, c, label);
                    }
                });
            ui.label(muted(
                "Stacking runs on the GPU when there is one; the CPU gives the same result, slower.",
            ));
            ui.add_space(GROUP_GAP);
            section(ui, "Memory");
            let mut fixed = draft.memory_gb.is_some();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 20.0;
                ui.radio_value(&mut fixed, false, "Automatic");
                ui.radio_value(&mut fixed, true, "At most");
            });
            match (fixed, self.installed_gb) {
                (false, _) => {
                    draft.memory_gb = None;
                    ui.label(muted(format!(
                        "{:.0}% of the memory free when a run starts.",
                        SHARE_OF_FREE * 100.0
                    )));
                }
                (true, Some(installed)) => {
                    let mut gb = draft.memory_gb.unwrap_or(installed / 2.0);
                    ui.add(
                        egui::Slider::new(&mut gb, 2.0..=installed.floor())
                            .suffix(" GB")
                            .max_decimals(0),
                    );
                    draft.memory_gb = Some(gb);
                    ui.label(muted(
                        "Runs use as many workers as fit; less memory is slower, not different.",
                    ));
                }
                (true, None) => {
                    ui.label(RichText::new("The installed memory is unknown.").color(WARN));
                }
            }
            ui.add_space(GROUP_GAP);
            section(ui, "New runs go to");
            ui.label(mono(draft.output.as_ref().map_or_else(
                || "~/Pictures/Pictor".to_string(),
                |p| p.display().to_string(),
            )));
            ui.horizontal(|ui| {
                if widgets::secondary(ui, "Choose…").clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    draft.output = Some(dir);
                }
                if draft.output.is_some() && ui.link("Use ~/Pictures/Pictor").clicked() {
                    draft.output = None;
                }
            });
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(WARN));
            }
            ui.add_space(GROUP_GAP);
            let row = vec2(ui.available_width(), 38.0);
            ui.allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| {
                if widgets::primary(ui, "Save").clicked() {
                    match draft.save() {
                        Ok(()) => {
                            saved = Some(draft.clone());
                            close = true;
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
                if widgets::secondary(ui, "Cancel").clicked() {
                    close = true;
                }
            });
        });
        if close || modal.should_close() {
            self.open = false;
            self.draft = None;
        }
        saved
    }
}
