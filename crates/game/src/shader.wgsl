// Level geometry: textured, lit with one directional light and ambient as the original's fixed-function
// pipeline does, with the material's alpha test. Lighting is per pixel with the interpolated normal.

struct Globals {
    view_proj: mat4x4<f32>,
    // xyz: direction the light travels; w unused.
    light_dir: vec4<f32>,
    // rgb: ambient light; a: unused.
    ambient: vec4<f32>,
    // world to camera space, and where the eye is
    view: mat4x4<f32>,
    eye: vec4<f32>,
    // rgb: the light's colour (the level's Light).
    light_color: vec4<f32>,
    // The passing lights (lights.rs): the last level's lightning (a second directional light) and the new ball's
    // point light (xyz where, w its reach); black when they are hidden.
    flash_dir: vec4<f32>,
    flash_color: vec4<f32>,
    point_pos: vec4<f32>,
    point_color: vec4<f32>,
    // The level's fog: rgb its colour, a its density a unit of distance; x the height it begins below, y the depth
    // under that where it is whole, z 1 when there is fog. Only for what is lit (the sky, sprites and the clouds,
    // drawn unlit, keep their own colour).
    fog_color: vec4<f32>,
    fog_height: vec4<f32>,
    // rgb: the fog's colour seen straight down (the world far below), the colour above seen towards the horizon
    fog_down: vec4<f32>,
    // A script's fog (Set Fog): x the mode (0 none, 1 exp, 2 exp2, 3 linear), y start, z end, w density; its colour.
    script_fog: vec4<f32>,
    script_fog_color: vec4<f32>,
};

struct Material {
    diffuse: vec4<f32>,
    // rgb: emissive; a: 1 when a texture is bound.
    emissive: vec4<f32>,
    // x: alpha cutoff (negative: no alpha test); y: 1 unlit; z: texture coordinates from the view (2 Reflect,
    // 3 Chrome; 0 the mesh's).
    params: vec4<f32>,
    // rgb: the material's ambient colour; a: the parallax depth in texture coordinates (0: none), the normal map's
    // alpha then a height.
    ambient: vec4<f32>,
    // rgb: the highlight's colour; a: its power (0: none).
    specular: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
struct Instance {
    m: mat4x4<f32>,
    // xy: added to the texture coordinates (a script's Texture Scroller); z: 1 to draw unlit (sprites).
    uv: vec4<f32>,
    // The diffuse colour instead of the material's; a < 0: the material's.
    diffuse: vec4<f32>,
    // The ball's blob shadow on this floor (TT Simple Shadow): xy its centre (world x, z), z the square's side;
    // w 1 when this is the shadow's pass.
    shadow: vec4<f32>,
};

@group(1) @binding(0) var<uniform> inst: Instance;
@group(2) @binding(0) var<uniform> mat: Material;
@group(2) @binding(1) var tex: texture_2d<f32>;
@group(2) @binding(2) var samp: sampler;
// The material's normal map (params.w 1): the relief a remastered level's toolbox gives it.
@group(2) @binding(3) var relief: texture_2d<f32>;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
    // the texture's u direction on the surface, and (w) which way its v goes
    @location(3) tangent: vec4<f32>,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>) -> VsOut {
    var out: VsOut;
    let world = inst.m * vec4<f32>(pos, 1.0);
    out.clip = g.view_proj * world;
    out.normal = (inst.m * vec4<f32>(normal, 0.0)).xyz;
    out.uv = uv + inst.uv.xy;
    out.world = world.xyz;
    out.tangent = vec4<f32>((inst.m * vec4<f32>(tangent.xyz, 0.0)).xyz, tangent.w);
    return out;
}

// Parallax occlusion: the texture coordinates where the view meets the relief whose height is the normal map's
// alpha (1 the top, `depth` deep at 0), stepped in layers and refined between the last two. `to_eye` is the view in
// the texture's frame (x along u, y along v, z out of the surface).
fn parallax(uv: vec2<f32>, to_eye: vec3<f32>, depth: f32, dx: vec2<f32>, dy: vec2<f32>) -> vec2<f32> {
    let layers = mix(32.0, 8.0, clamp(to_eye.z, 0.0, 1.0));
    let step_depth = 1.0 / layers;
    let shift = to_eye.xy / max(to_eye.z, 0.2) * depth * step_depth;
    var cur = uv;
    var layer = 0.0;
    var below = 1.0 - textureSampleGrad(relief, samp, cur, dx, dy).a;
    var prev = cur;
    var prev_below = below;
    for (var i = 0; i < 32; i++) {
        if layer >= below || f32(i) >= layers {
            break;
        }
        prev = cur;
        prev_below = below;
        cur = cur - shift;
        layer = layer + step_depth;
        below = 1.0 - textureSampleGrad(relief, samp, cur, dx, dy).a;
    }
    let after = below - layer;
    let before = prev_below - (layer - step_depth);
    let w = clamp(after / min(after - before, -1e-5), 0.0, 1.0);
    return mix(cur, prev, w);
}

// A texture read as the filter asks (g.fog_height.w, a view mod's `render.filter`): 0 as ever (mipmaps, anisotropic),
// 1 bilinear from the largest level only, 2 the nearest texel. The mode is the same for the whole frame.
fn sample_tex(t: texture_2d<f32>, uv: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, moved: bool) -> vec4<f32> {
    if g.fog_height.w > 1.5 {
        let d = vec2<f32>(textureDimensions(t, 0));
        let at = vec2<i32>(floor((uv - floor(uv)) * d));
        return textureLoad(t, min(at, vec2<i32>(d) - vec2<i32>(1)), 0);
    }
    if g.fog_height.w > 0.5 {
        return textureSampleLevel(t, samp, uv, 0.0);
    }
    return select(textureSample(t, samp, uv), textureSampleGrad(t, samp, uv, dx, dy), moved);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let uv_dx = dpdx(in.uv);
    let uv_dy = dpdy(in.uv);
    // The ball's shadow: the texture over a square centred under the ball, projected straight down, held at its
    // border outside (the texture's border is white), drawn as it is and multiplied into the frame by the blend.
    if inst.shadow.w > 0.5 {
        let st = clamp(vec2<f32>(0.5) + (inst.shadow.xy - in.world.xz) / inst.shadow.z, vec2<f32>(0.0), vec2<f32>(1.0));
        return textureSampleLevel(tex, samp, st, 0.0);
    }
    // The TexGen effect: Chrome maps the normal as the camera sees it, Reflect the view reflected about it (a
    // sphere map), as the fixed-function pipeline's camera-space normal and reflection coordinates do.
    var uv = in.uv;
    if mat.params.z > 0.5 {
        let nv = normalize((g.view * vec4<f32>(normalize(in.normal), 0.0)).xyz);
        if mat.params.z > 2.5 {
            uv = vec2<f32>(0.5 + 0.5 * nv.x, 0.5 - 0.5 * nv.y);
        } else {
            let e = normalize((g.view * vec4<f32>(in.world - g.eye.xyz, 0.0)).xyz);
            let r = reflect(e, nv);
            let m = 2.0 * sqrt(r.x * r.x + r.y * r.y + (r.z - 1.0) * (r.z - 1.0));
            uv = vec2<f32>(0.5 + r.x / max(m, 1e-4), 0.5 - r.y / max(m, 1e-4));
        }
    }
    // the coordinates' change across the pixel before the parallax moves them: sampling with these keeps the mip
    // level the surface's own (the moved coordinates jump at a brick's edge, and would pick the smallest mip there)
    let tex_dx = dpdx(uv);
    let tex_dy = dpdy(uv);
    // whether the parallax moved them (where it did not, the texture is sampled as it always was)
    var moved = false;
    // masonry and the like: the relief's height moves what is seen (the mesh's own coordinates only)
    if mat.ambient.a > 0.0 && mat.params.w > 0.5 && mat.params.z < 0.5 {
        // the side seen: a two-sided mesh seen from its back shows its relief as well
        let v = normalize(g.eye.xyz - in.world);
        let n0 = normalize(in.normal);
        let n = select(n0, -n0, dot(n0, v) < 0.0);
        let along = in.tangent.xyz - n * dot(n, in.tangent.xyz);
        if dot(along, along) > 1e-8 && dot(n, v) > 0.0 {
            let t = normalize(along);
            let b = cross(n, t) * in.tangent.w;
            uv = parallax(uv, vec3<f32>(dot(v, t), dot(v, b), dot(v, n)), mat.ambient.a, uv_dx, uv_dy);
            moved = true;
        }
    }
    var base = vec4<f32>(1.0);
    if mat.emissive.a > 0.5 {
        base = sample_tex(tex, uv, tex_dx, tex_dy, moved);
    }
    var diffuse = mat.diffuse;
    if inst.diffuse.a >= 0.0 {
        diffuse = inst.diffuse;
    }
    let color = base * diffuse;
    if mat.params.x >= 0.0 && color.a <= mat.params.x {
        discard;
    }
    if mat.params.y > 0.5 || inst.uv.z > 0.5 {
        return color;
    }
    // The original's fixed-function lighting (D3D7, as the C reference draws it): the lit colour is the material's
    // emissive, plus the scene's ambient times the material's ambient, plus the light's colour times the material's
    // diffuse and N.L, held to 0..1, and the texture is multiplied by it; the highlight (the light's colour times the
    // material's specular and N.H to its power, a local viewer) is added after. Two-sided: the original's meshes do
    // not agree on which side faces out, so the side facing the light is lit.
    // The surface's normal: the interpolated one, bent by the material's normal map when it has one (tangent space,
    // green towards -v as glTF has it). Without a map it is exactly the interpolated normal, as the original's.
    var surface = normalize(in.normal);
    if mat.params.w > 0.5 {
        let sampled = sample_tex(relief, uv, tex_dx, tex_dy, moved);
        let m = sampled.xyz * 2.0 - vec3<f32>(1.0);
        // the tangent, square to the normal; interpolated across a face it can vanish or lie along the normal (mirrored
        // coordinates), and then the surface stays as it is rather than turn into NaN
        let along = in.tangent.xyz - surface * dot(surface, in.tangent.xyz);
        if dot(along, along) > 1e-8 {
            let t = normalize(along);
            let b = cross(surface, t) * in.tangent.w;
            let bent = t * m.x - b * m.y + surface * m.z;
            if dot(bent, bent) > 1e-8 {
                surface = normalize(bent);
            }
        }
    }
    var n = surface;
    let l = -normalize(g.light_dir.xyz);
    if dot(n, l) < 0.0 {
        n = -n;
    }
    let lambert = max(dot(n, l), 0.0);
    // The passing lights add their own diffuse term (each lights the side facing it); the point light falls off as
    // 1/d and ends at its reach.
    let raw = surface;
    var extra = g.flash_color.rgb * abs(dot(raw, -normalize(g.flash_dir.xyz)));
    let to_point = g.point_pos.xyz - in.world;
    let d = length(to_point);
    if d < g.point_pos.w && d > 1e-3 {
        extra += g.point_color.rgb * abs(dot(raw, to_point / d)) / d;
    }
    let lit = clamp(mat.emissive.rgb + g.ambient.rgb * mat.ambient.rgb
        + (g.light_color.rgb * lambert + extra) * diffuse.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    var spec = vec3<f32>(0.0);
    if mat.specular.a > 0.0 && lambert > 0.0 {
        let v = normalize(g.eye.xyz - in.world);
        let h = normalize(l + v);
        // over parallax masonry the sheen stays the surface's own (each brick's tilt would spark like metal)
        var sn = n;
        if mat.ambient.a > 0.0 {
            sn = normalize(in.normal);
            if dot(sn, l) < 0.0 {
                sn = -sn;
            }
        }
        spec = g.light_color.rgb * mat.specular.rgb * pow(max(dot(sn, h), 0.0), mat.specular.a);
    }
    let shaded = min(base.rgb * lit + spec, vec3<f32>(1.0));
    return vec4<f32>(fogged(shaded, in.world), color.a);
}

// The air between the eye and a point: its distance thickens it, and so does its depth under the fog's height (the
// world far below fades into the haze), each as the other were clear.
fn fogged(c: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    if g.script_fog.x > 0.5 {
        let d = length(world - g.eye.xyz);
        var keep = 1.0;
        if g.script_fog.x > 2.5 {
            keep = clamp((g.script_fog.z - d) / max(g.script_fog.z - g.script_fog.y, 1e-4), 0.0, 1.0);
        } else if g.script_fog.x > 1.5 {
            let e = g.script_fog.w * d;
            keep = exp(-e * e);
        } else {
            keep = exp(-g.script_fog.w * d);
        }
        return mix(g.script_fog_color.rgb, c, keep);
    }
    if g.fog_height.z < 0.5 {
        return c;
    }
    let ray = world - g.eye.xyz;
    let dist = length(ray);
    let far = 1.0 - exp(-g.fog_color.a * dist);
    let deep = clamp((g.fog_height.x - world.y) / max(g.fog_height.y, 0.01), 0.0, 1.0);
    let f = 1.0 - (1.0 - far) * (1.0 - deep * deep);
    // the air's colour is what lies behind the point: the haze towards the horizon, the world below further down
    let down = smoothstep(0.05, 0.7, -ray.y / max(dist, 1e-4));
    return mix(c, mix(g.fog_color.rgb, g.fog_down.rgb, down), f);
}
