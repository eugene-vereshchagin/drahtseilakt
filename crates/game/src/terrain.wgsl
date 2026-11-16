// The land under a level (terrain.rs): its colour map (lit by the sky's sun when it was made), a finer grain near
// the eye (the colour map's texel is 2.4 units; the camera flies a few hundred over it), and the air between: the
// land fades by distance into the sky's haze just under its horizon (the ring), cooler and bluer looking down.

struct U {
    view_proj: mat4x4<f32>,
    // xyz the eye, w the far plane
    eye: vec4<f32>,
    // towards the sun
    sun: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var color: texture_2d<f32>;
@group(0) @binding(2) var ring: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) relight: f32,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) relight: f32) -> VsOut {
    var o: VsOut;
    var c = u.view_proj * vec4<f32>(pos, 1.0);
    // past the far plane the land is kept (at the back of the depth range) rather than cut: its far edge is wholly
    // in the haze, and the sky box behind shows only haze below its horizon
    c.z = min(c.z, c.w * 0.999999);
    o.pos = c;
    o.world = pos;
    o.uv = uv;
    o.relight = relight;
    return o;
}

fn hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1030));
    let r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    var c = textureSample(color, samp, in.uv).rgb;
    // the light where the land was lowered under the track (in linear light)
    c = pow(pow(c, vec3<f32>(2.2)) * in.relight, vec3<f32>(1.0 / 2.2));
    let ray = in.world - u.eye.xyz;
    let dist = length(ray);
    // the grain: rocks, tree crowns and tufts from a few dozen units down to a couple, lit from the sun's side (each
    // octave's slope towards the sun), stronger on the green; an octave fades where a pixel spans more than it does
    let fw = length(fwidth(in.world.xz));
    let p = in.world.xz;
    let s = normalize(u.sun.xz + vec2<f32>(1e-4, 0.0));
    let green = clamp((c.g - max(c.r, c.b)) * 6.0, 0.0, 1.0);
    var grain = 0.0;
    var size = 26.0;
    for (var o = 0; o < 4; o++) {
        let q = p / size + vec2<f32>(f32(o) * 17.3, f32(o) * 5.1);
        let n0 = vnoise(q);
        let lit = vnoise(q + s * 0.35) - n0;
        let keep = clamp(2.0 - 2.0 * fw / size * 2.0, 0.0, 1.0);
        grain += keep * ((n0 - 0.5) * 0.35 + lit * 1.1);
        size = size * 0.42;
    }
    // water (bluer than it is green) keeps still, and mirrors the haze over the horizon more as the view grazes it
    let water = clamp((c.b - c.g) * 10.0, 0.0, 1.0);
    c = c * clamp(1.0 + grain * (0.45 + 0.35 * green) * (1.0 - water), 0.5, 1.6);
    // the air: thicker with distance, wholly the haze towards the far plane and beyond
    let d = dist / 1700.0;
    var f = 1.0 - exp(-pow(d, 1.35));
    f = max(f, smoothstep(0.7 * u.eye.w, 1.6 * u.eye.w, dist));
    let dir = ray / max(dist, 1e-3);
    let ru = atan2(dir.x, dir.z) / 6.2831853 + 0.5;
    let horizon = textureSampleLevel(ring, samp, vec2<f32>(ru, 0.5), 0.0).rgb;
    // looking down the air between is a cool blue-white of the haze's brightness (as the sky faces drew it)
    let lum = dot(horizon, vec3<f32>(0.2126, 0.7152, 0.0722));
    let cool = lum * vec3<f32>(0.80, 0.90, 1.10) / 0.8975;
    let down = smoothstep(0.02, 0.3, -dir.y);
    let haze = mix(horizon, cool, 0.6 * down);
    let fres = 0.02 + 0.98 * pow(1.0 - clamp(-dir.y, 0.0, 1.0), 5.0);
    c = mix(c, horizon * 0.85, water * fres * 0.5);
    return vec4<f32>(mix(c, haze, f), 1.0);
}
