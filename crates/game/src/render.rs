// The renderer: uploads level models (meshes, materials, textures) once and draws a list of instances each
// frame, to a window surface or to an offscreen texture (screenshots, later video). Any size and aspect ratio:
// the projection always uses the real one.

use crate::particles::Blend as PBlend;
use glam::{Mat4, Vec3};
use level::{Blend, Level};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const UNIFORM_ALIGN: u64 = 256;

/// Anti-aliasing is always on (4 samples a pixel; fewer where the adapter cannot): the most the adapter does for the
/// formats the game draws with (1, 2 or 4). The mod back-to-2004 turns it off with the mipmaps (`RenderLook::filter`).
static MSAA_MAX: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
const MSAA_WANTED: u32 = 4;

/// Tells the renderer what the adapter can do (before the first frame).
pub fn note_adapter(adapter: &wgpu::Adapter) {
    let ok = |f: wgpu::TextureFormat, n: u32| {
        adapter
            .get_texture_format_features(f)
            .flags
            .sample_count_supported(n)
    };
    let both = |n: u32| {
        [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
            DEPTH_FORMAT,
        ]
        .into_iter()
        .all(|f| ok(f, n))
    };
    let max = [4, 2].into_iter().find(|&n| both(n)).unwrap_or(1);
    MSAA_MAX.store(max, std::sync::atomic::Ordering::Relaxed);
}

/// The samples a pixel to draw with: 4 down to what the adapter does, none for the 2004 look.
fn samples_now(look: &RenderLook) -> u32 {
    if look.filter != 0 {
        return 1;
    }
    let max = MSAA_MAX.load(std::sync::atomic::Ordering::Relaxed);
    [4, 2, 1]
        .into_iter()
        .find(|&n| n <= MSAA_WANTED && n <= max)
        .unwrap_or(1)
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    /// The direction the texture's u grows along the surface, and (w) which way its v goes against it: the frame a
    /// normal map's relief is read in.
    tangent: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
    ambient: [f32; 4],
    view: [[f32; 4]; 4],
    eye: [f32; 4],
    light_color: [f32; 4],
    flash_dir: [f32; 4],
    flash_color: [f32; 4],
    point_pos: [f32; 4],
    point_color: [f32; 4],
    /// The level's fog (level::Fog): rgb its colour, a its density a unit of distance; x the height it begins below,
    /// y the depth under that where it is whole, z 1 when there is fog.
    fog_color: [f32; 4],
    fog_height: [f32; 4],
    /// The fog's colour seen straight down (rgb).
    fog_down: [f32; 4],
    /// A script's fog (Set Fog): x the mode (0 none, 1 exp, 2 exp2, 3 linear), y the start, z the end, w the density.
    script_fog: [f32; 4],
    script_fog_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MaterialUniform {
    diffuse: [f32; 4],
    emissive: [f32; 4],
    params: [f32; 4],
    ambient: [f32; 4],
    /// rgb: the highlight's colour; a: its power (0: none).
    specular: [f32; 4],
    /// x: the alpha test's function (Virtools' 1..8).
    funcs: [f32; 4],
}

/// The scene's light as the original sets it (Gameplay.nmo's Light_Ingame, a directional light, and the scene
/// ambient of base.cmo): where it travels, its colour (the level table's Light) and the ambient.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub dir: Vec3,
    pub color: [f32; 3],
    pub ambient: [f32; 3],
}

impl Default for Light {
    fn default() -> Light {
        Light {
            dir: Vec3::new(0.3022, -0.7331, 0.6092),
            color: [1.0; 3],
            // 0x0F of 0xFF
            ambient: [15.0 / 255.0; 3],
        }
    }
}

/// A range of a shared buffer (a model's meshes live in a few large buffers: one buffer each took seconds to create).
#[derive(Clone)]
struct BufSlice {
    buf: wgpu::Buffer,
    off: u64,
    len: u64,
}

impl BufSlice {
    fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.buf.slice(self.off..self.off + self.len)
    }
}

/// A mesh's bytes before they are on the card: vertices, and per part its indices (and its channel's vertices).
struct MeshCpu {
    vertex: Vec<u8>,
    parts: Vec<PartCpu>,
}

struct PartCpu {
    index: Vec<u8>,
    count: u32,
    material: usize,
    channel: Option<(Vec<u8>, PipelineKey)>,
}

/// The most bytes one shared buffer takes (a model past it goes on in the next buffer).
const POOL_BYTES: usize = 64 << 20;

struct GpuPart {
    index: BufSlice,
    count: u32,
    material: usize,
    /// A material channel: its own vertices (the mesh's with the channel's coordinates) and blend, drawn over the
    /// mesh without writing depth.
    channel: Option<(BufSlice, PipelineKey)>,
}

struct GpuMesh {
    vertex: BufSlice,
    parts: Vec<GpuPart>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PipelineKey {
    blend: Option<(Blend, Blend)>,
    z_write: bool,
    /// Virtools' compare function of the depth test (1..8); the level's own is 4, "less or equal".
    z_func: u8,
}

/// The depth test the level's materials have, and the alpha test's function (greater): what drawing did before scripts.
pub const Z_FUNC_DEFAULT: u8 = 4;
pub const ALPHA_FUNC_DEFAULT: u8 = 5;
/// At most this many pipelines are kept; past it a script's depth function falls back to the default one.
const MAX_PIPELINES: usize = 96;

/// The depth test of Virtools' compare function number (1 never .. 8 always); a number out of range: less or equal.
fn compare(func: u8) -> wgpu::CompareFunction {
    match func {
        1 => wgpu::CompareFunction::Never,
        2 => wgpu::CompareFunction::Less,
        3 => wgpu::CompareFunction::Equal,
        5 => wgpu::CompareFunction::Greater,
        6 => wgpu::CompareFunction::NotEqual,
        7 => wgpu::CompareFunction::GreaterEqual,
        8 => wgpu::CompareFunction::Always,
        _ => wgpu::CompareFunction::LessEqual,
    }
}

/// How much a script sky must shrink for its farthest corner to be inside the far plane (1: not at all).
fn sky_scale(spec: &script::SkySpec, far: f32) -> f32 {
    let n = spec.sides.len().clamp(4, 16) as f32;
    let corner = spec.radius / (std::f32::consts::PI / n).cos();
    let half_h = if spec.quadratic {
        spec.radius * (std::f32::consts::PI / n).tan()
    } else {
        spec.side_height / 2.0
    };
    let bound = corner.hypot(spec.y.abs() + half_h);
    if bound.is_finite() && bound > 1e-3 && far.is_finite() {
        (far * 0.9 / bound).min(1.0)
    } else {
        1.0
    }
}

#[derive(Clone)]
struct GpuMaterial {
    bind: wgpu::BindGroup,
    key: PipelineKey,
}

/// One uploaded model (a level, or the balls).
pub struct Model {
    meshes: Vec<Option<GpuMesh>>,
    materials: Vec<GpuMaterial>,
    /// What the materials draw with, kept so a script's change of a material can rebuild it.
    textures: Vec<Option<wgpu::TextureView>>,
    keyed: Vec<bool>,
    /// The scripts' changes now drawn, by material (the level's own look otherwise).
    applied: std::collections::BTreeMap<usize, crate::matstate::MatOverride>,
    /// The textures a script wrote into that are drawn now: by texture, the copy's stamp and the level's own view
    /// that stands back in when the copy is gone (a rewind).
    written: std::collections::BTreeMap<usize, (u64, Option<wgpu::TextureView>)>,
}

/// Which of a run's texture copies the renderer must upload (new, or changed since: by stamp) and which of the
/// textures it draws a copy of must go back to the level's own (the copy is gone). `copies`: the run's, by texture,
/// with their stamps; `drawn`: what is drawn now, likewise.
pub(crate) fn texture_sync(
    copies: &std::collections::BTreeMap<usize, u64>,
    drawn: &std::collections::BTreeMap<usize, u64>,
) -> (Vec<usize>, Vec<usize>) {
    let upload = copies
        .iter()
        .filter(|(t, s)| drawn.get(t) != Some(s))
        .map(|(&t, _)| t)
        .collect();
    let restore = drawn
        .keys()
        .copied()
        .filter(|t| !copies.contains_key(t))
        .collect();
    (upload, restore)
}

/// One thing to draw this frame.
pub struct Instance {
    pub model: usize,
    pub mesh: usize,
    pub matrix: Mat4,
    /// Draw the mesh as a translucent, unlit marker of this colour instead of its own materials.
    pub marker: Option<[f32; 4]>,
    /// Added to the mesh's texture coordinates (a script's Texture Scroller).
    pub uv_offset: [f32; 2],
    /// In place of its materials' diffuse colour (a script's Set Diffuse).
    pub diffuse: Option<[f32; 4]>,
    /// Drawn without lighting, in its material's diffuse colour (sprites, as the original draws them).
    pub unlit: bool,
    /// The ball's shadow pass over this floor mesh: its centre (world x, z) and side.
    pub shadow: Option<[f32; 3]>,
    /// Materials this one thing draws in place of its mesh's own (a script's Set Material): (the level's, the
    /// replacement). Empty for nearly everything.
    pub swaps: Vec<(usize, usize)>,
}

impl Instance {
    /// The material index a part of this instance is drawn with.
    fn material_of(&self, part_material: usize) -> usize {
        self.swaps
            .iter()
            .find(|(from, _)| *from == part_material)
            .map_or(part_material, |(_, to)| *to)
    }

    /// A mesh drawn with its own materials as they are.
    pub fn plain(model: usize, mesh: usize, matrix: Mat4) -> Instance {
        Instance {
            model,
            mesh,
            matrix,
            marker: None,
            uv_offset: [0.0; 2],
            diffuse: None,
            unlit: false,
            shadow: None,
            swaps: Vec::new(),
        }
    }
}

/// The per-instance uniform: the model matrix, then (u, v) added to the texture coordinates, then the diffuse
/// colour to use instead of the material's (alpha below 0: the material's own).
const INSTANCE_BYTES: usize = 112;

pub struct View {
    pub eye: Vec3,
    pub target: Vec3,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct OverlayVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

/// How bright a material is drawn whose texture nobody has: a mid grey of its colour, as an unlit board would be.
const ABSENT_TEXTURE_TONE: f32 = 0.55;

fn overlay_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("overlay"),
        source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("overlay"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<OverlayVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
            })],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// What view mods ask of the picture's look (`render.*` actions); the default is ours, untouched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderLook {
    /// The size the picture is drawn at before it is scaled up to the window (nearest), if lower than the window.
    pub scale: Option<(u32, u32)>,
    /// The picture's shape (width, height): the scaled picture keeps it, with black bars.
    pub aspect: Option<(f32, f32)>,
    /// 0 as ours, 1 bilinear without mipmaps, 2 nearest.
    pub filter: u8,
    /// Frames a second at most (0: none).
    pub fps_cap: u32,
    /// 16: the colour cut to 5-6-5 bits, no dither.
    pub colour_bits: u8,
    /// The sacks' supports drawn: 0 none, `supports::PLAIN` or `supports::VARIED`.
    pub supports: u8,
    /// The level drawn as its original has it: no rounded looks, the textures before the toolbox grew them.
    pub plain: bool,
}

impl Default for RenderLook {
    fn default() -> RenderLook {
        RenderLook {
            scale: None,
            aspect: None,
            filter: 0,
            fps_cap: 0,
            colour_bits: 0,
            supports: 0,
            plain: false,
        }
    }
}

impl RenderLook {
    /// The picture must go through the scaling pass (a lower size, bars, or banded colour).
    pub fn post(&self) -> bool {
        self.scale.is_some() || self.aspect.is_some() || self.colour_bits == 16
    }

    /// The size to draw at for a window of `w` x `h`: the asked size (never larger than the window); with only a
    /// shape asked, the window's part that shape takes; else the window's.
    pub fn size(&self, w: u32, h: u32) -> (u32, u32) {
        match self.scale {
            Some((sw, sh)) if sw > 0 && sh > 0 => (sw.min(w.max(1)), sh.min(h.max(1))),
            _ => {
                let p = self.placed(w, h);
                (p.2, p.3)
            }
        }
    }

    /// Where the drawn picture lands in a window of `w` x `h`: x, y, width, height (centred, in its shape).
    pub fn placed(&self, w: u32, h: u32) -> (u32, u32, u32, u32) {
        let (w, h) = (w.max(1), h.max(1));
        let shape = match self.aspect {
            Some((a, b)) if a > 0.0 && b > 0.0 => a / b,
            _ => return (0, 0, w, h),
        };
        let (mut pw, mut ph) = (w, (w as f32 / shape).round() as u32);
        if ph > h {
            (pw, ph) = ((h as f32 * shape).round() as u32, h);
        }
        let (pw, ph) = (pw.clamp(1, w), ph.clamp(1, h));
        ((w - pw) / 2, (h - ph) / 2, pw, ph)
    }
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    material_layout: wgpu::BindGroupLayout,
    /// The scene's light (the level's colour once a level is loaded).
    pub light: Light,
    /// The last level's lightning (lights.rs): its brightness, 0 when it is hidden.
    pub flash: f32,
    /// The new ball's light (lights.rs): where it is and its colour, while it shows.
    pub point: Option<(Vec3, [f32; 3])>,
    /// The level's fog (its settings); none for the original's levels.
    pub fog: Option<level::Fog>,
    /// What a script's Set Fog set for this run: none when no script did, `Some(None)` when it switched fog off (the
    /// level's own fog too).
    pub script_fog: Option<Option<script::FogSpec>>,
    /// The sky a script built (its model and mesh), drawn first with no depth, behind everything.
    script_sky: Option<(usize, usize)>,
    /// The spec the script sky was built from, and whether any of it is drawn.
    script_sky_spec: Option<(script::SkySpec, bool)>,
    /// The level's sea of cloud (clouds.rs) and how finely it is drawn; none for the original's levels.
    pub clouds: Option<crate::clouds::Layer>,
    /// The frame shows another scene than the level (the menu's): neither the level's sea of cloud nor its fog.
    pub menu_frame: bool,
    /// The cloud pass, made the first time a level has clouds (its noise takes a moment).
    cloud_pass: Option<crate::clouds::Pass>,
    /// The clouds' clock (their drift).
    started: std::time::Instant,
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    models_buf: wgpu::Buffer,
    models_bind: wgpu::BindGroup,
    models_capacity: u64,
    model_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    /// The normal map of a material without one: the surface's own normal.
    flat_normal: wgpu::TextureView,
    /// The card draws BC-compressed textures (every desktop card; not every phone's or old Mac's).
    pub bc: bool,
    /// The compressed textures of this level already on the card, by their data's hash, format, size and mip count.
    shared_textures: HashMap<(u64, gamedata::gputex::Format, (u32, u32), usize), wgpu::TextureView>,
    pub texture_stats: TextureStats,
    pub models: Vec<Model>,
    overlay_pipeline: wgpu::RenderPipeline,
    markers: HashMap<[u32; 4], GpuMaterial>,
    /// The ball's shadow (TT Simple Shadow's material: the HardShadow texture as it is, multiplied into the frame).
    shadow_material: Option<GpuMaterial>,
    /// Particles: their pipelines by blend and lines, their layout, and their textures by name.
    particle_shader: wgpu::ShaderModule,
    particle_layout: wgpu::PipelineLayout,
    particle_tex_layout: wgpu::BindGroupLayout,
    particle_pipelines: HashMap<(PBlend, bool), wgpu::RenderPipeline>,
    named_textures: HashMap<&'static str, wgpu::BindGroup>,
    particle_white: wgpu::BindGroup,
    /// The original's 2D interface: its pipeline over the frame (named textures, alpha blended, no depth).
    ui_pipeline: wgpu::RenderPipeline,
    clamp: wgpu::Sampler,
    /// The samples a pixel the pipelines, the depth and the colour target were made for.
    samples: u32,
    msaa: Option<(wgpu::TextureView, u32, u32)>,
    /// How a view mod wants the picture to look (a 2004 PC's: low resolution, plain filtering, banded colour).
    pub look: RenderLook,
    /// Whether the level on the card was uploaded plain (`RenderLook::plain` at that time); a change draws it again.
    pub plain_uploaded: bool,
    /// The colour behind everything (the sky's zenith).
    pub clear: wgpu::Color,
    depth: Option<(wgpu::Texture, wgpu::TextureView, u32, u32)>,
    /// The scaling pass of `look` (retro.rs), made when first needed.
    pub retro: Option<crate::retro::Retro>,
    /// The title and loading screens' picture (titlescene.rs), made when first shown.
    title: Option<crate::titlescene::TitleScene>,
    /// The picture's pipeline being made on a thread (`prepare_title`), and when the picture was first there.
    title_coming: Option<std::sync::mpsc::Receiver<crate::titlescene::TitleScene>>,
    title_since: Option<std::time::Instant>,
}

/// A range in shared chunk `.0`: its offset and length.
type Range = (usize, u64, u64);

struct MeshPlan {
    vertex: Range,
    parts: Vec<(Range, u32, usize, Option<(Range, PipelineKey)>)>,
}

/// Meshes packed into chunks of bytes (each to become one buffer), with where each mesh's pieces lie. Made without
/// the card, so a loading thread can do it.
pub struct PackedMeshes {
    chunks: Vec<Vec<u8>>,
    plans: Vec<Option<MeshPlan>>,
}

impl PackedMeshes {
    /// The meshes once the chunks are buffers (`buffers[i]` holds chunk `i`).
    fn assemble(self, buffers: Vec<wgpu::Buffer>) -> Vec<Option<GpuMesh>> {
        let slice = |(c, off, len): Range| BufSlice {
            buf: buffers[c].clone(),
            off,
            len,
        };
        self.plans
            .into_iter()
            .map(|p| {
                p.map(|p| GpuMesh {
                    vertex: slice(p.vertex),
                    parts: p
                        .parts
                        .into_iter()
                        .map(|(index, count, material, channel)| GpuPart {
                            index: slice(index),
                            count,
                            material,
                            channel: channel.map(|(r, k)| (slice(r), k)),
                        })
                        .collect(),
                })
            })
            .collect()
    }
}

fn pack_meshes(meshes: Vec<Option<MeshCpu>>) -> PackedMeshes {
    let mut chunks: Vec<Vec<u8>> = vec![Vec::new()];
    // a range in chunk `c`, to be turned into a slice once the chunk is a buffer
    let mut put = |data: &[u8]| -> Range {
        if chunks
            .last()
            .is_some_and(|c| !c.is_empty() && c.len() + data.len() > POOL_BYTES)
        {
            chunks.push(Vec::new());
        }
        let c = chunks.len() - 1;
        let last = &mut chunks[c];
        let off = last.len() as u64;
        last.extend_from_slice(data);
        while last.len() % 4 != 0 {
            last.push(0);
        }
        (c, off, data.len() as u64)
    };
    let plans: Vec<Option<MeshPlan>> = meshes
        .iter()
        .map(|m| {
            m.as_ref().map(|m| MeshPlan {
                vertex: put(&m.vertex),
                parts: m
                    .parts
                    .iter()
                    .map(|p| {
                        (
                            put(&p.index),
                            p.count,
                            p.material,
                            p.channel.as_ref().map(|c| (put(&c.0), c.1)),
                        )
                    })
                    .collect(),
            })
        })
        .collect();
    PackedMeshes { chunks, plans }
}

/// What a model's meshes are as bytes, ready to go to the card (the part of the upload that needs no card): made
/// by a loading thread. `material_count` is the model's number of materials.
pub fn prepare_meshes(level: &Level) -> PackedMeshes {
    let default_material = level.materials.len();
    pack_meshes(
        level
            .meshes
            .iter()
            .map(|m| m.look.as_deref().unwrap_or(m))
            .map(|m| Renderer::gpu_mesh_cpu(m, default_material))
            .collect(),
    )
}

/// The textures a model draws with that are to be read: planned one after another (the level's budget is taken in
/// its own order).
pub fn texture_plans(
    level: &Level,
    load: &TextureLoader<'_>,
    bc: bool,
) -> Vec<(usize, gamedata::assets::TexturePlan)> {
    // only the textures a material draws with are loaded (a level may list textures it never uses: the loader
    // points a texture named twice at one of them)
    let mut used: std::collections::HashSet<usize> = level
        .materials
        .iter()
        .flat_map(|m| m.texture.into_iter().chain(m.normal))
        .collect();
    // a map whose scripts set textures may set any of its own (the loader's budget still applies)
    if level
        .scripts
        .as_ref()
        .is_some_and(|g| g.uses_proto(script::SET_TEXTURE))
    {
        used.extend(0..level.textures.len());
    }
    level
        .textures
        .iter()
        .enumerate()
        .filter(|(i, _)| used.contains(i))
        .filter_map(|(i, t)| load(t, bc).map(|p| (i, p)))
        .collect()
}

/// How many bytes of a texture a loading thread writes at once: each write has its own cost, and its time holds up
/// no frame.
const BAND_BYTES: usize = 512 << 10;

/// A texture on its way up, written band by band so a frame can take a few of them and go on.
pub struct TexJob {
    tex: wgpu::Texture,
    data: gamedata::gputex::TextureData,
    block: u32,
    mip: usize,
    /// The next row (in pixels, a whole number of blocks) of the level to write.
    row: u32,
    /// The bytes written at once.
    band: usize,
}

impl TexJob {
    /// Writes bands of the texture until `deadline` (at least one); true when all are written.
    fn bands(&mut self, queue: &wgpu::Queue, deadline: std::time::Instant) -> bool {
        loop {
            let Some(m) = self.data.mips.get(self.mip) else {
                break;
            };
            let block = self.block;
            let (w, h) = (
                m.width.div_ceil(block) * block,
                m.height.div_ceil(block) * block,
            );
            let row_bytes = (w / block) as usize * self.data.format.block_bytes();
            let rows_at_once = (self.band / row_bytes.max(1)).max(1) as u32;
            let first = self.row / block;
            let rows = rows_at_once.min(h / block - first);
            if let Some(level) = self.data.level(m) {
                let from = first as usize * row_bytes;
                let to = (first + rows) as usize * row_bytes;
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.tex,
                        mip_level: self.mip as u32,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: self.row,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    level.get(from..to).unwrap_or_default(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some((row_bytes) as u32),
                        rows_per_image: Some(rows),
                    },
                    wgpu::Extent3d {
                        width: w,
                        height: rows * block,
                        depth_or_array_layers: 1,
                    },
                );
            }
            self.row += rows * block;
            if self.row >= h {
                self.row = 0;
                self.mip += 1;
            }
            // the card does the copying at the submit: each band is sent, so the time is spent where it is counted
            queue.submit([]);
            if std::time::Instant::now() >= deadline {
                break;
            }
        }
        self.mip >= self.data.mips.len()
    }
}

/// The steps of one model's upload after its textures are on the card: materials, the meshes' buffers, pipelines.
/// `step` does what fits before `deadline` and says when the model is done (`None` deadline: all at once).
pub struct ModelBuild {
    src: Vec<level::Material>,
    textures: Vec<Option<wgpu::TextureView>>,
    keyed: Vec<bool>,
    same: std::collections::HashMap<String, GpuMaterial>,
    materials: Vec<GpuMaterial>,
    packed: Option<PackedMeshes>,
    buffers: Vec<wgpu::Buffer>,
    meshes: Option<Vec<Option<GpuMesh>>>,
    keys: Vec<PipelineKey>,
    pipelines: usize,
    particles: usize,
    /// The longest single step so far.
    worst: std::time::Duration,
}

impl ModelBuild {
    /// `packed`: the meshes' bytes when a loading thread made them, else they are made here.
    pub fn new(
        level: &Level,
        textures: Vec<Option<wgpu::TextureView>>,
        packed: Option<PackedMeshes>,
    ) -> ModelBuild {
        ModelBuild {
            src: level.materials.clone(),
            keyed: level
                .textures
                .iter()
                .map(|t| t.color_key.is_some())
                .collect(),
            textures,
            same: Default::default(),
            materials: Vec::new(),
            packed: Some(packed.unwrap_or_else(|| prepare_meshes(level))),
            buffers: Vec::new(),
            meshes: None,
            keys: Vec::new(),
            pipelines: 0,
            particles: 0,
            worst: Default::default(),
        }
    }

    pub fn step(
        &mut self,
        r: &mut Renderer,
        deadline: Option<std::time::Instant>,
    ) -> Option<Model> {
        // a step is not begun when the longest step seen so far would not end before the deadline
        let mut last = std::time::Instant::now();
        let mut worst = self.worst;
        // (the first step of a call always runs, or a step longer than the slice would never be taken)
        let mut begun = false;
        let mut late = |d: Option<std::time::Instant>| {
            let now = std::time::Instant::now();
            if begun {
                worst = worst.max(now - last);
            }
            last = now;
            let stop = begun && d.is_some_and(|d| now + worst >= d);
            begun = true;
            stop
        };
        // materials alike in all they draw with share one buffer and bind group (a map may carry a thousand copies
        // of one: each cost 4 ms on a software adapter)
        while self.materials.len() < self.src.len() {
            if late(deadline) {
                self.worst = worst;
                return None;
            }
            let m = self.src[self.materials.len()].clone();
            let mut same = m.clone();
            same.name.clear();
            let g = match self.same.get(&format!("{same:?}")) {
                Some(g) => g.clone(),
                None => {
                    let g = r.gpu_material(
                        &m,
                        &self.textures,
                        &self.keyed,
                        (Z_FUNC_DEFAULT, ALPHA_FUNC_DEFAULT),
                    );
                    self.same.insert(format!("{same:?}"), g.clone());
                    g
                }
            };
            self.materials.push(g);
        }
        if self.materials.len() == self.src.len() {
            // A part without a material is drawn with a plain white one, kept at the end.
            self.materials
                .push(r.flat_material([0.8, 0.8, 0.8, 1.0], false));
        }
        // the meshes' bytes go up one shared buffer at a time
        while self.meshes.is_none() {
            if late(deadline) {
                self.worst = worst;
                return None;
            }
            let Some(p) = &self.packed else { break };
            if let Some(chunk) = p.chunks.get(self.buffers.len()) {
                let b = r.mesh_buffer(chunk);
                self.buffers.push(b);
            } else if let Some(p) = self.packed.take() {
                self.meshes = Some(p.assemble(std::mem::take(&mut self.buffers)));
            }
        }
        if self.keys.is_empty() && self.pipelines == 0 {
            let mut keys: Vec<PipelineKey> = self.materials.iter().map(|m| m.key).collect();
            keys.sort_by_key(|k| {
                (
                    k.blend.map(|(a, b)| (a as u8, b as u8)),
                    k.z_write,
                    k.z_func,
                )
            });
            keys.dedup();
            keys.truncate(MAX_PIPELINES);
            self.keys = keys;
            self.pipelines = 1;
        }
        // the pipelines the model's materials draw with are made now, not in the frame they first show (a hitch
        // each)
        while self.pipelines - 1 < self.keys.len() {
            if late(deadline) {
                self.worst = worst;
                return None;
            }
            r.model_pipeline(self.keys[self.pipelines - 1]);
            self.pipelines += 1;
        }
        let kinds = [PBlend::AlphaAdd, PBlend::OneSrcColor, PBlend::Alpha];
        while self.particles < kinds.len() * 2 {
            if late(deadline) {
                self.worst = worst;
                return None;
            }
            let (b, lines) = (kinds[self.particles / 2], self.particles % 2 == 1);
            r.particle_pipeline((b, lines));
            self.particles += 1;
        }
        Some(Model {
            meshes: self.meshes.take().unwrap_or_default(),
            materials: std::mem::take(&mut self.materials),
            textures: std::mem::take(&mut self.textures),
            keyed: std::mem::take(&mut self.keyed),
            applied: Default::default(),
            written: Default::default(),
        })
    }
}

/// Puts textures on the card from any thread (the device and the queue may be used from several): a loading thread
/// reads a texture and has it uploaded in the same breath, the frames of the window going on meanwhile.
pub struct TextureUploader {
    device: wgpu::Device,
    queue: wgpu::Queue,
    shared: std::collections::HashMap<
        (u64, gamedata::gputex::Format, (u32, u32), usize),
        wgpu::TextureView,
    >,
    pub stats: TextureStats,
}

impl TextureUploader {
    pub fn new(r: &Renderer) -> TextureUploader {
        TextureUploader {
            device: r.device.clone(),
            queue: r.queue.clone(),
            shared: Default::default(),
            stats: TextureStats::default(),
        }
    }

    /// What comes next shares no texture with what went up before.
    pub fn end_sharing(&mut self) {
        self.shared.clear();
    }

    pub fn upload(&mut self, data: gamedata::gputex::TextureData) -> Option<wgpu::TextureView> {
        let key = data
            .hash
            .map(|h| (h, data.format, data.size(), data.mips.len()));
        if let Some(view) = key.and_then(|k| self.shared.get(&k)) {
            self.stats.shared += 1;
            return Some(view.clone());
        }
        let started = std::time::Instant::now();
        let (tex, block) = create_texture_for(&self.device, &data)?;
        let mut job = TexJob {
            tex,
            data,
            block,
            mip: 0,
            row: 0,
            band: BAND_BYTES,
        };
        let far = started + std::time::Duration::from_secs(3600);
        job.bands(&self.queue, far);
        let view = job.tex.create_view(&Default::default());
        self.stats.upload += started.elapsed();
        self.stats.uploaded += 1;
        self.stats.bytes += job.data.bytes();
        if job.data.format != gamedata::gputex::Format::Rgba {
            self.stats.compressed += 1;
        }
        if let Some(k) = key {
            self.shared.insert(k, view.clone());
        }
        Some(view)
    }
}

impl Renderer {
    /// The models a loading built take the place of the old ones, except the model at `keep` (the menu's scene),
    /// which goes after them; its new place.
    pub fn replace_models(&mut self, new: Vec<Model>, keep: Option<usize>) -> Option<usize> {
        let old = std::mem::replace(&mut self.models, new);
        let kept = keep.and_then(|k| old.into_iter().nth(k))?;
        self.models.push(kept);
        Some(self.models.len() - 1)
    }

    /// Models built in slices put at the end; the first one's index.
    pub fn append_models(&mut self, new: Vec<Model>) -> usize {
        let first = self.models.len();
        self.models.extend(new);
        first
    }
}

/// Plans a texture's loading (the caller looks its file up and counts it; `bool`: the card draws BC-compressed
/// textures); the renderer then reads the plans, several at a time.
pub type TextureLoader<'a> =
    dyn Fn(&level::Texture, bool) -> Option<gamedata::assets::TexturePlan> + 'a;

/// How many textures are read at a time, each on its own thread (a 4K picture decoded is 85 MB while it waits).
pub const LOADS_AT_ONCE: usize = 8;

/// What the textures of a level cost to load so far: said after each level's upload.
#[derive(Clone, Copy, Default, Debug)]
pub struct TextureStats {
    pub uploaded: usize,
    pub shared: usize,
    pub compressed: usize,
    pub bytes: usize,
    pub read: std::time::Duration,
    pub upload: std::time::Duration,
}

impl Renderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Renderer {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("level"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let uniform_entry = |binding, dynamic: bool, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: None,
            },
            count: None,
        };
        let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[uniform_entry(0, false, vf)],
        });
        let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model"),
            entries: &[uniform_entry(0, true, wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[
                uniform_entry(0, false, wgpu::ShaderStages::FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // the material's normal map (a flat one when it has none)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("level"),
            bind_group_layouts: &[
                Some(&globals_layout),
                Some(&model_layout),
                Some(&material_layout),
            ],
            immediate_size: 0,
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let (models_buf, models_bind) = model_buffer(&device, &model_layout, 1024);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("level"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        let overlay_pipeline = overlay_pipeline(&device, format);
        let white = upload_texture(&device, &queue, 1, 1, vec![255; 4]);
        let flat_normal = upload_texture(&device, &queue, 1, 1, vec![128, 128, 255, 255]);
        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particles"),
            source: wgpu::ShaderSource::Wgsl(include_str!("particles.wgsl").into()),
        });
        let particle_tex_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("particle texture"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let particle_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particles"),
            bind_group_layouts: &[Some(&globals_layout), Some(&particle_tex_layout)],
            immediate_size: 0,
        });
        let particle_white = particle_bind(&device, &particle_tex_layout, &white, &sampler);
        let clamp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("clamp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let ui_pipeline = ui_pipeline(&device, &particle_tex_layout, format);
        let bc = device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        Renderer {
            device,
            queue,
            format,
            shader,
            layout,
            pipelines: HashMap::new(),
            material_layout,
            light: Light::default(),
            flash: 0.0,
            fog: None,
            script_fog: None,
            script_sky: None,
            script_sky_spec: None,
            clouds: None,
            menu_frame: false,
            cloud_pass: None,
            started: std::time::Instant::now(),
            point: None,
            globals,
            globals_bind,
            models_buf,
            models_bind,
            models_capacity: 1024,
            model_layout,
            sampler,
            white,
            flat_normal,
            bc,
            shared_textures: HashMap::new(),
            texture_stats: TextureStats::default(),
            models: Vec::new(),
            overlay_pipeline,
            markers: HashMap::new(),
            shadow_material: None,
            particle_shader,
            particle_layout,
            particle_tex_layout,
            particle_pipelines: HashMap::new(),
            named_textures: HashMap::new(),
            particle_white,
            ui_pipeline,
            clamp,
            samples: 1,
            msaa: None,
            look: RenderLook::default(),
            plain_uploaded: false,
            clear: wgpu::Color {
                r: 0.4,
                g: 0.45,
                b: 0.6,
                a: 1.0,
            },
            depth: None,
            retro: None,
            title: None,
            title_coming: None,
            title_since: None,
        }
    }

    /// The GPU side of one material: its uniform and bind group, with the textures it draws with (by the level's
    /// texture index; `keyed` marks textures with a transparent colour). Used for the level's own materials and
    /// again for any a script changes.
    fn gpu_material(
        &self,
        m: &level::Material,
        textures: &[Option<wgpu::TextureView>],
        keyed: &[bool],
        funcs: (u8, u8),
    ) -> GpuMaterial {
        let view = m
            .texture
            .and_then(|t| textures.get(t))
            .and_then(Option::as_ref);
        let relief = m
            .normal
            .and_then(|t| textures.get(t))
            .and_then(Option::as_ref);
        // a texture the material draws with that nobody has (a stranger's map naming a picture it does not
        // carry): the material's own colour, toned down, so that a white material is no blank white box
        let absent = m.texture.is_some() && view.is_none();
        let tone = if absent { ABSENT_TEXTURE_TONE } else { 1.0 };
        let [dr, dg, db, da] = m.diffuse;
        let uniform = MaterialUniform {
            diffuse: [dr * tone, dg * tone, db * tone, da],
            emissive: [
                m.emissive[0],
                m.emissive[1],
                m.emissive[2],
                if view.is_some() { 1.0 } else { 0.0 },
            ],
            params: [
                // a texture with a transparent colour turns the alpha test on, dropping alpha 0
                if m.texture
                    .is_some_and(|t| keyed.get(t).copied().unwrap_or(false))
                {
                    0.0
                } else {
                    m.alpha_cutoff.unwrap_or(-1.0)
                },
                if m.unlit { 1.0 } else { 0.0 },
                match m.texgen {
                    Some(level::TexGen::Reflect) => 2.0,
                    Some(level::TexGen::Chrome) => 3.0,
                    None => 0.0,
                },
                if relief.is_some() { 1.0 } else { 0.0 },
            ],
            // w: the parallax depth (the normal map's alpha is then a height), only with a normal map
            ambient: [
                m.ambient[0],
                m.ambient[1],
                m.ambient[2],
                if relief.is_some() && m.parallax.is_finite() {
                    m.parallax.clamp(0.0, 0.25)
                } else {
                    0.0
                },
            ],
            specular: [m.specular[0], m.specular[1], m.specular[2], m.power],
            funcs: [f32::from(funcs.1), 0.0, 0.0, 0.0],
        };
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material"),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(view.unwrap_or(&self.white)),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        relief.unwrap_or(&self.flat_normal),
                    ),
                },
            ],
        });
        GpuMaterial {
            bind,
            key: PipelineKey {
                blend: m.blend,
                z_write: m.z_write || m.blend.is_none(),
                z_func: funcs.0,
            },
        }
    }

    /// How many pipelines are made so far (a new one is a hitch in the frame that needs it).
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len() + self.particle_pipelines.len()
    }

    /// Draws the materials a script changed as it changed them (matstate.rs): only a material whose change differs
    /// from what is drawn is rebuilt, and one whose change is gone (a rewind) goes back to the level's own. With
    /// nothing changed, before and now, this does nothing.
    pub fn sync_materials(&mut self, model: usize, state: &crate::matstate::MatState) {
        let Some(m) = self.models.get(model) else {
            return;
        };
        if state.over.is_empty()
            && m.applied.is_empty()
            && state.written.is_empty()
            && m.written.is_empty()
        {
            return;
        }
        // the textures scripts wrote into: a changed copy is uploaded again, a gone one gives the level's back
        let stamps = |it: &mut dyn Iterator<Item = (usize, u64)>| it.collect();
        let copies: std::collections::BTreeMap<usize, u64> =
            stamps(&mut state.written.iter().map(|(&t, c)| (t, c.stamp)));
        let drawn: std::collections::BTreeMap<usize, u64> =
            stamps(&mut m.written.iter().map(|(&t, (s, _))| (t, *s)));
        let (upload, restore) = texture_sync(&copies, &drawn);
        if !upload.is_empty() || !restore.is_empty() {
            let views: Vec<(usize, u64, wgpu::TextureView)> = upload
                .iter()
                .filter_map(|t| {
                    let c = state.written.get(t)?;
                    let v = upload_texture(
                        &self.device,
                        &self.queue,
                        c.pix.width,
                        c.pix.height,
                        c.pix.rgba.to_vec(),
                    );
                    Some((*t, c.stamp, v))
                })
                .collect();
            let Some(m) = self.models.get_mut(model) else {
                return;
            };
            for (t, stamp, v) in views {
                let Some(slot) = m.textures.get_mut(t) else {
                    continue;
                };
                let original = match m.written.remove(&t) {
                    Some((_, o)) => o,
                    None => slot.clone(),
                };
                *slot = Some(v);
                m.written.insert(t, (stamp, original));
            }
            for t in restore {
                if let Some((_, original)) = m.written.remove(&t)
                    && let Some(slot) = m.textures.get_mut(t)
                {
                    *slot = original;
                }
            }
            // every material that draws with one of those textures is built again
            let touched: std::collections::BTreeSet<usize> = upload
                .iter()
                .copied()
                .chain(drawn.keys().copied())
                .collect();
            let rebuild: Vec<usize> = (0..state.material_count())
                .filter(|&i| {
                    state.effective(i).is_some_and(|mat| {
                        mat.texture.is_some_and(|t| touched.contains(&t))
                            || mat.normal.is_some_and(|t| touched.contains(&t))
                    })
                })
                .collect();
            let Some(m) = self.models.get(model) else {
                return;
            };
            let rebuilt: Vec<(usize, GpuMaterial)> = rebuild
                .into_iter()
                .filter_map(|i| {
                    let mat = state.effective(i)?;
                    Some((
                        i,
                        self.gpu_material(&mat, &m.textures, &m.keyed, state.funcs(i)),
                    ))
                })
                .collect();
            if let Some(m) = self.models.get_mut(model) {
                for (i, g) in rebuilt {
                    if let Some(slot) = m.materials.get_mut(i) {
                        *slot = g;
                    }
                }
            }
        }
        let Some(m) = self.models.get(model) else {
            return;
        };
        let mut rebuilt: Vec<(usize, Option<crate::matstate::MatOverride>, GpuMaterial)> =
            Vec::new();
        for (&i, o) in &state.over {
            if m.applied.get(&i) != Some(o)
                && let Some(mat) = state.effective(i)
            {
                rebuilt.push((
                    i,
                    Some(o.clone()),
                    self.gpu_material(&mat, &m.textures, &m.keyed, state.funcs(i)),
                ));
            }
        }
        for &i in m.applied.keys() {
            if !state.over.contains_key(&i)
                && let Some(mat) = state.effective(i)
            {
                rebuilt.push((
                    i,
                    None,
                    self.gpu_material(&mat, &m.textures, &m.keyed, state.funcs(i)),
                ));
            }
        }
        let Some(m) = self.models.get_mut(model) else {
            return;
        };
        for (i, o, g) in rebuilt {
            if let Some(slot) = m.materials.get_mut(i) {
                *slot = g;
            }
            match o {
                Some(o) => m.applied.insert(i, o),
                None => m.applied.remove(&i),
            };
        }
    }

    /// A mesh on the GPU: its vertices, a part per material and a pass per material channel. A part without a material
    /// uses `default_material`.
    fn gpu_mesh_cpu(m: &level::Mesh, default_material: usize) -> Option<MeshCpu> {
        if m.positions.is_empty() {
            return None;
        }
        let tangents = tangents(m);
        let vertices: Vec<Vertex> = (0..m.positions.len())
            .map(|i| Vertex {
                pos: m.positions.get(i).copied().unwrap_or_default(),
                normal: m.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                uv: m.uvs.get(i).copied().unwrap_or_default(),
                tangent: tangents.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 1.0]),
            })
            .collect();
        let mut parts: Vec<PartCpu> = m
            .parts
            .iter()
            .filter(|p| !p.indices.is_empty())
            .map(|p| PartCpu {
                index: bytemuck::cast_slice(&p.indices).to_vec(),
                count: p.indices.len() as u32,
                material: p.material.unwrap_or(default_material),
                channel: None,
            })
            .collect();
        // the material channels: every face again, with the channel's material, coordinates and blend
        let all: Vec<u32> = m
            .parts
            .iter()
            .flat_map(|p| p.indices.iter().copied())
            .collect();
        for c in &m.channels {
            if all.is_empty() || c.material >= default_material {
                continue;
            }
            let vertices: Vec<Vertex> = vertices
                .iter()
                .enumerate()
                .map(|(i, v)| Vertex {
                    uv: c.uvs.get(i).copied().unwrap_or_default(),
                    ..*v
                })
                .collect();
            parts.push(PartCpu {
                index: bytemuck::cast_slice(&all).to_vec(),
                count: all.len() as u32,
                material: c.material,
                channel: Some((
                    bytemuck::cast_slice(&vertices).to_vec(),
                    PipelineKey {
                        blend: Some(c.blend),
                        z_write: false,
                        z_func: Z_FUNC_DEFAULT,
                    },
                )),
            });
        }
        Some(MeshCpu {
            vertex: bytemuck::cast_slice(&vertices).to_vec(),
            parts,
        })
    }

    /// The meshes on the card, their bytes packed into a few shared buffers (a buffer for each vertex list and each
    /// index list took 12 ms apiece on a software adapter: 25 s for a 2000-mesh map).
    fn upload_meshes(&self, meshes: Vec<Option<MeshCpu>>) -> Vec<Option<GpuMesh>> {
        let packed = pack_meshes(meshes);
        let buffers = packed.chunks.iter().map(|c| self.mesh_buffer(c)).collect();
        packed.assemble(buffers)
    }

    /// One shared buffer of packed mesh bytes.
    fn mesh_buffer(&self, bytes: &[u8]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("meshes"),
                contents: bytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX,
            })
    }

    /// The sky a script built (scriptsky.rs), as a mesh of the level's model (its parts name the level's materials);
    /// none takes it away. Returns its mesh index for `Instance::mesh` while it is there.
    pub fn set_script_sky(&mut self, model: usize, sky: Option<&level::Mesh>) -> Option<usize> {
        let gpu = sky.and_then(|m| {
            let default = self.models.get(model)?.materials.len().saturating_sub(1);
            let cpu = Self::gpu_mesh_cpu(m, default)?;
            self.upload_meshes(vec![Some(cpu)]).pop().flatten()
        });
        let m = self.models.get_mut(model)?;
        let at = match self.script_sky.filter(|s| s.0 == model) {
            Some((_, i)) => i,
            None => {
                m.meshes.push(None);
                m.meshes.len() - 1
            }
        };
        let shown = gpu.is_some();
        *m.meshes.get_mut(at)? = gpu;
        self.script_sky = Some((model, at));
        shown.then_some(at)
    }

    /// The sky a map's TT Sky block builds (`sky`), put into this frame's instances: its mesh is made when the block's
    /// state changes, drawn at the eye in place of the level's own sky. Nothing happens with no script sky.
    pub fn apply_script_sky(
        &mut self,
        model: usize,
        sky: Option<&script::SkySpec>,
        eye: Vec3,
        inst: &mut Vec<Instance>,
    ) {
        if self.script_sky_spec.as_ref().map(|s| &s.0) != sky {
            let mesh = sky.and_then(crate::scriptsky::build);
            let at = self.set_script_sky(model, mesh.as_ref());
            self.script_sky_spec = sky.map(|k| (k.clone(), at.is_some()));
        }
        let (Some((spec, true)), Some((_, at))) = (&self.script_sky_spec, self.script_sky) else {
            return;
        };
        inst.retain(|i| i.model != crate::scene::SKY_MODEL);
        let white = spec.color.iter().all(|&c| c >= 0.999);
        inst.push(Instance {
            unlit: true,
            diffuse: (!white).then_some(spec.color),
            ..Instance::plain(model, at, Mat4::from_translation(eye))
        });
    }

    /// Uploads a model; returns its index for `Instance::model`.
    pub fn add_model(&mut self, level: &Level, load: &TextureLoader<'_>) -> usize {
        let plans = texture_plans(level, load, self.bc);
        let mut textures: Vec<Option<wgpu::TextureView>> = vec![None; level.textures.len()];
        let mut plans = plans.into_iter().peekable();
        // read several at a time, each on its own thread, and uploaded; the queue is flushed between, so the
        // pictures never all wait in memory
        while plans.peek().is_some() {
            let started = std::time::Instant::now();
            let batch: Vec<_> = plans.by_ref().take(LOADS_AT_ONCE).collect();
            let loaded: Vec<(usize, Option<gamedata::gputex::TextureData>)> =
                std::thread::scope(|s| {
                    let threads: Vec<_> = batch
                        .into_iter()
                        .map(|(i, plan)| (i, s.spawn(move || plan.load())))
                        .collect();
                    threads
                        .into_iter()
                        .map(|(i, t)| (i, t.join().ok().flatten()))
                        .collect()
                });
            self.texture_stats.read += started.elapsed();
            let started = std::time::Instant::now();
            for (i, data) in loaded {
                if let (Some(data), Some(slot)) = (data, textures.get_mut(i)) {
                    *slot = self.texture_view(&data);
                }
            }
            self.queue.submit([]);
            self.texture_stats.upload += started.elapsed();
        }
        let mut build = ModelBuild::new(level, textures, None);
        loop {
            if let Some(m) = build.step(self, None) {
                self.models.push(m);
                return self.models.len() - 1;
            }
        }
    }

    /// The pipelines a model's materials draw with are made now, not in the frame they first show (a hitch each).
    fn model_pipeline(&mut self, k: PipelineKey) {
        self.pipeline(k);
    }

    /// A material of one colour: lit and opaque, or (for markers) unlit and blended.
    /// Sets the ball shadow's texture (the game's HardShadow): its pass multiplies the floor by it (ZERO / SRCCOLOR),
    /// unlit, without writing depth.
    pub fn set_shadow_texture(&mut self, width: u32, height: u32, rgba: Vec<u8>) {
        let view = upload_texture(&self.device, &self.queue, width, height, rgba);
        let uniform = MaterialUniform {
            diffuse: [1.0; 4],
            emissive: [1.0, 1.0, 1.0, 1.0],
            params: [-1.0, 1.0, 0.0, 0.0],
            ambient: [0.0; 4],
            specular: [0.0; 4],
            funcs: [f32::from(ALPHA_FUNC_DEFAULT), 0.0, 0.0, 0.0],
        };
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shadow material"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow material"),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.flat_normal),
                },
            ],
        });
        self.shadow_material = Some(GpuMaterial {
            bind,
            key: PipelineKey {
                blend: Some((Blend::Zero, Blend::SrcColor)),
                z_write: false,
                z_func: Z_FUNC_DEFAULT,
            },
        });
    }

    fn flat_material(&self, color: [f32; 4], marker: bool) -> GpuMaterial {
        let uniform = MaterialUniform {
            diffuse: color,
            emissive: [0.0; 4],
            params: [-1.0, if marker { 1.0 } else { 0.0 }, 0.0, 0.0],
            ambient: [0.0; 4],
            specular: [0.0; 4],
            funcs: [f32::from(ALPHA_FUNC_DEFAULT), 0.0, 0.0, 0.0],
        };
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("plain material"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("plain material"),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.white),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.flat_normal),
                },
            ],
        });
        GpuMaterial {
            bind,
            key: PipelineKey {
                blend: marker.then_some((Blend::SrcAlpha, Blend::InvSrcAlpha)),
                z_write: !marker,
                z_func: Z_FUNC_DEFAULT,
            },
        }
    }

    fn pipeline(&mut self, key: PipelineKey) -> &wgpu::RenderPipeline {
        let (device, shader, layout, format, samples) = (
            &self.device,
            &self.shader,
            &self.layout,
            self.format,
            self.samples,
        );
        self.pipelines.entry(key).or_insert_with(|| {
            let factor = |b: Blend| match b {
                Blend::Zero => wgpu::BlendFactor::Zero,
                Blend::One => wgpu::BlendFactor::One,
                Blend::SrcColor => wgpu::BlendFactor::Src,
                Blend::InvSrcColor => wgpu::BlendFactor::OneMinusSrc,
                Blend::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
                Blend::InvSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
                Blend::DestAlpha => wgpu::BlendFactor::DstAlpha,
                Blend::InvDestAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
                Blend::DestColor => wgpu::BlendFactor::Dst,
                Blend::InvDestColor => wgpu::BlendFactor::OneMinusDst,
            };
            let blend = key.blend.map(|(s, d)| {
                let c = wgpu::BlendComponent {
                    src_factor: factor(s),
                    dst_factor: factor(d),
                    operation: wgpu::BlendOperation::Add,
                };
                wgpu::BlendState { color: c, alpha: c }
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("level"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4
                        ],
                    })],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(key.z_write),
                    depth_compare: Some(compare(key.z_func)),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    /// Uploads a particle texture under its name (particles.rs names them).
    pub fn add_named_texture(
        &mut self,
        name: &'static str,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) {
        let view = upload_texture(&self.device, &self.queue, width, height, rgba);
        let bind = particle_bind(&self.device, &self.particle_tex_layout, &view, &self.clamp);
        self.named_textures.insert(name, bind);
    }

    /// Keeps under `name` the very texture kept under `from` (no second upload).
    pub fn share_named_texture(&mut self, name: &'static str, from: &'static str) {
        if let Some(bind) = self.named_textures.get(from).cloned() {
            self.named_textures.insert(name, bind);
        }
    }

    /// Forgets the texture kept under `name` (a level's frame the next level does not have).
    pub fn remove_named_texture(&mut self, name: &'static str) {
        self.named_textures.remove(name);
    }

    /// Starts making the title picture's pipeline on a thread: compiling its shader takes a tenth of a second or
    /// two, which the start-up does not wait for (the picture fades in when it is there).
    pub fn prepare_title(&mut self) {
        if self.title.is_some() || self.title_coming.is_some() {
            return;
        }
        let (device, format) = (self.device.clone(), self.format);
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("title picture".into())
            .spawn(move || {
                let began = std::time::Instant::now();
                let made = crate::titlescene::TitleScene::new(&device, format);
                eprintln!(
                    "startup: the title picture's pipeline made in {:.0} ms (on its thread)",
                    began.elapsed().as_secs_f64() * 1000.0
                );
                let _ = tx.send(made);
            });
        if spawned.is_ok() {
            self.title_coming = Some(rx);
        }
    }

    /// The picture's shot as it is drawn now, once its pipeline is there (None while it is still being made: the
    /// frame stays as it is; the title screen's picture then comes up in a quarter of a second).
    fn title_shot(&mut self, shot: &crate::titlescene::Shot) -> Option<crate::titlescene::Shot> {
        if let Some(rx) = &self.title_coming {
            match rx.try_recv() {
                Ok(made) => {
                    self.title = Some(made);
                    self.title_coming = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return None,
                // the thread is gone without a pipeline: it is made here
                Err(_) => self.title_coming = None,
            }
        }
        let (device, format) = (&self.device, self.format);
        self.title
            .get_or_insert_with(|| crate::titlescene::TitleScene::new(device, format));
        let since = *self.title_since.get_or_insert_with(std::time::Instant::now);
        let mut shot = *shot;
        if !shot.loading {
            shot.alpha *= (since.elapsed().as_secs_f32() / 0.25).clamp(0.0, 1.0);
        }
        Some(shot)
    }

    /// Draws the original's 2D interface over `target`, which keeps its content: quads in window pixels, each
    /// with a named texture (white when the copy lacks it).
    pub fn draw_ui(
        &self,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        quads: &[crate::ui::Quad],
    ) {
        if quads.is_empty() {
            return;
        }
        let (verts, runs) = Self::ui_vertices(width, height, quads);
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut enc = self.device.create_command_encoder(&Default::default());
        self.ui_pass(&mut enc, target, (&buf, 0..buf.size()), runs);
        self.queue.submit([enc.finish()]);
    }

    /// The interface, then the title or loading screen's picture over it, then that screen's words: one buffer and
    /// one batch of commands for the card, as the interface alone takes (every object made and every batch sent
    /// waits its turn behind the loading thread's uploads, and the frames of a load must not pay for that thrice).
    pub fn draw_screen(
        &mut self,
        target: &wgpu::TextureView,
        (width, height): (u32, u32),
        under: &[crate::ui::Quad],
        shot: Option<&crate::titlescene::Shot>,
        over: &[crate::ui::Quad],
    ) {
        // the words wait for their picture
        let waiting = shot.is_some();
        let shot = shot.and_then(|s| self.title_shot(s));
        let over = if waiting && shot.is_none() { &[] } else { over };
        if under.is_empty() && shot.is_none() && over.is_empty() {
            return;
        }
        let (under_verts, under_runs) = Self::ui_vertices(width, height, under);
        let (over_verts, over_runs) = Self::ui_vertices(width, height, over);
        let params = shot.map(|s| crate::titlescene::TitleScene::params((width, height), &s));
        // the three passes' vertices one after the other in one buffer
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(bytemuck::cast_slice(&under_verts));
        let over_at = bytes.len() as u64;
        bytes.extend_from_slice(bytemuck::cast_slice(&over_verts));
        let params_at = bytes.len() as u64;
        if let Some(p) = &params {
            bytes.extend_from_slice(bytemuck::cast_slice(p));
        }
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("screen"),
                contents: &bytes,
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut enc = self.device.create_command_encoder(&Default::default());
        self.ui_pass(&mut enc, target, (&buf, 0..over_at), under_runs);
        if let (Some(_), Some(title)) = (&params, &self.title) {
            title.pass(&mut enc, target, buf.slice(params_at..));
        }
        self.ui_pass(&mut enc, target, (&buf, over_at..params_at), over_runs);
        self.queue.submit([enc.finish()]);
    }

    /// The quads' vertices (in clip space on a target of `width` by `height`) and their runs by texture (name, first
    /// vertex, count).
    fn ui_vertices(width: u32, height: u32, quads: &[crate::ui::Quad]) -> (Vec<UiVertex>, UiRuns) {
        let (w, h) = (width.max(1) as f32, height.max(1) as f32);
        let mut verts: Vec<UiVertex> = Vec::with_capacity(quads.len() * 6);
        let mut runs: Vec<(&'static str, u32, u32)> = Vec::new();
        for q in quads {
            let [u0, v0, u1, v1] = q.uv;
            let v = |x: f32, y: f32, u: f32, t: f32, color: [f32; 4]| UiVertex {
                pos: [x / w * 2.0 - 1.0, 1.0 - y / h * 2.0],
                uv: [u, t],
                color,
            };
            // the corners in window pixels, turned about the centre by the quad's angle (0 leaves them as they are)
            let [x0, y0, x1, y1] = q.rect;
            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            let (sn, cs) = if q.angle.is_finite() {
                q.angle.to_radians().sin_cos()
            } else {
                (0.0, 1.0)
            };
            let turn = |x: f32, y: f32| {
                if sn == 0.0 {
                    (x, y)
                } else {
                    let (dx, dy) = (x - cx, y - cy);
                    (cx + dx * cs - dy * sn, cy + dx * sn + dy * cs)
                }
            };
            let (a, b, c, d) = (turn(x0, y0), turn(x1, y0), turn(x1, y1), turn(x0, y1));
            let first = verts.len() as u32;
            verts.extend([
                v(a.0, a.1, u0, v0, q.top),
                v(b.0, b.1, u1, v0, q.top),
                v(c.0, c.1, u1, v1, q.bottom),
                v(a.0, a.1, u0, v0, q.top),
                v(c.0, c.1, u1, v1, q.bottom),
                v(d.0, d.1, u0, v1, q.bottom),
            ]);
            match runs.last_mut() {
                Some(r) if r.0 == q.texture => r.2 += 6,
                _ => runs.push((q.texture, first, 6)),
            }
        }
        (verts, runs)
    }

    /// A pass of the interface's quads added to `enc`: `verts` holds their vertices.
    fn ui_pass(
        &self,
        enc: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        (buf, range): (&wgpu::Buffer, std::ops::Range<u64>),
        runs: UiRuns,
    ) {
        if runs.is_empty() {
            return;
        }
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.ui_pipeline);
            pass.set_vertex_buffer(0, buf.slice(range));
            for (texture, first, count) in runs {
                let bind = self.named_textures.get(texture);
                // a map's frame whose texture nobody has is not drawn: never a white rectangle over the picture
                if bind.is_none() && crate::ui::FRAME_TEXTURES.contains(&texture) {
                    continue;
                }
                pass.set_bind_group(0, bind.unwrap_or(&self.particle_white), &[]);
                pass.draw(first..first + count, 0..1);
            }
        }
    }

    fn particle_pipeline(&mut self, key: (PBlend, bool)) {
        let (device, shader, layout, format, samples) = (
            &self.device,
            &self.particle_shader,
            &self.particle_layout,
            self.format,
            self.samples,
        );
        self.particle_pipelines.entry(key).or_insert_with(|| {
            let (src, dst) = match key.0 {
                PBlend::AlphaAdd => (wgpu::BlendFactor::SrcAlpha, wgpu::BlendFactor::One),
                PBlend::OneSrcColor => (wgpu::BlendFactor::One, wgpu::BlendFactor::Src),
                PBlend::Alpha => (wgpu::BlendFactor::SrcAlpha, wgpu::BlendFactor::OneMinusSrcAlpha),
            };
            let c = wgpu::BlendComponent { src_factor: src, dst_factor: dst, operation: wgpu::BlendOperation::Add };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("particles"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<crate::particles::Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4],
                    })],
                },
                primitive: wgpu::PrimitiveState {
                    topology: if key.1 { wgpu::PrimitiveTopology::LineList } else { wgpu::PrimitiveTopology::TriangleList },
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState { color: c, alpha: c }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        });
    }

    /// Takes the anti-aliasing the look allows this frame: another sample count makes the pipelines, the
    /// depth and the cloud pass again.
    fn sync_quality(&mut self) {
        let samples = samples_now(&self.look);
        if samples != self.samples {
            self.samples = samples;
            self.pipelines.clear();
            self.particle_pipelines.clear();
            self.depth = None;
            self.msaa = None;
            self.cloud_pass = None;
        }
    }

    /// The multisampled colour the frame is drawn into (resolved to the target), when anti-aliasing is on.
    fn msaa_view(&mut self, width: u32, height: u32) -> Option<wgpu::TextureView> {
        if self.samples <= 1 {
            return None;
        }
        if !matches!(&self.msaa, Some((_, w, h)) if *w == width && *h == height) {
            let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("msaa"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: self.samples,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa = Some((tex.create_view(&Default::default()), width, height));
        }
        self.msaa.as_ref().map(|m| m.0.clone())
    }

    fn depth_view(&mut self, width: u32, height: u32) -> Option<wgpu::TextureView> {
        let fresh = match &self.depth {
            Some((_, _, w, h)) => *w != width || *h != height,
            None => true,
        };
        if fresh {
            let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: self.samples,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                // the cloud pass reads it
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            self.depth = Some((tex, view, width, height));
            if let Some(p) = &mut self.cloud_pass {
                p.depth_changed();
            }
        }
        self.depth.as_ref().map(|d| d.1.clone())
    }

    /// The haze round the horizon the clouds fade into (clouds.rs, horizon_ring).
    pub fn set_cloud_ring(&mut self, rgba: &[u8]) {
        if let Some(p) = &mut self.cloud_pass {
            p.set_ring(&self.device, &self.queue, rgba);
        }
    }

    /// Draws flat rectangles (pixels, top left origin) over `target`, which keeps its content.
    pub fn draw_overlay(
        &self,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        rects: &[crate::hud::Rect],
    ) {
        if rects.is_empty() {
            return;
        }
        let (w, h) = (width.max(1) as f32, height.max(1) as f32);
        let to_clip = |x: f32, y: f32| [x / w * 2.0 - 1.0, 1.0 - y / h * 2.0];
        let mut verts = Vec::with_capacity(rects.len() * 6);
        for r in rects {
            let (a, b, c, d) = (
                to_clip(r.x, r.y),
                to_clip(r.x + r.w, r.y),
                to_clip(r.x + r.w, r.y + r.h),
                to_clip(r.x, r.y + r.h),
            );
            for pos in [a, b, c, a, c, d] {
                verts.push(OverlayVertex {
                    pos,
                    color: r.color,
                });
            }
        }
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("overlay vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.overlay_pipeline);
            pass.set_vertex_buffer(0, buf.slice(..));
            pass.draw(0..verts.len() as u32, 0..1);
        }
        self.queue.submit([enc.finish()]);
    }

    /// Draws one frame into `target` of the given size.
    pub fn draw(
        &mut self,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        view: &View,
        instances: &[Instance],
        particles: &[crate::particles::Batch],
    ) {
        self.sync_quality();
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let proj = perspective_lh(view.fov_y, aspect, view.near, view.far);
        let look = look_at_lh(view.eye, view.target, Vec3::Y);
        let scripted = self.script_fog.filter(|_| !self.menu_frame);
        let fog = if scripted.is_some() {
            None
        } else {
            self.fog.filter(|_| !self.menu_frame)
        };
        let script_fog = scripted.flatten();
        let globals = Globals {
            view_proj: (proj * look).to_cols_array_2d(),
            light_dir: self
                .light
                .dir
                .normalize_or(Vec3::NEG_Y)
                .extend(0.0)
                .to_array(),
            ambient: [
                self.light.ambient[0],
                self.light.ambient[1],
                self.light.ambient[2],
                1.0,
            ],
            view: look.to_cols_array_2d(),
            eye: [view.eye.x, view.eye.y, view.eye.z, 1.0],
            light_color: [
                self.light.color[0],
                self.light.color[1],
                self.light.color[2],
                1.0,
            ],
            flash_dir: crate::lights::FLASH_DIR.normalize().extend(0.0).to_array(),
            flash_color: [self.flash, self.flash, self.flash, 0.0],
            point_pos: self.point.map_or([0.0; 4], |(p, _)| {
                p.extend(crate::lights::POINT_RANGE).to_array()
            }),
            point_color: self
                .point
                .map_or([0.0; 4], |(_, c)| [c[0], c[1], c[2], 0.0]),
            fog_color: fog.map_or([0.0; 4], |f| {
                [f.color[0], f.color[1], f.color[2], f.density]
            }),
            fog_height: fog.map_or([0.0, 0.0, 0.0, self.look.filter as f32], |f| {
                [f.below, f.depth, 1.0, self.look.filter as f32]
            }),
            fog_down: fog.map_or([0.0; 4], |f| {
                [f.color_down[0], f.color_down[1], f.color_down[2], 0.0]
            }),
            script_fog: script_fog
                .map_or([0.0; 4], |f| [f32::from(f.mode), f.start, f.end, f.density]),
            script_fog_color: script_fog
                .map_or([0.0; 4], |f| [f.color[0], f.color[1], f.color[2], 0.0]),
        };
        self.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        if instances.len() as u64 > self.models_capacity {
            let cap = (instances.len() as u64).next_power_of_two();
            let (b, g) = model_buffer(&self.device, &self.model_layout, cap);
            self.models_buf = b;
            self.models_bind = g;
            self.models_capacity = cap;
        }
        let mut matrices = vec![0u8; instances.len() * UNIFORM_ALIGN as usize];
        for (i, inst) in instances.iter().enumerate() {
            let mut data = [0f32; INSTANCE_BYTES / 4];
            // the script's sky is drawn at any distance the scene's far plane lets (it only turns about the eye, so
            // a smaller one looks the same)
            let matrix = match &self.script_sky_spec {
                Some((spec, true)) if self.script_sky == Some((inst.model, inst.mesh)) => {
                    inst.matrix * Mat4::from_scale(Vec3::splat(sky_scale(spec, view.far)))
                }
                _ => inst.matrix,
            };
            data[..16].copy_from_slice(&matrix.to_cols_array());
            data[16..18].copy_from_slice(&inst.uv_offset);
            data[18] = if inst.unlit { 1.0 } else { 0.0 };
            data[20..24].copy_from_slice(&inst.diffuse.unwrap_or([0.0, 0.0, 0.0, -1.0]));
            if let Some([x, z, side]) = inst.shadow {
                data[24..28].copy_from_slice(&[x, z, side, 1.0]);
            }
            let at = i * UNIFORM_ALIGN as usize;
            if let Some(dst) = matrices.get_mut(at..at + INSTANCE_BYTES) {
                dst.copy_from_slice(bytemuck::cast_slice(&data));
            }
        }
        self.queue.write_buffer(&self.models_buf, 0, &matrices);

        // Opaque first, then blended back to front.
        for inst in instances {
            if let Some(c) = inst.marker {
                let k = c.map(f32::to_bits);
                if !self.markers.contains_key(&k) {
                    let m = self.flat_material(c, true);
                    self.markers.insert(k, m);
                }
            }
        }
        let mut back = Vec::new();
        let mut opaque = Vec::new();
        let mut blended = Vec::new();
        for (i, inst) in instances.iter().enumerate() {
            let Some(mesh) = self
                .models
                .get(inst.model)
                .and_then(|m| m.meshes.get(inst.mesh))
                .and_then(Option::as_ref)
            else {
                continue;
            };
            for (p, part) in mesh.parts.iter().enumerate() {
                let key = match inst.marker {
                    // a shadow pass covers the mesh's own faces once each, a marker is one flat colour: no channels
                    _ if inst.shadow.is_some() && part.channel.is_some() => None,
                    _ if inst.shadow.is_some() => self.shadow_material.as_ref().map(|m| m.key),
                    Some(_) if part.channel.is_some() => None,
                    Some(c) => self.markers.get(&c.map(f32::to_bits)).map(|m| m.key),
                    None if part.channel.is_some() => part.channel.as_ref().map(|c| c.1),
                    None => self
                        .models
                        .get(inst.model)
                        .and_then(|m| m.materials.get(inst.material_of(part.material)))
                        .map(|m| m.key),
                };
                match key {
                    // the script's sky: first, over everything the frame has so far, writing no depth
                    Some(k) if self.script_sky == Some((inst.model, inst.mesh)) => back.push((
                        0.0,
                        i,
                        p,
                        PipelineKey {
                            z_write: false,
                            z_func: 8,
                            ..k
                        },
                    )),
                    Some(k) if k.blend.is_some() => {
                        let d = inst.matrix.w_axis.truncate().distance_squared(view.eye);
                        blended.push((d, i, p, k));
                    }
                    Some(k) => opaque.push((0.0, i, p, k)),
                    None => {}
                }
            }
        }
        // a script's depth function not seen before, with the pipelines at their cap, is drawn with the default one
        for d in back
            .iter_mut()
            .chain(opaque.iter_mut())
            .chain(blended.iter_mut())
        {
            if d.3.z_func != Z_FUNC_DEFAULT
                && !self.pipelines.contains_key(&d.3)
                && self.pipelines.len() >= MAX_PIPELINES
            {
                d.3.z_func = Z_FUNC_DEFAULT;
            }
        }
        blended.sort_by(|a, b| b.0.total_cmp(&a.0));
        let draws: Vec<(f32, usize, usize, PipelineKey)> =
            back.into_iter().chain(opaque).chain(blended).collect();
        for d in &draws {
            self.pipeline(d.3);
        }
        let particle_draws: Vec<(wgpu::Buffer, u32, (PBlend, bool), Option<&'static str>)> =
            particles
                .iter()
                .filter(|b| !b.vertices.is_empty())
                .map(|b| {
                    let key = (b.blend, b.lines);
                    self.particle_pipeline(key);
                    let buf = self
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("particles"),
                            contents: bytemuck::cast_slice(&b.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                    (buf, b.vertices.len() as u32, key, b.texture)
                })
                .collect();

        let Some(depth) = self.depth_view(width, height) else {
            return;
        };
        // with anti-aliasing on, the frame is drawn into a multisampled colour and resolved to the target at its end
        let msaa = self.msaa_view(width, height);
        let resolve_to = msaa.as_ref().map(|_| target);
        let target = msaa.as_ref().unwrap_or(target);
        // the sea of cloud goes between the opaque scene (whose depth it reads) and the blended one
        let quality = crate::clouds::quality();
        let clouds =
            self.clouds.is_some() && !self.menu_frame && quality != crate::clouds::Quality::Off;
        if clouds && self.cloud_pass.is_none() {
            self.cloud_pass = Some(crate::clouds::Pass::new(
                &self.device,
                &self.queue,
                self.format,
                self.samples,
            ));
            if let Some(ring) = self.clouds.as_ref().map(|l| l.ring.clone()) {
                self.set_cloud_ring(&ring);
            }
        }
        let opaque_count = draws.iter().take_while(|d| d.3.blend.is_none()).count();
        let mut enc = self.device.create_command_encoder(&Default::default());
        let phases: &[(std::ops::Range<usize>, bool)] = if clouds {
            &[(0..opaque_count, false), (opaque_count..draws.len(), true)]
        } else {
            &[(0..draws.len(), true)]
        };
        for (n, (range, last)) in phases.iter().enumerate() {
            if let (true, Some(p), Some(layer)) = (n > 0, &mut self.cloud_pass, &self.clouds) {
                let frame = crate::clouds::Frame {
                    view_proj: proj * look,
                    eye: view.eye,
                    time: self.started.elapsed().as_secs_f64(),
                    width,
                    height,
                    quality,
                };
                p.draw(
                    &self.device,
                    &self.queue,
                    &mut enc,
                    target,
                    &depth,
                    layer,
                    &frame,
                );
            }
            let keep = n > 0;
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: resolve_to.filter(|_| *last),
                    ops: wgpu::Operations {
                        load: if keep {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(self.clear)
                        },
                        store: if resolve_to.is_some() && *last {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: if keep {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(1.0)
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.globals_bind, &[]);
            for &(_, i, p, key) in &draws[range.clone()] {
                let Some(inst) = instances.get(i) else {
                    continue;
                };
                let Some(model) = self.models.get(inst.model) else {
                    continue;
                };
                let Some(mesh) = model.meshes.get(inst.mesh).and_then(Option::as_ref) else {
                    continue;
                };
                let Some(part) = mesh.parts.get(p) else {
                    continue;
                };
                let mat = match inst.marker {
                    _ if inst.shadow.is_some() => self.shadow_material.as_ref(),
                    Some(c) => self.markers.get(&c.map(f32::to_bits)),
                    None => model.materials.get(inst.material_of(part.material)),
                };
                let Some(mat) = mat else {
                    continue;
                };
                let Some(pipeline) = self.pipelines.get(&key) else {
                    continue;
                };
                pass.set_pipeline(pipeline);
                pass.set_bind_group(1, &self.models_bind, &[(i as u64 * UNIFORM_ALIGN) as u32]);
                pass.set_bind_group(2, &mat.bind, &[]);
                pass.set_vertex_buffer(
                    0,
                    part.channel.as_ref().map_or(&mesh.vertex, |c| &c.0).slice(),
                );
                pass.set_index_buffer(part.index.slice(), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..part.count, 0, 0..1);
            }
            // Particles last, over everything, testing depth without writing it.
            for (buf, count, key, texture) in particle_draws.iter().filter(|_| *last) {
                let Some(pipeline) = self.particle_pipelines.get(key) else {
                    continue;
                };
                let bind = texture
                    .and_then(|t| self.named_textures.get(t))
                    .unwrap_or(&self.particle_white);
                pass.set_pipeline(pipeline);
                pass.set_bind_group(1, bind, &[]);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..*count, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
    }
}

/// Left-handed perspective with depth 0..1 (Virtools' coordinates, WebGPU's depth range).
fn perspective_lh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let h = 1.0 / (fov_y * 0.5).tan();
    let w = h / aspect;
    let r = far / (far - near);
    Mat4::from_cols(
        glam::Vec4::new(w, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, h, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, r, 1.0),
        glam::Vec4::new(0.0, 0.0, -r * near, 0.0),
    )
}

/// Left-handed view matrix looking from `eye` at `target`.
fn look_at_lh(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let f = (target - eye).normalize_or(Vec3::Z);
    let s = up.cross(f).normalize_or(Vec3::X);
    let u = f.cross(s);
    Mat4::from_cols(
        glam::Vec4::new(s.x, u.x, f.x, 0.0),
        glam::Vec4::new(s.y, u.y, f.y, 0.0),
        glam::Vec4::new(s.z, u.z, f.z, 0.0),
        glam::Vec4::new(-s.dot(eye), -u.dot(eye), -f.dot(eye), 1.0),
    )
}

/// The interface's quads' runs by texture: (name, first vertex, count).
type UiRuns = Vec<(&'static str, u32, u32)>;

/// A vertex of the 2D interface: clip-space position, texture coordinates, colour.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct UiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

fn ui_pipeline(
    device: &wgpu::Device,
    textures: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ui"),
        source: wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ui"),
        bind_group_layouts: &[Some(textures)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ui"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<UiVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
            })],
        },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn particle_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("particle texture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn model_buffer(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    capacity: u64,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("models"),
        size: capacity * UNIFORM_ALIGN,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("models"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buf,
                offset: 0,
                size: wgpu::BufferSize::new(INSTANCE_BYTES as u64),
            }),
        }],
    });
    (buf, bind)
}

/// Uploads RGBA pixels with a full mip chain (box-filtered on the CPU).
/// Each vertex's tangent frame for normal maps: the direction its texture's u grows along the surface, made square to
/// the vertex's normal, and in w which way v goes (+1 or -1, mirrored coordinates flip it). Faces whose coordinates
/// are degenerate add nothing; a vertex no face gives a frame gets one square to its normal.
fn tangents(m: &level::Mesh) -> Vec<[f32; 4]> {
    let n = m.positions.len();
    let (mut tan, mut bit) = (vec![Vec3::ZERO; n], vec![Vec3::ZERO; n]);
    let at = |i: u32| -> Option<(Vec3, glam::Vec2)> {
        Some((
            Vec3::from(*m.positions.get(i as usize)?),
            glam::Vec2::from(*m.uvs.get(i as usize)?),
        ))
    };
    for t in m.parts.iter().flat_map(|p| p.indices.as_chunks::<3>().0) {
        let (Some((p0, w0)), Some((p1, w1)), Some((p2, w2))) = (at(t[0]), at(t[1]), at(t[2]))
        else {
            continue;
        };
        let (e1, e2, d1, d2) = (p1 - p0, p2 - p0, w1 - w0, w2 - w0);
        let det = d1.x * d2.y - d2.x * d1.y;
        if det.abs() < 1e-12 || !det.is_finite() {
            continue;
        }
        let (u, v) = ((e1 * d2.y - e2 * d1.y) / det, (e2 * d1.x - e1 * d2.x) / det);
        for &i in t {
            if let (Some(a), Some(b)) = (tan.get_mut(i as usize), bit.get_mut(i as usize)) {
                *a += u;
                *b += v;
            }
        }
    }
    (0..n)
        .map(|i| {
            let nrm = m
                .normals
                .get(i)
                .map(|&v| Vec3::from(v).normalize_or(Vec3::Y))
                .unwrap_or(Vec3::Y);
            let u = tan.get(i).copied().unwrap_or_default();
            let t = (u - nrm * nrm.dot(u)).normalize_or_zero();
            let t = if t == Vec3::ZERO {
                nrm.any_orthonormal_vector()
            } else {
                t
            };
            let b = bit.get(i).copied().unwrap_or_default();
            let w = if nrm.cross(t).dot(b) < 0.0 { -1.0 } else { 1.0 };
            [t.x, t.y, t.z, w]
        })
        .collect()
}

impl Renderer {
    /// A loaded texture on the card: one of the same data (a compressed texture's hash, format and size) is uploaded
    /// once for all the models that carry it.
    fn texture_view(&mut self, data: &gamedata::gputex::TextureData) -> Option<wgpu::TextureView> {
        let key = data
            .hash
            .map(|h| (h, data.format, data.size(), data.mips.len()));
        if let Some(view) = key.and_then(|k| self.shared_textures.get(&k)) {
            self.texture_stats.shared += 1;
            return Some(view.clone());
        }
        let view = upload_texture_data(&self.device, &self.queue, data)?;
        self.texture_stats.uploaded += 1;
        self.texture_stats.bytes += data.bytes();
        if data.format != gamedata::gputex::Format::Rgba {
            self.texture_stats.compressed += 1;
        }
        if let Some(k) = key {
            self.shared_textures.insert(k, view.clone());
        }
        Some(view)
    }

    /// The textures kept for sharing are let go: what is uploaded next (another level's folder, the menu's scene)
    /// never takes a picture from what came before, and a model dropped later frees its textures.
    pub fn end_sharing(&mut self) {
        self.shared_textures.clear();
    }

    /// A new level begins: the textures kept for sharing go with the old one's models, and the count starts again.
    pub fn begin_level(&mut self) {
        self.shared_textures.clear();
        self.texture_stats = TextureStats::default();
    }
}

/// Uploads RGBA pixels with a full mip chain (box-filtered on the CPU).
fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> wgpu::TextureView {
    gamedata::gputex::TextureData::from_rgba(width, height, rgba)
        .and_then(|data| upload_texture_data(device, queue, &data))
        .unwrap_or_else(|| {
            // a picture with no pixels: one white texel
            let one = gamedata::gputex::TextureData::from_rgba(1, 1, vec![255; 4]);
            one.and_then(|data| upload_texture_data(device, queue, &data))
                .unwrap_or_else(|| unreachable_texture(device))
        })
}

/// An empty 1 x 1 texture (never written): what stands in when even one texel cannot be made.
fn unreachable_texture(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

/// Uploads a texture's mip levels as they are: RGBA, or BC blocks (the caller planned those only for a device that
/// has them; the data's layout was checked by its reader, and is checked against the sizes again here).
fn create_texture_for(
    device: &wgpu::Device,
    data: &gamedata::gputex::TextureData,
) -> Option<(wgpu::Texture, u32)> {
    use gamedata::gputex::Format;
    let (width, height) = data.size();
    // Not sRGB: the original's fixed-function pipeline lights and blends in gamma space, and so do we.
    let (format, block) = match data.format {
        Format::Rgba => (wgpu::TextureFormat::Rgba8Unorm, 1),
        Format::Bc1 => (wgpu::TextureFormat::Bc1RgbaUnorm, 4),
        Format::Bc7 => (wgpu::TextureFormat::Bc7RgbaUnorm, 4),
    };
    let compressed = block > 1;
    if width == 0
        || height == 0
        || (compressed
            && (!device
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
                || width % block != 0
                || height % block != 0))
        || data.mips.iter().any(|m| {
            data.level(m).is_none() || data.format.level_bytes(m.width, m.height) != Some(m.len)
        })
    {
        return None;
    }
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: u32::try_from(data.mips.len()).ok()?,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    Some((tex, block))
}

fn upload_texture_data(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &gamedata::gputex::TextureData,
) -> Option<wgpu::TextureView> {
    let (tex, block) = create_texture_for(device, data)?;
    for (level, m) in data.mips.iter().enumerate() {
        // a compressed level is written in whole blocks, the small levels' too
        let (w, h) = (
            m.width.div_ceil(block) * block,
            m.height.div_ceil(block) * block,
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: u32::try_from(level).ok()?,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data.level(m)?,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w / block * u32::try_from(data.format.block_bytes()).ok()?),
                rows_per_image: Some(h / block),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
    Some(tex.create_view(&Default::default()))
}

#[cfg(test)]
mod texture_copy_tests {
    use super::texture_sync;
    use std::collections::BTreeMap;

    #[test]
    fn a_copy_is_uploaded_once_and_again_only_when_its_stamp_changes() {
        let none = BTreeMap::new();
        let one: BTreeMap<usize, u64> = [(3, 7)].into();
        // nothing written, nothing drawn: nothing to do
        assert_eq!(texture_sync(&none, &none), (vec![], vec![]));
        // a new copy is uploaded
        assert_eq!(texture_sync(&one, &none), (vec![3], vec![]));
        // the same stamp drawn: not again
        assert_eq!(texture_sync(&one, &one), (vec![], vec![]));
        // a changed copy (a new stamp) is uploaded again
        let newer: BTreeMap<usize, u64> = [(3, 8)].into();
        assert_eq!(texture_sync(&newer, &one), (vec![3], vec![]));
        // a rewind to before any write: the level's own texture goes back
        assert_eq!(texture_sync(&none, &newer), (vec![], vec![3]));
        // a rewind to an older copy is uploaded too (its stamp differs from the one drawn)
        let older: BTreeMap<usize, u64> = [(3, 5)].into();
        assert_eq!(texture_sync(&older, &newer), (vec![3], vec![]));
    }
}
