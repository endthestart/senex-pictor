//! The macOS menu bar (muda over AppKit): Pictor (About, Settings…, Hide, Quit), File (Open
//! Folder…, Close Window), View (Fit, 100%, 200%, Full Screen), Go (the six steps) and Window.
//! AppKit gives the menu bar only keys with ⌘ (or ⌃): those items take the key and the app sees
//! the command instead; the plain keys shown (F, 1, 2) still go to the window, where egui runs
//! them, so nothing happens twice. There is no Edit menu, so ⌘Z stays the brush's undo.

use crate::commands::Command;
use crate::shell::Step;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use pictor_engine::{Result, error};
use std::sync::mpsc::{Receiver, channel};

pub struct NativeMenu {
    _menu: Menu,
    chosen: Receiver<String>,
}

fn item(id: &str, text: &str, modifiers: Modifiers, key: Code) -> MenuItem {
    MenuItem::with_id(id, text, true, Some(Accelerator::new(modifiers, key)))
}

fn command(id: &str) -> Option<Command> {
    match id {
        "open" => Some(Command::Open),
        "settings" => Some(Command::Settings),
        "fit" => Some(Command::Fit),
        "zoom.1" => Some(Command::Zoom(1.0)),
        "zoom.2" => Some(Command::Zoom(2.0)),
        _ => id
            .strip_prefix("step.")
            .and_then(|k| k.parse::<usize>().ok())
            .and_then(|k| Step::ALL.get(k).copied())
            .map(Command::Step),
    }
}

impl NativeMenu {
    /// Installs the menu bar; menu choices wake `ctx`.
    pub fn install(ctx: &egui::Context) -> Result<NativeMenu> {
        let cmd = Modifiers::META;
        let about = AboutMetadata {
            name: Some("Pictor".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            copyright: Some("SENEX Pictor · MIT OR Apache-2.0".into()),
            ..Default::default()
        };
        let app = Submenu::with_items(
            "Pictor",
            true,
            &[
                &PredefinedMenuItem::about(Some("About Pictor"), Some(about)),
                &PredefinedMenuItem::separator(),
                &item("settings", "Settings…", cmd, Code::Comma),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(Some("Hide Pictor")),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(Some("Quit Pictor")),
            ],
        )
        .map_err(|e| error(e.to_string()))?;
        let file = Submenu::with_items(
            "File",
            true,
            &[
                &item("open", "Open Folder…", cmd, Code::KeyO),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::close_window(None),
            ],
        )
        .map_err(|e| error(e.to_string()))?;
        let view = Submenu::with_items(
            "View",
            true,
            &[
                &item("fit", "Fit", Modifiers::empty(), Code::KeyF),
                &item("zoom.1", "100%", Modifiers::empty(), Code::Digit1),
                &item("zoom.2", "200%", Modifiers::empty(), Code::Digit2),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::fullscreen(None),
            ],
        )
        .map_err(|e| error(e.to_string()))?;
        let digits = [
            Code::Digit1,
            Code::Digit2,
            Code::Digit3,
            Code::Digit4,
            Code::Digit5,
            Code::Digit6,
        ];
        let steps: Vec<MenuItem> = Step::ALL
            .iter()
            .zip(digits)
            .enumerate()
            .map(|(k, (step, key))| item(&format!("step.{k}"), step.title(), cmd, key))
            .collect();
        let refs: Vec<&dyn muda::IsMenuItem> =
            steps.iter().map(|i| i as &dyn muda::IsMenuItem).collect();
        let go = Submenu::with_items("Go", true, &refs).map_err(|e| error(e.to_string()))?;
        let window = Submenu::with_items(
            "Window",
            true,
            &[
                &PredefinedMenuItem::minimize(None),
                &PredefinedMenuItem::bring_all_to_front(None),
            ],
        )
        .map_err(|e| error(e.to_string()))?;
        let menu = Menu::with_items(&[&app, &file, &view, &go, &window])
            .map_err(|e| error(e.to_string()))?;
        menu.init_for_nsapp();
        window.set_as_windows_menu_for_nsapp();
        let (tx, chosen) = channel();
        let wake = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if tx.send(event.id.0).is_ok() {
                wake.request_repaint();
            }
        }));
        Ok(NativeMenu {
            _menu: menu,
            chosen,
        })
    }

    /// The commands chosen since the last call.
    pub fn commands(&self) -> Vec<Command> {
        self.chosen
            .try_iter()
            .filter_map(|id| command(&id))
            .collect()
    }
}
