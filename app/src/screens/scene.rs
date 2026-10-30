//! The Scene step (design board 4): the stacked sky is the same in every treatment; what
//! changes is the foreground and which moment the picture keeps. The long exposure is the
//! run's own result; a hero frame (one frame as shot, its light painting kept) and a
//! short-window moment (for skies that change in minutes) are made here in the background and
//! opened like any result.

mod make;
mod previews;

use crate::job::Job;
use crate::project::Run;
use crate::session::Session;
use crate::theme::{ACCENT, LINE, MUTED};
use crate::widgets::{self, Badge, strong};
use egui::{ColorImage, RichText, Stroke, TextureHandle, Ui};
use std::path::PathBuf;

pub use make::Making;

#[derive(Default)]
pub struct SceneStep {
    /// The frame chosen for a hero composite or a moment (index into the processed frames).
    pub frame: Option<usize>,
    all_frames: bool,
    pub making: Option<Making>,
    previews: Vec<(PathBuf, TextureHandle)>,
    loading: Vec<(PathBuf, Job<ColorImage>)>,
    /// A result to open, for the app.
    pub open: Option<String>,
    pub error: Option<String>,
}

pub fn detail(s: &Session) -> String {
    match s.result.as_deref() {
        Some("final" | "hero") => "Long exposure".into(),
        Some(r) if r.starts_with("hero-") => format!("Hero frame {}", &r[5..]),
        Some(r) if r.starts_with("moment-") => format!("Moment at {}", &r[7..]),
        Some(r) => r.into(),
        None => "Scene file".into(),
    }
}

struct Treatment {
    title: &'static str,
    note: String,
    result: Option<String>,
    tag: Option<(&'static str, Badge)>,
}

fn treatments(s: &Session, run: &Run, frame: Option<&str>) -> Vec<Treatment> {
    let stem = frame.map(|f| f.split('.').next().unwrap_or(f).to_string());
    let own = run.report.result.clone();
    let unlit = run.report.frames.saturating_sub(lit_frames(run).len());
    let current = |r: &Option<String>| r.is_some() && r.as_deref() == s.result.as_deref();
    let long = Treatment {
        title: "Long exposure",
        note: if run.report.sharp_moving {
            format!(
                "Foreground averaged over the {unlit} unlit frames; swaying branches and water \
                 come sharp from the reference minutes, a little noisier."
            )
        } else {
            format!(
                "Foreground averaged over the {unlit} unlit frames. Calm and clean; swaying \
                 branches go soft."
            )
        },
        result: own.clone(),
        tag: Some(("Recommended", Badge::Strong)),
    };
    let hero = Treatment {
        title: "Hero frame",
        note: "One frame as shot in front of the stacked sky. Its light painting and lit \
               details stay."
            .into(),
        result: stem.as_ref().map(|f| format!("hero-{f}")),
        tag: None,
    };
    let possible = pictor_engine::integrate::moment::possible(&run.store);
    let moment = Treatment {
        title: "Short-window moment",
        note: if possible {
            "For skies that change in minutes, like aurora: nine frames around one moment \
             plus the deep stars."
                .into()
        } else {
            "Needs the whole-sequence stack; this run drifted, so it was composed around one \
             frame instead."
                .into()
        },
        result: stem.filter(|_| possible).map(|f| format!("moment-{f}")),
        tag: None,
    };
    let mut all = vec![long, hero, moment];
    for t in &mut all {
        if current(&t.result) {
            t.tag = Some(("Showing", Badge::Strong));
        } else if t.title != "Long exposure" {
            t.tag = None;
        }
    }
    all
}

pub fn lit_frames(run: &Run) -> Vec<String> {
    let lit = run
        .survey
        .check
        .runs
        .get(run.report.run)
        .map(|r| r.lit.clone())
        .unwrap_or_default();
    run.store
        .manifest
        .frames
        .iter()
        .filter(|f| lit.contains(&f.name))
        .map(|f| f.name.clone())
        .collect()
}

pub fn show(ui: &mut Ui, s: &mut Session) {
    let Some(run) = &s.run else {
        return;
    };
    widgets::heading(ui, "Scene");
    ui.label(widgets::muted(
        "The stacked sky is the same in every treatment. What changes is the foreground and \
         which moment the picture keeps.",
    ));
    ui.add_space(10.0);
    ui.spacing_mut().item_spacing.y = 12.0;
    let frames: Vec<String> = run
        .store
        .manifest
        .frames
        .iter()
        .map(|f| f.name.clone())
        .collect();
    let lit = lit_frames(run);
    if s.scene.frame.is_none() {
        s.scene.frame = lit
            .first()
            .and_then(|n| frames.iter().position(|f| f == n))
            .or(Some(frames.len() / 2));
    }
    let chosen = s.scene.frame.and_then(|i| frames.get(i)).cloned();
    let list = treatments(s, run, chosen.as_deref());
    let ctx = ui.ctx().clone();
    let settings = crate::settings::Settings::get(&ctx);
    let width = ((ui.available_width() - 24.0) / 3.0).max(240.0);
    let out = run.out.clone();
    let existing = run.results().unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        for t in &list {
            ui.vertical(|ui| {
                ui.set_width(width);
                let made = t.result.as_ref().is_some_and(|r| existing.contains(r));
                let preview_path = t.result.as_ref().map(|r| {
                    out.join(if Some(r) == run.report.result.as_ref() {
                        "final.jpg".to_string()
                    } else {
                        format!("{r}.jpg")
                    })
                });
                let stroke = if t.tag.is_some_and(|(n, _)| n == "Showing") {
                    ACCENT
                } else {
                    LINE
                };
                egui::Frame::new()
                    .fill(crate::theme::CARD)
                    .stroke(Stroke::new(
                        if stroke == ACCENT { 2.0 } else { 1.0 },
                        stroke,
                    ))
                    .corner_radius(12)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.set_width(width - 20.0);
                        let size = egui::vec2(width - 20.0, (width - 20.0) / 1.5);
                        match preview_path
                            .as_deref()
                            .and_then(|p| s.scene.preview(&ctx, p))
                        {
                            Some(tex) => {
                                ui.add(
                                    egui::Image::new(&tex)
                                        .fit_to_exact_size(size)
                                        .corner_radius(6),
                                );
                            }
                            None => {
                                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                                ui.painter().rect_filled(rect, 6, crate::theme::CHROME);
                                let note = match preview_path.as_deref() {
                                    None => "Not available for this run",
                                    Some(p) if p.exists() => "Reading the preview…",
                                    _ if made => "No preview rendered",
                                    _ => "Not made yet",
                                };
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    note,
                                    egui::FontId::proportional(13.0),
                                    MUTED,
                                );
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label(strong(t.title));
                            if let Some((tag, kind)) = t.tag {
                                widgets::badge(ui, tag, kind);
                            }
                        });
                        ui.label(RichText::new(&t.note).size(12.5).color(MUTED));
                        if t.title == "Long exposure" && s.scene.making.is_none() {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Moving branches").size(12.5));
                                let now = usize::from(run.report.sharp_moving);
                                if let Some(k) =
                                    widgets::segmented(ui, &["Blurred", "Sharp"], Some(now))
                                    && k != now
                                {
                                    s.scene.making =
                                        Some(Making::recompose(run, k == 1, settings.clone()));
                                }
                            });
                        }
                        let Some(result) = &t.result else {
                            return;
                        };
                        let showing = Some(result.as_str()) == s.result.as_deref();
                        if made && !showing {
                            if widgets::secondary(ui, "Open in Compare").clicked() {
                                s.scene.open = Some(result.clone());
                            }
                        } else if !made
                            && s.scene.making.is_none()
                            && widgets::primary(ui, "Make It").clicked()
                        {
                            match (t.title, s.scene.frame) {
                                ("Hero frame", Some(i)) => {
                                    s.scene.making = Some(Making::hero(run, i, settings.clone()))
                                }
                                ("Short-window moment", Some(i)) => {
                                    s.scene.making = Some(Making::moment(run, i, settings.clone()))
                                }
                                _ => {}
                            }
                        }
                    });
            });
        }
    });
    if let Some(m) = &s.scene.making {
        m.card(ui);
    }
    if let Some(e) = &s.scene.error {
        ui.colored_label(crate::theme::WARN, e);
    }
    make::picker(ui, s, &frames, &lit);
}
