//! Integration of a separated store (ADR 0009): the CFA sky stack, the long-exposure ground
//! stack and their composite.

pub mod cfa;
pub mod combine;
pub mod composite;
pub mod foreground;
pub mod ground;
pub mod hero;
pub mod margins;
pub mod moment;
pub mod transient;

use crate::budget::Budget;
use crate::separate::{self, glow::CLEAR};
use crate::store::{Store, planes};
use crate::{Result, error, lens};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Report {
    pub kernel: cfa::Kernel,
    pub samples_per_colour_median: [f64; 3],
    pub samples_per_colour_p1: [f64; 3],
    pub saturated_filled_pixels: usize,
    pub sky_missing_pixels: usize,
    pub ground: foreground::Report,
    /// Moving-foreground pixels taken from the reference minutes (none unless asked for).
    pub sharp_moving_pixels: usize,
    /// Median single-frame temporal noise and the stack's noise (half the even/odd
    /// difference, robust σ of luminance) over clear sky, in ADU.
    pub single_noise_adu: f64,
    pub stack_noise_adu: f64,
    /// The device that stacked the sky (CPU unless a GPU was asked for).
    pub device: String,
    pub seconds: [f64; 3],
}

/// The flat field recorded by separation.
pub fn recorded_flat(store: &Store) -> Result<Vec<f32>> {
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(store.dir.join("separate.json"))?)?;
    let k: [f64; 3] = serde_json::from_value(report["vignetting"]["k"].clone())
        .map_err(|e| error(format!("separate.json has no vignetting profile: {e}")))?;
    Ok(lens::field(store.manifest.width, store.manifest.height, k))
}

/// Sets every colour of the stack's clipped cores to the sensor white (the lowest white above
/// black of any frame and site, flat-fielded like the samples), so they render white whatever
/// the colour of the samples that escaped saturation; returns how many.
pub(crate) fn saturate(sky: &mut cfa::SkyStack, store: &Store, flat: &[f32]) -> usize {
    let white = store
        .manifest
        .frames
        .iter()
        .flat_map(|f| f.black.iter().map(move |b| f.white - b))
        .fold(f32::INFINITY, f32::min);
    let mut n = 0;
    for ((rgb, _), f) in sky
        .rgb
        .iter_mut()
        .zip(&sky.clipped)
        .zip(flat)
        .filter(|((_, c), _)| **c)
    {
        *rgb = [white / f; 3];
        n += 1;
    }
    n
}

fn percentile(values: &mut [f64], q: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    crate::sky::report::percentile(values, q).unwrap_or(f64::NAN)
}

pub struct Options<'a> {
    pub kernel: cfa::Kernel,
    /// The device for the sky stack (`gpu::stack`); the CPU when absent.
    pub gpu: Option<&'a crate::gpu::Gpu>,
    /// Frames lit by local light (headlamps, cars, light painting), left out of the ground.
    pub lit: &'a [usize],
    /// Moving foreground (swaying branches, water) from the reference minutes, sharp but
    /// noisier, instead of the long exposure's blur (the owner's default).
    pub sharp_moving: bool,
}

/// Stacks the store in place: `sky.planes`, `ground.planes`, `final.planes`, `stack.json`.
pub fn stack(store: &Store, o: &Options, budget: &Budget) -> Result<Report> {
    let Options {
        kernel,
        gpu,
        lit,
        sharp_moving,
    } = *o;
    let (w, h, n) = (
        store.manifest.width,
        store.manifest.height,
        store.manifest.frames.len(),
    );
    let clock = std::time::Instant::now();
    let model = separate::model(store)?;
    let matte = planes::load(&store.dir.join("matte.planes"))?;
    let alpha = matte.get("alpha")?;
    let unstable = matte.get("unstable")?;
    let flat = recorded_flat(store)?;
    let glow = separate::load_glow(store)?;
    let clearance = composite::clearance(alpha, w, h);
    let every = vec![true; n];
    let inputs = cfa::Inputs {
        store,
        model: &model,
        alpha,
        clearance: &clearance,
        glow: &glow,
        flat: &flat,
        kernel,
        occlusion: "occ",
        frames: &every,
    };
    let mut sky = match gpu {
        Some(gpu) => crate::gpu::stack::stack(gpu, &inputs, budget)?,
        None => cfa::stack(&inputs, budget)?,
    };
    let t_sky = clock.elapsed().as_secs_f64();
    let filled = saturate(&mut sky, store, &flat);
    let mask: Vec<bool> = alpha.iter().map(|&a| a <= 0.5).collect();
    let fg = foreground::foreground(
        store,
        &flat,
        &mask,
        &model.camera,
        model.reference,
        None,
        lit,
        budget,
    )?;
    let (mut ground, window) = (fg.ground, fg.window);
    let mut moving = 0;
    for p in 0..w * h {
        if sharp_moving && unstable[p] > 0.5 {
            ground[p] = window[p];
            moving += 1;
        }
    }
    let t_ground = clock.elapsed().as_secs_f64() - t_sky;
    let per_frame = std::f32::consts::PI * (kernel.radius * kernel.radius) as f32 / 4.0;
    let coverage: Vec<f32> = sky
        .kept
        .iter()
        .map(|k| k[0] as f32 / (per_frame * n as f32))
        .collect();
    let (image, missing) = composite::composite(&sky.rgb, &ground, alpha, &window, &coverage);
    let channel = |v: &[[f32; 3]], k: usize| v.iter().map(|c| c[k]).collect::<Vec<f32>>();
    planes::save(
        &store.dir.join("sky.planes"),
        w,
        h,
        &[
            ("r", &channel(&sky.rgb, 0)),
            ("g", &channel(&sky.rgb, 1)),
            ("b", &channel(&sky.rgb, 2)),
        ],
    )?;
    planes::save(
        &store.dir.join("ground.planes"),
        w,
        h,
        &[
            ("r", &channel(&ground, 0)),
            ("g", &channel(&ground, 1)),
            ("b", &channel(&ground, 2)),
        ],
    )?;
    planes::save(
        &store.dir.join("final.planes"),
        w,
        h,
        &[
            ("r", &channel(&image, 0)),
            ("g", &channel(&image, 1)),
            ("b", &channel(&image, 2)),
        ],
    )?;
    let clear: Vec<usize> = (0..w * h).filter(|&p| alpha[p] >= CLEAR).collect();
    let per = |k: usize, q: f64| {
        percentile(
            &mut clear
                .iter()
                .map(|&p| f64::from(sky.kept[p][k]))
                .collect::<Vec<_>>(),
            q,
        )
    };
    let samples_median = [per(0, 50.0), per(1, 50.0), per(2, 50.0)];
    let samples_p1 = [per(0, 1.0), per(1, 1.0), per(2, 1.0)];
    let stats = planes::load(&store.dir.join("stats.planes"))?;
    let sigma = stats.get("sigma")?;
    let single = percentile(
        &mut clear
            .iter()
            .map(|&p| f64::from(sigma[p]))
            .collect::<Vec<_>>(),
        50.0,
    );
    let mut diff: Vec<f64> = clear
        .iter()
        .map(|&p| {
            (0..3)
                .map(|k| f64::from(sky.halves[0][p][k] - sky.halves[1][p][k]) / 2.0)
                .sum::<f64>()
        })
        .filter(|v| v.is_finite())
        .collect();
    let centre = percentile(&mut diff, 50.0);
    let stack_noise = 1.4826
        * percentile(
            &mut diff.iter().map(|v| (v - centre).abs()).collect::<Vec<_>>(),
            50.0,
        );
    let report = Report {
        kernel,
        samples_per_colour_median: samples_median,
        samples_per_colour_p1: samples_p1,
        saturated_filled_pixels: filled,
        sharp_moving_pixels: moving,
        sky_missing_pixels: missing.iter().filter(|&&m| m).count(),
        ground: fg.report,
        single_noise_adu: single,
        stack_noise_adu: stack_noise,
        device: gpu.map_or_else(|| "CPU".into(), |g| g.name.clone()),
        seconds: [
            t_sky,
            t_ground,
            clock.elapsed().as_secs_f64() - t_sky - t_ground,
        ],
    };
    std::fs::write(
        store.dir.join("stack.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
