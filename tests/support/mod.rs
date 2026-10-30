//! Synthetic sensor with known truth for calibration tests.
#![allow(dead_code)]

use lcc_raw::{
    BlackLevel, Cfa, ColorData, Metadata, OpcodeLists, Orientation, RawData, RawFormat, RawImage,
    Rect,
};
use pictor_engine::Result;
use pictor_engine::calibration::{Estimator, Mosaic};

pub const W: usize = 64;
pub const H: usize = 48;
pub const BLACK: f32 = 1008.0;
pub const WHITE: f32 = 16383.0;
pub const READ_NOISE_ADU: f64 = 1.5;
/// Digital gain per color (R, G, B), ADU per electron.
pub const GAIN: [f64; 3] = [1.2, 1.0, 1.2];
pub const HOT: [(usize, usize); 3] = [(10, 7), (33, 20), (50, 41)];

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn uniform(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    pub fn normal(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (std::f64::consts::TAU * self.uniform()).cos()
    }
    pub fn poisson(&mut self, mean: f64) -> f64 {
        if mean < 30.0 {
            let (limit, mut product, mut count) = ((-mean).exp(), self.uniform(), 0.0);
            while product > limit {
                product *= self.uniform();
                count += 1.0;
            }
            count
        } else {
            (mean + mean.sqrt() * self.normal()).round().max(0.0)
        }
    }
}

pub fn cfa() -> Cfa {
    Cfa {
        width: 2,
        height: 2,
        pattern: vec![0, 1, 1, 2],
    }
}

pub fn color(x: usize, y: usize) -> usize {
    usize::from(cfa().color_at(x, y))
}

/// Fixed properties of the simulated sensor and lens.
pub struct Sensor {
    /// Per-site offset around the pedestal (DSNU), ADU.
    pub offset: Vec<f64>,
    /// Dark signal rate, electrons per second.
    pub dark_rate: Vec<f64>,
    /// Relative response: vignetting per color times PRNU.
    pub response: Vec<f64>,
}

impl Sensor {
    pub fn new() -> Sensor {
        let mut rng = Rng::new(7);
        let falloff = [0.40, 0.35, 0.45];
        let mut sensor = Sensor {
            offset: vec![],
            dark_rate: vec![],
            response: vec![],
        };
        for y in 0..H {
            for x in 0..W {
                let dx = (x as f64 - (W as f64 - 1.0) / 2.0) / (W as f64 / 2.0);
                let dy = (y as f64 - (H as f64 - 1.0) / 2.0) / (H as f64 / 2.0);
                let rho2 = (dx * dx + dy * dy) / 2.0;
                sensor.offset.push(3.0 * rng.normal());
                sensor
                    .dark_rate
                    .push(if HOT.contains(&(x, y)) { 40.0 } else { 0.2 });
                sensor
                    .response
                    .push((1.0 - falloff[color(x, y)] * rho2) * (1.0 + 0.01 * rng.normal()));
            }
        }
        sensor
    }

    /// Median response of each color: the flat normalization the truth is expressed in.
    pub fn response_median(&self) -> [f64; 3] {
        let mut by_color: [Vec<f64>; 3] = Default::default();
        for (site, r) in self.response.iter().enumerate() {
            by_color[color(site % W, site / W)].push(*r);
        }
        by_color.map(|mut v| {
            v.sort_by(f64::total_cmp);
            let mid = v.len() / 2;
            if v.len() % 2 == 1 {
                v[mid]
            } else {
                (v[mid - 1] + v[mid]) / 2.0
            }
        })
    }
}

/// Illumination of one exposure, electrons per second at unit response.
pub struct Scene {
    pub exposure: f64,
    pub iso: u32,
    /// Per site; zero for darks and bias frames.
    pub light: Vec<f64>,
}

/// Uniform per-color background plus Gaussian stars `(x, y, electrons per second)`.
pub fn sky(background: [f64; 3], stars: &[(f64, f64, f64)]) -> Vec<f64> {
    let sigma = 1.2f64;
    (0..W * H)
        .map(|site| {
            let (x, y) = ((site % W) as f64, (site / W) as f64);
            let stellar: f64 = stars
                .iter()
                .map(|(sx, sy, flux)| {
                    let r2 = (x - sx).powi(2) + (y - sy).powi(2);
                    flux * (-r2 / (2.0 * sigma * sigma)).exp()
                        / (std::f64::consts::TAU * sigma * sigma)
                })
                .sum();
            background[color(site % W, site / W)] + stellar
        })
        .collect()
}

pub fn dark_scene(exposure: f64) -> Scene {
    Scene {
        exposure,
        iso: 1600,
        light: vec![0.0; W * H],
    }
}

/// Stored ADU of one exposure: pedestal, offset, gained Poisson electrons, read noise.
pub fn expose(sensor: &Sensor, scene: &Scene, rng: &mut Rng) -> Vec<u16> {
    (0..W * H)
        .map(|site| {
            let electrons = scene.exposure
                * (sensor.response[site] * scene.light[site] + sensor.dark_rate[site]);
            let adu = f64::from(BLACK)
                + sensor.offset[site]
                + GAIN[color(site % W, site / W)] * rng.poisson(electrons)
                + READ_NOISE_ADU * rng.normal();
            adu.round().clamp(0.0, f64::from(WHITE)) as u16
        })
        .collect()
}

pub fn raw_image(samples: Vec<u16>, scene: &Scene) -> RawImage {
    RawImage {
        format: RawFormat::Nef,
        width: W,
        height: H,
        cpp: 1,
        data: RawData::U16(samples),
        cfa: Some(cfa()),
        bits: 14,
        black: BlackLevel::uniform(BLACK),
        white: vec![WHITE],
        active_area: Rect::new(0, 0, W, H),
        crop: Rect::new(0, 0, W, H),
        orientation: Orientation::Normal,
        color: ColorData::default(),
        wb_multipliers: None,
        linearized: false,
        pixel_aspect: 1.0,
        opcodes: OpcodeLists::default(),
        metadata: Metadata {
            make: Some("Synthetic".into()),
            model: Some("Sensor".into()),
            iso: Some(scene.iso),
            exposure_time: Some(scene.exposure),
            lens_model: Some("Synthetic 24mm".into()),
            focal_length: Some(24.0),
            f_number: Some(4.0),
            ..Default::default()
        },
        lens_metadata: Default::default(),
    }
}

/// A decoded mosaic with the vendor fields the decoder does not expose.
pub fn mosaic(samples: Vec<u16>, scene: &Scene) -> Result<Mosaic> {
    let mut mosaic = Mosaic::from_raw(&raw_image(samples, scene))?;
    mosaic.capture.shutter_mode = Some("Electronic".into());
    mosaic.capture.compression = Some("Lossless".into());
    mosaic.capture.in_camera_nr = Some("Off".into());
    Ok(mosaic)
}

pub fn frames(sensor: &Sensor, scene: &Scene, count: usize, seed: u64) -> Result<Vec<Mosaic>> {
    let mut rng = Rng::new(seed);
    (0..count)
        .map(|_| mosaic(expose(sensor, scene, &mut rng), scene))
        .collect()
}

pub fn site(x: usize, y: usize) -> usize {
    y * W + x
}

pub const CLIPPED: Estimator = Estimator::WinsorizedSigmaClip {
    k: 3.0,
    iterations: 5,
};
pub const BACKGROUND: [f64; 3] = [30.0, 50.0, 25.0];
pub const STARS: [(f64, f64, f64); 3] = [
    (20.3, 15.6, 4500.0),
    (44.8, 30.2, 3000.0),
    (31.5, 38.4, 2000.0),
];

pub fn flat_scene(tint: [f64; 3]) -> Scene {
    let light = (0..W * H)
        .map(|s| 6000.0 * tint[color(s % W, s / W)])
        .collect();
    Scene {
        exposure: 0.5,
        iso: 1600,
        light,
    }
}

pub fn light_scene(background: [f64; 3]) -> Scene {
    Scene {
        exposure: 20.0,
        iso: 1600,
        light: sky(background, &STARS),
    }
}

/// Shifts a frame's pedestal and its recorded black alike, as masked-pixel black tracks drift.
pub fn shift_pedestal(mosaic: &mut Mosaic, drift: f32) {
    for v in &mut mosaic.data {
        *v += drift;
    }
    for b in mosaic
        .black
        .values
        .iter_mut()
        .chain(mosaic.capture.black.iter_mut())
    {
        *b += drift;
    }
}
