// Drahtseilakt: the game. Loads a level from the player's own copy (an original level or any .nmo map), plays
// it in a window, records and replays runs, and can run without a window (replay checks, screenshots).

// Hostile input is parsed only by the `nmo` crate; this crate works on validated models, and geometry code
// reads better with plain arithmetic and indexing of fixed-size arrays.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

mod app;
mod assets;
mod audio;
mod autopilot;
mod ballsound;
mod collide;
mod endscreen;
mod extra;
mod freecam;
mod hud;
mod menu;
mod menuscene;
mod modules;
mod music;
mod particles;
mod physics;
mod proximity;
mod render;
mod replay;
mod scene;
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

  --data DIR          the Ballance folder (default: $OPENBALLANCE_DATA, then ./data)
  --level N           play original level N (1-13; default 1)
  --map FILE.nmo      play a level file: a community map or any .nmo
  --open DIR          play a level in the open format (a folder with level.gltf)
  --export DIR        convert the chosen level to the open format in DIR (textures as PNG) and exit
  --record FILE       write a replay of the run (also written automatically on finishing a level)
  --replay FILE       play a replay instead of the keyboard
  --headless          no window: run the replay to its end and print what happened
  --autopilot         no window: the test player plays the level (to --ticks); saved with --record
  --follow FILE       the test player follows the ball's way in a replay of a player's run
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

keys: arrows roll the ball (relative to the camera), Shift+Left/Right turn the camera,
      Space lifts the camera, R restarts, F11 full screen, Esc opens the level menu;
      after a finished level Enter plays the next one";

#[derive(Clone)]
pub struct Options {
    pub data: Option<PathBuf>,
    pub source: Source,
    /// A level was named (--level, --map, --open): no menu at the start.
    pub chosen: bool,
    pub record: Option<PathBuf>,
    pub export: Option<PathBuf>,
    pub replay: Option<PathBuf>,
    pub headless: bool,
    /// Play the level by the test player (autopilot.rs), headless; the run is saved like a recording.
    pub autopilot: bool,
    /// The test player follows the ball's way in this replay (a player's run) instead of finding one.
    pub follow: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub ticks: Option<u64>,
    pub trace: Option<u64>,
    pub from: Option<u64>,
    pub size: (u32, u32),
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
        source: Source::Original(1),
        chosen: false,
        record: None,
        export: None,
        replay: None,
        headless: false,
        autopilot: false,
        follow: None,
        screenshot: None,
        ticks: None,
        trace: None,
        from: None,
        size: (1600, 900),
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
                o.source = Source::Open(value()?.into());
                o.chosen = true;
            }
            "--export" => o.export = Some(value()?.into()),
            "--record" => o.record = Some(value()?.into()),
            "--replay" => o.replay = Some(value()?.into()),
            "--headless" => o.headless = true,
            "--autopilot" => o.autopilot = true,
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
                let n: usize = value()?.parse().map_err(|_| "--sector needs a number".to_owned())?;
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

fn main() -> ExitCode {
    let result = parse_args().and_then(|o| {
        let data = Data::locate(o.data.clone())?;
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
