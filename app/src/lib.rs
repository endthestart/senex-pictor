//! Pictor desktop app (ADR 0011): the window, its steps and the GPU pieces they draw
//! with. `main.rs` parses the command line and starts it.

pub mod app;
pub mod bench;
mod commands;
mod compare;
mod display;
pub mod gpu;
mod import;
mod job;
#[cfg(target_os = "macos")]
mod menu;
mod opening;
pub mod project;
mod rows;
pub mod scene;
mod screens;
mod session;
pub mod settings;
pub mod shell;
mod shot;
mod stats;
mod synthetic;
mod theme;
mod thumbs;
pub mod viewer;
mod welcome;
mod widgets;
