//! Star Photos desktop app (ADR 0011): the window, its screens and the GPU pieces they draw
//! with. `main.rs` parses the command line and starts it.

pub mod app;
pub mod bench;
mod compare;
mod display;
pub mod gpu;
pub mod project;
pub mod scene;
mod shot;
mod stats;
mod synthetic;
mod theme;
