// The command line: the game's options and its help, for the game's binary and for programs built on the library,
// which add options of their own (`parse`'s `extra`) and pass the rest through.

use crate::{Options, default_options};
use gamedata::assets::Source;
use std::path::PathBuf;

pub const USAGE: &str = "\
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
  --sounds THEME      the sound theme the run plays, instead of the settings': neuro (generated), system,
                      original (the copy's) or a theme of the sounds folder
  --record FILE       write a replay of the run (also written automatically on finishing a level)
  --replay FILE       play a replay instead of the keyboard
  --headless          no window: run the replay to its end and print what happened
  --video FILE        with --replay: raw RGB frames, 60 a second at --size, into FILE (scripts/video.sh)
  --screenshot FILE   no window: render the frame after the replay (or --ticks N) to a PNG
  --ticks N           with --screenshot: how many ticks to simulate first (default: the replay's end, or 1)
  --trace N           with --headless: print the ball's position every N ticks
  --from N            with --replay: play the replay up to tick N without a window, then hand over the keys
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
  --audit FILE|DIR    no window: audit a map, an open-format level or a folder of them, with the copy and without it: bodies that collide
                      without being drawn (invisible blockers), stand-ins without a module, missing textures,
                      objects and modes not read; prints a summary by the number of maps hit (may be repeated)
  --show-colliders    draw the floors and rails a map hides (red) and its hidden stoppers (yellow) as translucent
                      shapes: what stops the ball without being seen
  --audit-report FILE|DIR
                      no window: check a map or a folder of them and write report.txt beside each level (what the
                      game does not support yet in it, read by the menu); prints a verdict per map (may be repeated)
  --convert-audit FILE|DIR
                      no window: convert each map to the open format (into --convert-out DIR, default the temp
                      folder's convert-audit/), load it again and compare it with the map: contents, scripts, and
                      --ticks N (default 3000) ticks played with the same keys; one line a map (may be repeated)
  --audit-csv FILE    with --audit: write one row per map, copy and kind of issue (with the names) to FILE
  --supports-check geometry|runs DIR
                      where the swinging sacks get a support (the sack-supports mod) in the original levels and the
                      shipped maps, each placed one measured again; with runs also against the ball's way in the
                      recorded runs in DIR (DIR/runs/Level_NN*.replay, DIR/Level_NN-owner.replay). Fails when one is
                      too near the ball's space
  --look X,Y,Z,TX,TY,TZ  with --screenshot: camera at X,Y,Z looking at TX,TY,TZ instead of the game camera

Without --level, --map or --open the window opens with the level menu.

Without the game found, the window offers to bring in your Ballance: from an installed game, the CD in
its drive or an image of it (found on this computer, dropped on the window or typed).

keys: arrows roll the ball (relative to the camera), Shift+Left/Right turn the camera,
      Space lifts the camera, F11 full screen, Esc pauses; Options > Controls binds other keys; a gamepad's
      left stick rolls, its shoulder buttons turn the camera, A lifts it, Start pauses; the mods' keys are
      on each mod's page (god-mode's in a debug run)";

/// A played game built from the source tree is a debug run unless `--no-debug` says otherwise (the owner's decision,
/// 2026-11-08: easier to start). A release package is built with `DRAHTSEILAKT_RELEASE` set
/// (`scripts/make-portable.sh` without `--dev`, owner 2026-11-16: release builds for every system start without
/// debug), and then a played game is plain unless `--debug` is given. Runs without a player (replays, headless,
/// a run an outside program drives, video, screenshots) stay plain unless `--debug` is given, so recorded runs and tests are unchanged.
const DEBUG_BY_DEFAULT: bool = option_env!("DRAHTSEILAKT_RELEASE").is_none();

/// What a program built on the library makes of an option before the game does: true when it was its own. It gets
/// the options, the option's name and a way to take the option's value.
pub type Extra<'a> = &'a mut dyn FnMut(
    &mut Options,
    &str,
    &mut dyn FnMut() -> Result<String, String>,
) -> Result<bool, String>;

/// The options from the command line's arguments (without the program's name). `usage` is printed for `--help` and
/// with an unknown option; `extra` sees each option first (the game's binary passes one that takes none).
pub fn parse(
    args: impl IntoIterator<Item = String>,
    usage: &str,
    extra: Extra,
) -> Result<Options, String> {
    let mut debug: Option<bool> = None;
    let mut o = default_options();
    // `--name=value` is the same as `--name value` (handy for values that start with a minus).
    let mut args = args.into_iter().flat_map(|a| match a.split_once('=') {
        Some((k, v)) if k.starts_with("--") => vec![k.to_owned(), v.to_owned()],
        _ => vec![a],
    });
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        if extra(&mut o, &a, &mut value)? {
            continue;
        }
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
            "--sounds" => {
                let v = value()?;
                if !gamedata::sound_themes::plain(&v) {
                    return Err(format!("--sounds: not a sound theme's name: {v}"));
                }
                o.sounds = Some(v);
            }
            "--record" => o.record = Some(value()?.into()),
            "--replay" => o.replay = Some(value()?.into()),
            "--headless" => o.headless = true,
            "--video" => o.video = Some(value()?.into()),
            "--debug" => debug = Some(true),
            "--no-debug" => debug = Some(false),
            "--show-colliders" => o.show_colliders = true,
            "--audit" => o.audit.push(value()?.into()),
            "--audit-report" => o.audit_report.push(value()?.into()),
            "--audit-csv" => o.audit_csv = Some(value()?.into()),
            "--convert-audit" => o.convert_audit.push(value()?.into()),
            "--convert-out" => o.convert_out = Some(value()?.into()),
            "--supports-check" => {
                o.supports_check = Some(match value()?.as_str() {
                    "geometry" => None,
                    "runs" => Some(
                        args.next()
                            .ok_or("--supports-check runs needs the folder of the runs")?
                            .into(),
                    ),
                    _ => return Err("--supports-check geometry|runs DIR".into()),
                })
            }
            "--screenshot" => o.screenshot = Some(value()?.into()),
            "--sky" => {
                let name = value()?;
                if !crate::sky::valid_name(&name) {
                    return Err(format!(
                        "--sky {name}: a sky's name is letters, digits, _ and - (e.g. Mountains)"
                    ));
                }
                crate::sky::set_override(name);
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
                println!("{usage}");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option {a}\n\n{usage}")),
        }
    }
    let unattended = o.replay.is_some()
        || o.headless
        || o.driven
        || o.video.is_some()
        || o.screenshot.is_some()
        || !o.audit.is_empty()
        || !o.audit_report.is_empty()
        || !o.convert_audit.is_empty()
        || o.export.is_some();
    o.debug = debug.unwrap_or(DEBUG_BY_DEFAULT && !unattended);
    o.dev_logs = unattended
        || debug == Some(true)
        || std::env::var_os("DRAHTSEILAKT_DEV_LOGS").is_some_and(|v| !v.is_empty() && v != "0")
        || gamedata::profile::setting("dev-logs").is_some_and(|v| matches!(v.trim(), "on" | "yes"));
    // the converter's audit plays hundreds of maps with made-up keys: none of it belongs in the player's runs; and
    // DRAHTSEILAKT_DEV_LOGS=0 turns the logs off for any run (tests and scripts, which must not write into the
    // player's profile)
    if !o.convert_audit.is_empty()
        || std::env::var_os("DRAHTSEILAKT_DEV_LOGS").is_some_and(|v| v == "0")
    {
        o.dev_logs = false;
    }
    Ok(o)
}

/// Where the game found its shipped assets and keeps the player's data (and whether the copy is portable).
pub fn paths_line() -> String {
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
