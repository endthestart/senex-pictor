//! Single-channel images for foreground tracking: the star-free feature image (half-resolution
//! log luminance, median-filtered so stars moving with the sky drop out), Gaussian pyramids,
//! derivatives and bilinear sampling.

#[derive(Clone, Debug, PartialEq)]
pub struct Gray {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

/// Median filter size applied twice to the feature image.
const MEDIAN: usize = 5;

impl Gray {
    pub fn at(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }

    /// Bilinear sample at (x, y), or None outside the image.
    pub fn sample(&self, x: f64, y: f64) -> Option<f32> {
        if !(x >= 0.0 && y >= 0.0 && x <= (self.width - 1) as f64 && y <= (self.height - 1) as f64)
        {
            return None;
        }
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
        let top = self.at(x0, y0) * (1.0 - fx) + self.at(x1, y0) * fx;
        let bottom = self.at(x0, y1) * (1.0 - fx) + self.at(x1, y1) * fx;
        Some(top * (1.0 - fy) + bottom * fy)
    }

    fn clamped(&self, x: isize, y: isize) -> f32 {
        let reflect = |v: isize, n: usize| {
            let n = n as isize;
            let v = if v < 0 { -v } else { v };
            (if v >= n { 2 * n - 2 - v } else { v }).clamp(0, n - 1) as usize
        };
        self.at(reflect(x, self.width), reflect(y, self.height))
    }

    /// Gaussian [1 4 6 4 1]/16 blur and 2× decimation (OpenCV pyrDown).
    pub fn down(&self) -> Gray {
        const K: [f32; 5] = [1.0, 4.0, 6.0, 4.0, 1.0];
        let (w, h) = (self.width.div_ceil(2), self.height.div_ceil(2));
        let mut rows = vec![0f32; w * self.height];
        for y in 0..self.height {
            for x in 0..w {
                let s: f32 = (0..5)
                    .map(|k| K[k] * self.clamped(2 * x as isize + k as isize - 2, y as isize))
                    .sum();
                rows[y * w + x] = s / 16.0;
            }
        }
        let tmp = Gray {
            width: w,
            height: self.height,
            data: rows,
        };
        let mut data = vec![0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let s: f32 = (0..5)
                    .map(|k| K[k] * tmp.clamped(x as isize, 2 * y as isize + k as isize - 2))
                    .sum();
                data[y * w + x] = s / 16.0;
            }
        }
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
        for y in 0..h as isize {
            for x in 0..w as isize {
                let p = |dx: isize, dy: isize| self.clamped(x + dx, y + dy);
                let i = y as usize * w + x as usize;
                gx[i] = (3.0 * (p(1, -1) - p(-1, -1))
                    + 10.0 * (p(1, 0) - p(-1, 0))
                    + 3.0 * (p(1, 1) - p(-1, 1)))
                    / 32.0;
                gy[i] = (3.0 * (p(-1, 1) - p(-1, -1))
                    + 10.0 * (p(0, 1) - p(0, -1))
                    + 3.0 * (p(1, 1) - p(1, -1)))
                    / 32.0;
            }
        }
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
    let mut v: Vec<f32> = image.iter().map(|x| (x.max(0.0) / scale).ln_1p()).collect();
    let (lo, hi) = v
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    let span = (hi - lo).max(f32::MIN_POSITIVE);
    for x in &mut v {
        *x = ((*x - lo) / span * 255.0).round();
    }
    let g = Gray {
        width,
        height,
        data: v,
    };
    median(&median(&g))
}

fn median(g: &Gray) -> Gray {
    let r = (MEDIAN / 2) as isize;
    let mut window = [0f32; MEDIAN * MEDIAN];
    let mut data = vec![0f32; g.data.len()];
    for y in 0..g.height as isize {
        for x in 0..g.width as isize {
            let mut k = 0;
            for dy in -r..=r {
                for dx in -r..=r {
                    let (xx, yy) = (
                        (x + dx).clamp(0, g.width as isize - 1),
                        (y + dy).clamp(0, g.height as isize - 1),
                    );
                    window[k] = g.at(xx as usize, yy as usize);
                    k += 1;
                }
            }
            data[y as usize * g.width + x as usize] =
                *window.select_nth_unstable_by(k / 2, f32::total_cmp).1;
        }
    }
    Gray {
        width: g.width,
        height: g.height,
        data,
    }
}
