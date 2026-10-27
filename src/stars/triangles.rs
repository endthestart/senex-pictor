//! Asterism (triangle) matching between two star lists, ported from astroalign 2.6.2
//! (MIT License, © 2016 Martin Beroiz): triangles from each star's nearest neighbours are
//! described by their side ratios, similar triangles are paired, and a RANSAC over those
//! pairs finds the similarity transform that most of them agree on.

use super::similarity::{Point, Random, Similarity};

/// Stars per list used as control points.
pub const CONTROL_POINTS: usize = 50;
const NEIGHBOURS: usize = 5;
const INVARIANT_RADIUS: f64 = 0.1;
const PIXEL_TOLERANCE: f64 = 2.0;
const MIN_MATCHES_FRACTION: f64 = 0.8;
const MAX_MIN_MATCHES: usize = 10;
const REFINEMENTS: usize = 3;

/// Vertices ordered (shared by the two shortest sides, the middle and longest, the longest
/// and shortest) and the invariants (longest/middle, middle/shortest).
struct Triangle {
    vertices: [usize; 3],
    invariant: [f64; 2],
}

fn triangles(points: &[Point]) -> Vec<Triangle> {
    let dist =
        |a: usize, b: usize| (points[a][0] - points[b][0]).hypot(points[a][1] - points[b][1]);
    let k = NEIGHBOURS.min(points.len());
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for i in 0..points.len() {
        let mut near: Vec<usize> = (0..points.len()).collect();
        near.sort_by(|&a, &b| dist(i, a).total_cmp(&dist(i, b)).then(a.cmp(&b)));
        near.truncate(k);
        for a in 0..k {
            for b in a + 1..k {
                for c in b + 1..k {
                    let v = [near[a], near[b], near[c]];
                    let mut key = v;
                    key.sort_unstable();
                    if seen.insert(key) {
                        out.push(arrange(v, &dist));
                    }
                }
            }
        }
    }
    out
}

fn arrange(v: [usize; 3], dist: &impl Fn(usize, usize) -> f64) -> Triangle {
    let sides = [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])];
    let mut order = [0, 1, 2];
    let length = |s: (usize, usize)| dist(s.0, s.1);
    order.sort_by(|&a, &b| length(sides[a]).total_cmp(&length(sides[b])));
    let shared = |p: (usize, usize), q: (usize, usize)| {
        if p.0 == q.0 || p.0 == q.1 { p.0 } else { p.1 }
    };
    let [l1, l2, l3] = order.map(|o| sides[o]);
    let (s1, s2, s3) = (length(l1), length(l2), length(l3));
    Triangle {
        vertices: [shared(l1, l2), shared(l2, l3), shared(l3, l1)],
        invariant: [s3 / s2, s2 / s1],
    }
}

/// The similarity taking `source` stars onto `target` stars (both brightest first), or None
/// when no consistent set of similar triangles is found.
pub fn find(source: &[Point], target: &[Point], random: &mut Random) -> Option<Similarity> {
    let source = &source[..source.len().min(CONTROL_POINTS)];
    let target = &target[..target.len().min(CONTROL_POINTS)];
    if source.len() < 3 || target.len() < 3 {
        return None;
    }
    let (ts, tt) = (triangles(source), triangles(target));
    let mut matches: Vec<[(usize, usize); 3]> = Vec::new();
    for a in &ts {
        for b in &tt {
            let d = (a.invariant[0] - b.invariant[0]).hypot(a.invariant[1] - b.invariant[1]);
            if d <= INVARIANT_RADIUS {
                matches.push(std::array::from_fn(|k| (a.vertices[k], b.vertices[k])));
            }
        }
    }
    if matches.is_empty() {
        return None;
    }
    let fit = |set: &[&[(usize, usize); 3]]| {
        let pairs = set.iter().flat_map(|m| m.iter());
        let (from, to): (Vec<Point>, Vec<Point>) =
            pairs.map(|&(s, t)| (source[s], target[t])).unzip();
        Similarity::estimate(&from, &to)
    };
    let error = |m: &[(usize, usize); 3], t: &Similarity| {
        m.iter()
            .map(|&(s, d)| t.residual(source[s], target[d]))
            .fold(0.0, f64::max)
    };
    if (source.len() == 3 || target.len() == 3) && matches.len() == 1 {
        return fit(&[&matches[0]]);
    }
    let min_matches =
        ((matches.len() as f64 * MIN_MATCHES_FRACTION) as usize).clamp(1, MAX_MIN_MATCHES);
    let mut order: Vec<usize> = (0..matches.len()).collect();
    random.shuffle(&mut order);
    let mut good = None;
    for &i in &order {
        let Some(maybe) = fit(&[&matches[i]]) else {
            continue;
        };
        let also: Vec<&[(usize, usize); 3]> = matches
            .iter()
            .enumerate()
            .filter(|&(j, m)| j != i && error(m, &maybe) < PIXEL_TOLERANCE)
            .map(|(_, m)| m)
            .collect();
        if also.len() >= min_matches {
            let mut set = vec![&matches[i]];
            set.extend(also);
            good = fit(&set);
            break;
        }
    }
    let mut better = good?;
    for _ in 0..REFINEMENTS {
        let inliers: Vec<&[(usize, usize); 3]> = matches
            .iter()
            .filter(|m| error(m, &better) < PIXEL_TOLERANCE)
            .collect();
        better = fit(&inliers)?;
    }
    Some(better)
}
