// The music of a level, as the original's Music_Manager plays it (3D Entities/Sound.nmo): an atmosphere part
// (Music_Atmo_1..3) after a random pause of up to 15 s, again and again; from 7 s on the level's theme
// (Music_Theme_<n>_1..3, n from the level table) after random pauses of up to 50 s; after the last checkpoint the
// theme stops, and within 200 of the level's end Music_EndCheckpoint loops; the finish plays Music_Final
// (Music_LastFinal after the last level). It runs on the game's time, so it waits with the run.
//
// A sound theme may give each role a pool of tracks (sound_themes.rs, docs/drahtseilakt/sound-themes.md): a player
// (the atmosphere, the theme) chooses among every track of its parts' pools and its group's (Music_Atmo,
// Music_Theme_<n>), never the same twice in a row; the end loop and the finish music pick one of theirs. The picks come
// from the seed given (each player its own generator), so the same seed plays the same order. The music only reads
// the run: it never changes the simulation.

use std::path::PathBuf;

/// A part not there yet (being decoded ahead) is tried again after this many seconds.
const RETRY: f64 = 1.0;

use crate::assets::Data;
use crate::audio::Audio;
use crate::sim::{Sim, TICK};

/// A small random generator (the original's Random blocks; the music never touches the game).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    /// A number in [0, 1).
    pub fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// One of `n` choices at random, other than `last` when there is another.
pub fn pick(rng: &mut Rng, n: usize, last: Option<usize>) -> usize {
    match last {
        Some(l) if n > 1 && l < n => {
            let k = ((rng.next() * (n - 1) as f64) as usize).min(n - 2);
            if k >= l { k + 1 } else { k }
        }
        _ => ((rng.next() * n as f64) as usize).min(n.saturating_sub(1)),
    }
}

/// The name the audio layer plays the `k`-th file of a role's pool by: the role itself for the first, `<role>#<k>`
/// for the others (a `#` is never in a sound's name).
pub fn key(role: &str, k: usize) -> String {
    if k == 0 {
        role.to_owned()
    } else {
        format!("{role}#{k}")
    }
}

/// The tracks of these roles, as keys to play, each file once.
fn candidates(roles: &[String], pool: &dyn Fn(&str) -> Vec<PathBuf>) -> Vec<String> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut keys = Vec::new();
    for role in roles {
        for (k, path) in pool(role).into_iter().enumerate() {
            if !seen.contains(&path) {
                seen.push(path);
                keys.push(key(role, k));
            }
        }
    }
    keys
}

/// A role's pool in the theme playing; while the copy's own sounds play, the part itself (the copy has one file each).
fn theme_pool(role: &str) -> Vec<PathBuf> {
    if gamedata::sound_themes::plays_copy() {
        let group = role == "Music_Atmo"
            || role
                .strip_prefix("Music_Theme_")
                .is_some_and(|r| !r.contains('_'));
        return if group {
            Vec::new()
        } else {
            vec![PathBuf::from(role)]
        };
    }
    gamedata::sound_themes::pool(role)
}

/// A pool of tracks to play one after another at random: the next one is chosen ahead (so that it can be decoded
/// ahead), never the one played last when there is another; a track that cannot be decoded is dropped from it.
struct Pool {
    tracks: Vec<String>,
    next: Option<usize>,
    last: Option<usize>,
    rng: Rng,
}

impl Pool {
    fn new(tracks: Vec<String>, seed: u64) -> Pool {
        let mut rng = Rng::new(seed);
        let next = (!tracks.is_empty()).then(|| pick(&mut rng, tracks.len(), None));
        Pool {
            tracks,
            next,
            last: None,
            rng,
        }
    }

    fn upcoming(&self) -> Option<&String> {
        self.tracks.get(self.next?)
    }

    /// The next track has begun: the one after it is chosen.
    fn played(&mut self) {
        self.last = self.next;
        self.next =
            (!self.tracks.is_empty()).then(|| pick(&mut self.rng, self.tracks.len(), self.last));
    }

    /// Drops the next track while it could not be decoded (broken, missing, over the music budget) and chooses
    /// another, so that one bad file never silences the pool.
    fn drop_failed(&mut self, failed: &mut dyn FnMut(&str) -> bool) {
        while let Some(k) = self.next {
            match self.tracks.get(k) {
                Some(t) if failed(t) => {}
                Some(_) => return,
                None => {
                    self.next = None;
                    return;
                }
            }
            self.tracks.remove(k);
            self.last = match self.last {
                Some(l) if l == k => None,
                Some(l) if l > k => Some(l.saturating_sub(1)),
                l => l,
            };
            self.next = (!self.tracks.is_empty())
                .then(|| pick(&mut self.rng, self.tracks.len(), self.last));
        }
    }

    /// The track playing (the one last begun).
    fn current(&self) -> Option<&String> {
        self.tracks.get(self.last?)
    }
}

/// One of the music's random players (Music_Atmo, Music_Theme): it waits a random pause of up to `pause` seconds,
/// plays one of its tracks, and waits again when the track has ended.
struct Player {
    pause: f64,
    wait_until: Option<f64>,
    playing: Option<u64>,
    pool: Pool,
}

impl Player {
    fn new(pause: f64, tracks: Vec<String>, seed: u64) -> Player {
        Player {
            pause,
            wait_until: None,
            playing: None,
            pool: Pool::new(tracks, seed),
        }
    }
}

pub struct Music {
    /// The run's tick it began at.
    began: u64,
    theme: Option<u32>,
    atmo: Player,
    themes: Player,
    end_loop: Option<u64>,
    /// The end loop's pool and the finish music's (one of each plays).
    end: Pool,
    fin: Pool,
    /// The run is finished: the other music has stopped, the finish music is waited for (while it is decoded).
    finishing: bool,
    finished: bool,
}

/// Seconds before the theme's first pause begins, and the longest random pauses.
const THEME_FROM: f64 = 7.0;
const ATMO_PAUSE: f64 = 15.0;
const THEME_PAUSE: f64 = 50.0;
/// Within this of the level's end, after the last checkpoint, the end music loops.
const END_DISTANCE: f32 = 200.0;

/// Whether the end music loops now: after the last checkpoint, within END_DISTANCE of the level's end.
pub fn near_end(sim: &Sim) -> bool {
    let (last_stage, end) = sim.last_stage();
    last_stage && end.is_some_and(|e| e.distance(sim.ball.position) < END_DISTANCE)
}

impl Music {
    /// The roles a level with theme `theme` may play (its parts; the groups' pools come on top): what a theme complete
    /// on its own holds (the tests').
    #[cfg(test)]
    pub fn roles(theme: Option<u32>, last_level: bool) -> Vec<String> {
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

    /// The music of a level with theme `theme`, from the theme playing; `last_level`: the finish plays the last
    /// level's music.
    pub fn new(theme: Option<u32>, last_level: bool, seed: u64, tick: u64) -> Music {
        Music::with_pools(theme, last_level, seed, tick, &theme_pool)
    }

    /// The same with the pools given (a test's).
    fn with_pools(
        theme: Option<u32>,
        last_level: bool,
        seed: u64,
        tick: u64,
        pool: &dyn Fn(&str) -> Vec<PathBuf>,
    ) -> Music {
        let group = |g: String, parts: &str| {
            let mut roles = vec![g];
            roles.extend((1..=3).map(|k| format!("{parts}_{k}")));
            roles
        };
        let atmo = candidates(&group("Music_Atmo".into(), "Music_Atmo"), pool);
        let themes = theme.map_or_else(Vec::new, |n| {
            candidates(
                &group(format!("Music_Theme_{n}"), &format!("Music_Theme_{n}")),
                pool,
            )
        });
        // each pool its own generator: the order each plays in follows from the seed alone
        let fin = candidates(
            &[if last_level {
                "Music_LastFinal"
            } else {
                "Music_Final"
            }
            .to_owned()],
            pool,
        );
        let mut m = Music {
            began: tick,
            theme,
            atmo: Player::new(ATMO_PAUSE, atmo, seed ^ 0x9e37_79b9_7f4a_7c15),
            themes: Player::new(THEME_PAUSE, themes, seed ^ 0xc2b2_ae3d_27d4_eb4f),
            end_loop: None,
            end: Pool::new(candidates(&["Music_EndCheckpoint".to_owned()], pool), seed),
            fin: Pool::new(fin, seed.rotate_left(17)),
            finishing: false,
            finished: false,
        };
        m.atmo.wait_until = Some(m.atmo.pool.rng.next() * ATMO_PAUSE);
        m.themes.wait_until = Some(THEME_FROM + m.themes.pool.rng.next() * THEME_PAUSE);
        m
    }

    /// What may play next, to be decoded ahead: each player's next track, the end loop and the finish music.
    pub fn upcoming(&self) -> Vec<String> {
        [
            self.atmo.pool.upcoming(),
            self.themes.pool.upcoming(),
            self.end.upcoming(),
            self.fin.upcoming(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect()
    }

    /// The music to keep decoded: what may play next and what plays now (the rest may be let go).
    pub fn keep(&self) -> Vec<String> {
        let mut v = self.upcoming();
        let playing = [
            self.atmo.playing.and(self.atmo.pool.current()),
            self.themes.playing.and(self.themes.pool.current()),
            self.end_loop.and(self.end.upcoming()),
        ];
        v.extend(playing.into_iter().flatten().cloned());
        v
    }

    /// Starts a player's next track when its pause is over, and begins its next pause when its track has ended.
    fn run(&mut self, theme: bool, now: f64, audio: &mut Audio, data: &Data) {
        let p = if theme {
            &mut self.themes
        } else {
            &mut self.atmo
        };
        if p.playing.is_some_and(|id| !audio.is_playing(id)) {
            p.playing = None;
            p.wait_until = Some(now + p.pool.rng.next() * p.pause);
        }
        if p.playing.is_none() && p.wait_until.is_some_and(|t| now >= t) {
            p.pool.drop_failed(&mut |t| audio.failed(t));
            let Some(track) = p.pool.upcoming() else {
                return;
            };
            p.playing = audio.play_music(data, track);
            // a track still being decoded is tried again a little later
            p.wait_until = if p.playing.is_some() {
                p.pool.played();
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
        if sim.finished || self.finishing {
            if !self.finishing {
                self.stop(audio);
                self.finished = false;
                self.finishing = true;
            }
            // the finish music once, as soon as it is decoded (another of its pool if it cannot be)
            self.fin.drop_failed(&mut |t| audio.failed(t));
            match self.fin.upcoming() {
                Some(t) if audio.pending(t) => {}
                Some(t) => {
                    audio.play_music(data, t);
                    self.finished = true;
                }
                None => self.finished = true,
            }
            return;
        }
        let now = sim.tick.saturating_sub(self.began) as f64 * f64::from(TICK);
        let (last_stage, _) = sim.last_stage();
        self.run(false, now, audio, data);
        match self.theme {
            _ if last_stage => {
                if let Some(id) = self.themes.playing.take() {
                    audio.stop(id);
                }
                self.themes.wait_until = None;
            }
            Some(_) => self.run(true, now, audio, data),
            None => {}
        }
        match (near_end(sim), self.end_loop) {
            (true, None) => {
                self.end.drop_failed(&mut |t| audio.failed(t));
                if let Some(track) = self.end.upcoming() {
                    self.end_loop = audio.play_music_loop(data, track);
                }
            }
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// A theme with three atmospheres in parts 1 and 2 and one more in the group, theme 4 with two parts' tracks,
    /// two finish pieces and one end loop.
    fn pools(role: &str) -> Vec<PathBuf> {
        let files: &[&str] = match role {
            "Music_Atmo" => &["a3.ogg", "a1.ogg"],
            "Music_Atmo_1" => &["a1.ogg", "a2.ogg"],
            "Music_Atmo_2" => &["a4.ogg"],
            "Music_Theme_4_1" => &["t1.ogg", "t2.ogg"],
            "Music_Theme_4_3" => &["t3.ogg"],
            "Music_Final" => &["f1.ogg", "f2.ogg"],
            "Music_EndCheckpoint" => &["e.ogg"],
            _ => &[],
        };
        files.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn a_player_chooses_among_every_track_of_its_parts_and_group() {
        let m = Music::with_pools(Some(4), false, 7, 0, &pools);
        assert_eq!(
            m.atmo.pool.tracks,
            [
                "Music_Atmo",
                "Music_Atmo#1",
                "Music_Atmo_1#1",
                "Music_Atmo_2"
            ]
        );
        assert_eq!(
            m.themes.pool.tracks,
            ["Music_Theme_4_1", "Music_Theme_4_1#1", "Music_Theme_4_3"]
        );
        assert_eq!(m.end.tracks, ["Music_EndCheckpoint"]);
        assert_eq!(m.fin.tracks, ["Music_Final", "Music_Final#1"]);
        let none = Music::with_pools(None, true, 7, 0, &pools);
        assert!(none.themes.pool.tracks.is_empty() && none.fin.upcoming().is_none());
        assert_eq!(none.upcoming().len(), 2, "an atmosphere and the end loop");
    }

    /// The order a pool plays its tracks in.
    fn order(seed: u64, n: usize) -> Vec<usize> {
        let mut p = Pool::new((0..4).map(|k| key("Music_Atmo", k)).collect(), seed);
        (0..n)
            .map(|_| {
                let k = p.next.unwrap();
                p.played();
                k
            })
            .collect()
    }

    #[test]
    fn the_picks_follow_the_seed_and_never_repeat_a_track_at_once() {
        let a = order(12345, 200);
        assert_eq!(a, order(12345, 200), "the same seed, the same order");
        assert_ne!(a, order(54321, 200));
        assert!(
            a.windows(2).all(|w| w[0] != w[1]),
            "never the same twice in a row"
        );
        for k in 0..4 {
            assert!(a.contains(&k), "track {k} plays");
        }
        // a pool of one plays it every time; a pool of none never picks out of range
        let mut rng = Rng::new(1);
        assert_eq!(pick(&mut rng, 1, Some(0)), 0);
        assert_eq!(pick(&mut rng, 0, None), 0);
        for _ in 0..1000 {
            assert!(pick(&mut rng, 3, Some(9)) < 3);
        }
    }

    #[test]
    fn a_track_that_cannot_be_decoded_is_dropped_and_another_plays() {
        let tracks = || (0..3).map(|k| key("Music_Atmo", k)).collect::<Vec<_>>();
        for seed in 1..50 {
            let mut p = Pool::new(tracks(), seed);
            p.played();
            let playing = p.current().cloned().unwrap();
            let bad = p.upcoming().cloned().unwrap();
            p.drop_failed(&mut |t| t == bad);
            // the broken one is gone, and the next is neither it nor the one just played
            assert_eq!(p.tracks.len(), 2);
            let next = p.upcoming().cloned().unwrap();
            assert!(next != bad && next != playing, "seed {seed}: {next}");
            assert_eq!(p.current(), Some(&playing));
            // every track broken: the pool is silent, never stuck or out of range
            p.drop_failed(&mut |_| true);
            assert!(p.tracks.is_empty() && p.upcoming().is_none() && p.next.is_none());
        }
        // the next pick never repeats the track just begun
        let mut p = Pool::new(tracks(), 3);
        for _ in 0..20 {
            let before = p.upcoming().cloned();
            p.played();
            assert_ne!(p.upcoming().cloned(), before);
        }
    }
}
