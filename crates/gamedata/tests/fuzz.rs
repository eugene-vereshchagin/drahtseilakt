// Fuzz targets of the files the game data layer reads (docs/drahtseilakt/fuzzing.md), on the stable fuzzer in
// crates/fuzzkit:
//   image        a texture or picture (.bmp/.tga/.png/.jpg) as the game decodes it, and its header as the pixel
//                budget reads it;
//   level_txt    a level folder's level.txt;
//   theme_txt    a sound theme's theme.txt;
//   pack_txt     a texture or model pack's pack.txt (the same shape, read through packs.rs);
//   sound_table  a sound theme's table (sounds.txt / music.txt).
// #[ignore]: `FUZZ_SECS=600 cargo test --release --offline -p gamedata --test fuzz -- --ignored`. Seeds: synthetic
// files made here, the game's own shipped files (assets/), and the player's files when present (never committed).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use fuzzkit::{Kind, Target, test_name};
use std::path::{Path, PathBuf};

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

// --- pictures ----------------------------------------------------------------------------------------------------

const EXTS: [Option<&str>; 5] = [Some("bmp"), Some("tga"), Some("png"), Some("jpg"), None];

/// The first byte picks the file's extension (the format the reader is told), the rest is the file.
pub fn run_image(input: &[u8]) {
    let Some((&pick, bytes)) = input.split_first() else {
        return;
    };
    let ext = EXTS[usize::from(pick) % EXTS.len()];
    if let Some((w, h, rgba)) = gamedata::assets::decode_image(bytes, ext) {
        // what the renderer uploads as it is: a picture of at least one pixel, its bytes exactly its size
        assert!(w >= 1 && h >= 1, "a decoded picture of {w}x{h}");
        assert_eq!(
            rgba.len() as u64,
            u64::from(w) * u64::from(h) * 4,
            "a decoded picture's bytes"
        );
    }
    // the header alone, as the pixel budget of a level's own textures reads it
    let _ = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_dimensions().ok());
}

fn image_seeds() -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let formats = [
        image::ImageFormat::Bmp,
        image::ImageFormat::Tga,
        image::ImageFormat::Png,
        image::ImageFormat::Jpeg,
    ];
    for (k, format) in formats.iter().enumerate() {
        for (w, h) in [(1u32, 1u32), (3, 2), (8, 8)] {
            let img = image::RgbaImage::from_fn(w, h, |x, y| {
                image::Rgba([(x * 40) as u8, (y * 60) as u8, 7, 200])
            });
            let img = if *format == image::ImageFormat::Jpeg {
                image::DynamicImage::ImageRgb8(image::DynamicImage::ImageRgba8(img).to_rgb8())
            } else {
                image::DynamicImage::ImageRgba8(img)
            };
            let mut bytes = std::io::Cursor::new(Vec::new());
            if img.write_to(&mut bytes, *format).is_ok() {
                let mut s = vec![k as u8];
                s.extend(bytes.into_inner());
                out.push(s);
            }
        }
    }
    let mut roots = vec![assets()];
    roots.extend(fuzzkit::real_roots());
    for p in fuzzkit::files_under(&roots, &["bmp", "tga", "png", "jpg", "jpeg"], 48, 1 << 20) {
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let k = EXTS
            .iter()
            .position(|e| *e == Some(ext.as_str()))
            .unwrap_or(3);
        if let Ok(b) = std::fs::read(&p) {
            let mut s = vec![k as u8];
            s.extend(b);
            out.push(s);
        }
    }
    out
}

#[test]
#[ignore]
fn fuzz_image() {
    let mut t = Target::new("image", test_name!("fuzz_image"), Kind::Binary);
    t.max_len = 4 << 20;
    // an 8192 x 8192 picture is within the caps: 256 MiB decoded, and again as RGBA
    t.rss_mb = 1100;
    fuzzkit::fuzz(t, image_seeds, run_image);
}

// --- small text files ------------------------------------------------------------------------------------------

pub fn run_level_txt(input: &[u8]) {
    // the names shown for a level folder or file (its title, else its name) go through the same cleaning
    let text = String::from_utf8_lossy(input);
    for max in [1, 2, 7, gamedata::package::TITLE_MAX] {
        let c = gamedata::package::clean(&text, max);
        assert!(c.chars().count() <= max, "clean() over {max} characters");
        assert!(
            !c.chars().any(char::is_control),
            "clean() kept a control character"
        );
    }
    assert!(
        !gamedata::package::printable(&text)
            .chars()
            .any(char::is_control),
        "printable() kept a control character"
    );
    let info = gamedata::package::parse_info(input);
    assert!(
        info.difficulty
            .is_none_or(|d| (1..=gamedata::package::DIFFICULTY_MAX).contains(&d)),
        "a difficulty out of 1..10"
    );
    for (v, max) in [
        (&info.title, gamedata::package::TITLE_MAX),
        (&info.author, gamedata::package::FIELD_MAX),
        (&info.version, gamedata::package::FIELD_MAX),
    ] {
        if let Some(v) = v {
            assert!(v.chars().count() <= max, "a level.txt value over its cap");
            assert!(!v.chars().any(char::is_control), "a control character kept");
        }
    }
}

fn level_txt_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![
        b"title = A level\nauthor = Someone\nversion = 1.2\n".to_vec(),
        b"\xef\xbb\xbf# comment\nTITLE=x\n".to_vec(),
        "title = \u{202e}汉字 level with a very long title that goes on and on\n"
            .as_bytes()
            .to_vec(),
        b"difficulty = 7\ndifficulty=11\ndifficulty = -1\n".to_vec(),
    ];
    let mut roots = fuzzkit::real_roots();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".local/share/drahtseilakt/levels/community"));
    }
    out.extend(fuzzkit::read_all(&fuzzkit::files_under(
        &roots,
        &["level.txt"],
        32,
        4096,
    )));
    out
}

#[test]
#[ignore]
fn fuzz_level_txt() {
    let mut t = Target::new("level_txt", test_name!("fuzz_level_txt"), Kind::Text);
    t.max_len = 64 * 1024;
    t.dict = &["title", "author", "version", "=", "#"];
    fuzzkit::fuzz(t, level_txt_seeds, run_level_txt);
}

pub fn run_theme_txt(input: &[u8]) {
    let (names, fallback) =
        gamedata::sound_themes::parse_theme_txt(&String::from_utf8_lossy(input));
    assert!(names.len() <= 64);
    if let Some(f) = fallback {
        assert!(
            gamedata::sound_themes::plain(&f),
            "a fallback that is not a plain name: {f:?}"
        );
    }
}

fn theme_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![b"name.en Neuro\nname.ru \xd0\x9d\nfallback system\n# c\n".to_vec()];
    out.extend(fuzzkit::read_all(&fuzzkit::files_under(
        &[assets().join("sounds")],
        &["theme.txt"],
        8,
        4096,
    )));
    out
}

#[test]
#[ignore]
fn fuzz_theme_txt() {
    let mut t = Target::new("theme_txt", test_name!("fuzz_theme_txt"), Kind::Text);
    t.max_len = 16 * 1024;
    t.dict = &[
        "name.en", "name.ru", "name.de", "name.zh", "fallback", "..", "/",
    ];
    fuzzkit::fuzz(t, theme_seeds, run_theme_txt);
}

pub fn run_pack_txt(input: &[u8]) {
    let (names, fallback) = gamedata::packs::parse_pack_txt(&String::from_utf8_lossy(input));
    assert!(names.len() <= 64);
    assert!(names.iter().all(
        |(lang, name)| ["en", "ru", "de", "zh"].contains(&lang.as_str())
            && name.chars().count() <= 64
    ));
    if let Some(f) = fallback {
        assert!(
            gamedata::packs::plain(&f),
            "a fallback that is not a plain name: {f:?}"
        );
    }
}

fn pack_seeds() -> Vec<Vec<u8>> {
    vec![
        b"name.en Stone\nname.zh \xe7\x9f\xb3\nauthor A\nlicence CC0 1.0\nfallback own\n".to_vec(),
        b"\xef\xbb\xbfname.de X\nfallback none\n# c\n".to_vec(),
    ]
}

#[test]
#[ignore]
fn fuzz_pack_txt() {
    let mut t = Target::new("pack_txt", test_name!("fuzz_pack_txt"), Kind::Text);
    t.max_len = 16 * 1024;
    t.dict = &[
        "name.en", "name.ru", "name.de", "name.zh", "author", "licence", "fallback", "none", "..",
        "/",
    ];
    fuzzkit::fuzz(t, pack_seeds, run_pack_txt);
}

pub fn run_sound_table(input: &[u8]) {
    let t = gamedata::sound_themes::parse_table(input);
    for (role, files) in &t.roles {
        assert!(gamedata::sound_themes::known_role(role));
        assert!(!files.is_empty() && files.len() <= gamedata::sound_themes::MAX_POOL);
        assert!(files.iter().all(|f| gamedata::sound_themes::plain(f)));
    }
}

fn table_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![
        b"Music_Theme_1_1 a.ogg b.wav\n# c\nMisc_Fall fall.wav\nMusic_Atmo_2 x.ogg\n".to_vec(),
        b"\xef\xbb\xbfHit_Paper p.ogg\nUnknown y.ogg\nMenu_click ../x.ogg\n".to_vec(),
    ];
    out.extend(fuzzkit::read_all(&fuzzkit::files_under(
        &[assets().join("sounds")],
        &["txt"],
        16,
        16 * 1024,
    )));
    out
}

#[test]
#[ignore]
fn fuzz_sound_table() {
    let mut t = Target::new("sound_table", test_name!("fuzz_sound_table"), Kind::Text);
    t.max_len = 64 * 1024;
    t.dict = &[
        "Music_Theme_99_3",
        "Music_Theme_0",
        "Music_Atmo_3",
        "Roll_Paper",
        ".ogg",
        ".wav",
        "..",
        "/",
        "\\",
        "C:",
    ];
    fuzzkit::fuzz(t, table_seeds, run_sound_table);
}

// --- committed repros --------------------------------------------------------------------------------------------

#[test]
fn fuzz_regressions() {
    fuzzkit::regressions("image", run_image);
    fuzzkit::regressions("level_txt", run_level_txt);
    fuzzkit::regressions("theme_txt", run_theme_txt);
    fuzzkit::regressions("pack_txt", run_pack_txt);
    fuzzkit::regressions("sound_table", run_sound_table);
}
