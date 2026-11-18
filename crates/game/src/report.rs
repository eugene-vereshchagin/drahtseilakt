// The map check a player reads before playing (docs/drahtseilakt/levels.md, "The map check"): the audit's kinds of
// issue (audit.rs) grouped into a few plain lines and one verdict. `drahtseilakt --audit-report DIR|FILE` writes a
// report.txt (`kind = count` lines, the audit's own kinds) into each level's folder; the menu reads it back and words
// it here in the player's language. The grouping is a table over the audit's kinds, not over maps, so a kind the
// audit gains is counted (as "other") until it is given a group, and a script block that becomes known stops
// appearing without a change here.

use crate::audit::{self, Issue, KINDS};

/// How a map plays, worst last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    /// Nothing the audit knows of is missing.
    Full,
    /// Looks or sounds differ; the way is the same.
    Differences,
    /// Something that moves or blocks the way is missing: the map may not be passable.
    Risky,
    /// The map does not load.
    Broken,
}

impl Verdict {
    /// The text key of the verdict's words.
    pub fn key(self) -> &'static str {
        match self {
            Verdict::Full => "report full",
            Verdict::Differences => "report differences",
            Verdict::Risky => "report risky",
            Verdict::Broken => "report broken",
        }
    }
}

/// A group of kinds: the text key of its line (with `{n}` for the count) and how much it matters.
struct Group {
    key: &'static str,
    kinds: &'static [&'static str],
    verdict: Verdict,
}

const GROUPS: [Group; 9] = [
    Group {
        key: "report scripts",
        kinds: &["script-block-unknown"],
        verdict: Verdict::Risky,
    },
    Group {
        key: "report moving",
        kinds: &["animation-played"],
        verdict: Verdict::Risky,
    },
    Group {
        key: "report parts",
        kinds: &["module-missing", "standin-solid"],
        verdict: Verdict::Risky,
    },
    Group {
        key: "report invisible",
        kinds: &[
            "loose-not-drawn",
            "body-no-material",
            "body-transparent",
            "standin-hidden",
        ],
        verdict: Verdict::Differences,
    },
    Group {
        key: "report textures",
        kinds: &["texture-missing"],
        verdict: Verdict::Differences,
    },
    Group {
        key: "report plain",
        kinds: &["body-plain", "part-no-material", "material-mode"],
        verdict: Verdict::Differences,
    },
    Group {
        key: "report sounds",
        kinds: &["sound-unplayed"],
        verdict: Verdict::Differences,
    },
    Group {
        key: "report frames",
        kinds: &["frame-not-drawn"],
        verdict: Verdict::Differences,
    },
    Group {
        key: "report skipped",
        kinds: &["class-skipped", "load-warning"],
        verdict: Verdict::Differences,
    },
];

/// The line for kinds no group names.
const OTHER: &str = "report other";

/// Kinds that are facts of the map, solid or hidden in the original too: nothing for the player to be told.
const QUIET: [&str; 2] = ["body-hidden", "stopper-hidden"];

/// Whether the audit lists a kind for its count only (its legend says "(count only)"): not an issue.
fn count_only(kind: &str) -> bool {
    KINDS
        .iter()
        .any(|(k, what)| *k == kind && what.ends_with("(count only)"))
}

/// What the check says of a map.
#[derive(Debug, PartialEq, Eq)]
pub struct Summary {
    pub verdict: Verdict,
    /// The lines, the ones that matter most first: (text key, count).
    pub lines: Vec<(&'static str, usize)>,
}

/// Groups the audit's kinds of issue (kind, count) into the player's lines and a verdict.
pub fn summarize(issues: &[(String, usize)]) -> Summary {
    let mut broken = false;
    let mut lines: Vec<(&'static str, usize, Verdict)> = Vec::new();
    let mut other = 0usize;
    for (kind, n) in issues {
        if *n == 0 || count_only(kind) || QUIET.contains(&kind.as_str()) {
            continue;
        }
        if kind == "load-failed" {
            broken = true;
            continue;
        }
        match GROUPS.iter().find(|g| g.kinds.contains(&kind.as_str())) {
            None => other = other.saturating_add(*n),
            Some(g) => match lines.iter_mut().find(|l| l.0 == g.key) {
                Some(l) => l.1 = l.1.saturating_add(*n),
                None => lines.push((g.key, *n, g.verdict)),
            },
        }
    }
    if other > 0 {
        lines.push((OTHER, other, Verdict::Differences));
    }
    // the worst first, the table's order within a level of worry
    lines.sort_by_key(|l| std::cmp::Reverse(l.2));
    let verdict = if broken {
        Verdict::Broken
    } else {
        lines.iter().map(|l| l.2).max().unwrap_or(Verdict::Full)
    };
    Summary {
        verdict,
        lines: lines.into_iter().map(|(k, n, _)| (k, n)).collect(),
    }
}

/// The verdict and the lines as text in the language `text` speaks (a key to its words, `{n}` the count): the verdict
/// first, then at most `max` lines.
pub fn words(s: &Summary, language: &str, text: &dyn Fn(&str) -> String, max: usize) -> String {
    let mut out = text(s.verdict.key());
    for (key, n) in s.lines.iter().take(max) {
        out.push('\n');
        out.push_str(&plural(&text(key), language, *n).replace("{n}", &n.to_string()));
    }
    out
}

/// The form of a count's text for `n` in `language`: forms are separated by `||` (one, other; Russian: one, few,
/// many; Chinese has one); a text with fewer forms than the language wants uses its last.
pub fn plural(forms: &str, language: &str, n: usize) -> String {
    let at = match language {
        "ru" => {
            if n % 10 == 1 && n % 100 != 11 {
                0
            } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
                1
            } else {
                2
            }
        }
        // French counts 0 and 1 as the singular
        "fr" => usize::from(n >= 2),
        "zh" => 0,
        _ => usize::from(n != 1),
    };
    let parts: Vec<&str> = forms.split("||").collect();
    parts
        .get(at)
        .or(parts.last())
        .map_or_else(String::new, |p| (*p).to_owned())
}

/// A report.txt for the audit's issues.
pub fn file_text(issues: &[Issue]) -> String {
    let mut out = String::from(
        "# What the game does not support yet in this map (drahtseilakt --audit-report); the menu words it.\n",
    );
    for i in issues {
        // the names are the audit's to print; the report keeps the counts only
        out.push_str(&format!("{} = {}\n", i.kind, i.count));
    }
    out
}

/// `--audit-report`: checks every map under `roots` and writes report.txt into its level folder (a folder holding
/// exactly that map, as the game lists it); prints the verdict and lines of each map in the language `language`. A map
/// outside a level folder is checked and printed, nothing written beside it.
pub fn run(
    roots: &[std::path::PathBuf],
    data: &crate::assets::Data,
    language: &str,
) -> Result<(), String> {
    let maps: Vec<_> = roots.iter().flat_map(|r| audit::maps(r)).collect();
    if maps.is_empty() {
        return Err("--audit-report: no .nmo file or open-format level there".into());
    }
    let text = |k: &str| crate::lang::Strings::ours_for(k, language);
    let mut written = 0usize;
    for map in &maps {
        let issues = audit::map_issues(map, data);
        let kinds: Vec<(String, usize)> = issues
            .iter()
            .map(|i| (i.kind.to_owned(), i.count))
            .collect();
        let summary = summarize(&kinds);
        // the level's folder name (a title for a player), the path only for a map outside a level folder
        let shown = if map.is_dir() {
            Some(map.as_path())
        } else {
            map.parent()
        }
        .and_then(|f| {
            gamedata::package::examine(f)
                .ok()
                .filter(|p| p.level == *map)
        })
        .map_or_else(|| map.display().to_string(), |p| p.title);
        // one line a map, as a log line reads: its title, the verdict, then the lines in brackets
        let all = words(&summary, language, &text, usize::MAX);
        let mut parts = all.lines();
        let verdict = parts.next().unwrap_or_default();
        let rest: Vec<&str> = parts.collect();
        if rest.is_empty() {
            println!("{shown}: {verdict}");
        } else {
            println!("{shown}: {verdict} ({})", rest.join("; "));
        }
        let folder = if map.is_dir() {
            Some(map.as_path())
        } else {
            map.parent()
        };
        let level = folder.filter(|f| {
            gamedata::package::examine(f)
                .ok()
                .is_some_and(|p| p.level == *map)
        });
        if let Some(f) = level {
            crate::files::write_plain(
                &f.join(gamedata::package::REPORT_FILE),
                &file_text(&issues),
            )?;
            written = written.saturating_add(1);
        }
    }
    println!(
        "report: {} maps checked, {written} reports written",
        maps.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(v: &[(&str, usize)]) -> Vec<(String, usize)> {
        v.iter().map(|(a, b)| ((*a).to_owned(), *b)).collect()
    }

    #[test]
    fn a_clean_map_plays_fully() {
        let s = summarize(&k(&[
            ("texture-found", 40),
            ("curve-unused", 3),
            ("body-hidden", 2),
        ]));
        assert_eq!(s.verdict, Verdict::Full);
        assert!(s.lines.is_empty());
        assert_eq!(summarize(&[]).verdict, Verdict::Full);
    }

    #[test]
    fn looks_and_sounds_differ_scripts_and_motion_are_risky() {
        let s = summarize(&k(&[("texture-missing", 5), ("sound-unplayed", 2)]));
        assert_eq!(s.verdict, Verdict::Differences);
        assert_eq!(s.lines, [("report textures", 5), ("report sounds", 2)]);
        let s = summarize(&k(&[
            ("texture-missing", 5),
            ("script-block-unknown", 3),
            ("animation-played", 2),
        ]));
        assert_eq!(s.verdict, Verdict::Risky);
        // the worst first
        assert_eq!(s.lines.first().map(|l| l.0), Some("report scripts"));
        assert_eq!(s.lines.last().map(|l| l.0), Some("report textures"));
    }

    #[test]
    fn a_kind_without_a_group_is_other_and_a_failed_load_is_broken() {
        let s = summarize(&k(&[("something-new", 4)]));
        assert_eq!(s.verdict, Verdict::Differences);
        assert_eq!(s.lines, [(OTHER, 4)]);
        assert_eq!(
            summarize(&k(&[("load-failed", 1)])).verdict,
            Verdict::Broken
        );
    }

    #[test]
    fn every_kind_of_the_audit_is_grouped_quiet_or_counted_only() {
        for (kind, _) in KINDS {
            let known = GROUPS.iter().any(|g| g.kinds.contains(&kind))
                || QUIET.contains(&kind)
                || count_only(kind)
                || kind == "load-failed";
            assert!(known, "{kind} has no place in the map check");
        }
    }

    #[test]
    fn counts_take_the_plural_form_of_their_language() {
        let ru = "a||b||c";
        let at = |n| plural(ru, "ru", n);
        assert_eq!([1, 21, 101].map(at), ["a", "a", "a"]);
        assert_eq!([2, 3, 4, 22, 24].map(at), ["b"; 5]);
        assert_eq!([0, 5, 11, 12, 14, 20, 25, 111].map(at), ["c"; 8]);
        assert_eq!(
            (plural("x||y", "en", 1), plural("x||y", "en", 2)),
            ("x".into(), "y".into())
        );
        assert_eq!(
            (plural("x||y", "fr", 0), plural("x||y", "fr", 2)),
            ("x".into(), "y".into())
        );
        assert_eq!(plural("x", "zh", 7), "x");
        // every worded line has the forms its language needs
        for lang in ["en", "ru", "de", "zh", "fr", "es", "it"] {
            let need = match lang {
                "ru" => 3,
                "zh" => 1,
                _ => 2,
            };
            for g in GROUPS.iter().map(|g| g.key).chain([OTHER]) {
                let t = crate::lang::Strings::ours_for(g, lang);
                assert_eq!(t.split("||").count(), need, "{g} in {lang}");
            }
        }
        assert_eq!(
            crate::lang::Strings::ours_for("report plain", "en")
                .split("||")
                .next(),
            Some("{n} surface drawn plain")
        );
    }

    #[test]
    fn every_line_is_worded_in_every_language() {
        let verdicts = [
            Verdict::Full,
            Verdict::Differences,
            Verdict::Risky,
            Verdict::Broken,
        ]
        .map(Verdict::key);
        for lang in ["en", "ru", "de", "zh", "fr", "es", "it"] {
            for key in GROUPS.iter().map(|g| g.key).chain([OTHER]).chain(verdicts) {
                let t = crate::lang::Strings::ours_for(key, lang);
                assert_ne!(t, key, "{key} in {lang}");
            }
        }
    }
}
