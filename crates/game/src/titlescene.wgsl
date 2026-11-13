// The title and loading screens' picture (titlescene.rs): a small scene traced per pixel, nothing loaded for it. A
// polished wooden ball on a rope over a sea of clouds under an evening sky. On the title screen the camera stands
// near the rope and drifts; on the loading screen the rope is a thin wire across the window and the ball rolls
// along it as the load goes on. Edges are covered analytically (no multisampling needed at any size), the rope
// blurs away from the focus, and the result is dithered against banding.

struct Params {
    // target width and height in pixels, seconds, opacity
    a: vec4<f32>,
    // 0 title / 1 loading, progress 0..1, the sun's height (tangent), stars
    b: vec4<f32>,
    // rgb the zenith, a how much cirrus
    zenith: vec4<f32>,
    // rgb the horizon, a how dark the loading screen's backdrop is kept
    horizon: vec4<f32>,
    sun: vec4<f32>,
    // the sea of clouds, lit and in shade
    lit: vec4<f32>,
    shade: vec4<f32>,
};

// The frame's parameters come as the one instance's attributes (no uniform buffer, no bind group: every object made
// for a frame waits its turn behind the loading thread's uploads) and are kept here for the functions below.
var<private> P: Params;

// The ball's own wooden texture (the game's own picture, equirectangular), with its mips.
@group(0) @binding(0) var ball_tex: texture_2d<f32>;
@group(0) @binding(1) var ball_samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) @interpolate(flat) a: vec4<f32>,
    @location(1) @interpolate(flat) b: vec4<f32>,
    @location(2) @interpolate(flat) zenith: vec4<f32>,
    @location(3) @interpolate(flat) horizon: vec4<f32>,
    @location(4) @interpolate(flat) sun: vec4<f32>,
    @location(5) @interpolate(flat) lit: vec4<f32>,
    @location(6) @interpolate(flat) shade: vec4<f32>,
};

@vertex
fn vs(
    @builtin(vertex_index) i: u32,
    @location(0) a: vec4<f32>,
    @location(1) b: vec4<f32>,
    @location(2) zenith: vec4<f32>,
    @location(3) horizon: vec4<f32>,
    @location(4) sun: vec4<f32>,
    @location(5) lit: vec4<f32>,
    @location(6) shade: vec4<f32>,
) -> VsOut {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(p * 2.0 - vec2<f32>(1.0), 0.0, 1.0);
    out.a = a;
    out.b = b;
    out.zenith = zenith;
    out.horizon = horizon;
    out.sun = sun;
    out.lit = lit;
    out.shade = shade;
    return out;
}

fn hash2(p: vec2<f32>) -> f32 {
    var q = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

fn hash3(p: vec3<f32>) -> f32 {
    var q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yxz + 33.33);
    return fract((q.x + q.y) * q.z);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(hash3(i), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x);
    let b = mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x);
    let c = mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x);
    let d = mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x);
    return mix(mix(a, b, u.y), mix(c, d, u.y), u.z);
}

fn fbm(p0: vec2<f32>, octaves: i32) -> f32 {
    var p = p0;
    var s = 0.0;
    var a = 0.5;
    for (var k = 0; k < octaves; k++) {
        s += a * noise2(p);
        p = mat2x2<f32>(1.6, 1.2, -1.2, 1.6) * p + vec2<f32>(3.1, 7.7);
        a *= 0.5;
    }
    return s;
}

/// The sky in direction `rd` seen from height `eye` over the clouds; `fine`: with cirrus, stars and the clouds'
/// detail (the background), else the plain one (what the ball mirrors). `tight`: how much narrower the sun's glow
/// is drawn (a long lens would spread it over the frame).
fn sky(rd: vec3<f32>, sd: vec3<f32>, eye: f32, tight: f32, fine: bool) -> vec3<f32> {
    let t = P.a.z;
    let up = clamp(rd.y, 0.0, 1.0);
    let s = max(dot(rd, sd), 0.0);
    // the sun's side of the horizon is warm, the far side takes the zenith's colour
    let side = 0.5 + 0.5 * dot(normalize(rd.xz + vec2<f32>(1e-5)), normalize(sd.xz));
    let low = mix(mix(P.zenith.rgb, P.horizon.rgb, 0.45), P.horizon.rgb, side * side);
    // three stops: the horizon's band, a rose-violet belt over it, the zenith
    let belt = mix(P.zenith.rgb * 1.8, P.horizon.rgb, 0.36) * vec3<f32>(1.12, 0.80, 1.05);
    var c = mix(low, belt, smoothstep(0.0, 0.13, up));
    c = mix(c, P.zenith.rgb, smoothstep(0.04, 0.34, up));
    c += P.sun.rgb * (0.10 * pow(s, 6.0 * tight) + 0.42 * pow(s, 70.0 * tight) + 1.2 * pow(s, 900.0 * tight));
    if fine {
        // stars where the sky is dark
        let g = rd * 190.0;
        let cell = floor(g);
        let h = hash3(cell);
        if h > 0.9965 {
            let d = length(fract(g) - vec3<f32>(0.5));
            let tw = 0.75 + 0.25 * sin(t * (1.0 + 3.0 * fract(h * 91.7)) + h * 400.0);
            c += vec3<f32>(0.9, 0.95, 1.0) * P.b.w * tw * smoothstep(0.5, 0.05, d)
                * smoothstep(0.10, 0.55, rd.y) * (0.4 + 0.6 * fract(h * 517.3));
        }
        // cirrus: long streaks high up, catching the sun
        let uv = rd.xz / (rd.y + 0.22);
        let n = fbm(uv * vec2<f32>(0.9, 2.6) + vec2<f32>(t * 0.012, 0.0), 5);
        let streak = smoothstep(0.46, 0.80, n) * smoothstep(0.02, 0.22, rd.y) * (1.0 - smoothstep(0.5, 0.95, rd.y));
        let tint = mix(mix(P.zenith.rgb, P.horizon.rgb, 0.75), P.sun.rgb * 1.15, 0.25 + 0.75 * pow(s, 2.0));
        c = mix(c, tint, streak * P.zenith.a);
        // the sun's disc
        c += (P.sun.rgb * 3.4 + vec3<f32>(0.3)) * smoothstep(1.0 - 0.00016 / tight, 1.0 - 0.00004 / tight, dot(rd, sd));
    }
    // the sea of clouds below
    if rd.y < 0.03 {
        let dy = min(rd.y, -0.0015);
        let dist = eye / -dy;
        let p = rd.xz * dist;
        var n: f32;
        var facing = 0.5;
        if fine {
            let drift = vec2<f32>(t * 0.02, t * 0.006);
            n = fbm(p * 0.011 + drift, 6);
            // the slopes turned to the sun catch its light
            let toward = fbm((p + normalize(sd.xz) * 14.0) * 0.011 + drift, 4);
            facing = clamp(0.5 + (n - toward) * 7.0, 0.0, 1.0);
        } else {
            n = fbm(p * 0.011, 3);
        }
        let puff = smoothstep(0.30, 0.74, n);
        var sea = mix(P.shade.rgb * (0.55 + 0.45 * puff), P.lit.rgb, puff * (0.25 + 0.75 * facing));
        sea += P.sun.rgb * (0.05 * pow(s, 3.0) + 0.75 * pow(s, 16.0)) * puff * (0.2 + 0.8 * facing);
        let haze = 1.0 - exp(-dist * 0.0007);
        sea = mix(sea, low * 0.9 + P.sun.rgb * 0.25 * pow(s, 8.0), haze);
        c = mix(c, sea, smoothstep(0.03, -0.012, rd.y));
    }
    return c;
}

fn turn(v: vec3<f32>, axis: vec3<f32>, angle: f32) -> vec3<f32> {
    let sn = sin(angle);
    let cs = cos(angle);
    return v * cs + cross(axis, v) * sn + axis * dot(axis, v) * (1.0 - cs);
}

/// The ball's wood at `q` on its surface (in the ball's own frame).
fn wood(q: vec3<f32>) -> vec3<f32> {
    let warp = vec2<f32>(noise3(q * 1.3 + vec3<f32>(4.0)), noise3(q * 1.3 + vec3<f32>(11.0))) - vec2<f32>(0.5);
    let d = length(q.xy * vec2<f32>(1.0, 1.3) + vec2<f32>(0.9, 0.3) + warp * 0.22);
    let ring = 0.5 + 0.5 * sin(d * 34.0 + 4.5 * noise3(q * 1.8) + 1.5 * noise3(q * 6.0));
    let grain = noise3(q * vec3<f32>(46.0, 46.0, 5.0));
    let pore = noise3(q * vec3<f32>(120.0, 120.0, 9.0));
    var c = mix(vec3<f32>(0.41, 0.19, 0.068), vec3<f32>(0.57, 0.30, 0.112), ring);
    c *= 0.82 + 0.26 * grain;
    c *= 0.90 + 0.14 * pore;
    return c;
}


struct Tube {
    cov: f32,
    s: f32,
    t: f32,
    d: f32,
    e: vec3<f32>,
}

/// A tube (a line `org + ad * s`, |s| <= reach, of radius `rr`) as the ray sees it: the nearest points of the two,
/// the coverage with the focus blur and the pixel's width.
fn tube_hit(ro: vec3<f32>, rd: vec3<f32>, org: vec3<f32>, ad: vec3<f32>, reach: f32, rr: f32, focus: f32, aperture: f32, px: f32) -> Tube {
    let b = dot(rd, ad);
    let w0 = ro - org;
    let denom = max(1.0 - b * b, 1e-5);
    let s = clamp((dot(ad, w0) - b * dot(rd, w0)) / denom, -reach, reach);
    let pa = org + ad * s;
    let tc = max(dot(pa - ro, rd), 0.0);
    let e = ro + rd * tc - pa;
    let d = length(e);
    let blur = aperture * abs(tc - focus) / focus + px * tc * 0.75;
    let cov = clamp((rr + blur - d) / (2.0 * blur), 0.0, 1.0) * min(1.0, 2.0 * rr / (rr + blur));
    return Tube(cov, s, tc, d, e);
}

/// Polished steel with normal `n`: the sky and the clouds mirrored (strong, a metal), the sun's glint, a little body.
fn metal(n: vec3<f32>, rd: vec3<f32>, sd: vec3<f32>, eye: f32, body: vec3<f32>, mirror: f32) -> vec3<f32> {
    let ndv = clamp(dot(n, -rd), 0.0, 1.0);
    let r = reflect(rd, n);
    let fres = mirror + (1.0 - mirror) * pow(1.0 - ndv, 5.0);
    let refl = sky(r, sd, eye, 1.0, false) * vec3<f32>(1.0, 1.0, 1.05) * 1.35;
    let hv = normalize(sd - rd);
    let ndh = max(dot(n, hv), 0.0);
    let spec = min(pow(ndh, 400.0) * 40.0, 6.0) + pow(ndh, 40.0) * 0.25;
    let ndl = clamp(dot(n, sd) * 0.6 + 0.4, 0.0, 1.0);
    let amb = mix(P.lit.rgb * 0.5 + P.horizon.rgb * 0.25, P.zenith.rgb * 1.4 + P.horizon.rgb * 0.2, n.y * 0.5 + 0.5);
    // a studio strip high up so the polish reads even with the sun behind
    let hk = normalize(normalize(vec3<f32>(0.45, 0.8, 0.5)) - rd);
    let strip = pow(max(dot(n, hk), 0.0), 60.0) * 0.6;
    return body * (P.sun.rgb * 1.2 * ndl + amb) * (1.0 - fres) + refl * fres
        + P.sun.rgb * spec * step(0.0, dot(n, sd) + 0.2) + vec3<f32>(strip) * (P.zenith.rgb + P.horizon.rgb + vec3<f32>(0.3));
}

/// A box in the rail's frame under the ray: entry and exit distances, the entry face's normal (local).
struct BoxHit {
    tn: f32,
    tf: f32,
    n: vec3<f32>,
}

fn box_hit(lo: vec3<f32>, ld: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> BoxHit {
    let sg = select(vec3<f32>(-1.0), vec3<f32>(1.0), ld >= vec3<f32>(0.0));
    let inv = 1.0 / (ld + sg * 1e-6);
    let t1 = (bmin - lo) * inv;
    let t2 = (bmax - lo) * inv;
    let tmin = min(t1, t2);
    let tmax = max(t1, t2);
    let tn = max(max(tmin.x, tmin.y), tmin.z);
    let tf = min(min(tmax.x, tmax.y), tmax.z);
    var n = vec3<f32>(0.0);
    if tn == tmin.x {
        n.x = -sg.x;
    } else if tn == tmin.y {
        n.y = -sg.y;
    } else {
        n.z = -sg.z;
    }
    return BoxHit(tn, tf, n);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    P = Params(in.a, in.b, in.zenith, in.horizon, in.sun, in.lit, in.shade);
    let frag = in.clip;
    let res = P.a.xy;
    let t = P.a.z;
    let loading = P.b.x > 0.5;
    let aspect = res.x / res.y;
    let ndc = vec2<f32>((frag.x / res.x * 2.0 - 1.0) * aspect, 1.0 - frag.y / res.y * 2.0);
    let world_up = vec3<f32>(0.0, 1.0, 0.0);

    // the shot: camera, rail, ball
    let radius = 1.0;
    var tr = 0.17;          // the tubes' radius
    var ro: vec3<f32>;
    var fwd: vec3<f32>;
    var focal: f32;
    var ad: vec3<f32>;     // the rail's direction
    var reach = 1.0e4;      // the rail's reach length
    var along: f32;        // where the ball is on it
    var eye: f32;          // the camera's height over the clouds
    var aperture = 0.0;
    var tight = 1.0;
    var bank = 0.0;
    var sd: vec3<f32>;
    if loading {
        focal = 5.67;
        let dist = 58.0;
        ro = vec3<f32>(0.0, -3.0, dist);
        fwd = normalize(vec3<f32>(0.0, 0.106, -1.0));
        ad = vec3<f32>(1.0, 0.0, 0.0);
        reach = min(0.30 * aspect, 0.42) * 2.0 * dist / focal;
        along = mix(-reach, reach, clamp(P.b.y, 0.0, 1.0));
        eye = 30.0;
        tr = 0.12;
        bank = 0.32;
        // a long lens: the sun low at the right, under the rail's height
        sd = normalize(vec3<f32>(0.235, 0.034, -1.0));
        tight = 4.0;
    } else {
        focal = 3.73;
        ad = normalize(vec3<f32>(1.0, 0.0, 1.35));
        along = -0.9 + 0.20 * t;
        ro = vec3<f32>(-3.0 + 0.16 * t, 1.05 + 0.03 * t, 10.9 - 0.14 * t);
        fwd = normalize(vec3<f32>(1.75, 1.15, -12.5));
        eye = 34.0;
        aperture = 0.10;
        sd = normalize(vec3<f32>(-0.16, P.b.z, -1.0));
    }
    let right = normalize(cross(fwd, world_up));
    let upv = cross(right, fwd);
    let rd = normalize(fwd * focal + right * ndc.x + upv * ndc.y);
    let px = 2.0 / (res.y * focal);   // a pixel's angle

    var col = sky(rd, sd, eye, tight, true);
    if loading {
        // the backdrop stays behind the words
        let g = dot(col, vec3<f32>(0.3, 0.55, 0.15));
        col = mix(vec3<f32>(g), col, 0.92) * P.horizon.a;
    }

    // the rail's frame: along, across (banked on the loading screen), and out of the rail
    let side = turn(normalize(cross(ad, world_up)), ad, bank);
    let upr = cross(side, ad);
    let gap = 0.70;                                   // half the distance between the tubes' axes
    let lift = 0.25;
    let org = vec3<f32>(0.0, lift, 0.0);
    let rest = sqrt((radius + tr) * (radius + tr) - gap * gap);
    let centre = org + ad * along + upr * rest;
    let zen = P.zenith.rgb;
    let hor = P.horizon.rgb;
    let focus = length(centre - ro);
    let gold = vec3<f32>(1.0, 0.70, 0.30);
    let steel_body = vec3<f32>(0.42, 0.44, 0.50);

    // the two tubes
    var tubes: array<Tube, 2>;
    var tube_col: array<vec3<f32>, 2>;
    var glow = vec3<f32>(0.0);
    for (var k = 0; k < 2; k++) {
        let off = side * (gap * (f32(k) * 2.0 - 1.0));
        let h = tube_hit(ro, rd, org + off, ad, reach, tr, focus, aperture, px);
        tubes[k] = h;
        var c = vec3<f32>(0.0);
        if h.cov > 0.0 {
            let b = dot(rd, ad);
            let inward = normalize(-(rd - ad * b));
            let kk = clamp(h.d / tr, 0.0, 1.0);
            let n = normalize(h.e / tr + inward * sqrt(1.0 - kk * kk));
            // the ball's shade where it rests
            let contact = mix(0.35, 1.0, smoothstep(0.1, 1.8, abs(h.s - along)));
            c = metal(n, rd, sd, eye, steel_body, 0.72) * contact;
            if loading {
                let glint_run = -reach + fract(t * 0.33) * (along + reach + 4.0);
                let glint = exp(-(h.s - glint_run) * (h.s - glint_run) / 3.0) * step(glint_run, along + 0.5);
                if h.s <= along {
                    // what is done glows, a glint running along it
                    c = c * 0.35 + gold * (1.5 + 2.0 * glint) * (0.55 + 0.45 * pow(1.0 - abs(dot(n, rd)), 0.5));
                } else {
                    c = c * 0.8 + col * 0.15;
                }
            }
        }
        tube_col[k] = c;
    }
    if loading {
        // a halo of light round what is done
        let b = dot(rd, ad);
        let w0 = ro - org;
        let denom = max(1.0 - b * b, 1e-5);
        let sg = clamp((dot(ad, w0) - b * dot(rd, w0)) / denom, -reach, along);
        let pg = org + ad * sg;
        let dg = length(ro + rd * max(dot(pg - ro, rd), 0.0) - pg);
        let run = -reach + fract(t * 0.33) * (along + reach + 4.0);
        let glint = exp(-(sg - run) * (sg - run) / 3.0) * step(run, along + 0.5);
        glow = gold * (0.45 * exp(-dg / 0.45) + 0.14 * exp(-dg / 1.8)) * (1.0 + 1.6 * glint);
    }

    // the cross-ties under the tubes, with a clamp over each tube
    var tie_cov = 0.0;
    var tie_t = 1.0e9;
    var tie_col = vec3<f32>(0.0);
    {
        let sp = 3.2;
        let wide = gap + tr + 0.16;
        let lo = vec3<f32>(dot(ro - org, ad), dot(ro - org, side), dot(ro - org, upr));
        let ld = vec3<f32>(dot(rd, ad), dot(rd, side), dot(rd, upr));
        // the stretch of the ray inside the rail's block, and so which ties it can meet
        let region = box_hit(lo, ld, vec3<f32>(-1.0e5, -wide, -tr - 0.26), vec3<f32>(1.0e5, wide, tr + 0.08));
        let t0 = max(region.tn, 0.0);
        if region.tf > t0 {
            let a0 = lo.x + ld.x * t0;
            let a1 = lo.x + ld.x * region.tf;
            let k0 = i32(floor((min(a0, a1) - 0.3) / sp - 0.5));
            let k1 = i32(ceil((max(a0, a1) + 0.3) / sp - 0.5));
            for (var kk = 0; kk <= 10; kk++) {
                let k = k0 + kk;
                if k > k1 {
                    break;
                }
                let ak = (f32(k) + 0.5) * sp;
                if abs(ak) > reach + 0.1 {
                    continue;
                }
                for (var part = 0; part < 3; part++) {
                    var bmin: vec3<f32>;
                    var bmax: vec3<f32>;
                    if part == 0 {
                        bmin = vec3<f32>(ak - 0.09, -wide, -tr - 0.14);
                        bmax = vec3<f32>(ak + 0.11, wide, -tr + 0.005);
                    } else {
                        let sgn = f32(part) * 2.0 - 3.0;
                        bmin = vec3<f32>(ak - 0.17, sgn * gap - tr - 0.05, -tr - 0.05);
                        bmax = vec3<f32>(ak + 0.17, sgn * gap + tr + 0.05, tr + 0.05);
                    }
                    let bh = box_hit(lo, ld, bmin, bmax);
                    if bh.tf > max(bh.tn, 0.0) && bh.tn < tie_t {
                        let c = clamp((bh.tf - bh.tn) / (px * max(bh.tn, 1.0) * 1.2), 0.0, 1.0);
                        let n = ad * bh.n.x + side * bh.n.y + upr * bh.n.z;
                        var tc = vec3<f32>(0.0);
                        if part == 0 {
                            tc = metal(n, rd, sd, eye, vec3<f32>(0.34, 0.35, 0.40), 0.2);
                        } else {
                            tc = metal(n, rd, sd, eye, steel_body * 1.3, 0.7);
                        }
                        if loading && ak <= along {
                            tc = tc * 0.4 + gold * 1.3;
                        }
                        tie_cov = c;
                        tie_t = bh.tn;
                        tie_col = tc;
                    }
                }
            }
        }
    }
    if loading {
        // the ends' knobs
        for (var k = 0; k < 2; k++) {
            let end = org + ad * (reach + 0.5) * (f32(k) * 2.0 - 1.0) + upr * (-0.05);
            let te = max(dot(end - ro, rd), 0.0);
            let de = length(ro + rd * te - end);
            let c = clamp((0.34 - de) / (px * te) + 0.5, 0.0, 1.0);
            let lit = k == 0 || P.b.y >= 0.999;
            var fc = col * 0.45 + vec3<f32>(0.42, 0.42, 0.46);
            if lit {
                fc = vec3<f32>(1.0, 0.82, 0.50) * 1.5;
            }
            if c > tie_cov {
                tie_col = fc * (1.2 - de * 1.2);
                tie_cov = c;
                tie_t = te;
            }
        }
    }

    // the ball
    let oc = centre - ro;
    let tca = dot(oc, rd);
    let d2 = max(dot(oc, oc) - tca * tca, 0.0);
    let dball = sqrt(d2);
    let ball_cov = clamp((radius - dball) / (px * tca) + 0.5, 0.0, 1.0);
    var ball_col = vec3<f32>(0.0);
    var ball_t = tca;
    if ball_cov > 0.0 {
        let thc = sqrt(max(radius * radius - d2, 0.0));
        ball_t = tca - thc;
        let n = normalize(ro + rd * ball_t - centre);
        let axis = -side;
        let q = turn(turn(n, axis, -along / radius), vec3<f32>(0.6, 0.0, 0.8), 0.9);
        var albedo: vec3<f32>;
        if P.sun.a > 0.5 {
            // the texture's equirectangular picture; the mip by the pixel's footprint on the sphere
            let uv = vec2<f32>(atan2(q.z, q.x) * 0.15915494 + 0.5, acos(clamp(q.y, -1.0, 1.0)) * 0.31830989);
            let texels = px * tca / radius * f32(textureDimensions(ball_tex).x) * 0.15915494;
            let tex = textureSampleLevel(ball_tex, ball_samp, uv, max(log2(max(texels, 1.0)), 0.0)).rgb;
            albedo = pow(tex, vec3<f32>(2.2)) * 1.05;
        } else {
            albedo = wood(q);
        }
        let ndv = clamp(dot(n, -rd), 0.0, 1.0);
        let ndl = dot(n, sd);
        let sunlit = clamp(ndl * 0.75 + 0.25, 0.0, 1.0);
        let amb = mix(P.lit.rgb * 0.6 + hor * 0.3, zen * 1.5 + hor * 0.25, n.y * 0.5 + 0.5);
        // a low warm fill from the camera's side, as the clouds throw back
        let fill = clamp(dot(n, normalize(vec3<f32>(0.5, 0.25, 1.0))), 0.0, 1.0);
        let shade = mix(0.45, 1.0, smoothstep(-1.0, -0.55, n.y));
        var c = albedo * (P.sun.rgb * 2.6 * sunlit * sunlit + amb * 1.5 + (hor * 0.5 + P.lit.rgb * 0.4) * fill) * shade;
        // the varnish: the sky mirrored, more at the rim; the sun's glint
        let r = reflect(rd, n);
        let fres = 0.075 + 0.925 * pow(1.0 - ndv, 5.0);
        let hv = normalize(sd - rd);
        let spec = min(pow(max(dot(n, hv), 0.0), 900.0) * 30.0, 4.0) + pow(max(dot(n, hv), 0.0), 60.0) * 0.5;
        c = c * (1.0 - fres) + sky(r, sd, eye, 1.0, false) * fres * 1.25 + P.sun.rgb * spec * step(0.0, ndl + 0.2);
        // a studio glint from above, so the polish reads even with the sun behind
        let key = normalize(vec3<f32>(0.45, 0.75, 0.5));
        let hk = normalize(key - rd);
        c += (zen * 0.6 + hor * 0.7 + vec3<f32>(0.25)) * (pow(max(dot(n, hk), 0.0), 220.0) * 1.3 + pow(max(dot(n, hk), 0.0), 18.0) * 0.07);
        if loading {
            // the ball is small here and the evening may be dark: a lamp of its own
            c += mix(albedo, vec3<f32>(0.5, 0.26, 0.10), 0.5) * vec3<f32>(1.0, 0.9, 0.74) * 1.2
                * pow(clamp(dot(n, normalize(vec3<f32>(-0.45, 0.65, 0.6))) * 0.6 + 0.4, 0.0, 1.0), 2.0);
        }
        ball_col = c;
    }

    if loading {
        col += glow;
    }
    // the layers, the farthest first
    var lt = array<f32, 4>(tubes[0].t, tubes[1].t, tie_t, ball_t);
    var lc = array<vec4<f32>, 4>(
        vec4<f32>(tube_col[0], tubes[0].cov),
        vec4<f32>(tube_col[1], tubes[1].cov),
        vec4<f32>(tie_col, tie_cov),
        vec4<f32>(ball_col, ball_cov),
    );
    for (var round = 0; round < 3; round++) {
        for (var i = 0; i < 3; i++) {
            if lt[i] < lt[i + 1] {
                let tt = lt[i];
                lt[i] = lt[i + 1];
                lt[i + 1] = tt;
                let cc = lc[i];
                lc[i] = lc[i + 1];
                lc[i + 1] = cc;
            }
        }
    }
    for (var i = 0; i < 4; i++) {
        col = mix(col, lc[i].rgb, lc[i].a);
    }
    if loading {
        // the ball's own warm halo
        col += vec3<f32>(1.0, 0.70, 0.32) * 0.20 * exp(-max(dball - radius, 0.0) / 1.1) * (1.0 - ball_cov);
    }

    // the frame: a soft vignette, the film's shoulder, the screen's curve, a grain against banding
    let v = ndc / vec2<f32>(aspect, 1.0);
    col *= 1.0 - 0.34 * pow(dot(v, v) * 0.5, 1.2);
    col = vec3<f32>(1.0) - exp(-col * 0.95);
    col = mix(col, col * col * (3.0 - 2.0 * col), 0.45);
    col = pow(col, vec3<f32>(1.0 / 2.2));
    col += vec3<f32>((hash2(frag.xy + vec2<f32>(fract(t) * 61.0, 17.0)) - 0.5) / 255.0);
    return vec4<f32>(col, P.a.w);
}
