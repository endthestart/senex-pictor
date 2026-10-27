//! Optional sensor-stage calibration on the CFA mosaic (ADR 0006, proposed).

mod apply;
mod combine;
mod flat;
mod master;
mod matching;
mod mosaic;

pub use apply::{Calibrated, FlatReport, MasterSummary, Report, calibrate};
pub use combine::Estimator;
pub use flat::{FlatField, FlatSummary, build_flat};
pub use master::{Master, Role, build_master};
pub use matching::{Capture, Warning};
pub use mosaic::Mosaic;
