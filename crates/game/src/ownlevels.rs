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
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/levels/drahtseilakt")
    }

    /// Our levels: the shipped folders made from a level description. The ones set aside in `levels-legacy_delme/`
    /// are not checked: they wait to be replaced, and their runs predate the balls remade with own-level-1.
    fn levels() -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = [shipped()]
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

    /// The level kit that made a level (`asset.generator` of its level.gltf): its version, 1 for the kits before the
    /// seamless floors (2026-11-03) wrote none.
    fn kit_version(dir: &Path) -> u32 {
        let text = std::fs::read_to_string(dir.join("level.gltf")).unwrap_or_default();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let generator = json["asset"]["generator"].as_str().unwrap_or("");
        generator
            .strip_prefix("drahtseilakt levelkit.py ")
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(1)
    }

    /// Seams in a level's floors: places where the top of one floor stops right beside the top of another at its
    /// height (a crack, a wedge of nothing between two pieces, a piece's edge running into the middle of another's
    /// face). The tops are the floors' faces turned up (Phys_Floors), their vertices joined where they are within a
    /// thousandth; an edge of the tops that only one face has is the tops' boundary, and there the floor must end:
    /// a little way out from it (0.05 and 0.3, level) there is no other top within 0.02 of its height. A step (a
    /// gutter's rim beside a flat piece, a ramp's foot) is not a seam; a gap the ball jumps is wider than 0.3.
    /// Returns the middles of the edges found.
    fn seams(level: &level::Level) -> Vec<[f32; 3]> {
        use std::collections::HashMap;
        type Tri = [glam::Vec3; 3];
        let mut tops: Vec<Tri> = Vec::new();
        for (_, o) in level.members("Phys_Floors") {
            let Some(mesh) = o.mesh.and_then(|m| level.meshes.get(m)) else {
                continue;
            };
            let m = glam::Mat4::from_cols_array_2d(&o.matrix);
            let n3 = glam::Mat3::from_mat4(m);
            for part in &mesh.parts {
                for t in part.indices.as_chunks::<3>().0 {
                    let idx = [t[0] as usize, t[1] as usize, t[2] as usize];
                    if idx
                        .iter()
                        .any(|&i| i >= mesh.positions.len() || i >= mesh.normals.len())
                    {
                        continue;
                    }
                    let up: f32 = idx
                        .iter()
                        .map(|&i| {
                            (n3 * glam::Vec3::from(mesh.normals[i]))
                                .normalize_or_zero()
                                .y
                        })
                        .sum::<f32>()
                        / 3.0;
                    let p = idx.map(|i| m.transform_point3(glam::Vec3::from(mesh.positions[i])));
                    if up > 0.3 && (p[1] - p[0]).cross(p[2] - p[0]).length() > 1e-6 {
                        tops.push(p);
                    }
                }
            }
        }
        let key = |p: glam::Vec3| ((p * 1000.0).round().as_ivec3()).to_array();
        let mut edges: HashMap<([i32; 3], [i32; 3]), (u32, usize)> = HashMap::new();
        for (k, t) in tops.iter().enumerate() {
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                let (ka, kb) = (key(t[a]), key(t[b]));
                let e = if ka < kb { (ka, kb) } else { (kb, ka) };
                edges.entry(e).or_insert((0, k)).0 += 1;
            }
        }
        // the tops by cells of 4 across the level, for finding a top over a point
        const CELL: f32 = 4.0;
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (k, t) in tops.iter().enumerate() {
            let lo = t.iter().fold(glam::Vec3::splat(f32::MAX), |a, &b| a.min(b));
            let hi = t.iter().fold(glam::Vec3::splat(f32::MIN), |a, &b| a.max(b));
            for x in (lo.x / CELL).floor() as i32..=(hi.x / CELL).floor() as i32 {
                for z in (lo.z / CELL).floor() as i32..=(hi.z / CELL).floor() as i32 {
                    grid.entry((x, z)).or_default().push(k);
                }
            }
        }
        let height_at = |t: &Tri, x: f32, z: f32| -> Option<f32> {
            let (a, b, c) = (t[0], t[1], t[2]);
            let d = (b.x - a.x) * (c.z - a.z) - (c.x - a.x) * (b.z - a.z);
            if d.abs() < 1e-9 {
                return None;
            }
            let u = ((x - a.x) * (c.z - a.z) - (c.x - a.x) * (z - a.z)) / d;
            let v = ((b.x - a.x) * (z - a.z) - (x - a.x) * (b.z - a.z)) / d;
            (u >= -1e-6 && v >= -1e-6 && u + v <= 1.0 + 1e-6)
                .then_some(a.y + u * (b.y - a.y) + v * (c.y - a.y))
        };
        let mut found = Vec::new();
        for (&(ka, kb), &(count, k)) in &edges {
            if count != 1 {
                continue;
            }
            let a = glam::IVec3::from_array(ka).as_vec3() / 1000.0;
            let b = glam::IVec3::from_array(kb).as_vec3() / 1000.0;
            let mid = (a + b) / 2.0;
            let d = glam::Vec3::new(b.x - a.x, 0.0, b.z - a.z);
            if d.length() < 1e-4 {
                continue;
            }
            let mut out = glam::Vec3::new(d.z, 0.0, -d.x).normalize();
            let centre = (tops[k][0] + tops[k][1] + tops[k][2]) / 3.0;
            if (centre - mid).dot(out) > 0.0 {
                out = -out;
            }
            let crack = [0.05f32, 0.3].iter().any(|&r| {
                let p = mid + out * r;
                let cell = ((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32);
                grid.get(&cell).is_some_and(|ts| {
                    ts.iter().any(|&j| {
                        j != k
                            && height_at(&tops[j], p.x, p.z)
                                .is_some_and(|h| (h - mid.y).abs() < 0.02)
                    })
                })
            });
            if crack {
                found.push(mid.to_array());
            }
        }
        found
    }

    /// A floor of two flat pieces: side by side along x (the second from `gap` on), as one mesh with shared
    /// vertices when `gap` is 0.
    fn two_pieces(gap: f32) -> level::Level {
        let quad = |x0: f32, x1: f32| {
            [
                [x0, 0.0, -5.0],
                [x1, 0.0, -5.0],
                [x1, 0.0, 5.0],
                [x0, 0.0, 5.0],
            ]
        };
        let mut positions: Vec<[f32; 3]> = quad(0.0, 10.0).to_vec();
        positions.extend(quad(10.0 + gap, 20.0));
        let mut indices = vec![0, 2, 1, 0, 3, 2, 4, 6, 5, 4, 7, 6];
        if gap == 0.0 {
            // the second piece's near corners are the first's far ones
            indices = vec![0, 2, 1, 0, 3, 2, 1, 6, 5, 1, 2, 6];
        }
        let mesh = level::Mesh {
            name: "Floor".into(),
            normals: vec![[0.0, 1.0, 0.0]; positions.len()],
            uvs: vec![[0.0, 0.0]; positions.len()],
            positions,
            parts: vec![level::Part {
                material: None,
                indices,
            }],
            ..Default::default()
        };
        level::Level {
            meshes: vec![mesh],
            objects: vec![level::Object {
                parent: None,
                name: "Floor_001".into(),
                matrix: glam::Mat4::IDENTITY.to_cols_array_2d(),
                mesh: Some(0),
                visible: true,
                sprite: None,
            }],
            groups: vec![level::Group {
                name: "Phys_Floors".into(),
                members: vec![0],
            }],
            ..Default::default()
        }
    }

    /// The seam finder finds a crack between two pieces and none where they share their edge.
    #[test]
    fn the_seam_finder_sees_a_crack() {
        assert!(seams(&two_pieces(0.0)).is_empty());
        assert!(!seams(&two_pieces(0.02)).is_empty());
        assert!(!seams(&two_pieces(0.2)).is_empty());
        // a gap the ball jumps is not a seam
        assert!(seams(&two_pieces(4.0)).is_empty());
    }

    /// The floors of our levels have no seams: the pieces laid one after another (straight, turning, rising, a
    /// width changing, the start's and the checkpoints' bowls) meet with no crack and no wedge between them.
    /// Levels made by a kit before the seamless one (kit_version 1) are named and passed over until they are made
    /// again.
    #[test]
    fn our_floors_have_no_seams() {
        let mut checked = 0;
        for dir in levels().into_iter().chain([shipped().join("Parts_Test")]) {
            if kit_version(&dir) < 2 {
                println!(
                    "{}: made by an earlier level kit, not checked",
                    dir.display()
                );
                continue;
            }
            let source = Source::Open(dir.clone());
            let data = Data::new(None).for_source(&source);
            let loaded = data.load(&source).expect("the level loads");
            let found = seams(&loaded.level);
            assert!(
                found.is_empty(),
                "{}: {} seams in the floors, e.g. at {:?}",
                dir.display(),
                found.len(),
                &found[..found.len().min(5)]
            );
            checked += 1;
        }
        println!("{checked} levels made by the seamless kit checked");
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
        // (while the owner's picks stand in for our own levels there may be none; the release wants ten again)
        println!("{count} of our own levels finished by their runs");
    }

    /// No loose body of our levels (boxes, loose balls) starts inside the floors (levelkit.py, check_clear): the
    /// physics would shove it out as its sector begins and throw the player's ball touching it far away (the game
    /// lifts one out if it does, sim.rs `rest_out_of`; this checks that none has to be). Measured before that lift,
    /// with the bodies the game makes (the paper ball's hull from its part file).
    #[test]
    fn our_loose_bodies_start_clear_of_the_floors() {
        let mut dirs = levels();
        dirs.push(shipped().join("Parts_Test"));
        let mut sunk = Vec::new();
        for dir in dirs {
            let source = Source::Open(dir.clone());
            let data = Data::new(None).for_source(&source);
            let loaded = data.load(&source).expect("the level loads");
            let data = data.with_parts(&loaded.level);
            let modules = crate::scene::load_modules(&loaded.level, &data);
            let sim = Sim::with_modules(&loaded.level, BallKind::Wood, &modules, false);
            for &(i, depth, _) in &sim.start_depths {
                if depth > 0.05 {
                    let name = loaded
                        .level
                        .objects
                        .get(i)
                        .map(|o| o.name.clone())
                        .unwrap_or_default();
                    sunk.push(format!("{}: {name} {depth:.2} deep", dir.display()));
                }
            }
        }
        assert!(
            sunk.is_empty(),
            "loose bodies made inside the floors:\n{}",
            sunk.join("\n")
        );
    }

    /// The original's levels (with the player's copy; skipped without OPENBALLANCE_DATA) place no loose body deeper
    /// than sim::SUNK_DEPTH in their floors (the deepest are level 10's paper balls, 0.031), so the lift of a sunk
    /// one never moves any of theirs and their runs play as before.
    #[test]
    fn the_original_never_starts_a_loose_body_sunk() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA")
            .and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        for n in 1..=13 {
            let Ok(loaded) = data.load(&Source::Original(n)) else {
                continue;
            };
            let modules = crate::scene::load_modules(&loaded.level, &data);
            let sim = Sim::with_modules(&loaded.level, BallKind::Wood, &modules, n == 1);
            for &(i, depth, lift) in &sim.start_depths {
                let name = loaded
                    .level
                    .objects
                    .get(i)
                    .map(|o| o.name.as_str())
                    .unwrap_or_default();
                assert!(
                    depth <= crate::sim::SUNK_DEPTH && lift == 0.0,
                    "level {n}: {name} {depth:.3} deep"
                );
            }
        }
    }
}
