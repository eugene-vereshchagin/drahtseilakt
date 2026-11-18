// A recorded run played without a window, tick by tick: the level as the run loads it, the ball's way through it and
// what happened on the way. The supports check (supports.rs) measures against it.

use crate::Options;
use crate::app::Game;
use crate::assets::Data;
use crate::sim::{BallKind, Event};
use glam::Vec3;

/// One tick of a replayed run.
#[derive(Clone, Copy, Debug)]
pub struct TickRec {
    pub pos: Vec3,
    pub ground: bool,
    /// As the game shows it (1 is the first).
    pub sector: usize,
    pub kind: BallKind,
    pub updraft: bool,
    pub held: bool,
    pub dying: bool,
}

/// A level as a run loads it, and the ball's way through it (`play`).
pub struct Run {
    pub level: level::Level,
    pub modules: crate::scene::Modules,
    pub ticks: Vec<TickRec>,
    /// Each event with its tick and the sector it came in.
    pub events: Vec<(u64, Event, usize)>,
    /// The loose things (boxes, balls) and how far each got from where it began.
    pub moved: Vec<(String, f32)>,
    pub finished: bool,
    pub begun_elsewhere: bool,
    /// A way given instead of a run (floor points: a tool's way through a level), when there is none.
    pub route: Option<Vec<Vec3>>,
    /// The checkpoints, then the end.
    pub checkpoints: Vec<Vec3>,
}

/// The level and its modules as a run loads them, and the ball's way through the replay (`o.replay`) played
/// without a window, tick by tick.
pub fn play(o: &Options, data: &Data) -> Result<Run, String> {
    let mut game = Game::new(o, data)?;
    let mut run = Run {
        level: game.level.level.clone(),
        modules: game.modules.clone(),
        ticks: Vec::new(),
        events: Vec::new(),
        moved: Vec::new(),
        finished: false,
        begun_elsewhere: game
            .start
            .as_ref()
            .is_some_and(|s| s.sector > 1 || s.at.is_some()),
        route: None,
        checkpoints: game.sim.stages(),
    };
    let end = game
        .replay_in
        .as_ref()
        .map(|r| r.end)
        .ok_or("a run to measure needs a replay")?;
    // each loose thing's first place and how far it got from it (sampled twice a second)
    let mut loose: std::collections::HashMap<usize, (glam::Vec3, f32)> =
        std::collections::HashMap::new();
    let mut seen = 0;
    while game.sim.tick < end && !game.sim.finished && !game.sim.game_over {
        game.tick(0);
        let s = &game.sim;
        run.ticks.push(TickRec {
            pos: s.ball.position,
            ground: s.ball.on_ground,
            sector: s.sector.saturating_add(1),
            kind: s.ball.kind,
            updraft: s.in_updraft(),
            held: s.is_held(),
            dying: s.is_dying(),
        });
        for e in s.events.iter().skip(seen) {
            run.events.push((e.0, e.1, s.sector.saturating_add(1)));
        }
        seen = s.events.len();
        if s.tick % 30 == 1 {
            for &(obj, body) in s.loose() {
                if let Some(st) = s.world().state(body) {
                    let e = loose.entry(obj).or_insert((st.pos, 0.0));
                    e.1 = e.1.max(st.pos.distance(e.0));
                }
            }
        }
    }
    run.finished = game.sim.finished;
    let mut moved: Vec<(usize, f32)> = loose.into_iter().map(|(o, (_, d))| (o, d)).collect();
    moved.sort_by_key(|m| m.0);
    run.moved = moved
        .into_iter()
        .filter_map(|(o, d)| Some((game.level.level.objects.get(o)?.name.clone(), d)))
        .collect();
    Ok(run)
}
