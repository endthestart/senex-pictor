//! An opened result: the viewer with it on the GPU, the run it came from (when there is one)
//! with its frames' thumbnails, and each step's screen state.

use crate::compare::Compare;
use crate::display::Display;
use crate::project::{Loaded, Run};
use crate::screens::frames::Frames;
use crate::screens::look::LookStep;
use crate::screens::scene::SceneStep;
use crate::screens::sky::Sky;
use crate::thumbs::Thumbs;
use crate::viewer::Viewer;
use pictor_engine::Result;

pub struct Session {
    pub name: String,
    pub run: Option<Run>,
    pub result: Option<String>,
    pub thumbs: Option<Thumbs>,
    pub viewer: Viewer,
    pub compare: Compare,
    pub frames: Frames,
    pub sky: Sky,
    pub scene: SceneStep,
    pub look: LookStep,
}

impl Session {
    pub fn new(
        rs: &eframe::egui_wgpu::RenderState,
        display: Display,
        loaded: Loaded,
        mp: Option<f64>,
    ) -> Result<Session> {
        let viewer = Viewer::new(rs, display, loaded.scene, mp)?;
        let dir = loaded.run.as_ref().map(|r| r.out.clone());
        let compare = Compare::new(&viewer, dir)?;
        let thumbs = loaded.run.as_ref().map(|r| {
            let frames = &r.store.manifest.frames;
            let middle = frames.get(frames.len() / 2).map(|f| f.name.as_str());
            let typical = r
                .survey
                .captures
                .iter()
                .position(|c| Some(c.name.as_str()) == middle);
            Thumbs::new(
                r.survey.thumbs.clone(),
                typical.unwrap_or(r.survey.captures.len() / 2),
            )
        });
        Ok(Session {
            name: loaded.name,
            run: loaded.run,
            result: loaded.result,
            thumbs,
            viewer,
            compare,
            frames: Frames::default(),
            sky: Sky::default(),
            scene: SceneStep::default(),
            look: LookStep::default(),
        })
    }

    /// Advances background work on this result.
    pub fn poll(&mut self, ctx: &egui::Context) {
        self.frames.poll(ctx);
        self.scene.poll_preview(ctx);
        if let Some(run) = &self.run
            && let Some(made) = self.scene.poll(&run.out)
        {
            self.scene.open = Some(made);
        }
        if self.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
    }

    /// Whether something is being made or read in the background.
    pub fn busy(&self) -> bool {
        self.scene.making.is_some()
            || self.scene.loading()
            || self.look.exporting()
            || self.frames.loading()
    }

    pub fn compare_detail(&self) -> String {
        match self.compare.judged() {
            0 => format!("{} vs {}", self.viewer.names[0], self.viewer.names[1]),
            n => format!("{n} regions judged"),
        }
    }
}
