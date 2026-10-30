// CFA (Bayer-drizzle) sky stack on the GPU: the twin of `integrate::cfa` for one band of
// output rows. One invocation per output pixel and colour gathers that colour's nearest CFA
// samples from every frame, clips them about the median of the near samples and writes the
// weighted mean, the even- and odd-frame means and the kept count (+0.5 where at least
// CLIPPED_SHARE of this colour's samples were saturated: a clipped core).

const MAX_SAMPLES: u32 = 640u;
const CENTRE_WEIGHT: f32 = 0.3;
const CLIP_K: f32 = 4.0;
const CLIP_ROUNDS: u32 = 2u;
const CLIPPED_SHARE: f32 = 0.1;  // integrate::cfa::CLIPPED_SHARE
const UNDISTORT_ROUNDS: u32 = 30u;
// Written where a mean has no weight (WGSL gives no portable NaN).
const NONE: f32 = -1e30;

struct Params {
    width: u32, height: u32, band_y0: u32, band_rows: u32,
    frames: u32, reference: u32, grid_columns: u32, grid_rows: u32,
    colours: vec4<u32>,
    f: f32, k1: f32, k2: f32, scale: f32,
    cx: f32, cy: f32, sigma: f32, radius: f32,
    clearance_px: f32, clear: f32, block: f32, words_per_row: u32,
}

struct Frame {
    r0: vec4<f32>, r1: vec4<f32>, r2: vec4<f32>,
    black: vec4<f32>,
    white: f32, cfa_offset: u32, occ_offset: u32, y0: u32,
    rows: u32, used: u32, pad0: u32, pad1: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> frames: array<Frame>;
@group(0) @binding(2) var<storage, read> cfa: array<u32>;
@group(0) @binding(3) var<storage, read> occ: array<u32>;
@group(0) @binding(4) var<storage, read> alpha: array<f32>;
@group(0) @binding(5) var<storage, read> clearance: array<f32>;
@group(0) @binding(6) var<storage, read> flat: array<f32>;
@group(0) @binding(7) var<storage, read> glow: array<vec4<f32>>;
@group(0) @binding(8) var<storage, read_write> out: array<f32>;

var<private> values: array<f32, MAX_SAMPLES>;
var<private> weights: array<f32, MAX_SAMPLES>;
var<private> scratch: array<f32, MAX_SAMPLES>;
var<private> flags: array<u32, MAX_SAMPLES>;  // bit 0 kept, bit 1 odd frame

fn distort(u: vec2<f32>) -> vec2<f32> {
    let c = vec2<f32>(params.cx, params.cy);
    let d = u - c;
    let r2 = dot(d, d) / (params.scale * params.scale);
    return c + d * (1.0 + params.k1 * r2 + params.k2 * r2 * r2);
}

fn ray(p: vec2<f32>) -> vec3<f32> {
    var u = p;
    for (var i = 0u; i < UNDISTORT_ROUNDS; i++) {
        u = u + (p - distort(u));
    }
    let c = vec2<f32>(params.cx, params.cy);
    return normalize(vec3<f32>((u - c) / params.f, 1.0));
}

// Bilinear glow of `frame` at full-resolution `p`; w = 1 when known (as `Grid::sample`).
fn glow_at(frame: u32, p: vec2<f32>) -> vec4<f32> {
    let b = params.block;
    let cols = f32(params.grid_columns);
    let rows = f32(params.grid_rows);
    var gx = (p.x - (b - 1.0) / 2.0) / b;
    var gy = (p.y - (b - 1.0) / 2.0) / b;
    if (!(gx >= -1.0 && gy >= -1.0 && gx <= cols && gy <= rows)) {
        return vec4<f32>(0.0);
    }
    gx = clamp(gx, 0.0, cols - 1.0);
    gy = clamp(gy, 0.0, rows - 1.0);
    let x0 = u32(floor(gx));
    let y0 = u32(floor(gy));
    let x1 = min(x0 + 1u, params.grid_columns - 1u);
    let y1 = min(y0 + 1u, params.grid_rows - 1u);
    let fx = gx - f32(x0);
    let fy = gy - f32(y0);
    let base = frame * params.grid_columns * params.grid_rows;
    let c00 = glow[base + y0 * params.grid_columns + x0];
    let c10 = glow[base + y0 * params.grid_columns + x1];
    let c01 = glow[base + y1 * params.grid_columns + x0];
    let c11 = glow[base + y1 * params.grid_columns + x1];
    let w00 = (1.0 - fx) * (1.0 - fy) * step(0.5, c00.w);
    let w10 = fx * (1.0 - fy) * step(0.5, c10.w);
    let w01 = (1.0 - fx) * fy * step(0.5, c01.w);
    let w11 = fx * fy * step(0.5, c11.w);
    let norm = w00 + w10 + w01 + w11;
    let value = w00 * c00.xyz + w10 * c10.xyz + w01 * c01.xyz + w11 * c11.xyz;
    if (norm <= 0.5) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(value / norm, 1.0);
}

// k-th smallest of scratch[0..n) (Hoare quickselect, in place).
fn select_kth(n: u32, k: u32) -> f32 {
    var lo = 0i;
    var hi = i32(n) - 1;
    let want = i32(k);
    loop {
        if (lo >= hi) {
            return scratch[want];
        }
        let pivot = scratch[(lo + hi) / 2];
        var i = lo;
        var j = hi;
        loop {
            while (scratch[i] < pivot) { i++; }
            while (scratch[j] > pivot) { j--; }
            if (i <= j) {
                let t = scratch[i];
                scratch[i] = scratch[j];
                scratch[j] = t;
                i++;
                j--;
            }
            if (i > j) { break; }
        }
        if (want <= j) {
            hi = j;
        } else if (want >= i) {
            lo = i;
        } else {
            return scratch[want];
        }
    }
    return scratch[want];
}

// Median of scratch[0..n): the mean of the two middle values for an even count.
fn median(n: u32) -> f32 {
    let hi = select_kth(n, n / 2u);
    if (n % 2u == 1u) {
        return hi;
    }
    var lo = scratch[0];
    for (var i = 1u; i < n / 2u; i++) {
        lo = max(lo, scratch[i]);
    }
    return (lo + hi) / 2.0;
}

fn cfa_value(index: u32) -> u32 {
    let word = cfa[index >> 1u];
    return select(word & 0xffffu, word >> 16u, (index & 1u) == 1u);
}

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let w = params.width;
    let pixel = id.x;
    let colour = id.y;
    if (pixel >= params.band_rows * w) {
        return;
    }
    let slot = (pixel * 3u + colour) * 4u;
    out[slot] = NONE;
    out[slot + 1u] = NONE;
    out[slot + 2u] = NONE;
    out[slot + 3u] = 0.0;
    let qx = pixel % w;
    let qy = params.band_y0 + pixel / w;
    let q = qy * w + qx;
    if (alpha[q] <= 0.0) {
        return;
    }
    let point = vec2<f32>(f32(qx), f32(qy));
    let v = ray(point);
    let reference = glow_at(params.reference, point);
    var count = 0u;
    var saturated = 0u;
    var overflow = false;
    for (var i = 0u; i < params.frames; i++) {
        let fr = frames[i];
        if (fr.used == 0u) {
            continue;
        }
        let r = vec3<f32>(dot(fr.r0.xyz, v), dot(fr.r1.xyz, v), dot(fr.r2.xyz, v));
        if (r.z <= 0.0) {
            continue;
        }
        let c = vec2<f32>(params.cx, params.cy);
        let p = distort(c + params.f * r.xy / r.z);
        let g = glow_at(i, p);
        for (var site = 0u; site < 4u; site++) {
            if (params.colours[site] != colour) {
                continue;
            }
            let dx = f32(site % 2u);
            let dy = f32(site / 2u);
            let sx = dx + 2.0 * round((p.x - dx) / 2.0);
            let sy = dy + 2.0 * round((p.y - dy) / 2.0);
            let d2 = (sx - p.x) * (sx - p.x) + (sy - p.y) * (sy - p.y);
            if (!(sx >= 0.0 && sy >= 0.0 && sx < f32(w) && sy < f32(params.height) && d2 <= params.radius * params.radius)) {
                continue;
            }
            let ix = u32(sx);
            let iy = u32(sy);
            let s = iy * w + ix;
            if (alpha[s] < params.clear || clearance[s] < params.clearance_px || iy < fr.y0 || iy >= fr.y0 + fr.rows) {
                continue;
            }
            let local = iy - fr.y0;
            let bit = (occ[fr.occ_offset + local * params.words_per_row + (ix >> 5u)] >> (ix & 31u)) & 1u;
            if (bit == 1u) {
                continue;
            }
            let raw = f32(cfa_value(fr.cfa_offset + local * w + ix));
            if (raw >= fr.white) {
                saturated++;
                continue;
            }
            if (g.w < 0.5 || reference.w < 0.5) {
                continue;
            }
            if (count >= MAX_SAMPLES) {
                overflow = true;
                continue;
            }
            values[count] = (raw - fr.black[site]) / flat[s] - (g[colour] - reference[colour]);
            weights[count] = exp(-d2 / (2.0 * params.sigma * params.sigma));
            flags[count] = 1u | ((i & 1u) << 1u);
            count++;
        }
    }
    if (overflow) {
        out[slot + 3u] = -1.0;
        return;
    }
    for (var iteration = 0u; iteration < CLIP_ROUNDS; iteration++) {
        var m = 0u;
        for (var s = 0u; s < count; s++) {
            if ((flags[s] & 1u) == 1u && weights[s] >= CENTRE_WEIGHT) {
                scratch[m] = values[s];
                m++;
            }
        }
        if (m < 3u) {
            break;
        }
        let centre = median(m);
        m = 0u;
        for (var s = 0u; s < count; s++) {
            if ((flags[s] & 1u) == 1u && weights[s] >= CENTRE_WEIGHT) {
                scratch[m] = abs(values[s] - centre);
                m++;
            }
        }
        let limit = CLIP_K * max(1.4826 * median(m), 1e-3);
        for (var s = 0u; s < count; s++) {
            if (abs(values[s] - centre) > limit) {
                flags[s] = flags[s] & ~1u;
            }
        }
    }
    var sums = array<f32, 6>(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    var kept = 0u;
    for (var s = 0u; s < count; s++) {
        if ((flags[s] & 1u) == 0u) {
            continue;
        }
        kept++;
        let wv = weights[s] * values[s];
        sums[0] += wv;
        sums[1] += weights[s];
        let part = 2u + 2u * ((flags[s] >> 1u) & 1u);
        sums[part] += wv;
        sums[part + 1u] += weights[s];
    }
    for (var k = 0u; k < 3u; k++) {
        if (sums[2u * k + 1u] > 0.0) {
            out[slot + k] = sums[2u * k] / sums[2u * k + 1u];
        }
    }
    let clipped = saturated > 0u && f32(saturated) >= CLIPPED_SHARE * f32(saturated + count);
    out[slot + 3u] = f32(kept) + select(0.0, 0.5, clipped);
}
