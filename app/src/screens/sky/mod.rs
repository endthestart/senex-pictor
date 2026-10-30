//! The Sky & ground step (design board 3): the result with the sky tinted blue and the
//! uncertain boundary orange (O toggles the overlay), corrections painted over it and applied
//! in the background, and what the separation found: how much is clear sky, ground and
//! boundary, how often the ground hid the sky, the lens profile used, and, when the camera
//! drifted, where the mask came from.

pub mod brush;

use crate::import::job_card;
use crate::job::Job;
use crate::project::Run;
use crate::session::Session;
use crate::settings::Settings;
use crate::shell::{State, Step, StepRow};
use crate::theme::{MUTED, WARN};
use crate::viewer::{Canvas, Viewer};
use crate::widgets::{self, card, key, section, strong};
use brush::{Brush, RADIUS_RANGE, Tool};
use egui::{Color32, CornerRadius, RichText, Sense, Ui, vec2};
use pictor_engine::process;
use pictor_engine::separate::corrections;
use pictor_engine::store::Store;

const SKY: Color32 = Color32::from_rgb(60, 111, 191);
const EDGE: Color32 = Color32::from_rgb(240, 160, 64);

pub struct Sky {
    pub canvas: Canvas,
    pub brush: Brush,
    /// Corrections being applied: the run again from its matte on.
    pub applying: Option<Job<process::Report>>,
    pub error: Option<String>,
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
        }
    }

    /// Saves the strokes to the store and does the run again from its matte on.
    fn apply(&mut self, run: &Run, settings: Settings) {
        let (dir, out, strokes) = (
            run.store.dir.clone(),
            run.out.clone(),
            self.brush.strokes.clone(),
        );
        self.brush.tool = Tool::Move;
        self.error = None;
        self.applying = Some(Job::start("Applying the corrections", move |progress| {
            corrections::save(&Store::open(&dir)?, &strokes)?;
            let gpu = settings.gpu()?;
            process::correct(&out, gpu.as_ref(), &settings.budget(progress)?)
        }));
    }
}

fn separate(run: &Run) -> Option<&serde_json::Value> {
    run.report.stage("separate").map(|s| &s.report)
}

pub fn row(run: &Run) -> StepRow {
    let detail = separate(run)
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

fn share(v: &serde_json::Value) -> String {
    v.as_f64()
        .map_or("–".into(), |s| format!("{:.1}%", s * 100.0))
}

impl Sky {
    pub fn show(&mut self, ui: &mut Ui, viewer: &mut Viewer) {
        if self.applying.is_some() {
            self.brush.tool = Tool::Move;
        }
        self.canvas.paint = self.brush.tool != Tool::Move;
        let response = viewer.canvas(ui, &mut self.canvas);
        self.brush.paint(ui, &response, viewer);
    }

    fn corrections(&mut self, ui: &mut Ui, run: &Run) {
        card(ui, |ui| {
            section(ui, "Corrections");
            let busy = self.applying.is_some();
            ui.add_enabled_ui(!busy, |ui| {
                let tools = [Tool::Move, Tool::Sky, Tool::Ground];
                let at = tools.iter().position(|&t| t == self.brush.tool);
                if let Some(k) = widgets::segmented(ui, &["Move", "Paint sky", "Paint ground"], at)
                {
                    self.brush.tool = tools[k];
                }
                ui.horizontal(|ui| {
                    ui.label("Brush");
                    ui.add(
                        egui::Slider::new(&mut self.brush.radius, RADIUS_RANGE)
                            .logarithmic(true)
                            .suffix(" px")
                            .max_decimals(0),
                    );
                });
                widgets::key_hints(
                    ui,
                    &[
                        ("V S G", "move, sky, ground"),
                        ("[ ]", "brush size"),
                        ("⌘Z", "undo"),
                    ],
                );
                ui.horizontal(|ui| {
                    let some = !self.brush.strokes.is_empty();
                    if ui.add_enabled(some, egui::Button::new("Undo")).clicked() {
                        self.brush.strokes.pop();
                    }
                    if ui
                        .add_enabled(some, egui::Button::new("Clear all"))
                        .clicked()
                    {
                        self.brush.strokes.clear();
                    }
                });
            });
            let pending = self.brush.changed();
            if pending {
                ui.label(RichText::new("Not applied yet").color(WARN));
            }
            if ui
                .add_enabled_ui(pending && !busy, |ui| {
                    widgets::primary(ui, "Apply corrections")
                })
                .inner
                .clicked()
            {
                self.apply(run, Settings::get(ui.ctx()));
            }
            ui.label(widgets::muted(
                "Applying draws the sky and ground again from the night's saved statistics, \
                 then stacks and renders the result again.",
            ));
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(WARN));
            }
        });
        if let Some(job) = &self.applying
            && job_card(
                ui,
                "Applying corrections",
                job,
                "Cancelling leaves the run to resume from Import.",
            )
        {
            job.cancel();
        }
    }

    pub fn panel(&mut self, ui: &mut Ui, run: Option<&Run>) {
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
        let Some(sep) = separate(run) else {
            return;
        };
        let m = &sep["matte"];
        card(ui, |ui| {
            section(ui, "What the separation found");
            for (label, value) in [
                ("Clear sky", share(&m["sky_clear_share"])),
                ("Ground", share(&m["ground_share"])),
                ("Boundary", share(&m["edge_share"])),
                ("Sky seen through gaps", share(&m["transit_seen_share"])),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(value).monospace());
                    });
                });
            }
            let pixels = (run.store.manifest.width * run.store.manifest.height) as f64;
            if let Some(unstable) = m["unstable_pixels"].as_u64()
                && unstable > 0
            {
                ui.label(
                    RichText::new(format!(
                        "{:.1}% of the picture changed between sky and ground during the night \
                     (swaying branches, water); it takes the reference frame's ground.",
                        unstable as f64 / pixels * 100.0
                    ))
                    .color(WARN),
                );
            }
            if let Some(lit) = m["lit_pixels"].as_u64()
                && lit > 0
            {
                ui.label(widgets::muted(format!(
                    "{:.1}% was lit in a colour of its own where no star crossed (a lamp on \
                     leaves) and became ground; paint sky where that is wrong.",
                    lit as f64 / pixels * 100.0
                )));
            }
            if let Some(painted) = sep["corrected_pixels"].as_u64()
                && painted > 0
            {
                ui.label(widgets::muted(format!(
                    "{:.1}% was painted by hand.",
                    painted as f64 / pixels * 100.0
                )));
            }
            let occluded = &sep["occluded_share_percentiles"];
            if let Some(median) = occluded[1].as_f64() {
                ui.label(widgets::muted(format!(
                    "Half the frames had the ground hiding at most {:.1}% of the sky.",
                    median * 100.0
                )));
            }
        });
        let v = &sep["vignetting"];
        if let Some(lens) = v["lens"].as_str() {
            card(ui, |ui| {
                section(ui, "Lens");
                ui.label(strong(lens));
                ui.label(widgets::muted(format!(
                    "Vignetting profile at {} mm f/{} from lensfun, divided out before stacking",
                    v["focal_mm"], v["aperture"]
                )));
            });
        }
        let around = run.report.stage("composite around the reference").is_some();
        if around {
            let widest = run
                .report
                .stage("foreground drift")
                .and_then(|s| s.report["widest_shift_px"].as_f64())
                .map_or(String::new(), |w| format!(" up to {w:.0} px"));
            card(ui, |ui| {
                ui.label(strong("The camera drifted"));
                ui.label(widgets::muted(format!(
                    "The trees moved{widest} from the reference frame, so the mask is drawn \
                     around that frame, and each frame keeps its sky samples clear of where its \
                     own trees were."
                )));
            });
        }
    }
}
