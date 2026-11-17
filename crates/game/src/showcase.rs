// Checks of the showcase level shipped with the game (assets/levels/drahtseilakt/00-Showcase, made by
// assets/own/make/make_synthetic_level.py; docs/drahtseilakt/levels.md, "The showcase level"): it holds every one of
// our parts, loads with no copy of the original, carries its own behaviour graphs, and each element does its job in
// the game's physics (the level's checks.txt: short runs with the arrows through each element, a ball of the right
// kind and of the wrong one). The built level is not in git (.gitignore: levels stay on disk until the release);
// where it has not been built the checks say so and pass.

#[cfg(test)]
mod tests {
    use crate::assets::{Data, Source};
    use crate::sim::{BallKind, Sim};
    use glam::Vec3;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    const NAME: &str = "00-Showcase";

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

    /// The keys that roll the ball toward `goal` (on the ground plane) at up to `speed`: the velocity it should
    /// have, less the one it has, along the two axes the arrows push.
    /// `from`: where the leg began; the ball is steered back onto the line from there to the goal.
    fn keys_toward(sim: &Sim, from: Vec3, goal: Vec3, speed: f32) -> u8 {
        use crate::sim::input;
        let p = sim.ball.position;
        let mut d = goal - p;
        d.y = 0.0;
        let mut want = if d.length() > 1e-3 {
            d.normalize() * (d.length() * 0.9).min(speed)
        } else {
            Vec3::ZERO
        };
        let mut u = goal - from;
        u.y = 0.0;
        if u.length() > 1.0 {
            let u = u.normalize();
            let mut off = p - from;
            off.y = 0.0;
            want -= (off - u * off.dot(u)) * 1.5;
        }
        let mut v = sim.ball.velocity;
        v.y = 0.0;
        let e = want - v;
        let (f, r) = (sim.camera.push_forward(), sim.camera.push_right());
        let mut keys = 0;
        if e.dot(f) > 0.4 {
            keys |= input::UP;
        } else if e.dot(f) < -0.4 {
            keys |= input::DOWN;
        }
        if e.dot(r) > 0.4 {
            keys |= input::RIGHT;
        } else if e.dot(r) < -0.4 {
            keys |= input::LEFT;
        }
        keys
    }

    fn point(w: &[&str]) -> Option<Vec3> {
        Some(Vec3::new(
            w.first()?.parse().ok()?,
            w.get(1)?.parse().ok()?,
            w.get(2)?.parse().ok()?,
        ))
    }

    /// The body of a module's part: the module by its stand-in's name, the part by its object's.
    fn part(sim: &Sim, level: &level::Level, module: &str, object: &str) -> Option<usize> {
        sim.module_bodies()
            .iter()
            .find(|m| {
                m.object == object
                    && level
                        .objects
                        .get(m.placeholder)
                        .is_some_and(|o| o.name == module)
            })
            .map(|m| m.body)
    }

    /// Runs one check of checks.txt (make_synthetic_level.py, `check`): what failed, and what was measured.
    fn run_check(
        level: &level::Level,
        modules: &crate::scene::Modules,
        balls: &level::Level,
        name: &str,
        steps: &[Vec<&str>],
    ) -> (Vec<String>, Vec<String>) {
        let mut sim = Sim::with_modules(level, BallKind::Wood, modules, false);
        sim.set_ball_boxes(crate::scene::BallMeshes::find(balls).boxes);
        sim.set_pieces(crate::pieces::Pieces::find(balls));
        let mut fails = Vec::new();
        let mut notes = Vec::new();
        // SHOWCASE_TRACE=<check name>: the ball's way through that check, every half second
        let trace = std::env::var("SHOWCASE_TRACE").is_ok_and(|t| t == name);
        let mut start: std::collections::HashMap<(String, String), (Vec3, glam::Quat)> =
            Default::default();
        let died = |sim: &Sim| {
            sim.events
                .iter()
                .any(|(_, e)| matches!(e, crate::sim::Event::Died))
        };
        for w in steps {
            let (verb, args) = (w[0], &w[1..]);
            match verb {
                "ball" => {
                    let kind = BallKind::from_name(args[0]).expect("a ball kind");
                    let p = point(&args[1..]).expect("a point");
                    let sector = crate::sim::sector_near(level, p).expect("its sector");
                    // standing on the floor: the ball's middle 2 over the point
                    sim.start_from(sector, Some(kind), Some(p + Vec3::Y * 2.0));
                    sim.events.clear();
                    for _ in 0..20 {
                        sim.step(0);
                    }
                    // every module part's pose as the check begins
                    for m in sim.module_bodies() {
                        if let (Some(o), Some(s)) =
                            (level.objects.get(m.placeholder), sim.world().state(m.body))
                        {
                            start.insert((o.name.clone(), m.object.clone()), (s.pos, s.rot));
                        }
                    }
                }
                "go" => {
                    let goal = point(args).expect("a point");
                    let speed: f32 = args[3].parse().expect("a speed");
                    let most: u64 = args[4].parse().expect("ticks");
                    let from = sim.ball.position;
                    for _ in 0..most {
                        let mut d = goal - sim.ball.position;
                        d.y = 0.0;
                        if d.length() < 1.2 || sim.is_dying() {
                            break;
                        }
                        let k = keys_toward(&sim, from, goal, speed);
                        sim.step(k);
                        if trace && sim.tick % 30 == 0 {
                            println!("  {} {goal:.1?}: ball {:.1?}", sim.tick, sim.ball.position);
                        }
                    }
                }
                "wait" => {
                    let t: u64 = args[0].parse().expect("ticks");
                    for _ in 0..t {
                        let k = keys_toward(&sim, sim.ball.position, sim.ball.position, 1.0);
                        sim.step(k);
                        if trace && sim.tick % 30 == 0 {
                            println!("  {} wait: ball {:.1?}", sim.tick, sim.ball.position);
                        }
                    }
                }
                "reach" | "away" => {
                    let p = point(args).expect("a point");
                    let r: f32 = args[3].parse().expect("a reach");
                    let d = sim.ball.position.distance(p);
                    notes.push(format!("{verb}: the ball {d:.1} from {p:.1?}"));
                    if (verb == "reach") != (d <= r) {
                        fails.push(format!(
                            "{verb} {p:.1?} within {r}: the ball at {:.1?}, {d:.1} away",
                            sim.ball.position
                        ));
                    }
                }
                "level" => {
                    // the ball rolls over a floor at height U (a fallen deck flush with the way): within 2.5 of
                    // the point seen from above, its bottom within TOL of U
                    let p = point(args).expect("a point");
                    let tol: f32 = args[3].parse().expect("a tolerance");
                    let b = sim.ball.position;
                    let flat = Vec3::new(b.x - p.x, 0.0, b.z - p.z).length();
                    let bottom = b.y - crate::sim::BALL_RADIUS;
                    notes.push(format!(
                        "level: the ball's bottom at {bottom:.3}, {flat:.1} from {p:.1?}"
                    ));
                    if flat > 2.5 || (bottom - p.y).abs() > tol {
                        fails.push(format!(
                            "level {p:.1?} within {tol}: the ball's bottom at {bottom:.3}, {flat:.1} away"
                        ));
                    }
                }
                "kind" => {
                    let want = BallKind::from_name(args[0]).expect("a ball kind");
                    if sim.ball.kind != want {
                        fails.push(format!("the ball is {:?}, not {want:?}", sim.ball.kind));
                    }
                }
                "lost" => {
                    if !died(&sim) {
                        fails.push("no ball was lost".into());
                    }
                }
                "kept" => {
                    if died(&sim) {
                        fails.push(format!("a ball was lost (at {:.1?})", sim.ball.position));
                    }
                }
                "turned" | "still" | "moved" => {
                    let key = (args[0].to_string(), args[1].to_string());
                    let limit: f32 = args[2].parse().expect("a limit");
                    let Some(b) = part(&sim, level, args[0], args[1]) else {
                        fails.push(format!("no part {} of {}", args[1], args[0]));
                        continue;
                    };
                    let (Some(s), Some(&(p0, q0))) = (sim.world().state(b), start.get(&key)) else {
                        fails.push(format!("{} of {}: no state", args[1], args[0]));
                        continue;
                    };
                    let angle = (q0.inverse() * s.rot).to_axis_angle().1.to_degrees();
                    let angle = if angle > 180.0 { 360.0 - angle } else { angle };
                    let moved = s.pos.distance(p0);
                    notes.push(format!(
                        "{} {}: turned {angle:.0} deg, moved {moved:.1}",
                        args[0], args[1]
                    ));
                    let ok = match verb {
                        "turned" => angle >= limit,
                        "still" => angle < limit,
                        _ => moved >= limit,
                    };
                    if !ok {
                        fails.push(format!(
                            "{verb} {} {} {limit}: turned {angle:.0} deg, moved {moved:.1}",
                            args[0], args[1]
                        ));
                    }
                }
                "lies" => {
                    // the part within R of a point (a crate knocked out that must stay on its floor)
                    let Some(b) = part(&sim, level, args[0], args[1]) else {
                        fails.push(format!("no part {} of {}", args[1], args[0]));
                        continue;
                    };
                    let (Some(s), Some(p)) = (sim.world().state(b), point(&args[2..])) else {
                        fails.push(format!("{} of {}: no state", args[1], args[0]));
                        continue;
                    };
                    let r: f32 = args[5].parse().expect("a reach");
                    let d = s.pos.distance(p);
                    notes.push(format!("{} {}: lies {d:.1} from {p:.1?}", args[0], args[1]));
                    if d > r {
                        fails.push(format!(
                            "lies {} {} within {r} of {p:.1?}: at {:.1?}",
                            args[0], args[1], s.pos
                        ));
                    }
                }
                _ => fails.push(format!("unknown step {verb}")),
            }
        }
        (fails, notes)
    }

    /// Every element of the classic part does its job in the game's physics, as the level's checks.txt says
    /// (make_synthetic_level.py writes it from its `check` lines): the bridges tip under the right ball and reach the
    /// far edge, the flap wheel turns, the pushers and the crate clear the way, the fans lift the paper ball up their
    /// steps, the troughs carry only the paper ball, the spiral is ridden down. Each check begins with the ball at
    /// rest at a point and rolls it through its points with the arrows, as a player would.
    #[test]
    fn the_showcase_elements_do_their_jobs() {
        let Some(dir) = built() else { return };
        let text = std::fs::read_to_string(dir.join("checks.txt")).expect("checks.txt");
        let source = Source::Open(dir);
        // SHOWCASE_COPY_PARTS=<the player's copy>: the same checks with the copy's parts in place of ours, to tell what
        // our part does differently from what the layout does
        let copy = std::env::var_os("SHOWCASE_COPY_PARTS").map(PathBuf::from);
        let data = Data::new(copy.clone()).for_source(&source);
        let mut level = data.load(&source).expect("the showcase level loads").level;
        if copy.is_some() {
            level.settings.own_parts = false;
        }
        let data = data.with_parts(&level);
        let modules = crate::scene::load_modules(&level, &data);
        let balls = data.part("3D Entities/Balls.nmo").expect("our balls");
        let mut checks: Vec<(String, Vec<Vec<&str>>)> = Vec::new();
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let w: Vec<&str> = line.split_whitespace().collect();
            if w[0] == "check" {
                checks.push((w[1..].join(" "), Vec::new()));
            } else if let Some(c) = checks.last_mut() {
                c.1.push(w);
            }
        }
        assert!(checks.len() >= 10, "{} checks", checks.len());
        let mut failed = Vec::new();
        // SHOWCASE_ONLY=<prefix>: only the checks whose names begin so (while one element is being built)
        let only = std::env::var("SHOWCASE_ONLY").ok();
        for (name, steps) in &checks {
            if only.as_deref().is_some_and(|o| !name.starts_with(o)) {
                continue;
            }
            let (fails, notes) = run_check(&level, &modules, &balls.level, name, steps);
            println!(
                "{name}: {} | {}",
                if fails.is_empty() { "ok" } else { "FAILED" },
                notes.join("; ")
            );
            // a check named pending-... is run and reported but not yet required (an element that does not yet do
            // its job in our physics; STATUS.md lists them)
            if !fails.is_empty() && !name.starts_with("pending-") {
                failed.push(format!("{name}: {}", fails.join("; ")));
            }
        }
        assert!(failed.is_empty(), "{}", failed.join("\n"));
    }
}
