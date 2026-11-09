// The scaling pass of the "Back to 2004" look: the small picture drawn nearest-neighbour onto the window (the
// viewport is the picture's place; the black bars are the pass's clear), its colour cut to 5-6-5 bits if asked.

struct Params {
    bits16: f32,
};

@group(0) @binding(0) var pic: texture_2d<f32>;
@group(0) @binding(1) var near: sampler;
@group(0) @binding(2) var<uniform> params: Params;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    // one triangle covering the viewport
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(p * 2.0 - vec2<f32>(1.0), 0.0, 1.0);
    out.uv = vec2<f32>(p.x, 1.0 - p.y);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    var c = textureSample(pic, near, in.uv);
    if params.bits16 > 0.5 {
        // 5 bits red and blue, 6 green, rounded, no dithering: the bands are the point
        c = vec4<f32>(
            floor(c.r * 31.0 + 0.5) / 31.0,
            floor(c.g * 63.0 + 0.5) / 63.0,
            floor(c.b * 31.0 + 0.5) / 31.0,
            1.0,
        );
    }
    return vec4<f32>(c.rgb, 1.0);
}
