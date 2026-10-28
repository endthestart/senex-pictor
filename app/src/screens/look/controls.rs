//! The guided controls on the look's fields.

use star_photos::render::Look;

/// A guided control on one field of the look; `relevel` when moving it changes the levels,
/// which are measured once the slider is let go.
pub struct Control {
    pub label: &'static str,
    pub range: std::ops::RangeInclusive<f32>,
    pub get: fn(&Look) -> f32,
    pub set: fn(&mut Look, f32),
    pub relevel: bool,
}

pub const BLACK: Control = Control {
    label: "Black point (percentile of the clear sky)",
    range: 0.0..=95.0,
    get: |l| l.black_percentile as f32,
    set: |l, v| l.black_percentile = f64::from(v),
    relevel: true,
};
pub const STRETCH: Control = Control {
    label: "Stretch",
    range: 1.0..=30.0,
    get: |l| l.stretch,
    set: |l, v| l.stretch = v,
    relevel: false,
};
pub const SATURATION: Control = Control {
    label: "Saturation",
    range: 0.0..=2.0,
    get: |l| l.saturation,
    set: |l, v| l.saturation = v,
    relevel: false,
};
pub const SHADOWS: Control = Control {
    label: "Shadow curve",
    range: 0.5..=3.0,
    get: |l| l.shadows,
    set: |l, v| l.shadows = v,
    relevel: false,
};
pub const WHITE: Control = Control {
    label: "White point (percentile)",
    range: 99.0..=99.99,
    get: |l| l.white_percentile as f32,
    set: |l, v| l.white_percentile = f64::from(v),
    relevel: true,
};
