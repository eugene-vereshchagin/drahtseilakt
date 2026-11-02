// The music of a level, as the original's Music_Manager plays it (3D Entities/Sound.nmo): an atmosphere part
// (Music_Atmo_1..3) after a random pause of up to 15 s, again and again; from 7 s on the level's theme
// (Music_Theme_<n>_1..3, n from the level table) after random pauses of up to 50 s; after the last checkpoint the
// theme stops, and within 200 of the level's end Music_EndCheckpoint loops; the finish plays Music_Final
// (Music_LastFinal after the last level). It runs on the game's time, so it waits with the run.

/// A part not there yet (being decoded ahead) is tried again after this many seconds.
const RETRY: f64 = 1.0;

use crate::assets::Data;
use crate::audio::Audio;
use crate::sim::{Sim, TICK};

/// One of the music's random players (Music_Atmo, Music_Theme): it waits a random pause of up to `pause` seconds,
/// plays one of its three parts, and waits again when the part has ended.
struct Player {
    pause: f64,
    wait_until: Option<f64>,
    playing: Option<u64>,
}

pub struct Music {
    /// The run's tick it began at.
    began: u64,
    theme: Option<u32>,
    last_level: bool,
    atmo: Player,
    themes: Player,
    end_loop: Option<u64>,
    finished: bool,
    /// A small random generator (the original's Random blocks; the music never touches the game).
    seed: u64,
}

/// Seconds before the theme's first pause begins, and the longest random pauses.
const THEME_FROM: f64 = 7.0;
const ATMO_PAUSE: f64 = 15.0;
const THEME_PAUSE: f64 = 50.0;
/// Within this of the level's end, after the last checkpoint, the end music loops.
const END_DISTANCE: f32 = 200.0;

impl Music {
    /// The music a level with theme `theme` may play, to be decoded ahead.
    pub fn wanted(theme: Option<u32>, last_level: bool) -> Vec<String> {
        let mut v: Vec<String> = (1..=3).map(|k| format!("Music_Atmo_{k}")).collect();
        if let Some(n) = theme {
            v.extend((1..=3).map(|k| format!("Music_Theme_{n}_{k}")));
        }
        v.push("Music_EndCheckpoint".to_owned());
        v.push(
            if last_level {
                "Music_LastFinal"
            } else {
                "Music_Final"
            }
            .to_owned(),
        );
        v
    }

    /// The music of a level with theme `theme`; `last_level`: the finish plays the last level's music.
    pub fn new(theme: Option<u32>, last_level: bool, seed: u64, tick: u64) -> Music {
        let mut m = Music {
            began: tick,
            theme,
            last_level,
            atmo: Player {
                pause: ATMO_PAUSE,
                wait_until: None,
                playing: None,
            },
            themes: Player {
                pause: THEME_PAUSE,
                wait_until: None,
                playing: None,
            },
            end_loop: None,
            finished: false,
            seed: seed | 1,
        };
        m.atmo.wait_until = Some(m.random() * ATMO_PAUSE);
        m.themes.wait_until = Some(THEME_FROM + m.random() * THEME_PAUSE);
        m
    }

    fn random(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Starts a player's next part when its pause is over, and begins its next pause when its part has ended.
    fn run(&mut self, theme: bool, name: &str, now: f64, audio: &mut Audio, data: &Data) {
        let r = (self.random(), self.random());
        let p = if theme {
            &mut self.themes
        } else {
            &mut self.atmo
        };
        if p.playing.is_some_and(|id| !audio.is_playing(id)) {
            p.playing = None;
            p.wait_until = Some(now + r.0 * p.pause);
        }
        if p.playing.is_none() && p.wait_until.is_some_and(|t| now >= t) {
            let part = 1 + (r.1 * 3.0) as u32 % 3;
            p.playing = audio.play_music(data, &format!("{name}_{part}"));
            // a part not there (or still being decoded) is tried again a little later
            p.wait_until = if p.playing.is_some() {
                None
            } else {
                Some(now + RETRY)
            };
        }
    }

    /// One frame of the run: start, stop and change the music as the run is.
    pub fn frame(&mut self, sim: &Sim, audio: &mut Audio, data: &Data) {
        if self.finished {
            return;
        }
        if sim.finished {
            self.stop(audio);
            let name = if self.last_level {
                "Music_LastFinal"
            } else {
                "Music_Final"
            };
            audio.play_music(data, name);
            return;
        }
        let now = sim.tick.saturating_sub(self.began) as f64 * f64::from(TICK);
        let (last_stage, end) = sim.last_stage();
        self.run(false, "Music_Atmo", now, audio, data);
        match self.theme {
            _ if last_stage => {
                if let Some(id) = self.themes.playing.take() {
                    audio.stop(id);
                }
                self.themes.wait_until = None;
            }
            Some(n) => self.run(true, &format!("Music_Theme_{n}"), now, audio, data),
            None => {}
        }
        let near = last_stage && end.is_some_and(|e| e.distance(sim.ball.position) < END_DISTANCE);
        match (near, self.end_loop) {
            (true, None) => self.end_loop = audio.play_music_loop(data, "Music_EndCheckpoint"),
            (false, Some(id)) => {
                audio.stop(id);
                self.end_loop = None;
            }
            _ => {}
        }
    }

    /// The music stops (the run is over, left or restarted).
    pub fn stop(&mut self, audio: &mut Audio) {
        for id in [
            self.atmo.playing.take(),
            self.themes.playing.take(),
            self.end_loop.take(),
        ]
        .into_iter()
        .flatten()
        {
            audio.stop(id);
        }
        self.finished = true;
    }
}
