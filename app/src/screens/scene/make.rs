//! Making a hero composite or a moment in the background (the engine's `hero` and `moment`
//! stages, then a preview rendering), and choosing the frame they are made around.

use super::SceneStep;
use crate::job::Job;
use crate::project::Run;
use crate::session::Session;
use crate::settings::Settings;
use crate::theme::{ACCENT, CARD, LINE, MUTED, WARN, radius};
use crate::widgets::{self, card, section};
use egui::{Color32, CornerRadius, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use pictor_engine::integrate::{cfa, hero, moment};
use pictor_engine::store::Store;
use pictor_engine::{process, render, review};
use std::path::Path;

pub struct Making {
    pub label: String,
    /// Makes the result; its name.
    pub job: Job<String>,
    /// The preview file the result renders to, read again once made.
    preview: Option<String>,
}

/// Renders result `name` of the store in `out` to `out/name.jpg` for its preview.
fn preview(store: &Store, out: &Path, name: &str) -> pictor_engine::Result<()> {
    review::render(
        store,
        &review::Options {
            planes: name,
            look: render::NATURAL,
            matrix: None,
            out: &out.join(name),
            jpeg: Some(review::SIDE),
            tiff: false,
            single: false,
            linear: false,
            hdr: None,
        },
    )?;
    Ok(())
}

impl Making {
    /// A hero composite around processed frame `frame`.
    pub fn hero(run: &Run, frame: usize, settings: Settings) -> Making {
        let name = run.store.manifest.frames[frame].name.clone();
        let (dir, out, lens) = (
            run.store.dir.clone(),
            run.out.clone(),
            run.report.lens.clone(),
        );
        Making {
            preview: None,
            label: format!("Making the hero composite around {name}"),
            job: Job::start("Making a hero composite", move |progress| {
                let store = Store::open(&dir)?;
                let gpu = settings.gpu()?;
                let result = process::hero_name(&name);
                let options = hero::Options {
                    hero: frame,
                    foreground: hero::Foreground::Frame,
                    first: 0,
                    labels: None,
                    lens: lens.as_deref(),
                    kernel: cfa::KERNEL,
                    gpu: gpu.as_ref(),
                    name: &result,
                };
                hero::hero(&store, &options, &settings.budget(progress)?)?;
                preview(&store, &out, &result)?;
                Ok(result)
            }),
        }
    }

    /// A moment of the frames around processed frame `frame`.
    pub fn moment(run: &Run, frame: usize, settings: Settings) -> Making {
        let name = run.store.manifest.frames[frame].name.clone();
        let (dir, out) = (run.store.dir.clone(), run.out.clone());
        Making {
            preview: None,
            label: format!("Making the moment at {name}"),
            job: Job::start("Making a moment", move |progress| {
                let store = Store::open(&dir)?;
                moment::moment(&store, frame, moment::HALF, &settings.budget(progress)?)?;
                let result = format!("moment-{}", name.split('.').next().unwrap_or(&name));
                preview(&store, &out, &result)?;
                Ok(result)
            }),
        }
    }

    /// The run's own composite again, with moving foreground sharp or blurred.
    pub fn recompose(run: &Run, sharp_moving: bool, settings: Settings) -> Making {
        let out = run.out.clone();
        Making {
            preview: Some("final".into()),
            label: if sharp_moving {
                "Taking moving branches sharp from the reference minutes".into()
            } else {
                "Blurring moving branches in the long exposure".into()
            },
            job: Job::start("Redoing the composite", move |progress| {
                let gpu = settings.gpu()?;
                let report = process::redo(
                    &out,
                    process::Redo::Composite { sharp_moving },
                    gpu.as_ref(),
                    &settings.budget(progress)?,
                )?;
                report
                    .result
                    .ok_or_else(|| pictor_engine::error("the composite was not rendered"))
            }),
        }
    }

    pub fn card(&self, ui: &mut Ui) {
        let s = self.job.snapshot();
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&self.label);
            });
            if let Some(step) = &s.step {
                let count = if s.total > 0 {
                    format!(" · {} of {}", s.done, s.total)
                } else {
                    String::new()
                };
                ui.label(widgets::muted(format!("{step}{count}")));
                if s.total > 0 {
                    widgets::bar(ui, s.done as f32 / s.total as f32, 4.0);
                }
            }
            ui.label(widgets::muted(format!(
                "{} so far",
                widgets::duration(self.job.started.elapsed().as_secs_f64())
            )));
        });
    }
}

impl SceneStep {
    /// Advances the job; the result name once made.
    pub fn poll(&mut self, out: &Path) -> Option<String> {
        let making = self.making.as_ref()?;
        let result = making.job.poll()?;
        let preview = making.preview.clone();
        self.making = None;
        match result {
            Ok(name) => {
                let file = preview.unwrap_or_else(|| name.clone());
                self.refresh(&out.join(format!("{file}.jpg")));
                Some(name)
            }
            Err(e) => {
                self.error = Some(e.to_string());
                None
            }
        }
    }
}

/// The frames to make a hero or moment around: the lit ones, or every frame.
pub fn picker(ui: &mut Ui, s: &mut Session, frames: &[String], lit: &[String]) {
    let Some(run) = &s.run else {
        return;
    };
    let ctx = ui.ctx().clone();
    card(ui, |ui| {
        let every = s.scene.all_frames || lit.is_empty();
        ui.horizontal(|ui| {
            section(
                ui,
                match (every, lit.is_empty()) {
                    (_, true) => "Frame to compose around: every frame (none is lit)",
                    (true, false) => "Frame to compose around: every frame",
                    (false, false) => "Frame to compose around: lit frames",
                },
            );
            if !lit.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if every {
                        "Show lit frames"
                    } else {
                        "Show every frame"
                    };
                    if ui.link(label).clicked() {
                        s.scene.all_frames = !s.scene.all_frames;
                    }
                });
            }
        });
        let shown: Vec<usize> = (0..frames.len())
            .filter(|&i| every || lit.contains(&frames[i]))
            .collect();
        let cell = vec2(150.0, 124.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for &i in &shown {
                let (rect, response) = ui.allocate_exact_size(cell, Sense::click());
                let on = s.scene.frame == Some(i);
                let p = ui.painter();
                p.rect_filled(rect, CornerRadius::same(radius::TILE), CARD);
                p.rect_stroke(
                    rect,
                    CornerRadius::same(radius::TILE),
                    Stroke::new(if on { 2.0 } else { 1.0 }, if on { ACCENT } else { LINE }),
                    StrokeKind::Inside,
                );
                let image = Rect::from_min_size(
                    rect.min + vec2(5.0, 5.0),
                    vec2(cell.x - 10.0, cell.y - 32.0),
                );
                if let (Some(k), Some(thumbs)) = (
                    run.survey.captures.iter().position(|c| c.name == frames[i]),
                    s.thumbs.as_mut(),
                ) && let Some(t) = thumbs.texture(&ctx, k)
                {
                    p.image(
                        t.id(),
                        image,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                let short = frames[i].split('.').next().unwrap_or(&frames[i]);
                p.text(
                    pos2(rect.min.x + 8.0, rect.max.y - 22.0),
                    egui::Align2::LEFT_TOP,
                    short,
                    egui::FontId::monospace(11.0),
                    MUTED,
                );
                if lit.contains(&frames[i]) {
                    p.text(
                        pos2(rect.max.x - 8.0, rect.max.y - 22.0),
                        egui::Align2::RIGHT_TOP,
                        "Lit",
                        egui::FontId::proportional(11.0),
                        WARN,
                    );
                }
                if response.clicked() {
                    s.scene.frame = Some(i);
                }
            }
        });
        if let Some(i) = s.scene.frame {
            ui.label(
                RichText::new(format!(
                    "{} as the photograph. Any frame can be chosen.",
                    frames[i]
                ))
                .color(MUTED),
            );
        }
    });
}
