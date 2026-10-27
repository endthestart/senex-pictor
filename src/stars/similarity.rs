//! 2-D similarity transforms (rotation, uniform scale, translation): least-squares estimate,
//! RANSAC over point pairs, and a nearest-neighbour grid for point sets.

pub type Point = [f64; 2];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Similarity {
    /// x' = a·x − b·y + tx, y' = b·x + a·y + ty.
    pub a: f64,
    pub b: f64,
    pub t: Point,
}

impl Similarity {
    pub fn apply(&self, p: Point) -> Point {
        [
            self.a * p[0] - self.b * p[1] + self.t[0],
            self.b * p[0] + self.a * p[1] + self.t[1],
        ]
    }

    pub fn residual(&self, from: Point, to: Point) -> f64 {
        let q = self.apply(from);
        (q[0] - to[0]).hypot(q[1] - to[1])
    }

    /// Least-squares similarity taking `from` onto `to` (Umeyama), or None when degenerate.
    pub fn estimate(from: &[Point], to: &[Point]) -> Option<Similarity> {
        let n = from.len() as f64;
        if from.len() != to.len() || from.len() < 2 {
            return None;
        }
        let mean = |v: &[Point]| {
            let s = v.iter().fold([0.0, 0.0], |s, p| [s[0] + p[0], s[1] + p[1]]);
            [s[0] / n, s[1] / n]
        };
        let (ms, md) = (mean(from), mean(to));
        let (mut dot, mut cross, mut var) = (0.0, 0.0, 0.0);
        for (p, q) in from.iter().zip(to) {
            let (px, py) = (p[0] - ms[0], p[1] - ms[1]);
            let (qx, qy) = (q[0] - md[0], q[1] - md[1]);
            dot += px * qx + py * qy;
            cross += px * qy - py * qx;
            var += px * px + py * py;
        }
        if var <= 0.0 {
            return None;
        }
        let (a, b) = (dot / var, cross / var);
        Some(Similarity {
            a,
            b,
            t: [
                md[0] - (a * ms[0] - b * ms[1]),
                md[1] - (b * ms[0] + a * ms[1]),
            ],
        })
    }
}

/// Deterministic generator (SplitMix64) for RANSAC sampling.
pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Random {
        Random(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in 0..n (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i + 1));
        }
    }

    /// `k` distinct indices below `n` (k ≤ n).
    pub fn choose(&mut self, n: usize, k: usize) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::with_capacity(k);
        while out.len() < k {
            let i = self.below(n);
            if !out.contains(&i) {
                out.push(i);
            }
        }
        out
    }
}

/// RANSAC as in scikit-image: random minimal samples of three pairs, the model with most
/// inliers (then the smallest residual sum of squares) wins, trials stop early once that
/// many inliers make another win unlikely. Returns the final least-squares model on the
/// winner's inliers and the winner's inlier mask.
pub fn ransac(
    from: &[Point],
    to: &[Point],
    threshold: f64,
    max_trials: usize,
    random: &mut Random,
) -> Option<(Similarity, Vec<bool>)> {
    const SAMPLE: usize = 3;
    let n = from.len();
    if n < SAMPLE || to.len() != n {
        return None;
    }
    let mut best: Option<(usize, f64, Vec<bool>)> = None;
    let mut trials = max_trials;
    let mut done = 0;
    while done < trials {
        done += 1;
        let pick = random.choose(n, SAMPLE);
        let sample = |v: &[Point]| pick.iter().map(|&i| v[i]).collect::<Vec<_>>();
        let Some(model) = Similarity::estimate(&sample(from), &sample(to)) else {
            continue;
        };
        let residuals: Vec<f64> = from
            .iter()
            .zip(to)
            .map(|(p, q)| model.residual(*p, *q))
            .collect();
        let inliers: Vec<bool> = residuals.iter().map(|r| *r < threshold).collect();
        let count = inliers.iter().filter(|v| **v).count();
        let sum: f64 = residuals.iter().map(|r| r * r).sum();
        let better = best
            .as_ref()
            .is_none_or(|(c, s, _)| count > *c || (count == *c && sum < *s));
        if better && count > 0 {
            let ratio = count as f64 / n as f64;
            let needed =
                f64::EPSILON.ln() / (1.0 - ratio.powi(SAMPLE as i32)).max(f64::EPSILON).ln();
            trials = trials.min(needed.ceil() as usize);
            best = Some((count, sum, inliers));
        }
    }
    let (_, _, inliers) = best?;
    let keep = |v: &[Point]| {
        v.iter()
            .zip(&inliers)
            .filter(|(_, k)| **k)
            .map(|(p, _)| *p)
            .collect::<Vec<_>>()
    };
    Some((Similarity::estimate(&keep(from), &keep(to))?, inliers))
}

/// Points binned into square cells for nearest-neighbour queries within one cell size.
pub struct Grid<'a> {
    points: &'a [Point],
    origin: Point,
    cell: f64,
    columns: usize,
    rows: usize,
    starts: Vec<usize>,
    members: Vec<usize>,
}

impl<'a> Grid<'a> {
    pub fn new(points: &'a [Point], cell: f64) -> Grid<'a> {
        let finite = points
            .iter()
            .filter(|p| p[0].is_finite() && p[1].is_finite());
        let (lo, hi) = finite.fold(
            ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]),
            |(lo, hi), p| {
                (
                    [lo[0].min(p[0]), lo[1].min(p[1])],
                    [hi[0].max(p[0]), hi[1].max(p[1])],
                )
            },
        );
        let origin = if lo[0].is_finite() { lo } else { [0.0, 0.0] };
        let span = |k: usize| ((hi[k] - origin[k]).max(0.0) / cell) as usize + 1;
        let (columns, rows) = (span(0), span(1));
        let mut grid = Grid {
            points,
            origin,
            cell,
            columns,
            rows,
            starts: vec![0; columns * rows + 1],
            members: Vec::with_capacity(points.len()),
        };
        let cells: Vec<Option<usize>> = points.iter().map(|p| grid.cell_of(*p)).collect();
        for c in cells.iter().flatten() {
            grid.starts[c + 1] += 1;
        }
        for i in 0..columns * rows {
            grid.starts[i + 1] += grid.starts[i];
        }
        let mut fill = grid.starts.clone();
        grid.members = vec![0; grid.starts[columns * rows]];
        for (i, c) in cells.iter().enumerate() {
            if let Some(c) = c {
                grid.members[fill[*c]] = i;
                fill[*c] += 1;
            }
        }
        grid
    }

    fn cell_of(&self, p: Point) -> Option<usize> {
        let (i, j) = self.coordinates(p)?;
        (i >= 0 && j >= 0 && (i as usize) < self.columns && (j as usize) < self.rows)
            .then(|| j as usize * self.columns + i as usize)
    }

    fn coordinates(&self, p: Point) -> Option<(isize, isize)> {
        (p[0].is_finite() && p[1].is_finite()).then(|| {
            (
                ((p[0] - self.origin[0]) / self.cell).floor() as isize,
                ((p[1] - self.origin[1]) / self.cell).floor() as isize,
            )
        })
    }

    /// Index and distance of the nearest point closer than `radius` (at most the cell size).
    pub fn nearest(&self, q: Point, radius: f64) -> Option<(usize, f64)> {
        let (ci, cj) = self.coordinates(q)?;
        let mut best: Option<(usize, f64)> = None;
        for j in cj - 1..=cj + 1 {
            for i in ci - 1..=ci + 1 {
                if i < 0 || j < 0 || i as usize >= self.columns || j as usize >= self.rows {
                    continue;
                }
                let c = j as usize * self.columns + i as usize;
                for &m in &self.members[self.starts[c]..self.starts[c + 1]] {
                    let p = self.points[m];
                    let d = (p[0] - q[0]).hypot(p[1] - q[1]);
                    if d < radius && best.is_none_or(|(_, b)| d < b) {
                        best = Some((m, d));
                    }
                }
            }
        }
        best
    }
}
