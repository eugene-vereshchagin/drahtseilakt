// The map scripts' sounds in the audio: the commands the script host queued (scripting.rs `SoundEvent`) become
// voices in the mixer, at most MAX_SCRIPT_VOICES at a time. Sound only goes out: what a script can ask about a sound
// ("is it still playing") is answered from the run's own clock, so nothing here can change a run. A voice ends when
// its script is deactivated (the Wave Player's event sends a Stop), and all of them end when the run is taken back or
// the level changes.

use crate::assets::Data;
use crate::audio::Audio;
use crate::scripting::{MAX_SCRIPT_VOICES, SoundEvent};
use crate::sim::Sim;
use script::SoundCmd;
use std::collections::HashMap;

/// Where a positional sound is no longer heard, in level units from the ball (our choice: the original does not
/// place its instances, docs/gameplay_bbs.md "Sounds").
const HEARD_WITHIN: f32 = 100.0;

#[derive(Default)]
pub struct Voices {
    /// The audio voice of each wave sound a Wave Player plays, and the gain / pitch its controls set.
    players: HashMap<usize, u64>,
    gain: HashMap<usize, f32>,
    pitch: HashMap<usize, f32>,
    paused: Vec<usize>,
    /// One-shot instances, oldest first.
    instances: Vec<u64>,
    seen_tick: u64,
}

impl Voices {
    fn count(&mut self, audio: &Audio) -> usize {
        self.instances.retain(|&id| audio.is_playing(id));
        self.players.retain(|_, &mut id| audio.is_playing(id));
        self.players.len().saturating_add(self.instances.len())
    }

    fn stop_all(&mut self, audio: &mut Audio) {
        for (_, id) in self.players.drain() {
            audio.stop(id);
        }
        for id in self.instances.drain(..) {
            audio.stop(id);
        }
        self.gain.clear();
        self.pitch.clear();
        self.paused.clear();
    }

    /// Plays the events the scripts queued since the last frame; a run taken back, or without scripts, silences them.
    pub fn frame(&mut self, audio: &mut Audio, data: &Data, sim: &mut Sim) {
        let tick = sim.tick;
        let ball = sim.ball.position;
        let Some(scripts) = &mut sim.scripts else {
            self.stop_all(audio);
            self.seen_tick = 0;
            return;
        };
        if tick < self.seen_tick {
            self.stop_all(audio);
        }
        self.seen_tick = tick;
        let events: Vec<SoundEvent> = std::mem::take(&mut scripts.sound.events);
        for e in events {
            self.apply(audio, data, ball, e);
        }
    }

    fn apply(&mut self, audio: &mut Audio, data: &Data, ball: glam::Vec3, e: SoundEvent) {
        let key = e.key;
        match e.cmd {
            SoundCmd::Play { looping, .. } => {
                if let Some(old) = self.players.remove(&key) {
                    audio.stop(old);
                }
                if self.count(audio) >= MAX_SCRIPT_VOICES {
                    return;
                }
                let volume = self.gain.get(&key).copied().unwrap_or(1.0);
                let id = if looping {
                    audio.play_loop(data, &e.file, volume)
                } else {
                    audio.play(data, &e.file, volume, 0.0)
                };
                if let Some(id) = id {
                    if let Some(&p) = self.pitch.get(&key) {
                        audio.set_pitch(id, p);
                    }
                    self.players.insert(key, id);
                    self.paused.retain(|&k| k != key);
                }
            }
            SoundCmd::Stop { .. } => {
                if let Some(id) = self.players.remove(&key) {
                    audio.stop(id);
                }
                self.paused.retain(|&k| k != key);
            }
            // the mixer has no pause: a paused sound is muted and goes on (its length is kept by the run's clock)
            SoundCmd::Pause => {
                if let Some(&id) = self.players.get(&key) {
                    audio.set_volume(id, 0.0);
                    self.paused.push(key);
                }
            }
            SoundCmd::Resume => {
                if let Some(&id) = self.players.get(&key) {
                    audio.set_volume(id, self.gain.get(&key).copied().unwrap_or(1.0));
                }
                self.paused.retain(|&k| k != key);
            }
            SoundCmd::Gain(g) => {
                self.gain.insert(key, g);
                if let Some(&id) = self.players.get(&key)
                    && !self.paused.contains(&key)
                {
                    audio.set_volume(id, g);
                }
            }
            SoundCmd::Pitch(p) => {
                self.pitch.insert(key, p);
                if let Some(&id) = self.players.get(&key) {
                    audio.set_pitch(id, p);
                }
            }
            // the mixer has no panning yet
            SoundCmd::Pan(_) => {}
            SoundCmd::Instance { volume, at } => {
                if self.count(audio) >= MAX_SCRIPT_VOICES {
                    return;
                }
                let near = at.map_or(1.0, |p| {
                    let d = (glam::Vec3::from(p) - ball).length();
                    (1.0 - d / HEARD_WITHIN).clamp(0.0, 1.0)
                });
                if near > 0.0
                    && let Some(id) = audio.play(data, &e.file, volume.max(0.0) * near, 0.0)
                {
                    self.instances.push(id);
                }
            }
        }
    }
}
