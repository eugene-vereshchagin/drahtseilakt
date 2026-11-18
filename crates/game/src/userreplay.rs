// The players' replays: saved on request (Options, Replays: Save replays Off / Ask / Always), kept in the game's
// `user-replays/` folder as `YYYY-MM-DD-HH-MM-SS-<level>.replay` (local time), listed and played from the Replays
// screen, and sendable to the developer for a bug. The file is an ordinary replay (replay.rs) with a few comment lines
// in front (`# saved`, `# result`), which the reader skips. Nothing here is written unless the player asked for it
// (the setting is Off by default); the development logs of `data/replays/` are a separate matter (app.rs, `dev_logs`).

use crate::replay::Replay;
use std::path::{Path, PathBuf};

/// The setting's key in settings.txt, and the one for a folder of the player's own that the Replays screen also lists.
pub const SETTING: &str = "save-replays";
pub const FOLDER_SETTING: &str = "replays-folder";

/// The folder's name under the game's data folder.
pub const DIR_NAME: &str = "user-replays";

/// The most replays one list shows, and the most files looked at (a folder of thousands stays cheap).
pub const MAX_LISTED: usize = 300;
const MAX_FILES: usize = 2000;

/// A run shorter than this (60 ticks a second) is not worth a replay unless it ended in a finish.
pub const MIN_TICKS: u64 = 5 * 60;

/// How long the Ask prompt of a lost ball stays up when nobody answers.
pub const PROMPT_SECS: f32 = 10.0;

/// What a player's replays do: nothing, ask after a run (and at a lost ball), or save every run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Off,
    Ask,
    Always,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Off, Mode::Ask, Mode::Always];

    pub fn key(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Ask => "ask",
            Mode::Always => "always",
        }
    }

    pub fn from_key(v: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.key() == v.trim())
    }

    /// The setting as the settings file holds it; anything else is Off.
    pub fn load(get: impl Fn(&str) -> Option<String>) -> Mode {
        get(SETTING)
            .as_deref()
            .and_then(Mode::from_key)
            .unwrap_or_default()
    }

    /// The next choice to the left (-1) or right (+1), round.
    pub fn next(self, by: i32) -> Mode {
        let i = Mode::ALL.iter().position(|&m| m == self).unwrap_or(0) as i32;
        Mode::ALL[(i + by).rem_euclid(Mode::ALL.len() as i32) as usize]
    }
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Finished,
    GameOver,
    /// Left before the end (restart, back to the menu, another level).
    Left,
    /// A ball lost, the run goes on.
    BallLost,
}

impl Outcome {
    pub fn key(self) -> &'static str {
        match self {
            Outcome::Finished => "finished",
            Outcome::GameOver => "game-over",
            Outcome::Left => "left",
            Outcome::BallLost => "ball-lost",
        }
    }

    pub fn from_key(v: &str) -> Option<Outcome> {
        [
            Outcome::Finished,
            Outcome::GameOver,
            Outcome::Left,
            Outcome::BallLost,
        ]
        .into_iter()
        .find(|o| o.key() == v.trim())
    }
}

/// A moment of the clock as the file name has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Stamp {
    /// `YYYY-MM-DD-HH-MM-SS`.
    pub fn file_part(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// `YYYY-MM-DD HH:MM`, as the list shows it.
    pub fn shown(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }

    /// A stamp read from the front of a file name (`YYYY-MM-DD-HH-MM-SS`), None when it is not one.
    pub fn from_file_name(name: &str) -> Option<Stamp> {
        let mut parts = name.splitn(7, '-');
        let mut num = |digits: usize| -> Option<u32> {
            let p = parts.next()?;
            (p.len() == digits && p.bytes().all(|b| b.is_ascii_digit()))
                .then(|| p.parse().ok())
                .flatten()
        };
        let s = Stamp {
            year: i32::try_from(num(4)?).ok()?,
            month: num(2)?,
            day: num(2)?,
            hour: num(2)?,
            minute: num(2)?,
            second: num(2)?,
        };
        ((1..=12).contains(&s.month)
            && (1..=31).contains(&s.day)
            && s.hour < 24
            && s.minute < 60
            && s.second < 62)
            .then_some(s)
    }

    /// The stamp of a UTC time in seconds since the epoch (civil-from-days, no leap seconds): the tests' clock.
    #[cfg(test)]
    pub fn from_unix_utc(secs: i64) -> Stamp {
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400);
        // Howard Hinnant's civil_from_days
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Stamp {
            year,
            month,
            day,
            hour: (rem / 3600) as u32,
            minute: (rem % 3600 / 60) as u32,
            second: (rem % 60) as u32,
        }
    }
}

/// The local time now, in the system's zone (chrono reads it; where it cannot, UTC).
pub fn local_now() -> Stamp {
    use chrono::{Datelike, Timelike};
    let t = chrono::Local::now();
    Stamp {
        year: t.year(),
        month: t.month(),
        day: t.day(),
        hour: t.hour(),
        minute: t.minute(),
        second: t.second().min(59),
    }
}

/// A level's name as a file name's part: letters and digits (any script), `-` and `_` kept, everything else (spaces,
/// dots, slashes, quotes, control characters) one `_`; at most 40 characters; never empty.
pub fn sanitize(name: &str) -> String {
    let stem = name
        .trim()
        .trim_end_matches(".NMO")
        .trim_end_matches(".nmo");
    let mut out = String::new();
    let mut gap = false;
    for c in stem.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            if gap && !out.is_empty() {
                out.push('_');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
        if out.chars().count() >= 40 {
            break;
        }
    }
    let out = out.trim_matches(['-', '_']).to_owned();
    if out.is_empty() {
        "level".to_owned()
    } else {
        out
    }
}

/// The file's name for a run: `YYYY-MM-DD-HH-MM-SS-<level>.replay`.
pub fn file_name(stamp: &Stamp, level: &str) -> String {
    format!("{}-{}.replay", stamp.file_part(), sanitize(level))
}

/// `name` in `dir`, or with `-2`, `-3`... before the extension when the file is already there; never an existing file's
/// name.
pub fn free_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if std::fs::symlink_metadata(&first).is_err() {
        return first;
    }
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    (2..)
        .map(|n| {
            dir.join(if ext.is_empty() {
                format!("{stem}-{n}")
            } else {
                format!("{stem}-{n}.{ext}")
            })
        })
        .find(|p| std::fs::symlink_metadata(p).is_err())
        .unwrap_or(first)
}

/// The comment lines a saved replay starts with.
pub fn with_header(text: &str, stamp: &Stamp, outcome: Outcome) -> String {
    format!(
        "# Drahtseilakt replay, saved by the player (docs: user-replays/README.txt)\n# saved {}\n# result {}\n{text}",
        stamp.shown(),
        outcome.key()
    )
}

/// Writes a replay's text into `dir` under a new name (never over a file: `create_new`), the folder made when it is
/// missing, with the folder's README beside it. Returns where it went.
pub fn save(dir: &Path, stamp: &Stamp, level: &str, text: &str) -> Result<PathBuf, String> {
    use std::io::Write as _;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    write_readme(dir);
    let name = file_name(stamp, level);
    // a name taken between the check and the write (two saves in one second) tries the next
    for _ in 0..50 {
        let path = free_path(dir, &name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                f.write_all(text.as_bytes())
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Err(format!("{}: no free name", dir.display()))
}

const README: &str = "Drahtseilakt replays\n\
====================\n\
\n\
Each .replay file here is one of your runs, saved because Options > Replays > Save replays was set to Ask or Always.\n\
The name is the date and time (your local time) and the level: YYYY-MM-DD-HH-MM-SS-level.replay.\n\
\n\
A replay holds only your key presses, never any game data. It plays only on the very level file it was made on,\n\
and only in a game version that knows its format.\n\
\n\
Play them from Options > Replays (Enter plays one, Esc stops). \"Open folder...\" there lists the replays of any other folder.\n\
You can delete the files you do not want. If the game has a bug, a replay of the run that shows it is the best report:\n\
send the file to the developer.\n\
\n\
By default no replay is saved (Save replays: Off).\n";

fn write_readme(dir: &Path) {
    let p = dir.join("README.txt");
    if std::fs::symlink_metadata(&p).is_err() {
        let _ = std::fs::write(p, README);
    }
}

/// Whether a replay can be played as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Ok,
    /// Recorded by another version of the game (the format is not read here): the words say which.
    Stale(String),
    /// Not a replay, or damaged.
    Broken(String),
}

/// One replay of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub path: PathBuf,
    /// The level it was recorded on (its file's name, as the replay says).
    pub level: String,
    /// When it was saved: from the file's name, else its `# saved` line.
    pub saved: Option<String>,
    /// The run's length in ticks.
    pub ticks: u64,
    pub outcome: Option<Outcome>,
    pub state: State,
}

/// The replay files of a folder, newest first (by the date in the name, then the name), at most `MAX_LISTED`. A file
/// that is not a replay, is too big or has a stale format is listed with its state, never skipped and never a panic;
/// anything that is not a plain file (a folder, a link) is left out.
pub fn scan(dir: &Path) -> Vec<Item> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut items: Vec<(String, Item)> = Vec::new();
    for e in read.flatten().take(MAX_FILES) {
        let path = e.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".replay") {
            continue;
        }
        if !std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file()) {
            continue;
        }
        let item = read_item(&path, &name);
        let key = item.saved.clone().unwrap_or_default();
        items.push((format!("{key}\u{0}{name}"), item));
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    items.truncate(MAX_LISTED);
    items.into_iter().map(|i| i.1).collect()
}

/// Several folders' replays as one list, newest first.
pub fn scan_all(dirs: &[PathBuf]) -> Vec<Item> {
    let mut all: Vec<Item> = dirs.iter().flat_map(|d| scan(d)).collect();
    all.sort_by(|a, b| {
        let key = |i: &Item| {
            (
                i.saved.clone().unwrap_or_default(),
                i.path.file_name().map(|n| n.to_os_string()),
            )
        };
        key(b).cmp(&key(a))
    });
    all.dedup_by(|a, b| a.path == b.path);
    all.truncate(MAX_LISTED);
    all
}

fn read_item(path: &Path, name: &str) -> Item {
    let broken = |why: &str| Item {
        path: path.to_path_buf(),
        level: String::new(),
        saved: Stamp::from_file_name(name).map(|s| s.shown()),
        ticks: 0,
        outcome: None,
        state: State::Broken(why.to_owned()),
    };
    let bytes = match crate::hints::read_plain(path, crate::hints::MAX_REPLAY_FILE) {
        Ok(b) => b,
        Err(e) => return broken(&e),
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return broken("not text");
    };
    let mut saved = Stamp::from_file_name(name).map(|s| s.shown());
    let mut outcome = None;
    for line in text.lines().take(12) {
        if let Some(v) = line.strip_prefix("# saved ") {
            saved = saved.or_else(|| Some(v.trim().chars().take(16).collect()));
        } else if let Some(v) = line.strip_prefix("# result ") {
            outcome = Outcome::from_key(v);
        }
    }
    match Replay::parse(&text) {
        Ok(r) => Item {
            path: path.to_path_buf(),
            level: r.level_name.clone(),
            saved,
            ticks: r.end,
            outcome,
            state: State::Ok,
        },
        Err(e) => {
            // a header of ours the reader does not know is a newer (or much older) game's replay; anything else is no replay
            let first = text
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with('#'))
                .unwrap_or_default();
            let state =
                if first.starts_with("drahtseilakt-replay ") && e.starts_with("not a replay") {
                    State::Stale(first.chars().take(40).collect())
                } else {
                    State::Broken(e.chars().take(80).collect())
                };
            Item {
                path: path.to_path_buf(),
                level: String::new(),
                saved,
                ticks: 0,
                outcome,
                state,
            }
        }
    }
}

/// The default folder.
pub fn default_dir() -> PathBuf {
    crate::levels::data_dir().join(DIR_NAME)
}

/// The folders the Replays screen lists: the default one, and the player's own when the settings name one.
pub fn dirs(get: impl Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut v = vec![default_dir()];
    if let Some(p) = get(FOLDER_SETTING)
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && *p != v[0])
    {
        v.push(p);
    }
    v
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "drahtseilakt-userreplay-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_setting_is_off_unless_asked() {
        assert_eq!(Mode::default(), Mode::Off);
        assert_eq!(Mode::load(|_| None), Mode::Off);
        assert_eq!(Mode::load(|_| Some("nonsense".into())), Mode::Off);
        assert_eq!(
            Mode::load(|k| (k == SETTING).then(|| "ask".to_owned())),
            Mode::Ask
        );
        assert_eq!(Mode::load(|_| Some("always".into())), Mode::Always);
        for m in Mode::ALL {
            assert_eq!(Mode::from_key(m.key()), Some(m));
        }
        assert_eq!(Mode::Always.next(1), Mode::Off);
        assert_eq!(Mode::Off.next(-1), Mode::Always);
    }

    #[test]
    fn names_are_dated_and_safe() {
        let s = Stamp {
            year: 2026,
            month: 11,
            day: 3,
            hour: 7,
            minute: 5,
            second: 9,
        };
        assert_eq!(
            file_name(&s, "Level_03.NMO"),
            "2026-11-03-07-05-09-Level_03.replay"
        );
        assert_eq!(
            file_name(&s, "../../etc/pass wd: \"x\".nmo"),
            "2026-11-03-07-05-09-etc_pass_wd_x.replay"
        );
        assert_eq!(sanitize(""), "level");
        assert_eq!(sanitize("///"), "level");
        assert_eq!(sanitize("Карта №1"), "Карта_1");
        assert_eq!(sanitize(&"W".repeat(100)).chars().count(), 40);
        assert!(!sanitize("a\0b\nc\\d/e").contains(['\0', '\n', '\\', '/']));
        assert_eq!(
            Stamp::from_file_name("2026-11-03-07-05-09-Level_03.replay"),
            Some(s)
        );
        assert_eq!(Stamp::from_file_name("best.replay"), None);
        assert_eq!(Stamp::from_file_name("2026-13-03-07-05-09-x"), None);
        assert_eq!(Stamp::from_unix_utc(0).file_part(), "1970-01-01-00-00-00");
        assert_eq!(Stamp::from_unix_utc(1_794_000_000).year, 2026);
    }

    #[test]
    fn a_saved_replay_never_overwrites_another() {
        let dir = temp("names");
        let s = Stamp::from_unix_utc(1_794_000_000);
        let a = save(&dir, &s, "Level_01.NMO", "one").unwrap();
        let b = save(&dir, &s, "Level_01.NMO", "two").unwrap();
        let c = save(&dir, &s, "Level_01.NMO", "three").unwrap();
        assert_ne!(a, b);
        assert!(b.to_string_lossy().ends_with("-Level_01-2.replay"));
        assert!(c.to_string_lossy().ends_with("-Level_01-3.replay"));
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "one");
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "two");
        assert!(
            dir.join("README.txt").is_file(),
            "the folder explains itself"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_list_reads_good_stale_and_broken_files_and_a_missing_folder() {
        let dir = temp("list");
        let mut r = Replay::new("Level_02.NMO", 0xabcd);
        r.record(1, 1);
        r.record(600, 0);
        let s = |n: i64| Stamp::from_unix_utc(1_794_000_000 + n);
        save(
            &dir,
            &s(0),
            "Level_02.NMO",
            &with_header(&r.to_text(), &s(0), Outcome::Finished),
        )
        .unwrap();
        // a later one, ended in a lost ball, and a stale one from another version, a broken one and some noise
        save(
            &dir,
            &s(100),
            "Level_02.NMO",
            &with_header(&r.to_text(), &s(100), Outcome::GameOver),
        )
        .unwrap();
        std::fs::write(
            dir.join("2026-01-01-00-00-00-Old.replay"),
            "drahtseilakt-replay 99\nlevel A 00\n",
        )
        .unwrap();
        std::fs::write(dir.join("junk.replay"), [0xffu8, 0xfe, 0, 1]).unwrap();
        std::fs::write(dir.join("empty.replay"), "").unwrap();
        std::fs::write(dir.join("half.replay"), "drahtseilakt-replay 11\nlevel\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "hello").unwrap();
        std::fs::create_dir_all(dir.join("sub.replay")).unwrap();
        let items = scan(&dir);
        assert_eq!(items.len(), 6, "{items:#?}");
        // newest first by the date in the name; the ones without a date last
        assert_eq!(items[0].outcome, Some(Outcome::GameOver));
        assert_eq!(items[0].level, "Level_02.NMO");
        assert_eq!(items[0].ticks, 600);
        assert_eq!(items[1].outcome, Some(Outcome::Finished));
        assert_eq!(items[1].state, State::Ok);
        let old = items
            .iter()
            .find(|i| i.path.ends_with("2026-01-01-00-00-00-Old.replay"))
            .unwrap();
        assert!(matches!(old.state, State::Stale(_)), "{:?}", old.state);
        for name in ["junk.replay", "empty.replay", "half.replay"] {
            let it = items.iter().find(|i| i.path.ends_with(name)).unwrap();
            assert!(
                matches!(it.state, State::Broken(_)),
                "{name}: {:?}",
                it.state
            );
        }
        assert!(scan(&dir.join("nothing here")).is_empty());
        assert_eq!(scan_all(&[dir.clone(), dir.clone()]).len(), 6);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_extra_folder_is_listed_when_the_settings_name_one() {
        let one = dirs(|_| None);
        assert_eq!(one.len(), 1);
        assert!(one[0].ends_with(DIR_NAME));
        let two = dirs(|k| (k == FOLDER_SETTING).then(|| "/tmp/some replays".to_owned()));
        assert_eq!(two.len(), 2);
        // a relative or empty value is no folder
        assert_eq!(dirs(|_| Some("relative/dir".into())).len(), 1);
        assert_eq!(dirs(|_| Some("  ".into())).len(), 1);
    }
}
