// The "Modified originals" kind (docs/drahtseilakt/levels.md): community remakes of the original levels (the 45-degree
// rotated set, the reverse maps, the invisible maps). They carry the original's geometry, so the game never ships
// them: they live in the player's `levels/modified/<set>/<level folder or .nmo>` and are listed only when the
// player has a copy of the original. The set, the original level's number, the variant and the author are read out
// of the names ("反向Level_11(普通-fiype).NMO" is Reverse, Level 11, Normal, by fiype) and shown in the menu's language.

use gamedata::package::{Package, TITLE_MAX, clean};
use std::path::{Path, PathBuf};

use crate::levels::Kind;

/// Set folders read, and levels per set (a folder of strangers' files is never read without bounds).
const SETS_MAX: usize = 64;

/// A set of remakes, by what its folder is called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Set {
    Rotated,
    Reverse,
    Invisible,
    /// Any other folder, named as it is.
    Other(String),
}

impl Set {
    /// The set a folder holds, by its name (English or Chinese words of the known collections).
    pub fn of_folder(name: &str) -> Set {
        let l = name.to_lowercase();
        if l.contains("rotat") || l.contains("45-degree") || l.contains("旋转") {
            Set::Rotated
        } else if l.contains("revers") || l.contains("反向") {
            Set::Reverse
        } else if l.contains("invisib") || l.contains("隐形") {
            Set::Invisible
        } else {
            Set::Other(clean(name, TITLE_MAX))
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Set::Rotated => 0,
            Set::Reverse => 1,
            Set::Invisible => 2,
            Set::Other(_) => 3,
        }
    }

    fn key(&self) -> Option<&'static str> {
        match self {
            Set::Rotated => Some("modified set rotated"),
            Set::Reverse => Some("modified set reverse"),
            Set::Invisible => Some("modified set invisible"),
            Set::Other(_) => None,
        }
    }
}

/// A variant of a level a name gives, in the order they are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variant {
    Easy,
    Normal,
    Hard,
    New,
}

impl Variant {
    fn of_word(w: &str) -> Option<Variant> {
        match w.trim() {
            "新" => Some(Variant::New),
            "困难" => Some(Variant::Hard),
            "简单" => Some(Variant::Easy),
            "普通" => Some(Variant::Normal),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Variant::New => "modified new",
            Variant::Hard => "modified hard",
            Variant::Easy => "modified easy",
            Variant::Normal => "modified normal",
        }
    }
}

/// What a level's file or folder name says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Name {
    /// The original level's number ("Level_11", "Level 11").
    pub level: Option<u32>,
    pub variant: Option<Variant>,
    pub author: Option<String>,
    /// The name without the reverse prefix, the level number and the brackets: what is left to show for a map
    /// that is not a numbered level ("反向双轨世界.Level" is "双轨世界.Level").
    pub rest: String,
}

/// Reads a name (without its `.nmo`): an optional "反向" prefix, "Level" and a number, and a bracket "(variant)",
/// "(variant-author)" or "(author)".
pub fn parse_name(stem: &str) -> Name {
    let stem = stem.trim();
    let stem = stem.strip_prefix("反向").unwrap_or(stem);
    // the bracket at the end
    let (head, inner) = match (stem.rfind('('), stem.rfind(')')) {
        (Some(a), Some(b)) if a < b => {
            let after = stem.get(b.saturating_add(1)..).unwrap_or("x");
            match (stem.get(..a), stem.get(a.saturating_add(1)..b)) {
                (Some(h), Some(i)) if after.trim().is_empty() => (h, Some(i)),
                _ => (stem, None),
            }
        }
        _ => (stem, None),
    };
    let (mut variant, mut author) = (None, None);
    if let Some(inner) = inner {
        let (first, tail) = inner
            .split_once('-')
            .map_or((inner, None), |(a, b)| (a, Some(b)));
        match Variant::of_word(first) {
            Some(v) => {
                variant = Some(v);
                author = tail.map(str::to_owned);
            }
            None => author = Some(inner.to_owned()),
        }
    }
    let author = author
        .map(|a| clean(&a, TITLE_MAX))
        .filter(|a| !a.is_empty());
    // "Level" (any case), an optional "_" or space, and digits
    let lower = head.to_ascii_lowercase();
    let level = lower.find("level").and_then(|at| {
        let after = head.get(at.saturating_add(5)..)?;
        let digits: String = after
            .trim_start_matches(['_', ' '])
            .chars()
            .take_while(char::is_ascii_digit)
            .take(3)
            .collect();
        digits.parse::<u32>().ok()
    });
    Name {
        level,
        variant,
        author,
        rest: head.trim().to_owned(),
    }
}

/// The set folders inside the modified folder, by name.
pub fn set_folders(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(root)
        .map(|r| {
            r.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .map(|e| e.path())
                .take(SETS_MAX)
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// The text in `lang` for the key (English where the language has none).
fn word(lang: &str, key: &str) -> String {
    crate::lang::Strings::ours_for(key, lang)
}

/// A level's title in `lang`: "Reverse - Level 11 Normal"; a map that is no numbered level shows its name.
fn title_in(lang: &str, set: &Set, name: &Name) -> String {
    let set_text = match (set.key(), set) {
        (Some(k), _) => word(lang, k),
        (None, Set::Other(n)) => n.clone(),
        _ => String::new(),
    };
    let mut what = match name.level {
        Some(n) => format!("{} {n:02}", word(lang, "modified level")),
        None => name.rest.clone(),
    };
    if let Some(v) = name.variant {
        what = format!("{what} {}", word(lang, v.key()));
    }
    let text = if set_text.is_empty() {
        what
    } else {
        format!("{set_text} - {what}")
    };
    clean(&text, TITLE_MAX)
}

/// One scanned level of a set, with the title the names give in every language, its author, and the order.
fn describe(set: &Set, mut p: Package) -> (Package, Name) {
    let named = if p.level.is_file() {
        p.level.file_stem()
    } else {
        p.folder.file_name()
    };
    let name = parse_name(
        &named
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    // a level.txt of the player's own wins
    if !p.titled {
        p.title = title_in("en", set, &name);
        p.titles = crate::lang::languages()
            .iter()
            .filter(|(code, _)| *code != "en")
            .map(|(code, _)| ((*code).to_owned(), title_in(code, set, &name)))
            .collect();
        p.titled = true;
    }
    if p.author.is_none() {
        p.author = name.author.clone();
    }
    (p, name)
}

/// The levels of the modified folder `root`, grouped by set (rotated, reverse, invisible, the others by name), each
/// set by original level number, variant and author.
pub fn levels(root: &Path) -> Vec<Package> {
    let mut all: Vec<(Set, Name, Package)> = Vec::new();
    for folder in set_folders(root) {
        let set = Set::of_folder(
            &folder
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
        let (found, skipped) = gamedata::package::scan(&folder);
        for s in skipped.iter().take(10) {
            eprintln!("  {s}");
        }
        all.extend(found.into_iter().filter(|p| !p.hidden).map(|p| {
            let (p, name) = describe(&set, p);
            (set.clone(), name, p)
        }));
    }
    all.sort_by(|(sa, na, pa), (sb, nb, pb)| {
        let key = |s: &Set, n: &Name, p: &Package| {
            (
                s.rank(),
                format!("{s:?}"),
                n.level.is_none(),
                n.level,
                n.variant,
                n.author.clone(),
                p.title.to_lowercase(),
            )
        };
        key(sa, na, pa)
            .cmp(&key(sb, nb, pb))
            .then_with(|| pa.level.cmp(&pb.level))
    });
    all.into_iter().map(|(_, _, p)| p).collect()
}

/// Whether the modified folder holds a level at all: the menu offers the kind only then (and only with a copy).
pub fn any(root: &Path) -> bool {
    set_folders(root)
        .iter()
        .any(|f| !gamedata::package::scan(f).0.is_empty())
}

/// Every set, level and variant a name can give, titled in every language and credited to a long author: what the
/// menu's layout test lists (the player's own folder is never read there).
#[cfg(test)]
pub fn sample_packages() -> Vec<Package> {
    let mut v = Vec::new();
    for set in [
        Set::Rotated,
        Set::Reverse,
        Set::Invisible,
        Set::Other("A long collection's name".to_owned()),
    ] {
        for level in [Some(1), Some(11), Some(13), None] {
            for variant in [
                None,
                Some(Variant::Easy),
                Some(Variant::Normal),
                Some(Variant::Hard),
                Some(Variant::New),
            ] {
                let name = Name {
                    level,
                    variant,
                    author: Some("fiype-and-friends".to_owned()),
                    rest: "Double Track World.Level".to_owned(),
                };
                let mut titles: Vec<(String, String)> = Vec::new();
                for (code, _) in crate::lang::languages().iter().filter(|l| l.0 != "en") {
                    titles.push(((*code).to_owned(), title_in(code, &set, &name)));
                }
                v.push(Package {
                    level: PathBuf::from("x.nmo"),
                    folder: PathBuf::from("x.nmo"),
                    title: title_in("en", &set, &name),
                    author: name.author.clone(),
                    version: None,
                    posted: None,
                    report: None,
                    titles,
                    order: None,
                    hidden: false,
                    difficulty: level.map(|l| u8::try_from(l % 10 + 1).unwrap_or(1)),
                    titled: true,
                });
            }
        }
    }
    v
}

/// Wheter to offer the kind: a copy of the original is there and so are the levels.
pub fn offered(copy: bool) -> bool {
    copy && any(&crate::levels::dir(Kind::Modified))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn names_give_the_level_the_variant_and_the_author() {
        let n = parse_name("反向Level_11(普通-fiype)");
        assert_eq!(
            (n.level, n.variant, n.author.as_deref()),
            (Some(11), Some(Variant::Normal), Some("fiype"))
        );
        let n = parse_name("反向Level_01(新-Burning红云)");
        assert_eq!(
            (n.level, n.variant, n.author.as_deref()),
            (Some(1), Some(Variant::New), Some("Burning红云"))
        );
        let n = parse_name("反向Level_09(困难)");
        assert_eq!(
            (n.level, n.variant, n.author),
            (Some(9), Some(Variant::Hard), None)
        );
        assert_eq!(
            parse_name("反向Level_09(简单)").variant,
            Some(Variant::Easy)
        );
        assert_eq!(parse_name("反向Level_04").variant, None);
        // an author alone
        let n = parse_name("Level 05(Someone)");
        assert_eq!(
            (n.level, n.variant, n.author.as_deref()),
            (Some(5), None, Some("Someone"))
        );
        // a map with no number keeps its name
        let n = parse_name("反向双轨世界.Level");
        assert_eq!((n.level, n.rest.as_str()), (None, "双轨世界.Level"));
        assert_eq!(parse_name("45-Degree Rotated - Level 13").level, Some(13));
        // junk does not panic
        for s in [
            "",
            "(",
            ")(",
            "Level_",
            "Level_(新-)",
            "(新-a)(b",
            "反向",
            "Level_99999999999",
        ] {
            let _ = parse_name(s);
        }
        assert_eq!(parse_name("Level_(新-)").author, None);
    }

    #[test]
    fn sets_are_known_by_their_folders() {
        assert_eq!(Set::of_folder("45-Degree Rotated Maps"), Set::Rotated);
        assert_eq!(Set::of_folder("Reverse Maps"), Set::Reverse);
        assert_eq!(Set::of_folder("Invisible Maps"), Set::Invisible);
        assert_eq!(Set::of_folder("Mirror"), Set::Other("Mirror".to_owned()));
    }

    #[test]
    fn titles_come_in_every_language() {
        let set = Set::Reverse;
        let n = parse_name("反向Level_11(普通-fiype)");
        assert_eq!(title_in("en", &set, &n), "Reverse - Level 11 Normal");
        assert_eq!(title_in("ru", &set, &n), "Обратные - Уровень 11 Обычная");
        assert_eq!(title_in("de", &set, &n), "Rückwärts - Level 11 Normal");
        assert_eq!(title_in("zh", &set, &n), "反向 - 关卡 11 普通");
        assert_eq!(
            title_in("en", &Set::Rotated, &parse_name("Level_03")),
            "45-Degree Rotated - Level 03"
        );
        assert_eq!(
            title_in("en", &Set::Reverse, &parse_name("反向蓝星序曲")),
            "Reverse - 蓝星序曲"
        );
    }

    #[test]
    fn levels_are_grouped_by_set_and_ordered_by_number_and_variant() {
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-modified-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mk = |set: &str, folder: &str, file: &str| {
            let d = root.join(set).join(folder);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(file), b"").unwrap();
        };
        mk("Invisible Maps", "Level_01", "Level_01.NMO");
        mk(
            "Reverse Maps",
            "反向Level_11(普通-fiype)",
            "反向Level_11(普通-fiype).NMO",
        );
        mk(
            "Reverse Maps",
            "反向Level_11(困难)",
            "反向Level_11(困难).NMO",
        );
        mk("Reverse Maps", "反向Level_02", "反向Level_02.NMO");
        mk("45-Degree Rotated Maps", "Level_13", "Level_13.NMO");
        mk("Mirror", "Odd", "Odd.nmo");
        assert!(any(&root));
        let v = levels(&root);
        let titles: Vec<&str> = v.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "45-Degree Rotated - Level 13",
                "Reverse - Level 02",
                "Reverse - Level 11 Normal",
                "Reverse - Level 11 Hard",
                "Invisible - Level 01",
                "Mirror - Odd",
            ]
        );
        assert_eq!(v.get(2).and_then(|p| p.author.as_deref()), Some("fiype"));
        assert_eq!(
            v.get(1).map(|p| p.title_in("ru")),
            Some("Обратные - Уровень 02")
        );
        let _ = std::fs::remove_dir_all(&root);
        assert!(!any(&root), "no folder, no kind");
    }
}
