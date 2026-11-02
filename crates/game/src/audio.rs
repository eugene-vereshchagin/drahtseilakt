// Sound: the chosen sound theme's sounds and music (assets/sounds/<theme> and the player's own themes; Ogg Vorbis
// or WAV, sound_themes.rs) or the WAV files of the player's copy (Sounds/), mixed in software and played through the
// system's output (cpal: ALSA on Linux, WASAPI on Windows, CoreAudio on macOS). The game asks for sounds by name, as
// the original's Wave Player and Play Sound Instance blocks do. The level's music is decoded ahead on a thread of its
// own, as our pieces are long. Without an output device the game runs silent.

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

/// A sound by its name: from the chosen sound theme (sound_themes.rs: the system or generated theme shipped with the
/// game, or a player's own, "original" among them), else its fallback theme; a sound no theme has comes from the
/// copy's WAV (a level's own sounds), never music: until the original's theme is chosen only our own music plays.
/// `name` may be a role's other file, `<role>#<k>` (music.rs `key`): the `k`-th of the role's pool, then the role's first.
fn load(data: &Data, name: &str) -> Option<Arc<Sound>> {
    let (name, k) = match name.split_once('#') {
        Some((role, k)) => (role, k.parse::<usize>().unwrap_or(0)),
        None => (name, 0),
    };
    let copy = gamedata::sound_themes::plays_copy();
    // a level's own sound (a community level brings its own) comes before any theme's
    let own = data.level_own(&format!("Sounds/{name}.wav")).into_iter();
    // another file of a pool is that file or nothing: falling back to the role's first could play a track twice in a
    // row (music.rs drops a track that does not decode; a sound plays its first file instead)
    if k > 0 {
        let variant = if copy { None } else { gamedata::sound_themes::pool(name).into_iter().nth(k) };
        return variant.and_then(|path| decode_file(&path));
    }
    let themed = if copy { Vec::new() } else { gamedata::sound_themes::files(name) };
    for path in own.chain(themed) {
        // a broken file is passed over for the next theme's
        if let Some(s) = decode_file(&path) {
            return Some(s);
        }
    }
    if !copy && name.starts_with("Music_") && !name.eq_ignore_ascii_case("Music_thunder") {
        return None;
    }
    let path = data.file(&format!("Sounds/{name}.wav"))?;
    // it may come from a level's game folder (a stranger's): read through the cap, never past it
    let bytes = read_capped(&path)?;
    match parse_wav(&bytes) {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            eprintln!("sound {}: {e}", path.display());
            None
        }
    }
}

/// A theme's sound file decoded (Ogg Vorbis or WAV, by its extension); none, with a note, when it is broken.
fn decode_file(path: &std::path::Path) -> Option<Arc<Sound>> {
    let ogg = path.extension().is_some_and(|x| x.eq_ignore_ascii_case("ogg"));
    let decoded = read_capped(path)
        .ok_or_else(|| "unreadable".to_owned())
        .and_then(|b| if ogg { parse_ogg(&b) } else { parse_wav(&b) });
    match decoded {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            eprintln!("sound {}: {e}", path.display());
            None
        }
    }
}

/// Music as the audio layer names it: a role beginning with Music_ (a pool's other file too), not the thunder (an
/// effect despite its name).
fn is_music(name: &str) -> bool {
    let role = name.split('#').next().unwrap_or(name);
    role.starts_with("Music_") && !role.eq_ignore_ascii_case("Music_thunder")
}

/// The most decoded music held at once, in frames (stereo, 8 bytes each: 384 MB); a piece that would go over it is
/// not played, so that a theme of very long pieces cannot take the machine's memory.
const MUSIC_BUDGET: usize = 48 << 20;

/// The most frames an Ogg Vorbis sound may decode to (ten minutes at 48 kHz).
const MAX_OGG_FRAMES: usize = 48_000 * 600;

/// Decodes an Ogg Vorbis file (mono or stereo). Our own files are read through the same cap and checks as the
/// copy's: a malformed or too long file is refused.
pub fn parse_ogg(bytes: &[u8]) -> Result<Sound, String> {
    if bytes.len() > MAX_WAV_BYTES {
        return Err("too large".into());
    }
    let mut r = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes))
        .map_err(|e| e.to_string())?;
    let (channels, rate) = (r.ident_hdr.audio_channels, r.ident_hdr.audio_sample_rate);
    if !(1..=2).contains(&channels) || !(1000..=192_000).contains(&rate) {
        return Err(format!("unsupported: {channels} channels, {rate} Hz"));
    }
    let mut frames: Vec<[f32; 2]> = Vec::new();
    while let Some(packet) = r.read_dec_packet_itl().map_err(|e| e.to_string())? {
        if channels == 1 {
            frames.extend(packet.iter().map(|&v| {
                let x = f32::from(v) / 32768.0;
                [x, x]
            }));
        } else {
            frames.extend(
                packet
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|[l, r]| [f32::from(*l) / 32768.0, f32::from(*r) / 32768.0]),
            );
        }
        if frames.len() > MAX_OGG_FRAMES {
            return Err("too long".into());
        }
    }
    // grown by doubling: the room never used is given back (a piece of music would keep nearly twice its size)
    frames.shrink_to_fit();
    Ok(Sound { frames, rate })
}

/// The largest WAV file read (the game's longest music part is about 4 MB).
const MAX_WAV_BYTES: usize = 64 << 20;

/// Reads an uncompressed PCM WAV file (8 or 16 bit, mono or stereo). The file is the player's, so every read is
/// checked and a malformed file is refused.
/// A sound file's bytes, at most MAX_WAV_BYTES of a plain file (more is refused, never read).
fn read_capped(path: &std::path::Path) -> Option<Vec<u8>> {
    use std::io::Read;
    if !std::fs::metadata(path).is_ok_and(|m| m.is_file()) {
        return None;
    }
    let cap = u64::try_from(MAX_WAV_BYTES).ok()?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= MAX_WAV_BYTES).then_some(bytes)
}

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
    /// Music (its own volume setting) rather than a sound.
    music: bool,
}

#[derive(Default)]
struct Mixer {
    voices: Vec<Voice>,
    rate: u32,
    /// The player's volumes for sounds and for music (the Settings page).
    volumes: [f32; 2],
}

impl Mixer {
    /// Adds the voices into `out` (interleaved, `channels` a frame).
    fn fill(&mut self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        let dt = 1.0 / f64::from(self.rate.max(1));
        let volumes = self.volumes;
        for v in &mut self.voices {
            let gain = v.volume * volumes[usize::from(v.music)];
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
                    *o += s[c.min(1)] * gain;
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

/// A sound decoded ahead: its name and the sound (None if it could not be read).
type Decoded = (String, Option<Arc<Sound>>);

pub struct Audio {
    output: Output,
    mixer: Arc<Mutex<Mixer>>,
    sounds: HashMap<String, Option<Arc<Sound>>>,
    next: u64,
    /// Sounds being decoded ahead (by name), and where their decoding thread sends them.
    pending: std::collections::HashSet<String>,
    decoded: (
        std::sync::mpsc::Sender<Decoded>,
        std::sync::mpsc::Receiver<Decoded>,
    ),
    variants: Variants,
}

/// The sounds' pools (a theme may give a sound several files): how many each has and which played last, and the
/// generator that picks among them (a sound's pick never touches the game).
#[derive(Default)]
struct Variants {
    rng: Option<crate::music::Rng>,
    pools: HashMap<String, (usize, Option<usize>)>,
}

impl Variants {
    /// The key to play the sound `name` by: one of its pool's files at random, never the same twice in a row; the
    /// sound itself when a level brings its own, the copy's sounds play or the pool has one file.
    fn size(&mut self, data: &Data, name: &str) -> usize {
        self.entry(data, name).0
    }

    fn entry(&mut self, data: &Data, name: &str) -> &mut (usize, Option<usize>) {
        self.pools.entry(name.to_owned()).or_insert_with(|| {
            let own = data.level_own(&format!("Sounds/{name}.wav")).is_some();
            let copy = gamedata::sound_themes::plays_copy();
            (if own || copy { 1 } else { gamedata::sound_themes::pool(name).len() }, None)
        })
    }

    fn pick(&mut self, data: &Data, name: &str) -> String {
        if self.entry(data, name).0 <= 1 {
            return name.to_owned();
        }
        let rng = self.rng.get_or_insert_with(|| {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
            crate::music::Rng::new(now.map_or(1, |d| d.as_nanos() as u64))
        });
        let (n, last) = self.pools.entry(name.to_owned()).or_insert((1, None));
        let k = crate::music::pick(rng, *n, *last);
        *last = Some(k);
        crate::music::key(name, k)
    }
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
                    volumes: [1.0; 2],
                })),
                sounds: HashMap::new(),
                next: 1,
                pending: std::collections::HashSet::new(),
                decoded: std::sync::mpsc::channel(),
                variants: Variants::default(),
            });
        }
        let device = cpal::default_host().default_output_device()?;
        let config = device.default_output_config().ok()?;
        let channels = usize::from(config.channels());
        let mixer = Arc::new(Mutex::new(Mixer {
            voices: Vec::new(),
            rate: config.sample_rate(),
            volumes: [1.0; 2],
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
            pending: std::collections::HashSet::new(),
            decoded: std::sync::mpsc::channel(),
            variants: Variants::default(),
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

    /// Forgets the sounds loaded (a new level may bring its own).
    pub fn forget_sounds(&mut self) {
        self.sounds.clear();
        // what is still being decoded was the old level's: its thread sends into a channel no longer listened to
        // (and stops), and the new level decodes its own afresh
        self.pending.clear();
        self.decoded = std::sync::mpsc::channel();
        // a new level or theme may have other pools
        self.variants.pools.clear();
    }

    /// A sound of the game by its name (the original's file name without `.wav`), loaded once: our own when our own
    /// sounds play and we have it, else the copy's (our own music never falls back to the copy's). None if there is
    /// none, it is broken, or it is still being decoded ahead.
    fn sound(&mut self, data: &Data, name: &str) -> Option<Arc<Sound>> {
        self.receive();
        if self.pending.contains(name) {
            return None;
        }
        if let Some(s) = self.sounds.get(name) {
            return s.clone();
        }
        let s = self.admit(name, load(data, name));
        self.sounds.insert(name.to_owned(), s.clone());
        s
    }

    /// Takes in what the decoding thread has finished.
    fn receive(&mut self) {
        while let Ok((n, s)) = self.decoded.1.try_recv() {
            self.pending.remove(&n);
            let s = self.admit(&n, s);
            self.sounds.insert(n, s);
        }
    }

    /// A decoded sound as it is kept: music only within MUSIC_BUDGET together with the music held already.
    fn admit(&self, name: &str, s: Option<Arc<Sound>>) -> Option<Arc<Sound>> {
        let s = s?;
        if !is_music(name) {
            return Some(s);
        }
        let held: usize = self
            .sounds
            .iter()
            .filter(|(k, _)| is_music(k))
            .filter_map(|(_, v)| v.as_ref().map(|v| v.frames.len()))
            .fold(0, usize::saturating_add);
        if held.saturating_add(s.frames.len()) > MUSIC_BUDGET {
            eprintln!("music {name}: over the budget of decoded music, not played");
            return None;
        }
        Some(s)
    }

    /// The sound `name` was tried and cannot be played (broken, missing, over the music budget).
    pub fn failed(&mut self, name: &str) -> bool {
        self.receive();
        !self.pending.contains(name) && matches!(self.sounds.get(name), Some(None))
    }

    /// The sound `name` is being decoded ahead.
    pub fn pending(&mut self, name: &str) -> bool {
        self.receive();
        self.pending.contains(name)
    }

    /// Lets go of the decoded music not in `keep` (what plays now and may play next; a piece still playing is held
    /// by the mixer until it ends). What could not be decoded is remembered, so as not to be tried again.
    pub fn keep_music(&mut self, keep: &[String]) {
        self.receive();
        self.sounds
            .retain(|k, v| !is_music(k) || v.is_none() || keep.contains(k));
    }

    /// The sounds' picks come from this seed from now on (a replay's: the same each time it plays).
    pub fn seed_sounds(&mut self, seed: u64) {
        self.variants.rng = Some(crate::music::Rng::new(seed));
    }

    /// Decodes ahead the other files of the sounds' pools (the first files are decoded when first played, as
    /// always), so that a pick is not decoded while the game runs.
    pub fn preload_sound_pools(&mut self, data: &Data) {
        let mut keys = Vec::new();
        for name in gamedata::sound_themes::KNOWN_SOUNDS.iter().filter(|n| !is_music(n)) {
            let n = self.variants.size(data, name);
            keys.extend((1..n).map(|k| crate::music::key(name, k)));
        }
        self.preload(data, &keys);
    }

    /// Decodes these sounds on a thread of their own (the level's music: long pieces), so that none is decoded while
    /// the game runs; until one is ready it is not played.
    pub fn preload(&mut self, data: &Data, names: &[String]) {
        let wanted: Vec<String> = names
            .iter()
            .filter(|n| !self.sounds.contains_key(*n) && !self.pending.contains(*n))
            .cloned()
            .collect();
        if wanted.is_empty() {
            return;
        }
        let (send, data) = (
            self.decoded.0.clone(),
            Data {
                root: data.root.clone(),
                overlay: data.overlay.clone(),
            },
        );
        self.pending.extend(wanted.iter().cloned());
        let names = wanted.clone();
        let started = std::thread::Builder::new().name("music decoding".into()).spawn(move || {
            for n in wanted {
                // a decoder that fails hard leaves that piece silent, never the game stopped or the name pending
                let s = std::panic::catch_unwind(|| load(&data, &n)).unwrap_or(None);
                if send.send((n, s)).is_err() {
                    return;
                }
            }
        });
        if started.is_err() {
            // no thread to be had: they are decoded when first played instead
            for n in names {
                self.pending.remove(&n);
            }
        }
    }

    /// Plays a sound once, after `wait` seconds, at a volume (0 to 1). Returns its handle.
    pub fn play(&mut self, data: &Data, name: &str, volume: f32, wait: f64) -> Option<u64> {
        self.start(data, name, volume, wait, false, false)
    }

    /// Plays a sound over and over until stopped.
    pub fn play_loop(&mut self, data: &Data, name: &str, volume: f32) -> Option<u64> {
        self.start(data, name, volume, 0.0, true, false)
    }

    /// Plays music once (at the music volume).
    pub fn play_music(&mut self, data: &Data, name: &str) -> Option<u64> {
        self.start(data, name, 1.0, 0.0, false, true)
    }

    /// Plays music over and over until stopped.
    pub fn play_music_loop(&mut self, data: &Data, name: &str) -> Option<u64> {
        self.start(data, name, 1.0, 0.0, true, true)
    }

    /// The player's volumes for sounds and for music (0 to 1).
    pub fn set_volumes(&mut self, sound: f32, music: f32) {
        if let Ok(mut m) = self.mixer.lock() {
            m.volumes = [sound.clamp(0.0, 1.0), music.clamp(0.0, 1.0)];
        }
    }

    fn start(
        &mut self,
        data: &Data,
        name: &str,
        volume: f32,
        wait: f64,
        looping: bool,
        music: bool,
    ) -> Option<u64> {
        // a sound plays one of its pool's files (the music picks its own: music.rs)
        let picked = (!music).then(|| self.variants.pick(data, name));
        // a pick still being decoded, or broken, plays the sound's first file instead
        let sound = match picked.as_deref().map(|p| self.sound(data, p)) {
            Some(Some(s)) => s,
            _ => self.sound(data, name)?,
        };
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
            music,
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

    /// The system sound theme, as shipped in assets/sounds.
    fn own() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sounds/system")
    }

    #[test]
    fn the_generated_themes_sounds_decode() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sounds/neuro/Sounds");
        let mut n = 0;
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let bytes = std::fs::read(e.path()).unwrap_or_default();
            assert!(parse_ogg(&bytes).is_ok(), "{}", e.path().display());
            n += 1;
        }
        assert!(n >= 20, "{n} files");
    }

    /// Every sound and piece the game asks for by name: the events' (app.rs), the balls' (ballsound.rs), the fans',
    /// the menu's, the thunder and all the music of every level theme (music.rs).
    fn every_role() -> Vec<String> {
        let mut v: Vec<String> = [
            "Misc_StartLevel",
            "Misc_Checkpoint",
            "Extra_Life_Blob",
            "Misc_extraball",
            "Extra_Start",
            "Extra_Hit",
            "Misc_Trafo",
            "Misc_Fall",
            "Misc_Lightning",
            "Misc_RopeTears",
            "Misc_Ventilator",
            "Menu_load",
            "Menu_click",
            "Music_thunder",
        ]
        .map(str::to_owned)
        .to_vec();
        for kind in [crate::sim::BallKind::Wood, crate::sim::BallKind::Stone, crate::sim::BallKind::Paper] {
            let (rolls, hits) = crate::ballsound::table(kind);
            v.extend(rolls.iter().chain(hits.iter()).map(|s| (*s).to_owned()));
        }
        for n in 1..=5 {
            for last in [false, true] {
                v.extend(crate::music::Music::roles(Some(n), last));
            }
        }
        v.sort();
        v.dedup();
        v
    }

    #[test]
    fn the_generated_theme_holds_every_role_without_a_fallback() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sounds/neuro");
        let txt = std::fs::read_to_string(dir.join("theme.txt")).unwrap_or_default();
        assert!(txt.lines().any(|l| l.trim() == "fallback none"), "the neuro theme takes nothing from another");
        let roles = every_role();
        assert!(roles.len() >= 50, "{} roles", roles.len());
        for name in &roles {
            let p = gamedata::sound_themes::in_theme(&dir, name);
            assert!(p.is_some(), "neuro lacks {name}");
            let bytes = p.and_then(|p| std::fs::read(p).ok()).unwrap_or_default();
            let decoded = parse_ogg(&bytes);
            assert!(decoded.as_ref().is_ok_and(|s| !s.frames.is_empty()), "{name}: {:?}", decoded.err());
        }
    }

    #[test]
    fn our_own_sounds_and_music_decode() {
        let mut n = 0;
        for folder in ["Sounds", "Music"] {
            for e in std::fs::read_dir(own().join(folder))
                .into_iter()
                .flatten()
                .flatten()
            {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "ogg") {
                    let bytes = std::fs::read(&p).unwrap_or_default();
                    let s = parse_ogg(&bytes).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                    assert!(!s.frames.is_empty() && s.rate == 44100, "{}", p.display());
                    n += 1;
                }
            }
        }
        assert!(n >= 30, "{n} files");
    }

    #[test]
    fn a_broken_ogg_is_refused() {
        let good = std::fs::read(own().join("Sounds/Extra_Hit.ogg")).unwrap_or_default();
        assert!(parse_ogg(&good).is_ok());
        assert!(parse_ogg(b"OggS not really").is_err());
        // a file cut short is read as far as it goes or refused, and never panics
        let _ = parse_ogg(&good[..good.len() / 3]);
        assert!(parse_ogg(&[]).is_err());
    }

    #[test]
    fn the_music_table_names_our_tracks() {
        // the table is read without a note, its tracks are all there, and every track in Music/ is played somewhere
        let bytes = std::fs::read(own().join("Music/music.txt")).unwrap_or_default();
        let table = gamedata::sound_themes::parse_table(&bytes);
        assert!(table.notes.is_empty(), "{:?}", table.notes);
        let named: Vec<&String> = table.roles.iter().flat_map(|(_, files)| files).collect();
        for file in &named {
            assert!(own().join("Music").join(file).is_file(), "{file}");
        }
        for e in std::fs::read_dir(own().join("Music")).into_iter().flatten().flatten() {
            let f = e.file_name().to_string_lossy().into_owned();
            let role = f.trim_end_matches(".ogg");
            if f.ends_with(".ogg") && !gamedata::sound_themes::known_role(role) {
                assert!(named.contains(&&f), "{f} plays in no role");
            }
        }
        assert!(table.roles.iter().any(|(r, _)| r == "Music_Theme_1_1"));
    }

    #[test]
    fn the_system_theme_holds_every_role_the_game_plays() {
        // every role in the shipped system theme (the theme every theme falls back to), each file of each pool
        // decoding, the level music's pools apart from one another, and themes 4 and 5 with tracks of their own
        let roles = every_role();
        for name in &roles {
            assert!(gamedata::sound_themes::known_role(name), "{name} is a role the tables accept");
            let pool = gamedata::sound_themes::pool_in_theme(&own(), name);
            assert!(!pool.is_empty(), "system lacks {name}");
            for p in &pool {
                let bytes = std::fs::read(p).unwrap_or_default();
                let decoded = parse_ogg(&bytes);
                assert!(decoded.as_ref().is_ok_and(|s| !s.frames.is_empty()), "{}: {:?}", p.display(), decoded.err());
            }
        }
        let pool = |name: &str| gamedata::sound_themes::pool_in_theme(&own(), name);
        let atmos: Vec<_> = (1..=3).flat_map(|k| pool(&format!("Music_Atmo_{k}"))).collect();
        assert_eq!(atmos.len(), 8, "the eight atmospheres");
        let theme = |n: u32| -> Vec<_> { (1..=3).flat_map(|k| pool(&format!("Music_Theme_{n}_{k}"))).collect() };
        for n in 1..=5 {
            for m in 1..n {
                assert!(theme(n).iter().all(|t| !theme(m).contains(t)), "themes {m} and {n} share a track");
            }
        }
        assert_eq!(theme(4).len() + theme(5).len(), 9, "the third round's nine tracks");
    }

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
            volumes: [1.0; 2],
        };
        m.voices.push(Voice {
            id: 1,
            sound: sound.clone(),
            pos: 0.0,
            volume: 0.5,
            pitch: 1.0,
            looping: false,
            wait: 0.0,
            music: false,
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
            music: false,
        });
        m.fill(&mut out, 2);
        assert_eq!(m.voices.len(), 1, "a loop goes on");
    }
}
