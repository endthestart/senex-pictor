//! Exporting the result: the editable linear master, the look as a 16-bit TIFF, or a JPEG,
//! rendered by the engine in the background into the output folder.

use super::LookStep;
use crate::job::Job;
use crate::project::Run;
use crate::theme::MUTED;
use crate::viewer::Viewer;
use crate::widgets::{self, card, mono, section};
use egui::{RichText, Ui};
use star_photos::review;
use star_photos::store::Store;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq)]
pub enum Format {
    Master,
    Tiff,
    Jpeg,
}

/// `dir/name.ext`, numbered when taken.
fn free_stem(dir: &Path, name: &str, ext: &str) -> PathBuf {
    let mut stem = dir.join(name);
    let mut k = 2;
    while stem.with_extension(ext).exists() {
        stem = dir.join(format!("{name} {k}"));
        k += 1;
    }
    stem
}

impl LookStep {
    pub(super) fn export_card(
        &mut self,
        ui: &mut Ui,
        viewer: &Viewer,
        run: Option<&Run>,
        result: Option<&str>,
    ) {
        if let Some(job) = &self.export
            && let Some(done) = job.poll()
        {
            self.export = None;
            match done {
                Ok(files) => self.exported = files,
                Err(e) => self.error = Some(format!("Export failed: {e}")),
            }
        }
        card(ui, |ui| {
            section(ui, "Export");
            let (Some(run), Some(result)) = (run, result) else {
                ui.label(widgets::muted("Export needs a processed folder."));
                return;
            };
            for (format, name, note) in [
                (
                    Format::Master,
                    "Linear 32-bit float TIFF",
                    "Editable master for another editor; the look is not applied",
                ),
                (
                    Format::Tiff,
                    "16-bit TIFF, as shown",
                    "This look at full resolution",
                ),
                (
                    Format::Jpeg,
                    "JPEG",
                    "This look, 2400 px on the long side, for sharing",
                ),
            ] {
                ui.radio_value(&mut self.format, format, name);
                ui.label(RichText::new(note).size(12.0).color(MUTED));
            }
            ui.label(mono(run.out.display().to_string()));
            if self.export.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Exporting…");
                });
            } else if widgets::primary(ui, "Export").clicked() {
                self.start_export(viewer, run, result);
            }
            for f in &self.exported {
                ui.label(mono(format!("Saved {f}")));
            }
        });
    }

    fn start_export(&mut self, viewer: &Viewer, run: &Run, result: &str) {
        let look_name = viewer.names[0].to_lowercase();
        let (name, ext) = match self.format {
            Format::Master => (format!("{result}-linear-master"), "tif"),
            Format::Tiff => (format!("{result}-{look_name}"), "tif"),
            Format::Jpeg => (format!("{result}-{look_name}"), "jpg"),
        };
        let stem = free_stem(&run.out, &name, ext);
        let (jpeg, tiff, linear) = match self.format {
            Format::Master => (None, false, true),
            Format::Tiff => (None, true, false),
            Format::Jpeg => (Some(review::SIDE), false, false),
        };
        let (dir, result, look) = (run.store.dir.clone(), result.to_string(), viewer.look[0]);
        self.error = None;
        self.export = Some(Job::start("Exporting", move |_| {
            let store = Store::open(&dir)?;
            let report = review::render(
                &store,
                &review::Options {
                    planes: &result,
                    look,
                    matrix: None,
                    out: &stem,
                    jpeg,
                    tiff,
                    single: false,
                    linear,
                },
            )?;
            Ok(report.files)
        }));
    }
}
