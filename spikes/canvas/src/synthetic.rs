//! A synthetic nightscape for machines without a processed store: sky glow brightening toward a
//! ragged treeline, a Milky Way band, tens of thousands of stars, and a dark foreground with a
//! warm lit patch. Linear sRGB with sensor white at 1; deterministic.

use crate::source::{self, Scene};
use star_photos::Result;

const STARS: usize = 60_000;

struct Random(u64);

impl Random {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Smooth noise in 0..1: bilinear value noise over `octaves` grids.
fn noise(grids: &[(usize, Vec<f32>)], x: f32, y: f32) -> f32 {
    let mut sum = 0.0;
    let mut weight = 0.0;
    for (k, (n, g)) in grids.iter().enumerate() {
        let (gx, gy) = (x * (*n - 1) as f32, y * (*n - 1) as f32);
        let (i, j) = ((gx as usize).min(n - 2), (gy as usize).min(n - 2));
        let (fx, fy) = (gx - i as f32, gy - j as f32);
        let v = |a: usize, b: usize| g[b * n + a];
        let value = (v(i, j) * (1.0 - fx) + v(i + 1, j) * fx) * (1.0 - fy)
            + (v(i, j + 1) * (1.0 - fx) + v(i + 1, j + 1) * fx) * fy;
        let w = 0.5f32.powi(k as i32);
        sum += value * w;
        weight += w;
    }
    sum / weight
}

pub fn scene(width: u32, height: u32) -> Result<Scene> {
    let (w, h) = (width as usize, height as usize);
    let mut random = Random(7);
    let grids: Vec<(usize, Vec<f32>)> = [8usize, 17, 33, 65, 129]
        .iter()
        .map(|&n| (n, (0..n * n).map(|_| random.next()).collect()))
        .collect();
    let horizon: Vec<f32> = (0..w)
        .map(|x| {
            let u = x as f32 / w as f32;
            let ridge = 0.72 + 0.04 * (u * 9.0).sin() + 0.02 * (u * 31.0).sin();
            let tree = (noise(&grids, u, 0.13) - 0.5).max(0.0) * 0.5;
            let spike = ((u * 211.0).sin().abs().powf(12.0)) * 0.12;
            (ridge - tree - spike) * h as f32
        })
        .collect();
    let mut base = vec![[0f32; 3]; w * h];
    let mut glow = vec![[0f32; 3]; w * h];
    let mut alpha = vec![0f32; w * h];
    for y in 0..h {
        for (x, &edge) in horizon.iter().enumerate() {
            let p = y * w + x;
            let (u, v) = (x as f32 / w as f32, y as f32 / h as f32);
            let low = (v / 0.75).clamp(0.0, 1.0).powi(3);
            let pollution = [0.030 * low, 0.022 * low, 0.012 * low];
            if (y as f32) < edge {
                let band = (-((v - 0.9 * u + 0.05).powi(2)) / 0.012).exp();
                let milky = band * (0.4 + 0.9 * noise(&grids, u, v)).powi(2) * 0.05;
                glow[p] = pollution;
                base[p] = [
                    0.008 + milky * 1.05 + pollution[0],
                    0.009 + milky + pollution[1],
                    0.011 + milky * 0.95 + pollution[2],
                ];
                alpha[p] = 1.0;
            } else {
                let lit = (-((u - 0.62).powi(2) + (v - 0.86).powi(2)) / 0.004).exp() * 0.06;
                let texture = 0.004 + 0.006 * noise(&grids, u * 3.0 % 1.0, v);
                base[p] = [
                    texture + lit,
                    texture * 0.9 + lit * 0.7,
                    texture * 0.8 + lit * 0.4,
                ];
            }
        }
    }
    for _ in 0..STARS {
        let (sx, sy) = (random.next() * w as f32, random.next() * h as f32);
        if sy >= horizon[(sx as usize).min(w - 1)] {
            continue;
        }
        let flux = 0.02 * random.next().powf(-0.6).min(60.0);
        let tint = [
            1.0 + 0.3 * (random.next() - 0.5),
            1.0,
            1.0 - 0.3 * (random.next() - 0.5),
        ];
        let sigma = 1.1f32;
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let (x, y) = (sx as i32 + dx, sy as i32 + dy);
                if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                    continue;
                }
                let r2 = (x as f32 + 0.5 - sx).powi(2) + (y as f32 + 0.5 - sy).powi(2);
                let v = flux * (-r2 / (2.0 * sigma * sigma)).exp();
                let p = y as usize * w + x as usize;
                for k in 0..3 {
                    base[p][k] += v * tint[k];
                }
            }
        }
    }
    let clear: Vec<bool> = alpha.iter().map(|&a| a >= 0.98).collect();
    source::assemble(width, height, &base, &glow, &alpha, &clear, 1.0)
}
