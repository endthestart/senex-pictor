//! Stars: detection in each frame and tracking from frame to frame (ADR 0008).

pub mod background;
pub mod detect;
pub mod io;
pub mod similarity;
pub mod track;
pub mod triangles;

pub use detect::{Star, detect};
pub use track::{Tracks, track};
