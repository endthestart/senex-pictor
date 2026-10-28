//! Hero composite (ADR 0010): one chosen frame is the photograph and the stacked sky goes
//! behind it. The hero keeps its own foreground, lighting and transient light (light
//! painting, headlights), or (for a drifting camera's whole-sequence composite) the
//! long-exposure ground moved to its moment; stacked sky replaces it only where the matte is
//! confidently sky.
//! The sky is a CFA stack of the frames from `first`, aligned to the hero's moment with a
//! free rotation per frame so drift cannot blur it. The matte comes from the frames around the
//! hero; for every other frame it is moved by that frame's foreground displacement, with
//! `margins::CLEARANCE_PX` plus the fit's residual as the margin for its stars and its glow samples.

use super::cfa::{self, Kernel};
use super::composite::clearance;
use super::foreground::{self, Foreground as Ground};
use super::ground::median_of;
use super::margins;
use super::transient::transient;
use crate::budget::Budget;
use crate::flow::displacement::foreground_displacement;
use crate::separate::glow::{self, CLEAR};
use crate::separate::{matte, stats};
use crate::sky::SkyModel;
use crate::sky::drift::free_rotations;
use crate::store::{Store, planes};
use crate::{Result, error, stars};
use serde::Serialize;

pub const WINDOW: usize = 12;

#[derive(Debug, Serialize)]
pub struct Report {
    pub hero: String,
    pub first: String,
    pub frames_used: usize,
    /// Frames whose foreground could not be tracked; they are left out.
    pub frames_lost: Vec<String>,
    pub foreground_fit_residual_p90_px: Vec<(String, f64)>,
    pub widest_shift_px: f64,
    pub glow_iteration_rms_change_adu: Vec<f64>,
    pub samples_per_colour_median: [f64; 3],
    pub transient_pixels: usize,
    /// The long-exposure foreground, when it was asked for instead of the frame.
    pub long_exposure: Option<foreground::Report>,
    pub seconds: f64,
}

/// What the foreground of the composite is.
#[derive(Clone, Copy)]
pub enum Foreground<'a> {
    /// The hero frame as shot, with its own transient light.
    Frame,
    /// The long exposure of the frames not in `lit`, moved to the hero's moment.
    LongExposure { lit: &'a [usize] },
}

pub struct Options<'a> {
    pub hero: usize,
    pub foreground: Foreground<'a>,
    pub first: usize,
    pub labels: Option<&'a [u32]>,
    /// A lens the files do not record, for its vignetting profile.
    pub lens: Option<&'a str>,
    pub kernel: Kernel,
    pub gpu: Option<&'a crate::gpu::Gpu>,
    /// The result's name in the store (`{name}.planes`, `{name}.json`): `hero`, or
    /// `hero-{frame}` for one of several.
    pub name: &'a str,
}

pub fn hero(store: &Store, o: &Options, budget: &Budget) -> Result<Report> {
    let Options {
        hero,
        foreground,
        first,
        labels,
        lens,
        kernel,
        gpu,
        name,
    } = *o;
    let m = &store.manifest;
    let (w, h, n) = (m.width, m.height, m.frames.len());
    if hero < first || hero >= n {
        return Err(error(
            "the hero frame must come at or after the first frame used",
        ));
    }
    let clock = std::time::Instant::now();
    let model = crate::separate::model(store)?;
    let fixed = SkyModel {
        rotations: None,
        ..model
    };
    let (found, tracks) = stars::io::load(&store.dir.join("stars.bin"))?;
    let sky_model = free_rotations(&fixed, &found, &tracks, hero)?;
    let w1 = (hero.saturating_sub(WINDOW / 2) + WINDOW).min(n);
    let w0 = w1.saturating_sub(WINDOW);
    let s = stats::temporal_stats(store, &fixed, w0..w1, budget)?;
    let mt = matte::matte(&s, w1 - w0, labels)?;
    let alpha = mt.alpha;
    let ground: Vec<bool> = alpha.iter().map(|&a| a <= 0.5).collect();
    let used: Vec<usize> = (first..n).collect();
    let camera = sky_model.camera;
    let tracked = foreground_displacement(store, &ground, hero, &used, Some(&camera), budget)?;
    let fields = &tracked.fields;
    let distance = clearance(&alpha, w, h);
    let widest = margins::write(store, "hero-occ", &distance, fields, &camera, budget)?;
    let (flat, _) = crate::separate::flat(store, lens)?;
    let (medians, _) = glow::block_medians(store, &alpha, &flat, Some("hero-occ"), budget)?;
    let mut frames = vec![false; n];
    for (i, _) in fields {
        frames[*i] = true;
    }
    let (glow, _, changes) = glow::decompose(&medians, &sky_model, &frames)?;
    let inputs = cfa::Inputs {
        store,
        model: &sky_model,
        alpha: &alpha,
        clearance: &distance,
        glow: &glow,
        flat: &flat,
        kernel,
        occlusion: "hero-occ",
        frames: &frames,
    };
    let sky = match gpu {
        Some(gpu) => crate::gpu::stack::stack(gpu, &inputs, budget)?,
        None => cfa::stack(&inputs, budget)?,
    };
    let divide = |rgb: Vec<f32>| -> Vec<[f32; 3]> {
        rgb.as_chunks::<3>()
            .0
            .iter()
            .zip(&flat)
            .map(|(c, f)| c.map(|v| v / f))
            .collect()
    };
    let (front, light, long) = match foreground {
        Foreground::Frame => {
            let frame = divide(store.rgb_rows(hero, 0, h)?);
            let still = median_of(store, &flat, w0..w1, budget)?;
            let mut clear_sigma: Vec<f32> = (0..w * h)
                .filter(|&p| alpha[p] >= CLEAR)
                .map(|p| s.sigma[p])
                .collect();
            if clear_sigma.is_empty() {
                return Err(error("hero: the window matte has no clear sky"));
            }
            let noise = stats::median(&mut clear_sigma) / 5.0;
            let light = transient(&frame, &still, &alpha, noise, w, h);
            (frame, light, None)
        }
        Foreground::LongExposure { lit } => {
            let Ground {
                mut ground,
                window,
                report,
            } = foreground::foreground(
                store,
                &flat,
                &ground,
                &camera,
                hero,
                Some(&tracked),
                lit,
                budget,
            )?;
            for (p, g) in ground.iter_mut().enumerate() {
                if mt.unstable[p] {
                    *g = window[p];
                }
            }
            (ground, vec![0.0; w * h], Some(report))
        }
    };
    let weight: Vec<f32> = (0..w * h)
        .map(|p| {
            let confident = ((alpha[p] - 0.5) / (CLEAR - 0.5)).clamp(0.0, 1.0);
            let finite = sky.rgb[p].iter().all(|v| v.is_finite());
            if finite {
                confident * (1.0 - light[p])
            } else {
                0.0
            }
        })
        .collect();
    let image: Vec<[f32; 3]> = (0..w * h)
        .map(|p| {
            let a = weight[p];
            std::array::from_fn(|k| {
                let s = if a > 0.0 { sky.rgb[p][k] } else { 0.0 };
                a * s + (1.0 - a) * front[p][k]
            })
        })
        .collect();
    let glow_hero: Vec<[f32; 3]> = (0..w * h)
        .map(|p| glow[hero].sample([(p % w) as f64, (p / w) as f64]))
        .collect();
    let channel = |v: &[[f32; 3]], k: usize| v.iter().map(|c| c[k]).collect::<Vec<f32>>();
    planes::save(
        &store.dir.join(format!("{name}.planes")),
        w,
        h,
        &[
            ("r", &channel(&image, 0)),
            ("g", &channel(&image, 1)),
            ("b", &channel(&image, 2)),
            ("alpha", &alpha),
            ("sky", &weight),
            ("transient", &light),
            ("glow_r", &channel(&glow_hero, 0)),
            ("glow_g", &channel(&glow_hero, 1)),
            ("glow_b", &channel(&glow_hero, 2)),
        ],
    )?;
    let clear: Vec<usize> = (0..w * h).filter(|&p| alpha[p] >= CLEAR).collect();
    let samples = std::array::from_fn(|k| {
        let mut v: Vec<f64> = clear.iter().map(|&p| f64::from(sky.kept[p][k])).collect();
        v.sort_by(f64::total_cmp);
        crate::sky::report::percentile(&v, 50.0).unwrap_or(0.0)
    });
    let report = Report {
        hero: m.frames[hero].name.clone(),
        first: m.frames[first].name.clone(),
        frames_used: fields.len(),
        frames_lost: tracked
            .lost
            .iter()
            .map(|&i| m.frames[i].name.clone())
            .collect(),
        foreground_fit_residual_p90_px: fields
            .iter()
            .map(|(i, d)| (m.frames[*i].name.clone(), d.residual_p90_px))
            .collect(),
        widest_shift_px: widest,
        glow_iteration_rms_change_adu: changes,
        samples_per_colour_median: samples,
        transient_pixels: light.iter().filter(|&&t| t > 0.5).count(),
        long_exposure: long,
        seconds: clock.elapsed().as_secs_f64(),
    };
    std::fs::write(
        store.dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
