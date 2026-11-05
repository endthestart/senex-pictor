//! "Before you start": what the run will use and whether it can run, worked out once per
//! choice of frames, lens and output folder. Problems that would stop the run part-way
//! (no lens profile, no colour matrix, too little disk) are found here instead.

use super::Import;
use super::Lookups;
use crate::theme::{GOOD, MUTED, WARN};
use crate::widgets::{Icon, bytes, duration};
use egui::Color32;
use pictor_engine::cameras;
use pictor_engine::process::{self, disk};

pub struct Check {
    pub icon: Icon,
    pub title: &'static str,
    pub lines: Vec<(String, Color32)>,
}

pub struct Checks {
    pub items: Vec<Check>,
    /// Why the run cannot start, when it cannot.
    pub blocking: Option<String>,
    /// Lenses with a profile at the frames' focal length, when the files record no lens.
    pub lens_choices: Option<Vec<String>>,
    /// About how long the run takes (s).
    pub estimate_s: f64,
}

fn line(text: impl Into<String>) -> (String, Color32) {
    (text.into(), MUTED)
}

/// Seconds per unit of plan weight on the machine the weights were measured on (8 threads,
/// 24 MP frames); scaled by frame size and threads here.
const MEASURED_THREADS: f64 = 8.0;
const MEASURED_PIXELS: f64 = 24.3e6;

/// "NIKON D700", not "NIKON CORPORATION NIKON D700": the model, with the maker's first word
/// when the model does not start with it.
pub fn camera_name(make: &str, model: &str) -> String {
    let maker = make.split_whitespace().next().unwrap_or("");
    if model.to_lowercase().starts_with(&maker.to_lowercase()) {
        model.to_string()
    } else {
        format!("{maker} {model}").trim().to_string()
    }
}

impl Checks {
    pub fn of(im: &Import, look: &mut Lookups) -> Checks {
        let mut c = Checks {
            items: Vec::new(),
            blocking: None,
            lens_choices: None,
            estimate_s: 0.0,
        };
        let (Some(survey), Some(run)) = (&im.survey, im.chosen_run()) else {
            return c;
        };
        let selected = im.selected();
        let Some(&first) = selected.first() else {
            c.blocking = Some("No frames chosen".into());
            return c;
        };
        let names: Vec<&String> = selected.iter().map(|&i| &survey.captures[i].name).collect();
        let over = (im.first..=im.last)
            .filter(|&i| run.overexposed.contains(&survey.captures[i].name))
            .count();
        let lit = run.lit.iter().filter(|n| names.contains(n)).count();
        c.items.push(Check {
            icon: Icon::Done,
            title: "Frames",
            lines: vec![line(format!(
                "{} frames; {over} overexposed left out; {lit} lit, kept for a hero frame",
                selected.len()
            ))],
        });
        let s = &survey.captures[first].settings;
        let lens_name = im.lens.as_deref().or(s.lens.as_deref());
        match (lens_name, s.focal_mm.zip(s.aperture)) {
            (_, None) => {
                c.blocking = Some("No focal length or aperture recorded".into());
                c.items.push(Check {
                    icon: Icon::Failed,
                    title: "Lens",
                    lines: vec![(
                        "The files record no focal length or aperture; vignetting needs them"
                            .into(),
                        WARN,
                    )],
                });
            }
            (None, Some((focal, _))) => {
                c.blocking = Some("Choose the lens".into());
                let (choices, note) = match look.candidates(focal) {
                    Ok(l) => (
                        l,
                        format!("Lenses with a profile at {focal:.0} mm are listed"),
                    ),
                    Err(e) => (Vec::new(), e),
                };
                c.lens_choices = Some(choices);
                c.items.push(Check {
                    icon: Icon::Warn,
                    title: "Lens",
                    lines: vec![
                        ("The files record no lens; vignetting needs it".into(), WARN),
                        line(note),
                    ],
                });
            }
            (Some(name), Some((focal, aperture))) => {
                match look.profile(s.make.as_deref(), name, focal, aperture) {
                    Ok((f, a)) => c.items.push(Check {
                        icon: Icon::Done,
                        title: "Lens",
                        lines: vec![line(format!(
                            "Vignetting profile for {name} at {f:.0} mm f/{a} (lensfun)"
                        ))],
                    }),
                    Err(e) => {
                        c.blocking = Some("No vignetting profile for the lens".into());
                        c.items.push(Check {
                            icon: Icon::Failed,
                            title: "Lens",
                            lines: vec![(e.to_string(), WARN)],
                        });
                    }
                }
            }
        }
        let (make, model) = (
            s.make.clone().unwrap_or_default(),
            s.model.clone().unwrap_or_default(),
        );
        let camera = camera_name(&make, &model);
        match cameras::built_in_matrix(&make, &model) {
            Ok(Some(_)) => c.items.push(Check {
                icon: Icon::Done,
                title: "Colour",
                lines: vec![line(format!("Built-in matrix for the {camera}"))],
            }),
            Ok(None) | Err(_) => {
                c.blocking = Some("No colour matrix for the camera".into());
                c.items.push(Check {
                    icon: Icon::Failed,
                    title: "Colour",
                    lines: vec![(format!("No built-in colour matrix for the {camera}"), WARN)],
                });
            }
        }
        let threads = match look.budget(&im.settings) {
            Ok((budget, threads)) => {
                c.items.push(Check {
                    icon: Icon::Done,
                    title: "Memory",
                    lines: vec![line(format!(
                        "{} budget ({}) on {threads} threads",
                        bytes(budget as f64),
                        if im.settings.memory_gb.is_some() {
                            "set in Settings"
                        } else {
                            "75% of free memory"
                        },
                    ))],
                });
                threads
            }
            Err(e) => {
                c.blocking = Some("Free memory unknown".into());
                c.items.push(Check {
                    icon: Icon::Failed,
                    title: "Memory",
                    lines: vec![(e, WARN)],
                });
                1
            }
        };
        let pixels = look.pixels(&survey.paths[first]);
        let need = pixels
            .as_ref()
            .map_or(0.0, |&p| disk::store_bytes(p, selected.len()));
        let resuming = im.interrupted.is_some();
        match (
            disk::free_bytes(&im.out).map_err(|e| e.to_string()),
            pixels.clone(),
        ) {
            (Ok(free), Ok(_)) if resuming || need <= free => c.items.push(Check {
                icon: Icon::Done,
                title: "Disk",
                lines: vec![line(format!(
                    "{} frames need about {}; {} free there",
                    selected.len(),
                    bytes(need),
                    bytes(free)
                ))],
            }),
            (Ok(free), Ok(_)) => {
                c.blocking = Some("Not enough disk space".into());
                c.items.push(Check {
                    icon: Icon::Failed,
                    title: "Disk",
                    lines: vec![(
                        format!(
                            "{} frames need about {}; only {} free. Choose fewer frames or \
                             another output folder",
                            selected.len(),
                            bytes(need),
                            bytes(free)
                        ),
                        WARN,
                    )],
                });
            }
            (Err(e), _) | (_, Err(e)) => {
                c.blocking = Some("Disk space unknown".into());
                c.items.push(Check {
                    icon: Icon::Failed,
                    title: "Disk",
                    lines: vec![(e, WARN)],
                });
            }
        }
        let done: Vec<&str> = im
            .interrupted
            .as_ref()
            .map(|r| r.stages.iter().map(|s| s.name.as_str()).collect())
            .unwrap_or_default();
        let weight: f64 = process::plan(survey.paths.len(), selected.len(), false)
            .iter()
            .filter(|(name, _)| *name != "sequence check" && !done.contains(name))
            .map(|(_, w)| w)
            .sum();
        let scale = pixels.as_ref().map_or(1.0, |&p| p as f64 / MEASURED_PIXELS);
        c.estimate_s = weight * scale * MEASURED_THREADS / threads.max(1) as f64;
        if c.blocking.is_none() {
            c.items.push(Check {
                icon: Icon::Done,
                title: "Time",
                lines: vec![(format!("About {}", duration(c.estimate_s)), GOOD)],
            });
        }
        c
    }
}
