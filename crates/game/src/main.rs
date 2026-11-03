// Drahtseilakt: the game. Loads a level (its own, from the player's copy of the original, or any .nmo map), plays
// it in a window, records and replays runs, and can run without a window (replay checks, screenshots).

// Hostile input is parsed only by the `nmo` crate; this crate works on validated models, and geometry code
// reads better with plain arithmetic and indexing of fixed-size arrays.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

mod app;
use gamedata::assets;
mod audio;
mod autopilot;
mod ballsound;
mod collide;
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
mod levels;
mod lights;
mod locate;
mod menu;
mod menuscene;
mod modules;
mod music;
mod options;
mod ownlevels;
mod particles;
mod partcheck;
mod physics;
mod pieces;
mod proximity;
mod render;
mod replay;
mod scene;
mod setup;
mod sim;
mod sky;
mod trafo;
mod ui;
mod volume;
mod watch;

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
  --textures SET      own (our free textures; the default for playing) or original (the copy's; the default for
                      --export): the textures the levels are drawn, or exported, with
  --record FILE       write a replay of the run (also written automatically on finishing a level)
  --replay FILE       play a replay instead of the keyboard
  --headless          no window: run the replay to its end and print what happened
  --autopilot         no window: the test player plays the level (to --ticks); saved with --record
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
  --sector N          begin at sector N's reset point (testing a place; 1 is the level start)
  --at X,Y,Z|OBJECT   begin with the ball at rest at a point, or 4 above a level object (e.g. P_Modul_34_01);
                      the sector is the nearest object's unless --sector is given
  --ball KIND         begin with a wood, stone or paper ball
                      (a run begun elsewhere is recorded with its start and is never a level's best run)
  --debug             debug mode: F2 takes up the free camera (the run waits; arrows fly and turn, A/D move
                      sideways, PageUp/PageDown up and down, Home/End look up and down, Shift faster)
  --size WxH          window or screenshot size (default 1600x900)
  --look X,Y,Z,TX,TY,TZ  with --screenshot: camera at X,Y,Z looking at TX,TY,TZ instead of the game camera

Without --level, --map or --open the window opens with the level menu.

Without the game found, the window offers to bring in your Ballance: from an installed game, the CD in
its drive or an image of it (found on this computer, dropped on the window or typed).

keys: arrows roll the ball (relative to the camera), Shift+Left/Right turn the camera,
      Space lifts the camera, F11 full screen, Esc pauses; Options > Controls binds other keys; a gamepad's
      left stick rolls, its shoulder buttons turn the camera, A lifts it, Start pauses; in the extended mode
      R restarts, F2 free camera, [ ] slower and faster, F5 / F9 save the moment and back to it";

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
    /// The textures the levels are drawn (and exported) with, instead of the settings' (resources.rs).
    pub textures: Option<gamedata::resources::TextureSet>,
    pub replay: Option<PathBuf>,
    pub headless: bool,
    /// Play the level by the test player (autopilot.rs), headless; the run is saved like a recording.
    pub autopilot: bool,
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
}

fn parse_args() -> Result<Options, String> {
    let mut o = Options {
        data: None,
        import: None,
        source: Source::Original(1),
        chosen: false,
        record: None,
        export: None,
        textures: None,
        replay: None,
        headless: false,
        autopilot: false,
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
                o.source = Source::Open(level::open::full_path(&dir).map_err(|e| format!("{}: {e}", dir.display()))?);
                o.chosen = true;
            }
            "--export" => o.export = Some(value()?.into()),
            "--textures" => {
                let v = value()?;
                o.textures = Some(
                    gamedata::resources::TextureSet::parse(&v).ok_or("--textures is own or original".to_owned())?,
                );
            }
            "--record" => o.record = Some(value()?.into()),
            "--replay" => o.replay = Some(value()?.into()),
            "--headless" => o.headless = true,
            "--autopilot" => o.autopilot = true,
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
                let s: u64 = value()?.parse().map_err(|_| "--give-up needs seconds".to_owned())?;
                o.give_up = s.clamp(5, 3600);
            }
            "--rewinds" => {
                let n: u32 = value()?.parse().map_err(|_| "--rewinds needs a number".to_owned())?;
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
            "--debug" => o.debug = true,
            "--screenshot" => o.screenshot = Some(value()?.into()),
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
                if !["wood", "stone", "paper"].contains(&v.as_str()) {
                    return Err("--ball is wood, stone or paper".into());
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

fn main() -> ExitCode {
    let result = parse_args().and_then(|o| {
        if let Some(from) = &o.import {
            let copied = levels::import(from)?;
            levels::prepare(&lang::docs_language(app::setting("language").as_deref()));
            println!("{copied} files copied to {}", levels::dir(levels::Kind::Original).display());
            return Ok(());
        }
        // the menu's Import: the window that brings in the player's copy, in a process of its own (winit runs one
        // event loop a process), then the game again, with the copy or without it
        if std::env::var_os(IMPORT).is_some() {
            if let Err(w) = setup::run(o.size) {
                eprintln!("drahtseilakt: the window to bring in the game did not open: {w}");
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
            o.source = Source::Open(first.level);
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
