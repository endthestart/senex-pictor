//! What the separation found, for the Sky & ground step's side panel: how much is clear sky,
//! ground and boundary, how often the ground hid the sky, the lens profile used, and, when the
//! camera drifted, where the mask came from.

use crate::project::Run;
use crate::theme::WARN;
use crate::widgets::{self, card, section, strong};
use egui::{RichText, Ui};

pub fn separate(run: &Run) -> Option<&serde_json::Value> {
    run.report.stage("separate").map(|s| &s.report)
}

fn share(v: &serde_json::Value) -> String {
    v.as_f64()
        .map_or("–".into(), |s| format!("{:.1}%", s * 100.0))
}

pub fn cards(ui: &mut Ui, run: &Run) {
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
