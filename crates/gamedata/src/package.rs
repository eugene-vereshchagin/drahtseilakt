// What a level is in the custom levels' folders (docs/drahtseilakt/levels.md, "A level is a folder"): a folder directly
// in `levels/community/` or `levels/open/` holding exactly one level — one `.nmo` file or one open-format level
// (level.gltf) — with an optional `level.txt` naming it, or a single `.nmo` dropped loose into the folder. Anything
// else in a level's folder and any deeper folder is not looked at, so a player's archive dump shows nothing but
// levels. The folders come from strangers: links are never followed, counts are capped, level.txt is read up to a
// few KiB and its text cleaned of control and direction characters, and a title is at most TITLE_MAX characters.

use std::io::Read;
use std::path::{Path, PathBuf};

/// The longest title shown, in characters (not bytes): longer ones are cut at a word and end in "…".
pub const TITLE_MAX: usize = 40;
/// The longest author or version kept, in characters.
pub const FIELD_MAX: usize = 40;
/// A level's own description of itself: `key = value` lines (title, author, version).
pub const INFO_FILE: &str = "level.txt";
/// The largest level.txt read; a larger one is ignored as not ours.
pub const INFO_MAX: u64 = 4096;
/// Entries of a levels folder looked at, and of each level's folder.
const ENTRIES_MAX: usize = 10_000;
const FOLDER_ENTRIES_MAX: usize = 4096;

/// One level of a custom levels folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    /// What the game loads: the `.nmo` file, or the open-format level's folder.
    pub level: PathBuf,
    /// The level's folder (the loose `.nmo` itself when it has none).
    pub folder: PathBuf,
    /// Cleaned and at most TITLE_MAX characters: level.txt's title, else the folder's (or loose file's) name.
    pub title: String,
    pub author: Option<String>,
    pub version: Option<String>,
}

/// Why a folder in a levels folder is not a level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Skipped {
    /// No `.nmo` and no level.gltf directly in it.
    Empty(PathBuf),
    /// More than one level file (several `.nmo`, or `.nmo` beside level.gltf).
    Several(PathBuf, usize),
}

impl std::fmt::Display for Skipped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Skipped::Empty(p) => write!(f, "{}: no level in it", p.display()),
            Skipped::Several(p, n) => write!(f, "{}: {n} level files, a level folder holds one", p.display()),
        }
    }
}

/// The levels of a custom levels folder, by title (then by path), and the folders that are not levels. Hidden entries
/// (a name beginning with a dot), links and other files are passed over silently.
pub fn scan(root: &Path) -> (Vec<Package>, Vec<Skipped>) {
    let (mut found, mut skipped) = (Vec::new(), Vec::new());
    for (path, kind) in entries(root, ENTRIES_MAX) {
        if kind.is_file() && is_nmo(&path) {
            found.push(loose(&path));
        } else if kind.is_dir() {
            match examine(&path) {
                Ok(p) => found.push(p),
                Err(s) => skipped.push(s),
            }
        }
    }
    found.sort_by(|a, b| {
        let key = |p: &Package| p.title.to_lowercase();
        key(a).cmp(&key(b)).then_with(|| a.level.cmp(&b.level))
    });
    (found, skipped)
}

/// The level a folder holds: exactly one `.nmo` directly in it, or it is an open-format level (level.gltf).
pub fn examine(folder: &Path) -> Result<Package, Skipped> {
    let list = entries(folder, FOLDER_ENTRIES_MAX);
    let open = list.iter().any(|(p, k)| k.is_file() && name_is(p, "level.gltf"));
    let nmos: Vec<&PathBuf> = list
        .iter()
        .filter(|(p, k)| k.is_file() && is_nmo(p))
        .map(|(p, _)| p)
        .collect();
    let level = match (open, nmos.as_slice()) {
        (true, []) => folder.to_path_buf(),
        (false, [one]) => (*one).clone(),
        (false, []) => return Err(Skipped::Empty(folder.to_path_buf())),
        (o, more) => return Err(Skipped::Several(folder.to_path_buf(), more.len().saturating_add(usize::from(o)))),
    };
    let info = read_info(&folder.join(INFO_FILE));
    let fallback = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Package {
        level,
        folder: folder.to_path_buf(),
        title: info.title.unwrap_or_else(|| clean(&fallback, TITLE_MAX)),
        author: info.author,
        version: info.version,
    })
}

/// A loose `.nmo` in the levels folder: a level of its own, named by its file.
fn loose(file: &Path) -> Package {
    let stem = file
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Package {
        level: file.to_path_buf(),
        folder: file.to_path_buf(),
        title: clean(&stem, TITLE_MAX),
        author: None,
        version: None,
    }
}

/// The title to show for a level the game plays (`level` an `.nmo` or an open-format folder): its folder's when the
/// level is in a level folder of `roots` (the custom levels folders) or is itself one, else its file's name; cleaned
/// and capped either way.
pub fn title_of(level: &Path, roots: &[PathBuf]) -> String {
    let same = |a: &Path, b: &Path| {
        a == b || matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
    };
    let in_roots = |p: Option<&Path>| p.is_some_and(|p| roots.iter().any(|r| same(p, r)));
    let folder = if is_nmo(level) { level.parent() } else { Some(level) };
    if let Some(folder) = folder.filter(|f| in_roots(f.parent()))
        && let Ok(p) = examine(folder)
        && same(&p.level, level)
    {
        return p.title;
    }
    let stem = level
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    clean(&stem, TITLE_MAX)
}

/// What level.txt says, each value cleaned; nothing when it is missing, a link, larger than INFO_MAX or unreadable.
#[derive(Default)]
struct Info {
    title: Option<String>,
    author: Option<String>,
    version: Option<String>,
}

fn read_info(path: &Path) -> Info {
    let Some(bytes) = read_small(path, INFO_MAX) else {
        return Info::default();
    };
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let text = String::from_utf8_lossy(bytes);
    let mut info = Info::default();
    for line in text.lines().take(64) {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let slot = match key.trim().to_ascii_lowercase().as_str() {
            "title" => (&mut info.title, TITLE_MAX),
            "author" => (&mut info.author, FIELD_MAX),
            "version" => (&mut info.version, FIELD_MAX),
            _ => continue,
        };
        let v = clean(value, slot.1);
        if slot.0.is_none() && !v.is_empty() {
            *slot.0 = Some(v);
        }
    }
    info
}

/// A plain file's bytes, at most `cap` of them: a link, a folder or a larger file gives nothing.
fn read_small(path: &Path, cap: u64) -> Option<Vec<u8>> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > cap {
        return None;
    }
    let file = std::fs::File::open(path).ok()?;
    // what was opened is checked again (it may have been swapped since), and never more than the cap is read
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1)).read_to_end(&mut bytes).ok()?;
    (u64::try_from(bytes.len()).ok()? <= cap).then_some(bytes)
}

/// Text fit to show: control, format and direction characters removed (no right-to-left overrides, no zero-width
/// joiners, no line breaks), runs of spaces made one, trimmed, and at most `max` characters, a cut one cut at a
/// word and ending in "…" (the menu draws it as "...", which the game's font has).
pub fn clean(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut space = false;
    for c in s.chars() {
        if c.is_whitespace() || c == '\u{a0}' {
            space = true;
            continue;
        }
        if c.is_control() || is_format(c) {
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(c);
    }
    if out.chars().count() <= max {
        return out;
    }
    cut_words(&out, max)
}

/// `s` in at most `max` characters: when longer, cut at the last word boundary that leaves it at least half its
/// room (else in the word), separators at the cut dropped, and ended with "…".
pub fn cut_words(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_owned();
    }
    let room = max.saturating_sub(1);
    let at_space = (room / 2..=room)
        .rev()
        .find(|&i| chars.get(i).is_some_and(|c| c.is_whitespace()));
    let head: String = chars.iter().take(at_space.unwrap_or(room)).collect();
    let head = head.trim_end_matches(|c: char| c.is_whitespace() || "-–—,;:(/.".contains(c));
    format!("{head}…")
}

/// Unicode's invisible format characters that change how text around them is shown: the direction marks, embeddings,
/// overrides and isolates, zero-width spaces and joiners, the byte order mark, and the like.
fn is_format(c: char) -> bool {
    matches!(u32::from(c),
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 | 0x180B..=0x180F | 0x200B..=0x200F
        | 0x2028..=0x202F | 0x205F..=0x206F | 0x3164 | 0xFE00..=0xFE0F | 0xFEFF | 0xFFA0 | 0xFFF0..=0xFFFB
        | 0x1D173..=0x1D17A | 0xE0000..=0xE0FFF)
}

/// A folder's entries without following links: path and its own kind, hidden names left out, at most `max`.
fn entries(dir: &Path, max: usize) -> Vec<(PathBuf, std::fs::FileType)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .take(max)
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| Some((e.path(), e.file_type().ok()?)))
        .filter(|(_, k)| !k.is_symlink())
        .collect()
}

fn is_nmo(p: &Path) -> bool {
    p.extension().is_some_and(|x| x.eq_ignore_ascii_case("nmo"))
}

fn name_is(p: &Path, name: &str) -> bool {
    p.file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("drahtseilakt-package-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn titles_are_cleaned_and_capped_in_characters() {
        assert_eq!(clean("  A\tlevel \n name ", 40), "A level name");
        // right-to-left override, isolates, zero-width joiner, BOM, bell, escape
        assert_eq!(
            clean("\u{202e}evil\u{2066}\u{200d}\u{feff}\u{7}\u{1b}[31m", 40),
            "evil[31m"
        );
        let long = "Ж".repeat(100);
        let c = clean(&long, TITLE_MAX);
        assert_eq!(c.chars().count(), TITLE_MAX);
        assert!(c.starts_with("ЖЖЖ") && c.ends_with('…'));
        // at a word, the separator before the cut dropped
        let t = clean(
            "Ballance Skyscraper (3D Maze; Hard Ver.-Fortress Besieged) v1.3.2",
            TITLE_MAX,
        );
        assert_eq!(t, "Ballance Skyscraper (3D Maze; Hard…");
        assert_eq!(clean(&"汉".repeat(40), TITLE_MAX), "汉".repeat(40));
        assert_eq!(clean("\u{202e}\u{200b}", 40), "");
    }

    #[test]
    fn only_folders_with_one_level_are_levels() {
        let root = temp("scan");
        let mk = |p: &str, b: &[u8]| {
            let p = root.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b).unwrap();
        };
        mk("Good/map.NMO", b"");
        mk("Good/preview.jpg", b"");
        mk("Good/README.txt", b"");
        mk("Good/textures/a.png", b"");
        mk("Good/deeper/other.nmo", b"");
        mk("Two/a.nmo", b"");
        mk("Two/b.nmo", b"");
        mk("Mixed/a.nmo", b"");
        mk("Mixed/level.gltf", b"");
        mk("Open/level.gltf", b"");
        mk("Open/level.bin", b"");
        mk("Junk/notes.doc", b"");
        mk("Nested/Season 1/deep.nmo", b"");
        mk("Loose.nmo", b"");
        mk("pack.tar.gz", b"");
        mk("photo.jpg", b"");
        mk(".hidden/x.nmo", b"");
        mk(".y.nmo", b"");
        let (found, skipped) = scan(&root);
        let titles: Vec<&str> = found.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(titles, ["Good", "Loose", "Open"]);
        assert_eq!(found[0].level, root.join("Good/map.NMO"));
        assert_eq!(found[2].level, root.join("Open"));
        assert_eq!(skipped.len(), 4, "{skipped:?}");
        assert!(skipped.contains(&Skipped::Several(root.join("Two"), 2)));
        assert!(skipped.contains(&Skipped::Several(root.join("Mixed"), 2)));
        assert!(skipped.contains(&Skipped::Empty(root.join("Nested"))));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn level_txt_names_a_level_within_limits() {
        let root = temp("info");
        let level = |name: &str, info: &[u8]| {
            let d = root.join(name);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("x.nmo"), b"").unwrap();
            std::fs::write(d.join(INFO_FILE), info).unwrap();
            examine(&d).unwrap()
        };
        let p = level(
            "a",
            b"\xef\xbb\xbf# made by hand\ntitle = Sky \xe2\x80\xae Road\nauthor=Me\nversion = 1.2\n",
        );
        assert_eq!(
            (p.title.as_str(), p.author.as_deref(), p.version.as_deref()),
            ("Sky Road", Some("Me"), Some("1.2"))
        );
        let p = level("b", format!("title={}\n", "W".repeat(500)).as_bytes());
        assert_eq!(p.title.chars().count(), TITLE_MAX);
        // an empty or invisible title, broken UTF-8, and a huge file fall back to the folder's name
        assert_eq!(level("c", b"title = \xe2\x80\x8b\n").title, "c");
        assert_eq!(level("d", b"title = \xff\xfe ok\n").title, "\u{fffd}\u{fffd} ok");
        let mut huge = b"title = Huge\n".to_vec();
        huge.resize(usize::try_from(INFO_MAX).unwrap() + 1, b' ');
        assert_eq!(level("e", &huge).title, "e");
        // a folder name is cleaned and capped like a title
        let long = format!("{}\u{202e}", "N".repeat(60));
        assert_eq!(level(&long, b"").title, format!("{}…", "N".repeat(TITLE_MAX - 1)));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn links_are_never_followed() {
        use std::os::unix::fs::symlink;
        let root = temp("links");
        let outside = temp("links-outside");
        std::fs::write(outside.join("secret.nmo"), b"").unwrap();
        std::fs::write(outside.join("info.txt"), b"title = Stolen\n").unwrap();
        std::fs::create_dir_all(outside.join("lvl")).unwrap();
        std::fs::write(outside.join("lvl/x.nmo"), b"").unwrap();
        symlink(outside.join("lvl"), root.join("Linked folder")).unwrap();
        symlink(outside.join("secret.nmo"), root.join("linked.nmo")).unwrap();
        let d = root.join("Real");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("x.nmo"), b"").unwrap();
        symlink(outside.join("info.txt"), d.join(INFO_FILE)).unwrap();
        symlink(outside.join("secret.nmo"), d.join("second.nmo")).unwrap();
        let (found, skipped) = scan(&root);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].title.as_str(), &found[0].level), ("Real", &d.join("x.nmo")));
        assert!(skipped.is_empty());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn a_played_level_is_named_by_its_folder() {
        let root = temp("title");
        let d = root.join("Folder Name");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("Level_07.NMO"), b"").unwrap();
        std::fs::write(root.join("Loose One.nmo"), b"").unwrap();
        let roots = [root.clone()];
        assert_eq!(title_of(&d.join("Level_07.NMO"), &roots), "Folder Name");
        assert_eq!(title_of(&root.join("Loose One.nmo"), &roots), "Loose One");
        // outside the levels folders a level is named by its file
        assert_eq!(title_of(&d.join("Level_07.NMO"), &[]), "Level_07");
        let _ = std::fs::remove_dir_all(&root);
    }
}
