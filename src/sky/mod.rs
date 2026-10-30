//! Sky registration (ADR 0008, 0010): camera, sky model, robust fits and quality reports.

pub mod camera;
pub mod chroma;
pub mod drift;
pub mod fit;
pub mod joint;
pub mod model;
pub mod report;
pub mod residuals;
pub mod solve;

pub use camera::Camera;
pub use model::SkyModel;
