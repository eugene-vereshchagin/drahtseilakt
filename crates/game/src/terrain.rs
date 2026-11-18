// The land under a level (terrain.wgsl): a sky with a green land below (make_skies.py `land`: Mountains, Forest,
// Mist) brings it as a height grid, a colour map lit by the sky's own sun and a few numbers; the level hangs over it.
//
// Why a mesh and not the sky box: the sky box is at an infinite distance, so its land never moves as the camera does
// and its features read as seen from an airliner (the owner, 2026-11-16, with Ballex's levels as the reference: there
// the mountains, cliffs and water are a modest distance below the track, rise to about its height at the sides and
// slide past as the camera moves). Here the land is real geometry a few hundred units under the lowest floor: it is
// placed so that the level lies over its lowest ground, lowered where it would still come near the track (the light
// on a lowered slope corrected as the sun would light it), and faded by distance into the sky's haze just under its
// horizon, which the sky's faces show beyond it. The physics never sees it, so replays are unchanged; the original's
// levels and every sky without these files have none.
// Reason for the allow: the land file is validated in `load` (file size and decoded sides capped, extent/heights ranges checked, grid 2..=MAX_GRID square); everything below works on that validated grid and the level model.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// The most a land's files may hold: the height grid's side and the colour map's (a stranger's texture pack could
/// carry any files of these names).
const MAX_GRID: u32 = 1025;
const MAX_TEX: u32 = 4096;
const MAX_FILE: u64 = 64 << 20;

/// How far under the lowest floor the land must stay within CLEAR_NEAR of anything of the level (the clouds' banks
/// lie from 35 to 145 under it, the columns reach 240 down and fade), and how much higher it may rise for each unit
/// further away: the land rises towards the track's height at the sides, as in Ballex.
const CLEAR_DEPTH: f32 = 290.0;
const CLEAR_NEAR: f32 = 140.0;
const CLEAR_RISE: f32 = 0.85;
/// The footprint grid's cell (world units) and its most cells a side.
const CELL: f32 = 24.0;
const MAX_CELLS: usize = 512;

/// A picture decoded with its sides capped at `side` before any pixel is decoded (a file's header may claim any size;
/// checking the sides after decoding would already have allocated them).
fn decode_capped(bytes: &[u8], side: u32) -> Option<image::DynamicImage> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(side);
    limits.max_image_height = Some(side);
    reader.limits(limits);
    reader.decode().ok()
}

/// The land as its files give it.
pub struct Land {
    /// World units a side, the grid centred on its middle.
    extent: f32,
    /// Heights a side and the heights (relative to the lowest floor), row by row along z.
    n: usize,
    heights: Vec<f32>,
    /// The colour map (RGBA).
    color: (u32, u32, Vec<u8>),
    /// Towards the sun (the game's axes) and the sun's light against the sky's on the colour map.
    sun: Vec3,
    sun_over_sky: f32,
}

/// Reads a sky's land: `Textures/Sky/Sky_<name>_Land.txt`, `..._LandHeight.png`, `..._LandColor.jpg` through `file`
/// (a path under the data). None when any is missing or malformed.
pub fn load(name: &str, file: impl Fn(&str) -> Option<std::path::PathBuf>) -> Option<Land> {
    let read = |suffix: &str| -> Option<Vec<u8>> {
        let path = file(&format!("Textures/Sky/Sky_{name}_{suffix}"))?;
        if std::fs::metadata(&path).ok()?.len() > MAX_FILE {
            return None;
        }
        std::fs::read(path).ok()
    };
    let text = String::from_utf8(read("Land.txt")?).ok()?;
    let (mut extent, mut lo, mut hi, mut sun, mut ratio) = (None, None, None, None, 4.0f32);
    for line in text.lines() {
        let mut w = line.split_whitespace();
        let key = w.next();
        let v: Vec<f32> = w.filter_map(|s| s.parse().ok()).collect();
        match (key, v.as_slice()) {
            (Some("extent"), [e]) if e.is_finite() && (1000.0..=40000.0).contains(e) => {
                extent = Some(*e)
            }
            (Some("heights"), [a, b])
                if a.is_finite() && b.is_finite() && a < b && b - a < 10000.0 =>
            {
                lo = Some(*a);
                hi = Some(*b);
            }
            (Some("sun"), [x, y, z]) => sun = Some(Vec3::new(*x, *y, *z).normalize_or(Vec3::Y)),
            (Some("light"), [r]) if r.is_finite() => ratio = r.clamp(0.0, 50.0),
            _ => {}
        }
    }
    let (extent, lo, hi) = (extent?, lo?, hi?);
    let grid = decode_capped(&read("LandHeight.png")?, MAX_GRID)?;
    if grid.width() != grid.height() || !(2..=MAX_GRID).contains(&grid.width()) {
        return None;
    }
    let n = grid.width() as usize;
    let heights = grid
        .to_luma16()
        .into_raw()
        .into_iter()
        .map(|q| lo + (hi - lo) * f32::from(q) / 65535.0)
        .collect();
    let img = decode_capped(&read("LandColor.jpg")?, MAX_TEX)?;
    if img.width() > MAX_TEX || img.height() > MAX_TEX || img.width() == 0 || img.height() == 0 {
        return None;
    }
    let color = (img.width(), img.height(), img.to_rgba8().into_raw());
    Some(Land {
        extent,
        n,
        heights,
        color,
        sun: sun.unwrap_or(Vec3::Y),
        sun_over_sky: ratio,
    })
}

/// A vertex: its place, its colour map's coordinates, and how much brighter or darker its light is than the map's
/// where the land was lowered (1 elsewhere).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
    relight: f32,
}

/// The land placed under one level.
pub struct Placed {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub color: (u32, u32, Vec<u8>),
    pub sun: Vec3,
}

/// Where the level is, seen from above: the cells its visible meshes cover, and each cell's distance to them.
struct Footprint {
    x0: f32,
    z0: f32,
    cell: f32,
    w: usize,
    h: usize,
    dist: Vec<f32>,
}

impl Footprint {
    fn of(level: &level::Level) -> Option<Footprint> {
        let mut tris: Vec<[f32; 4]> = Vec::new();
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for o in level
            .objects
            .iter()
            .filter(|o| o.visible && o.sprite.is_none())
        {
            let Some(mesh) = o.mesh.and_then(|m| level.meshes.get(m)) else {
                continue;
            };
            let m = Mat4::from_cols_array_2d(&o.matrix);
            let world: Vec<Vec3> = mesh
                .positions
                .iter()
                .map(|p| m.transform_point3(Vec3::from(*p)))
                .collect();
            for part in &mesh.parts {
                for t in part.indices.as_chunks::<3>().0 {
                    let (Some(a), Some(b), Some(c)) = (
                        world.get(t[0] as usize),
                        world.get(t[1] as usize),
                        world.get(t[2] as usize),
                    ) else {
                        continue;
                    };
                    let r = [
                        a.x.min(b.x).min(c.x),
                        a.z.min(b.z).min(c.z),
                        a.x.max(b.x).max(c.x),
                        a.z.max(b.z).max(c.z),
                    ];
                    if !r.iter().all(|v| v.is_finite() && v.abs() < 1.0e6) {
                        continue;
                    }
                    lo = [lo[0].min(r[0]), lo[1].min(r[1])];
                    hi = [hi[0].max(r[2]), hi[1].max(r[3])];
                    tris.push(r);
                }
            }
        }
        if tris.is_empty() {
            return None;
        }
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let cell = CELL.max(span / (MAX_CELLS - 2) as f32);
        let w = (((hi[0] - lo[0]) / cell) as usize + 2).min(MAX_CELLS);
        let h = (((hi[1] - lo[1]) / cell) as usize + 2).min(MAX_CELLS);
        let mut dist = vec![f32::MAX; w * h];
        for r in tris {
            let i0 = ((r[0] - lo[0]) / cell) as usize;
            let j0 = ((r[1] - lo[1]) / cell) as usize;
            let i1 = (((r[2] - lo[0]) / cell) as usize).min(w - 1);
            let j1 = (((r[3] - lo[1]) / cell) as usize).min(h - 1);
            for j in j0.min(h - 1)..=j1 {
                for i in i0.min(w - 1)..=i1 {
                    dist[j * w + i] = 0.0;
                }
            }
        }
        // the distance to the nearest covered cell (two chamfer passes, sqrt 2 on the diagonals)
        let d2 = cell * std::f32::consts::SQRT_2;
        for pass in 0..2 {
            for k in 0..w * h {
                let k = if pass == 0 { k } else { w * h - 1 - k };
                let (i, j) = ((k % w) as isize, (k / w) as isize);
                let s: isize = if pass == 0 { -1 } else { 1 };
                let mut best = dist[k];
                for (di, dj, step) in [(s, 0, cell), (0, s, cell), (s, s, d2), (-s, s, d2)] {
                    let (a, b) = (i + di, j + dj);
                    if a >= 0 && b >= 0 && (a as usize) < w && (b as usize) < h {
                        best = best.min(dist[b as usize * w + a as usize] + step);
                    }
                }
                dist[k] = best;
            }
        }
        Some(Footprint {
            x0: lo[0],
            z0: lo[1],
            cell,
            w,
            h,
            dist,
        })
    }

    fn centre(&self) -> (f32, f32) {
        (
            self.x0 + self.w as f32 * self.cell / 2.0,
            self.z0 + self.h as f32 * self.cell / 2.0,
        )
    }

    /// The distance from a world point to the level (outside the grid, to its edge and on).
    fn distance(&self, x: f32, z: f32) -> f32 {
        let fi = ((x - self.x0) / self.cell).clamp(0.0, (self.w - 1) as f32);
        let fj = ((z - self.z0) / self.cell).clamp(0.0, (self.h - 1) as f32);
        let ex = (x - (self.x0 + fi * self.cell)).abs();
        let ez = (z - (self.z0 + fj * self.cell)).abs();
        let d = self.dist[fj as usize * self.w + fi as usize];
        d + (ex * ex + ez * ez).sqrt()
    }

    /// How high the land may be at a distance from the level (relative to the lowest floor).
    fn allowed(d: f32) -> f32 {
        -CLEAR_DEPTH + (d - CLEAR_NEAR).max(0.0) * CLEAR_RISE
    }
}

impl Land {
    fn at(&self, i: usize, j: usize) -> f32 {
        self.heights
            .get(j.min(self.n - 1) * self.n + i.min(self.n - 1))
            .copied()
            .unwrap_or(0.0)
    }

    /// The grid's height at a point of the land (from its corner, world units), bilinear.
    fn sample(&self, x: f32, z: f32) -> f32 {
        let step = self.extent / (self.n - 1) as f32;
        let fi = (x / step).clamp(0.0, (self.n - 1) as f32);
        let fj = (z / step).clamp(0.0, (self.n - 1) as f32);
        let (i, j) = (fi as usize, fj as usize);
        let (u, v) = (fi - i as f32, fj - j as f32);
        let a = self.at(i, j) + (self.at(i + 1, j) - self.at(i, j)) * u;
        let b = self.at(i, j + 1) + (self.at(i + 1, j + 1) - self.at(i, j + 1)) * u;
        a + (b - a) * v
    }

    /// The land under a level whose lowest floor is at `lowest`: its middle where the level lies over the lowest
    /// ground (the least lowering), lowered near the track, as a mesh with its colour map.
    pub fn place(self, level: &level::Level, lowest: f32) -> Option<Placed> {
        let fp = Footprint::of(level)?;
        let (cx, cz) = fp.centre();
        let half = self.extent / 2.0;
        // try the level's middle over a 9 x 9 of points of the land's middle part; score: how much of the land under
        // the level would have to come down (sampled every few cells)
        let reach = (half - (fp.w.max(fp.h) as f32 * fp.cell) / 2.0 - 2500.0).max(0.0);
        let stride = (fp.w.max(fp.h) / 48).max(1);
        let mut best = (f32::MAX, 0.0, 0.0);
        for a in 0..9 {
            for b in 0..9 {
                let ox = half + reach * (a as f32 / 4.0 - 1.0);
                let oz = half + reach * (b as f32 / 4.0 - 1.0);
                let mut score = 0.0;
                for j in (0..fp.h).step_by(stride) {
                    for i in (0..fp.w).step_by(stride) {
                        let d = fp.dist[j * fp.w + i];
                        if d > 600.0 {
                            continue;
                        }
                        let x = fp.x0 + i as f32 * fp.cell - cx + ox;
                        let z = fp.z0 + j as f32 * fp.cell - cz + oz;
                        score += (self.sample(x, z) - Footprint::allowed(d)).max(0.0);
                    }
                }
                if score < best.0 {
                    best = (score, ox, oz);
                }
            }
        }
        // the land's corner in the world
        let (x0, z0) = (cx - best.1, cz - best.2);
        let n = self.n;
        let step = self.extent / (n - 1) as f32;
        let mut raw = Vec::with_capacity(n * n);
        let mut cut = Vec::with_capacity(n * n);
        for j in 0..n {
            for i in 0..n {
                let h = self.at(i, j);
                let x = x0 + i as f32 * step;
                let z = z0 + j as f32 * step;
                let limit = Footprint::allowed(fp.distance(x, z));
                // a smooth minimum (no crease where the limit takes over)
                let k = 40.0;
                let t = (0.5 + 0.5 * (limit - h) / k).clamp(0.0, 1.0);
                raw.push(h);
                cut.push(limit + (h - limit) * t - k * t * (1.0 - t));
            }
        }
        let normal = |g: &[f32], i: usize, j: usize| {
            let at = |a: usize, b: usize| g[b.min(n - 1) * n + a.min(n - 1)];
            let dx = at(i + 1, j) - at(i.saturating_sub(1), j);
            let dz = at(i, j + 1) - at(i, j.saturating_sub(1));
            Vec3::new(-dx, 2.0 * step, -dz).normalize_or(Vec3::Y)
        };
        let lit = |nrm: Vec3| self.sun_over_sky * nrm.dot(self.sun).max(0.0) + 0.5 + 0.5 * nrm.y;
        let mut vertices = Vec::with_capacity(n * n);
        for j in 0..n {
            for i in 0..n {
                let k = j * n + i;
                let relight = if (raw[k] - cut[k]).abs() > 0.5 {
                    (lit(normal(&cut, i, j)) / lit(normal(&raw, i, j)).max(1e-3)).clamp(0.4, 2.5)
                } else {
                    1.0
                };
                vertices.push(Vertex {
                    pos: [x0 + i as f32 * step, lowest + cut[k], z0 + j as f32 * step],
                    uv: [i as f32 / (n - 1) as f32, j as f32 / (n - 1) as f32],
                    relight,
                });
            }
        }
        let mut indices = Vec::with_capacity((n - 1) * (n - 1) * 6);
        for j in 0..n - 1 {
            for i in 0..n - 1 {
                let a = (j * n + i) as u32;
                let b = a + 1;
                let c = a + n as u32;
                let d = c + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        Some(Placed {
            vertices,
            indices,
            color: self.color,
            sun: self.sun,
        })
    }
}

/// What a frame tells the land.
pub struct Frame {
    pub view_proj: Mat4,
    pub eye: Vec3,
    pub far: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    view_proj: [f32; 16],
    // xyz the eye, w the far plane
    eye: [f32; 4],
    // towards the sun (the grain's light and shade)
    sun: [f32; 4],
}

/// The land on the card: its pipeline, mesh and textures.
pub struct Gpu {
    pipeline: wgpu::RenderPipeline,
    /// What the pipeline was made for (the anti-aliasing can change while a level is played).
    samples: u32,
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    count: u32,
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
    sun: Vec3,
    /// What it holds on the card (bytes): the mesh and the colour map with its mips.
    pub bytes: u64,
}

impl Gpu {
    /// `ring`: the sky's haze just under its horizon round the circle (clouds::ring_at), RGBA.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        samples: u32,
        placed: Placed,
        ring: &[u8],
    ) -> Gpu {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain"),
            source: wgpu::ShaderSource::Wgsl(include_str!("terrain.wgsl").into()),
        });
        let tex = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                tex(1),
                tex(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = make_pipeline(device, &shader, &pipeline_layout, format, samples);
        let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain"),
            contents: bytemuck::cast_slice(&placed.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain"),
            contents: bytemuck::cast_slice(&placed.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sun = placed.sun;
        let (w, h, rgba) = placed.color;
        let color = crate::render::upload_texture(device, queue, w, h, rgba);
        let ring_w = (ring.len() / 4).max(1) as u32;
        let ring_px = if ring.len() >= 4 {
            ring.to_vec()
        } else {
            vec![200; 4]
        };
        let ring = crate::render::upload_texture(device, queue, ring_w, 1, ring_px);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("terrain"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&color),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&ring),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let bytes = (placed.vertices.len() * std::mem::size_of::<Vertex>()
            + placed.indices.len() * 4) as u64
            + u64::from(w) * u64::from(h) * 4 * 4 / 3;
        Gpu {
            pipeline,
            samples,
            shader,
            pipeline_layout,
            format,
            vertex,
            index,
            count: placed.indices.len() as u32,
            uniform,
            bind,
            sun,
            bytes,
        }
    }

    /// Makes the pipeline again when the frame's samples a pixel changed.
    pub fn ensure(&mut self, device: &wgpu::Device, samples: u32) {
        if samples != self.samples {
            self.pipeline = make_pipeline(
                device,
                &self.shader,
                &self.pipeline_layout,
                self.format,
                samples,
            );
            self.samples = samples;
        }
    }

    /// Sets the frame's view (before the frame's passes are recorded).
    pub fn prepare(&self, queue: &wgpu::Queue, f: &Frame) {
        let u = Uniform {
            view_proj: f.view_proj.to_cols_array(),
            eye: [f.eye.x, f.eye.y, f.eye.z, f.far],
            sun: [self.sun.x, self.sun.y, self.sun.z, 0.0],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&u));
    }

    /// Draws the land into the frame's pass, after the opaque scene (the sky box writes no depth: the land covers
    /// it; the level's depth hides the land behind it).
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.set_vertex_buffer(0, self.vertex.slice(..));
        pass.set_index_buffer(self.index.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.count, 0, 0..1);
    }
}

fn make_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    samples: u32,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("terrain"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32],
            })],
        },
        // both sides drawn (seen from above; a steep slope seen edge-on is never culled into a hole)
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: crate::render::DEPTH_FORMAT,
            depth_write_enabled: Some(true),
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
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn land(n: usize, h: f32) -> Land {
        Land {
            extent: 8000.0,
            n,
            heights: vec![h; n * n],
            color: (1, 1, vec![255; 4]),
            sun: Vec3::Y,
            sun_over_sky: 4.0,
        }
    }

    fn strip() -> level::Level {
        let mut l = level::Level::default();
        l.meshes.push(level::Mesh {
            positions: vec![[0.0, 0.0, 0.0], [600.0, 0.0, 0.0], [0.0, 0.0, 10.0]],
            parts: vec![level::Part {
                indices: vec![0, 1, 2],
                ..Default::default()
            }],
            ..Default::default()
        });
        l.objects.push(level::Object {
            name: "floor".into(),
            matrix: Mat4::IDENTITY.to_cols_array_2d(),
            mesh: Some(0),
            visible: true,
            sprite: None,
            parent: None,
        });
        l
    }

    /// Land higher than the track is lowered under it (by CLEAR_DEPTH) and left as it is far off.
    #[test]
    fn the_land_keeps_clear_of_the_track() {
        let placed = land(65, 200.0).place(&strip(), 10.0).unwrap();
        let near = placed
            .vertices
            .iter()
            .filter(|v| (0.0..600.0).contains(&v.pos[0]) && v.pos[2].abs() < 100.0)
            .map(|v| v.pos[1])
            .fold(f32::MIN, f32::max);
        assert!(near <= 10.0 - CLEAR_DEPTH + 1.0, "{near}");
        let far = placed
            .vertices
            .iter()
            .map(|v| v.pos[1])
            .fold(f32::MIN, f32::max);
        assert!((far - 210.0).abs() < 1.0, "{far}");
        assert_eq!(placed.indices.len(), 64 * 64 * 6);
    }

    /// A land file that is not a land is refused.
    #[test]
    fn a_malformed_land_is_refused() {
        let dir = std::env::temp_dir().join(format!("terrain-test-{}", std::process::id()));
        let sky = dir.join("Textures/Sky");
        std::fs::create_dir_all(&sky).unwrap();
        std::fs::write(sky.join("Sky_X_Land.txt"), "extent 5\nheights 1 0\n").unwrap();
        std::fs::write(sky.join("Sky_X_LandHeight.png"), b"not a png").unwrap();
        assert!(load("X", |r| Some(dir.join(r))).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
