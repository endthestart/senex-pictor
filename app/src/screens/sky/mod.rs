//! The Sky & ground step (design board 3): the result with the sky tinted blue and the
//! uncertain boundary orange (O toggles the overlay), corrections painted over it and applied
//! in the background (`edit`), the regions the colour check made ground (`review`) and what
//! the separation found (`found`).

pub mod brush;
mod edit;
mod found;
mod review;

use crate::job::Job;
use crate::project::Run;
use crate::session::Session;
use crate::shell::{State, Step, StepRow};
use crate::theme::MUTED;
use crate::viewer::{Canvas, Viewer};
use crate::widgets::{self, card, key, section};
use brush::{Brush, Tool};
use egui::{Color32, CornerRadius, RichText, Sense, Ui, vec2};
use pictor_engine::process;
use pictor_engine::separate::corrections;

const SKY: Color32 = Color32::from_rgb(60, 111, 191);
const EDGE: Color32 = Color32::from_rgb(240, 160, 64);

pub struct Sky {
    pub canvas: Canvas,
    pub brush: Brush,
    /// Corrections being applied: the run again from its matte on.
    pub applying: Option<Job<process::Report>>,
    pub error: Option<String>,
    pub review: review::Review,
}

impl Sky {
    /// The step for `run`, with the corrections its store holds.
    pub fn new(run: Option<&Run>) -> Sky {
        let (saved, error) = match run.map(|r| corrections::load(&r.store)) {
            Some(Ok(strokes)) => (strokes, None),
            Some(Err(e)) => (Vec::new(), Some(e.to_string())),
            None => (Vec::new(), None),
        };
        Sky {
            canvas: Canvas {
                overlay: true,
                ..Canvas::single(false)
            },
            brush: Brush::new(saved),
            applying: None,
            error,
            review: run.map_or_else(Default::default, |r| review::Review::start(&r.store.dir)),
        }
    }
}

pub fn row(run: &Run) -> StepRow {
    let detail = found::separate(run)
        .and_then(|s| s["matte"]["sky_clear_share"].as_f64())
        .map_or("Separated".into(), |share| {
            format!("{:.0}% clear sky", share * 100.0)
        });
    StepRow {
        step: Step::Sky,
        state: State::Done,
        detail,
    }
}

pub fn keys(ctx: &egui::Context, s: &mut Session) {
    if !ctx.egui_wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::O)) {
        s.sky.canvas.overlay = !s.sky.canvas.overlay;
    }
    if s.run.is_some() && s.sky.applying.is_none() {
        s.sky.brush.keys(ctx);
    }
}

fn swatch(ui: &mut Ui, color: Color32, text: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(2), color);
    ui.label(RichText::new(text).size(12.5).color(MUTED));
}

impl Sky {
    pub fn show(&mut self, ui: &mut Ui, viewer: &mut Viewer) {
        if self.applying.is_some() {
            self.brush.tool = Tool::Move;
        }
        self.canvas.paint = self.brush.tool != Tool::Move;
        let response = viewer.canvas(ui, &mut self.canvas);
        self.review.draw(ui, &response, viewer);
        self.brush.paint(ui, &response, viewer);
    }

    pub fn panel(&mut self, ui: &mut Ui, viewer: &mut Viewer, run: Option<&Run>) {
        ui.spacing_mut().item_spacing.y = 10.0;
        card(ui, |ui| {
            section(ui, "Sky & ground");
            ui.horizontal(|ui| {
                if widgets::toggle(ui, self.canvas.overlay, "Overlay", Some("O")).clicked() {
                    self.canvas.overlay = !self.canvas.overlay;
                }
            });
            ui.horizontal_wrapped(|ui| {
                swatch(ui, SKY, "Sky");
                swatch(ui, EDGE, "Uncertain boundary");
                ui.label(RichText::new("Untinted: ground").size(12.5).color(MUTED));
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                key(ui, "F");
                key(ui, "1");
                key(ui, "2");
                ui.label(widgets::muted("fit, 100%, 200% · drag to pan"));
            });
        });
        let Some(run) = run else {
            return;
        };
        self.corrections(ui, run);
        self.review.panel(ui, viewer);
        found::cards(ui, run);
    }
}
