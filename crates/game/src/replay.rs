// Replays: the input of every tick, stored as the ticks where it changes, in a versioned text format. The level
// is identified by a hash of its file, so a replay never runs against the wrong data. A replay holds only our
// input, never game data, so it can live in the repository.
//
//   drahtseilakt-replay 11       (1: recorded before the ball burst into pieces in transformers, 2: before the
//                                 pieces went after 20 s, 3: before the checkpoints followed their names, 4: before
//                                 the loose objects were made of their module file's mesh, 5: before the camera
//                                 looked along the reset point after a lost ball, 6: before a run begun at a sector took the
//                                 sector's own ball, 7: before the camera's start and respawn heading was the reset point's exact one, 8: before a map's own scripts ran; each played so; 9: before
//                                 a script's activation waited for the next tick, as the original's: a run of a map
//                                 with scripts in format 9 is stale and refused, one without scripts plays as before; 10: before the keyboard and
//                                 the mouse were part of the input: a map's scripts saw none of them, so a replay of
//                                 format <= 10 gives the scripts an empty keyboard and a still mouse)
//   level <file name> <fnv-1a 64 of the file, hex>
//   start <sector> <ball kind or -> <x,y,z or ->   (optional: a run begun elsewhere than the level start)
//   mod <key> <fnv-1a 64 of its mod.txt, hex> [<option>=<value>]...   (optional, at most MAX_MODS: a mod that
//                                 changes the run, so the run is modded and plays back only with that very mod;
//                                 docs/drahtseilakt/mods.md. A replay without them is a classic run, as before)
//   turn <tick> <degrees>         (optional, 0-359: the view's turn a `view.turn` mod hands the run, from this tick on)
//   <tick> <input bits, hex>      (from this tick on; tick 1 is the first)
//   keys <tick> <+|-><hex scan code>...   (format 11, optional: keys that went down (+) or up (-) from this tick on, the
//                                 DirectInput scan codes a map's scripts read; keys are up at the start)
//   mouse <tick> <x> <y> <buttons>   (format 11, optional: the mouse's position in the original's 640x480 frame and its
//                                 buttons, bit 0 left, 1 right, 2 middle, from this tick on; at the start 0 0 0)
//   pause <tick> <1|0>   (format 11, optional: the pause menu opened (1) or closed (0); the game's pause messages reach the
//                                 scripts at this tick)
//   end <last tick>

use script::InputFrame;
use std::fmt::Write as _;

pub const FORMAT: &str = "drahtseilakt-replay 11";
/// The most pause events a replay holds.
pub const MAX_PAUSES: usize = 4096;
/// The format written (the number in `FORMAT`).
pub const VERSION: u32 = 11;
/// The formats read: 1 (no bursts), 2 (the pieces lie until the ball is lost), 3 (the checkpoints in their group's
/// order), 4 (the loose objects of their stand-ins' meshes), 5 (the respawn at the start heading), 6, 7 (a run begun at a sector
/// keeps the sector's ball), 8 (the camera's exact heading), 9 (a map's own scripts run; before, they did not) and 10
/// (the scripts run as the original's behaviour manager: an activation applies at the next tick) and 11 (the keyboard
/// and the mouse are in the input stream, for a map's scripts).
const FORMATS: [&str; 11] = [
    "drahtseilakt-replay 1",
    "drahtseilakt-replay 2",
    "drahtseilakt-replay 3",
    "drahtseilakt-replay 4",
    "drahtseilakt-replay 5",
    "drahtseilakt-replay 6",
    "drahtseilakt-replay 7",
    "drahtseilakt-replay 8",
    "drahtseilakt-replay 9",
    "drahtseilakt-replay 10",
    FORMAT,
];

/// Where a run begins when it is not the level start (for testing a place): the sector (1 is the first), the ball
/// kind (the sector's usual one if `None`) and the place (the sector's reset point if `None`).
#[derive(Debug, Clone, PartialEq)]
pub struct Start {
    pub sector: usize,
    pub ball: Option<String>,
    pub at: Option<[f32; 3]>,
}

/// A mod a run was played with (a `mod` line): its key, its mod.txt's hash and its option values.
#[derive(Debug, Clone, PartialEq)]
pub struct ModRef {
    pub key: String,
    pub hash: u64,
    pub opts: Vec<(String, String)>,
}

/// The most `mod` lines a replay has, and options per line.
const MAX_MODS: usize = 64;
const MAX_MOD_OPTS: usize = 16;

/// The first format whose scripts run as they do now; a run of a map with scripts recorded in an older format
/// that ran scripts (9) would play differently, so it is refused as stale, never played silently otherwise.
pub const SCRIPTS_FROM: u32 = 10;

/// Why a replay cannot play on a level with scripts, if it cannot: recorded in a format whose scripts ran
/// otherwise. Formats before 9 ran no scripts and play as they were recorded.
pub fn stale_for_scripts(version: u32, has_scripts: bool) -> Option<String> {
    (has_scripts && (9..SCRIPTS_FROM).contains(&version)).then(|| {
        format!(
            "recorded in replay format {version}, whose map scripts ran otherwise (format {SCRIPTS_FROM}: an \
             activation applies at the next tick, as in the original); the run is stale, record it again"
        )
    })
}

impl ModRef {
    /// A `mod` line read: plain words only.
    pub fn parse(line: &str) -> Option<ModRef> {
        let mut f = line.split_whitespace();
        if f.next()? != "mod" {
            return None;
        }
        let word = |s: &str| {
            !s.is_empty()
                && s.len() <= 48
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        let key = f.next().filter(|k| word(k))?.to_owned();
        let hash = u64::from_str_radix(f.next()?, 16).ok()?;
        let mut opts = Vec::new();
        for o in f {
            let (k, v) = o.split_once('=')?;
            if !word(k) || !word(v) || opts.len() >= MAX_MOD_OPTS {
                return None;
            }
            opts.push((k.to_owned(), v.to_owned()));
        }
        Some(ModRef { key, hash, opts })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    /// The format it was recorded in (1 to 5).
    pub version: u32,
    pub level_name: String,
    pub level_hash: u64,
    pub start: Option<Start>,
    /// The mods that changed the run (none: a classic run).
    pub mods: Vec<String>,
    /// (first tick, input) in increasing tick order.
    pub changes: Vec<(u64, u8)>,
    /// (first tick, the view's turn in whole degrees) where it changes: a second input channel, only for a run whose
    /// mod hands its view's turn to the run (`view.turn`); empty otherwise.
    pub turns: Vec<(u64, u16)>,
    /// (first tick, keyboard and mouse) where they change: what a map's scripts see (format 11; empty before).
    pub frames: Vec<(u64, InputFrame)>,
    /// (tick, paused) where the pause menu opened (true) or closed (false) over the run: the tick whose scripts first
    /// hear the game's pause messages (format 11; empty before).
    pub pauses: Vec<(u64, bool)>,
    pub end: u64,
}

impl Replay {
    pub fn new(level_name: &str, level_hash: u64) -> Replay {
        Replay {
            version: VERSION,
            level_name: level_name.to_owned(),
            level_hash,
            start: None,
            mods: Vec::new(),
            changes: Vec::new(),
            turns: Vec::new(),
            frames: Vec::new(),
            pauses: Vec::new(),
            end: 0,
        }
    }

    /// The mods it names, read.
    pub fn mod_refs(&self) -> Vec<ModRef> {
        self.mods.iter().filter_map(|l| ModRef::parse(l)).collect()
    }

    /// Records the input of `tick` (ticks must come in order).
    pub fn record(&mut self, tick: u64, input: u8) {
        if self.changes.last().map(|c| c.1) != Some(input) {
            self.changes.push((tick, input));
        }
        self.end = tick;
    }

    /// Records the view's turn of `tick` (only where it changes).
    pub fn record_turn(&mut self, tick: u64, turn: u16) {
        if self.turns.last().map_or(0, |c| c.1) != turn {
            self.turns.push((tick, turn));
        }
    }

    /// Records the keyboard and the mouse of `tick` (only where they change; nothing before format 11).
    pub fn record_frame(&mut self, tick: u64, frame: InputFrame) {
        let frame = frame.clamped();
        if self.version >= 11 && self.frames.last().map_or(InputFrame::default(), |c| c.1) != frame
        {
            self.frames.push((tick, frame));
        }
    }

    /// Takes the run's pause events (nothing before format 11).
    pub fn record_pauses(&mut self, pauses: &[(u64, bool)]) {
        if self.version >= 11 {
            self.pauses = pauses.to_vec();
        }
    }

    /// The keyboard and the mouse at a tick (an empty keyboard and a still mouse before format 11).
    pub fn frame_at(&self, tick: u64) -> InputFrame {
        match self.frames.partition_point(|c| c.0 <= tick) {
            0 => InputFrame::default(),
            n => self
                .frames
                .get(n.saturating_sub(1))
                .map_or(InputFrame::default(), |c| c.1),
        }
    }

    /// The view's turn at a tick, degrees.
    pub fn turn_at(&self, tick: u64) -> u16 {
        match self.turns.partition_point(|c| c.0 <= tick) {
            0 => 0,
            n => self.turns.get(n.saturating_sub(1)).map_or(0, |c| c.1),
        }
    }

    /// The input at a tick.
    pub fn input_at(&self, tick: u64) -> u8 {
        match self.changes.partition_point(|c| c.0 <= tick) {
            0 => 0,
            n => self
                .changes
                .get(n.saturating_sub(1))
                .map(|c| c.1)
                .unwrap_or(0),
        }
    }

    pub fn to_text(&self) -> String {
        // the format the run was played in (a run going on from an older replay keeps that replay's rules)
        let head = usize::try_from(self.version.saturating_sub(1))
            .ok()
            .and_then(|i| FORMATS.get(i))
            .unwrap_or(&FORMAT);
        let mut s = format!(
            "{head}\nlevel {} {:016x}\n",
            self.level_name.replace(' ', "_"),
            self.level_hash
        );
        if let Some(st) = &self.start {
            let at = st
                .at
                .map_or("-".to_owned(), |[x, y, z]| format!("{x},{y},{z}"));
            let _ = writeln!(
                s,
                "start {} {} {at}",
                st.sector,
                st.ball.as_deref().unwrap_or("-")
            );
        }
        for m in &self.mods {
            let _ = writeln!(s, "{m}");
        }
        // the input changes, with the view's turns in their tick order among them
        let mut turns = self.turns.iter().peekable();
        for (t, i) in &self.changes {
            while let Some((tt, d)) = turns.next_if(|c| c.0 <= *t) {
                let _ = writeln!(s, "turn {tt} {d}");
            }
            let _ = writeln!(s, "{t} {i:02x}");
        }
        for (tt, d) in turns {
            let _ = writeln!(s, "turn {tt} {d}");
        }
        let mut before = InputFrame::default();
        for (t, f) in &self.frames {
            let keys: Vec<String> = (0..=255u8)
                .filter(|&k| f.down(k) != before.down(k))
                .map(|k| format!("{}{k:02x}", if f.down(k) { '+' } else { '-' }))
                .collect();
            if !keys.is_empty() {
                let _ = writeln!(s, "keys {t} {}", keys.join(" "));
            }
            if (f.mouse, f.buttons) != (before.mouse, before.buttons) {
                let _ = writeln!(s, "mouse {t} {} {} {}", f.mouse[0], f.mouse[1], f.buttons);
            }
            before = *f;
        }
        for (t, p) in &self.pauses {
            let _ = writeln!(s, "pause {t} {}", u8::from(*p));
        }
        let _ = writeln!(s, "end {}", self.end);
        s
    }

    pub fn parse(text: &str) -> Result<Replay, String> {
        let mut lines = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'));
        let head = lines.next().unwrap_or_default();
        let Some(version) = FORMATS.iter().position(|f| *f == head) else {
            return Err(format!("not a replay (expected \"{FORMAT}\")"));
        };
        let version = version as u32 + 1;
        let level = lines.next().ok_or("missing level line")?;
        let mut f = level.split_whitespace();
        if f.next() != Some("level") {
            return Err("missing level line".into());
        }
        let level_name = f.next().ok_or("missing level name")?.to_owned();
        let level_hash = u64::from_str_radix(f.next().ok_or("missing level hash")?, 16)
            .map_err(|e| e.to_string())?;
        let mut r = Replay {
            version,
            level_name,
            level_hash,
            start: None,
            mods: Vec::new(),
            changes: Vec::new(),
            turns: Vec::new(),
            frames: Vec::new(),
            pauses: Vec::new(),
            end: 0,
        };
        for line in lines {
            let mut f = line.split_whitespace();
            let a = f.next().unwrap_or_default();
            if a == "start" && r.start.is_none() && r.changes.is_empty() {
                r.start = Some(parse_start(f).ok_or_else(|| format!("bad start: {line}"))?);
                continue;
            }
            if a == "mod" && r.changes.is_empty() {
                if r.mods.len() >= MAX_MODS || ModRef::parse(line).is_none() {
                    return Err(format!("bad mod line: {line}"));
                }
                r.mods
                    .push(line.split_whitespace().collect::<Vec<_>>().join(" "));
                continue;
            }
            if a == "keys" || a == "mouse" {
                if version < 11 {
                    return Err(format!("{a} lines need replay format 11: {line}"));
                }
                parse_frame_line(&mut r.frames, a, f).ok_or_else(|| format!("bad {a}: {line}"))?;
                continue;
            }
            if a == "pause" {
                if version < 11 {
                    return Err(format!("pause lines need replay format 11: {line}"));
                }
                let tick: Option<u64> = f.next().and_then(|v| v.parse().ok());
                let on = match f.next() {
                    Some("1") => Some(true),
                    Some("0") => Some(false),
                    _ => None,
                };
                let (Some(tick), Some(on)) = (tick, on) else {
                    return Err(format!("bad pause: {line}"));
                };
                if f.next().is_some()
                    || r.pauses.len() >= MAX_PAUSES
                    || r.pauses.last().is_some_and(|p| p.0 > tick)
                {
                    return Err(format!("bad pause: {line}"));
                }
                r.pauses.push((tick, on));
                continue;
            }
            if a == "turn" {
                let tick: u64 = f
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| format!("bad turn: {line}"))?;
                let deg: u16 = f
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|d| *d < 360)
                    .ok_or_else(|| format!("bad turn: {line}"))?;
                if r.turns.last().is_some_and(|c| c.0 >= tick) {
                    return Err(format!("ticks out of order: {line}"));
                }
                r.turns.push((tick, deg));
                continue;
            }
            let b = f.next().ok_or_else(|| format!("bad line: {line}"))?;
            if a == "end" {
                r.end = b.parse().map_err(|_| format!("bad end: {line}"))?;
                break;
            }
            let tick: u64 = a.parse().map_err(|_| format!("bad tick: {line}"))?;
            let input = u8::from_str_radix(b, 16).map_err(|_| format!("bad input: {line}"))?;
            if r.changes.last().is_some_and(|c| c.0 >= tick) {
                return Err(format!("ticks out of order: {line}"));
            }
            r.changes.push((tick, input));
        }
        Ok(r)
    }
}

/// A `keys` or `mouse` line applied to the frames: the last frame (or an empty one) changed at that tick.
fn parse_frame_line<'a>(
    frames: &mut Vec<(u64, InputFrame)>,
    kind: &str,
    mut f: impl Iterator<Item = &'a str>,
) -> Option<()> {
    let tick: u64 = f.next()?.parse().ok()?;
    let last = frames.last().copied();
    if last.is_some_and(|c| c.0 > tick) {
        return None;
    }
    let mut cur = last.map_or(InputFrame::default(), |c| c.1);
    if kind == "keys" {
        let mut n = 0;
        for tok in f {
            let (on, hex) = match (tok.strip_prefix('+'), tok.strip_prefix('-')) {
                (Some(h), _) => (true, h),
                (_, Some(h)) => (false, h),
                _ => return None,
            };
            cur.set(u8::from_str_radix(hex, 16).ok()?, on);
            n += 1;
        }
        if n == 0 {
            return None;
        }
    } else {
        let x: i16 = f.next()?.parse().ok()?;
        let y: i16 = f.next()?.parse().ok()?;
        let b: u8 = f.next()?.parse().ok()?;
        if !(0..=MOUSE_W).contains(&x) || !(0..=MOUSE_H).contains(&y) || b > 7 || f.next().is_some()
        {
            return None;
        }
        cur.mouse = [x, y];
        cur.buttons = b;
    }
    match frames.last_mut() {
        Some(l) if l.0 == tick => l.1 = cur,
        _ => frames.push((tick, cur)),
    }
    Some(())
}

const MOUSE_W: i16 = script::input::MOUSE_FRAME.0;
const MOUSE_H: i16 = script::input::MOUSE_FRAME.1;

fn parse_start<'a>(mut f: impl Iterator<Item = &'a str>) -> Option<Start> {
    let sector = f.next()?.parse().ok().filter(|&n| n >= 1)?;
    let ball = match f.next()? {
        "-" => None,
        b => Some(b.to_owned()),
    };
    let at = match f.next()? {
        "-" => None,
        v => Some(parse_point(v)?),
    };
    Some(Start { sector, ball, at })
}

/// A point written `x,y,z`, all finite.
pub fn parse_point(v: &str) -> Option<[f32; 3]> {
    let p: Vec<f32> = v
        .split(',')
        .map(|c| c.trim().parse::<f32>().ok().filter(|c| c.is_finite()))
        .collect::<Option<_>>()?;
    p.try_into().ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_lookup() -> Result<(), String> {
        let mut r = Replay::new("Level 01", 0xabc);
        for t in 1..=10 {
            r.record(t, if (3..6).contains(&t) { 1 } else { 0 });
        }
        let back = Replay::parse(&r.to_text())?;
        assert_eq!(back.changes, vec![(1, 0), (3, 1), (6, 0)]);
        assert_eq!(back.end, 10);
        assert_eq!(
            (back.input_at(2), back.input_at(4), back.input_at(9)),
            (0, 1, 0)
        );
        Ok(())
    }

    #[test]
    fn the_views_turn_is_a_second_channel_and_a_run_without_it_is_as_before() -> Result<(), String>
    {
        let mut r = Replay::new("Level_01.NMO", 0xabc);
        for t in 1..=6 {
            r.record(t, 0);
            r.record_turn(t, if t >= 4 { 90 } else { 0 });
        }
        assert_eq!(r.turns, vec![(4, 90)]);
        let back = Replay::parse(&r.to_text())?;
        assert_eq!(back, r);
        assert_eq!(
            (back.turn_at(3), back.turn_at(4), back.turn_at(6)),
            (0, 90, 90)
        );
        let mut plain = Replay::new("Level_01.NMO", 0xabc);
        plain.record(1, 0);
        assert!(!plain.to_text().contains("turn"));
        assert!(Replay::parse("drahtseilakt-replay 7\nlevel a 1\nturn 1 400\nend 1\n").is_err());
        Ok(())
    }

    #[test]
    fn format_11_round_trips_and_older_formats_still_read() -> Result<(), String> {
        let mut r = Replay::new("Level_01.NMO", 0xabc);
        r.record(1, 0);
        assert_eq!(r.version, 11);
        assert!(r.to_text().starts_with("drahtseilakt-replay 11\n"));
        assert_eq!(Replay::parse(&r.to_text())?, r);
        let old = Replay::parse("drahtseilakt-replay 7\nlevel a 1\n1 00\nend 1")?;
        assert_eq!(old.version, 7);
        Ok(())
    }

    #[test]
    fn a_modded_run_names_its_mods_and_a_classic_one_none() -> Result<(), String> {
        let mut r = Replay::new("Level_01.NMO", 0xabc);
        r.record(1, 0);
        assert!(
            !r.to_text().contains("mod "),
            "a classic replay is as before"
        );
        r.mods
            .push("mod sticky-ball 00000000000000ff strength=60".into());
        let back = Replay::parse(&r.to_text())?;
        assert_eq!(back, r);
        assert_eq!(
            back.mod_refs()[0].opts,
            vec![("strength".to_owned(), "60".to_owned())]
        );
        for bad in ["mod ../x 1", "mod a zz", "mod a 1 k", "mod a 1 k=../v"] {
            assert!(
                Replay::parse(&format!(
                    "drahtseilakt-replay 5\nlevel a 1\n{bad}\n1 00\nend 1"
                ))
                .is_err(),
                "{bad}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_start_elsewhere_round_trips() -> Result<(), String> {
        let mut r = Replay::new("Level_02.NMO", 0xabc);
        r.start = Some(Start {
            sector: 3,
            ball: Some("stone".into()),
            at: Some([1.5, -2.0, 300.25]),
        });
        r.record(1, 0);
        assert_eq!(Replay::parse(&r.to_text())?, r);
        r.start = Some(Start {
            sector: 2,
            ball: None,
            at: None,
        });
        assert_eq!(Replay::parse(&r.to_text())?, r);
        assert!(Replay::parse("drahtseilakt-replay 1\nlevel a 1\nstart 0 - -\nend 1").is_err());
        let old = Replay::parse("drahtseilakt-replay 1\nlevel a 1\n1 01\nend 5")
            .expect("format 1 still reads");
        assert_eq!(old.version, 1);
        let two = Replay::parse("drahtseilakt-replay 2\nlevel a 1\n1 01\nend 5")
            .expect("format 2 still reads");
        assert_eq!(two.version, 2);
        let three = Replay::parse("drahtseilakt-replay 3\nlevel a 1\n1 01\nend 5")
            .expect("format 3 still reads");
        assert_eq!(three.version, 3);
        let four = Replay::parse("drahtseilakt-replay 4\nlevel a 1\n1 01\nend 5")
            .expect("format 4 still reads");
        assert_eq!(four.version, 4);
        let five = Replay::parse("drahtseilakt-replay 5\nlevel a 1\n1 01\nend 5")
            .expect("format 5 still reads");
        assert_eq!(five.version, 5);
        let six = Replay::parse("drahtseilakt-replay 6\nlevel a 1\n1 01\nend 5")
            .expect("format 6 still reads");
        assert_eq!(six.version, 6);
        let seven = Replay::parse("drahtseilakt-replay 7\nlevel a 1\n1 01\nend 5")
            .expect("format 7 still reads");
        assert_eq!(seven.version, 7);
        let eight = Replay::parse("drahtseilakt-replay 8\nlevel a 1\n1 01\nend 5")
            .expect("format 8 still reads");
        assert_eq!(eight.version, 8);
        let nine = Replay::parse("drahtseilakt-replay 9\nlevel a 1\n1 01\nend 5")
            .expect("format 9 still reads");
        assert_eq!(nine.version, 9);
        let ten = Replay::parse("drahtseilakt-replay 10\nlevel a 1\n1 01\nend 5")
            .expect("format 10 still reads");
        assert_eq!(ten.version, 10);
        let new = Replay::parse(&Replay::new("a", 1).to_text()).expect("format 11");
        assert_eq!(new.version, 11);
        assert!(
            Replay::parse("drahtseilakt-replay 12\nlevel a 1\nend 1").is_err(),
            "a later format is refused"
        );
        // format 9's scripts ran otherwise: stale on a map with scripts, fine without; older ones ran none
        assert!(stale_for_scripts(9, true).is_some());
        assert!(stale_for_scripts(9, false).is_none());
        assert!(stale_for_scripts(8, true).is_none());
        assert!(stale_for_scripts(10, true).is_none());
        assert!(stale_for_scripts(11, true).is_none());
        let mut kept = Replay::new("a", 1);
        kept.version = 2;
        assert_eq!(
            Replay::parse(&kept.to_text())?.version,
            2,
            "an older format is written as it is"
        );
        assert!(Replay::parse("drahtseilakt-replay 1\nlevel a 1\nstart 1 - 1,2\nend 1").is_err());
        Ok(())
    }

    #[test]
    fn the_keyboard_and_the_mouse_round_trip_as_changes() -> Result<(), String> {
        let mut r = Replay::new("Level_01.NMO", 0xabc);
        let mut f = InputFrame::default();
        for t in 1..=8u64 {
            f.set(0x1c, (3..6).contains(&t));
            f.set(0xc8, t >= 5);
            f.mouse = [if t >= 7 { 320 } else { 0 }, if t >= 7 { 240 } else { 0 }];
            f.buttons = if t == 8 { script::input::LEFT } else { 0 };
            r.record(t, 0);
            r.record_frame(t, f);
        }
        // only the ticks where something changed are stored
        assert_eq!(
            r.frames.iter().map(|c| c.0).collect::<Vec<_>>(),
            vec![3, 5, 6, 7, 8]
        );
        let text = r.to_text();
        assert!(text.contains("keys 3 +1c\n"), "{text}");
        assert!(text.contains("keys 5 +c8\n") || text.contains("keys 5 -1c +c8"));
        assert!(text.contains("mouse 7 320 240 0\n"), "{text}");
        let back = Replay::parse(&text)?;
        assert_eq!(back, r);
        assert!(!back.frame_at(2).down(0x1c));
        assert!(back.frame_at(4).down(0x1c));
        assert!(!back.frame_at(6).down(0x1c) && back.frame_at(6).down(0xc8));
        assert_eq!(back.frame_at(8).mouse, [320, 240]);
        assert_eq!(back.frame_at(8).buttons, 1);
        // a run without input is as before: no input lines
        let mut plain = Replay::new("Level_01.NMO", 0xabc);
        plain.record(1, 0);
        plain.record_frame(1, InputFrame::default());
        assert!(!plain.to_text().contains("keys") && !plain.to_text().contains("mouse"));
        // an older format records and reads no input lines
        let mut old = Replay::new("Level_01.NMO", 0xabc);
        old.version = 10;
        old.record_frame(1, f);
        assert!(old.frames.is_empty());
        assert!(Replay::parse("drahtseilakt-replay 10\nlevel a 1\nkeys 1 +1c\nend 1").is_err());
        // hostile lines
        for bad in [
            "keys 1",
            "keys 1 1c",
            "keys 1 +zz",
            "mouse 1 700 0 0",
            "mouse 1 0 0 9",
            "mouse 1 0 0",
            "keys 2 +1c\nkeys 1 -1c",
            "pause 1",
            "pause 1 2",
            "pause x 1",
            "pause 1 1 1",
            "pause 2 1\npause 1 0",
        ] {
            assert!(
                Replay::parse(&format!("drahtseilakt-replay 11\nlevel a 1\n{bad}\nend 2")).is_err(),
                "{bad}"
            );
        }
        Ok(())
    }

    #[test]
    fn pauses_round_trip() -> Result<(), String> {
        let mut r = Replay::new("Level_01.NMO", 0xabc);
        r.record_pauses(&[(5, true), (9, false)]);
        let text = r.to_text();
        assert!(text.contains("pause 5 1\npause 9 0\n"), "{text}");
        assert_eq!(Replay::parse(&text)?, r);
        let mut old = Replay::new("a", 1);
        old.version = 10;
        old.record_pauses(&[(5, true)]);
        assert!(old.pauses.is_empty());
        assert!(Replay::parse("drahtseilakt-replay 10\nlevel a 1\npause 1 1\nend 1").is_err());
        Ok(())
    }
}
