// What the test player has learned from the watcher's marks (docs/drahtseilakt/autopilot-rules.md, "The learned
// table"): each zone the owner marks (zones.rs) also says what kind of place it was, so that the same kind of place
// on any other level, one not yet passed too, is driven the same way. A place is described by a few features the
// test player reads from the level and its way (autopilot.rs, `Pilot::situation`): what the ball is on, how wide the
// floor is, what borders it, the bend and slope of the way, what comes just ahead, the module near, the ball. A
// rule is a plain conjunction of feature values, read and written by people:
//
//   flow when surface=rail ahead=rail-end kind=wood   # Level_01 sector 2 (147.2 -2.1 -552.0) 2026-11-03: ...
//   careful speed 4.5 when surface=floor width=narrow curve=sharp
//
// The rule holds where every named feature has that value; the features not named do not matter. A rule has to
// name something special about the place (`distinctive`), so none holds almost everywhere. The level's own zones
// come first: where the ball is in one, the learned rules are not looked at.
//
// The file (`learned.txt` beside the hints) is shared between levels and untrusted like a hint file: its size, the
// number of rules and conditions and every number are capped, the values are from fixed lists (a module's name:
// letters, digits and `_`), and a malformed line is left out with a note naming it.

use crate::zones::{Effect, Kind};
use std::path::Path;

const MAX_FILE: u64 = 64 * 1024;
const MAX_RULES: usize = 512;
const MAX_NOTES: usize = 20;
const MAX_SPEED: f32 = 40.0;
const MIN_SPEED: f32 = 0.1;
const MAX_MODULE: usize = 32;

/// The features of a place, in the order a rule names them, and the values each may have (`module`: a module's
/// name or `none`).
pub const FEATURES: [&str; 8] = [
    "surface", "width", "border", "curve", "slope", "ahead", "module", "kind",
];
const VALUES: [&[&str]; 8] = [
    &["floor", "rail", "off"],
    &["narrow", "medium", "wide"],
    &["walls", "open", "far"],
    &["straight", "bend", "sharp"],
    &["flat", "up", "down"],
    &[
        "none",
        "gap",
        "drop",
        "rail-end",
        "rail-start",
        "sill",
        "transformer",
    ],
    &[],
    &["wood", "stone", "paper", "sticky"],
];
const MODULE: usize = 6;

/// A place as the test player reads it: a value for each feature (None where it does not apply: the width of a rail).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Situation {
    pub values: [Option<String>; 8],
}

impl Situation {
    /// The place in words: `surface=rail ahead=rail-end ...`.
    pub fn describe(&self) -> String {
        let mut out = Vec::new();
        for (f, v) in FEATURES.iter().zip(&self.values) {
            if let Some(v) = v {
                out.push(format!("{f}={v}"));
            }
        }
        out.join(" ")
    }
}

/// Whether `value` of feature `f` says something special about a place (not what most of a level is like: a plain
/// floor, straight, flat, nothing ahead).
fn distinctive(f: usize, value: &str) -> bool {
    match FEATURES.get(f).copied() {
        Some("surface") => value != "floor",
        Some("width") => value == "narrow",
        Some("curve") => value != "straight",
        Some("slope") => value != "flat",
        Some("ahead") => value != "none",
        Some("module") => value != "none",
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub kind: Kind,
    pub speed: Option<f32>,
    /// The conditions: a feature (its index in FEATURES) and its value.
    pub when: Vec<(usize, String)>,
    /// Where it was learned, for people (the line's comment).
    pub evidence: String,
}

impl Rule {
    pub fn holds(&self, s: &Situation) -> bool {
        self.when
            .iter()
            .all(|(f, v)| s.values.get(*f).and_then(|x| x.as_deref()) == Some(v.as_str()))
    }

    pub fn cap(&self) -> Option<f32> {
        (self.kind == Kind::Careful).then(|| self.speed.unwrap_or(crate::zones::CAREFUL_SPEED))
    }

    /// The rule as the watcher reads it: `rail, rail-end ahead → don't dawdle`.
    pub fn label(&self) -> String {
        let when: Vec<String> = self
            .when
            .iter()
            .map(|(f, v)| match FEATURES.get(*f).copied() {
                Some("ahead") => format!("{v} ahead"),
                Some("surface" | "curve" | "slope" | "kind") => v.clone(),
                Some(name) => format!("{name} {v}"),
                None => v.clone(),
            })
            .collect();
        let what = match self.cap() {
            Some(c) => format!("careful, at most {c}"),
            None => "don't dawdle".to_owned(),
        };
        format!("{} → {what}", when.join(", "))
    }

    /// The rule as a line of the file.
    pub fn line(&self) -> String {
        let mut s = self.kind.word().to_owned();
        if let Some(v) = self.speed {
            s += &format!(" speed {v}");
        }
        s += " when";
        for (f, v) in &self.when {
            s += &format!(" {}={v}", FEATURES.get(*f).copied().unwrap_or("?"));
        }
        if !self.evidence.is_empty() {
            let e: String = self
                .evidence
                .chars()
                .map(|c| if c == '#' || c.is_control() { ' ' } else { c })
                .collect();
            s += &format!("   # {}", e.trim());
        }
        s
    }
}

#[derive(Debug, Clone, Default)]
pub struct Learned {
    pub rules: Vec<Rule>,
}

impl Learned {
    /// What the learned rules ask at the place `s`: flow when a flow rule holds, the lowest limit of the careful ones,
    /// and the label of the rule that names the most (the one most particular to the place).
    pub fn at(&self, s: &Situation) -> Option<(Effect, String)> {
        let mut e = Effect::default();
        let mut best: Option<&Rule> = None;
        for r in self.rules.iter().filter(|r| r.holds(s)) {
            match r.cap() {
                Some(c) => e.cap = Some(e.cap.map_or(c, |o: f32| o.min(c))),
                None => e.flow = true,
            }
            if best.is_none_or(|b| r.when.len() > b.when.len()) {
                best = Some(r);
            }
        }
        best.map(|r| (e, r.label()))
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// A module's name as a value: letters, digits and `_`, not too long.
fn module_name(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= MAX_MODULE
        && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Whether `v` is a value of feature `f`.
fn valid(f: usize, v: &str) -> bool {
    if f == MODULE {
        return module_name(v);
    }
    VALUES.get(f).is_some_and(|vals| vals.contains(&v))
}

/// The conditions are enough for a rule: at least two, one of them saying something special about the place; for a
/// careful rule two (it holds the ball back wherever it holds: a sharp bend alone, or a bend alone, is almost every
/// level's every corner, and one bend near a reset point is not every bend).
fn enough(when: &[(usize, String)], kind: Kind) -> bool {
    let special = when.iter().filter(|(f, v)| distinctive(*f, v)).count();
    when.len() >= 2 && special >= if kind == Kind::Careful { 2 } else { 1 }
}

fn rule(line: &str, evidence: &str) -> Result<Rule, String> {
    let mut w = line.split_whitespace();
    let kind = match w.next() {
        Some("flow") => Kind::Flow,
        Some("careful") => Kind::Careful,
        Some(k) => return Err(format!("unknown kind {k:.16}")),
        None => return Err("no kind".into()),
    };
    let mut speed = None;
    match w.next() {
        Some("speed") if kind == Kind::Careful => {
            let s = w.next().ok_or("speed missing")?;
            if s.len() > 16 {
                return Err("speed: number too long".into());
            }
            let v: f32 = s.parse().map_err(|_| "speed: not a number")?;
            if !(MIN_SPEED..=MAX_SPEED).contains(&v) {
                return Err("speed out of range".into());
            }
            speed = Some(v);
            if w.next() != Some("when") {
                return Err("no 'when'".into());
            }
        }
        Some("when") => {}
        _ => return Err("no 'when'".into()),
    }
    let mut when: Vec<(usize, String)> = Vec::new();
    for cond in w {
        let (f, v) = cond
            .split_once('=')
            .ok_or_else(|| format!("not feature=value: {cond:.32}"))?;
        let k = FEATURES
            .iter()
            .position(|x| *x == f)
            .ok_or_else(|| format!("unknown feature {f:.16}"))?;
        if !valid(k, v) {
            return Err(format!("{f}: unknown value {v:.32}"));
        }
        if when.iter().any(|c| c.0 == k) {
            return Err(format!("{f} twice"));
        }
        when.push((k, v.to_owned()));
    }
    if !enough(&when, kind) {
        return Err(match kind {
            Kind::Careful => {
                "too plain: a careful rule names two features special to the place (it holds the ball back \
                 wherever it holds)"
            }
            Kind::Flow | Kind::Calm | Kind::Plain => "too plain: a rule names two features or more, one special to the place",
        }
        .into());
    }
    // (a plain line of words: no comment mark or control character in it, as it is written back)
    let evidence: String = evidence
        .chars()
        .map(|c| if c == '#' || c.is_control() { ' ' } else { c })
        .collect();
    let evidence: String = evidence.trim().chars().take(200).collect();
    let evidence = evidence.trim_end().to_owned();
    Ok(Rule {
        kind,
        speed,
        when,
        evidence,
    })
}

/// The rules of a file's text, and a note for each line left out.
pub fn parse(text: &str) -> (Learned, Vec<String>) {
    // (a leading byte order mark, as some editors write, is no part of the first line)
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rules = Vec::new();
    let mut notes = Vec::new();
    // (the notes quote the file: shown with its control and format characters escaped)
    let mut note = |n: String| {
        if notes.len() < MAX_NOTES {
            notes.push(gamedata::package::printable(&n));
        }
    };
    for (i, raw) in text.lines().enumerate() {
        let (line, evidence) = raw.split_once('#').unwrap_or((raw, ""));
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let number = i.saturating_add(1);
        if rules.len() >= MAX_RULES {
            note(format!(
                "line {number}: more than {MAX_RULES} rules, the rest left out"
            ));
            break;
        }
        match rule(line, evidence) {
            Ok(r) => rules.push(r),
            Err(e) => note(format!("line {number}: {e}; left out")),
        }
    }
    (Learned { rules }, notes)
}

/// The learned table at `path`: none there is no rules; too big, a link or not UTF-8 is none, with a note.
pub fn load(path: &Path) -> (Learned, Vec<String>) {
    if let Err(e) = std::fs::symlink_metadata(path)
        && e.kind() == std::io::ErrorKind::NotFound
    {
        return (Learned::default(), Vec::new());
    }
    let text = crate::hints::read_plain(path, MAX_FILE)
        .and_then(|b| String::from_utf8(b).map_err(|_| format!("{}: not UTF-8", path.display())));
    match text {
        Ok(t) => {
            let (l, notes) = parse(&t);
            (
                l,
                notes
                    .into_iter()
                    .map(|n| format!("{}: {n}", path.display()))
                    .collect(),
            )
        }
        Err(e) => (Learned::default(), vec![format!("{e}; no rules read")]),
    }
}

/// A rule learned from a marked stretch: the places along it (`samples`), the zone's kind and limit. The conditions
/// are what was the same over most of the stretch (`SHARE`): the surface, the ball, and the features special to the
/// place; the features as plain as anywhere are left out (they would only make the rule hold in fewer places). None
/// when nothing special was the same along it: the zone stays the level's own.
pub fn learn(
    samples: &[Situation],
    kind: Kind,
    speed: Option<f32>,
    evidence: String,
) -> Option<Rule> {
    const SHARE: f32 = 0.6;
    if samples.is_empty() {
        return None;
    }
    let mut when = Vec::new();
    for f in 0..FEATURES.len() {
        let mut counts: Vec<(&str, usize)> = Vec::new();
        for v in samples
            .iter()
            .filter_map(|s| s.values.get(f).and_then(|v| v.as_deref()))
        {
            match counts.iter_mut().find(|c| c.0 == v) {
                Some(c) => c.1 += 1,
                None => counts.push((v, 1)),
            }
        }
        let Some(&(v, n)) = counts.iter().max_by_key(|c| c.1) else {
            continue;
        };
        if (n as f32) < SHARE * samples.len() as f32 || !valid(f, v) {
            continue;
        }
        let name = FEATURES.get(f).copied().unwrap_or_default();
        if name == "surface" || name == "kind" || distinctive(f, v) {
            when.push((f, v.to_owned()));
        }
    }
    enough(&when, kind).then_some(Rule {
        kind,
        speed,
        when,
        evidence,
    })
}

/// A rule learned now, added at the end of the file (made with a heading when new): true. False when the file has
/// the same rule already (the same kind, limit and conditions): it is not added again. A file already holding
/// MAX_RULES rules is full: the rule is not added, and the error says so.
pub fn append(path: &Path, r: &Rule) -> Result<bool, String> {
    let mut text = crate::hints::read_text(path, MAX_FILE)?;
    let known = parse(&text).0;
    if known
        .rules
        .iter()
        .any(|k| k.kind == r.kind && k.speed == r.speed && k.when == r.when)
    {
        return Ok(false);
    }
    if known.rules.len() >= MAX_RULES {
        return Err(format!(
            "{}: full ({MAX_RULES} rules), delete some",
            path.display()
        ));
    }
    if text.is_empty() {
        text = "# What the test player learned from the watcher's marks, for every level (autopilot-rules.md,\n\
                # \"The learned table\"): KIND [speed V] when FEATURE=VALUE ...   # where it was learned\n"
            .to_owned();
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text += &r.line();
    text.push('\n');
    if u64::try_from(text.len()).map_or(true, |n| n > MAX_FILE) {
        return Err(format!("{}: full ({MAX_FILE} bytes)", path.display()));
    }
    crate::hints::write_plain(path, &text)?;
    Ok(true)
}

/// Today's date (UTC), for a rule's evidence.
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    // civil from days (proleptic Gregorian)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sit(pairs: &[(&str, &str)]) -> Situation {
        let mut s = Situation::default();
        for (f, v) in pairs {
            let k = FEATURES.iter().position(|x| x == f).unwrap();
            s.values[k] = Some((*v).to_owned());
        }
        s
    }

    #[test]
    fn rules_read_write_back_and_hold_where_their_features_are() {
        let (l, notes) = parse(
            "# learned\nflow when surface=rail ahead=rail-end kind=wood   # Level_01 sector 2\n\
             careful speed 3.5 when surface=floor width=narrow curve=sharp\n\
             careful when module=P_Modul_25 surface=floor curve=bend\n",
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(l.rules.len(), 3);
        assert_eq!(l.rules[0].evidence, "Level_01 sector 2");
        for r in &l.rules {
            let (again, n) = parse(&r.line());
            assert!(n.is_empty(), "{n:?}");
            assert_eq!(&again.rules[0], r);
        }
        let here = sit(&[
            ("surface", "rail"),
            ("ahead", "rail-end"),
            ("kind", "wood"),
            ("curve", "bend"),
        ]);
        let (e, label) = l.at(&here).unwrap();
        assert!(e.flow && e.cap.is_none());
        assert_eq!(label, "rail, rail-end ahead, wood → don't dawdle");
        // another ball: not the place the rule knows
        assert!(
            l.at(&sit(&[
                ("surface", "rail"),
                ("ahead", "rail-end"),
                ("kind", "paper")
            ]))
            .is_none()
        );
        let narrow = sit(&[
            ("surface", "floor"),
            ("width", "narrow"),
            ("curve", "sharp"),
            ("kind", "stone"),
        ]);
        assert_eq!(l.at(&narrow).unwrap().0.cap, Some(3.5));
    }

    #[test]
    fn a_malformed_or_too_plain_rule_is_left_out_with_a_note() {
        for bad in [
            "flow surface=rail ahead=gap",
            "flow when",
            "flow when surface=floor kind=wood",
            "flow when ahead=gap",
            "flow when surface=rail surface=floor ahead=gap",
            "flow when surface=water ahead=gap",
            "flow when colour=red ahead=gap",
            "flow when surface=rail ahead=gap extra",
            "flow speed 3 when surface=rail ahead=gap",
            "careful speed 0 when surface=rail ahead=gap",
            "careful speed nan when surface=rail ahead=gap",
            "careful speed 99 when surface=rail ahead=gap",
            "careful speed when surface=rail ahead=gap",
            "careful when module=../x surface=rail",
            "careful when module=P_Modul_0123456789012345678901234567 surface=rail",
            "fast when surface=rail ahead=gap",
            // a careful rule names two features special to the place
            "careful when surface=floor curve=sharp kind=wood",
            "careful speed 3 when module=P_Modul_25 surface=floor",
        ] {
            let (l, notes) = parse(&format!("\nflow when surface=rail ahead=gap\n{bad}\n"));
            assert_eq!(l.rules.len(), 1, "{bad}");
            assert_eq!(notes.len(), 1, "{bad}");
            assert!(notes[0].starts_with("line 3:"), "{bad}: {}", notes[0]);
        }
        let many = "flow when surface=rail ahead=gap\n".repeat(MAX_RULES + 3);
        let (l, notes) = parse(&many);
        assert_eq!((l.rules.len(), notes.len()), (MAX_RULES, 1));
        assert_eq!(parse(&"x\n".repeat(500)).1.len(), MAX_NOTES);
    }

    #[test]
    fn a_rule_is_learned_from_what_the_stretch_had_in_common() {
        let mut samples = vec![
            sit(&[
                ("surface", "rail"),
                ("curve", "straight"),
                ("slope", "down"),
                ("ahead", "rail-end"),
                ("kind", "wood"),
                ("module", "none")
            ]);
            8
        ];
        // a few places differ: under the share they are no condition
        samples.push(sit(&[
            ("surface", "floor"),
            ("curve", "bend"),
            ("slope", "flat"),
            ("ahead", "none"),
            ("kind", "wood"),
            ("module", "none"),
        ]));
        let r = learn(&samples, Kind::Flow, None, "here".into()).unwrap();
        assert_eq!(
            r.line(),
            "flow when surface=rail slope=down ahead=rail-end kind=wood   # here"
        );
        // a stretch with nothing special: no rule
        let plain = vec![
            sit(&[
                ("surface", "floor"),
                ("curve", "straight"),
                ("kind", "wood")
            ]);
            5
        ];
        assert!(learn(&plain, Kind::Flow, None, String::new()).is_none());
        assert!(learn(&[], Kind::Flow, None, String::new()).is_none());
        // one special feature teaches a flow rule, not a careful one (it would slow every sharp bend)
        let sharp = vec![
            sit(&[
                ("surface", "floor"),
                ("curve", "sharp"),
                ("slope", "flat"),
                ("kind", "wood")
            ]);
            5
        ];
        assert!(learn(&sharp, Kind::Flow, None, String::new()).is_some());
        assert!(learn(&sharp, Kind::Careful, None, String::new()).is_none());
    }

    #[test]
    fn a_rule_learned_by_a_module_holds_only_by_that_module() {
        let (l, notes) =
            parse("careful when surface=floor curve=bend module=PR_Resetpoint kind=wood\n");
        assert!(notes.is_empty(), "{notes:?}");
        let bend = |module: &str| {
            sit(&[
                ("surface", "floor"),
                ("curve", "bend"),
                ("module", module),
                ("kind", "wood"),
            ])
        };
        assert!(l.at(&bend("PR_Resetpoint")).is_some());
        assert!(
            l.at(&bend("none")).is_none(),
            "a bend with no reset point near"
        );
        assert!(l.at(&bend("P_Box")).is_none());
    }

    #[test]
    fn today_is_a_date() {
        let t = today();
        assert_eq!(t.len(), 10, "{t}");
        assert!(t.starts_with("20"), "{t}");
    }

    #[test]
    fn a_rule_known_already_is_not_added_again_and_a_mark_undone_by_its_id() {
        let dir = std::env::temp_dir().join(format!("drahtseilakt-learned-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("learned.txt");
        let (l, _) = parse(
            "flow when surface=rail ahead=rail-end kind=wood\ncareful when surface=rail ahead=gap\n",
        );
        let mut a = l.rules[0].clone();
        a.evidence = "mark=11112222: level A".into();
        assert!(append(&path, &a).unwrap());
        let mut again = a.clone();
        again.evidence = "mark=33334444: level B".into();
        assert!(
            !append(&path, &again).unwrap(),
            "the same rule is not added twice"
        );
        let mut b = l.rules[1].clone();
        b.evidence = "mark=55556666: level C".into();
        assert!(append(&path, &b).unwrap());
        assert_eq!(load(&path).0.rules.len(), 2);
        // undone by id: the first, though it is not the last line
        assert!(
            crate::zones::remove_mark(&path, "11112222")
                .unwrap()
                .is_some()
        );
        assert_eq!(load(&path).0.rules, vec![b]);
        // a byte order mark before the first line is no part of it; notes show control characters escaped
        assert_eq!(
            parse("\u{feff}flow when surface=rail ahead=gap\n")
                .0
                .rules
                .len(),
            1
        );
        let (_, notes) = parse("flow when surface=\u{1b}[31m ahead=gap\n");
        assert!(notes[0].contains("\\u{1b}"), "{notes:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
