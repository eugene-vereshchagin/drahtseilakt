// The picture of the title and loading screens: one pass over the whole window that traces a small scene of its own
// (titlescene.wgsl: the ball on a steel rail over a sea of clouds, under a sky in the mood of the level's). It needs
// no model, only the ball's own wooden texture (read and cut to mips when the pipeline is made, on its thread), and
// costs the load nothing. The very first frames show a still of it instead (splashpic.rs).

/// A sky's colours (linear): the mood of the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mood {
    zenith: [f32; 3],
    horizon: [f32; 3],
    sun: [f32; 3],
    /// The sea of clouds, lit and in shade.
    lit: [f32; 3],
    shade: [f32; 3],
    /// The sun's height over the horizon (a tangent), how many stars, how much cirrus.
    sun_height: f32,
    stars: f32,
    cirrus: f32,
}

/// The evening the title screen stands in.
pub const DUSK: Mood = Mood {
    zenith: [0.022, 0.035, 0.13],
    horizon: [0.80, 0.30, 0.10],
    sun: [1.0, 0.62, 0.30],
    lit: [0.52, 0.24, 0.20],
    shade: [0.05, 0.035, 0.11],
    sun_height: 0.065,
    stars: 1.0,
    cirrus: 0.55,
};
const NIGHT: Mood = Mood {
    zenith: [0.004, 0.008, 0.04],
    horizon: [0.035, 0.07, 0.20],
    sun: [0.36, 0.46, 0.74],
    lit: [0.07, 0.11, 0.24],
    shade: [0.008, 0.012, 0.04],
    sun_height: 0.16,
    stars: 2.2,
    cirrus: 0.3,
};
const STORM: Mood = Mood {
    zenith: [0.035, 0.045, 0.065],
    horizon: [0.30, 0.31, 0.34],
    sun: [0.34, 0.33, 0.32],
    lit: [0.22, 0.23, 0.26],
    shade: [0.03, 0.035, 0.05],
    sun_height: 0.12,
    stars: 0.0,
    cirrus: 1.0,
};
const DAY: Mood = Mood {
    zenith: [0.035, 0.15, 0.46],
    horizon: [0.80, 0.70, 0.56],
    sun: [1.0, 0.86, 0.62],
    lit: [0.95, 0.86, 0.76],
    shade: [0.16, 0.24, 0.44],
    sun_height: 0.20,
    stars: 0.0,
    cirrus: 0.5,
};
const DESERT: Mood = Mood {
    zenith: [0.10, 0.22, 0.44],
    horizon: [0.98, 0.72, 0.42],
    sun: [1.0, 0.80, 0.52],
    lit: [0.86, 0.62, 0.40],
    shade: [0.34, 0.20, 0.16],
    sun_height: 0.18,
    stars: 0.0,
    cirrus: 0.3,
};
const MIST: Mood = Mood {
    zenith: [0.24, 0.29, 0.38],
    horizon: [0.66, 0.62, 0.64],
    sun: [0.80, 0.70, 0.62],
    lit: [0.62, 0.60, 0.64],
    shade: [0.24, 0.26, 0.33],
    sun_height: 0.14,
    stars: 0.0,
    cirrus: 0.85,
};

impl Mood {
    /// The mood of a level's sky, by the sky's name (level.txt's `sky`, or the original's letters); a level that
    /// names none gets one by its number.
    pub fn of(sky: Option<&str>, number: Option<u32>) -> Mood {
        const ROUND: [Mood; 6] = [DAY, DUSK, MIST, DESERT, NIGHT, STORM];
        let by = |n: usize| ROUND.get(n % ROUND.len()).copied().unwrap_or(DUSK);
        match sky.map(str::to_ascii_lowercase).as_deref() {
            Some("sunset") => DUSK,
            Some("night") => NIGHT,
            Some("storm") => STORM,
            Some("clouds" | "sea" | "mountains" | "forest") => DAY,
            Some("desert") => DESERT,
            Some("mist") => MIST,
            // the original's skies go by a letter
            Some(s) if s.len() == 1 => by(usize::from(s.as_bytes()[0].wrapping_sub(b'a'))),
            Some(_) => DUSK,
            None => number.map_or(DUSK, |n| by(n.saturating_sub(1) as usize)),
        }
    }
}

/// What a frame of the picture shows.
#[derive(Clone, Copy, Debug)]
pub struct Shot {
    /// The loading screen (the ball on the wire at `progress`), else the title screen.
    pub loading: bool,
    /// Seconds since the screen came up.
    pub time: f32,
    /// 0 to 1 of the load.
    pub progress: f32,
    /// How much of the picture is there (it fades in and out over what is behind it).
    pub alpha: f32,
    pub mood: Mood,
}

pub struct TitleScene {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    /// Whether the ball's texture was there (else the shader paints a grain of its own).
    pub textured: bool,
}

/// The ball's texture in the game's own parts (assets/own), as RGBA with its mip chain, largest first.
fn ball_texture() -> Option<Vec<(u32, u32, Vec<u8>)>> {
    let path = gamedata::resources::own_root()?.join("3D Entities/Balls/textures/Ball_Wood.png");
    // a texture of a sane size only: this is a file of the game's folder, but it is read on the first frame
    if std::fs::metadata(&path).ok()?.len() > 8 << 20 {
        return None;
    }
    let img = image::open(&path).ok()?.into_rgba8();
    let (mut w, mut h) = img.dimensions();
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        return None;
    }
    let mut level = img.into_raw();
    let mut chain = vec![(w, h, level.clone())];
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = Vec::with_capacity((nw * nh * 4) as usize);
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4usize {
                    let at = |dx: u32, dy: u32| {
                        let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                        u32::from(level[((sy * w + sx) * 4) as usize + c])
                    };
                    next.push(((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8);
                }
            }
        }
        level = next;
        (w, h) = (nw, nh);
        chain.push((w, h, level.clone()));
    }
    Some(chain)
}

/// How many numbers a frame of the picture takes (seven vectors of four: titlescene.wgsl's `Params`).
pub const PARAMS: usize = 28;

impl TitleScene {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> TitleScene {
        let chain = ball_texture();
        let textured = chain.is_some();
        let chain = chain.unwrap_or_else(|| vec![(1, 1, vec![140, 80, 40, 255])]);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("title ball"),
            size: wgpu::Extent3d {
                width: chain[0].0,
                height: chain[0].1,
                depth_or_array_layers: 1,
            },
            mip_level_count: chain.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, (w, h, data)) in chain.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(*h),
                },
                wgpu::Extent3d {
                    width: *w,
                    height: *h,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("title scene"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    1,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
            ],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("title scene"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("title scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("titlescene.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("title scene"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("title scene"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                // the frame's parameters: one instance
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: (PARAMS * std::mem::size_of::<f32>()) as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attributes,
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
        });
        TitleScene {
            pipeline,
            bind,
            textured,
        }
    }

    /// The numbers of a frame of the picture on a target of `w` by `h` pixels: the pass's one instance.
    pub fn params((w, h): (u32, u32), shot: &Shot) -> [f32; PARAMS] {
        let m = &shot.mood;
        // the loading screen's backdrop is kept darker under a bright sky, so the words stand on it
        let bright = m.horizon.iter().sum::<f32>() / 3.0;
        let keep = (0.92 - 0.2 * bright).clamp(0.6, 0.92);
        let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
        [
            w.max(1) as f32,
            h.max(1) as f32,
            // the noise loses its grain far from zero: the clock goes round
            finite(shot.time).rem_euclid(3600.0),
            finite(shot.alpha).clamp(0.0, 1.0),
            if shot.loading { 1.0 } else { 0.0 },
            finite(shot.progress).clamp(0.0, 1.0),
            m.sun_height,
            m.stars,
            m.zenith[0],
            m.zenith[1],
            m.zenith[2],
            m.cirrus,
            m.horizon[0],
            m.horizon[1],
            m.horizon[2],
            keep,
            m.sun[0],
            m.sun[1],
            m.sun[2],
            0.0,
            m.lit[0],
            m.lit[1],
            m.lit[2],
            0.0,
            m.shade[0],
            m.shade[1],
            m.shade[2],
            0.0,
        ]
    }

    /// Draws the picture over `target`, which keeps what the picture does not cover yet: a pass added to `enc`,
    /// its parameters (`params`) in `instance`.
    pub fn pass(
        &self,
        enc: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        instance: wgpu::BufferSlice<'_>,
    ) {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("title scene"),
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
        pass.set_bind_group(0, &self.bind, &[]);
        pass.set_vertex_buffer(0, instance);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_gets_the_mood_of_its_sky() {
        assert_eq!(Mood::of(Some("Night"), None), NIGHT);
        assert_eq!(Mood::of(Some("Mountains"), Some(3)), DAY);
        assert_eq!(Mood::of(Some("Sunset"), None), DUSK);
        assert_eq!(Mood::of(None, None), DUSK);
        // the original's letters and the bare numbers go round the moods
        assert_eq!(Mood::of(Some("A"), None), DAY);
        assert_eq!(Mood::of(None, Some(2)), DUSK);
        assert_eq!(Mood::of(Some("?"), None), Mood::of(Some("?"), Some(9)));
    }

    #[test]
    fn a_frames_numbers_are_finite_and_in_range() {
        let shot = Shot {
            loading: true,
            time: f32::NAN,
            progress: 7.0,
            alpha: f32::INFINITY,
            mood: DUSK,
        };
        let p = TitleScene::params((0, 0), &shot);
        assert!(p.iter().all(|v| v.is_finite()));
        assert_eq!((p[0], p[1], p[2], p[3], p[5]), (1.0, 1.0, 0.0, 0.0, 1.0));
    }
}
