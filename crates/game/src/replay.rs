// Replays: the input of every tick, stored as the ticks where it changes, in a versioned text format. The level
// is identified by a hash of its file, so a replay never runs against the wrong data. A replay holds only our
// input, never game data, so it can live in the repository.
//
//   drahtseilakt-replay 1
//   level <file name> <fnv-1a 64 of the file, hex>
//   <tick> <input bits, hex>      (from this tick on; tick 1 is the first)
//   end <last tick>

use std::fmt::Write as _;

pub const FORMAT: &str = "drahtseilakt-replay 1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    pub level_name: String,
    pub level_hash: u64,
    /// (first tick, input) in increasing tick order.
    pub changes: Vec<(u64, u8)>,
    pub end: u64,
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

impl Replay {
    pub fn new(level_name: &str, level_hash: u64) -> Replay {
        Replay {
            level_name: level_name.to_owned(),
            level_hash,
            changes: Vec::new(),
            end: 0,
        }
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
        let mut s = format!(
            "{FORMAT}\nlevel {} {:016x}\n",
            self.level_name.replace(' ', "_"),
            self.level_hash
        );
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
        if lines.next() != Some(FORMAT) {
            return Err(format!("not a replay (expected \"{FORMAT}\")"));
        }
        let level = lines.next().ok_or("missing level line")?;
        let mut f = level.split_whitespace();
        if f.next() != Some("level") {
            return Err("missing level line".into());
        }
        let level_name = f.next().ok_or("missing level name")?.to_owned();
        let level_hash = u64::from_str_radix(f.next().ok_or("missing level hash")?, 16)
            .map_err(|e| e.to_string())?;
        let mut r = Replay {
            level_name,
            level_hash,
            changes: Vec::new(),
            end: 0,
        };
        for line in lines {
            let mut f = line.split_whitespace();
            let a = f.next().unwrap_or_default();
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
}
