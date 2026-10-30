//! Pictor desktop app (ADR 0011): the window, its steps and the GPU pieces they draw
//! with. `main.rs` parses the command line and starts it.

pub mod app;
pub mod bench;
mod compare;
mod display;
pub mod gpu;
mod import;
mod job;
mod opening;
pub mod project;
mod rows;
pub mod scene;
mod screens;
mod session;
pub mod shell;
mod shot;
mod stats;
mod synthetic;
mod theme;
mod thumbs;
mod viewer;
mod welcome;
mod widgets;
