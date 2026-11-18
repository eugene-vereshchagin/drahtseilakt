// Drahtseilakt as a library: the game's modules and its entry (`run`), for the game's own thin binary (main.rs) and for
// tools of ours that drive the game from outside (through `app::Controller`).

// Tests run on fixed fixtures they write themselves, never on hostile input; plain indexing and arithmetic keep
// their assertions readable. The shipped code gets no such allow: each module that reads outside data is fixed.
#![cfg_attr(test, allow(clippy::arithmetic_side_effects, clippy::indexing_slicing))]

pub mod app;
mod audio;
mod audit;
mod ballsound;
mod clouds;
pub mod collide;
pub mod control;
mod convert_audit;
mod dik;
mod endscreen;
mod entities;
mod extra;
mod facade;
mod fallback;
mod features;
pub mod files;
mod freecam;
#[cfg(test)]
mod fuzz_tests;
mod highscore;
pub mod hud;
pub mod lang;
pub mod levels;
mod lights;
mod loading;
mod look;
mod manual;
mod matstate;
mod menu;
mod menuscene;
mod modified;
mod modrt;
mod modules;
mod music;
mod options;
mod partcheck;
mod particles;
mod physics;
mod pieces;
pub mod pilot;
mod playtime;
mod proximity;
pub mod render;
pub mod replay;
mod report;
mod retro;
pub mod runway;
pub mod scene;
mod script_sound;
mod script_text;
mod script_text3d;
mod scripting;
mod scriptsky;
mod showcase;
pub mod sim;
mod sky;
mod splashpic;
pub mod supports;
mod terrain;
mod testlevels;
mod texpaint;
mod titlescene;
mod trafo;
pub mod ui;
mod userreplay;
mod volume;
mod watch;
pub use gamedata::assets;

pub use pilot::autopilot::Style as PilotStyle;
pub use sky::{set_override as set_sky_override, valid_name as valid_sky_name};

use assets::{Data, Source};
use std::path::PathBuf;

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
    /// The sound theme the run plays, instead of the settings' (sound_themes.rs).
    pub sounds: Option<String>,
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
    /// How the test player rides (calm or speedrun).
    pub style: crate::pilot::autopilot::Style,
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
    /// Measure the sacks' supports of every original level and shipped map, then stop; with a folder of recorded
    /// runs (`runs DIR`) also against the ball's way in them.
    pub supports_check: Option<Option<PathBuf>>,
    /// No window: audit these maps (files or folders of them) for what is not drawn or not there (audit.rs).
    pub audit: Vec<PathBuf>,
    pub audit_csv: Option<PathBuf>,
    /// No window: convert these maps to the open format and compare the copies (convert_audit.rs).
    pub convert_audit: Vec<PathBuf>,
    /// Where `--convert-audit` writes its exports (each removed after its comparison).
    pub convert_out: Option<PathBuf>,
    /// No window: check these maps and write report.txt beside each (report.rs).
    pub audit_report: Vec<PathBuf>,
    /// Draw the floors, rails and stoppers a map hides as translucent markers.
    pub show_colliders: bool,
    /// The development logs of `data/replays/<level>/` (session logs, watch log, spin-loss dumps, attempts and every
    /// finished run's replay) are written: unattended runs, `--debug` given, DRAHTSEILAKT_DEV_LOGS set (`0` turns them off for any run), or
    /// `dev-logs on` in the settings. A player's game writes none (their replays are the Replays setting's).
    pub dev_logs: bool,
    /// Nothing is written to the player's profile: no runs, attempts or moved folders (the converter's audit plays
    /// maps headless with made-up keys).
    pub no_save: bool,
}

/// The options with nothing asked for.
pub fn default_options() -> Options {
    Options {
        data: None,
        import: None,
        source: Source::Original(1),
        chosen: false,
        record: None,
        export: None,
        export_parts: None,
        textures: None,
        sounds: None,
        models: None,
        replay: None,
        headless: false,
        autopilot: false,
        watch: false,
        follow: None,
        then: Vec::new(),
        plan: None,
        hints: None,
        style: crate::pilot::autopilot::Style::Calm,
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
        convert_audit: Vec::new(),
        convert_out: None,
        audit_report: Vec::new(),
        show_colliders: false,
        dev_logs: false,
        no_save: false,
    }
}

/// Runs what the options ask for: the import, a command without a window, or the game.
pub fn run(o: Options) -> Result<(), String> {
    if let Some(from) = &o.import {
        let copied = levels::import(from)?;
        levels::prepare(&lang::docs_language(app::setting("language").as_deref()));
        println!(
            "{copied} files copied to {}",
            levels::dir(levels::Kind::Original).display()
        );
        return Ok(());
    }
    // the copy of the original is optional: without it the game plays its shipped levels (decisions.md, "The game
    // plays out of the box"); a folder named with --data must itself be a copy (no other place is tried)
    let data = match o.data.clone() {
        Some(d) => Data::at(d)?,
        None => Data::find(None),
    };
    let mut o = o;
    if !data.has_copy() && !o.chosen {
        // the level behind the menu at the start: the first shipped level past the showcase (order 0, the check
        // level with its sign boards, levels.md "The showcase level") unless it is the only one
        let shipped = levels::shipped();
        let first = shipped
            .iter()
            .find(|p| p.order != Some(0))
            .or(shipped.first())
            .cloned()
            .ok_or(
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
    if pilot::wanted(&o) {
        return pilot::run(o, data);
    }
    app::run(o, data)
}
