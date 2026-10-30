// Level geometry: textured, lit with one directional light and ambient as the original's fixed-function
// pipeline does, with the material's alpha test. Lighting is per pixel with the interpolated normal.

struct Globals {
    view_proj: mat4x4<f32>,
    // xyz: direction the light travels; w unused.
    light_dir: vec4<f32>,
    // rgb: ambient light; a: unused.
    ambient: vec4<f32>,
};

struct Material {
    diffuse: vec4<f32>,
    // rgb: emissive; a: 1 when a texture is bound.
    emissive: vec4<f32>,
    // x: alpha cutoff (negative: no alpha test).
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;
@group(2) @binding(0) var<uniform> mat: Material;
@group(2) @binding(1) var tex: texture_2d<f32>;
@group(2) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>) -> VsOut {
    var out: VsOut;
    let world = model * vec4<f32>(pos, 1.0);
    out.clip = g.view_proj * world;
    out.normal = (model * vec4<f32>(normal, 0.0)).xyz;
    out.uv = uv;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    var base = vec4<f32>(1.0);
    if mat.emissive.a > 0.5 {
        base = textureSample(tex, samp, in.uv);
    }
    let color = base * mat.diffuse;
    if mat.params.x >= 0.0 && color.a <= mat.params.x {
        discard;
    }
    if mat.params.y > 0.5 {
        return color;
    }
    // Two-sided lighting: the original's meshes do not agree on which side faces out.
    let n = normalize(in.normal);
    let lambert = abs(dot(n, -g.light_dir.xyz));
    let light = g.ambient.rgb + (vec3<f32>(1.0) - g.ambient.rgb) * lambert;
    return vec4<f32>(color.rgb * light + mat.emissive.rgb, color.a);
}
