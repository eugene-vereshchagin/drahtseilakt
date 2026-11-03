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

    /// Our levels: the shipped folders made from a level description.
    fn levels() -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(shipped())
            .map(|r| {
                r.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.join("level.lvl").is_file())
                    .collect()
            })
            .unwrap_or_default();
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
        let mut sim = Sim::with_modules(
            &loaded.level,
            ball,
            &modules,
            loaded.level.settings.tutorial,
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
            "at least ten of our own levels ship with the game: {count}"
        );
    }
}
