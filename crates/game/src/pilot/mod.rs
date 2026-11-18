// The test player (the autopilot), our tool for the videos, and the video tracking it learns from. It drives the game
// only through the library's public API and the controller hook (control.rs); it moves out of the game into our tools
// repository next.

pub mod autopilot;
pub mod edges;
pub mod hints;
pub mod learned;
pub mod levelstats;
pub mod locate;
pub mod pilotdata;
pub mod run;
pub mod watch;
pub mod zones;

use crate::Options;
use crate::app::Game;
use crate::assets::Data;

/// Whether the options ask for the test player or the video tracking rather than the game.
pub fn wanted(o: &Options) -> bool {
    o.autopilot
        || o.locate.is_some()
        || o.follow.is_some()
        || o.frames.is_some()
        || o.level_stats.is_some()
}

/// Runs what the options ask of the test player: the level statistics, the video tracking (--locate, --frames), a
/// followed way (--follow), or the test player's own run (headless, or in the window with --watch).
pub fn run(mut o: Options, data: Data) -> Result<(), String> {
    crate::app::prepare(&mut o, &data)?;
    if let Some(dir) = &o.level_stats {
        let Some(store) = pilotdata::root() else {
            return Err(format!(
                "--level-stats needs the autopilot store ({} or ../drahtseilakt-tools/autopilot)",
                pilotdata::ENV
            ));
        };
        return levelstats::run(&o, &data, dir, &store.join("hints"));
    }
    // a level folder's own game files (modules, sky, sounds) come before the copy's
    let data = data.for_source(&o.source);
    // the video tracking draws the levels as it was tuned on: without the cloud layer
    if o.locate.is_some() || o.follow.is_some() || o.frames.is_some() {
        crate::scene::set_clouds(false);
    }
    if let Some(dir) = &o.locate {
        let mut game = Game::new(&o, &data)?;
        return run::locate_run(&mut game, &o, dir);
    }
    if let Some(follow) = &o.follow {
        return run::follow_run(&o, &data, follow);
    }
    let mut game = Game::new(&o, &data)?;
    if let Some(dir) = &o.frames {
        return run::frames_run(&mut game, dir);
    }
    // (--replay with --from: the run's keys up to that tick, then the test player from where they left it)
    if let Some(tick) = o.from {
        game.fast_forward(tick);
    }
    run::autopilot(game, &data, &o)
}
