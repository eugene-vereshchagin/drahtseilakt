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

@group(0) @binding(0) var<uniform> P: Params;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - vec2<f32>(1.0), 0.0, 1.0);
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

@fragment
fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let res = P.a.xy;
    let t = P.a.z;
    let loading = P.b.x > 0.5;
    let aspect = res.x / res.y;
    let ndc = vec2<f32>((frag.x / res.x * 2.0 - 1.0) * aspect, 1.0 - frag.y / res.y * 2.0);
    let world_up = vec3<f32>(0.0, 1.0, 0.0);

    // the shot: camera, rope, ball
    let radius = 1.0;
    var rope_r = 0.085;
    var ro: vec3<f32>;
    var fwd: vec3<f32>;
    var focal: f32;
    var ad: vec3<f32>;     // the rope's direction
    var reach = 1.0e4;      // the rope's reach length
    var along: f32;        // where the ball is on it
    var eye: f32;          // the camera's height over the clouds
    var aperture = 0.0;
    var tight = 1.0;
    var sd: vec3<f32>;
    if loading {
        focal = 5.67;
        let dist = 58.0;
        ro = vec3<f32>(0.0, -3.58, dist);
        fwd = normalize(vec3<f32>(0.0, 0.106, -1.0));
        ad = vec3<f32>(1.0, 0.0, 0.0);
        reach = min(0.30 * aspect, 0.42) * 2.0 * dist / focal;
        along = mix(-reach, reach, clamp(P.b.y, 0.0, 1.0));
        eye = 30.0;
        rope_r = 0.075;
        // a long lens: the sun low at the right, under the wire's height
        sd = normalize(vec3<f32>(0.235, 0.034, -1.0));
        tight = 4.0;
    } else {
        focal = 3.73;
        ad = normalize(vec3<f32>(1.0, 0.0, 1.35));
        along = -0.9 + 0.20 * t;
        ro = vec3<f32>(-3.0 + 0.16 * t, 0.62 + 0.03 * t, 10.9 - 0.14 * t);
        fwd = normalize(vec3<f32>(1.75, 1.42, -12.5));
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

    let centre = ad * along + world_up * (radius + rope_r);
    let zen = P.zenith.rgb;
    let hor = P.horizon.rgb;

    // the rope: the nearest points of the ray and its axis
    let b = dot(rd, ad);
    let w0 = ro;
    let denom = max(1.0 - b * b, 1e-5);
    let s_axis = clamp((dot(ad, w0) - b * dot(rd, w0)) / denom * 1.0, -reach, reach);
    let pa = ad * s_axis;
    let tc = max(dot(pa - ro, rd), 0.0);
    let pr = ro + rd * tc;
    let e = pr - pa;
    let dline = length(e);
    var rope_cov = 0.0;
    var rope_col = vec3<f32>(0.0);
    var glow = vec3<f32>(0.0);
    {
        let focus = length(centre - ro);
        let blur = aperture * abs(tc - focus) / focus + px * tc * 0.75;
        rope_cov = clamp((rope_r + blur - dline) / (2.0 * blur), 0.0, 1.0) * min(1.0, 2.0 * rope_r / (rope_r + blur));
        if rope_cov > 0.0 {
            let inward = normalize(-(rd - ad * b));
            let k = clamp(dline / rope_r, 0.0, 1.0);
            let n0 = normalize(e / rope_r + inward * sqrt(1.0 - k * k));
            // three strands laid in a twist
            let side = normalize(cross(ad, world_up));
            let phi = atan2(dot(n0, side), dot(n0, world_up));
            let lay = phi * 3.0 + s_axis * 9.0;
            // the strands show while they are in focus and wider than a few pixels
            let soft = clamp(1.0 - blur / (rope_r * 0.8), 0.0, 1.0) * clamp(0.7 / (px * tc * 7.0), 0.0, 1.0);
            let n = normalize(n0 + ad * cos(lay) * 0.55 * soft);
            let fibre = noise2(vec2<f32>(s_axis * 30.0 + phi * 4.0, phi * 9.0 - s_axis * 14.0));
            var base = vec3<f32>(0.50, 0.36, 0.20) * (0.62 + 0.38 * mix(0.6, abs(sin(lay * 0.5)), soft))
                * (0.85 + 0.3 * mix(0.5, fibre, soft));
            let ndl = clamp(dot(n, sd) * 0.6 + 0.4, 0.0, 1.0);
            let amb = mix(P.lit.rgb * 0.5 + hor * 0.25, zen * 1.4 + hor * 0.2, n.y * 0.5 + 0.5);
            // the ball's shade where it rests
            let near = abs(s_axis - along);
            let contact = mix(0.25, 1.0, smoothstep(0.1, 1.6, near));
            rope_col = base * (P.sun.rgb * 2.2 * ndl + amb * 1.3) * contact;
            // the light along its edge against the sun
            rope_col += P.sun.rgb * 0.5 * pow(1.0 - abs(dot(n, rd)), 3.0) * max(dot(-rd, -sd), 0.0);
        }
        if loading {
            // what is done glows; a glint runs along it
            let done = s_axis <= along;
            let sg = clamp((dot(ad, w0) - b * dot(rd, w0)) / denom, -reach, along);
            let pg = ad * sg;
            let dg = length(ro + rd * max(dot(pg - ro, rd), 0.0) - pg);
            let gold = vec3<f32>(1.0, 0.70, 0.30);
            let run = -reach + fract(t * 0.33) * (along + reach + 4.0);
            let glint = exp(-(sg - run) * (sg - run) / 3.0) * step(run, along + 0.5);
            glow = gold * (0.55 * exp(-dg / 0.30) + 0.16 * exp(-dg / 1.5)) * (1.0 + 1.6 * glint);
            if done {
                rope_col = vec3<f32>(1.0, 0.86, 0.56) * (1.6 + 2.0 * glint);
            } else {
                rope_col = col * 0.45 + vec3<f32>(0.42, 0.42, 0.46);
                rope_cov *= 0.85;
            }
            // the ends' ferrules
            for (var k = 0; k < 2; k++) {
                let end = ad * (reach + 0.2) * (f32(k) * 2.0 - 1.0);
                let te = max(dot(end - ro, rd), 0.0);
                let de = length(ro + rd * te - end);
                let c = clamp((0.26 - de) / (px * te) + 0.5, 0.0, 1.0);
                let lit = k == 0 || P.b.y >= 0.999;
                var fc = col * 0.45 + vec3<f32>(0.42, 0.42, 0.46);
                if lit {
                    fc = vec3<f32>(1.0, 0.82, 0.50) * 1.5;
                }
                rope_col = mix(rope_col, fc * (1.2 - de * 1.6), c);
                rope_cov = max(rope_cov, c);
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
        let axis = normalize(cross(world_up, ad));
        let q = turn(turn(n, axis, -along / radius), vec3<f32>(0.6, 0.0, 0.8), 0.9);
        let albedo = wood(q);
        let ndv = clamp(dot(n, -rd), 0.0, 1.0);
        let ndl = dot(n, sd);
        let sunlit = clamp(ndl * 0.75 + 0.25, 0.0, 1.0);
        let amb = mix(P.lit.rgb * 0.6 + hor * 0.3, zen * 1.5 + hor * 0.25, n.y * 0.5 + 0.5);
        // a low warm fill from the camera's side, as the clouds throw back
        let fill = clamp(dot(n, normalize(vec3<f32>(0.5, 0.25, 1.0))), 0.0, 1.0);
        let rest = mix(0.45, 1.0, smoothstep(-1.0, -0.55, n.y));
        var c = albedo * (P.sun.rgb * 2.6 * sunlit * sunlit + amb * 1.5 + (hor * 0.5 + P.lit.rgb * 0.4) * fill) * rest;
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
            // the ball is small here and the evening may be dark: a lamp of its own, and a calmer grain
            c += mix(albedo, vec3<f32>(0.5, 0.26, 0.10), 0.5) * vec3<f32>(1.0, 0.9, 0.74) * 1.5
                * pow(clamp(dot(n, normalize(vec3<f32>(-0.45, 0.65, 0.6))) * 0.6 + 0.4, 0.0, 1.0), 2.0);
        }
        ball_col = c;
    }

    // the nearer of the two over the farther
    if loading {
        col += glow;
    }
    if tc > ball_t {
        col = mix(col, rope_col, rope_cov);
        col = mix(col, ball_col, ball_cov);
    } else {
        col = mix(col, ball_col, ball_cov);
        col = mix(col, rope_col, rope_cov);
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
