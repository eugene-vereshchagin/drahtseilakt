// Checks of our own parts (assets/own/3D Entities, made by assets/own/make/make_parts.py): the shipped test level
// plays with no copy of the original at all, and our parts behave as the original's do. The original's parts come
// from the player's copy (OPENBALLANCE_DATA; those checks are skipped without it): every module of levels 1 to 12 is
// run twice from the same start with the same input, once made of the copy's part and once of ours, and the ball's
// ways and the parts' ways are compared (decisions.md, "The game plays out of the box": physics values unchanged,
// collision shapes of the measured sizes, so the same input gives the same way within a tolerance).
// Reason for the allow: a developer check over the validated level model.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use crate::assets::{Data, Source};
    use crate::sim::{BallKind, Sim, input};
    use glam::Vec3;
    use std::path::{Path, PathBuf};

    fn shipped(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/levels/drahtseilakt")
            .join(name)
    }

    /// A run's record: the ball's places and those of the parts the physics moves, tick by tick, the tick the level
    /// was finished, whether a ball was lost, and which parts moved more than a unit from where they started.
    struct Run {
        way: Vec<Vec3>,
        parts: Vec<Vec<Vec3>>,
        finished: Option<u64>,
        lost: bool,
        moved: Vec<bool>,
        names: Vec<String>,
    }

    impl Run {
        /// What came of the run at its end: finished, a ball lost, the parts that moved. A finished level's run is
        /// over at its finish: what the balloon does with the ball after it is not its outcome.
        fn outcome(&self) -> (bool, bool, &[bool]) {
            match self.finished {
                Some(_) => (true, false, &[]),
                None => (false, self.lost, &self.moved),
            }
        }

        /// The part furthest from where another run's is, over the first `n` ticks: its name, the distance, and the
        /// first tick it was more than 0.1 away.
        fn furthest(&self, other: &Run, n: usize) -> (String, f32, Option<usize>) {
            let mut best = (String::new(), 0.0f32, None);
            for (k, name) in self.names.iter().enumerate() {
                let d: Vec<f32> = self
                    .parts
                    .iter()
                    .zip(&other.parts)
                    .take(n)
                    .map(|(x, y)| x.get(k).zip(y.get(k)).map_or(0.0, |(p, q)| p.distance(*q)))
                    .collect();
                let m = d.iter().copied().fold(0.0, f32::max);
                if m > best.1 {
                    best = (name.clone(), m, d.iter().position(|&x| x > 0.1));
                }
            }
            best
        }
    }

    /// A run of `ticks` from sector `sector` with `ball` at `at`, the input given by the tick.
    fn run(
        level: &level::Level,
        data: &Data,
        sector: usize,
        ball: BallKind,
        at: Vec3,
        ticks: u64,
        input: &dyn Fn(u64) -> u8,
    ) -> Run {
        let modules = crate::scene::load_modules(level, data);
        let mut sim = Sim::with_modules(level, ball, &modules, false);
        if let Some(balls) = data.part("3D Entities/Balls.nmo") {
            sim.set_ball_boxes(crate::scene::BallMeshes::find(&balls.level).boxes);
        }
        sim.start_from(sector, Some(ball), Some(at));
        let lives = sim.lives;
        let names = sim
            .module_bodies()
            .iter()
            .map(|m| m.object.clone())
            .collect();
        let mut r = Run {
            way: Vec::new(),
            parts: Vec::new(),
            finished: None,
            lost: false,
            moved: Vec::new(),
            names,
        };
        for t in 0..ticks {
            sim.step(input(t));
            if sim.finished && r.finished.is_none() {
                r.finished = Some(t);
            }
            // a ball fallen far below where it was dropped is lost, whether or not the game has taken it yet
            r.lost |= sim.lives < lives || sim.ball.position.y < at.y - 40.0;
            r.way.push(sim.ball.position);
            r.parts.push(
                sim.module_bodies()
                    .iter()
                    .filter_map(|m| sim.world().state(m.body).map(|s| s.pos))
                    .collect(),
            );
        }
        if let (Some(first), Some(last)) = (r.parts.first(), r.parts.last()) {
            r.moved = first
                .iter()
                .zip(last)
                .map(|(a, b)| a.distance(*b) > 1.0)
                .collect();
        }
        r
    }

    /// The shipped test level, with no copy and nothing in the player's folders: it loads, every part it names is
    /// ours, and the ball rolled straight on along +X passes the three sectors' transformers and checkpoints and
    /// finishes on the balloon.
    #[test]
    fn the_test_level_plays_without_a_copy() {
        let dir = shipped("Parts_Test");
        let source = Source::Open(dir.clone());
        let data = Data::new(None).for_source(&source);
        let level = data.load(&source).expect("the test level loads").level;
        let data = data.with_parts(&level);
        assert!(data.own_parts, "our level says it is made for our parts");
        let modules = crate::scene::load_modules(&level, &data);
        let named: std::collections::BTreeSet<String> = (0..level.objects.len())
            .filter_map(|i| crate::scene::module_of(&level, i).map(str::to_owned))
            // a reset point's stand-in names no part (the original has none either)
            .filter(|n| !n.starts_with("PR_"))
            .collect();
        let missing: Vec<&String> = named.iter().filter(|n| modules.get(n).is_none()).collect();
        assert!(
            missing.is_empty(),
            "parts not found among ours: {missing:?}"
        );
        assert!(
            data.part("3D Entities/Balls.nmo").is_some()
                && data.part("3D Entities/AnimTrafo.nmo").is_some()
        );
        let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, false);
        if let Some(balls) = data.part("3D Entities/Balls.nmo") {
            sim.set_ball_boxes(crate::scene::BallMeshes::find(&balls.level).boxes);
        }
        let mut kinds = Vec::new();
        for _ in 0..60 * 120 {
            sim.step(input::UP);
            if kinds.last() != Some(&sim.ball.kind) {
                kinds.push(sim.ball.kind);
            }
            if sim.finished {
                break;
            }
        }
        assert!(
            sim.finished,
            "not finished: the ball at {:?}, sector {}",
            sim.ball.position,
            sim.sector + 1
        );
        println!(
            "finished at tick {} ({:.1} s), the balls {kinds:?}",
            sim.tick,
            sim.tick as f32 / 60.0
        );
        assert_eq!(
            sim.lives, 4,
            "the extra life on the way taken, no ball lost"
        );
        assert_eq!(
            kinds,
            [
                BallKind::Wood,
                BallKind::Stone,
                BallKind::Paper,
                BallKind::Wood
            ],
            "the transformers"
        );
    }

    /// The largest distance between two ways, over their first `n` places.
    fn apart(a: &[Vec3], b: &[Vec3], n: usize) -> f32 {
        a.iter()
            .zip(b)
            .take(n)
            .map(|(p, q)| p.distance(*q))
            .fold(0.0, f32::max)
    }

    /// The largest distance between the parts of two runs over their first `n` ticks.
    fn parts_apart(a: &Run, b: &Run, n: usize) -> f32 {
        a.parts
            .iter()
            .zip(&b.parts)
            .take(n)
            .flat_map(|(x, y)| x.iter().zip(y).map(|(p, q)| p.distance(*q)))
            .fold(0.0, f32::max)
    }

    /// Every module of levels 1 to 12 with the copy's part and with ours: the ball dropped onto the first stand-in of
    /// each from 6 above (a wooden one, and a stone one, which works the bridges and pushers), held forward for a
    /// second, then left alone for four more. How far apart two runs may come is measured on the copy itself: the
    /// same run started 0.01 to 0.05 aside (NUDGES; a baseline of how chaotic the case is). Wherever the copy's outcome
    /// after five seconds (finished, a ball lost, which parts moved) does not change for any nudge, ours must come to
    /// the same outcome and stay within twice the baseline, or within TOLERANCE where the case is calm, over the first
    /// two seconds (a finished run up to its finish). A case whose outcome a nudge already changes is reported only.
    #[test]
    fn our_modules_behave_as_the_copys() {
        let Some(copy) =
            std::env::var_os("OPENBALLANCE_DATA").and_then(|_| Data::locate(None).ok())
        else {
            return;
        };
        let mut ours = copy.clone();
        ours.own_parts = true;
        let (mut report, mut off) = (Vec::new(), Vec::new());
        let mut done: Vec<String> = Vec::new();
        for n in 1..=12 {
            let Ok(loaded) = copy.load(&Source::Original(n)) else {
                continue;
            };
            let level = loaded.level;
            for i in 0..level.objects.len() {
                let Some(name) = crate::scene::module_of(&level, i) else {
                    continue;
                };
                if crate::modules::spec(name).is_none() || done.iter().any(|d| d == name) {
                    continue;
                }
                done.push(name.to_owned());
                let Some(sector) =
                    crate::sim::sector_near(&level, Vec3::from(level.objects[i].position()))
                else {
                    continue;
                };
                let at = Vec3::from(level.objects[i].position()) + Vec3::Y * 6.0;
                for ball in [BallKind::Wood, BallKind::Stone] {
                    let push = |t: u64| if t < 60 { input::UP } else { 0 };
                    let a = run(&level, &copy, sector, ball, at, 300, &push);
                    let nudged: Vec<Run> = NUDGES
                        .iter()
                        .map(|&d| run(&level, &copy, sector, ball, at + Vec3::from(d), 300, &push))
                        .collect();
                    let b = run(&level, &ours, sector, ball, at, 300, &push);
                    // a finished level's run is over: what follows (the balloon's flight) is not compared
                    let upto = nudged
                        .iter()
                        .map(|r| r.finished)
                        .chain([a.finished, b.finished])
                        .flatten()
                        .min()
                        .map_or(120, |f| usize::try_from(f).unwrap_or(120).min(120));
                    let base = nudged
                        .iter()
                        .map(|r| apart(&a.way, &r.way, upto).max(parts_apart(&a, r, upto)))
                        .fold(0.0, f32::max);
                    let ours_apart = apart(&a.way, &b.way, upto).max(parts_apart(&a, &b, upto));
                    let allowed = TOLERANCE.max(2.0 * base);
                    let robust = nudged.iter().all(|r| r.outcome() == a.outcome());
                    let same = a.outcome() == b.outcome();
                    let case = format!("level {n:2} {name:12} {ball:?}");
                    report.push(format!(
                        "{case}: ours {ours_apart:.3}, nudged copy {base:.3} (2 s); outcome {} ({}), finished \
                         {:?} / {:?}, lost {} / {}",
                        if same { "same" } else { "DIFFERENT" },
                        if robust { "robust" } else { "chaotic: the nudge changes it too" },
                        a.finished,
                        b.finished,
                        a.lost,
                        b.lost
                    ));
                    let (part, far, from) = a.furthest(&b, upto);
                    report.push(format!(
                        "    furthest part {part} {far:.3} from tick {from:?}"
                    ));
                    // where a nudge of the copy's own run already changes the outcome the case is chaotic: it is
                    // reported, not held to a bound
                    if robust && (ours_apart > allowed || !same) {
                        off.push(format!(
                            "{case}: {ours_apart:.3} allowed {allowed:.3}, outcome same {same}"
                        ));
                    }
                }
            }
        }
        println!("{}", report.join("\n"));
        assert!(
            off.is_empty(),
            "our parts differ: {off:?}\n{}",
            report.join("\n")
        );
    }

    /// The copy's run started this far aside, for how far apart two runs of the same part come: the largest of these
    /// is the case's baseline.
    const NUDGES: [[f32; 3]; 6] = [
        [0.01, 0.0, 0.0],
        [-0.02, 0.0, 0.0],
        [0.0, 0.0, 0.02],
        [0.0, 0.0, -0.05],
        [0.05, 0.0, 0.05],
        [0.0, 0.03, 0.0],
    ];

    /// How far our part may take the ball (or itself) from where the copy's would in the first two seconds of a calm
    /// case: half the ball's radius (2).
    const TOLERANCE: f32 = 1.0;

    /// Only a level's own marker (`"parts": "own"`, written by our level kit) takes our parts first: an original
    /// level, a map, and an open-format level without the marker (made or exported before) keep the copy's.
    #[test]
    fn only_a_level_made_for_our_parts_takes_them_first() {
        let data = Data::new(Some(PathBuf::from("/nonexistent")));
        let mut level = level::Level::default();
        assert!(
            !data.with_parts(&level).own_parts,
            "no marker: the copy's parts first"
        );
        level.settings.source_hash = None;
        level.settings.own_parts = true;
        assert!(data.with_parts(&level).own_parts);
        let shipped = data
            .load(&Source::Open(shipped("Parts_Test")))
            .expect("the test level loads")
            .level;
        assert!(
            shipped.settings.own_parts,
            "our level kit writes the marker"
        );
    }
}
