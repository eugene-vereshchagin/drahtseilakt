// The picture of the title and loading screens: one pass over the whole window that traces a small scene of its own
// (titlescene.wgsl: the ball on a rope over a sea of clouds, under a sky in the mood of the level's). It needs no
// model and no texture, so it is there on the first frame, before anything is loaded, and costs the load nothing.

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
}

/// How many numbers a frame of the picture takes (seven vectors of four: titlescene.wgsl's `Params`).
pub const PARAMS: usize = 28;

impl TitleScene {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> TitleScene {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("title scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("titlescene.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("title scene"),
            bind_group_layouts: &[],
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
        TitleScene { pipeline }
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
