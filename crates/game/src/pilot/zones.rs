// The watcher's zones for the test player (docs/drahtseilakt/autopilot.md, "Zones"): stretches of a level the owner
// marked while watching it play (--watch, F8 / F4), kept beside the level's hints as `<level>.zones` and read on
// every run, headless and in the window. A zone a line:
//
//   flow sector N r R : X Y Z ; X Y Z ; ...
//   careful sector N r R [speed V] : X Y Z ; X Y Z ; ...
//   calm sector N r R : X Y Z ; X Y Z ; ...
//   plain sector N r R : X Y Z ; X Y Z ; ...
//
// The points are the ball's path through the stretch as it was marked (one point: a place); the zone is everything
// within R of that path, in that sector. `flow`: don't dawdle here (no stops, waits or slowing down that are not
// needed to keep the ball). `careful`: keep under the speed (CAREFUL_SPEED when not given) and on the middle of the
// track, no corner cutting. `calm`: in the speedrun style (autopilot.rs, `Style`), ride the calm style here (a
// puzzle the steady pace gets through and the speedrun's does not); no effect in the calm style. `plain`: no learned
// rule here (learned.rs): the test player rides as it does by itself, where a rule learned on another level would
// lose the ball. `#` begins a comment.
//
// The file is untrusted (it may be shared with the hints): its size, the number of zones and points and every number
// are capped; a malformed line is left out with a note naming it (the others still count), a file too big, a link or
// not UTF-8 gives no zones at all.

use glam::Vec3;
use std::path::Path;

const MAX_FILE: u64 = 64 * 1024;
const MAX_ZONES: usize = 256;
pub const MAX_POINTS: usize = 256;
const MAX_COORD: f32 = 100_000.0;
const MAX_RADIUS: f32 = 50.0;
/// The least radius and speed a line may give (a smaller one would be no zone, or a ball held still).
const MIN_SIZE: f32 = 0.1;
const MAX_SPEED: f32 = 40.0;
const MAX_SECTOR: usize = 99;
const MAX_NOTES: usize = 20;
/// A careful zone's speed limit when its line gives none (units a second: a moderate rail pace is 6).
pub const CAREFUL_SPEED: f32 = 4.5;
/// How far round the marked path a zone reaches when the watcher marks it.
pub const MARK_RADIUS: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Flow,
    Careful,
    Calm,
    Plain,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Flow => "flow",
            Kind::Careful => "careful",
            Kind::Calm => "calm",
            Kind::Plain => "plain",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub kind: Kind,
    /// As the game shows it (1 is the first).
    pub sector: usize,
    pub radius: f32,
    pub speed: Option<f32>,
    pub points: Vec<Vec3>,
}

/// What the zones the ball is in ask of the test player.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Effect {
    /// Don't dawdle: no stop, wait or slowing down that is not needed.
    pub flow: bool,
    /// Careful: not faster than this, on the middle of the track.
    pub cap: Option<f32>,
    /// A speedrun rides the calm style here.
    pub calm: bool,
    /// No learned rule holds here.
    pub plain: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Zones {
    pub zones: Vec<Zone>,
}

/// The distance from `p` to the segment `a`–`b`.
fn to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 1e-9 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

impl Zone {
    /// How far `p` is from the zone's path (its point, for one).
    pub fn distance(&self, p: Vec3) -> f32 {
        match self.points.as_slice() {
            [] => f32::INFINITY,
            [a] => p.distance(*a),
            pts => pts
                .windows(2)
                .filter_map(|w| Some(to_segment(p, *w.first()?, *w.get(1)?)))
                .fold(f32::INFINITY, f32::min),
        }
    }

    /// The speed limit of a careful zone.
    pub fn cap(&self) -> Option<f32> {
        (self.kind == Kind::Careful).then(|| self.speed.unwrap_or(CAREFUL_SPEED))
    }

    /// The zone as a line of the file.
    pub fn line(&self) -> String {
        let mut s = format!(
            "{} sector {} r {}",
            self.kind.word(),
            self.sector,
            self.radius
        );
        if let Some(v) = self.speed {
            s += &format!(" speed {v}");
        }
        s += " :";
        for (i, p) in self.points.iter().enumerate() {
            s += &format!(
                "{} {:.2} {:.2} {:.2}",
                if i == 0 { " " } else { " ; " },
                p.x,
                p.y,
                p.z
            );
        }
        s
    }
}

impl Zones {
    /// What the zones at `p` in `sector` (from 0, as the sim counts) ask: flow when any flow zone holds it, the
    /// lowest speed limit of the careful ones.
    pub fn at(&self, p: Vec3, sector: usize) -> Effect {
        let mut e = Effect::default();
        for z in self
            .zones
            .iter()
            .filter(|z| z.sector == sector.saturating_add(1))
        {
            if z.distance(p) > z.radius {
                continue;
            }
            match (z.kind, z.cap()) {
                (Kind::Calm, _) => e.calm = true,
                (Kind::Plain, _) => e.plain = true,
                (_, Some(c)) => e.cap = Some(e.cap.map_or(c, |o: f32| o.min(c))),
                (_, None) => e.flow = true,
            }
        }
        e
    }

    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }
}

fn num(s: Option<&str>, what: &str) -> Result<f32, String> {
    let s = s.ok_or_else(|| format!("{what} missing"))?;
    if s.len() > 32 {
        return Err(format!("{what}: number too long"));
    }
    let v: f32 = s
        .parse()
        .map_err(|_| format!("{what}: not a number: {s:.32}"))?;
    if !v.is_finite() || v.abs() > MAX_COORD {
        return Err(format!("{what}: out of range"));
    }
    Ok(v)
}

/// One zone line (comment already cut off).
fn zone(line: &str) -> Result<Zone, String> {
    let (head, tail) = line.split_once(':').ok_or("no ':' before the points")?;
    let mut w = head.split_whitespace();
    let kind = match w.next() {
        Some("flow") => Kind::Flow,
        Some("careful") => Kind::Careful,
        Some("calm") => Kind::Calm,
        Some("plain") => Kind::Plain,
        Some(k) => return Err(format!("unknown kind {k:.16}")),
        None => return Err("no kind".into()),
    };
    let (mut sector, mut radius, mut speed) = (None, None, None);
    while let Some(word) = w.next() {
        match word {
            "sector" if sector.is_none() => {
                let s = w.next().ok_or("sector missing")?;
                let n: usize = s.parse().map_err(|_| "sector: not a number")?;
                if n == 0 || n > MAX_SECTOR {
                    return Err("sector out of range".into());
                }
                sector = Some(n);
            }
            "r" if radius.is_none() => {
                let r = num(w.next(), "r")?;
                if r < MIN_SIZE || r > MAX_RADIUS {
                    return Err("r out of range".into());
                }
                radius = Some(r);
            }
            "speed" if speed.is_none() && kind == Kind::Careful => {
                let v = num(w.next(), "speed")?;
                if v < MIN_SIZE || v > MAX_SPEED {
                    return Err("speed out of range".into());
                }
                speed = Some(v);
            }
            other => return Err(format!("unexpected {other:.16}")),
        }
    }
    let sector = sector.ok_or("no sector")?;
    let radius = radius.ok_or("no r")?;
    let mut points = Vec::new();
    for p in tail.split(';') {
        let mut c = p.split_whitespace();
        let v = Vec3::new(
            num(c.next(), "x")?,
            num(c.next(), "y")?,
            num(c.next(), "z")?,
        );
        if c.next().is_some() {
            return Err("a point has more than three numbers".into());
        }
        if points.len() >= MAX_POINTS {
            return Err(format!("more than {MAX_POINTS} points"));
        }
        points.push(v);
    }
    Ok(Zone {
        kind,
        sector,
        radius,
        speed,
        points,
    })
}

/// The zones of a file's text, and a note for each line left out (malformed, or past the cap).
pub fn parse(text: &str) -> (Zones, Vec<String>) {
    // (a leading byte order mark, as some editors write, is no part of the first line)
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut zones = Vec::new();
    let mut notes = Vec::new();
    // (the notes quote the file: shown with its control and format characters escaped)
    let mut note = |n: String| {
        if notes.len() < MAX_NOTES {
            notes.push(gamedata::package::printable(&n));
        }
    };
    for (i, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let number = i.saturating_add(1);
        if zones.len() >= MAX_ZONES {
            note(format!(
                "line {number}: more than {MAX_ZONES} zones, the rest left out"
            ));
            break;
        }
        match zone(line) {
            Ok(z) => zones.push(z),
            Err(e) => note(format!("line {number}: {e}; left out")),
        }
    }
    (Zones { zones }, notes)
}

/// The zones file at `path`: none there is no zones; one too big, a link or not UTF-8 is no zones, with a note.
pub fn load(path: &Path) -> (Zones, Vec<String>) {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (Zones::default(), Vec::new());
        }
        _ => {}
    }
    let text = crate::files::read_plain(path, MAX_FILE)
        .and_then(|b| String::from_utf8(b).map_err(|_| format!("{}: not UTF-8", path.display())));
    match text {
        Ok(t) => {
            let (z, notes) = parse(&t);
            (
                z,
                notes
                    .into_iter()
                    .map(|n| format!("{}: {n}", path.display()))
                    .collect(),
            )
        }
        Err(e) => (Zones::default(), vec![format!("{e}; no zones read")]),
    }
}

/// A new mark's id: written after the line it adds (`# mark=<id>`), so that undoing it takes out that line and no
/// other (a file may hold another session's marks after it).
pub fn new_id() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in t
        .to_le_bytes()
        .into_iter()
        .chain(std::process::id().to_le_bytes())
    {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:08x}", h as u32)
}

/// The tag of a mark's id, as written in a line's comment.
fn tag(id: &str) -> String {
    format!("mark={id}")
}

/// A zone marked now (with the mark's `id`), added at the end of the file (made with a heading when new). A file
/// already holding MAX_ZONES zones is full: the zone is not added, and the error says so.
pub fn append(path: &Path, z: &Zone, id: &str) -> Result<(), String> {
    let mut text = crate::files::read_text(path, MAX_FILE)?;
    if parse(&text).0.zones.len() >= MAX_ZONES {
        return Err(format!(
            "{}: full ({MAX_ZONES} zones), delete some",
            path.display()
        ));
    }
    if text.is_empty() {
        text = "# Zones the watcher marked for the test player (docs/drahtseilakt/autopilot.md, \"Zones\"):\n\
                # flow = don't dawdle here; careful = keep under the speed, on the middle of the track.\n"
            .to_owned();
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text += &format!("{}   # {}\n", z.line(), tag(id));
    if u64::try_from(text.len()).map_or(true, |n| n > MAX_FILE) {
        return Err(format!("{}: full ({MAX_FILE} bytes)", path.display()));
    }
    crate::files::write_plain(path, &text)
}

/// The line of the mark `id` taken out of the file at `path` (a zones file or the learned table; the other lines
/// stay as they are): the line, or None when no line has that id.
pub fn remove_mark(path: &Path, id: &str) -> Result<Option<String>, String> {
    let text = crate::files::read_text(path, MAX_FILE)?;
    let t = tag(id);
    let has = |l: &str| {
        l.split_once('#').is_some_and(|(_, c)| {
            c.split_whitespace()
                .any(|w| w.trim_end_matches([':', ',']) == t)
        })
    };
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = lines.iter().rposition(|l| has(l)) else {
        return Ok(None);
    };
    let gone = lines.get(k).map(|l| (*l).to_owned());
    let out: String = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != k)
        .map(|(_, l)| format!("{l}\n"))
        .collect();
    crate::files::write_plain(path, &out)?;
    Ok(gone)
}

/// A path the ball went, thinned to a point every `step` units (the first and last kept), at most MAX_POINTS.
pub fn thin(path: &[Vec3], step: f32) -> Vec<Vec3> {
    let mut out: Vec<Vec3> = Vec::new();
    for &p in path {
        if out.last().is_none_or(|l| l.distance(p) >= step) {
            out.push(p);
        }
    }
    if let (Some(&last), Some(&end)) = (out.last(), path.last())
        && last != end
    {
        out.push(end);
    }
    if out.len() > MAX_POINTS {
        // every n-th, the end kept
        let n = out.len().div_ceil(MAX_POINTS - 1);
        let end = out.last().copied();
        out = out.iter().step_by(n).copied().collect();
        out.extend(end);
        out.truncate(MAX_POINTS);
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn zone_lines_read_and_write_back() {
        let (z, notes) = parse(
            "# marked\nflow sector 2 r 4 : 1 2 3 ; 4 5 6\ncareful sector 1 r 3.5 speed 3 : 7 8 9 # a bend\n\
             careful sector 3 r 4 : 0 0 0\n",
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(z.zones.len(), 3);
        assert_eq!(z.zones[0].kind, Kind::Flow);
        assert_eq!(
            z.zones[0].points,
            vec![Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)]
        );
        assert_eq!(z.zones[1].cap(), Some(3.0));
        assert_eq!(z.zones[2].cap(), Some(CAREFUL_SPEED));
        for zone in &z.zones {
            let (again, n) = parse(&zone.line());
            assert!(n.is_empty());
            assert_eq!(&again.zones[0], zone);
        }
    }

    #[test]
    fn a_malformed_line_is_left_out_with_a_note_and_the_rest_kept() {
        for bad in [
            "flow sector 1 r 4",
            "flow sector 1 r 4 :",
            "flow r 4 : 1 2 3",
            "flow sector 1 : 1 2 3",
            "flow sector 0 r 4 : 1 2 3",
            "flow sector 100 r 4 : 1 2 3",
            "flow sector 1 r 0 : 1 2 3",
            "flow sector 1 r 1e-40 : 1 2 3",
            "careful sector 1 r 4 speed 0.01 : 1 2 3",
            "flow sector 1 r 51 : 1 2 3",
            "flow sector 1 r nan : 1 2 3",
            "flow sector 1 r 4 : 1 2",
            "flow sector 1 r 4 : 1 2 3 4",
            "flow sector 1 r 4 : 1e9 2 3",
            "flow sector 1 r 4 : inf 2 3",
            "flow sector 1 r 4 speed 3 : 1 2 3",
            "careful sector 1 r 4 speed 0 : 1 2 3",
            "careful sector 1 r 4 speed 99 : 1 2 3",
            "careful sector 1 sector 2 r 4 : 1 2 3",
            "fast sector 1 r 4 : 1 2 3",
            "flow sector -1 r 4 : 1 2 3",
            "flow sector 1 r 4 : 1 2 3 ;",
        ] {
            let (z, notes) = parse(&format!(
                "\nflow sector 1 r 4 : 0 0 0\n{bad}\ncareful sector 1 r 4 : 0 0 0\n"
            ));
            assert_eq!(z.zones.len(), 2, "{bad}");
            assert_eq!(notes.len(), 1, "{bad}");
            assert!(notes[0].starts_with("line 3:"), "{bad}: {}", notes[0]);
        }
        let many = "flow sector 1 r 1 : 0 0 0\n".repeat(MAX_ZONES + 5);
        let (z, notes) = parse(&many);
        assert_eq!(z.zones.len(), MAX_ZONES);
        assert_eq!(notes.len(), 1);
        let long = format!(
            "flow sector 1 r 1 : {}",
            vec!["1 2 3"; MAX_POINTS + 1].join(" ; ")
        );
        assert!(parse(&long).0.is_empty());
        // notes are capped too
        let bad = "x\n".repeat(1000);
        assert_eq!(parse(&bad).1.len(), MAX_NOTES);
    }

    #[test]
    fn a_zone_holds_what_is_near_its_path_in_its_sector() {
        let (z, _) = parse(
            "flow sector 1 r 4 : 0 0 0 ; 20 0 0 ; 20 0 20\ncareful sector 1 r 3 speed 5 : 10 0 0\n\
             careful sector 1 r 3 speed 3 : 12 0 0\ncareful sector 2 r 50 speed 1 : 0 0 0\n",
        );
        // along the path, between its points, and round the bend
        assert!(z.at(Vec3::new(5.0, 0.0, 3.0), 0).flow);
        assert!(z.at(Vec3::new(21.0, 0.0, 10.0), 0).flow);
        // not across the bend's open side, not above, not in another sector
        assert!(!z.at(Vec3::new(10.0, 0.0, 10.0), 0).flow);
        assert!(!z.at(Vec3::new(5.0, 6.0, 0.0), 0).flow);
        assert_eq!(z.at(Vec3::new(30.0, 0.0, 30.0), 0), Effect::default());
        // the lowest limit of the careful zones holding the place
        assert_eq!(z.at(Vec3::new(8.0, 0.0, 0.0), 0).cap, Some(5.0));
        assert_eq!(z.at(Vec3::new(11.0, 0.0, 0.0), 0).cap, Some(3.0));
        assert_eq!(z.at(Vec3::new(11.0, 0.0, 0.0), 1).cap, Some(1.0));
        assert!(!z.at(Vec3::new(11.0, 0.0, 0.0), 1).flow);
    }

    #[test]
    fn a_plain_zone_asks_only_that_no_learned_rule_holds() {
        let (z, notes) = parse("plain sector 2 r 5 : 0 0 0 ; 10 0 0\n");
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(z.zones[0].kind, Kind::Plain);
        assert_eq!(z.zones[0].cap(), None);
        let e = z.at(Vec3::new(5.0, 0.0, 2.0), 1);
        assert!(e.plain && !e.flow && !e.calm && e.cap.is_none());
        assert_eq!(z.at(Vec3::new(5.0, 0.0, 2.0), 0), Effect::default());
        let (again, n) = parse(&z.zones[0].line());
        assert!(n.is_empty());
        assert_eq!(again.zones[0], z.zones[0]);
        // a speed is only a careful zone's
        assert!(parse("plain sector 1 r 4 speed 3 : 1 2 3").0.is_empty());
    }

    #[test]
    fn marks_are_appended_and_undone_by_their_id() {
        let dir = std::env::temp_dir().join(format!("drahtseilakt-zones-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("Level_01.zones");
        let a = Zone {
            kind: Kind::Flow,
            sector: 1,
            radius: 4.0,
            speed: None,
            points: vec![Vec3::ZERO, Vec3::X * 5.0],
        };
        let b = Zone {
            kind: Kind::Careful,
            sector: 2,
            radius: 4.0,
            speed: Some(3.5),
            points: vec![Vec3::Z],
        };
        append(&path, &a, "aaaa0001").unwrap();
        append(&path, &b, "bbbb0002").unwrap();
        // another session's mark after ours, by hand without an id
        let mut text = std::fs::read_to_string(&path).unwrap();
        text += "flow sector 3 r 2 : 1 1 1\n";
        std::fs::write(&path, text).unwrap();
        let (z, notes) = load(&path);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(z.zones.len(), 3);
        assert_eq!(z.zones[..2], [a.clone(), b]);
        // ours goes, the later one stays
        assert!(
            remove_mark(&path, "bbbb0002")
                .unwrap()
                .is_some_and(|l| l.starts_with("careful"))
        );
        let left = load(&path).0.zones;
        assert_eq!(
            (left.len(), left[0].kind, left[1].sector),
            (2, Kind::Flow, 3)
        );
        assert!(remove_mark(&path, "bbbb0002").unwrap().is_none());
        assert!(remove_mark(&path, "aaaa0001").unwrap().is_some());
        assert_eq!(load(&path).0.zones.len(), 1);
        // a full file takes no more
        let full: String = "flow sector 1 r 1 : 0 0 0\n".repeat(MAX_ZONES);
        std::fs::write(&path, full).unwrap();
        assert!(append(&path, &a, "cccc0003").unwrap_err().contains("full"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn a_link_is_never_written_through() {
        let dir = std::env::temp_dir().join(format!("drahtseilakt-zlink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("real")).unwrap();
        let target = dir.join("real/target.txt");
        std::fs::write(&target, "keep\n").unwrap();
        let a = Zone {
            kind: Kind::Flow,
            sector: 1,
            radius: 4.0,
            speed: None,
            points: vec![Vec3::ZERO],
        };
        // the file a link
        std::os::unix::fs::symlink(&target, dir.join("Level_01.zones")).unwrap();
        assert!(append(&dir.join("Level_01.zones"), &a, "x").is_err());
        // a folder on the way a link
        std::os::unix::fs::symlink(dir.join("real"), dir.join("hints")).unwrap();
        assert!(append(&dir.join("hints/target.txt"), &a, "x").is_err());
        assert!(append(&dir.join("hints/new.zones"), &a, "x").is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "keep\n");
        assert!(!dir.join("real/new.zones").exists());
        // a byte order mark before the first line is no part of it
        assert_eq!(
            parse("\u{feff}flow sector 1 r 4 : 0 0 0\n").0.zones.len(),
            1
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_path_is_thinned_and_capped() {
        let path: Vec<Vec3> = (0..2000).map(|i| Vec3::X * (i as f32 * 0.1)).collect();
        let t = thin(&path, 1.5);
        assert!(t.len() <= MAX_POINTS);
        assert_eq!(t.first(), path.first());
        assert_eq!(t.last(), path.last());
        assert_eq!(thin(&[Vec3::ONE], 1.5), vec![Vec3::ONE]);
        assert!(thin(&[], 1.5).is_empty());
    }
}
