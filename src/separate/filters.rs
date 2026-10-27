//! Full-resolution image operations with scipy.ndimage semantics (the prototype's oracle):
//! box and median filters (reflected edges), binary morphology with the 3×3 cross (outside
//! the image counts as background), exact Euclidean distance transform with nearest indices
//! and 4-connected components.

/// scipy's reflect mode: d c b a | a b c d | d c b a.
fn reflect(i: isize, n: usize) -> usize {
    let n = n as isize;
    let mut i = i;
    loop {
        if i < 0 {
            i = -i - 1;
        } else if i >= n {
            i = 2 * n - i - 1;
        } else {
            return i as usize;
        }
    }
}

/// Mean over a size×size box.
pub fn uniform(image: &[f32], width: usize, height: usize, size: usize) -> Vec<f32> {
    let r = (size / 2) as isize;
    let pass = |src: &[f64], along_x: bool| -> Vec<f64> {
        (0..width * height)
            .map(|p| {
                let (x, y) = ((p % width) as isize, (p / width) as isize);
                (-r..=r)
                    .map(|d| {
                        if along_x {
                            src[y as usize * width + reflect(x + d, width)]
                        } else {
                            src[reflect(y + d, height) * width + x as usize]
                        }
                    })
                    .sum::<f64>()
                    / size as f64
            })
            .collect()
    };
    let wide: Vec<f64> = image.iter().map(|&v| f64::from(v)).collect();
    pass(&pass(&wide, true), false)
        .into_iter()
        .map(|v| v as f32)
        .collect()
}

/// Median (rank size²/2) over a size×size box spanning −size/2 ..= size − 1 − size/2.
pub fn median(image: &[f32], width: usize, height: usize, size: usize) -> Vec<f32> {
    let (lo, hi) = (-((size / 2) as isize), (size - 1 - size / 2) as isize);
    let mut window = vec![0f32; size * size];
    (0..width * height)
        .map(|p| {
            let (x, y) = ((p % width) as isize, (p / width) as isize);
            let mut k = 0;
            for dy in lo..=hi {
                for dx in lo..=hi {
                    window[k] = image[reflect(y + dy, height) * width + reflect(x + dx, width)];
                    k += 1;
                }
            }
            *window.select_nth_unstable_by(k / 2, f32::total_cmp).1
        })
        .collect()
}

fn cross(mask: &[bool], width: usize, height: usize, grow: bool) -> Vec<bool> {
    (0..width * height)
        .map(|p| {
            let (x, y) = (p % width, p / width);
            let near = [
                (x > 0).then(|| p - 1),
                (x + 1 < width).then(|| p + 1),
                (y > 0).then(|| p - width),
                (y + 1 < height).then(|| p + width),
            ];
            if grow {
                mask[p] || near.iter().any(|q| q.is_some_and(|q| mask[q]))
            } else {
                mask[p] && near.iter().all(|q| q.is_some_and(|q| mask[q]))
            }
        })
        .collect()
}

pub fn erode(mask: &[bool], width: usize, height: usize, iterations: usize) -> Vec<bool> {
    (0..iterations).fold(mask.to_vec(), |m, _| cross(&m, width, height, false))
}

pub fn dilate(mask: &[bool], width: usize, height: usize, iterations: usize) -> Vec<bool> {
    (0..iterations).fold(mask.to_vec(), |m, _| cross(&m, width, height, true))
}

/// Opening with the cross: erosion then dilation.
pub fn open(mask: &[bool], width: usize, height: usize) -> Vec<bool> {
    dilate(&erode(mask, width, height, 1), width, height, 1)
}

/// One dimension of the exact squared Euclidean distance transform (Felzenszwalb and
/// Huttenlocher), carrying the index of the nearest source.
fn lower_envelope(f: &[f64], from: &[usize], out: &mut [f64], at: &mut [usize]) {
    let n = f.len();
    let (mut v, mut z) = (vec![0usize; n], vec![0f64; n + 1]);
    let mut k = 0;
    let first = (0..n).find(|&q| f[q].is_finite());
    let Some(first) = first else {
        out.iter_mut().for_each(|o| *o = f64::INFINITY);
        return;
    };
    v[0] = first;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in first + 1..n {
        if !f[q].is_finite() {
            continue;
        }
        loop {
            let p = v[k];
            let s =
                ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * (q as f64 - p as f64));
            if s <= z[k] && k > 0 {
                k -= 1;
                continue;
            }
            if s <= z[k] {
                v[0] = q;
                z[1] = f64::INFINITY;
                break;
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
            break;
        }
    }
    let mut k = 0;
    for q in 0..n {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let d = q as f64 - v[k] as f64;
        out[q] = d * d + f[v[k]];
        at[q] = from[v[k]];
    }
}

/// Squared distance to, and index of, the nearest `known` pixel (None when none is known).
pub fn distance_squared(
    known: &[bool],
    width: usize,
    height: usize,
) -> Option<(Vec<f64>, Vec<usize>)> {
    if !known.iter().any(|&k| k) {
        return None;
    }
    let mut dist = vec![f64::INFINITY; width * height];
    let mut index = vec![0usize; width * height];
    let (mut col, mut from, mut out, mut at) = (
        vec![0.0; height],
        vec![0; height],
        vec![0.0; height],
        vec![0; height],
    );
    for x in 0..width {
        for y in 0..height {
            let p = y * width + x;
            col[y] = if known[p] { 0.0 } else { f64::INFINITY };
            from[y] = p;
        }
        lower_envelope(&col, &from, &mut out, &mut at);
        for y in 0..height {
            dist[y * width + x] = out[y];
            index[y * width + x] = at[y];
        }
    }
    let (mut out, mut at) = (vec![0.0; width], vec![0; width]);
    for y in 0..height {
        let row = y * width..(y + 1) * width;
        lower_envelope(&dist[row.clone()], &index[row.clone()], &mut out, &mut at);
        dist[row.clone()].copy_from_slice(&out);
        index[row].copy_from_slice(&at);
    }
    Some((dist, index))
}

/// Index of the nearest `known` pixel for every pixel.
pub fn nearest_known(known: &[bool], width: usize, height: usize) -> Option<Vec<usize>> {
    distance_squared(known, width, height).map(|(_, i)| i)
}

/// Sizes of the 4-connected components of `mask` and each pixel's component (usize::MAX
/// outside the mask).
pub fn components(mask: &[bool], width: usize, height: usize) -> (Vec<usize>, Vec<usize>) {
    let mut label = vec![usize::MAX; mask.len()];
    let mut sizes = Vec::new();
    let mut queue = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || label[start] != usize::MAX {
            continue;
        }
        let id = sizes.len();
        label[start] = id;
        queue.clear();
        queue.push(start);
        let mut next = 0;
        while next < queue.len() {
            let p = queue[next];
            next += 1;
            let (x, y) = (p % width, p / width);
            let near = [
                (x > 0).then(|| p - 1),
                (x + 1 < width).then(|| p + 1),
                (y > 0).then(|| p - width),
                (y + 1 < height).then(|| p + width),
            ];
            for q in near.into_iter().flatten() {
                if mask[q] && label[q] == usize::MAX {
                    label[q] = id;
                    queue.push(q);
                }
            }
        }
        sizes.push(queue.len());
    }
    (sizes, label)
}
