//! What rendering B shows: the session's result in its own look, or another result of the
//! same run (the long exposure against a hero frame or a moment), read in the background.

use crate::job::Job;
use crate::project::Run;
use crate::scene::{self, Scene};
use crate::theme::MUTED;
use crate::viewer::Viewer;
use crate::widgets::strong;
use egui::{ComboBox, RichText, Ui};
use std::path::PathBuf;

pub struct Source {
    store: Option<PathBuf>,
    /// The run's other results.
    others: Vec<String>,
    loading: Option<(String, Job<Scene>)>,
}

/// What B can show, for the chooser.
enum Pick {
    Own,
    Other(String),
}

impl Source {
    pub fn new(run: Option<&Run>, own: Option<&str>) -> Source {
        let others = run
            .and_then(|r| r.results().ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|r| Some(r.as_str()) != own)
            .collect();
        Source {
            store: run.map(|r| r.store.dir.clone()),
            others,
            loading: None,
        }
    }

    pub fn loading(&self) -> bool {
        self.loading.is_some()
    }

    /// The "B shows" row; a result to show that finished reading, or `Some(None)` to show the
    /// session's own result again.
    pub fn row(&mut self, ui: &mut Ui, viewer: &Viewer) -> Option<Option<(String, Scene)>> {
        if self.others.is_empty() {
            return None;
        }
        let shown = viewer.other.as_ref().map(|(name, _)| name.clone());
        let mut pick = None;
        ui.horizontal(|ui| {
            let label = ui.label(strong("B shows"));
            ComboBox::from_id_salt("b-source")
                .selected_text(shown.as_deref().unwrap_or("This result"))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(shown.is_none(), "This result")
                        .clicked()
                    {
                        pick = Some(Pick::Own);
                    }
                    for name in &self.others {
                        let on = shown.as_deref() == Some(name.as_str());
                        if ui.selectable_label(on, name).clicked() {
                            pick = Some(Pick::Other(name.clone()));
                        }
                    }
                })
                .response
                .labelled_by(label.id);
            if let Some((name, _)) = &self.loading {
                ui.spinner();
                ui.label(RichText::new(format!("Reading {name}…")).color(MUTED));
            }
        });
        match (pick, &self.store) {
            (Some(Pick::Own), _) if shown.is_some() => return Some(None),
            (Some(Pick::Other(name)), Some(store)) if shown.as_ref() != Some(&name) => {
                let (dir, planes) = (store.clone(), name.clone());
                self.loading = Some((
                    name,
                    Job::start("Reading the result", move |_| scene::load(&dir, &planes)),
                ));
            }
            _ => {}
        }
        let done = self.loading.as_ref().and_then(|(_, job)| job.poll());
        match done {
            Some(Ok(scene)) => self.loading.take().map(|(name, _)| Some((name, scene))),
            Some(Err(e)) => {
                self.loading = None;
                ui.colored_label(crate::theme::WARN, e.to_string());
                None
            }
            None => None,
        }
    }
}
