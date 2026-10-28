//! The window: open a processed folder (button, drag and drop, or the command line) or the demo
//! scene, wait while it is prepared in the background, then compare. The step list shows only
//! the steps that exist so far.

use crate::compare::Compare;
use crate::display::Display;
use crate::project::{self, Project};
use crate::stats;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;

enum Screen {
    Open,
    Loading(String, Receiver<star_photos::Result<Project>>),
    Compare(Box<Compare>),
}

pub struct Start {
    pub folder: Option<PathBuf>,
    pub project: Option<Project>,
    /// The synthetic demo scene at this size.
    pub demo: Option<[u32; 2]>,
    /// Render at this many megapixels instead of the scene's own size.
    pub mp: Option<f64>,
    pub gpu_name: String,
    pub device: wgpu::Device,
    pub bench_seconds: Option<f64>,
    pub screenshot: Option<PathBuf>,
}

pub struct StarPhotos {
    screen: Screen,
    display: Display,
    mp: Option<f64>,
    error: Option<String>,
    bench: Option<Bench>,
    shot: Option<crate::shot::Shot>,
}

/// A scripted run: its length, when it started, frame intervals per phase, the last frame.
struct Bench {
    seconds: f64,
    start: Instant,
    samples: Vec<(&'static str, f64)>,
    last: Option<Instant>,
}

impl StarPhotos {
    /// The app; without a wgpu renderer it opens on that error.
    pub fn new(cc: &eframe::CreationContext<'_>, s: Start) -> StarPhotos {
        crate::theme::apply(&cc.egui_ctx);
        let rs = cc.wgpu_render_state.as_ref();
        let display = match rs {
            Some(rs) => Display::of(rs, &s.gpu_name, rs.device == s.device),
            None => Display {
                scale: 1.0,
                mode: crate::gpu::canvas::MODE_ENCODE,
                lines: vec!["the window did not start with wgpu".into()],
            },
        };
        let mut app = StarPhotos {
            screen: Screen::Open,
            display,
            mp: s.mp,
            error: None,
            shot: s.screenshot.map(crate::shot::Shot::new),
            bench: s.bench_seconds.map(|seconds| Bench {
                seconds,
                start: Instant::now(),
                samples: Vec::new(),
                last: None,
            }),
        };
        match (rs, s.project, s.folder, s.demo) {
            (None, ..) => app.fail("the window did not start with wgpu".into()),
            (Some(rs), Some(p), ..) => app.show(rs, p),
            (Some(_), None, Some(folder), _) => {
                app.screen = Screen::Loading(folder.display().to_string(), project::open(folder));
            }
            (Some(_), None, None, Some(size)) => {
                app.screen = Screen::Loading("the demo scene".into(), project::demo(size));
            }
            (Some(_), None, None, None) => {}
        }
        app
    }

    /// Whether the Compare screen is up.
    pub fn comparing(&self) -> bool {
        matches!(self.screen, Screen::Compare(_))
    }

    fn show(&mut self, rs: &eframe::egui_wgpu::RenderState, p: Project) {
        let name = p.name.clone();
        match Compare::new(rs, self.display.clone(), p, self.mp) {
            Ok(c) => {
                eprintln!("star-photos-app: opened {name}");
                self.screen = Screen::Compare(Box::new(c));
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    /// Back to the open screen with `message`, also on stderr for terminal launches.
    fn fail(&mut self, message: String) {
        eprintln!("star-photos-app: {message}");
        self.error = Some(message);
        self.screen = Screen::Open;
    }

    fn open_screen(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            ui.heading("Star Photos");
            ui.label(
                "Open a folder made by star-photos process (or a store) to compare its looks.",
            );
            ui.add_space(12.0);
            if ui.button("Open a processed folder…").clicked()
                && let Some(folder) = rfd::FileDialog::new().pick_folder()
            {
                self.screen = Screen::Loading(folder.display().to_string(), project::open(folder));
            }
            if ui.button("Open a scene file…").clicked()
                && let Some(file) = rfd::FileDialog::new()
                    .add_filter("scene", &["scene"])
                    .pick_file()
            {
                self.screen = Screen::Loading(file.display().to_string(), project::open(file));
            }
            if ui.button("Show the demo scene").clicked() {
                self.screen = Screen::Loading("the demo scene".into(), project::demo([6048, 4024]));
            }
            ui.weak("You can also drop a folder or a .scene file on the window.");
            if let Some(e) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, e);
            }
        });
        let dropped = ui
            .ctx()
            .input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()));
        if let Some(folder) = dropped {
            self.screen = Screen::Loading(folder.display().to_string(), project::open(folder));
        }
    }

    /// Drives a scripted run once the comparison is up: records frame intervals per phase and
    /// ends it, printing the report.
    fn bench_frame(&mut self, ctx: &egui::Context) {
        let Some(Bench {
            seconds,
            start,
            samples,
            last,
        }) = &mut self.bench
        else {
            return;
        };
        let Screen::Compare(c) = &mut self.screen else {
            *start = Instant::now();
            return;
        };
        let now = Instant::now();
        let t = now.duration_since(*start).as_secs_f64() / *seconds;
        let phase = c.script(t, samples.len());
        if let Some(prev) = last.replace(now) {
            samples.push((phase, now.duration_since(prev).as_secs_f64()));
        }
        if t > 1.0 {
            println!(
                "{}",
                serde_json::json!({"window": stats::by_phase(samples), "display": self.display.lines})
            );
            self.bench = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for StarPhotos {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Screen::Loading(_, rx) = &self.screen
            && let Ok(result) = rx.try_recv()
        {
            match (result, frame.wgpu_render_state()) {
                (Ok(p), Some(rs)) => self.show(&rs.clone(), p),
                (Err(e), _) => self.fail(e.to_string()),
                (Ok(_), None) => self.fail("the window has no GPU renderer".into()),
            }
        }
        self.bench_frame(&ctx);
        let mut back = false;
        egui::Panel::left("steps").min_size(280.0).show(ui, |ui| {
            ui.heading("Star Photos");
            ui.separator();
            let open = matches!(self.screen, Screen::Open | Screen::Loading(..));
            if ui.selectable_label(open, "1  Open").clicked() {
                back = true;
            }
            let _ = ui.selectable_label(!open, "2  Compare");
            ui.separator();
            if let Screen::Compare(c) = &mut self.screen {
                ui.strong(&c.name);
                for f in &c.facts {
                    ui.weak(f);
                }
                ui.separator();
                c.looks_panel(ui);
                if let Some(e) = &c.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, e);
                }
            } else {
                for line in &self.display.lines {
                    ui.weak(line);
                }
            }
        });
        if back {
            self.screen = Screen::Open;
        }
        match &mut self.screen {
            Screen::Open => {
                egui::CentralPanel::default().show(ui, |ui| self.open_screen(ui));
            }
            Screen::Loading(what, _) => {
                let what = what.clone();
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.4);
                        ui.spinner();
                        ui.label(format!("Preparing {what}…"));
                    });
                });
            }
            Screen::Compare(c) => {
                if let Some(shot) = &mut self.shot
                    && shot.frame(&ctx)
                {
                    self.shot = None;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                c.keys(&ctx);
                egui::Panel::right("regions")
                    .min_size(340.0)
                    .show(ui, |ui| c.regions_panel(ui));
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| c.canvas(ui));
            }
        }
        if matches!(self.screen, Screen::Loading(..)) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.bench.is_some() {
            ctx.request_repaint();
        }
    }
}
