//! What the owner asks for beyond the current screen, from the keyboard or the menu bar:
//! Settings (⌘,), Open Folder (⌘O), a step (⌘1–⌘6) and, from the menu, Third-Party Notices and
//! the zoom (its keys, F, 1 and 2, reach the viewer directly); the dialogs they open and the
//! window's title.

use crate::app::{Pictor, Workspace};
use crate::shell::{State, Step, StepRow};
use crate::viewer::Zoom;
use egui::{Key, Modifiers};

/// A command the keyboard or the menu bar gave.
pub enum Command {
    Open,
    Settings,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Notices,
    Step(Step),
    /// From the menu bar, which only macOS has yet.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Fit,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Zoom(f32),
}

fn keyboard(ctx: &egui::Context) -> Vec<Command> {
    let digits = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
    ];
    ctx.input_mut(|i| {
        let mut out = Vec::new();
        if i.consume_key(Modifiers::COMMAND, Key::Comma) {
            out.push(Command::Settings);
        }
        if i.consume_key(Modifiers::COMMAND, Key::O) {
            out.push(Command::Open);
        }
        for (key, step) in digits.into_iter().zip(Step::ALL) {
            if i.consume_key(Modifiers::COMMAND, key) {
                out.push(Command::Step(step));
            }
        }
        out
    })
}

impl Pictor {
    /// Carries out the keyboard's and the menu bar's commands; `steps` are the step list's rows.
    pub(crate) fn commands(&mut self, ctx: &egui::Context, steps: &[StepRow]) {
        #[allow(unused_mut)]
        let mut chosen = keyboard(ctx);
        #[cfg(target_os = "macos")]
        if let Some(menu) = &self.menu {
            chosen.extend(menu.commands());
        }
        for c in chosen {
            match c {
                Command::Settings => self.settings_window.open(self.settings.clone()),
                Command::Notices => self.notices.open(),
                Command::Open => {
                    if self.idle()
                        && let Some(dir) = rfd::FileDialog::new().pick_folder()
                    {
                        self.open(dir);
                    }
                }
                Command::Step(step) => {
                    if steps
                        .iter()
                        .any(|r| r.step == step && r.state != State::Unavailable)
                    {
                        self.step = step;
                    }
                }
                Command::Fit | Command::Zoom(_) => {
                    if let Workspace::Open(s) = &mut self.workspace {
                        s.viewer.zoom = match c {
                            Command::Zoom(z) => Zoom::Factor(z),
                            _ => Zoom::Fit,
                        };
                    }
                }
            }
        }
    }

    /// Settings and Third-Party Notices, while open.
    pub(crate) fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(saved) = self.settings_window.show(ctx) {
            self.chips[0] = crate::app::memory_chip(&saved);
            self.settings = saved.clone();
            saved.put(ctx);
        }
        if self.settings_window.notices_wanted() {
            self.notices.open();
        }
        self.notices.show(ctx);
    }

    /// "<what is open> — Pictor", or "Pictor" alone.
    pub(crate) fn retitle(&mut self, ctx: &egui::Context, name: &str) {
        if name != self.title {
            let title = if name.is_empty() {
                "Pictor".to_string()
            } else {
                format!("{name} — Pictor")
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
            self.title = name.to_string();
        }
    }
}
