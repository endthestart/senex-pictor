// A look applied to the linear scene on the GPU: the review renderer's `finish` for the sky
// (black removed, saturation, asinh stretch, shadow curve) blended in display values with the
// as-shot foreground by the sky weight, stored as linear display light with the sky weight in
// alpha (for the sky and ground overlay).

struct Look {
    black: vec4<f32>,
    scale: f32,
    stretch: f32,
    saturation: f32,
    shadows: f32,
    white: f32,
    remove_glow: f32,
    norm: f32,
    top: f32,
};

@group(0) @binding(0) var<uniform> look: Look;
@group(0) @binding(1) var base: texture_2d<f32>;
@group(0) @binding(2) var glow: texture_2d<f32>;
@group(0) @binding(3) var smp: sampler;
@group(0) @binding(4) var out: texture_storage_2d<rgba16float, write>;

const WEIGHTS = vec3<f32>(0.2126, 0.7152, 0.0722);

// render::encode_to: the sRGB curve, extended past 1 up to `top` times paper white.
fn encode(v: f32, top: f32) -> f32 {
    let a = clamp(v, 0.0, top);
    return select(1.055 * pow(a, 1.0 / 2.4) - 0.055, 12.92 * a, a <= 0.0031308);
}

fn decode(v: f32) -> f32 {
    return select(pow((v + 0.055) / 1.055, 2.4), v / 12.92, v <= 0.04045);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(out);
    if (id.x >= size.x || id.y >= size.y) {
        return;
    }
    let uv = (vec2<f32>(id.xy) + 0.5) / vec2<f32>(size);
    let s = textureSampleLevel(base, smp, uv, 0.0);
    let c = s.rgb - look.remove_glow * textureSampleLevel(glow, smp, uv, 0.0).rgb;
    let d = c - look.black.rgb;
    let l = dot(d, WEIGHTS);
    var shown: vec3<f32>;
    for (var k = 0; k < 3; k++) {
        let sv = max(l + look.saturation * (d[k] - l), d[k] / look.saturation);  // render::saturate
        let y = asinh(look.stretch * max(sv / look.scale, 0.0)) / look.norm;
        let sky = encode(pow(y, look.shadows), look.top);
        let ground = encode(c[k] / look.white, 1.0);
        shown[k] = decode(s.a * sky + (1.0 - s.a) * ground);
    }
    textureStore(out, id.xy, vec4<f32>(shown, s.a));
}
