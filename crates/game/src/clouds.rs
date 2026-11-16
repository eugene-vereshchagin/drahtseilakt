// The sea of cloud under our levels: a slab of cumulus drawn as a volume (clouds.wgsl), ray-marched through tiling
// noise made here once, lit by the level's sun and sky, fading into the sky's own horizon haze far off. It is drawn
// after the opaque scene, reading its depth, so the columns and supports that reach down sink into it; the blended
// scene and the particles come after it. Only drawn: the physics never sees it. Levels without a cloud layer in their
// settings (the original's) never draw it.

use glam::{Mat4, Vec3};

/// How finely the layer is drawn (the Graphics settings' `cloud-quality`): steps along the view, steps towards the
/// sun, and the small billows on the edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Off,
    Low,
    Medium,
    High,
}

impl Quality {
    pub fn key(self) -> &'static str {
        match self {
            Quality::Off => "off",
            Quality::Low => "low",
            Quality::Medium => "medium",
            Quality::High => "high",
        }
    }

    pub fn from_key(k: &str) -> Option<Quality> {
        [Quality::Off, Quality::Low, Quality::Medium, Quality::High]
            .into_iter()
            .find(|q| q.key() == k.trim())
    }

    /// The march: steps along the view, steps towards the sun, 1 with the edges' billows.
    fn march(self) -> [f32; 3] {
        match self {
            Quality::Off => [0.0, 0.0, 0.0],
            Quality::Low => [32.0, 3.0, 0.0],
            Quality::Medium => [56.0, 4.0, 1.0],
            Quality::High => [96.0, 6.0, 1.0],
        }
    }
}

/// The quality the player chose (the Graphics settings; Off when the Clouds switch is off).
static QUALITY: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(2);

pub fn set_quality(q: Quality) {
    QUALITY.store(q as u8, std::sync::atomic::Ordering::Relaxed);
}

pub fn quality() -> Quality {
    match QUALITY.load(std::sync::atomic::Ordering::Relaxed) {
        0 => Quality::Off,
        1 => Quality::Low,
        3 => Quality::High,
        _ => Quality::Medium,
    }
}

/// The side of the 3D noise (texels; it tiles).
pub const NOISE: usize = 64;
/// The side of the 2D map of where the clouds are and how high they reach (texels; it tiles).
pub const WEATHER: usize = 256;
/// The texels round the horizon of the haze's colour by direction.
pub const RING: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    inv_view_proj: [[f32; 4]; 4],
    /// xyz the eye, w the time (seconds).
    eye: [f32; 4],
    /// xyz towards the sun.
    sun: [f32; 4],
    sun_color: [f32; 4],
    sky: [f32; 4],
    shadow: [f32; 4],
    /// x the bottom, y the top, z the cover, w the clouds' size.
    slab: [f32; 4],
    /// x steps along the view, y steps to the sun, z the haze's density a unit of distance, w how much the haze
    /// under the layer hides.
    march: [f32; 4],
    /// xy the drift (x, z a second), z 1 with the edges' billows, w the frame's width.
    wind: [f32; 4],
    /// rgb the haze's colour straight down (the horizon's average), a the frame's height.
    haze: [f32; 4],
}

/// The GPU side: the pipeline, the noise, the uniform; the bind group follows the depth texture.
pub struct Pass {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    noise: wgpu::TextureView,
    weather: wgpu::TextureView,
    ring: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind: Option<(wgpu::BindGroup, u32, u32)>,
}

/// The sea of cloud under a level that names a sky with a land below it (Mountains, Forest, Mist, Desert) and has no
/// sea of its own: a stranger's map made without the toolkit would show that land (a green swamp from far above)
/// instead of clouds. Lit like the toolkit's (`levelkit.py` CLOUD_SEA, `sky_light.json`): Mist takes Forest's light,
/// Desert Mountains'. Under Desert a closed sea (`under` 1: nothing below it shows through); under the skies whose land
/// is a mesh under the level (terrain.rs) scattered banks with gaps the land shows through, as the toolkit's levels
/// have. None for any other sky.
pub fn default_sea(sky: &str, lowest: f32) -> Option<level::Clouds> {
    let (sun, sun_color, sky_c, shadow) = match sky {
        "Mountains" | "Desert" => (
            [0.742, 0.224, 0.632],
            [2.6, 2.461, 2.051],
            [0.1448, 0.1718, 0.26],
            [0.1322, 0.1074, 0.1926],
        ),
        "Forest" | "Mist" => (
            [0.3771, 0.7422, 0.554],
            [2.5903, 2.6, 2.4851],
            [0.1998, 0.2428, 0.4016],
            [0.1824, 0.1517, 0.2975],
        ),
        _ => return None,
    };
    let open = sky != "Desert";
    Some(level::Clouds {
        top: lowest - 35.0,
        thickness: 110.0,
        cover: if open { 0.3 } else { 0.8 },
        scale: 85.0,
        sun,
        sun_color,
        sky: sky_c,
        shadow,
        haze: 0.0005,
        under: if open { 0.0 } else { 1.0 },
        wind: level::Clouds::DEFAULT.wind,
    })
}

/// What the level's layer looks like this run: its settings, the haze round the horizon (the sky's colour just above
/// it, RGBA8, gamma) and its average.
pub struct Layer {
    pub settings: level::Clouds,
    pub ring: Vec<u8>,
    pub haze: [f32; 3],
}

/// One frame's view of the layer.
pub struct Frame {
    pub view_proj: Mat4,
    pub eye: Vec3,
    pub time: f64,
    pub width: u32,
    pub height: u32,
    pub quality: Quality,
}

impl Pass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        samples: u32,
    ) -> Pass {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("clouds"),
            source: wgpu::ShaderSource::Wgsl(shader_source(samples).into()),
        });
        let tex = |binding, dim, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: dim,
                // the scene's depth (binding 4) is multisampled with anti-aliasing: each sample is read
                multisampled: binding == 4 && samples > 1,
            },
            count: None,
        };
        let float = wgpu::TextureSampleType::Float { filterable: true };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("clouds"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                tex(1, wgpu::TextureViewDimension::D3, float),
                tex(2, wgpu::TextureViewDimension::D2, float),
                tex(3, wgpu::TextureViewDimension::D2, float),
                tex(
                    4,
                    wgpu::TextureViewDimension::D2,
                    wgpu::TextureSampleType::Depth,
                ),
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("clouds"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        // premultiplied: the clouds' light added, what is behind dimmed by their opacity
        let over = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("clouds"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: over,
                        alpha: over,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("clouds"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let n = NOISE as u32;
        let noise = texture(
            device,
            queue,
            [n, n, n],
            wgpu::TextureDimension::D3,
            &noise3d(NOISE),
        );
        let w = WEATHER as u32;
        let weather = texture(
            device,
            queue,
            [w, w, 1],
            wgpu::TextureDimension::D2,
            &weather_map(WEATHER),
        );
        let ring = texture(
            device,
            queue,
            [RING as u32, 1, 1],
            wgpu::TextureDimension::D2,
            &[200u8; RING * 4],
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("clouds"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Pass {
            pipeline,
            layout,
            uniform,
            noise,
            weather,
            ring,
            sampler,
            bind: None,
        }
    }

    /// The haze round the horizon of the level's sky (RING texels, RGBA8).
    pub fn set_ring(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, rgba: &[u8]) {
        if rgba.len() == RING * 4 {
            self.ring = texture(
                device,
                queue,
                [RING as u32, 1, 1],
                wgpu::TextureDimension::D2,
                rgba,
            );
            self.bind = None;
        }
    }

    /// Draws the layer over `target`, reading the scene's depth (`depth`, `w` x `h`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        enc: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        layer: &Layer,
        frame: &Frame,
    ) {
        let c = &layer.settings;
        let [steps, light, detail] = frame.quality.march();
        let (offset, rise) = drift(c.wind, c.scale, frame.time);
        let u = Uniform {
            inv_view_proj: frame.view_proj.inverse().to_cols_array_2d(),
            eye: frame.eye.extend(rise).to_array(),
            sun: Vec3::from(c.sun)
                .normalize_or(Vec3::Y)
                .extend(0.0)
                .to_array(),
            sun_color: [c.sun_color[0], c.sun_color[1], c.sun_color[2], 0.0],
            sky: [c.sky[0], c.sky[1], c.sky[2], 0.0],
            shadow: [c.shadow[0], c.shadow[1], c.shadow[2], 0.0],
            slab: [c.top - c.thickness, c.top, c.cover, c.scale],
            march: [steps, light, c.haze, c.under],
            wind: [offset[0], offset[1], detail, frame.width as f32],
            haze: [
                layer.haze[0],
                layer.haze[1],
                layer.haze[2],
                frame.height as f32,
            ],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&u));
        let fresh =
            !matches!(&self.bind, Some((_, w, h)) if *w == frame.width && *h == frame.height);
        if fresh {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("clouds"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.noise),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.weather),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.ring),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.bind = Some((group, frame.width, frame.height));
        }
        let Some((group, _, _)) = &self.bind else {
            return;
        };
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clouds"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Forgets the bind group (the depth texture it reads was made again).
    pub fn depth_changed(&mut self) {
        self.bind = None;
    }
}

/// The shader: the built-in one, or in a debug build, for tuning the look, the file DRAHTSEILAKT_CLOUD_SHADER names.
fn shader_source(samples: u32) -> String {
    #[cfg(debug_assertions)]
    if let Some(text) =
        std::env::var_os("DRAHTSEILAKT_CLOUD_SHADER").and_then(|p| std::fs::read_to_string(p).ok())
    {
        return depth_binding(text, samples);
    }
    depth_binding(include_str!("clouds.wgsl").to_owned(), samples)
}

/// Every texture of the layer repeats over this many clouds' sizes (clouds.wgsl: the map of banks every 30 and its
/// heaps every 5, the noise every 4 and its billows every 0.75), so the drift is wrapped at it without a jump.
const DRIFT_PERIOD: f32 = 60.0;
/// The billows' slow rise repeats every this many seconds (clouds.wgsl: 0.004 of their texture a second).
const RISE_PERIOD: f32 = 250.0;

/// The drift after `time` seconds of `wind` (x, z a second) over clouds `size` across, wrapped at DRIFT_PERIOD, and
/// the clock the billows rise by, wrapped at RISE_PERIOD: small numbers however long the run, where 32-bit floats keep
/// their precision.
fn drift(wind: [f32; 2], size: f32, time: f64) -> ([f32; 2], f32) {
    let period = f64::from(DRIFT_PERIOD * size.max(1e-3));
    let at = |w: f32| (f64::from(w) * time).rem_euclid(period) as f32;
    (
        [at(wind[0]), at(wind[1])],
        time.rem_euclid(f64::from(RISE_PERIOD)) as f32,
    )
}

fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    [w, h, d]: [u32; 3],
    dimension: wgpu::TextureDimension,
    rgba: &[u8],
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: d,
    };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("clouds"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * w),
            rows_per_image: Some(h),
        },
        size,
    );
    tex.create_view(&Default::default())
}

// --- the noise -------------------------------------------------------------------------------------------------------

/// A hash of integer coordinates to 0..1 (the same on every platform: integer arithmetic only).
fn hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f)
        ^ seed.wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Cellular (Worley) noise tiling every `cells` cells over 0..1: 1 at a feature point falling to 0 a cell away, so
/// it is a field of round blobs (the billows).
fn worley(p: [f32; 3], cells: i32, seed: u32) -> f32 {
    let q = [
        p[0] * cells as f32,
        p[1] * cells as f32,
        p[2] * cells as f32,
    ];
    let c = q.map(|v| v.floor() as i32);
    let mut best = f32::MAX;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y, z) = (c[0] + dx, c[1] + dy, c[2] + dz);
                let (wx, wy, wz) = (
                    x.rem_euclid(cells),
                    y.rem_euclid(cells),
                    z.rem_euclid(cells),
                );
                let f = [
                    x as f32 + hash(wx, wy, wz, seed),
                    y as f32 + hash(wx, wy, wz, seed + 1),
                    z as f32 + hash(wx, wy, wz, seed + 2),
                ];
                let d = (f[0] - q[0]).powi(2) + (f[1] - q[1]).powi(2) + (f[2] - q[2]).powi(2);
                best = best.min(d);
            }
        }
    }
    (1.0 - best.sqrt()).clamp(0.0, 1.0)
}

/// Gradient (Perlin) noise tiling every `cells` cells over 0..1, about -1..1.
fn perlin(p: [f32; 3], cells: i32, seed: u32) -> f32 {
    let q = [
        p[0] * cells as f32,
        p[1] * cells as f32,
        p[2] * cells as f32,
    ];
    let c = q.map(|v| v.floor() as i32);
    let f = [q[0] - c[0] as f32, q[1] - c[1] as f32, q[2] - c[2] as f32];
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let grad = |x: i32, y: i32, z: i32, d: [f32; 3]| {
        let (x, y, z) = (
            x.rem_euclid(cells),
            y.rem_euclid(cells),
            z.rem_euclid(cells),
        );
        // one of the twelve edge directions of a cube
        let k = (hash(x, y, z, seed) * 12.0) as u32 % 12;
        let (a, b) = match k / 4 {
            0 => (d[0], d[1]),
            1 => (d[0], d[2]),
            _ => (d[1], d[2]),
        };
        (if k & 1 == 0 { a } else { -a }) + (if k & 2 == 0 { b } else { -b })
    };
    let mut out = [0.0f32; 8];
    for (i, o) in out.iter_mut().enumerate() {
        let (dx, dy, dz) = ((i & 1) as i32, ((i >> 1) & 1) as i32, ((i >> 2) & 1) as i32);
        *o = grad(
            c[0] + dx,
            c[1] + dy,
            c[2] + dz,
            [f[0] - dx as f32, f[1] - dy as f32, f[2] - dz as f32],
        );
    }
    let (u, v, w) = (fade(f[0]), fade(f[1]), fade(f[2]));
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = l(out[0], out[1], u);
    let x10 = l(out[2], out[3], u);
    let x01 = l(out[4], out[5], u);
    let x11 = l(out[6], out[7], u);
    l(l(x00, x10, v), l(x01, x11, v), w)
}

/// Billows at three sizes, larger ones weighing more.
fn worley_fbm(p: [f32; 3], cells: i32, seed: u32) -> f32 {
    worley(p, cells, seed) * 0.625
        + worley(p, cells * 2, seed + 7) * 0.25
        + worley(p, cells * 4, seed + 13) * 0.125
}

fn remap(v: f32, lo: f32, hi: f32) -> f32 {
    ((v - lo) / (hi - lo).max(1e-5)).clamp(0.0, 1.0)
}

/// The 3D noise, `n` texels a side, tiling: R the clouds' body (gradient noise whose valleys the billows round off,
/// "Perlin-Worley"), G, B and A billows at 4, 8 and 16 cells a side.
pub fn noise3d(n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n * n * n * 4];
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    for (i, px) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (x, y, z) = (i % n, (i / n) % n, i / (n * n));
        let p = [
            (x as f32 + 0.5) / n as f32,
            (y as f32 + 0.5) / n as f32,
            (z as f32 + 0.5) / n as f32,
        ];
        let mut g = 0.0;
        let mut amp = 1.0;
        let mut norm = 0.0;
        for o in 0..4 {
            g += perlin(p, 4 << o, 101 + o as u32) * amp;
            norm += amp;
            amp *= 0.5;
        }
        let g = (g / norm * 0.5 + 0.5).clamp(0.0, 1.0);
        let w = worley_fbm(p, 4, 11);
        let body = remap(g, w - 1.0, 1.0);
        px[0] = byte(body);
        px[1] = byte(w);
        px[2] = byte(worley_fbm(p, 8, 23));
        px[3] = byte(worley_fbm(p, 16, 37));
    }
    out
}

/// The map over the layer, `n` texels a side, tiling: R how cloudy each place is (large banks broken by gaps), G how
/// high its clouds reach.
pub fn weather_map(n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n * n * 4];
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    for (i, px) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (x, y) = (i % n, i / n);
        let p = [
            (x as f32 + 0.5) / n as f32,
            (y as f32 + 0.5) / n as f32,
            0.37,
        ];
        let mut g = 0.0;
        let mut amp = 1.0;
        let mut norm = 0.0;
        for o in 0..5 {
            g += perlin(p, 4 << o, 211 + o as u32) * amp;
            norm += amp;
            amp *= 0.55;
        }
        let g = (g / norm * 0.5 + 0.5).clamp(0.0, 1.0);
        let banks = worley(p, 6, 301) * 0.5 + worley(p, 12, 302) * 0.3 + worley(p, 24, 303) * 0.2;
        px[0] = byte(remap(g * 0.55 + banks * 0.45, 0.2, 0.8));
        let mut h = 0.0;
        let mut amp = 1.0;
        for o in 0..3 {
            h += perlin(p, 3 << o, 401 + o as u32) * amp;
            amp *= 0.5;
        }
        px[1] = byte(h * 0.6 + 0.5);
        px[2] = 0;
        px[3] = 255;
    }
    out
}

// --- the haze from the sky -------------------------------------------------------------------------------------------

/// A face of the sky box (sky.rs: Front, Right, Back, Left, Down, Top) and where on its texture (0..1, rows down) a
/// direction meets it.
fn face_at(d: Vec3) -> (usize, f32, f32) {
    let a = d.abs();
    if a.x >= a.y && a.x >= a.z {
        let u = (-d.z / d.x + 1.0) / 2.0;
        let v = (1.0 - d.y / a.x) / 2.0;
        (if d.x > 0.0 { 1 } else { 3 }, u, v)
    } else if a.z >= a.y {
        let u = (d.x / d.z + 1.0) / 2.0;
        let v = (1.0 - d.y / a.z) / 2.0;
        (if d.z > 0.0 { 0 } else { 2 }, u, v)
    } else if d.y > 0.0 {
        // the top: its bottom edge on the front side, x to the right
        (5, (d.x / a.y + 1.0) / 2.0, (1.0 - d.z / a.y) / 2.0)
    } else {
        (4, (d.x / a.y + 1.0) / 2.0, (1.0 + d.z / a.y) / 2.0)
    }
}

/// The haze round the horizon from the sky's side faces (`faces`: Front, Right, Back, Left as width, height, RGBA):
/// the colour a little above the horizon by direction (u = atan2(x, z) / 2 pi + 1/2, as clouds.wgsl reads it),
/// averaged over a few degrees, and the average of it all.
pub fn horizon_ring(faces: &[Option<(u32, u32, Vec<u8>)>]) -> Option<(Vec<u8>, [f32; 3])> {
    ring_at(faces, [0.4, 1.0, 1.6])
}

/// The colour round the circle at three elevations (degrees; below the horizon if negative), as `horizon_ring`.
pub fn ring_at(
    faces: &[Option<(u32, u32, Vec<u8>)>],
    elevations: [f32; 3],
) -> Option<(Vec<u8>, [f32; 3])> {
    let mut ring = vec![[0.0f32; 3]; RING];
    for (i, slot) in ring.iter_mut().enumerate() {
        let mut sum = [0.0f32; 3];
        let mut n = 0.0;
        for k in 0..9 {
            let u = (i as f32 + 0.5 + (k % 3) as f32 / 3.0 - 0.33) / RING as f32;
            let az = (u - 0.5) * std::f32::consts::TAU;
            let el = elevations[k / 3].to_radians();
            let d = Vec3::new(az.sin() * el.cos(), el.sin(), az.cos() * el.cos());
            let (face, fu, fv) = face_at(d);
            let (w, h, rgba) = faces.get(face)?.as_ref()?;
            let x = ((fu.clamp(0.0, 1.0) * *w as f32) as u32).min(w.saturating_sub(1));
            let y = ((fv.clamp(0.0, 1.0) * *h as f32) as u32).min(h.saturating_sub(1));
            let at = (y as usize * *w as usize + x as usize) * 4;
            let px = rgba.get(at..at + 3)?;
            for c in 0..3 {
                sum[c] += f32::from(px[c]) / 255.0;
            }
            n += 1.0;
        }
        *slot = sum.map(|s| s / n);
    }
    let mean = ring
        .iter()
        .fold([0.0f32; 3], |a, c| [a[0] + c[0], a[1] + c[1], a[2] + c[2]])
        .map(|s| s / RING as f32);
    let bytes = ring
        .iter()
        .flat_map(|c| [c[0], c[1], c[2], 1.0].map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8))
        .collect();
    Some((bytes, mean))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drift stays small however long the run, and wrapping it moves every texture by whole repeats.
    #[test]
    fn the_drift_wraps_at_the_textures_period() {
        let (o, r) = drift([7.0, -3.0], 85.0, 1.0e7);
        assert!(
            o.iter().all(|v| (0.0..DRIFT_PERIOD * 85.0).contains(v))
                && (0.0..RISE_PERIOD).contains(&r)
        );
        for repeat in [30.0f32, 5.0, 4.0, 0.75] {
            let n = DRIFT_PERIOD / repeat;
            assert!((n - n.round()).abs() < 1e-4, "{repeat}");
        }
        assert_eq!(drift([2.0, 0.0], 10.0, 1.0).0, [2.0, 0.0]);
    }

    #[test]
    fn qualities_read_back() {
        for q in [Quality::Off, Quality::Low, Quality::Medium, Quality::High] {
            assert_eq!(Quality::from_key(q.key()), Some(q));
        }
        assert_eq!(Quality::from_key("ultra"), None);
    }

    /// The noise tiles: the texel past the last is the first (checked on the functions, at a small size).
    #[test]
    fn the_noise_tiles() {
        for (a, b) in [
            ([0.0, 0.3, 0.6], [1.0, 0.3, 0.6]),
            ([0.2, 0.0, 0.9], [0.2, 1.0, 0.9]),
        ] {
            assert!((worley(a, 4, 1) - worley(b, 4, 1)).abs() < 1e-4);
            assert!((perlin(a, 8, 1) - perlin(b, 8, 1)).abs() < 1e-4);
        }
        let n = noise3d(8);
        assert_eq!(n.len(), 8 * 8 * 8 * 4);
        let w = weather_map(16);
        assert!(w.chunks_exact(4).any(|p| p[0] > 128) && w.chunks_exact(4).any(|p| p[0] < 128));
    }

    /// The sky's faces are met where sky.rs puts them: the front's middle straight ahead, the right face's left edge
    /// where it meets the front.
    #[test]
    fn directions_meet_the_faces_as_drawn() {
        let (f, u, v) = face_at(Vec3::Z);
        assert_eq!(
            (f, (u * 100.0).round(), (v * 100.0).round()),
            (0, 50.0, 50.0)
        );
        let (f, u, _) = face_at(Vec3::new(1.0, 0.0, 0.999));
        assert_eq!((f, (u * 100.0).round()), (1, 0.0));
        let (f, u, _) = face_at(Vec3::new(0.999, 0.0, 1.0));
        assert_eq!((f, (u * 100.0).round()), (0, 100.0));
        let (f, _, v) = face_at(Vec3::new(0.0, -1.0, 0.5));
        assert_eq!((f, (v * 100.0).round()), (4, 75.0));
    }

    #[test]
    fn the_ring_takes_the_horizon() {
        // four faces, each one colour above its middle row and black below
        let face = |c: [u8; 3]| {
            let mut px = vec![0u8; 16 * 16 * 4];
            for (i, p) in px.chunks_exact_mut(4).enumerate() {
                if i / 16 < 8 {
                    p[..3].copy_from_slice(&c);
                }
            }
            Some((16, 16, px))
        };
        let faces = [
            face([255, 0, 0]),
            face([0, 255, 0]),
            face([0, 0, 255]),
            face([255, 255, 0]),
        ];
        let (ring, _) = horizon_ring(&faces).unwrap();
        // straight ahead (+z, u = 1/2) is the front's red
        let mid = RING / 2 * 4;
        assert_eq!(&ring[mid..mid + 3], &[255, 0, 0]);
        // +x (u = 3/4) is the right face's green
        let right = RING * 3 / 4 * 4;
        assert_eq!(&ring[right..right + 3], &[0, 255, 0]);
        assert!(horizon_ring(&faces[..2]).is_none());
    }
}

/// The shader's depth binding as the scene's depth texture is: multisampled with anti-aliasing (its sample 0 is read).
fn depth_binding(text: String, samples: u32) -> String {
    let text = format!("const NS: i32 = {};\n{text}", samples.max(1));
    if samples > 1 {
        text.replace("texture_depth_2d", "texture_depth_multisampled_2d")
    } else {
        text
    }
}
