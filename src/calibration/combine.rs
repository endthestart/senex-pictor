use crate::{Result, error};
use serde::Serialize;

/// Per-site estimator for combining calibration frames.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Estimator {
    Mean,
    /// Mean after iteratively rejecting samples farther than `k` winsorized standard
    /// deviations from the median of the remaining samples.
    WinsorizedSigmaClip {
        k: f32,
        iterations: u32,
    },
}

pub(crate) struct Combined {
    pub data: Vec<f32>,
    pub accepted: Vec<u16>,
    pub standard_error: Vec<f32>,
}

/// Combines `frames` samples at each of `sites`; `sample(frame, site)` supplies values.
pub(crate) fn combine(
    frames: usize,
    sites: usize,
    estimator: Estimator,
    sample: impl Fn(usize, usize) -> f32 + Sync,
) -> Result<Combined> {
    if frames < 2 {
        return Err(error(
            "a master needs at least two frames to estimate its noise",
        ));
    }
    if frames > usize::from(u16::MAX) {
        return Err(error("too many frames for one master"));
    }
    if let Estimator::WinsorizedSigmaClip { k, iterations } = estimator
        && (!k.is_finite() || k < 1.0 || iterations == 0)
    {
        return Err(error(
            "sigma clipping needs k >= 1 and at least one iteration",
        ));
    }
    let mut combined = Combined {
        data: vec![0.0; sites],
        accepted: vec![0; sites],
        standard_error: vec![0.0; sites],
    };
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = sites.div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let parts = combined
            .data
            .chunks_mut(chunk)
            .zip(combined.accepted.chunks_mut(chunk))
            .zip(combined.standard_error.chunks_mut(chunk))
            .enumerate();
        for (index, ((data, accepted), standard_error)) in parts {
            let sample = &sample;
            scope.spawn(move || {
                let mut values = Vec::with_capacity(frames);
                let mut scratch = Vec::with_capacity(frames);
                let mut winsorized = Vec::with_capacity(frames);
                for offset in 0..data.len() {
                    let site = index * chunk + offset;
                    values.clear();
                    values.extend((0..frames).map(|frame| f64::from(sample(frame, site))));
                    reject(&mut values, &mut winsorized, &mut scratch, estimator);
                    let n = values.len() as f64;
                    data[offset] = (values.iter().sum::<f64>() / n) as f32;
                    accepted[offset] = values.len() as u16;
                    standard_error[offset] = (deviation(&values) / n.sqrt()) as f32;
                }
            });
        }
    });
    Ok(combined)
}

/// Winsorized sigma clipping: sigma comes from the sample winsorized at 1.5 sigma about the
/// median, scaled by 1.134 (the inverse standard deviation of a normal winsorized at 1.5);
/// samples beyond `k` sigma from the median are rejected. At least two samples remain.
fn reject(
    values: &mut Vec<f64>,
    winsorized: &mut Vec<f64>,
    scratch: &mut Vec<f64>,
    estimator: Estimator,
) {
    let Estimator::WinsorizedSigmaClip { k, iterations } = estimator else {
        return;
    };
    for _ in 0..iterations {
        let center = median(values, scratch);
        let mut sigma = deviation(values);
        for _ in 0..50 {
            winsorized.clear();
            winsorized.extend(
                values
                    .iter()
                    .map(|v| v.clamp(center - 1.5 * sigma, center + 1.5 * sigma)),
            );
            let next = 1.134 * deviation(winsorized);
            let converged = (next - sigma).abs() <= 5e-4 * sigma;
            sigma = next;
            if converged {
                break;
            }
        }
        let limit = f64::from(k) * sigma;
        let kept = values
            .iter()
            .filter(|v| (*v - center).abs() <= limit)
            .count();
        if sigma == 0.0 || kept == values.len() || kept < 2 {
            return;
        }
        values.retain(|v| (v - center).abs() <= limit);
    }
}

fn deviation(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
}

pub(crate) fn median(values: &[f64], scratch: &mut Vec<f64>) -> f64 {
    scratch.clear();
    scratch.extend_from_slice(values);
    let mid = scratch.len() / 2;
    let (lower, upper, _) = scratch.select_nth_unstable_by(mid, f64::total_cmp);
    if values.len() % 2 == 1 {
        *upper
    } else {
        let below = lower.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (below + *upper) / 2.0
    }
}
