// Sound: the game's WAV files from the player's copy (Sounds/), mixed in software and played through the system's
// output (cpal: ALSA on Linux, WASAPI on Windows, CoreAudio on macOS). The game asks for sounds by name, as the
// original's Wave Player and Play Sound Instance blocks do; without an output device it runs silent.

use crate::assets::Data;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A decoded sound: stereo frames at its own rate.
pub struct Sound {
    frames: Vec<[f32; 2]>,
    rate: u32,
}

/// The rate of a file output.
const FILE_RATE: u32 = 44100;

impl Drop for Audio {
    /// A file output is written when the game ends.
    fn drop(&mut self) {
        if let Output::File { path, samples } = &self.output {
            let data: Vec<u8> = samples
                .iter()
                .flat_map(|s| ((s * 32767.0) as i16).to_le_bytes())
                .collect();
            let mut out = b"RIFF".to_vec();
            out.extend((36 + data.len() as u32).to_le_bytes());
            out.extend(b"WAVEfmt ");
            out.extend(16u32.to_le_bytes());
            out.extend(1u16.to_le_bytes());
            out.extend(2u16.to_le_bytes());
            out.extend(FILE_RATE.to_le_bytes());
            out.extend((FILE_RATE * 4).to_le_bytes());
            out.extend(4u16.to_le_bytes());
            out.extend(16u16.to_le_bytes());
            out.extend(b"data");
            out.extend((data.len() as u32).to_le_bytes());
            out.extend(data);
            if let Err(e) = std::fs::write(path, out) {
                eprintln!("sound file {}: {e}", path.display());
            }
        }
    }
}

/// The largest WAV file read (the game's longest music part is about 4 MB).
const MAX_WAV_BYTES: usize = 64 << 20;

/// Reads an uncompressed PCM WAV file (8 or 16 bit, mono or stereo). The file is the player's, so every read is
/// checked and a malformed file is refused.
pub fn parse_wav(bytes: &[u8]) -> Result<Sound, String> {
    if bytes.len() > MAX_WAV_BYTES {
        return Err("too large".into());
    }
    let u16_at = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err("not a RIFF WAVE file".into());
    }
    let mut at = 12usize;
    let mut format: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while let (Some(id), Some(len)) = (bytes.get(at..at + 4), u32_at(at + 4)) {
        let body = at.checked_add(8).ok_or("bad chunk")?;
        let end = body.checked_add(len as usize).ok_or("bad chunk")?;
        let chunk = bytes.get(body..end.min(bytes.len())).ok_or("bad chunk")?;
        match id {
            b"fmt " => {
                let tag = u16_at(body).ok_or("short fmt")?;
                let channels = u16_at(body + 2).ok_or("short fmt")?;
                let rate = u32_at(body + 4).ok_or("short fmt")?;
                let bits = u16_at(body + 14).ok_or("short fmt")?;
                format = Some((tag, channels, rate, bits));
            }
            b"data" => data = Some(chunk),
            _ => {}
        }
        // chunks are padded to an even length
        at = end.checked_add(len as usize & 1).ok_or("bad chunk")?;
    }
    let (tag, channels, rate, bits) = format.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    if tag != 1
        || !(1..=2).contains(&channels)
        || !(1000..=192_000).contains(&rate)
        || !(bits == 8 || bits == 16)
    {
        return Err(format!(
            "unsupported format {tag}, {channels} channels, {rate} Hz, {bits} bit"
        ));
    }
    let sample = |b: &[u8]| -> f32 {
        match b {
            [x] => (f32::from(*x) - 128.0) / 128.0,
            [lo, hi] => f32::from(i16::from_le_bytes([*lo, *hi])) / 32768.0,
            _ => 0.0,
        }
    };
    let width = usize::from(bits / 8);
    let frame = width * usize::from(channels);
    let frames = data
        .chunks_exact(frame)
        .map(|f| {
            let l = sample(f.get(..width).unwrap_or_default());
            let r = if channels == 2 {
                sample(f.get(width..).unwrap_or_default())
            } else {
                l
            };
            [l, r]
        })
        .collect();
    Ok(Sound { frames, rate })
}

/// One sound playing: where it is (in its own frames), how fast it moves through them, its volume and whether
/// it loops; `wait` seconds before it starts.
struct Voice {
    id: u64,
    sound: Arc<Sound>,
    pos: f64,
    volume: f32,
    /// How fast it plays (1: as recorded), as the original's Pitch Control.
    pitch: f32,
    looping: bool,
    wait: f64,
}

#[derive(Default)]
struct Mixer {
    voices: Vec<Voice>,
    rate: u32,
}

impl Mixer {
    /// Adds the voices into `out` (interleaved, `channels` a frame).
    fn fill(&mut self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        let dt = 1.0 / f64::from(self.rate.max(1));
        for v in &mut self.voices {
            let step = f64::from(v.sound.rate) * dt * f64::from(v.pitch.max(0.0));
            for frame in out.chunks_mut(channels.max(1)) {
                if v.wait > 0.0 {
                    v.wait -= dt;
                    continue;
                }
                let n = v.sound.frames.len();
                if n == 0 {
                    break;
                }
                if v.pos as usize >= n {
                    if !v.looping {
                        break;
                    }
                    v.pos -= n as f64;
                }
                // linear between the two nearest frames
                let i = v.pos as usize;
                let t = (v.pos - i as f64) as f32;
                let a = v.sound.frames.get(i).copied().unwrap_or([0.0; 2]);
                let b = v
                    .sound
                    .frames
                    .get(i + 1)
                    .or(if v.looping {
                        v.sound.frames.first()
                    } else {
                        None
                    })
                    .copied()
                    .unwrap_or(a);
                let s = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                for (c, o) in frame.iter_mut().enumerate() {
                    *o += s[c.min(1)] * v.volume;
                }
                v.pos += step;
            }
        }
        self.voices
            .retain(|v| v.looping || v.wait > 0.0 || (v.pos as usize) < v.sound.frames.len());
        for o in out.iter_mut() {
            *o = o.clamp(-1.0, 1.0);
        }
    }
}

/// The game's sound: the output stream, the mixer it plays, and the sounds loaded so far.
/// Where the mix goes: the system's output, or a WAV file (DRAHTSEILAKT_SOUND_FILE, for checking the sound
/// without speakers), mixed as the game's frames pass.
enum Output {
    Device {
        _stream: cpal::Stream,
    },
    File {
        path: std::path::PathBuf,
        samples: Vec<f32>,
    },
}

pub struct Audio {
    output: Output,
    mixer: Arc<Mutex<Mixer>>,
    sounds: HashMap<String, Option<Arc<Sound>>>,
    next: u64,
}

impl Audio {
    /// The default output device, or None (no device, or it refused a stream): the game is then silent. With
    /// DRAHTSEILAKT_SOUND_FILE set, the mix is written to that WAV file instead (44100 Hz stereo, 16 bit).
    pub fn open() -> Option<Audio> {
        if let Some(path) = std::env::var_os("DRAHTSEILAKT_SOUND_FILE") {
            return Some(Audio {
                output: Output::File {
                    path: path.into(),
                    samples: Vec::new(),
                },
                mixer: Arc::new(Mutex::new(Mixer {
                    voices: Vec::new(),
                    rate: FILE_RATE,
                })),
                sounds: HashMap::new(),
                next: 1,
            });
        }
        let device = cpal::default_host().default_output_device()?;
        let config = device.default_output_config().ok()?;
        let channels = usize::from(config.channels());
        let mixer = Arc::new(Mutex::new(Mixer {
            voices: Vec::new(),
            rate: config.sample_rate(),
        }));
        let m = mixer.clone();
        let err = |e| eprintln!("sound: {e}");
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                config.config(),
                move |out: &mut [f32], _| {
                    if let Ok(mut m) = m.lock() {
                        m.fill(out, channels);
                    }
                },
                err,
                None,
            ),
            cpal::SampleFormat::I16 => {
                let mut buf = Vec::new();
                device.build_output_stream(
                    config.config(),
                    move |out: &mut [i16], _| {
                        buf.resize(out.len(), 0.0);
                        if let Ok(mut m) = m.lock() {
                            m.fill(&mut buf, channels);
                        }
                        for (o, s) in out.iter_mut().zip(&buf) {
                            *o = (s * 32767.0) as i16;
                        }
                    },
                    err,
                    None,
                )
            }
            _ => return None,
        }
        .ok()?;
        stream.play().ok()?;
        Some(Audio {
            output: Output::Device { _stream: stream },
            mixer,
            sounds: HashMap::new(),
            next: 1,
        })
    }

    /// `seconds` of the game passed: with a file output, that much is mixed into it (a device mixes by itself).
    pub fn pass(&mut self, seconds: f64) {
        if let Output::File { samples, .. } = &mut self.output {
            let frames = (seconds * f64::from(FILE_RATE)).round() as usize;
            let mut buf = vec![0.0f32; frames * 2];
            if let Ok(mut m) = self.mixer.lock() {
                m.fill(&mut buf, 2);
            }
            samples.extend(buf);
        }
    }

    /// A sound of the game by its file name without `.wav`, loaded once (None if the copy lacks it or it is broken).
    fn sound(&mut self, data: &Data, name: &str) -> Option<Arc<Sound>> {
        self.sounds
            .entry(name.to_owned())
            .or_insert_with(|| {
                let path = data.file(&format!("Sounds/{name}.wav"))?;
                let bytes = std::fs::read(&path).ok()?;
                match parse_wav(&bytes) {
                    Ok(s) => Some(Arc::new(s)),
                    Err(e) => {
                        eprintln!("sound {}: {e}", path.display());
                        None
                    }
                }
            })
            .clone()
    }

    /// Plays a sound once, after `wait` seconds, at a volume (0 to 1). Returns its handle.
    pub fn play(&mut self, data: &Data, name: &str, volume: f32, wait: f64) -> Option<u64> {
        self.start(data, name, volume, wait, false)
    }

    /// Plays a sound over and over until stopped.
    pub fn play_loop(&mut self, data: &Data, name: &str, volume: f32) -> Option<u64> {
        self.start(data, name, volume, 0.0, true)
    }

    fn start(
        &mut self,
        data: &Data,
        name: &str,
        volume: f32,
        wait: f64,
        looping: bool,
    ) -> Option<u64> {
        let sound = self.sound(data, name)?;
        let id = self.next;
        self.next += 1;
        let mut m = self.mixer.lock().ok()?;
        m.voices.push(Voice {
            id,
            sound,
            pos: 0.0,
            volume: volume.clamp(0.0, 1.0),
            pitch: 1.0,
            looping,
            wait,
        });
        Some(id)
    }

    /// Changes a playing sound's volume.
    pub fn set_volume(&mut self, id: u64, volume: f32) {
        if let Ok(mut m) = self.mixer.lock()
            && let Some(v) = m.voices.iter_mut().find(|v| v.id == id)
        {
            // as the original's Volume Control, not limited to 1 (the rolling sound goes above it when fast)
            v.volume = volume.clamp(0.0, 4.0);
        }
    }

    /// Changes a playing sound's speed (1: as recorded).
    pub fn set_pitch(&mut self, id: u64, pitch: f32) {
        if let Ok(mut m) = self.mixer.lock()
            && let Some(v) = m.voices.iter_mut().find(|v| v.id == id)
        {
            v.pitch = pitch.clamp(0.05, 8.0);
        }
    }

    /// Whether a sound is still playing (or waiting to start).
    pub fn is_playing(&self, id: u64) -> bool {
        self.mixer
            .lock()
            .is_ok_and(|m| m.voices.iter().any(|v| v.id == id))
    }

    /// Stops a sound.
    pub fn stop(&mut self, id: u64) {
        if let Ok(mut m) = self.mixer.lock() {
            m.voices.retain(|v| v.id != id);
        }
    }

    /// Stops every sound.
    pub fn stop_all(&mut self) {
        if let Ok(mut m) = self.mixer.lock() {
            m.voices.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(channels: u16, bits: u16, rate: u32, samples: &[u8]) -> Vec<u8> {
        let mut v = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        v.extend(16u32.to_le_bytes());
        v.extend(1u16.to_le_bytes());
        v.extend(channels.to_le_bytes());
        v.extend(rate.to_le_bytes());
        v.extend((rate * u32::from(channels * bits / 8)).to_le_bytes());
        v.extend((channels * bits / 8).to_le_bytes());
        v.extend(bits.to_le_bytes());
        v.extend(b"data");
        v.extend((samples.len() as u32).to_le_bytes());
        v.extend(samples);
        v
    }

    #[test]
    fn pcm_files_are_read_and_broken_ones_refused() {
        let s = parse_wav(&wav(1, 16, 22050, &[0, 0x40, 0, 0xc0])).expect("mono 16 bit");
        assert_eq!(s.rate, 22050);
        assert_eq!(s.frames, vec![[0.5, 0.5], [-0.5, -0.5]]);
        let s = parse_wav(&wav(2, 8, 11025, &[255, 0])).expect("stereo 8 bit");
        assert_eq!(s.frames.len(), 1);
        assert!(parse_wav(b"RIFF").is_err());
        assert!(parse_wav(&wav(3, 16, 22050, &[0; 6])).is_err());
        // a data chunk claiming more than the file holds is read as far as it goes
        let mut short = wav(1, 16, 22050, &[0, 0x40]);
        let n = short.len();
        short[n - 6..n - 2].copy_from_slice(&1000u32.to_le_bytes());
        assert_eq!(parse_wav(&short).expect("truncated").frames.len(), 1);
    }

    #[test]
    fn the_mixer_resamples_loops_and_drops_finished_sounds() {
        let sound = Arc::new(Sound {
            frames: vec![[1.0, 1.0], [0.0, 0.0]],
            rate: 1,
        });
        let mut m = Mixer {
            voices: Vec::new(),
            rate: 2,
        };
        m.voices.push(Voice {
            id: 1,
            sound: sound.clone(),
            pos: 0.0,
            volume: 0.5,
            pitch: 1.0,
            looping: false,
            wait: 0.0,
        });
        let mut out = [0.0f32; 8];
        m.fill(&mut out, 2);
        // half a source frame per output frame: 1, 0.5, 0, then the end
        assert_eq!(out, [0.5, 0.5, 0.25, 0.25, 0.0, 0.0, 0.0, 0.0]);
        assert!(m.voices.is_empty());
        m.voices.push(Voice {
            id: 2,
            sound,
            pos: 0.0,
            volume: 1.0,
            pitch: 1.0,
            looping: true,
            wait: 0.0,
        });
        m.fill(&mut out, 2);
        assert_eq!(m.voices.len(), 1, "a loop goes on");
    }
}
