//! The player's manual: local HTML pages (made by tools/make_manual.py into `assets/manual/`) opened with the system's
//! opener when the player asks (Options, Help, or F1 in the menu). Never during a run, never online: only a file
//! next to the game is opened, and a failure comes back as words for the menu's note. The one page outside the game,
//! the form for problem reports (Options, Report a problem), is opened the same way: the game hands the system's
//! browser a fixed address and never connects anywhere itself.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Where problems are reported: a new issue of the project on GitHub. Fixed, never built from anything the player or a
/// file gives (it goes to the system's opener, on Windows through `cmd /c start`, so it must hold no shell characters).
pub const REPORT_URL: &str = "https://github.com/eugene-vereshchagin/drahtseilakt/issues/new";

/// The languages the manual is written in; the others read English.
const LANGUAGES: [&str; 4] = ["en", "ru", "de", "zh"];

/// The page file of the manual in `language` (`page` is `index`, `levels`, ...) under the assets folder `root`.
pub fn page_path(root: &Path, language: &str, page: &str) -> PathBuf {
    let lang = if LANGUAGES.contains(&language) {
        language
    } else {
        "en"
    };
    root.join("manual").join(lang).join(format!("{page}.html"))
}

/// Opens the manual's page with the system's opener, detached. Err is the menu's words ("manual not found at ...").
pub fn open(strings: &crate::lang::Strings, page: &str) -> Result<(), String> {
    let root = gamedata::resources::assets_root().unwrap_or_default();
    let path = page_path(&root, strings.language(), page);
    if !path.is_file() {
        return Err(strings
            .ours("manual missing")
            .replace("{path}", &path.display().to_string()));
    }
    spawn(path.as_os_str()).map_err(|e| {
        strings
            .ours("manual failed")
            .replace("{error}", &e.to_string())
    })
}

/// Opens the page for problem reports in the system's browser, detached. Err is the menu's words.
pub fn report_problem(strings: &crate::lang::Strings) -> Result<(), String> {
    spawn(OsStr::new(REPORT_URL)).map_err(|e| {
        strings
            .ours("report failed")
            .replace("{error}", &e.to_string())
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn command(path: &OsStr) -> std::process::Command {
    let mut c = std::process::Command::new("xdg-open");
    c.arg(path);
    c
}

#[cfg(target_os = "macos")]
fn command(path: &OsStr) -> std::process::Command {
    let mut c = std::process::Command::new("open");
    c.arg(path);
    c
}

#[cfg(windows)]
fn command(path: &OsStr) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut c = std::process::Command::new("cmd");
    // the empty title keeps `start` from taking the path for it; no console window
    c.args(["/c", "start", ""])
        .arg(path)
        .creation_flags(0x0800_0000);
    c
}

fn spawn(path: &OsStr) -> std::io::Result<()> {
    let mut c = command(path);
    c.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let mut child = c.spawn()?;
    // the opener may outlive us or end at once: a thread reaps it so no zombie stays
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The report address is the project's new-issue form, plain https, with nothing a shell (cmd's `start`) would
    /// read as its own.
    #[test]
    fn the_report_address_is_fixed_and_plain() {
        assert_eq!(
            REPORT_URL,
            "https://github.com/eugene-vereshchagin/drahtseilakt/issues/new"
        );
        assert!(
            REPORT_URL
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-./:".contains(c))
        );
    }

    #[test]
    fn page_paths_fall_back_to_english() {
        let root = Path::new("/a");
        assert_eq!(
            page_path(root, "ru", "levels"),
            Path::new("/a/manual/ru/levels.html")
        );
        assert_eq!(
            page_path(root, "fr", "index"),
            Path::new("/a/manual/en/index.html")
        );
    }

    /// The generator builds every page and no internal link is broken (skipped where python3 or `markdown` is missing).
    #[test]
    fn the_generated_manual_is_whole() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = std::env::temp_dir().join(format!("manual-test-{}", std::process::id()));
        let run = std::process::Command::new("python3")
            .args(["-I", "tools/make_manual.py"])
            .arg(&out)
            .current_dir(&root)
            .output();
        let Ok(run) = run else { return };
        if !run.status.success() {
            let err = String::from_utf8_lossy(&run.stderr);
            if err.contains("No module named") {
                return;
            }
            panic!("generator failed: {err}");
        }
        let pages = [
            "index", "playing", "levels", "looks", "toolbox", "mods", "sound", "help",
        ];
        for lang in LANGUAGES {
            for p in pages {
                let f = out.join(lang).join(format!("{p}.html"));
                let html =
                    std::fs::read_to_string(&f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
                for href in hrefs(&html) {
                    if href.starts_with("http") || href.starts_with('#') {
                        continue;
                    }
                    let (file, _) = href.split_once('#').unwrap_or((&href, ""));
                    let target = out.join(lang).join(file);
                    assert!(target.is_file(), "{lang}/{p}.html links to {href}");
                }
            }
        }
        for f in [
            "index.html",
            "style.css",
            "fonts/Tektur-Variable.ttf",
            "fonts/NotoSansSC-Regular.otf",
        ] {
            assert!(out.join(f).is_file(), "{f}");
        }
        let _ = std::fs::remove_dir_all(&out);
    }

    fn hrefs(html: &str) -> Vec<String> {
        let mut v = Vec::new();
        for part in html.split("href=\"").skip(1) {
            if let Some((h, _)) = part.split_once('"') {
                v.push(h.to_owned());
            }
        }
        v
    }
}
