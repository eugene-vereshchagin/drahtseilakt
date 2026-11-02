// The renderer: uploads level models (meshes, materials, textures) once and draws a list of instances each
// frame, to a window surface or to an offscreen texture (screenshots, later video). Any size and aspect ratio:
// the projection always uses the real one.

use glam::{Mat4, Vec3};
use crate::particles::Blend as PBlend;
use level::{Blend, Level};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const UNIFORM_ALIGN: u64 = 256;

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

struct GpuPart {
    index: wgpu::Buffer,
    count: u32,
    material: usize,
    /// A material channel: its own vertices (the mesh's with the channel's coordinates) and blend, drawn over the
    /// mesh without writing depth.
    channel: Option<(wgpu::Buffer, PipelineKey)>,
}

struct GpuMesh {
    vertex: wgpu::Buffer,
    parts: Vec<GpuPart>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PipelineKey {
    blend: Option<(Blend, Blend)>,
    z_write: bool,
}

struct GpuMaterial {
    bind: wgpu::BindGroup,
    key: PipelineKey,
}

/// One uploaded model (a level, or the balls).
pub struct Model {
    meshes: Vec<Option<GpuMesh>>,
    materials: Vec<GpuMaterial>,
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
}

impl Instance {
    /// A mesh drawn with its own materials as they are.
    pub fn plain(model: usize, mesh: usize, matrix: Mat4) -> Instance {
        Instance { model, mesh, matrix, marker: None, uv_offset: [0.0; 2], diffuse: None, unlit: false, shadow: None }
    }
}

/// The per-instance uniform: the model matrix, then (u, v) added to the texture coordinates, then the diffuse
/// colour to use instead of the material's (alpha below 0: the material's own).
const INSTANCE_BYTES: usize = 112;

pub struct View {
    pub eye: Vec3,
    pub target: Vec3,
    pub fov_y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct OverlayVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

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
    /// The colour behind everything (the sky's zenith).
    pub clear: wgpu::Color,
    depth: Option<(wgpu::Texture, wgpu::TextureView, u32, u32)>,
}

/// Loads a texture's pixels: a file is looked up by the caller-supplied function.
pub type TextureLoader<'a> = dyn Fn(&level::Texture) -> Option<(u32, u32, Vec<u8>)> + 'a;

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
        let particle_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            clear: wgpu::Color {
                r: 0.4,
                g: 0.45,
                b: 0.6,
                a: 1.0,
            },
            depth: None,
        }
    }

    /// Uploads a model; returns its index for `Instance::model`.
    pub fn add_model(&mut self, level: &Level, load: &TextureLoader<'_>) -> usize {
        let textures: Vec<Option<wgpu::TextureView>> = level
            .textures
            .iter()
            .map(|t| {
                load(t).map(|(w, h, rgba)| upload_texture(&self.device, &self.queue, w, h, rgba))
            })
            .collect();
        let materials = level
            .materials
            .iter()
            .map(|m| {
                let view = m
                    .texture
                    .and_then(|t| textures.get(t))
                    .and_then(Option::as_ref);
                let relief = m.normal.and_then(|t| textures.get(t)).and_then(Option::as_ref);
                let uniform = MaterialUniform {
                    diffuse: m.diffuse,
                    emissive: [
                        m.emissive[0],
                        m.emissive[1],
                        m.emissive[2],
                        if view.is_some() { 1.0 } else { 0.0 },
                    ],
                    params: [
                        // a texture with a transparent colour turns the alpha test on, dropping alpha 0
                        match m.texture.and_then(|t| level.textures.get(t)).and_then(|t| t.color_key) {
                            Some(_) => 0.0,
                            None => m.alpha_cutoff.unwrap_or(-1.0),
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
                        if relief.is_some() && m.parallax.is_finite() { m.parallax.clamp(0.0, 0.25) } else { 0.0 },
                    ],
                    specular: [m.specular[0], m.specular[1], m.specular[2], m.power],
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
                            resource: wgpu::BindingResource::TextureView(
                                view.unwrap_or(&self.white),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(relief.unwrap_or(&self.flat_normal)),
                        },
                    ],
                });
                GpuMaterial {
                    bind,
                    key: PipelineKey {
                        blend: m.blend,
                        z_write: m.z_write || m.blend.is_none(),
                    },
                }
            })
            .collect::<Vec<_>>();
        // A part without a material is drawn with a plain white one, kept at the end.
        let default_material = materials.len();
        let mut materials = materials;
        materials.push(self.flat_material([0.8, 0.8, 0.8, 1.0], false));

        let meshes = level
            .meshes
            .iter()
            // a mesh with a finer look (a remastered level's) is drawn as that look
            .map(|m| m.look.as_deref().unwrap_or(m))
            .map(|m| {
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
                let vertex = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&m.name),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                let mut parts: Vec<GpuPart> = m
                    .parts
                    .iter()
                    .filter(|p| !p.indices.is_empty())
                    .map(|p| GpuPart {
                        index: self
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some(&m.name),
                                contents: bytemuck::cast_slice(&p.indices),
                                usage: wgpu::BufferUsages::INDEX,
                            }),
                        count: p.indices.len() as u32,
                        material: p.material.unwrap_or(default_material),
                        channel: None,
                    })
                    .collect();
                // the material channels: every face again, with the channel's material, coordinates and blend
                let all: Vec<u32> = m.parts.iter().flat_map(|p| p.indices.iter().copied()).collect();
                for c in &m.channels {
                    if all.is_empty() || c.material >= default_material {
                        continue;
                    }
                    let vertices: Vec<Vertex> = vertices
                        .iter()
                        .enumerate()
                        .map(|(i, v)| Vertex { uv: c.uvs.get(i).copied().unwrap_or_default(), ..*v })
                        .collect();
                    let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&m.name),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                    parts.push(GpuPart {
                        index: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&m.name),
                            contents: bytemuck::cast_slice(&all),
                            usage: wgpu::BufferUsages::INDEX,
                        }),
                        count: all.len() as u32,
                        material: c.material,
                        channel: Some((buf, PipelineKey { blend: Some(c.blend), z_write: false })),
                    });
                }
                Some(GpuMesh { vertex, parts })
            })
            .collect();
        self.models.push(Model { meshes, materials });
        self.models.len() - 1
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
        };
        let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow material"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow material"),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.clamp) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&self.flat_normal) },
            ],
        });
        self.shadow_material = Some(GpuMaterial {
            bind,
            key: PipelineKey { blend: Some((Blend::Zero, Blend::SrcColor)), z_write: false },
        });
    }

    fn flat_material(&self, color: [f32; 4], marker: bool) -> GpuMaterial {
        let uniform = MaterialUniform {
            diffuse: color,
            emissive: [0.0; 4],
            params: [-1.0, if marker { 1.0 } else { 0.0 }, 0.0, 0.0],
            ambient: [0.0; 4],
            specular: [0.0; 4],
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
            },
        }
    }

    fn pipeline(&mut self, key: PipelineKey) -> &wgpu::RenderPipeline {
        let (device, shader, layout, format) =
            (&self.device, &self.shader, &self.layout, self.format);
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
                let c = wgpu::BlendComponent { src_factor: factor(s), dst_factor: factor(d), operation: wgpu::BlendOperation::Add };
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
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(key.z_write),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    /// Uploads a particle texture under its name (particles.rs names them).
    pub fn add_named_texture(&mut self, name: &'static str, width: u32, height: u32, rgba: Vec<u8>) {
        let view = upload_texture(&self.device, &self.queue, width, height, rgba);
        let bind = particle_bind(&self.device, &self.particle_tex_layout, &view, &self.clamp);
        self.named_textures.insert(name, bind);
    }

    /// Draws the original's 2D interface over `target`, which keeps its content: quads in window pixels, each
    /// with a named texture (white when the copy lacks it).
    pub fn draw_ui(&self, target: &wgpu::TextureView, width: u32, height: u32, quads: &[crate::ui::Quad]) {
        if quads.is_empty() {
            return;
        }
        let (w, h) = (width.max(1) as f32, height.max(1) as f32);
        let mut verts: Vec<UiVertex> = Vec::with_capacity(quads.len() * 6);
        let mut runs: Vec<(&'static str, u32, u32)> = Vec::new();
        for q in quads {
            let [x0, y0, x1, y1] = q.rect;
            let (cx0, cy0, cx1, cy1) = (x0 / w * 2.0 - 1.0, 1.0 - y0 / h * 2.0, x1 / w * 2.0 - 1.0, 1.0 - y1 / h * 2.0);
            let [u0, v0, u1, v1] = q.uv;
            let v = |x: f32, y: f32, u: f32, t: f32, color: [f32; 4]| UiVertex { pos: [x, y], uv: [u, t], color };
            let first = verts.len() as u32;
            verts.extend([
                v(cx0, cy0, u0, v0, q.top),
                v(cx1, cy0, u1, v0, q.top),
                v(cx1, cy1, u1, v1, q.bottom),
                v(cx0, cy0, u0, v0, q.top),
                v(cx1, cy1, u1, v1, q.bottom),
                v(cx0, cy1, u0, v1, q.bottom),
            ]);
            match runs.last_mut() {
                Some(r) if r.0 == q.texture => r.2 += 6,
                _ => runs.push((q.texture, first, 6)),
            }
        }
        let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ui"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.ui_pipeline);
            pass.set_vertex_buffer(0, buf.slice(..));
            for (texture, first, count) in runs {
                pass.set_bind_group(0, self.named_textures.get(texture).unwrap_or(&self.particle_white), &[]);
                pass.draw(first..first + count, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
    }

    fn particle_pipeline(&mut self, key: (PBlend, bool)) {
        let (device, shader, layout, format) = (&self.device, &self.particle_shader, &self.particle_layout, self.format);
        self.particle_pipelines.entry(key).or_insert_with(|| {
            let (src, dst) = match key.0 {
                PBlend::AlphaAdd => (wgpu::BlendFactor::SrcAlpha, wgpu::BlendFactor::One),
                PBlend::OneSrcColor => (wgpu::BlendFactor::One, wgpu::BlendFactor::Src),
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
                multisample: Default::default(),
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
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            self.depth = Some((tex, view, width, height));
        }
        self.depth.as_ref().map(|d| d.1.clone())
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
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let proj = perspective_lh(view.fov_y, aspect, 0.5, 3000.0);
        let look = look_at_lh(view.eye, view.target, Vec3::Y);
        let globals = Globals {
            view_proj: (proj * look).to_cols_array_2d(),
            light_dir: self.light.dir.normalize_or(Vec3::NEG_Y).extend(0.0).to_array(),
            ambient: [self.light.ambient[0], self.light.ambient[1], self.light.ambient[2], 1.0],
            view: look.to_cols_array_2d(),
            eye: [view.eye.x, view.eye.y, view.eye.z, 1.0],
            light_color: [self.light.color[0], self.light.color[1], self.light.color[2], 1.0],
            flash_dir: crate::lights::FLASH_DIR.normalize().extend(0.0).to_array(),
            flash_color: [self.flash, self.flash, self.flash, 0.0],
            point_pos: self.point.map_or([0.0; 4], |(p, _)| p.extend(crate::lights::POINT_RANGE).to_array()),
            point_color: self.point.map_or([0.0; 4], |(_, c)| [c[0], c[1], c[2], 0.0]),
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
            data[..16].copy_from_slice(&inst.matrix.to_cols_array());
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
                        .and_then(|m| m.materials.get(part.material))
                        .map(|m| m.key),
                };
                match key {
                    Some(k) if k.blend.is_some() => {
                        let d = inst.matrix.w_axis.truncate().distance_squared(view.eye);
                        blended.push((d, i, p, k));
                    }
                    Some(k) => opaque.push((0.0, i, p, k)),
                    None => {}
                }
            }
        }
        blended.sort_by(|a, b| b.0.total_cmp(&a.0));
        let draws: Vec<(f32, usize, usize, PipelineKey)> =
            opaque.into_iter().chain(blended).collect();
        for d in &draws {
            self.pipeline(d.3);
        }
        let particle_draws: Vec<(wgpu::Buffer, u32, (PBlend, bool), Option<&'static str>)> = particles
            .iter()
            .filter(|b| !b.vertices.is_empty())
            .map(|b| {
                let key = (b.blend, b.lines);
                self.particle_pipeline(key);
                let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.globals_bind, &[]);
            for &(_, i, p, key) in &draws {
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
                    None => model.materials.get(part.material),
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
                pass.set_vertex_buffer(0, part.channel.as_ref().map_or(&mesh.vertex, |c| &c.0).slice(..));
                pass.set_index_buffer(part.index.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..part.count, 0, 0..1);
            }
            // Particles last, over everything, testing depth without writing it.
            for (buf, count, key, texture) in &particle_draws {
                let Some(pipeline) = self.particle_pipelines.get(key) else {
                    continue;
                };
                let bind = texture.and_then(|t| self.named_textures.get(t)).unwrap_or(&self.particle_white);
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

/// A vertex of the 2D interface: clip-space position, texture coordinates, colour.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct UiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

fn ui_pipeline(device: &wgpu::Device, textures: &wgpu::BindGroupLayout, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
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
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
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
        Some((Vec3::from(*m.positions.get(i as usize)?), glam::Vec2::from(*m.uvs.get(i as usize)?)))
    };
    for t in m.parts.iter().flat_map(|p| p.indices.as_chunks::<3>().0) {
        let (Some((p0, w0)), Some((p1, w1)), Some((p2, w2))) = (at(t[0]), at(t[1]), at(t[2])) else { continue };
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
            let nrm = m.normals.get(i).map(|&v| Vec3::from(v).normalize_or(Vec3::Y)).unwrap_or(Vec3::Y);
            let u = tan.get(i).copied().unwrap_or_default();
            let t = (u - nrm * nrm.dot(u)).normalize_or_zero();
            let t = if t == Vec3::ZERO { nrm.any_orthonormal_vector() } else { t };
            let b = bit.get(i).copied().unwrap_or_default();
            let w = if nrm.cross(t).dot(b) < 0.0 { -1.0 } else { 1.0 };
            [t.x, t.y, t.z, w]
        })
        .collect()
}

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> wgpu::TextureView {
    let levels = 32 - width.max(height).max(1).leading_zeros();
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Not sRGB: the original's fixed-function pipeline lights and blends in gamma space, and so do we.
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (mut w, mut h, mut data) = (width, height, rgba);
    for level in 0..levels {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        if w == 1 && h == 1 {
            break;
        }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4usize {
                    let px = |xx: u32, yy: u32| {
                        let (xx, yy) = (xx.min(w - 1), yy.min(h - 1));
                        u32::from(
                            data.get(((yy * w + xx) * 4) as usize + c)
                                .copied()
                                .unwrap_or(0),
                        )
                    };
                    let s = px(2 * x, 2 * y)
                        + px(2 * x + 1, 2 * y)
                        + px(2 * x, 2 * y + 1)
                        + px(2 * x + 1, 2 * y + 1);
                    if let Some(d) = next.get_mut(((y * nw + x) * 4) as usize + c) {
                        *d = (s / 4) as u8;
                    }
                }
            }
        }
        (w, h, data) = (nw, nh, next);
    }
    tex.create_view(&Default::default())
}
