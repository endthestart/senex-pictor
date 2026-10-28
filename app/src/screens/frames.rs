//! The Frames step (design board 2): what was used and why. Counts of lit, overexposed and
//! lost frames, each frame's foreground drift, every processed frame as a thumbnail with its
//! flag, and the chosen frame larger, blinked against the reference frame with B.

mod details;
mod facts;
mod strip;

pub use facts::Facts;

use crate::job::Job;
use crate::project::Run;
use crate::session::Session;
use crate::shell::{State, Step, StepRow};
use crate::theme::{TEXT, WARN};
use crate::widgets;
use egui::{ColorImage, TextureHandle, TextureOptions, Ui};

#[derive(Default)]
pub struct Frames {
    /// The chosen frame (index into the processed frames).
    pub selected: usize,
    pub blink: bool,
    previews: Vec<(usize, TextureHandle)>,
    loading: Option<(usize, Job<ColorImage>)>,
    /// The frame the filmstrip last scrolled to.
    followed: Option<usize>,
}

pub fn row(run: &Run) -> StepRow {
    let f = Facts::of(run);
    let drift = f
        .widest
        .map_or(String::new(), |w| format!(" · drift {w:.0} px"));
    StepRow {
        step: Step::Frames,
        state: if f.lost.is_empty() {
            State::Done
        } else {
            State::Attention
        },
        detail: format!(
            "{} lit{drift}",
            f.lit.iter().filter(|n| f.names.contains(n)).count()
        ),
    }
}

impl Frames {
    pub fn loading(&self) -> bool {
        self.loading.is_some()
    }

    /// Keeps a preview once it has been read.
    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some((k, job)) = &self.loading else {
            return;
        };
        let Some(result) = job.poll() else {
            return;
        };
        let k = *k;
        self.loading = None;
        if let Ok(image) = result {
            let t = ctx.load_texture(format!("frame {k}"), image, TextureOptions::LINEAR);
            self.previews.push((k, t));
            let keep = self.previews.len().saturating_sub(6);
            self.previews.drain(..keep);
        }
    }

    fn texture(
        &mut self,
        ctx: &egui::Context,
        s: &Option<Run>,
        stretch: Option<crate::thumbs::Stretch>,
        i: usize,
    ) -> Option<TextureHandle> {
        self.poll(ctx);
        if let Some((_, t)) = self.previews.iter().find(|(k, _)| *k == i) {
            return Some(t.clone());
        }
        if self.loading.is_some() {
            return None;
        }
        let (Some(run), Some(stretch)) = (s, stretch) else {
            return None;
        };
        let dir = run.store.dir.clone();
        self.loading = Some((
            i,
            Job::start("Reading the frame", move |_| {
                facts::preview(&dir, i, stretch)
            }),
        ));
        None
    }
}

pub fn keys(ctx: &egui::Context, s: &mut Session) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let n = s.run.as_ref().map_or(0, |r| r.store.manifest.frames.len());
    ctx.input(|i| {
        if i.key_pressed(egui::Key::B) {
            s.frames.blink = !s.frames.blink;
        }
        if i.key_pressed(egui::Key::ArrowRight) && s.frames.selected + 1 < n {
            s.frames.selected += 1;
            s.frames.blink = false;
        }
        if i.key_pressed(egui::Key::ArrowLeft) && s.frames.selected > 0 {
            s.frames.selected -= 1;
            s.frames.blink = false;
        }
    });
}

pub fn show(ui: &mut Ui, s: &mut Session) {
    let Some(run) = &s.run else {
        return;
    };
    let f = Facts::of(run);
    let lit_used = f.lit.iter().filter(|n| f.names.contains(n)).count();
    widgets::heading(ui, "Frames");
    let drift = match (f.widest, &f.reference) {
        (Some(w), Some(r)) if w > star_photos::process::DRIFT_PX => format!(
            " The foreground drifted, up to {w:.0} px from the middle frame, so the image is built around {r}."
        ),
        (Some(w), _) => format!(" The foreground held still ({w:.1} px at most)."),
        _ => String::new(),
    };
    let lit = match lit_used {
        0 => "None is lit by a torch or a car.".to_string(),
        n => format!(
            "{n} are lit by a torch or a car: they stay available for a hero frame and are left \
             out of the long-exposure foreground."
        ),
    };
    ui.label(widgets::muted(format!(
        "Using {} frames. {lit}{drift}",
        f.names.len()
    )));
    ui.add_space(10.0);
    ui.spacing_mut().item_spacing.y = 12.0;
    widgets::tiles(
        ui,
        &[
            ("Frames", f.names.len().to_string(), TEXT),
            (
                "Lit",
                lit_used.to_string(),
                if lit_used > 0 { WARN } else { TEXT },
            ),
            (
                "Overexposed, left out",
                f.overexposed.len().to_string(),
                TEXT,
            ),
            (
                "Foreground lost",
                f.lost.len().to_string(),
                if f.lost.is_empty() { TEXT } else { WARN },
            ),
            (
                "Stars per frame",
                f.stars.map_or("–".into(), |n| n.to_string()),
                TEXT,
            ),
        ],
    );
    let wide = ui.available_width() > 1000.0;
    let total = ui.available_width();
    s.frames.selected = s.frames.selected.min(f.names.len().saturating_sub(1));
    if wide {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(total * 0.62 - 12.0);
                strips(ui, s, &f);
            });
            ui.vertical(|ui| {
                ui.set_width(total * 0.38 - 12.0);
                details::show(ui, s, &f);
            });
        });
    } else {
        strips(ui, s, &f);
        details::show(ui, s, &f);
    }
}

fn strips(ui: &mut Ui, s: &mut Session, f: &Facts) {
    let selected = s.frames.selected;
    if !f.drift.is_empty()
        && let Some(i) = strip::drift_chart(ui, f, selected)
    {
        s.frames.selected = i;
        s.frames.blink = false;
    }
    if let (Some(thumbs), Some(run)) = (s.thumbs.as_mut(), s.run.as_ref())
        && let Some(i) = strip::filmstrip(
            ui,
            thumbs,
            &run.survey,
            f,
            selected,
            s.frames.followed != Some(selected),
        )
    {
        s.frames.selected = i;
        s.frames.blink = false;
    }
    s.frames.followed = Some(s.frames.selected);
}
