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
};

struct Material {
    diffuse: vec4<f32>,
    // rgb: emissive; a: 1 when a texture is bound.
    emissive: vec4<f32>,
    // x: alpha cutoff (negative: no alpha test); y: 1 unlit; z: texture coordinates from the view (2 Reflect,
    // 3 Chrome; 0 the mesh's).
    params: vec4<f32>,
    // rgb: the material's ambient colour.
    ambient: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
struct Instance {
    m: mat4x4<f32>,
    // xy: added to the texture coordinates (a script's Texture Scroller); z: 1 to draw unlit (sprites).
    uv: vec4<f32>,
    // The diffuse colour instead of the material's; a < 0: the material's.
    diffuse: vec4<f32>,
};

@group(1) @binding(0) var<uniform> inst: Instance;
@group(2) @binding(0) var<uniform> mat: Material;
@group(2) @binding(1) var tex: texture_2d<f32>;
@group(2) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>) -> VsOut {
    var out: VsOut;
    let world = inst.m * vec4<f32>(pos, 1.0);
    out.clip = g.view_proj * world;
    out.normal = (inst.m * vec4<f32>(normal, 0.0)).xyz;
    out.uv = uv + inst.uv.xy;
    out.world = world.xyz;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
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
    var base = vec4<f32>(1.0);
    if mat.emissive.a > 0.5 {
        base = textureSample(tex, samp, uv);
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
    // emissive, plus the scene's ambient times the material's ambient, plus the light times the material's diffuse
    // and N.L, held to 0..1, and the texture is multiplied by it. Two-sided: the original's meshes do not agree on
    // which side faces out.
    let n = normalize(in.normal);
    let lambert = abs(dot(n, -g.light_dir.xyz));
    let lit = clamp(mat.emissive.rgb + g.ambient.rgb * mat.ambient.rgb + diffuse.rgb * lambert, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(base.rgb * lit, color.a);
}
