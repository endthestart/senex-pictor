//! Opening what the user chose (a RAW folder goes to Import; a result is prepared in the
//! background and opens in the viewer) and advancing what is open: checks, runs, results.

use crate::app::{Pictor, Workspace};
use crate::import::Import;
use crate::job::Job;
use crate::project::{self, Kind, Loaded};
use crate::session::Session;
use crate::shell::Step;
use std::path::PathBuf;

impl Pictor {
    pub(crate) fn load(&mut self, job: Job<Loaded>) {
        self.error = None;
        self.workspace = Workspace::Loading(job);
    }

    /// Opens what `path` holds.
    pub fn open(&mut self, path: PathBuf) {
        self.error = None;
        if let Err(e) = self.settings.opened(&path) {
            eprintln!("pictor: the recent folders were not saved ({e})");
        }
        self.settings.clone().put(&self.ctx);
        match project::classify(&path) {
            Ok(Kind::Raw(raw)) => {
                self.workspace =
                    Workspace::Import(Box::new(Import::raw(raw, self.settings.clone())));
                self.step = Step::Import;
            }
            Ok(Kind::Interrupted(out, report)) => {
                match Import::interrupted(out, *report, self.settings.clone()) {
                    Ok(i) => {
                        self.workspace = Workspace::Import(Box::new(i));
                        self.step = Step::Import;
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            Ok(Kind::Processed(out)) => self.load(Job::start("Opening the result", move |_| {
                project::load_run(&out, None)
            })),
            Ok(Kind::Store(dir)) => self.load(Job::start("Opening the store", move |_| {
                project::load_store(&dir)
            })),
            Ok(Kind::SceneFile(file)) => self
                .load(Job::start("Opening the scene file", move |_| {
                    project::load_scene_file(&file)
                })),
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn opened_result(&mut self, rs: &eframe::egui_wgpu::RenderState, loaded: Loaded) {
        let name = loaded.name.clone();
        match Session::new(rs, self.display.clone(), loaded, self.mp) {
            Ok(s) => {
                eprintln!("pictor: opened {name}");
                self.workspace = Workspace::Open(Box::new(s));
                self.step = self.next;
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    /// Back to the first screen with `message`, also on stderr for terminal launches.
    pub(crate) fn fail(&mut self, message: String) {
        eprintln!("pictor: {message}");
        self.error = Some(message);
        self.workspace = Workspace::Empty;
        self.step = Step::Import;
    }

    /// Advances background work.
    pub(crate) fn poll(&mut self, frame: &eframe::Frame) {
        match &mut self.workspace {
            Workspace::Loading(job) => {
                if let Some(result) = job.poll() {
                    match (result, frame.wgpu_render_state()) {
                        (Ok(l), Some(rs)) => self.opened_result(&rs.clone(), l),
                        (Err(e), _) => self.fail(e.to_string()),
                        (Ok(_), None) => self.fail("the window has no GPU renderer".into()),
                    }
                }
            }
            Workspace::Import(i) => {
                if let Some(out) = i.poll() {
                    self.next = Step::Compare;
                    self.load(Job::start("Opening the result", move |_| {
                        project::load_run(&out, None)
                    }));
                }
            }
            Workspace::Open(s) => {
                if let (Some((step, error)), Some(run)) = (s.reopen.take(), &s.run) {
                    let out = run.out.clone();
                    self.next = step;
                    self.open(out);
                    self.error = error;
                    return;
                }
                if let (Some(result), Some(run)) = (s.scene.open.take(), &s.run) {
                    let out = run.out.clone();
                    self.next = Step::Compare;
                    self.load(Job::start("Opening the result", move |_| {
                        project::load_run(&out, Some(result))
                    }));
                }
            }
            Workspace::Empty => {}
        }
    }
}
