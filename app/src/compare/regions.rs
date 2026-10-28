//! Where to look when comparing two renderings: regions chosen from the scene itself, each
//! showing what tends to differ between looks (sky noise and black level, star colour, sky
//! structure, halos at the horizon, foreground brightness). Blocks of BLOCK px are scored on a
//! sample of their pixels; each region is the best block for its question.

use crate::scene::Scene;
use half::f16;

const BLOCK: usize = 128;
const SAMPLE: usize = 4;
const SKY: f32 = 0.98;
const GROUND: f32 = 0.02;

pub struct Region {
    pub name: &'static str,
    pub look_for: &'static str,
    /// Centre in scene pixels.
    pub centre: [f32; 2],
}

struct Block {
    centre: [f32; 2],
    alpha: f32,
    median: f32,
    edge: f32,
}

fn luminance(scene: &Scene, p: usize) -> (f32, f32) {
    let v = |k: usize| f16::from_bits(scene.base[4 * p + k]).to_f32();
    (0.2126 * v(0) + 0.7152 * v(1) + 0.0722 * v(2), v(3))
}

fn blocks(scene: &Scene) -> Vec<Block> {
    let (w, h) = (scene.width as usize, scene.height as usize);
    let mut out = Vec::new();
    for by in (0..h.saturating_sub(BLOCK)).step_by(BLOCK) {
        for bx in (0..w.saturating_sub(BLOCK)).step_by(BLOCK) {
            let mut lum = Vec::new();
            let (mut alpha, mut edge) = (0.0, 0.0);
            for y in (by..by + BLOCK).step_by(SAMPLE) {
                for x in (bx..bx + BLOCK).step_by(SAMPLE) {
                    let (l, a) = luminance(scene, y * w + x);
                    lum.push(l);
                    alpha += a;
                    edge += f32::from(u8::from(a > GROUND && a < SKY));
                }
            }
            let n = lum.len() as f32;
            lum.sort_by(f32::total_cmp);
            out.push(Block {
                centre: [(bx + BLOCK / 2) as f32, (by + BLOCK / 2) as f32],
                alpha: alpha / n,
                median: lum[lum.len() / 2],
                edge: edge / n,
            });
        }
    }
    out
}

/// The brightest sky pixel short of saturation and at least half a block from the border,
/// sampled every other pixel.
fn brightest_star(scene: &Scene) -> Option<[f32; 2]> {
    let (w, h) = (scene.width as usize, scene.height as usize);
    let m = BLOCK / 2;
    let mut best: Option<(f32, [f32; 2])> = None;
    for y in (m..h.saturating_sub(m)).step_by(2) {
        for x in (m..w.saturating_sub(m)).step_by(2) {
            let (l, a) = luminance(scene, y * w + x);
            if a >= SKY && l < 0.9 && best.is_none_or(|(b, _)| l > b) {
                best = Some((l, [x as f32, y as f32]));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// The regions this scene has (a sky-only frame has no foreground ones).
pub fn choose(scene: &Scene) -> Vec<Region> {
    let all = blocks(scene);
    let sky: Vec<&Block> = all.iter().filter(|b| b.alpha >= SKY).collect();
    let ground: Vec<&Block> = all.iter().filter(|b| b.alpha <= GROUND).collect();
    let by = |v: &[&Block], key: fn(&Block) -> f32, most: bool| -> Option<[f32; 2]> {
        let pick = v.iter().max_by(|a, b| {
            let (ka, kb) = (key(a), key(b));
            if most {
                ka.total_cmp(&kb)
            } else {
                kb.total_cmp(&ka)
            }
        });
        pick.map(|b| b.centre)
    };
    let typical_ground = {
        let mut g = ground.clone();
        g.sort_by(|a, b| a.median.total_cmp(&b.median));
        g.get(g.len() / 2).map(|b| b.centre)
    };
    let edge = all
        .iter()
        .max_by(|a, b| (a.edge * a.median).total_cmp(&(b.edge * b.median)))
        .filter(|b| b.edge > 0.0)
        .map(|b| b.centre);
    [
        (
            "Darkest sky",
            "noise and how black the sky is",
            by(&sky, |b| b.median, false),
        ),
        (
            "Brightest star",
            "star colour, size and trails",
            brightest_star(scene),
        ),
        (
            "Brightest sky",
            "aurora or Milky Way structure and colour",
            by(&sky, |b| b.median, true),
        ),
        (
            "Horizon edge",
            "halos and seams where sky meets ground",
            edge,
        ),
        (
            "Typical foreground",
            "foreground brightness and noise",
            typical_ground,
        ),
        (
            "Brightest foreground",
            "lit trees, light painting, highlights",
            by(&ground, |b| b.median, true),
        ),
    ]
    .into_iter()
    .filter_map(|(name, look_for, centre)| {
        centre.map(|centre| Region {
            name,
            look_for,
            centre,
        })
    })
    .collect()
}
