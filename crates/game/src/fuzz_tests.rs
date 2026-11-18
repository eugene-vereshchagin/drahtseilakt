// Fuzz targets of the files the game itself reads (docs/drahtseilakt/fuzzing.md), on the stable fuzzer in
// crates/fuzzkit. They live in the binary crate's tests because the parsers are its modules:
//   replay     a replay file (replay.rs), read and written back;
//   hints      a hint file of the test player (hints.rs);
//   zones      a zones file of the test player (zones.rs);
//   learned    the test player's learned table (learned.rs);
//   wav, ogg   a sound or music file as audio.rs decodes it;
//   highscore  the saved highscore table;
//   settings   the settings file as every reader of it takes its values (graphics, keys, extended mode's additions,
//              language, interface, volumes);
//   level_sim  a level (an .nmo, or the open format) loaded and played for a second: what a stranger's map does to
//              the game past its reader (the physics over its shapes, the rules over its objects);
//   module_sim a module file (a level folder may bring its own) in place of one an original level of the player's
//              copy uses, and that level played for a second (needs the copy).
// #[ignore]: `FUZZ_SECS=600 cargo test --release --offline -p drahtseilakt fuzz_tests -- --ignored`. Seeds: synthetic
// inputs, the repository's own hints and replays and shipped sounds, and the player's files when present.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use fuzzkit::{Kind, Target, test_name};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The developers' store's hints folder as a seed root (none without the store: the fuzzer then has the built-in seeds).
fn store_hints() -> Vec<PathBuf> {
    crate::devdata::for_test("the fuzz seeds from hints/")
        .map(|d| d.join("hints"))
        .into_iter()
        .collect()
}

fn profile(sub: &str) -> Vec<PathBuf> {
    if std::env::var_os("FUZZ_NO_REAL").is_some() {
        return Vec::new();
    }
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".local/share/drahtseilakt").join(sub))
        .into_iter()
        .filter(|p| p.is_dir())
        .collect()
}

// --- replays and hints -------------------------------------------------------------------------------------------

fn run_replay(input: &[u8]) {
    let Ok(text) = std::str::from_utf8(input) else {
        return;
    };
    if let Ok(r) = crate::replay::Replay::parse(text) {
        // what is read is written back readable
        let again = crate::replay::Replay::parse(&r.to_text());
        assert!(again.is_ok(), "a replay read but not read back: {again:?}");
        let _ = r.input_at(r.end);
    }
}

fn replay_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![
        b"drahtseilakt-replay 4\nlevel Level_01.NMO 0123456789abcdef\nstart 2 Ball_Stone 1,2.5,-3\n1 00\n5 0a\n90 ff\nend 120\n"
            .to_vec(),
        b"# c\ndrahtseilakt-replay 1\nlevel x 0\nstart 1 - -\nend 0\n".to_vec(),
    ];
    let mut roots: Vec<PathBuf> = store_hints();
    roots.extend(profile("replays"));
    out.extend(fuzzkit::read_all(&fuzzkit::files_under(
        &roots,
        &["replay"],
        40,
        256 * 1024,
    )));
    out
}

#[test]
#[ignore]
fn fuzz_replay() {
    let mut t = Target::new("replay", test_name!("fuzz_replay"), Kind::Text);
    t.dict = &["drahtseilakt-replay", "level", "start", "end", "-", ","];
    fuzzkit::fuzz(t, replay_seeds, run_replay);
}

fn run_hints(input: &[u8]) {
    let Ok(text) = std::str::from_utf8(input) else {
        return;
    };
    let _ = crate::hints::parse(text);
}

fn hint_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![
        b"at 1 2 3 r 4 sector 2 kind wood : goto 1 2 3 600 ; via 4 5 6 ; push 1 0 30 ; air 0 1 20 ; wait 5 ; turn -1 ; become stone ; replay Level_01-owner.replay 10 20\n"
            .to_vec(),
    ];
    let mut roots: Vec<PathBuf> = store_hints();
    roots.extend(profile("hints"));
    out.extend(fuzzkit::read_all(&fuzzkit::files_under(
        &roots,
        &["txt"],
        40,
        64 * 1024,
    )));
    out
}

fn run_zones(input: &[u8]) {
    let Ok(text) = std::str::from_utf8(input) else {
        return;
    };
    let (zones, _) = crate::zones::parse(text);
    // every zone written back reads the same
    for z in &zones.zones {
        let (again, notes) = crate::zones::parse(&z.line());
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(again.zones.len(), 1);
        let _ = zones.at(z.points[0], z.sector.saturating_sub(1));
    }
}

fn zone_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![
        b"flow sector 1 r 4 : 1 2 3 ; 4 5 6\ncareful sector 2 r 3.5 speed 4 : 0 0 0 # c\n".to_vec(),
    ];
    let mut roots: Vec<PathBuf> = store_hints();
    roots.extend(profile("hints"));
    out.extend(fuzzkit::read_all(&fuzzkit::files_under(
        &roots,
        &["zones"],
        40,
        64 * 1024,
    )));
    out
}

fn run_learned(input: &[u8]) {
    let Ok(text) = std::str::from_utf8(input) else {
        return;
    };
    let (learned, _) = crate::learned::parse(text);
    // every rule written back reads the same, and holds where its features are
    for r in &learned.rules {
        let (again, notes) = crate::learned::parse(&r.line());
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(again.rules.first(), Some(r));
        let mut s = crate::learned::Situation::default();
        for (f, v) in &r.when {
            s.values[*f] = Some(v.clone());
        }
        assert!(learned.at(&s).is_some());
    }
}

fn learned_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![
        b"flow when surface=rail ahead=rail-end kind=wood   # Level_01\ncareful speed 3 when surface=floor width=narrow curve=sharp module=P_Modul_25\n".to_vec(),
    ];
    let mut roots: Vec<PathBuf> = store_hints();
    roots.extend(profile("hints"));
    out.extend(
        fuzzkit::read_all(&fuzzkit::files_under(&roots, &["txt"], 40, 64 * 1024))
            .into_iter()
            .filter(|b| b.starts_with(b"# What the test player learned")),
    );
    out
}

#[test]
#[ignore]
fn fuzz_learned() {
    let mut t = Target::new("learned", test_name!("fuzz_learned"), Kind::Text);
    t.dict = &[
        "flow", "careful", "speed", "when", "surface=", "width=", "border=", "curve=", "slope=",
        "ahead=", "module=", "kind=", "rail", "floor", "rail-end", "narrow", "#",
    ];
    fuzzkit::fuzz(t, learned_seeds, run_learned);
}

#[test]
#[ignore]
fn fuzz_zones() {
    let mut t = Target::new("zones", test_name!("fuzz_zones"), Kind::Text);
    t.dict = &["flow", "careful", "sector", "r", "speed", ":", ";", "#"];
    fuzzkit::fuzz(t, zone_seeds, run_zones);
}

#[test]
#[ignore]
fn fuzz_hints() {
    let mut t = Target::new("hints", test_name!("fuzz_hints"), Kind::Text);
    t.dict = &[
        "at", "r", "sector", "kind", "wood", "stone", "paper", ":", ";", "goto", "via", "push",
        "air", "wait", "turn", "swing", "become", "replay", "#",
    ];
    fuzzkit::fuzz(t, hint_seeds, run_hints);
}

// --- sounds ------------------------------------------------------------------------------------------------------

fn run_wav(input: &[u8]) {
    let _ = crate::audio::parse_wav(input);
}

fn wav(channels: u16, bits: u16, data: &[u8]) -> Vec<u8> {
    let mut w = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&22050u32.to_le_bytes());
    w.extend_from_slice(&(22050 * u32::from(channels) * u32::from(bits / 8)).to_le_bytes());
    w.extend_from_slice(&(channels * bits / 8).to_le_bytes());
    w.extend_from_slice(&bits.to_le_bytes());
    w.extend_from_slice(b"LIST\x04\0\0\0abcd");
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data.len() as u32).to_le_bytes());
    w.extend_from_slice(data);
    let len = (w.len() - 8) as u32;
    w[4..8].copy_from_slice(&len.to_le_bytes());
    w
}

fn wav_seeds() -> Vec<Vec<u8>> {
    let data: Vec<u8> = (0..64u8).collect();
    let mut out = vec![wav(1, 8, &data), wav(2, 16, &data), wav(1, 16, &data[..63])];
    let mut roots = fuzzkit::real_roots();
    roots.push(repo().join("assets/sounds"));
    out.extend(fuzzkit::read_all(&fuzzkit::files_under(
        &roots,
        &["wav"],
        24,
        256 * 1024,
    )));
    out
}

#[test]
#[ignore]
fn fuzz_wav() {
    let t = Target::new("wav", test_name!("fuzz_wav"), Kind::Binary);
    fuzzkit::fuzz(t, wav_seeds, run_wav);
}

fn run_ogg(input: &[u8]) {
    let _ = crate::audio::parse_ogg(input);
}

fn ogg_seeds() -> Vec<Vec<u8>> {
    let mut roots = vec![repo().join("assets/sounds")];
    roots.extend(fuzzkit::real_roots());
    fuzzkit::read_all(&fuzzkit::files_under(&roots, &["ogg"], 16, 160 * 1024))
}

#[test]
#[ignore]
fn fuzz_ogg() {
    let mut t = Target::new("ogg", test_name!("fuzz_ogg"), Kind::Binary);
    // ten minutes of stereo at 48 kHz decode to 230 MB of frames, which the cap allows
    t.rss_mb = 900;
    fuzzkit::fuzz(t, ogg_seeds, run_ogg);
}

// --- the player's own files --------------------------------------------------------------------------------------

fn run_highscore(input: &[u8]) {
    let t = crate::highscore::Table::parse(input);
    for level in 1..=70u32 {
        let list = t.list(level);
        assert!(list.len() <= crate::lang::PLACES);
        assert!(
            list.iter()
                .all(|(name, _)| name.len() <= crate::highscore::NAME_MAX)
        );
    }
}

fn highscore_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![b"1 100 Mr. Default\n1 90 b\r\n13 5 zz\n".to_vec()];
    if std::env::var_os("FUZZ_NO_REAL").is_none()
        && let Some(home) = std::env::var_os("HOME")
        && let Ok(b) =
            std::fs::read(PathBuf::from(home).join(".local/share/drahtseilakt/highscores.txt"))
    {
        out.push(b);
    }
    out
}

#[test]
#[ignore]
fn fuzz_highscore() {
    let t = Target::new("highscore", test_name!("fuzz_highscore"), Kind::Text);
    fuzzkit::fuzz(t, highscore_seeds, run_highscore);
}

fn run_settings(input: &[u8]) {
    let text = gamedata::profile::settings_from_bytes(input);
    let get = |k: &str| gamedata::profile::setting_in(&text, k);
    let g = crate::options::Graphics::load(get);
    if let Some((w, h)) = g.resolution {
        assert!(w <= crate::options::SURFACE_MAX && h <= crate::options::SURFACE_MAX);
    }
    let _ = crate::options::Bindings::load(get, [0, 1, 2, 3, 4, 5], false);
    let _ = crate::options::Bindings::load(get, [usize::MAX, 7, 7, 1000, 2, 3], true);
    let _ = crate::features::Features::load(true, false, get);
    let _ = crate::lang::chosen(get("language").as_deref());
    let _ = crate::lang::docs_language(get("language").as_deref());
    let _ = get("interface").and_then(|s| gamedata::resources::Provider::parse(&s));
    let _ = get("sound-theme").map(|s| gamedata::sound_themes::plain(&s));
    let _ = get("player").map(|n| crate::lang::latin1(n.as_bytes()));
    for k in ["sound", "music"] {
        let _ = get(k)
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .map(|v| v.clamp(0.0, 1.0));
    }
}

fn settings_seeds() -> Vec<Vec<u8>> {
    let mut out = vec![
        b"sound 0.5\nmusic 1\nprofile extended\ninterface own\nsound-theme neuro\nresolution 1920x1080\nfullscreen borderless\nvsync off\nkey.forward W\ninvert-rotation on\nextended.shadows off\nlanguage ru\nplayer Name\n"
            .to_vec(),
    ];
    if std::env::var_os("FUZZ_NO_REAL").is_none()
        && let Some(home) = std::env::var_os("HOME")
        && let Ok(b) =
            std::fs::read(PathBuf::from(home).join(".local/share/drahtseilakt/settings.txt"))
    {
        out.push(b);
    }
    out
}

#[test]
#[ignore]
fn fuzz_settings() {
    let mut t = Target::new("settings", test_name!("fuzz_settings"), Kind::Text);
    t.dict = &[
        "resolution",
        "fullscreen",
        "exclusive",
        "borderless",
        "vsync",
        "on",
        "off",
        "key.forward",
        "key.left",
        "invert-rotation",
        "extended.",
        "profile",
        "extended",
        "classic",
        "interface",
        "copy",
        "remastered",
        "own",
        "language",
        "player",
        "sound",
        "music",
        "x",
        "*",
    ];
    fuzzkit::fuzz(t, settings_seeds, run_settings);
}

// --- a level played ----------------------------------------------------------------------------------------------

/// A level from an input: "Nemo" first an .nmo file, anything else the open format (level.gltf, then level.bin).
fn level_of(input: &[u8]) -> Option<level::Level> {
    if input.starts_with(b"Nemo") {
        level::from_nmo::load(input).ok().map(|l| l.level)
    } else {
        let parts = fuzzkit::split_parts(input, 2);
        level::open::import_bytes(&fuzzkit::scratch(), parts[0], parts[1]).ok()
    }
}

fn run_level_sim(input: &[u8]) {
    let Some(level) = level_of(input) else { return };
    // half the inputs with the modules of the player's copy, when there is one (its moving parts placed by the
    // fuzzed level's objects), half with none
    static DATA: std::sync::OnceLock<Option<crate::assets::Data>> = std::sync::OnceLock::new();
    let data = DATA.get_or_init(|| {
        (std::env::var_os("FUZZ_NO_REAL").is_none())
            .then(|| crate::assets::Data::locate(None).ok())
            .flatten()
    });
    let mut sim = match data {
        Some(d) if fuzzkit::fnv(input) & 1 == 0 => {
            let modules = crate::scene::load_modules(&level, d);
            crate::sim::Sim::with_modules(&level, crate::sim::BallKind::Wood, &modules, false)
        }
        _ => crate::sim::Sim::new(&level, crate::sim::BallKind::Wood),
    };
    // a second of play, the arrows and the camera keys taken from the input's last bytes
    for k in 0..60usize {
        let bits = input
            .get(input.len().saturating_sub(1 + k % 16))
            .copied()
            .unwrap_or(0);
        sim.step(bits);
    }
}

fn level_sim_seeds() -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let d = fuzzkit::scratch().join("seed");
    let _ = std::fs::create_dir_all(&d);
    if level::open::export(&crate::sim::tests::flat_level(), &d).is_ok()
        && let (Ok(g), Ok(b)) = (
            std::fs::read(d.join("level.gltf")),
            std::fs::read(d.join("level.bin")),
        )
    {
        out.push(fuzzkit::join_parts(&[&g, &b]));
    }
    // the player's converted levels and maps, small ones
    let mut roots = profile("levels/open");
    roots.extend(fuzzkit::real_roots());
    for p in fuzzkit::files_under(&roots, &["gltf", "nmo"], 16, 2 << 20) {
        if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("nmo")) {
            // written plain: the mutations then reach the objects instead of the compressed stream
            if let Ok(b) = std::fs::read(&p)
                && let Ok(f) = nmo::read(&b)
            {
                out.push(plain_nmo(&f));
            }
        } else if let Some(dir) = p.parent()
            && std::fs::metadata(dir.join("level.bin")).is_ok_and(|m| m.len() <= 4 << 20)
            && let (Ok(g), Ok(b)) = (std::fs::read(&p), std::fs::read(dir.join("level.bin")))
        {
            out.push(fuzzkit::join_parts(&[&g, &b]));
        }
    }
    out
}

/// A file's parts as a plain (uncompressed) file.
fn plain_nmo(f: &nmo::File) -> Vec<u8> {
    let h = &f.header;
    let (h1, data) = (&f.header1, &f.data);
    let fields = [
        0,
        h.ck_version,
        h.file_version,
        0,
        0,
        h1.len() as u32,
        data.len() as u32,
        data.len() as u32,
        h.manager_count,
        h.object_count,
        h.max_id_saved,
        h.product_version,
        h.product_build,
        h1.len() as u32,
    ];
    let mut out = b"Nemo Fi\0".to_vec();
    fields
        .iter()
        .for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
    out.extend_from_slice(h1);
    out.extend_from_slice(data);
    out
}

#[test]
#[ignore]
fn fuzz_level_sim() {
    // text mutations too: the open format's names and numbers (a group "Sector_1000000000", a far position)
    let mut t = Target::new("level_sim", test_name!("fuzz_level_sim"), Kind::Text);
    t.dict = &[
        "Sector_",
        "Sector_01",
        "PC_Checkpoints",
        "PC_TwoFlames_01",
        "PS_Levelstart",
        "PS_FourFlames_01",
        "PR_Resetpoints",
        "PR_Resetpoint_01",
        "PE_Levelende",
        "P_Box",
        "P_Box_01",
        "P_Dome",
        "P_Extra_Point",
        "P_Trafo_Stone",
        "P_Ball_Paper",
        "Phys_Floors",
        "Phys_FloorRails",
        "Phys_FloorStopper",
        "P_Modul_01",
        "\"members\"",
        "\"matrix\"",
        "\"groups\"",
        "\"name\"",
        "1e30",
        "-1e30",
        "3.4e38",
    ];
    t.max_len = 16 << 20;
    t.rss_mb = 1024;
    fuzzkit::fuzz(t, level_sim_seeds, run_level_sim);
}

/// The player's copy, and some of its levels with the modules they use (none without a copy).
#[allow(clippy::type_complexity)]
fn copy_levels() -> &'static Option<(
    crate::assets::Data,
    Vec<(level::Level, crate::scene::Modules)>,
)> {
    static COPY: std::sync::OnceLock<
        Option<(
            crate::assets::Data,
            Vec<(level::Level, crate::scene::Modules)>,
        )>,
    > = std::sync::OnceLock::new();
    COPY.get_or_init(|| {
        if std::env::var_os("FUZZ_NO_REAL").is_some() {
            return None;
        }
        let data = crate::assets::Data::locate(None).ok()?;
        let levels: Vec<_> = [1, 4, 8, 12]
            .iter()
            .filter_map(|&n| data.load(&data.original(n)?).ok())
            .map(|l| {
                let modules = crate::scene::load_modules(&l.level, &data);
                (l.level, modules)
            })
            .collect();
        Some((data, levels))
    })
}

/// A module file (a stranger's level folder may bring its own, game/3D Entities/PH/<name>.nmo, or one in the open
/// format) in place of one an original level uses, and the level played for a second: the first byte picks the
/// level, the second the module, the rest is the module's file.
fn run_module_sim(input: &[u8]) {
    let Some((_, levels)) = copy_levels() else {
        return;
    };
    let (Some(&a), Some(&b)) = (input.first(), input.get(1)) else {
        return;
    };
    let Some((level, modules)) = levels.get(usize::from(a) % levels.len().max(1)) else {
        return;
    };
    let Some(model) = input.get(2..).and_then(level_of) else {
        return;
    };
    let mut modules = modules.clone();
    if modules.models.is_empty() {
        return;
    }
    let k = usize::from(b) % modules.models.len();
    modules.models[k] = model;
    let mut sim = crate::sim::Sim::with_modules(level, crate::sim::BallKind::Wood, &modules, false);
    for k in 0..60usize {
        sim.step(
            input
                .get(input.len().saturating_sub(1 + k % 16))
                .copied()
                .unwrap_or(0),
        );
    }
}

fn module_sim_seeds() -> Vec<Vec<u8>> {
    let Some((data, levels)) = copy_levels() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (a, (_, modules)) in levels.iter().enumerate() {
        for (b, name) in modules.names.iter().enumerate().take(12) {
            let Some(path) = data.file(&format!("3D Entities/PH/{name}.nmo")) else {
                continue;
            };
            if let Ok(bytes) = std::fs::read(&path)
                && let Ok(f) = nmo::read(&bytes)
            {
                let mut s = vec![a as u8, b as u8];
                s.extend(plain_nmo(&f));
                out.push(s);
            }
        }
    }
    out
}

#[test]
#[ignore]
fn fuzz_module_sim() {
    let mut t = Target::new("module_sim", test_name!("fuzz_module_sim"), Kind::Binary);
    t.max_len = 8 << 20;
    t.rss_mb = 1024;
    fuzzkit::fuzz(t, module_sim_seeds, run_module_sim);
}

// --- committed repros --------------------------------------------------------------------------------------------

#[test]
fn fuzz_regressions() {
    fuzzkit::regressions("replay", run_replay);
    fuzzkit::regressions("hints", run_hints);
    fuzzkit::regressions("zones", run_zones);
    fuzzkit::regressions("learned", run_learned);
    fuzzkit::regressions("wav", run_wav);
    fuzzkit::regressions("ogg", run_ogg);
    fuzzkit::regressions("highscore", run_highscore);
    fuzzkit::regressions("settings", run_settings);
    fuzzkit::regressions("level_sim", run_level_sim);
    fuzzkit::regressions("module_sim", run_module_sim);
}
