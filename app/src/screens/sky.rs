//! The Sky & ground step (design board 3): the result with the sky tinted blue and the
//! uncertain boundary orange (O toggles the overlay), and what the separation found: how much
//! is clear sky, ground and boundary, how often the ground hid the sky, the lens profile used,
//! and, when the camera drifted, where the mask came from.

use crate::project::Run;
use crate::session::Session;
use crate::shell::{State, Step, StepRow};
use crate::theme::{MUTED, WARN};
use crate::viewer::{Canvas, Viewer};
use crate::widgets::{self, card, key, section, strong};
use egui::{Color32, CornerRadius, RichText, Sense, Ui, vec2};

const SKY: Color32 = Color32::from_rgb(60, 111, 191);
const EDGE: Color32 = Color32::from_rgb(240, 160, 64);

pub struct Sky {
    pub canvas: Canvas,
}

impl Default for Sky {
    fn default() -> Sky {
        Sky {
            canvas: Canvas {
                overlay: true,
                ..Canvas::single(false)
            },
        }
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
        viewer.canvas(ui, &mut self.canvas);
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
