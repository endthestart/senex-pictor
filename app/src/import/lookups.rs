//! Lookups behind the checks that do not change while frames are chosen.

use star_photos::budget::Budget;
use star_photos::{input, lens};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A lens name at a focal length and aperture (as bits, to be a key).
type LensAt = (String, u64, u64);
/// The profile found: its focal length and aperture.
type Profile = (f64, f64);

/// Answers that do not change while frames are chosen (lensfun lookups, the sensor size, the
/// memory budget), kept between checks so dragging the frame handles stays smooth.
#[derive(Default)]
pub struct Lookups {
    candidates: HashMap<u64, Result<Vec<String>, String>>,
    profiles: HashMap<LensAt, Result<Profile, String>>,
    pixels: HashMap<PathBuf, Result<usize, String>>,
    budget: Option<Result<(u64, usize), String>>,
}

impl Lookups {
    pub fn candidates(&mut self, focal: f64) -> Result<Vec<String>, String> {
        self.candidates
            .entry(focal.to_bits())
            .or_insert_with(|| lens::candidates(focal).map_err(|e| e.to_string()))
            .clone()
    }

    pub fn profile(
        &mut self,
        make: Option<&str>,
        name: &str,
        focal: f64,
        aperture: f64,
    ) -> Result<Profile, String> {
        self.profiles
            .entry((name.to_string(), focal.to_bits(), aperture.to_bits()))
            .or_insert_with(|| {
                lens::for_camera(make, name, focal, aperture)
                    .map(|v| (v.focal_mm, v.aperture))
                    .map_err(|e| e.to_string())
            })
            .clone()
    }

    pub fn pixels(&mut self, raw: &Path) -> Result<usize, String> {
        self.pixels
            .entry(raw.to_path_buf())
            .or_insert_with(|| {
                input::inspect(raw)
                    .map(|i| i.sensor[0] * i.sensor[1])
                    .map_err(|e| e.to_string())
            })
            .clone()
    }

    pub fn budget(&mut self) -> Result<(u64, usize), String> {
        self.budget
            .get_or_insert_with(|| {
                Budget::from_free()
                    .map(|b| (b.bytes, b.threads))
                    .map_err(|e| e.to_string())
            })
            .clone()
    }
}
