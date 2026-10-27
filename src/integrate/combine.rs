//! Robust combination of a pixel's samples across frames: σ-clipping about the median
//! (σ = 1.4826 MAD, CLIP_K σ, CLIP_ROUNDS rounds, only with at least three references),
//! then a (weighted) mean of what is kept, also split into even and odd frames.

pub const CLIP_K: f32 = 4.0;
pub const CLIP_ROUNDS: usize = 2;
/// CFA samples at least this heavy (near their site) set the clipping centre.
pub const CENTRE_WEIGHT: f32 = 0.3;

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub value: f32,
    pub weight: f32,
    pub frame: u32,
}

fn median(v: &mut [f32]) -> f32 {
    crate::separate::stats::median(v)
}

/// Which of `values` survive clipping; `reference` marks the values that set the centre.
fn clip(values: &[f32], reference: &[bool]) -> Vec<bool> {
    let mut keep = vec![true; values.len()];
    let mut buf = Vec::with_capacity(values.len());
    for _ in 0..CLIP_ROUNDS {
        buf.clear();
        buf.extend(
            values
                .iter()
                .zip(&keep)
                .zip(reference)
                .filter(|((_, k), r)| **k && **r)
                .map(|((v, _), _)| *v),
        );
        if buf.len() < 3 {
            break;
        }
        let centre = median(&mut buf);
        for b in buf.iter_mut() {
            *b = (*b - centre).abs();
        }
        let limit = CLIP_K * (1.4826 * median(&mut buf)).max(1e-3);
        for (k, v) in keep.iter_mut().zip(values) {
            if (v - centre).abs() > limit {
                *k = false;
            }
        }
    }
    keep
}

/// Weighted mean, kept count and even/odd-frame halves of one colour's CFA samples.
pub fn weighted(samples: &[Sample]) -> (f32, u32, [f32; 2]) {
    let values: Vec<f32> = samples.iter().map(|s| s.value).collect();
    let near: Vec<bool> = samples.iter().map(|s| s.weight >= CENTRE_WEIGHT).collect();
    let keep = clip(&values, &near);
    let (mut sum, mut total) = (0f64, 0f64);
    let (mut half_sum, mut half_total) = ([0f64; 2], [0f64; 2]);
    let mut kept = 0;
    for (s, k) in samples.iter().zip(&keep) {
        if !k {
            continue;
        }
        kept += 1;
        let (w, v) = (f64::from(s.weight), f64::from(s.value));
        sum += w * v;
        total += w;
        let parity = (s.frame % 2) as usize;
        half_sum[parity] += w * v;
        half_total[parity] += w;
    }
    let ratio = |s: f64, t: f64| if t > 0.0 { (s / t) as f32 } else { f32::NAN };
    (
        ratio(sum, total),
        kept,
        [
            ratio(half_sum[0], half_total[0]),
            ratio(half_sum[1], half_total[1]),
        ],
    )
}

/// Mean RGB of the frames whose luminance survives clipping, the kept and valid counts.
pub fn luminance_clipped(rgb: &[[f32; 3]]) -> ([f32; 3], u32, u32) {
    let valid: Vec<&[f32; 3]> = rgb
        .iter()
        .filter(|c| c.iter().all(|v| v.is_finite()))
        .collect();
    let lum: Vec<f32> = valid.iter().map(|c| c[0] + c[1] + c[2]).collect();
    let keep = clip(&lum, &vec![true; lum.len()]);
    let mut sum = [0f64; 3];
    let mut kept = 0;
    for (c, k) in valid.iter().zip(&keep) {
        if *k {
            kept += 1;
            for i in 0..3 {
                sum[i] += f64::from(c[i]);
            }
        }
    }
    let mean = if kept > 0 {
        sum.map(|s| (s / f64::from(kept)) as f32)
    } else {
        [f32::NAN; 3]
    };
    (mean, kept, valid.len() as u32)
}
