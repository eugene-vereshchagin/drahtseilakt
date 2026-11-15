// Checks of the showcase level shipped with the game (assets/levels/drahtseilakt/00_Синтетический_уровень, made by
// assets/own/make/make_synthetic_level.py; docs/drahtseilakt/levels.md, "The showcase level"): it holds every one of
// our parts, loads with no copy of the original, carries its own behaviour graphs, and the run kept for it
// (hints/runs/Level_00-synthetic.replay, the test player's) plays to the balloon. The built level is not in git
// (.gitignore: levels stay on disk until the release); where it has not been built the checks say so and pass.

#[cfg(test)]
mod tests {
    use crate::assets::{Data, Source};
    use crate::replay::Replay;
    use crate::sim::{BallKind, Sim};
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    const NAME: &str = "00_Синтетический_уровень";

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The level's folder, when it has been built here.
    fn built() -> Option<PathBuf> {
        let dir = root().join("assets/levels/drahtseilakt").join(NAME);
        if dir.join("level.gltf").is_file() {
            Some(dir)
        } else {
            println!(
                "{}: not built here (make_synthetic_level.py), not checked",
                dir.display()
            );
            None
        }
    }

    /// Every part of ours (a folder of assets/own/3D Entities/PH) stands in the level, every part the level names
    /// is ours, and it has the sectors and the scripts its description says.
    #[test]
    fn the_showcase_level_holds_every_part_of_ours() {
        let Some(dir) = built() else { return };
        let source = Source::Open(dir);
        let data = Data::new(None).for_source(&source);
        let level = data.load(&source).expect("the showcase level loads").level;
        let data = data.with_parts(&level);
        assert!(data.own_parts, "made for our parts");
        let modules = crate::scene::load_modules(&level, &data);
        let named: BTreeSet<String> = (0..level.objects.len())
            .filter_map(|i| crate::scene::module_of(&level, i).map(str::to_owned))
            .filter(|n| !n.starts_with("PR_"))
            .collect();
        let missing: Vec<&String> = named.iter().filter(|n| modules.get(n).is_none()).collect();
        assert!(
            missing.is_empty(),
            "parts not found among ours: {missing:?}"
        );
        let ours: BTreeSet<String> = std::fs::read_dir(root().join("assets/own/3D Entities/PH"))
            .expect("our parts")
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(ours.len() >= 28, "our parts: {}", ours.len());
        // (the start, the checkpoints and the extra points are named by their marker groups, not by their parts:
        // the sectors counted below and the run's points stand for them)
        let by_group = ["PS_FourFlames", "PC_TwoFlames", "P_Extra_Point"];
        let absent: Vec<&String> = ours
            .difference(&named)
            .filter(|n| !by_group.contains(&n.as_str()))
            .collect();
        assert!(
            absent.is_empty(),
            "parts of ours not in the showcase: {absent:?}"
        );
        let sim = Sim::with_modules(&level, BallKind::Wood, &modules, false);
        assert!(sim.sector_count() >= 6, "{} sectors", sim.sector_count());
        assert!(
            level
                .scripts
                .as_ref()
                .is_some_and(|g| !g.scripts.is_empty()),
            "the community part's behaviour graphs"
        );
        assert_eq!(level.sky.as_deref(), Some("Mountains"));
    }

    /// The run kept for the level was recorded on it as it is built here and finishes it.
    #[test]
    fn the_showcase_level_is_finished_by_its_run() {
        let Some(dir) = built() else { return };
        let replay = root().join("hints/runs/Level_00-synthetic.replay");
        let source = Source::Open(dir);
        let data = Data::new(None).for_source(&source);
        let loaded = data.load(&source).expect("the showcase level loads");
        let data = data.with_parts(&loaded.level);
        let text = std::fs::read_to_string(&replay).expect("the run reads");
        let run = Replay::parse(&text).expect("the run parses");
        assert_eq!(run.level_hash, loaded.hash, "recorded on this level");
        assert!(run.start.is_none(), "a run from the level's start");
        let modules = crate::scene::load_modules(&loaded.level, &data);
        let mut sim = Sim::with_rules(
            &loaded.level,
            BallKind::Wood,
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
        assert!(
            sim.finished,
            "not finished: the ball at {:?}, sector {}",
            sim.ball.position,
            sim.sector + 1
        );
        println!(
            "finished at {:.2} s, {} lives",
            sim.tick as f32 / 60.0,
            sim.lives
        );
    }
}
