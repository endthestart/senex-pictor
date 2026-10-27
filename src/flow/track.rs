//! Corners and pyramidal Lucas–Kanade tracking (after Shi–Tomasi and Bouguet, as in OpenCV's
//! goodFeaturesToTrack and calcOpticalFlowPyrLK, the prototype's oracle).

use super::image::Gray;
use crate::stars::similarity::Point;
use rayon::prelude::*;

pub const WINDOW: usize = 31;
pub const LEVELS: usize = 3;
const ROUNDS: usize = 30;
const STEP_EPSILON: f64 = 0.01;
/// Smallest eigenvalue of the window's gradient matrix per pixel, in (intensity/px)², for a
/// point to be trackable: OpenCV's default 1e-4 in its fixed-point units (Scharr ×32, 2⁻²⁰).
const MIN_EIGEN: f64 = 1e-4 * 1024.0;
const QUALITY: f64 = 0.01;

/// Up to `count` corners where `allowed`, strongest first, at least `spacing` px apart, whose
/// smaller structure-tensor eigenvalue (3×3 Sobel window) reaches QUALITY of the strongest.
pub fn corners(image: &Gray, allowed: &[bool], count: usize, spacing: f64) -> Vec<Point> {
    let (w, h) = (image.width, image.height);
    let (gx, gy) = image.gradients();
    let mut score = vec![0f32; w * h];
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let (mut a, mut b, mut c) = (0f32, 0f32, 0f32);
            for yy in y - 1..=y + 1 {
                for xx in x - 1..=x + 1 {
                    let (u, v) = (gx.at(xx, yy), gy.at(xx, yy));
                    a += u * u;
                    b += u * v;
                    c += v * v;
                }
            }
            score[y * w + x] = (a + c) / 2.0 - (((a - c) / 2.0).powi(2) + b * b).sqrt();
        }
    }
    let best = score
        .iter()
        .zip(allowed)
        .filter(|(_, ok)| **ok)
        .fold(0f32, |m, (s, _)| m.max(*s));
    let floor = (QUALITY * f64::from(best)) as f32;
    let mut candidates: Vec<(f32, usize)> = (0..w * h)
        .filter(|&i| allowed[i] && score[i] > floor && score[i] > 0.0)
        .filter(|&i| {
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            (-1..=1).all(|dy| {
                (-1..=1).all(|dx| {
                    let (xx, yy) = (x + dx, y + dy);
                    xx < 0
                        || yy < 0
                        || xx >= w as isize
                        || yy >= h as isize
                        || score[yy as usize * w + xx as usize] <= score[i]
                })
            })
        })
        .map(|i| (score[i], i))
        .collect();
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut kept: Vec<Point> = Vec::new();
    let cell = spacing.max(1.0);
    let columns = (w as f64 / cell) as usize + 1;
    let mut grid: Vec<Vec<Point>> = vec![Vec::new(); columns * ((h as f64 / cell) as usize + 1)];
    for (_, i) in candidates {
        if kept.len() >= count {
            break;
        }
        let p = [(i % w) as f64, (i / w) as f64];
        let (cx, cy) = ((p[0] / cell) as isize, (p[1] / cell) as isize);
        let crowded = (cy - 1..=cy + 1).any(|j| {
            (cx - 1..=cx + 1).any(|k| {
                j >= 0
                    && k >= 0
                    && (k as usize) < columns
                    && (j as usize * columns + k as usize) < grid.len()
                    && grid[j as usize * columns + k as usize]
                        .iter()
                        .any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < spacing)
            })
        });
        if !crowded {
            grid[cy as usize * columns + cx as usize].push(p);
            kept.push(p);
        }
    }
    kept
}

struct Level {
    image: Gray,
    gx: Gray,
    gy: Gray,
}

pub struct Pyramid(Vec<Level>);

impl Pyramid {
    pub fn new(image: &Gray) -> Pyramid {
        let mut levels = Vec::with_capacity(LEVELS + 1);
        let mut current = image.clone();
        for k in 0..=LEVELS {
            let (gx, gy) = current.gradients();
            let next = (k < LEVELS).then(|| current.down());
            levels.push(Level {
                image: current,
                gx,
                gy,
            });
            match next {
                Some(n) => current = n,
                None => break,
            }
        }
        Pyramid(levels)
    }
}

/// Where each of `points` in `from` went in `to`, starting from `guess` (or the point
/// itself); None where the window leaves the image or has too little texture.
pub fn follow(
    from: &Pyramid,
    to: &Pyramid,
    points: &[Point],
    guess: Option<&[Point]>,
) -> Vec<Option<Point>> {
    let half = (WINDOW / 2) as f64;
    points
        .par_iter()
        .enumerate()
        .map(|(n, p)| {
            let start = guess.map_or(*p, |g| g[n]);
            let mut flow = [0.0f64, 0.0];
            for k in (0..from.0.len()).rev() {
                let scale = (1u32 << k) as f64;
                let (a, b) = (&from.0[k], &to.0[k]);
                let centre = [p[0] / scale, p[1] / scale];
                if k == from.0.len() - 1 {
                    flow = [(start[0] - p[0]) / scale, (start[1] - p[1]) / scale];
                } else {
                    flow = [flow[0] * 2.0, flow[1] * 2.0];
                }
                let mut patch = Vec::with_capacity(WINDOW * WINDOW);
                let (mut gxx, mut gxy, mut gyy) = (0.0f64, 0.0f64, 0.0f64);
                for j in 0..WINDOW {
                    for i in 0..WINDOW {
                        let (x, y) = (centre[0] + i as f64 - half, centre[1] + j as f64 - half);
                        let (Some(v), Some(dx), Some(dy)) =
                            (a.image.sample(x, y), a.gx.sample(x, y), a.gy.sample(x, y))
                        else {
                            return None;
                        };
                        let (dx, dy) = (f64::from(dx), f64::from(dy));
                        gxx += dx * dx;
                        gxy += dx * dy;
                        gyy += dy * dy;
                        patch.push((x, y, f64::from(v), dx, dy));
                    }
                }
                let area = (WINDOW * WINDOW) as f64;
                let min_eigen =
                    ((gxx + gyy) / 2.0 - (((gxx - gyy) / 2.0).powi(2) + gxy * gxy).sqrt()) / area;
                let det = gxx * gyy - gxy * gxy;
                if min_eigen < MIN_EIGEN || det <= 0.0 {
                    return None;
                }
                for _ in 0..ROUNDS {
                    let (mut bx, mut by) = (0.0, 0.0);
                    for &(x, y, v, dx, dy) in &patch {
                        let w = f64::from(b.image.sample(x + flow[0], y + flow[1])?);
                        bx += (v - w) * dx;
                        by += (v - w) * dy;
                    }
                    let step = [(gyy * bx - gxy * by) / det, (gxx * by - gxy * bx) / det];
                    flow = [flow[0] + step[0], flow[1] + step[1]];
                    if step[0].hypot(step[1]) < STEP_EPSILON {
                        break;
                    }
                }
            }
            Some([p[0] + flow[0], p[1] + flow[1]])
        })
        .collect()
}

/// `follow` there and back: points that return within `tolerance` px of where they started.
pub fn follow_checked(
    from: &Pyramid,
    to: &Pyramid,
    points: &[Point],
    guess: Option<&[Point]>,
    tolerance: f64,
) -> Vec<Option<Point>> {
    let forward = follow(from, to, points, guess);
    let found: Vec<Point> = forward
        .iter()
        .zip(points)
        .map(|(f, p)| f.unwrap_or(*p))
        .collect();
    let back = follow(to, from, &found, Some(points));
    forward
        .iter()
        .zip(&back)
        .zip(points)
        .map(|((f, b), p)| match (f, b) {
            (Some(f), Some(b)) if (b[0] - p[0]).hypot(b[1] - p[1]) < tolerance => Some(*f),
            _ => None,
        })
        .collect()
}
