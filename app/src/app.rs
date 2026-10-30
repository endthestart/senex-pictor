//! The window: the header, the step list and the status line around the current step's
//! screen. A RAW folder goes through Import (checked, chosen, processed); a processed folder,
//! a store, a scene file or the demo scene is prepared in the background and opens on Compare,
//! the hub every step returns to.

use crate::display::Display;
use crate::import::Import;
use crate::job::Job;
use crate::project::{self, Loaded};
use crate::session::Session;
use crate::settings::Settings;
use crate::shell::{self, Step};
use crate::{rows, welcome};
use std::path::PathBuf;

pub(crate) enum Workspace {
    Empty,
    Import(Box<Import>),
    Loading(Job<Loaded>),
    Open(Box<Session>),
}

pub struct Start {
    /// A folder or file to open.
    pub open: Option<PathBuf>,
    /// The synthetic demo scene at this size.
    pub demo: Option<[u32; 2]>,
    /// The step to show once something is open.
    pub step: Option<Step>,
    /// Render at this many megapixels instead of the scene's own size.
    pub mp: Option<f64>,
    pub gpu_name: String,
    pub device: wgpu::Device,
    pub bench_seconds: Option<f64>,
    pub screenshot: Option<PathBuf>,
}

pub struct Pictor {
    pub(crate) workspace: Workspace,
    pub(crate) step: Step,
    /// The step to show once the next result opens.
    pub(crate) next: Step,
    pub(crate) display: Display,
    /// What the window title names.
    title: String,
    /// The header's chips: the memory budget and the GPU.
    pub(crate) chips: Vec<String>,
    pub(crate) mp: Option<f64>,
    pub(crate) error: Option<String>,
    pub(crate) bench: Option<crate::bench::Window>,
    shot: Option<crate::shot::Shot>,
    pub(crate) settings: Settings,
    settings_window: crate::settings::Window,
}

/// The header's memory chip for `settings`.
fn memory_chip(settings: &Settings) -> String {
    match settings.memory_gb {
        Some(gb) => format!("Memory {gb:.0} GB set"),
        None => match pictor_engine::budget::Budget::from_free() {
            Ok(b) => format!("Memory {} budget", crate::widgets::bytes(b.bytes as f64)),
            Err(_) => "Memory unknown".into(),
        },
    }
}

impl Pictor {
    /// The app; without a wgpu renderer it opens on that error.
    pub fn new(cc: &eframe::CreationContext<'_>, s: Start) -> Pictor {
        crate::theme::apply(&cc.egui_ctx);
        let rs = cc.wgpu_render_state.as_ref();
        let display = match rs {
            Some(rs) => Display::of(rs, &s.gpu_name, rs.device == s.device),
            None => Display {
                scale: 1.0,
                mode: crate::gpu::canvas::MODE_ENCODE,
                headroom: None,
                lines: vec!["the window did not start with wgpu".into()],
            },
        };
        let (settings, settings_error) = match Settings::load() {
            Ok(s) => (s, None),
            Err(e) => (
                Settings::default(),
                Some(format!(
                    "Settings could not be read ({e}); using the defaults"
                )),
            ),
        };
        settings.clone().put(&cc.egui_ctx);
        let memory = memory_chip(&settings);
        let gpu = s
            .gpu_name
            .split(" (")
            .next()
            .unwrap_or(&s.gpu_name)
            .to_string();
        let mut app = Pictor {
            workspace: Workspace::Empty,
            step: Step::Import,
            next: s.step.unwrap_or(Step::Compare),
            chips: vec![memory, gpu],
            title: String::new(),
            display,
            mp: s.mp,
            error: None,
            shot: s.screenshot.map(crate::shot::Shot::new),
            bench: s.bench_seconds.map(crate::bench::Window::new),
            settings,
            settings_window: Default::default(),
        };
        app.error = settings_error;
        if rs.is_none() {
            app.error = Some("the window did not start with wgpu".into());
        } else if let Some(path) = s.open {
            app.open(path);
        } else if let Some(size) = s.demo {
            app.load(Job::start("Making the demo scene", move |_| {
                project::demo(size)
            }));
        }
        if let (Some(step), Workspace::Import(_)) = (s.step, &app.workspace) {
            app.step = step;
        }
        app
    }

    /// Shows `step` (as a click in the step list would).
    pub fn show_step(&mut self, step: Step) {
        self.step = step;
    }

    /// Whether a result is open (on any step).
    pub fn opened(&self) -> bool {
        matches!(self.workspace, Workspace::Open(_))
    }

    /// The open result, if any.
    pub fn session(&self) -> Option<&crate::session::Session> {
        match &self.workspace {
            Workspace::Open(s) => Some(s),
            _ => None,
        }
    }

    /// The step on screen.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Whether the Compare screen is up.
    pub fn comparing(&self) -> bool {
        self.opened() && self.step == Step::Compare
    }

    /// Whether nothing is being checked, processed or prepared.
    pub fn idle(&self) -> bool {
        match &self.workspace {
            Workspace::Import(i) => !i.checking() && !i.processing(),
            Workspace::Loading(_) => false,
            Workspace::Open(s) => !s.busy(),
            Workspace::Empty => true,
        }
    }

    fn screen(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let mut action = None;
        match (&mut self.workspace, self.step) {
            (Workspace::Empty, _) => {
                egui::CentralPanel::default().show(ui, |ui| {
                    action = welcome::show(ui, self.error.as_deref());
                });
            }
            (Workspace::Loading(job), _) => {
                let label = job.label.clone();
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.4);
                        ui.spinner();
                        ui.label(format!("{label}…"));
                    });
                });
            }
            (Workspace::Import(i), _) => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(28, 24)))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| i.show(ui));
                    });
            }
            (Workspace::Open(s), step) => crate::screens::show(ui, &ctx, s, step),
        }
        match action {
            Some(welcome::Action::Open(path)) => self.open(path),
            Some(welcome::Action::Demo) => {
                self.load(Job::start("Making the demo scene", |_| {
                    project::demo([6048, 4024])
                }));
            }
            None => {}
        }
    }
}

impl eframe::App for Pictor {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(frame);
        self.bench_frame(&ctx);
        if self.opened()
            && let Some(shot) = &mut self.shot
            && shot.frame(&ctx)
        {
            self.shot = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let (name, chips) = rows::header(self);
        if name != self.title {
            let title = if name.is_empty() {
                "Pictor".to_string()
            } else {
                format!("{name} — Pictor")
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
            self.title = name.clone();
        }
        let clicked = egui::Panel::top("header")
            .frame(egui::Frame::NONE)
            .show(ui, |ui| shell::header(ui, &name, &chips))
            .inner;
        if clicked || ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.settings_window.open(self.settings.clone());
        }
        if let Some(saved) = self.settings_window.show(&ctx) {
            self.chips[0] = memory_chip(&saved);
            self.settings = saved.clone();
            saved.put(&ctx);
        }
        let (icon, status) = rows::status(self);
        egui::Panel::bottom("status")
            .frame(egui::Frame::NONE)
            .show(ui, |ui| shell::footer(ui, icon, &status));
        let steps = rows::steps(self);
        let pressed = ctx.input(|i| {
            let keys = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
            ];
            keys.iter()
                .position(|k| i.modifiers.command && i.key_pressed(*k))
        });
        if let Some(row) = pressed.and_then(|k| steps.get(k))
            && row.state != shell::State::Unavailable
        {
            self.step = row.step;
        }
        egui::Panel::left("steps")
            .exact_size(240.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(crate::theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(12, 12)),
            )
            .show(ui, |ui| {
                if let Some(step) = shell::rail(ui, &steps, self.step) {
                    self.step = step;
                }
                let busy = !self.idle() || matches!(self.workspace, Workspace::Empty);
                if !busy {
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        if ui.link("Open another folder…").clicked()
                            && let Some(dir) = rfd::FileDialog::new().pick_folder()
                        {
                            self.open(dir);
                        }
                    });
                }
            });
        self.screen(ui);
        let busy = !self.idle() || self.bench.is_some();
        if busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.bench.is_some() {
            ctx.request_repaint();
        }
    }
}
