// Drahtseilakt: the game. Loads a level (its own, from the player's copy of the original, or any .nmo map), plays
// it in a window, records and replays runs, and can run without a window (replay checks, screenshots).

// A release build on Windows is a windowed program: no console window opens beside the game. Its messages then go to
// a log file (log_without_console); a debug build stays a console program.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
// Hostile input is parsed only by the `nmo` crate; this crate works on validated models, and geometry code
// reads better with plain arithmetic and indexing of fixed-size arrays.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

mod app;
mod audit;
use gamedata::assets;
mod audio;
mod autopilot;
mod ballsound;
mod clouds;
mod collide;
mod edges;
mod endscreen;
mod extra;
mod fallback;
mod features;
mod freecam;
#[cfg(test)]
mod fuzz_tests;
mod highscore;
mod hints;
mod hud;
mod lang;
mod learned;
mod levels;
mod levelstats;
mod look;
mod lights;
mod locate;
mod menu;
mod menuscene;
mod modified;
mod modrt;
mod modules;
mod music;
mod options;
mod ownlevels;
mod partcheck;
mod particles;
mod physics;
mod pieces;
mod playtime;
mod proximity;
mod render;
mod retro;
mod replay;
mod scene;
mod scripting;
mod setup;
mod sim;
mod sky;
mod supports;
mod trafo;
mod ui;
mod volume;
mod watch;
mod zones;

use assets::{Data, Source};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
usage: drahtseilakt [options]

  --data DIR          the Ballance folder (default: $OPENBALLANCE_DATA, then the imported copy, then ./data)
  --import DIR        copy the original game into the levels folder from your installed Ballance in DIR, or
                      from your game CD's image (a .cue, .bin or .iso)
  --level N           play original level N (1-13; default 1)
  --map FILE.nmo      play a level file: a community map or any .nmo
  --open DIR          play a level in the open format (a folder with level.gltf)
  --export DIR        convert the chosen level to the open format in DIR (textures as PNG) and exit
  --export-parts WHAT with --export: keep (the copy's modules, balls, sky, particles and sounds in DIR/game, so the
                      level plays without the copy; the default) or none (nothing of the copy: the level uses the
                      game's own parts and sounds; the default with --textures own)
  --textures PACK    the texture pack the levels are drawn, or exported, with: own (our free textures; the default
                      for playing), original (the copy's; the default for --export), remastered, or a pack of the
                      data folder's textures/ (docs/drahtseilakt/packs.md)
  --models PACK       the model pack the parts and balls are drawn with: level (as the level takes them; the
                      default), own, original, or a pack of the data folder's models/ (the physics never changes)
  --record FILE       write a replay of the run (also written automatically on finishing a level)
  --replay FILE       play a replay instead of the keyboard
  --headless          no window: run the replay to its end and print what happened
  --autopilot         no window: the test player plays the level (to --ticks); saved with --record
  --watch             with --autopilot: the test player plays in the window (Space pause, +/- speed, F5 take
                      over by hand and give back, F6 skip a hint's step, F7 a stuck report)
  --follow FILE       the test player follows the ball's way in a replay of a player's run, or a --locate track
  --then FILE         with --follow: from the sector this replay begins at, its way instead (up to 8 times)
  --plan FILE         the test player's way goes through the level objects named in FILE, in order
  --hints FILE        the test player's hints (default: hints/<level>.txt in the data folder; autopilot.md)
  --rewinds N         the test player goes back to a saved state at most N times (lost or stuck; 0: never)
  --report DIR        where the test player writes a report each time it is stuck (default: the temp folder)
  --locate DIR        find the ball's way in a video's frames (DIR/s0001.jpg, four a second); track to --record
  --frames DIR        with --replay: draw four frames a second into DIR, with the ball's true places
  --video FILE        with --replay: raw RGB frames, 60 a second at --size, into FILE (scripts/video.sh)
  --screenshot FILE   no window: render the frame after the replay (or --ticks N) to a PNG
  --ticks N           with --screenshot: how many ticks to simulate first (default: the replay's end, or 1)
  --trace N           with --headless: print the ball's position every N ticks
  --from N            with --replay: play the replay up to tick N without a window, then hand over the keys
                      (with --autopilot: to the test player)
  --sector N          begin at sector N's reset point with the ball that sector begins with (testing a place;
                      1 is the level start; --ball overrides the ball)
  --at X,Y,Z|OBJECT   begin with the ball at rest at a point, or 4 above a level object (e.g. P_Modul_34_01);
                      the sector is the nearest object's unless --sector is given
  --ball KIND         begin with a wood, stone, paper or sticky ball
                      (a run begun elsewhere is recorded with its start and is never a level's best run)
  --no-debug          a plain run (until the release a played game is a debug run by default)
  --debug             debug run: the extended mode, all levels open, and the development mod god-mode
                      (assets/mods-dev; switch it on under Options > Extended > Mods): endless lives, F5 / F9 save
                      and load the moment, F3 seconds back, R restart, [ ] slower and faster, F2 the free camera
                      (arrows fly and turn, A/D sideways, PageUp/PageDown up and down, Home/End look, Shift faster)
  --sky NAME          play under another sky (A, B, C, Mountains, Forest, Sea, Storm, Sunset, Night, Desert,
                      Mist, Clouds, or any Sky_<NAME>_* the textures have) instead of the level's own
  --size WxH          window or screenshot size (default 1600x900)
  --audit FILE|DIR    no window: audit a map or a folder of maps, with the copy and without it: bodies that collide
                      without being drawn (invisible blockers), stand-ins without a module, missing textures,
                      objects and modes not read; prints a summary by the number of maps hit (may be repeated)
  --show-colliders    draw the floors and rails a map hides (red) and its hidden stoppers (yellow) as translucent
                      shapes: what stops the ball without being seen
  --audit-csv FILE    with --audit: write one row per map, copy and kind of issue (with the names) to FILE
  --supports-check geometry|runs
                      where the swinging sacks get a support (the sack-supports mod) in the original levels and the
                      shipped maps, each placed one measured again; with runs also against the ball's way in the
                      recorded runs of ./hints. Fails when one is too near the ball's space
  --level-stats DIR   measure the original levels (or --level N alone) along the finished runs and plans in ./hints
                      for designing our own: DIR/level-stats-auto.csv and the way's samples (scripts/level-stats.py);
                      with --open LEVEL --replay RUN one of our levels along that run: DIR/level-stats-own.csv
  --look X,Y,Z,TX,TY,TZ  with --screenshot: camera at X,Y,Z looking at TX,TY,TZ instead of the game camera

Without --level, --map or --open the window opens with the level menu.

Without the game found, the window offers to bring in your Ballance: from an installed game, the CD in
its drive or an image of it (found on this computer, dropped on the window or typed).

keys: arrows roll the ball (relative to the camera), Shift+Left/Right turn the camera,
      Space lifts the camera, F11 full screen, Esc pauses; Options > Controls binds other keys; a gamepad's
      left stick rolls, its shoulder buttons turn the camera, A lifts it, Start pauses; the mods' keys are
      on each mod's page (god-mode's in a debug run)";

#[derive(Clone)]
pub struct Options {
    pub data: Option<PathBuf>,
    /// Copy the original game from this installed copy into the levels folder, then stop.
    pub import: Option<PathBuf>,
    pub source: Source,
    /// A level was named (--level, --map, --open): no menu at the start.
    pub chosen: bool,
    pub record: Option<PathBuf>,
    pub export: Option<PathBuf>,
    /// With `--export`: whether the copy's game files go into the export (`--export-parts`); None: by the pack.
    pub export_parts: Option<bool>,
    /// The texture pack the levels are drawn (and exported) with, instead of the settings' (packs.rs).
    pub textures: Option<String>,
    /// The model pack the parts are drawn with, instead of the settings'.
    pub models: Option<String>,
    pub replay: Option<PathBuf>,
    pub headless: bool,
    /// Play the level by the test player (autopilot.rs), headless; the run is saved like a recording.
    pub autopilot: bool,
    /// The test player plays in the window, for watching it and taking over by hand.
    pub watch: bool,
    /// The test player follows the ball's way in this replay (a player's run) or `--locate` track instead of finding
    /// one.
    pub follow: Option<PathBuf>,
    /// Runs begun at a sector: the followed way is theirs from that sector on (in order).
    pub then: Vec<PathBuf>,
    /// The test player's way goes through these level objects in order (a file of names).
    pub plan: Option<PathBuf>,
    /// The test player's hint file, instead of the level's in the data folder.
    pub hints: Option<PathBuf>,
    /// Where the test player's stuck reports go.
    pub report: Option<PathBuf>,
    /// The test player gives up when its ball has not moved for this many seconds (or its way has not got on for
    /// four times as long): the run is dead, its report is written and it stops.
    pub give_up: u64,
    /// How often the test player may go back to a saved state when its ball is lost or its way stands still.
    pub rewinds: u32,
    /// Find the ball's way in the frames (s0001.jpg, `locate::RATE` a second) of a video of the original; the track is
    /// written to --record.
    pub locate: Option<PathBuf>,
    /// Draw frames of the replay (`locate::RATE` a second) into this folder, with the ball's true places (to check
    /// --locate).
    pub frames: Option<PathBuf>,
    /// Record the replay as a video: raw RGB frames, one a tick at --size, into this file (or pipe).
    pub video: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub ticks: Option<u64>,
    pub trace: Option<u64>,
    pub from: Option<u64>,
    pub size: (u32, u32),
    /// The size was given (--size): the settings' resolution does not replace it.
    pub size_given: bool,
    /// Debug mode from the start: the free camera on F2.
    pub debug: bool,
    /// --sector, --at, --ball: a run begun elsewhere than the level start.
    pub sector: Option<usize>,
    pub at: Option<String>,
    pub ball: Option<String>,
    pub look: Option<[f32; 6]>,
    /// Measure the original levels into this folder (levelstats.rs) and stop.
    pub level_stats: Option<PathBuf>,
    /// Measure the sacks' supports of every original level and shipped map, then stop.
    pub supports_check: Option<bool>,
    /// No window: audit these maps (files or folders of them) for what is not drawn or not there (audit.rs).
    pub audit: Vec<PathBuf>,
    pub audit_csv: Option<PathBuf>,
    /// Draw the floors, rails and stoppers a map hides as translucent markers.
    pub show_colliders: bool,
}

/// Until the release, a played game is a debug run unless `--no-debug` says otherwise (the owner's decision,
/// 2026-11-08: easier to start). Runs without a player (replays, headless, autopilot, video, screenshots) stay
/// plain unless `--debug` is given, so recorded runs and tests are unchanged. The release cleanup sets this false.
const DEBUG_BY_DEFAULT: bool = true;

fn parse_args() -> Result<Options, String> {
    let mut debug: Option<bool> = None;
    let mut o = Options {
        data: None,
        import: None,
        source: Source::Original(1),
        chosen: false,
        record: None,
        export: None,
        export_parts: None,
        textures: None,
        models: None,
        replay: None,
        headless: false,
        autopilot: false,
        watch: false,
        follow: None,
        then: Vec::new(),
        plan: None,
        hints: None,
        report: None,
        give_up: 30,
        rewinds: 600,
        locate: None,
        frames: None,
        video: None,
        screenshot: None,
        ticks: None,
        trace: None,
        from: None,
        size: (1600, 900),
        size_given: false,
        sector: None,
        at: None,
        ball: None,
        debug: false,
        look: None,
        level_stats: None,
        supports_check: None,
        audit: Vec::new(),
        audit_csv: None,
        show_colliders: false,
    };
    // `--name=value` is the same as `--name value` (handy for values that start with a minus).
    let mut args = std::env::args()
        .skip(1)
        .flat_map(|a| match a.split_once('=') {
            Some((k, v)) if k.starts_with("--") => vec![k.to_owned(), v.to_owned()],
            _ => vec![a],
        });
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--data" => o.data = Some(value()?.into()),
            "--import" => o.import = Some(value()?.into()),
            "--level" => {
                let n: u32 = value()?
                    .parse()
                    .map_err(|_| "--level needs a number".to_owned())?;
                if !(1..=13).contains(&n) {
                    return Err("--level is 1 to 13".into());
                }
                o.source = Source::Original(n);
                o.chosen = true;
            }
            "--map" => {
                o.source = Source::Nmo(value()?.into());
                o.chosen = true;
            }
            "--open" => {
                // known by its full path (its own files are, and a remaster by where it lies)
                let dir = PathBuf::from(value()?);
                o.source = Source::Open(
                    level::open::full_path(&dir).map_err(|e| format!("{}: {e}", dir.display()))?,
                );
                o.chosen = true;
            }
            "--export" => o.export = Some(value()?.into()),
            "--export-parts" => {
                o.export_parts = match value()?.as_str() {
                    "keep" => Some(true),
                    "none" => Some(false),
                    v => return Err(format!("--export-parts {v}: keep or none")),
                }
            }
            // a pack's key: checked against the packs found when the game starts (an unknown one is refused there)
            "--textures" | "--models" => {
                let v = value()?;
                if !gamedata::packs::plain(&v) {
                    return Err(format!("{a}: not a pack's name: {v}"));
                }
                if a == "--textures" {
                    o.textures = Some(v);
                } else {
                    o.models = Some(v);
                }
            }
            "--record" => o.record = Some(value()?.into()),
            "--replay" => o.replay = Some(value()?.into()),
            "--headless" => o.headless = true,
            "--autopilot" => o.autopilot = true,
            "--watch" => {
                o.autopilot = true;
                o.watch = true;
            }
            "--locate" => o.locate = Some(value()?.into()),
            "--frames" => o.frames = Some(value()?.into()),
            "--video" => o.video = Some(value()?.into()),
            "--plan" => {
                o.plan = Some(value()?.into());
                o.autopilot = true;
            }
            "--hints" => o.hints = Some(value()?.into()),
            "--report" => o.report = Some(value()?.into()),
            "--give-up" => {
                let s: u64 = value()?
                    .parse()
                    .map_err(|_| "--give-up needs seconds".to_owned())?;
                o.give_up = s.clamp(5, 3600);
            }
            "--rewinds" => {
                let n: u32 = value()?
                    .parse()
                    .map_err(|_| "--rewinds needs a number".to_owned())?;
                o.rewinds = n.min(100_000);
            }
            "--then" => {
                if o.then.len() >= MAX_THEN {
                    return Err(format!("more than {MAX_THEN} --then runs"));
                }
                o.then.push(value()?.into());
            }
            "--follow" => {
                o.follow = Some(value()?.into());
                o.autopilot = true;
            }
            "--debug" => debug = Some(true),
            "--no-debug" => debug = Some(false),
            "--show-colliders" => o.show_colliders = true,
            "--audit" => o.audit.push(value()?.into()),
            "--audit-csv" => o.audit_csv = Some(value()?.into()),
            "--level-stats" => o.level_stats = Some(value()?.into()),
            "--supports-check" => {
                o.supports_check = Some(match value()?.as_str() {
                    "geometry" => false,
                    "runs" => true,
                    _ => return Err("--supports-check geometry|runs".into()),
                })
            }
            "--screenshot" => o.screenshot = Some(value()?.into()),
            "--sky" => {
                let name = value()?;
                if !sky::valid_name(&name) {
                    return Err(format!(
                        "--sky {name}: a sky's name is letters, digits, _ and - (e.g. Mountains)"
                    ));
                }
                sky::set_override(name);
            }
            "--ticks" => {
                o.ticks = Some(
                    value()?
                        .parse()
                        .map_err(|_| "--ticks needs a number".to_owned())?,
                )
            }
            "--from" => {
                o.from = Some(
                    value()?
                        .parse()
                        .map_err(|_| "--from needs a tick number".to_owned())?,
                )
            }
            "--trace" => {
                o.trace = Some(
                    value()?
                        .parse()
                        .map_err(|_| "--trace needs a number".to_owned())?,
                )
            }
            "--sector" => {
                let n: usize = value()?
                    .parse()
                    .map_err(|_| "--sector needs a number".to_owned())?;
                if n == 0 {
                    return Err("--sector starts at 1".into());
                }
                o.sector = Some(n);
            }
            "--at" => o.at = Some(value()?),
            "--ball" => {
                let v = value()?.to_ascii_lowercase();
                if !["wood", "stone", "paper", "sticky"].contains(&v.as_str()) {
                    return Err("--ball is wood, stone, paper or sticky".into());
                }
                o.ball = Some(v);
            }
            "--look" => {
                let v: Vec<f32> = value()?
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                let a: [f32; 6] = v
                    .try_into()
                    .map_err(|_| "--look needs six numbers".to_owned())?;
                o.look = Some(a);
            }
            "--size" => {
                o.size_given = true;
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size is WxH")?;
                o.size = (
                    w.parse().map_err(|_| "bad width")?,
                    h.parse().map_err(|_| "bad height")?,
                );
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option {a}\n\n{USAGE}")),
        }
    }
    let unattended = o.replay.is_some()
        || o.headless
        || o.autopilot
        || o.video.is_some()
        || o.frames.is_some()
        || o.screenshot.is_some()
        || !o.audit.is_empty()
        || o.export.is_some();
    o.debug = debug.unwrap_or(DEBUG_BY_DEFAULT && !unattended);
    Ok(o)
}

/// How many --then runs may follow on.
const MAX_THEN: usize = 8;

/// Set in the process that opens the import window for the menu's Import.
pub const IMPORT: &str = "DRAHTSEILAKT_IMPORT";

/// Starts this program again with the same arguments, waits for it and ends with its exit code.
fn restart() -> ! {
    let status = std::env::current_exe().and_then(|exe| {
        std::process::Command::new(exe)
            .args(std::env::args_os().skip(1))
            .env_remove(IMPORT)
            .status()
    });
    match status {
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("drahtseilakt: could not start the game again: {e}");
            std::process::exit(1)
        }
    }
}

/// Where the game found its shipped assets and keeps the player's data (and whether the copy is portable).
fn paths_line() -> String {
    let shown =
        |p: Option<std::path::PathBuf>| p.map_or("(none)".into(), |p| p.display().to_string());
    format!(
        "paths: assets {} | data {} ({})",
        shown(gamedata::resources::assets_root()),
        gamedata::profile::data_dir().display(),
        if gamedata::profile::portable_root().is_some() {
            "portable"
        } else {
            "per-user"
        }
    )
}

/// Set in the game started again with its output going to the log file.
#[cfg(windows)]
const LOGGED: &str = "DRAHTSEILAKT_LOGGED";

/// A windowed program started from Explorer (or from a console without redirection) has no output at all: it starts
/// itself again with its output and errors going to `drahtseilakt.log` in the data folder, waits for that one and ends
/// with its exit code. With output to a file or a pipe (`2> log.txt`, `| more`) it prints there as usual. If the log
/// cannot be made the game plays on without one.
#[cfg(windows)]
fn log_without_console() {
    use std::os::windows::io::AsRawHandle;
    // null: the parent gave no handle (std's documented value for a missing standard handle)
    if !std::io::stderr().as_raw_handle().is_null() || std::env::var_os(LOGGED).is_some() {
        return;
    }
    let dir = gamedata::profile::data_dir();
    // the last start's log is kept beside (a crash's log survives the next start)
    let log = dir.join("drahtseilakt.log");
    let _ = std::fs::rename(&log, dir.join("drahtseilakt.old.log"));
    let Ok(out) = std::fs::create_dir_all(&dir).and_then(|()| std::fs::File::create(&log))
    else {
        return;
    };
    let (Ok(err), Ok(exe)) = (out.try_clone(), std::env::current_exe()) else {
        return;
    };
    let status = std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .env(LOGGED, "1")
        .stdin(std::process::Stdio::null())
        .stdout(out)
        .stderr(err)
        .status();
    if let Ok(s) = status {
        std::process::exit(s.code().unwrap_or(1));
    }
}

fn main() -> ExitCode {
    #[cfg(windows)]
    log_without_console();
    let result = parse_args().and_then(|o| {
        println!("{}", paths_line());
        if let Some(from) = &o.import {
            let copied = levels::import(from)?;
            levels::prepare(&lang::docs_language(app::setting("language").as_deref()));
            println!("{copied} files copied to {}", levels::dir(levels::Kind::Original).display());
            return Ok(());
        }
        // the menu's Import: the window that brings in the player's copy, in a process of its own (winit runs one
        // event loop a process), then the game again, with the copy or without it
        if std::env::var_os(IMPORT).is_some() {
            match setup::run(o.size) {
                // the close button ends the program: no game window follows
                Ok(setup::Outcome::Quit) => return Ok(()),
                Ok(_) => {}
                Err(w) => eprintln!("drahtseilakt: the window to bring in the game did not open: {w}"),
            }
            restart();
        }
        // the copy of the original is optional: without it the game plays its own levels (decisions.md, "The game
        // plays out of the box"); a folder named with --data must itself be a copy (no other place is tried)
        let data = match o.data.clone() {
            Some(d) => Data::at(d)?,
            None => Data::find(None),
        };
        let mut o = o;
        if !data.has_copy() && !o.chosen {
            // the level behind the menu at the start: our first own level
            let first = levels::shipped().into_iter().next().ok_or(
                "no copy of Ballance and no levels of the game's own (assets/levels): import your Ballance with \
                 --import DIR, or pass --data DIR",
            )?;
            // the shipped levels are open-format folders or community maps' .nmo files (the owner's picks)
            o.source = if first.level.is_dir() {
                Source::Open(first.level)
            } else {
                Source::Nmo(first.level)
            };
        }
        app::run(o, data)
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("drahtseilakt: {e}");
            ExitCode::FAILURE
        }
    }
}
