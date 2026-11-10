// Checks of the levels shipped with the game (assets/levels, made from their level.lvl by
// assets/own/make/leveldsl.py; docs/drahtseilakt/levels.md, "Our levels"): with no copy of the original and nothing in
// the player's folders, each loads, every part it names is ours, and each of the runs kept beside it
// (pilot/<style>.replay, the test player's runs in its styles) plays to its finish. A run is the proof that the level
// can be finished; a level changed without its runs recorded again fails here.

#[cfg(test)]
mod tests {
    use crate::assets::{Data, Source};
    use crate::replay::Replay;
    use crate::sim::{BallKind, Sim};
    use std::path::{Path, PathBuf};

    fn shipped() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/levels")
    }

    /// Our levels: the folders made from a level description, shipped or set aside in `levels-legacy_delme/` while
    /// the owner's picks of community maps stand in their place (CREDITS.md).
    fn levels() -> Vec<PathBuf> {
        let legacy = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../levels-legacy_delme");
        let mut v: Vec<PathBuf> = [shipped(), legacy]
            .iter()
            .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
            .map(|e| e.path())
            .filter(|p| p.join("level.lvl").is_file())
            .collect();
        v.sort();
        v
    }

    /// What came of playing a replay: finished (at which tick), lives at the end, extra points and lives taken.
    struct Outcome {
        finished: Option<u64>,
        lives: u32,
        sectors: usize,
    }

    fn play(dir: &Path, replay: &Path) -> Outcome {
        let source = Source::Open(dir.to_path_buf());
        let data = Data::new(None).for_source(&source);
        let loaded = data.load(&source).expect("the level loads");
        let data = data.with_parts(&loaded.level);
        assert!(
            data.own_parts,
            "{}: our level is made for our parts",
            dir.display()
        );
        let text = std::fs::read_to_string(replay).expect("the replay reads");
        let run = Replay::parse(&text).expect("the replay parses");
        assert_eq!(
            run.level_hash,
            loaded.hash,
            "{}: recorded on this level",
            replay.display()
        );
        assert!(
            run.start.is_none(),
            "{}: a run from the level's start",
            replay.display()
        );
        let modules = crate::scene::load_modules(&loaded.level, &data);
        let named: std::collections::BTreeSet<String> = (0..loaded.level.objects.len())
            .filter_map(|i| crate::scene::module_of(&loaded.level, i).map(str::to_owned))
            .filter(|n| !n.starts_with("PR_"))
            .collect();
        let missing: Vec<&String> = named.iter().filter(|n| modules.get(n).is_none()).collect();
        assert!(
            missing.is_empty(),
            "{}: parts not among ours: {missing:?}",
            dir.display()
        );
        let ball = loaded
            .level
            .start_ball
            .as_deref()
            .and_then(BallKind::from_name)
            .unwrap_or(BallKind::Wood);
        // played by the rules of the format it was recorded in, as the game plays it
        let mut sim = Sim::with_rules(
            &loaded.level,
            ball,
            &modules,
            loaded.level.settings.tutorial,
            run.version,
        );
        let balls = data.part("3D Entities/Balls.nmo").expect("our balls");
        sim.set_ball_boxes(crate::scene::BallMeshes::find(&balls.level).boxes);
        sim.set_pieces(crate::pieces::Pieces::find(&balls.level));
        sim.level_bonus = loaded.level.settings.level_bonus.unwrap_or(0);
        while sim.tick < run.end && !sim.finished && !sim.game_over {
            sim.step(run.input_at(sim.tick + 1));
        }
        Outcome {
            finished: sim.finished.then_some(sim.tick),
            lives: sim.lives,
            sectors: sim.sector_count(),
        }
    }

    /// How a ball thrown sideways at the edge of a stretch of track of each safety fares (Parts_Test's stretches
    /// beside the way, made by make_test_level.py: along +X from 20 to 100, 12 wide, at z 80, 120, 160, 200 for
    /// railings, gutter, border, none): where it is after four seconds of being thrown at `speed` towards +Z (and
    /// pushed on that way when `push`), and whether it was lost.
    fn thrown(kind: BallKind, stretch: usize, speed: f32, push: bool) -> (glam::Vec3, bool) {
        let dir = shipped().join("Parts_Test");
        let source = Source::Open(dir.clone());
        let data = Data::new(None).for_source(&source);
        let loaded = data.load(&source).expect("the test level loads");
        let data = data.with_parts(&loaded.level);
        let modules = crate::scene::load_modules(&loaded.level, &data);
        let mut sim = Sim::with_modules(&loaded.level, kind, &modules, false);
        let z = 80.0 + 40.0 * stretch as f32;
        sim.start_from(0, Some(kind), Some(glam::Vec3::new(60.0, 2.05, z)));
        for _ in 0..30 {
            sim.step(0);
        }
        sim.throw_ball(glam::Vec3::new(0.0, 0.0, speed));
        let mut lost = false;
        for _ in 0..240 {
            sim.step(if push { crate::sim::input::LEFT } else { 0 });
            lost |= sim.is_dying() || sim.ball.position.y < -3.0;
        }
        (sim.ball.position - glam::Vec3::new(0.0, 0.0, z), lost)
    }

    /// The railings and the borders of our tracks are solid: a ball of each kind thrown at them sideways, faster
    /// than it rolls, and pushed on into them stays on the track; the gutter turns back a ball drifting into its
    /// edge; the bare floor keeps nothing (so the throw would have taken the ball off).
    #[test]
    fn borders_and_railings_keep_the_ball_on_the_track() {
        for kind in [BallKind::Wood, BallKind::Stone, BallKind::Paper] {
            for (stretch, name) in [(0, "railings"), (2, "border")] {
                for speed in [12.0, 25.0, 40.0] {
                    let (at, lost) = thrown(kind, stretch, speed, true);
                    assert!(
                        !lost && at.y > 1.0 && at.z.abs() < 6.0,
                        "{kind:?} thrown at {speed} into the {name}: {at:?}, lost {lost}"
                    );
                }
            }
            let (at, lost) = thrown(kind, 1, 6.0, false);
            assert!(
                !lost && at.z.abs() < 3.5,
                "{kind:?} drifting into the gutter: {at:?}"
            );
            let (_, lost) = thrown(kind, 3, 25.0, true);
            assert!(lost, "{kind:?} thrown off the bare floor");
        }
    }

    /// Every one of our levels has its runs, in the three styles, and each finishes the level.
    #[test]
    fn each_of_our_levels_is_finished_by_its_runs() {
        let levels = levels();
        let count = levels.len();
        for dir in levels {
            for style in ["safe", "speed", "collector"] {
                let replay = dir.join("pilot").join(format!("{style}.replay"));
                assert!(replay.is_file(), "{}: no {style} run", dir.display());
                let o = play(&dir, &replay);
                assert!(
                    o.finished.is_some(),
                    "{}: the {style} run does not finish",
                    dir.display()
                );
                assert!(
                    (3..=6).contains(&o.sectors),
                    "{}: {} sectors",
                    dir.display(),
                    o.sectors
                );
                println!(
                    "{} {style}: finished at {:.2} s, {} lives",
                    dir.file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default(),
                    o.finished.unwrap_or(0) as f32 / 60.0,
                    o.lives
                );
            }
        }
        assert!(
            count >= 10,
            "at least ten of our own levels are kept: {count}"
        );
    }
}
