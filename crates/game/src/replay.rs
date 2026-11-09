// Replays: the input of every tick, stored as the ticks where it changes, in a versioned text format. The level
// is identified by a hash of its file, so a replay never runs against the wrong data. A replay holds only our
// input, never game data, so it can live in the repository.
//
//   drahtseilakt-replay 8        (1: recorded before the ball burst into pieces in transformers, 2: before the
//                                 pieces went after 20 s, 3: before the checkpoints followed their names, 4: before
//                                 the loose objects were made of their module file's mesh, 5: before the camera
//                                 looked along the reset point after a lost ball, 6: before a run begun at a sector took the
//                                 sector's own ball, 7: before a map's own scripts ran; each played so)
//   level <file name> <fnv-1a 64 of the file, hex>
//   start <sector> <ball kind or -> <x,y,z or ->   (optional: a run begun elsewhere than the level start)
//   mod <key> <fnv-1a 64 of its mod.txt, hex> [<option>=<value>]...   (optional, at most MAX_MODS: a mod that
//                                 changes the run, so the run is modded and plays back only with that very mod;
//                                 docs/drahtseilakt/mods.md. A replay without them is a classic run, as before)
//   <tick> <input bits, hex>      (from this tick on; tick 1 is the first)
//   end <last tick>

use std::fmt::Write as _;

pub const FORMAT: &str = "drahtseilakt-replay 8";
/// The format written (the number in `FORMAT`).
pub const VERSION: u32 = 8;
/// The formats read: 1 (no bursts), 2 (the pieces lie until the ball is lost), 3 (the checkpoints in their group's
/// order), 4 (the loose objects of their stand-ins' meshes), 5 (the respawn at the start heading), 6, 7 (a run begun
/// at a sector keeps the level's start ball) and 8 (a map's own scripts run; before, they did not).
const FORMATS: [&str; 8] = [
    "drahtseilakt-replay 1",
    "drahtseilakt-replay 2",
    "drahtseilakt-replay 3",
    "drahtseilakt-replay 4",
    "drahtseilakt-replay 5",
    "drahtseilakt-replay 6",
    "drahtseilakt-replay 7",
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
        for (t, i) in &self.changes {
            let _ = writeln!(s, "{t} {i:02x}");
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
        let new = Replay::parse(&Replay::new("a", 1).to_text()).expect("format 8");
        assert_eq!(new.version, 8);
        assert!(
            Replay::parse("drahtseilakt-replay 9\nlevel a 1\nend 1").is_err(),
            "a later format is refused"
        );
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
}
