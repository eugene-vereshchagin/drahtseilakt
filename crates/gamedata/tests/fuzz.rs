// Fuzz targets of the files the game data layer reads (docs/drahtseilakt/fuzzing.md), on the stable fuzzer in
// crates/fuzzkit:
//   image        a texture or picture (.bmp/.tga/.png/.jpg) as the game decodes it, and its header as the pixel
//                budget reads it;
//   gputex       a level's compressed texture (.dtx, gputex.rs): its header, the whole file, its decoding;
//   gputex_data  the same with the data's hash set right first (the reader then goes on to the blocks and the
//                decoders), and the decoders on blocks of any bytes;
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
    // a 4096 x 4096 picture is within the caps: 256 MiB decoded, and again as RGBA
    t.rss_mb = 1100;
    fuzzkit::fuzz(t, image_seeds, run_image);
}

// --- compressed textures -----------------------------------------------------------------------------------------

/// The first byte picks how many of the largest levels are dropped, the rest is the file.
pub fn run_gputex(input: &[u8]) {
    use gamedata::gputex;
    let Some((&skip, bytes)) = input.split_first() else {
        return;
    };
    // the header alone, as the budget reads it: every size it gives is within the caps
    if let Ok(h) = gputex::header(bytes) {
        assert!(h.width <= gputex::SIDE_MAX && h.height <= gputex::SIDE_MAX);
        assert!(h.width % 4 == 0 && h.height % 4 == 0 && h.width >= 4 && h.height >= 4);
        assert!(
            h.file_len() <= gputex::FILE_MAX,
            "a file of {} bytes",
            h.file_len()
        );
        assert!(h.droppable() < h.mips.len());
    }
    let Ok(t) = gputex::texture(bytes.to_vec(), usize::from(skip)) else {
        return;
    };
    // what the renderer uploads as it is: every level inside the data, of exactly its size, whole blocks first
    let (w, h) = t.size();
    assert!(w % 4 == 0 && h % 4 == 0 && w >= 4 && h >= 4);
    for m in &t.mips {
        let level = t.level(m).expect("a level outside the data");
        assert_eq!(Some(level.len()), t.format.level_bytes(m.width, m.height));
    }
    // (a file can be 4096 x 4096 only by being that long: the fuzzer's are small, so decoding them is quick)
    let rgba = gputex::decoded(&t).expect("a read texture decodes");
    assert_eq!(rgba.mips.len(), t.mips.len());
    for m in &rgba.mips {
        assert_eq!(m.len as u64, u64::from(m.width) * u64::from(m.height) * 4);
    }
}

/// A file's hash is set to what its data hashes to, so the fuzzer's changes to the data get past the hash to the
/// block checks and the decoders; then the bytes after the first three are decoded as blocks of a size the first
/// three name (any bytes at all: what a card without the reader's checks would be given).
pub fn run_gputex_data(input: &[u8]) {
    use gamedata::gputex;
    let mut file = input.to_vec();
    if let Some(data) = file.get(1 + gputex::HEADER_LEN..) {
        let hash = gputex::data_hash(data).to_le_bytes();
        file[1 + 32..1 + 40].copy_from_slice(&hash);
    }
    run_gputex(&file);
    let [pick, w, h, blocks @ ..] = input else {
        return;
    };
    let format = if pick & 1 == 0 {
        gputex::Format::Bc1
    } else {
        gputex::Format::Bc7
    };
    let (w, h) = (u32::from(*w % 33) + 1, u32::from(*h % 33) + 1);
    let need = format.level_bytes(w, h).unwrap();
    let mut data = blocks.to_vec();
    data.resize(need, 0x40);
    let rgba =
        gputex::decode_level(format, w, h, &data).expect("blocks of the right length decode");
    assert_eq!(rgba.len() as u64, u64::from(w) * u64::from(h) * 4);
    // a cut or grown level is refused
    assert!(gputex::decode_level(format, w, h, &data[..need - 1]).is_none());
}

#[test]
#[ignore]
fn fuzz_gputex_data() {
    let mut t = Target::new("gputex_data", test_name!("fuzz_gputex_data"), Kind::Binary);
    t.max_len = 1 << 20;
    fuzzkit::fuzz(t, gputex_seeds, run_gputex_data);
}

fn gputex_seeds() -> Vec<Vec<u8>> {
    use gamedata::gputex::{Format, write};
    let mut out = Vec::new();
    for (format, block) in [
        (Format::Bc1, vec![0xff, 0xff, 0, 0, 0xe4, 0x1b, 0xe4, 0x1b]),
        (Format::Bc7, {
            let mut b = vec![0x40u8; 1];
            b.extend([0x5a; 15]);
            b
        }),
    ] {
        for (w, h) in [(4u32, 4u32), (8, 4), (16, 16), (32, 8)] {
            let (mut lw, mut lh, mut levels) = (w, h, Vec::new());
            loop {
                levels.push(block.repeat((lw.div_ceil(4) * lh.div_ceil(4)) as usize));
                if lw == 1 && lh == 1 {
                    break;
                }
                (lw, lh) = ((lw / 2).max(1), (lh / 2).max(1));
            }
            let file = write(format, w, h, (77, 99), &levels).expect("a seed is a valid file");
            for skip in [0u8, 1] {
                let mut s = vec![skip];
                s.extend(&file);
                out.push(s);
            }
        }
    }
    out
}

#[test]
#[ignore]
fn fuzz_gputex() {
    let mut t = Target::new("gputex", test_name!("fuzz_gputex"), Kind::Binary);
    t.max_len = 1 << 20;
    fuzzkit::fuzz(t, gputex_seeds, run_gputex);
}

/// The seeds themselves pass (the fuzzer's entry point, run on every test run).
#[test]
fn gputex_seeds_read() {
    for s in gputex_seeds() {
        run_gputex(&s);
        run_gputex_data(&s);
        assert!(gamedata::gputex::read(&s[1..]).is_ok());
    }
    // and cut anywhere, or with any one byte changed, a seed is refused or still sound
    let s = gputex_seeds().pop().unwrap();
    for cut in 0..s.len() {
        run_gputex(&s[..cut]);
    }
    for at in 1..s.len().min(200) {
        for bit in [1u8, 0x80] {
            let mut t = s.clone();
            t[at] ^= bit;
            run_gputex(&t);
            run_gputex_data(&t);
        }
    }
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
    fuzzkit::regressions("gputex", run_gputex);
    fuzzkit::regressions("gputex_data", run_gputex_data);
    fuzzkit::regressions("level_txt", run_level_txt);
    fuzzkit::regressions("theme_txt", run_theme_txt);
    fuzzkit::regressions("pack_txt", run_pack_txt);
    fuzzkit::regressions("sound_table", run_sound_table);
}
