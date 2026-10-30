// The renderer: uploads level models (meshes, materials, textures) once and draws a list of instances each
// frame, to a window surface or to an offscreen texture (screenshots, later video). Any size and aspect ratio:
// the projection always uses the real one.

use glam::{Mat4, Vec3};
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
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
    ambient: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MaterialUniform {
    diffuse: [f32; 4],
    emissive: [f32; 4],
    params: [f32; 4],
}

struct GpuPart {
    index: wgpu::Buffer,
    count: u32,
    material: usize,
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
}

pub struct View {
    pub eye: Vec3,
    pub target: Vec3,
    pub fov_y: f32,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    material_layout: wgpu::BindGroupLayout,
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    models_buf: wgpu::Buffer,
    models_bind: wgpu::BindGroup,
    models_capacity: u64,
    model_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    pub models: Vec<Model>,
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
            entries: &[uniform_entry(0, true, wgpu::ShaderStages::VERTEX)],
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
        let white = upload_texture(&device, &queue, 1, 1, vec![255; 4]);
        Renderer {
            device,
            queue,
            format,
            shader,
            layout,
            pipelines: HashMap::new(),
            material_layout,
            globals,
            globals_bind,
            models_buf,
            models_bind,
            models_capacity: 1024,
            model_layout,
            sampler,
            white,
            models: Vec::new(),
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
                let uniform = MaterialUniform {
                    diffuse: m.diffuse,
                    emissive: [
                        m.emissive[0],
                        m.emissive[1],
                        m.emissive[2],
                        if view.is_some() { 1.0 } else { 0.0 },
                    ],
                    params: [
                        m.alpha_cutoff.unwrap_or(-1.0),
                        if m.unlit { 1.0 } else { 0.0 },
                        0.0,
                        0.0,
                    ],
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
        materials.push(self.plain_material());

        let meshes = level
            .meshes
            .iter()
            .map(|m| {
                if m.positions.is_empty() {
                    return None;
                }
                let vertices: Vec<Vertex> = (0..m.positions.len())
                    .map(|i| Vertex {
                        pos: m.positions.get(i).copied().unwrap_or_default(),
                        normal: m.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                        uv: m.uvs.get(i).copied().unwrap_or_default(),
                    })
                    .collect();
                let vertex = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&m.name),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                let parts = m
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
                    })
                    .collect();
                Some(GpuMesh { vertex, parts })
            })
            .collect();
        self.models.push(Model { meshes, materials });
        self.models.len() - 1
    }

    fn plain_material(&self) -> GpuMaterial {
        let uniform = MaterialUniform {
            diffuse: [0.8, 0.8, 0.8, 1.0],
            emissive: [0.0; 4],
            params: [-1.0, 0.0, 0.0, 0.0],
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
            ],
        });
        GpuMaterial {
            bind,
            key: PipelineKey {
                blend: None,
                z_write: true,
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
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
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

    /// Draws one frame into `target` of the given size.
    pub fn draw(
        &mut self,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        view: &View,
        instances: &[Instance],
    ) {
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let proj = perspective_lh(view.fov_y, aspect, 0.5, 3000.0);
        let look = look_at_lh(view.eye, view.target, Vec3::Y);
        let globals = Globals {
            view_proj: (proj * look).to_cols_array_2d(),
            light_dir: [0.35, -0.85, 0.4, 0.0],
            ambient: [0.38, 0.38, 0.42, 1.0],
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
            let m = inst.matrix.to_cols_array();
            let at = i * UNIFORM_ALIGN as usize;
            if let Some(dst) = matrices.get_mut(at..at + 64) {
                dst.copy_from_slice(bytemuck::cast_slice(&m));
            }
        }
        self.queue.write_buffer(&self.models_buf, 0, &matrices);

        // Opaque first, then blended back to front.
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
                let key = self
                    .models
                    .get(inst.model)
                    .and_then(|m| m.materials.get(part.material))
                    .map(|m| m.key);
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
                let Some(mat) = model.materials.get(part.material) else {
                    continue;
                };
                let Some(pipeline) = self.pipelines.get(&key) else {
                    continue;
                };
                pass.set_pipeline(pipeline);
                pass.set_bind_group(1, &self.models_bind, &[(i as u64 * UNIFORM_ALIGN) as u32]);
                pass.set_bind_group(2, &mat.bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertex.slice(..));
                pass.set_index_buffer(part.index.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..part.count, 0, 0..1);
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
                size: wgpu::BufferSize::new(64),
            }),
        }],
    });
    (buf, bind)
}

/// Uploads RGBA pixels with a full mip chain (box-filtered on the CPU).
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
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
