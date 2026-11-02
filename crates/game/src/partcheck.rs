// Checks of our own parts (assets/own/3D Entities, made by assets/own/make/make_parts.py): the shipped test level
// plays with no copy of the original at all, and our parts behave as the original's do. The original's parts come
// from the player's copy (OPENBALLANCE_DATA; those checks are skipped without it): every module of levels 1 to 12 is
// run twice from the same start with the same input, once made of the copy's part and once of ours, and the ball's
// ways and the parts' ways are compared (decisions.md, "The game plays out of the box": physics values unchanged,
// collision shapes of the measured sizes, so the same input gives the same way within a tolerance).

#[cfg(test)]
mod tests {
    use crate::assets::{Data, Source};
    use crate::sim::{BallKind, Sim, input};
    use glam::Vec3;
    use std::path::{Path, PathBuf};

    fn shipped(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/levels")
            .join(name)
    }

    /// A run of `ticks` from sector `sector` with `ball` at `at`, the input given by the tick; the ball's places and
    /// those of the parts the physics moves.
    fn run(
        level: &level::Level,
        data: &Data,
        sector: usize,
        ball: BallKind,
        at: Vec3,
        ticks: u64,
        input: &dyn Fn(u64) -> u8,
    ) -> (Vec<Vec3>, Vec<Vec<Vec3>>, Option<u64>) {
        let modules = crate::scene::load_modules(level, data);
        let mut sim = Sim::with_modules(level, ball, &modules, false);
        if let Some(balls) = data.part("3D Entities/Balls.nmo") {
            sim.set_ball_boxes(crate::scene::BallMeshes::find(&balls.level).boxes);
        }
        sim.start_from(sector, Some(ball), Some(at));
        let (mut way, mut parts, mut finished) = (Vec::new(), Vec::new(), None);
        for t in 0..ticks {
            sim.step(input(t));
            if sim.finished && finished.is_none() {
                finished = Some(t);
            }
            way.push(sim.ball.position);
            parts.push(
                sim.module_bodies()
                    .iter()
                    .filter_map(|m| sim.world().state(m.body).map(|s| s.pos))
                    .collect(),
            );
        }
        (way, parts, finished)
    }

    /// The shipped test level, with no copy and nothing in the player's folders: it loads, every part it names is
    /// ours, and the ball rolled straight on along +X passes the three sectors' transformers and checkpoints and
    /// finishes on the balloon.
    #[test]
    fn the_test_level_plays_without_a_copy() {
        let dir = shipped("Parts_Test");
        let source = Source::Open(dir.clone());
        let data = Data::new(None).for_source(&source);
        assert!(data.own_parts, "a new level takes our parts");
        let level = data.load(&source).expect("the test level loads").level;
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

    /// Every module of levels 1 to 12 with the copy's part and with ours: the ball dropped onto the first stand-in of
    /// each from 6 above (a wooden one, and a stone one, which works the bridges and pushers), held forward for a
    /// second, then left alone; the ball's way and the parts' ways compared over 5 seconds. A part of ours made to
    /// the measured sizes gives the same way within a fraction of the ball's radius while the ball is on it; after
    /// that a chaotic run (a ball falling off an edge or bouncing between parts) may part ways, so only the first
    /// two seconds are held to the tolerance and the rest is reported.
    #[test]
    fn our_modules_behave_as_the_copys() {
        let Some(copy) =
            std::env::var_os("OPENBALLANCE_DATA").and_then(|_| Data::locate(None).ok())
        else {
            return;
        };
        let mut ours = copy.clone();
        ours.own_parts = true;
        let mut report = Vec::new();
        let mut worst: Vec<(String, f32)> = Vec::new();
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
                    let (wa, pa, fa) = run(&level, &copy, sector, ball, at, 300, &push);
                    let (wb, pb, fb) = run(&level, &ours, sector, ball, at, 300, &push);
                    let ball_2s = apart(&wa, &wb, 120);
                    let ball_5s = apart(&wa, &wb, 300);
                    let parts_2s = pa
                        .iter()
                        .zip(&pb)
                        .take(120)
                        .flat_map(|(x, y)| x.iter().zip(y).map(|(p, q)| p.distance(*q)))
                        .fold(0.0, f32::max);
                    let same_count = pa.last().map(Vec::len) == pb.last().map(Vec::len);
                    report.push(format!(
                        "level {n:2} {name:12} {ball:?}: ball {ball_2s:.3} (2 s) {ball_5s:.3} (5 s), parts {parts_2s:.3} \
                         (2 s){}",
                        if same_count { "" } else { ", NOT the same parts" }
                    ) + &format!(
                        ", apart from tick {:?}, finished {fa:?} / {fb:?}",
                        wa.iter().zip(&wb).position(|(p, q)| p.distance(*q) > 0.1)
                    ));
                    let score = if same_count {
                        ball_2s.max(parts_2s)
                    } else {
                        f32::MAX
                    };
                    worst.push((format!("{name} {ball:?}"), score));
                }
            }
        }
        println!("{}", report.join("\n"));
        let off: Vec<_> = worst.iter().filter(|w| w.1 > TOLERANCE).collect();
        assert!(
            off.is_empty(),
            "beyond {TOLERANCE} in the first two seconds: {off:?}\n{}",
            report.join("\n")
        );
    }

    /// Half the ball's radius: how far our part may take the ball (or itself) from where the copy's would in the first
    /// two seconds.
    const TOLERANCE: f32 = 1.0;

    #[test]
    fn an_original_level_keeps_the_copys_parts() {
        let data = Data::new(Some(PathBuf::from("/nonexistent")));
        assert!(!data.for_source(&Source::Original(1)).own_parts);
        assert!(
            !data
                .for_source(&Source::Nmo(PathBuf::from("x.nmo")))
                .own_parts
        );
        assert!(
            data.for_source(&Source::Open(shipped("Parts_Test")))
                .own_parts
        );
    }
}
