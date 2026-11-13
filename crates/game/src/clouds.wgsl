// The sea of cloud (clouds.rs): a full-screen pass that marches each pixel's view ray through a slab of cumulus made of
// tiling noise, up to the scene's depth, and lights it from the sun and the sky. Lit in linear light, drawn
// premultiplied over the frame (gamma, as the rest of the game). Far off the clouds fade into the sky's own colour at
// the horizon, so they meet the sky box without a seam.

struct Cloud {
    inv_view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    sky: vec4<f32>,
    shadow: vec4<f32>,
    // x bottom, y top, z cover, w size of a cloud
    slab: vec4<f32>,
    // x steps along the view, y steps to the sun, z haze density, w how much the haze under the layer hides
    march: vec4<f32>,
    // xy drift a second, z 1 with the edges' billows, w the frame's width
    wind: vec4<f32>,
    // rgb the haze's average, a the frame's height
    haze: vec4<f32>,
};

@group(0) @binding(0) var<uniform> c: Cloud;
@group(0) @binding(1) var noise: texture_3d<f32>;
@group(0) @binding(2) var weather: texture_2d<f32>;
@group(0) @binding(3) var ring: texture_2d<f32>;
@group(0) @binding(4) var depth: texture_depth_2d;
@group(0) @binding(5) var rep: sampler;

const PI: f32 = 3.14159265;
const FAR: f32 = 2900.0;
// the sky box's half side (sky.rs SIZE)
const SKY: f32 = 1200.0;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    var out: VsOut;
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    out.pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

fn to_gamma(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

fn remap(v: f32, lo: f32, hi: f32) -> f32 {
    return clamp((v - lo) / max(hi - lo, 1e-5), 0.0, 1.0);
}

// Where the clouds are: 0 clear air, about 1 the thick of a cloud. A cloud is where the billowy noise rises over a
// threshold that grows with height: low down nearly everything is cloud (the sea), towards the top only the
// strongest billows reach, so the tops are domes of different heights. `detail` adds the small billows on the edges.
fn density(p: vec3<f32>, detail: bool) -> f32 {
    let bottom = c.slab.x;
    let top = c.slab.y;
    let h = (p.y - bottom) / (top - bottom);
    if h <= 0.0 || h >= 1.0 {
        return 0.0;
    }
    let size = c.slab.w;
    // the drift, wrapped by clouds.rs at a period every texture below repeats over
    let q = p - vec3<f32>(c.wind.x, 0.0, c.wind.y);
    let w = textureSampleLevel(weather, rep, q.xz / (size * 30.0), 0.0);
    // how cloudy this place is: the level's cover, varied by the map into banks and gaps
    let local = clamp(c.slab.z + (w.r - 0.5) * 0.9, 0.0, 1.0);
    // how high its clouds reach (a fraction of the slab): by region, and heap by heap
    let heap = textureSampleLevel(weather, rep, q.xz / (size * 5.0) + vec2<f32>(0.37, 0.61), 0.0).g;
    let reach = mix(0.3, 1.0, clamp(w.g * 0.45 + heap * 0.75 - 0.1, 0.0, 1.0));
    let n = textureSampleLevel(noise, rep, q / (size * 4.0), 0.0);
    let billows = n.g * 0.625 + n.b * 0.25 + n.a * 0.125;
    let body = remap(n.r, billows - 1.0, 1.0) * 0.55 + billows * 0.45;
    let hr = h / reach;
    let threshold = mix(1.0 - local, 1.0, clamp(pow(hr, 1.3), 0.0, 1.0)) - 0.1;
    var d = (body - threshold) * 14.0 * remap(h, 0.0, 0.08);
    if detail && d > -0.2 && d < 0.6 {
        let m = textureSampleLevel(noise, rep, q / (size * 0.75), 0.0);
        let hf = m.g * 0.625 + m.b * 0.25 + m.a * 0.125;
        d -= (1.0 - hf) * 0.55;
    }
    return clamp(d, 0.0, 1.0);
}

// Henyey-Greenstein: how much light goes from the sun's direction into the view's.
fn hg(cos_t: f32, g: f32) -> f32 {
    let g2 = g * g;
    return (1.0 - g2) / (4.0 * PI * pow(1.0 + g2 - 2.0 * g * cos_t, 1.5));
}

// The sky's colour near the horizon in a direction (the ring), towards its average looking down.
fn haze_colour(dir: vec3<f32>) -> vec3<f32> {
    let u = atan2(dir.x, dir.z) / (2.0 * PI) + 0.5;
    let side = to_linear(textureSampleLevel(ring, rep, vec2<f32>(u, 0.5), 0.0).rgb);
    let down = clamp(-dir.y, 0.0, 1.0);
    return mix(side, to_linear(c.haze.rgb), smoothstep(0.1, 0.8, down));
}

// A per-pixel offset of the march's start (white noise: a pattern would show as lines over the clouds).
fn jitter(p: vec2<f32>) -> f32 {
    var h = u32(p.x) * 1973u + u32(p.y) * 9277u + 26699u;
    h = (h ^ (h >> 16u)) * 0x7feb352du;
    h = (h ^ (h >> 15u)) * 0x846ca68bu;
    h = h ^ (h >> 16u);
    return f32(h & 0xffffu) / 65536.0;
}

struct FsOut {
    @location(0) colour: vec4<f32>,
    @builtin(sample_mask) mask: u32,
}

@fragment
fn fs(in: VsOut) -> FsOut {
    let size = vec2<f32>(c.wind.w, c.haze.a);
    let px = vec2<i32>(in.pos.xy);
    let ndc = vec2<f32>(in.pos.x / size.x * 2.0 - 1.0, 1.0 - in.pos.y / size.y * 2.0);
    let far = c.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    let eye = c.eye.xyz;
    let dir = normalize(far.xyz / far.w - eye);
    // how far the scene is along this ray. The sky box (sky.rs: a cube SIZE round the eye) writes depth as an opaque
    // mesh does, so where the depth is the box's own along this ray, there is only sky, behind the clouds: the box's
    // distance is SIZE over the ray's largest component. Anything nearer is the level.
    // Each sample of an anti-aliased pixel has its own depth: the cloud is shaded once, along the farthest, and
    // written only to the samples that see about as far, so an edge against the clouds keeps its coverage.
    var dists: array<f32, NS>;
    var scene = 0.0;
    for (var s = 0; s < NS; s++) {
        let z = textureLoad(depth, px, s);
        var d = 1e9;
        if z < 1.0 {
            let wp = c.inv_view_proj * vec4<f32>(ndc, z, 1.0);
            d = length(wp.xyz / wp.w - eye);
            let a = abs(dir);
            let box = SKY / max(max(a.x, a.y), a.z);
            if d > box * 0.999 {
                d = 1e9;
            }
        }
        dists[s] = d;
        scene = max(scene, d);
    }
    var mask = 0u;
    for (var s = 0; s < NS; s++) {
        if dists[s] >= scene * 0.98 {
            mask |= 1u << u32(s);
        }
    }
    let bottom = c.slab.x;
    let top = c.slab.y;
    // the stretch of the ray inside the slab
    var t0 = 0.0;
    var t1 = FAR;
    if abs(dir.y) > 1e-5 {
        let a = (top - eye.y) / dir.y;
        let b = (bottom - eye.y) / dir.y;
        t0 = max(min(a, b), 0.0);
        t1 = min(max(a, b), FAR);
    } else if eye.y > top || eye.y < bottom {
        discard;
    }
    let end = min(t1, scene);
    // below the layer: the haze there hides the world under it, as much as the level says
    let under = dir.y < 0.0 && scene > t1;
    if end <= t0 && !under {
        discard;
    }
    let haze_density = c.march.z;
    let hz = haze_colour(dir);
    let sun = normalize(c.sun.xyz);
    // the light is followed a little higher than the sun stands: a sun on the horizon would leave a sea of cloud all
    // in the shade of its own billows, where the eye sees their tops lit
    let to_light = normalize(sun + vec3<f32>(0.0, 0.25, 0.0));
    let sun_col = c.sun_color.rgb;
    let sky_col = c.sky.rgb;
    let shade_col = c.shadow.rgb;
    let cos_t = dot(dir, sun);
    // forward scattering (the bright rims towards the sun) with a little back, made 1 on average
    let phase = (mix(hg(cos_t, -0.15), hg(cos_t, 0.55), 0.5)) * 4.0 * PI;
    let ext = 40.0 / c.slab.w;
    let steps = max(c.march.x, 1.0);
    let light_steps = i32(c.march.y);
    let detail = c.wind.z > 0.5;

    var col = vec3<f32>(0.0);
    var tr = 1.0;
    if end > t0 {
        // steps no longer than a twelfth of a cloud near the eye (looking down the slab is crossed in a few dozen),
        // growing with the distance, where a pixel covers more; what the steps allowed do not reach is haze
        let len = end - t0;
        let base = clamp(len / steps, 0.6, c.slab.w * 0.08);
        var t = t0 + base * jitter(in.pos.xy);
        var i = 0.0;
        var fine = false;
        loop {
            if t >= end || tr < 0.01 {
                break;
            }
            if i >= steps * 2.0 {
                col += tr * hz;
                tr = 0.0;
                break;
            }
            var dt = base + t * 0.01;
            let p = eye + dir * t;
            // farther off the billows are below a pixel and only cost
            let d = density(p, detail && t < 900.0);
            // the first cloud met: back a step and on in finer ones from there, so its edge is found where it is
            // (else the start's jitter shows as grain on every billow)
            if d > 0.002 && !fine {
                fine = true;
                t = max(t - dt, t0);
                i += 1.0;
                continue;
            }
            if fine {
                dt *= 0.35;
            }
            if d > 0.002 {
                // the light reaching here from the sun through the cloud, in growing steps
                var od = 0.0;
                var ls = c.slab.w * 0.06;
                var lp = p;
                for (var j = 0; j < light_steps; j++) {
                    lp = lp + to_light * ls;
                    od += density(lp, false) * ls;
                    ls *= 1.9;
                }
                od *= ext;
                let h = clamp((p.y - bottom) / (top - bottom), 0.0, 1.0);
                // the sun's light, with the many-times scattered part that keeps the shaded sides from going black
                let direct = exp(-od) * phase + 0.5 * exp(-od * 0.25) * mix(1.0, phase, 0.3) * 0.5;
                // the sky over the tops, the bounced light under them, darker deep inside; the sky's light is shut out
                // where more cloud lies over this point (the creases between the billows)
                let over = density(p + vec3<f32>(0.0, c.slab.w * 0.25, 0.0), false);
                let amb = mix(shade_col, sky_col * exp(-over * 1.5), h * h * (3.0 - 2.0 * h)) * (1.0 - 0.3 * d);
                // a soft shoulder rather than a clip: the sunlit tops stay white without flattening
                var s = 1.0 - exp(-(sun_col * direct + amb) * 0.95);
                // the air between the eye and here, and wholly the haze by the farthest the march goes (so the sea
                // meets the horizon without an edge)
                let f = max(1.0 - exp(-haze_density * t), smoothstep(FAR * 0.4, FAR, t));
                s = mix(s, hz, f);
                let a = exp(-d * ext * dt);
                col += tr * s * (1.0 - a);
                tr *= a;
            }
            t += dt;
            i += 1.0;
        }
    }
    if under && tr > 0.0 {
        // the haze under the layer (the shadow's colour near, the horizon's farther off), and wholly the horizon's past
        // the farthest the march goes
        let f = max(1.0 - exp(-haze_density * t1), smoothstep(FAR * 0.4, FAR, t1));
        let deep = mix(mix(1.0 - exp(-shade_col * 1.15), hz, 0.35), hz, f);
        var k = c.march.w;
        if t1 >= FAR {
            k = 1.0;
        }
        col += tr * deep * k;
        tr *= 1.0 - k;
    }
    let alpha = 1.0 - tr;
    if alpha < 0.002 {
        discard;
    }
    var out: FsOut;
    out.colour = vec4<f32>(to_gamma(col / alpha) * alpha, alpha);
    out.mask = mask;
    return out;
}
