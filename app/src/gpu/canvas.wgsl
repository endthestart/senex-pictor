// The canvas: one quad for the visible image in the callback's normalized device coordinates,
// sampled from two GPU-resident renderings. `swap` picks which one is shown; framebuffer pixels
// right of `split` show the other, so a split view and an instant swap need no extra pass.
// Linear display light goes out as is to float and sRGB targets, encoded for 8-bit targets
// that egui treats as gamma space, and scaled to paper white.

struct View {
    lo: vec2<f32>,
    hi: vec2<f32>,
    scale: f32,
    mode: u32,
    swap: u32,
    split: f32,
};

@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var first: texture_2d<f32>;
@group(1) @binding(1) var second: texture_2d<f32>;
@group(1) @binding(2) var smp: sampler;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

const ENCODE: u32 = 1u;

fn encode(v: f32) -> f32 {
    let a = clamp(v, 0.0, 1.0);
    return select(1.055 * pow(a, 1.0 / 2.4) - 0.055, 12.92 * a, a <= 0.0031308);
}

@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    let corner = vec2<f32>(f32(i & 1u), f32((i >> 1u) & 1u));
    var o: Out;
    o.pos = vec4<f32>(mix(view.lo, view.hi, corner), 0.0, 1.0);
    o.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    return o;
}

@fragment
fn fs(in: Out) -> @location(0) vec4<f32> {
    let a = textureSample(first, smp, in.uv).rgb;
    let b = textureSample(second, smp, in.uv).rgb;
    let other = in.pos.x >= view.split;
    var c = select(a, b, (view.swap == 1u) != other) * view.scale;
    if (view.mode & ENCODE) != 0u {
        c = vec3<f32>(encode(c.r), encode(c.g), encode(c.b));
    }
    return vec4<f32>(c, 1.0);
}
