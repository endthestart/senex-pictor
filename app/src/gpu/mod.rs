//! GPU pieces of the app, all on the device shared with the window: looks computed from the
//! scene (`look`), their mip levels (`mips`) and the canvas that draws them (`canvas`).

pub mod canvas;
pub mod look;
pub mod mips;
