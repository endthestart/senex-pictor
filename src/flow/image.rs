//! Single-channel images for foreground tracking: the star-free feature image (half-resolution
//! log luminance, median-filtered so stars moving with the sky drop out), Gaussian pyramids,
//! derivatives and bilinear sampling.

use rayon::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Gray {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

/// Median filter size applied twice to the feature image.
const MEDIAN: usize = 5;

/// Where a coordinate falls along one axis: the samples either side and the fraction between.
#[derive(Clone, Copy)]
pub struct Tap {
    lo: usize,
    hi: usize,
    f: f32,
}

impl Tap {
    /// The tap at `v` along an axis of `n` samples, or None outside it.
    pub fn at(v: f64, n: usize) -> Option<Tap> {
        if !(v >= 0.0 && v <= (n - 1) as f64) {
            return None;
        }
        let lo = v.floor() as usize;
        Some(Tap {
            lo,
            hi: (lo + 1).min(n - 1),
            f: (v - lo as f64) as f32,
        })
    }
}

impl Gray {
    pub fn at(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }

    /// Bilinear sample at the taps `x` and `y`.
    pub fn sample(&self, x: Tap, y: Tap) -> f32 {
        let (r0, r1) = (y.lo * self.width, y.hi * self.width);
        let top = self.data[r0 + x.lo] * (1.0 - x.f) + self.data[r0 + x.hi] * x.f;
        let bottom = self.data[r1 + x.lo] * (1.0 - x.f) + self.data[r1 + x.hi] * x.f;
        top * (1.0 - y.f) + bottom * y.f
    }

    /// Gaussian [1 4 6 4 1]/16 blur and 2× decimation (OpenCV pyrDown).
    pub fn down(&self) -> Gray {
        const K: [f32; 5] = [1.0, 4.0, 6.0, 4.0, 1.0];
        let (w, h) = (self.width.div_ceil(2), self.height.div_ceil(2));
        let mut rows = vec![0f32; w * self.height];
        rows.par_chunks_mut(w.max(1))
            .zip(self.data.par_chunks(self.width.max(1)))
            .for_each(|(out, row)| {
                for (x, o) in out.iter_mut().enumerate() {
                    let s: f32 = (0..5)
                        .map(|k| K[k] * row[reflect(2 * x as isize + k as isize - 2, self.width)])
                        .sum();
                    *o = s / 16.0;
                }
            });
        let mut data = vec![0f32; w * h];
        data.par_chunks_mut(w.max(1))
            .enumerate()
            .for_each(|(y, out)| {
                let taps: [&[f32]; 5] = std::array::from_fn(|k| {
                    let yy = reflect(2 * y as isize + k as isize - 2, self.height);
                    &rows[yy * w..(yy + 1) * w]
                });
                for (x, o) in out.iter_mut().enumerate() {
                    let s: f32 = (0..5).map(|k| K[k] * taps[k][x]).sum();
                    *o = s / 16.0;
                }
            });
        Gray {
            width: w,
            height: h,
            data,
        }
    }

    /// Scharr derivatives (x, y) per pixel, scaled to intensity per pixel.
    pub fn gradients(&self) -> (Gray, Gray) {
        let (w, h) = (self.width, self.height);
        let (mut gx, mut gy) = (vec![0f32; w * h], vec![0f32; w * h]);
        gx.par_chunks_mut(w.max(1))
            .zip(gy.par_chunks_mut(w.max(1)))
            .enumerate()
            .for_each(|(y, (ox, oy))| {
                let row = |dy: isize| {
                    let yy = reflect(y as isize + dy, h);
                    &self.data[yy * w..(yy + 1) * w]
                };
                let (up, mid, down) = (row(-1), row(0), row(1));
                for x in 0..w {
                    let (l, r) = (reflect(x as isize - 1, w), reflect(x as isize + 1, w));
                    ox[x] = (3.0 * (up[r] - up[l])
                        + 10.0 * (mid[r] - mid[l])
                        + 3.0 * (down[r] - down[l]))
                        / 32.0;
                    oy[x] = (3.0 * (down[l] - up[l])
                        + 10.0 * (down[x] - up[x])
                        + 3.0 * (down[r] - up[r]))
                        / 32.0;
                }
            });
        (
            Gray {
                width: w,
                height: h,
                data: gx,
            },
            Gray {
                width: w,
                height: h,
                data: gy,
            },
        )
    }
}

/// `v` reflected into 0..n about its edge samples (OpenCV's BORDER_REFLECT_101).
fn reflect(v: isize, n: usize) -> usize {
    let n = n as isize;
    let v = if v < 0 { -v } else { v };
    (if v >= n { 2 * n - 2 - v } else { v }).clamp(0, n - 1) as usize
}

/// 2×2 block sums of an image (odd last row or column dropped).
pub fn half_sums(image: &[f32], width: usize, height: usize) -> (Vec<f32>, usize, usize) {
    let (w, h) = (width / 2, height / 2);
    let sums = (0..w * h)
        .map(|i| {
            let (x, y) = (2 * (i % w), 2 * (i / w));
            image[y * width + x]
                + image[y * width + x + 1]
                + image[(y + 1) * width + x]
                + image[(y + 1) * width + x + 1]
        })
        .collect();
    (sums, w, h)
}

/// The feature image: log1p(max(v, 0) / scale) stretched to 0–255 and quantised, then
/// MEDIAN×MEDIAN median-filtered twice.
pub fn features(image: &[f32], width: usize, height: usize, scale: f32) -> Gray {
    let v: Vec<f32> = image.iter().map(|x| (x.max(0.0) / scale).ln_1p()).collect();
    let (lo, hi) = v
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    let span = (hi - lo).max(f32::MIN_POSITIVE);
    let levels: Vec<u8> = v
        .iter()
        .map(|x| ((x - lo) / span * 255.0).round() as u8)
        .collect();
    Gray {
        width,
        height,
        data: median(&median(&levels, width, height), width, height)
            .into_iter()
            .map(f32::from)
            .collect(),
    }
}

/// MEDIAN×MEDIAN median of 8-bit levels, edges replicated: per row a running histogram whose
/// median moves as each column enters and leaves (Huang's filter).
fn median(levels: &[u8], width: usize, height: usize) -> Vec<u8> {
    let r = (MEDIAN / 2) as isize;
    let clamp = |v: isize, n: usize| v.clamp(0, n as isize - 1) as usize;
    let mut out = vec![0u8; levels.len()];
    out.par_chunks_mut(width.max(1))
        .enumerate()
        .for_each(|(y, row)| {
            let rows: [usize; MEDIAN] =
                std::array::from_fn(|k| clamp(y as isize + k as isize - r, height));
            let column = |x: isize| rows.map(|yy| levels[yy * width + clamp(x, width)]);
            let mut window = Window {
                histogram: [0; 256],
                at: 0,
                below: 0,
            };
            for dx in -r..=r {
                column(dx).into_iter().for_each(|v| window.add(v));
            }
            for (x, out) in (0..).zip(row.iter_mut()) {
                if x > 0 {
                    column(x - 1 - r).into_iter().for_each(|v| window.remove(v));
                    column(x + r).into_iter().for_each(|v| window.add(v));
                }
                *out = window.median();
            }
        });
    out
}

/// A histogram of levels and its median: the level `at` with `below` levels under it.
struct Window {
    histogram: [u32; 256],
    at: usize,
    below: u32,
}

impl Window {
    /// Rank of the median among MEDIAN² levels, from 0.
    const RANK: u32 = (MEDIAN * MEDIAN / 2) as u32;

    fn add(&mut self, v: u8) {
        self.histogram[usize::from(v)] += 1;
        self.below += u32::from(usize::from(v) < self.at);
    }

    fn remove(&mut self, v: u8) {
        self.histogram[usize::from(v)] -= 1;
        self.below -= u32::from(usize::from(v) < self.at);
    }

    fn median(&mut self) -> u8 {
        while self.below > Self::RANK {
            self.at -= 1;
            self.below -= self.histogram[self.at];
        }
        while self.below + self.histogram[self.at] <= Self::RANK {
            self.below += self.histogram[self.at];
            self.at += 1;
        }
        self.at as u8
    }
}
