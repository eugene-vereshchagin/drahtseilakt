// The test player's runs and the video tracking, driven through the game's library: finding the ball's way in a
// video's frames (--locate) and drawing a replay's frames to check it (--frames), following a player's way
// (--follow, --then), the autopilot's own way (--plan, the hints, the zones and the learned table), its run tick by
// tick with its stuck reports and rewinds, headless or in the window (watch.rs), and the level statistics' runs.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use crate::Options;
use crate::app::{Game, Renderer, clock, device, level_stem, remake_of, screenshot, upload};
use crate::assets::Data;
use crate::replay::Replay;
use crate::scene::{self, Snapshot};
use crate::sim::Sim;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Plays the replay and draws `locate::RATE` frames of it a second (640 × 480 with the in-game interface, as a
/// video of the original would show it) into `dir` as s0001.jpg, ..., with `truth.txt`: each frame's number, the
/// ball's place and the view's quarter. A video with a known answer, to check `--locate` with.
pub fn frames_run(game: &mut Game, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let end = game
        .replay_in
        .as_ref()
        .map(|r| r.end)
        .ok_or("--frames needs --replay")?;
    let (_, device, queue) = pollster::block_on(device(None))?;
    let mut r = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
    upload(&mut r, game);
    let off = crate::pilot::locate::Offscreen::sized(&r, 640, 480);
    let mut truth = String::from("# frame x y z quarter\n");
    let mut k = 0;
    while game.sim.tick < end && !game.sim.game_over {
        game.tick(0);
        if !game
            .sim
            .tick
            .is_multiple_of(u64::from(60 / crate::pilot::locate::RATE))
        {
            continue;
        }
        k += 1;
        let snap = Snapshot::of(&game.sim);
        let mut inst = scene::instances(
            &game.placements,
            &game.sim.taken,
            &game.sim.unseen,
            &game.sim.world().bodies,
            &game.ball_meshes,
            game.trafo.as_ref().map(|t| &t.1),
            (&game.sim.extra_views(), game.extra_model.as_ref()),
            &snap,
            &game.sim.pieces_view(),
            game.sim.script_entities(),
        );
        if snap.kind.is_some() {
            scene::shadows(&game.shadow_receivers, snap.ball, &mut inst);
        }
        let screen = crate::ui::Screen { w: 640.0, h: 480.0 };
        let quads = crate::hud::interface(&game.sim, game.font.as_ref(), screen);
        if let Some(sc) = &game.sim.scripts {
            r.sync_materials(scene::LEVEL_MODEL, &sc.mats);
        }
        r.apply_script_sky(
            scene::LEVEL_MODEL,
            game.sim.script_env().sky.as_ref(),
            snap.eye,
            &mut inst,
        );
        let px = off
            .pixels(&mut r, &scene::view(&snap), &inst, &quads)
            .ok_or("the frame could not be read back")?;
        let img = image::RgbaImage::from_raw(640, 480, px).ok_or("frame size")?;
        let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
        rgb.save(dir.join(format!("s{k:04}.jpg")))
            .map_err(|e| e.to_string())?;
        let b = game.sim.ball.position;
        truth.push_str(&format!(
            "{k} {:.2} {:.2} {:.2} {}\n",
            b.x, b.y, b.z, game.sim.camera.quarter
        ));
    }
    std::fs::write(dir.join("truth.txt"), truth).map_err(|e| e.to_string())?;
    println!("{k} frames in {}", dir.display());
    Ok(())
}

/// Finds the ball's way in a video's frames (`locate::RATE` a second) and writes it as a track: per frame its
/// number, the ball's place, the view's quarter and how alike the best picture was.
pub fn locate_run(game: &mut Game, o: &Options, dir: &Path) -> Result<(), String> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('s') && n.ends_with(".jpg"))
        })
        .collect();
    // in the frames' order (s0001.jpg, ..., s10000.jpg: by number, not by name)
    let number =
        |p: &PathBuf| -> Option<u64> { p.file_stem()?.to_str()?.strip_prefix('s')?.parse().ok() };
    names.retain(|p| number(p).is_some());
    names.sort_by_key(|p| number(p));
    if let Some(n) = o.ticks {
        names.truncate(n as usize);
    }
    let frames: Vec<Vec<f32>> = names
        .iter()
        .map(|p| {
            crate::pilot::locate::frame_edges(p)
                .ok_or_else(|| format!("{}: not a picture", p.display()))
        })
        .collect::<Result<_, _>>()?;
    println!("{} frames", frames.len());
    let start = game.sim.ball.position;
    let route = crate::pilot::autopilot::Route::find(
        &game.level.level,
        &game.modules,
        &game.sim.updrafts(),
        start,
        &[],
    );
    let floor: Vec<glam::Vec3> = route.floor_points().collect();
    let (_, device, queue) = pollster::block_on(device(None))?;
    let mut r = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
    upload(&mut r, game);
    let off = crate::pilot::locate::Offscreen::new(&r);
    let base = Snapshot::of(&game.sim);
    let mut draw = |view: &crate::render::View, ball: glam::Vec3| {
        let snap = Snapshot {
            ball,
            eye: view.eye,
            target: view.target,
            ..base
        };
        let inst = scene::instances(
            &game.placements,
            &game.sim.taken,
            &game.sim.unseen,
            &game.sim.world().bodies,
            &game.ball_meshes,
            None,
            (&[], None),
            &snap,
            &game.sim.pieces_view(),
            game.sim.script_entities(),
        );
        off.edges(&mut r, view, &inst)
    };
    let quarter = game.sim.camera.quarter;
    let fixes = crate::pilot::locate::track(&frames, &floor, start, quarter, &mut draw)?;
    let mut text = format!("{}\n", crate::pilot::locate::TRACK_HEAD);
    for (k, f) in fixes.iter().enumerate() {
        let (b, q, s) = (f.ball, f.quarter, f.score);
        text.push_str(&format!(
            "{} {:.2} {:.2} {:.2} {q} {s:.3}\n",
            k + 1,
            b.x,
            b.y,
            b.z
        ));
    }
    match &o.record {
        Some(p) => std::fs::write(p, &text).map_err(|e| format!("{}: {e}", p.display()))?,
        None => print!("{text}"),
    }
    Ok(())
}

/// The test player follows a track `--locate` found in a video: the places it was sure of (a good likeness, not
/// far from the frame before), in order, are its way, without the loops of it (the player went round, or waited by a
/// tutorial's text) unless a loop went to a loose object or a module's moving part (to push a box: the box moved is
/// part of the way). Where the way passes a transformer of another kind than the ball's, the ball is taken to have
/// gone into it (a gate of the way, which no loop is cut across).
fn follow_track(o: &Options, data: &Data, text: &str) -> Result<(), String> {
    let mut own = o.clone();
    own.follow = None;
    let game = Game::new(&own, data)?;
    let trafos: Vec<_> = game.sim.trafos().collect();
    // where the loose objects (boxes, balls) and the modules' moving parts start, and how near the ball comes to
    // one to move it
    const PUSH: f32 = 8.0;
    // (the level's objects where they start: the bodies of later sectors are not in the physics yet)
    // (a module's part is placed by the module, not a level object: where its body is made, or else the module's)
    let objects = &game.level.level.objects;
    let at = |o: usize| {
        objects
            .get(o)
            .map(|o| glam::Vec4::from(o.matrix[3]).truncate())
    };
    let parts = game.sim.module_bodies().iter().filter_map(|m| {
        game.sim
            .world()
            .state(m.body)
            .map(|s| s.pos)
            .or_else(|| at(m.placeholder))
    });
    let loose: Vec<glam::Vec3> = game
        .sim
        .loose()
        .iter()
        .filter_map(|l| at(l.0))
        .chain(parts)
        .collect();
    let mut kind = game.sim.ball.kind;
    let mut path: Vec<glam::Vec3> = Vec::new();
    let mut gates: Vec<crate::pilot::autopilot::Gate> = Vec::new();
    let mut last: Option<glam::Vec3> = None;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let v: Option<Vec<f32>> = line.split_whitespace().map(|x| x.parse().ok()).collect();
        let Some([_, x, y, z, _, likeness]) =
            v.as_deref().and_then(|v| <[f32; 6]>::try_from(v).ok())
        else {
            return Err(format!("not a track line: {line}"));
        };
        let ball = glam::Vec3::new(x, y, z);
        let floor = ball - glam::Vec3::Y * crate::sim::BALL_RADIUS;
        // (a likeness that is not a number is not sure either)
        if likeness.is_nan() || likeness < crate::pilot::locate::SURE || !floor.is_finite() {
            continue;
        }
        // a slip: far from the frame before (the next frames, if they go on from here, are taken again)
        let slip = last.is_some_and(|l: glam::Vec3| l.distance(floor) > crate::pilot::locate::JUMP);
        last = Some(floor);
        if slip || path.last().is_some_and(|p| p.distance(floor) < 1.5) {
            continue;
        }
        let kept = gates.last().map_or(0, |g| g.0);
        if let Some(i) = path
            .iter()
            .skip(kept)
            .rposition(|p| p.distance(floor) < 2.5)
        {
            // a loop to a loose object (a box to push, say) did something there: it stays
            let pushed = |p: &glam::Vec3| loose.iter().any(|o| o.distance(*p) < PUSH);
            if !path.get(kept + i..).unwrap_or_default().iter().any(pushed) {
                // (the last gate's own point stays)
                let keep = if gates.is_empty() { 0 } else { kept + 1 };
                path.truncate((kept + i).max(keep));
            }
        }
        path.push(floor);
        let trafo = trafos
            .iter()
            .find(|t| t.1 != kind && t.0.distance(ball) < crate::sim::TRAFO_REACH);
        if let Some(&(q, k)) = trafo {
            gates.push((path.len() - 1, q, k));
            kind = k;
        }
    }
    // the way between the track's places is found over the floors (a frame or a lost stretch may skip a corner,
    // and a straight line from one place to the next would go through the wall there): the places, one every
    // `STAGE`, and the gates' are its stages
    const STAGE: f32 = 6.0;
    let mut stages: Vec<glam::Vec3> = Vec::new();
    let mut gate_stages: Vec<(usize, glam::Vec3, crate::sim::BallKind)> = Vec::new();
    for (i, &p) in path.iter().enumerate() {
        let gate = gates.iter().find(|g| g.0 == i);
        // (by a loose object every place is a stage: how far the player went to push it matters)
        let near_loose = loose.iter().any(|o| o.distance(p) < PUSH);
        let apart = stages
            .last()
            .is_none_or(|l| l.distance(p) >= if near_loose { 1.5 } else { STAGE });
        if gate.is_some() || i + 1 == path.len() || apart {
            stages.push(p);
        }
        if let Some(&(_, q, kind)) = gate {
            gate_stages.push((stages.len() - 1, q, kind));
        }
    }
    // a stage the next one comes back from (the track jittered along a straight, or the player overshot and came
    // back) is no stage; a gate's is kept, and one by a loose object (the player went to push it)
    let mut k = 1;
    while k + 1 < stages.len() {
        let (a, b, c) = (stages[k - 1], stages[k], stages[k + 1]);
        let spike = a.distance(c) < 0.5 * a.distance(b).max(b.distance(c));
        let pushing = loose.iter().any(|o| o.distance(b) < PUSH);
        if spike && !pushing && gate_stages.iter().all(|g| g.0 != k) {
            stages.remove(k);
            for g in gate_stages.iter_mut().filter(|g| g.0 > k) {
                g.0 -= 1;
            }
            k = k.saturating_sub(1).max(1);
        } else {
            k += 1;
        }
    }
    let start = game.sim.ball.position;
    let fans = game.sim.updrafts();
    let route = crate::pilot::autopilot::Route::through(
        &game.level.level,
        &game.modules,
        &fans,
        start,
        &stages,
    );
    let gates: Vec<crate::pilot::autopilot::Gate> = gate_stages
        .iter()
        .filter_map(|&(k, q, kind)| route.marks.get(k).map(|&m| (m, q, kind)))
        .collect();
    let (places, ways) = (path.len(), route.points.len());
    println!(
        "track: {places} places, {} stages, way {ways} points, transformers {gates:?}",
        stages.len()
    );
    if std::env::var_os("AUTOPILOT_FLOOR").is_some() {
        for p in route.floor_points() {
            eprintln!("floor {:.1} {:.1} {:.1}", p.x, p.y, p.z);
        }
    }
    if std::env::var_os("AUTOPILOT_DEBUG").is_some() {
        for (i, p) in route.points.iter().enumerate() {
            eprintln!("way {i} {:.1} {:.1} {:.1}", p.x, p.y, p.z);
        }
    }
    pilot_run(
        game,
        data,
        &own,
        crate::pilot::autopilot::Pilot::new(route, gates),
    )
}

/// A run that did not finish is followed this many way points (and ticks of the player's) past the player's last
/// transformer in its last sector.
const LAST_GATE_ON: usize = 3;
const LAST_GATE_TICKS: usize = 60;

/// The followed player's keys this long before a lost ball are not followed (four seconds).
const DOOMED_TICKS: usize = 240;

/// A watched player's run: the replay, the ball's way (and the sector of each of its points), the transformers it
/// went into (gates) and the player's ticks.
struct Watched {
    run: Replay,
    path: Vec<glam::Vec3>,
    sectors: Vec<usize>,
    gates: Vec<crate::pilot::autopilot::Gate>,
    demo: crate::pilot::autopilot::Demo,
    /// Whether the run finished the level, and where on its way it lost each ball.
    finished: bool,
    deaths: Vec<usize>,
}

/// The test player follows a player's run: the ball's way in that replay (where it rolled, in order, every 1.5;
/// not while a ball is being lost) is its way, the transformers it went into its gates, and it begins where the run
/// began. Each --then run (one begun at a sector) is followed from its sector on instead.
pub fn follow_run(o: &Options, data: &Data, follow: &Path) -> Result<(), String> {
    let text = crate::files::read_utf8(follow, crate::files::MAX_REPLAY_FILE)?;
    if text.starts_with(crate::pilot::locate::TRACK_HEAD) {
        return follow_track(o, data, &text);
    }
    let mut w = watch_run(o, data, follow, &text)?;
    for more in &o.then {
        let text = crate::files::read_utf8(more, crate::files::MAX_REPLAY_FILE)?;
        let m = watch_run(o, data, more, &text)?;
        let Some(from) = m.run.start.as_ref().map(|st| st.sector.saturating_sub(1)) else {
            return Err(format!(
                "{}: --then needs a run begun at a sector",
                more.display()
            ));
        };
        // the followed way up to that sector, then the other run's
        let cut = w
            .sectors
            .iter()
            .position(|&s| s >= from)
            .unwrap_or(w.path.len());
        w.path.truncate(cut);
        w.sectors.truncate(cut);
        w.gates.retain(|g| g.0 < cut);
        w.gates
            .extend(m.gates.iter().map(|g| (g.0 + cut, g.1, g.2)));
        w.path.extend(m.path);
        w.sectors.extend(m.sectors);
        let ticks = w
            .demo
            .ticks
            .iter()
            .position(|t| t.sector >= from)
            .unwrap_or(w.demo.ticks.len());
        w.demo.ticks.truncate(ticks);
        w.demo.ticks.extend(m.demo.ticks);
        w.deaths = m.deaths.iter().map(|d| d + cut).collect();
        w.finished = m.finished;
    }
    println!(
        "followed: {} points, transformers {:?}",
        w.path.len(),
        w.gates
    );
    // the test player's own run begins where the followed one began
    // (with --replay and --from: that run's keys up to the tick, then the test player, as without --follow)
    let mut own = o.clone();
    if o.from.is_none() {
        own.replay = None;
    }
    own.follow = None;
    if let Some(st) = &w.run.start {
        own.sector = Some(st.sector);
        own.ball = st.ball.clone();
        own.at = st.at.map(|[x, y, z]| format!("{x},{y},{z}"));
    }
    let mut game = Game::new(&own, data)?;
    if let Some(tick) = own.from.filter(|_| own.replay.is_some()) {
        game.fast_forward(tick);
    }
    if !w.finished {
        way_on(&game, &own, &mut w)?;
    }
    // (a way partly found by the test player takes the steeper floors that way was found over)
    let route = crate::pilot::autopilot::Route::along(
        &game.level.level,
        &game.modules,
        &game.sim.updrafts(),
        w.path,
        !w.finished,
    );
    let mut pilot = crate::pilot::autopilot::Pilot::new(route, w.gates);
    pilot.demo = Some(std::sync::Arc::new(w.demo));
    pilot_run(game, data, &own, pilot)
}

/// A followed run that did not finish: its way up to its last sector and its try there that got furthest (lost its
/// ball furthest from where it came into that sector), then the way the test player finds by itself from there to
/// the end.
fn way_on(game: &Game, o: &Options, w: &mut Watched) -> Result<(), String> {
    let Some(&last) = w.sectors.last() else {
        return Ok(());
    };
    let first = w.sectors.iter().position(|&s| s == last).unwrap_or(0);
    let entry = w.path.get(first).copied().unwrap_or_default();
    // its tries in that sector, each from where it came in (or a new ball) to where it lost the ball
    let mut ends: Vec<usize> = w
        .deaths
        .iter()
        .copied()
        .filter(|&d| d > first && d <= w.path.len())
        .collect();
    ends.push(w.path.len());
    let tries: Vec<(usize, usize)> = std::iter::once(first)
        .chain(ends.iter().copied())
        .zip(ends.iter().copied())
        .filter(|(a, b)| b > a)
        .collect();
    let far = |t: &(usize, usize)| w.path.get(t.1 - 1).map_or(0.0, |p| p.distance(entry));
    if let Some(&(a, b)) = tries.iter().max_by(|x, y| far(x).total_cmp(&far(y))) {
        let mut path: Vec<glam::Vec3> = w.path.get(..first).unwrap_or_default().to_vec();
        path.extend(w.path.get(a..b).unwrap_or_default());
        let mut gates: Vec<crate::pilot::autopilot::Gate> =
            w.gates.iter().copied().filter(|g| g.0 < first).collect();
        gates.extend(
            w.gates
                .iter()
                .filter(|g| g.0 >= a && g.0 < b)
                .map(|g| (g.0 - a + first, g.1, g.2)),
        );
        w.sectors.truncate(first);
        w.sectors.extend(std::iter::repeat_n(last, b - a));
        w.path = path;
        w.gates = gates;
    }
    // After the player's last transformer in that sector the player was looking for the way (and may have rolled
    // where the ball is stuck: a stone ball into a fan's pit): the way on is found from just past it, and the
    // player's line ends there.
    if let Some(&(g, at, kind)) = w.gates.iter().rev().find(|g| g.0 >= first) {
        let cut = (g + LAST_GATE_ON).min(w.path.len());
        w.path.truncate(cut);
        w.sectors.truncate(cut);
        let made = w
            .demo
            .ticks
            .iter()
            .rposition(|t| t.sector == last && t.kind == kind && t.pos.distance(at) < 6.0);
        if let Some(k) = made {
            for t in w.demo.ticks.iter_mut().skip(k + LAST_GATE_TICKS) {
                t.alive = false;
                t.doomed_in = None;
            }
        }
    }
    let Some(&from) = w.path.last() else {
        return Ok(());
    };
    let kind = w.gates.last().map_or(game.sim.ball.kind, |g| g.2);
    let up = from + glam::Vec3::Y * crate::sim::BALL_RADIUS;
    let took: Vec<glam::Vec3> = w.gates.iter().map(|g| g.1).collect();
    let (route, gates) = found_way(game, o, up, last, &w.path, &took, kind)?;
    let n = w.path.len();
    if std::env::var_os("AUTOPILOT_DEBUG").is_some() {
        for (i, p) in route.points.iter().enumerate() {
            eprintln!("way on {} {:.1} {:.1} {:.1}", i + n, p.x, p.y, p.z);
        }
    }
    println!(
        "the followed run ends unfinished at {from:?} (sector {}): the way on is found, {} points, transformers \
        {gates:?}",
        last + 1,
        route.points.len()
    );
    w.gates.extend(gates.iter().map(|g| (g.0 + n, g.1, g.2)));
    w.sectors
        .extend(std::iter::repeat_n(last, route.points.len()));
    w.path.extend(route.points);
    Ok(())
}

/// The options a player's replay `run` is watched with (followed, or read for a hint's stretch): on the chosen level,
/// or, when it was recorded on the original level the chosen map is a remake of, on that original. A remake holding
/// the original's geometry (the 45-degree and the invisible sets: the same objects at the same places, only the view
/// or the looks changed) is followed by the original's runs; the run itself must be seen where it was played (its
/// keys are in its own view), and only its ball's way and pushes in the world are taken over.
fn watch_options(o: &Options, data: &Data, run: &Replay) -> Options {
    let mut watch = o.clone();
    if matches!(o.source, crate::assets::Source::Original(_)) {
        return watch;
    }
    let Ok(chosen) = data.load(&o.source) else {
        return watch;
    };
    if run.level_hash == chosen.hash || Some(run.level_hash) == chosen.level.settings.source_hash {
        return watch;
    }
    if let Some((n, original)) = remake_of(data, &chosen.level, run, false) {
        println!(
            "{}: the original's run, seen on the original level {n} (the map holds its geometry)",
            run.level_name
        );
        watch.source = original;
    }
    watch
}

/// Watches a player's run (`text`, read from `file`) to its end.
fn watch_run(o: &Options, data: &Data, file: &Path, text: &str) -> Result<Watched, String> {
    let run = Replay::parse(text)?;
    let mut watch = watch_options(o, data, &run);
    watch.replay = Some(file.to_path_buf());
    watch.follow = None;
    let mut seen = Game::new(&watch, data)?;
    let mut path: Vec<glam::Vec3> = Vec::new();
    let mut sectors: Vec<usize> = Vec::new();
    // the tick of each place of the way
    let mut at_tick: Vec<u64> = Vec::new();
    let mut gates: Vec<crate::pilot::autopilot::Gate> = Vec::new();
    let mut demo = crate::pilot::autopilot::Demo::default();
    let mut lost = false;
    let mut deaths = Vec::new();
    while seen.sim.tick < run.end && !seen.sim.finished && !seen.sim.game_over {
        // the player's tick: the ball as it is, and the push the player gives now, in the world
        let s = &seen.sim;
        demo.ticks.push(crate::pilot::autopilot::DemoTick {
            pos: s.ball.position,
            vel: s.ball.velocity,
            kind: s.ball.kind,
            sector: s.sector,
            alive: !s.is_dying() && !lost,
            push: crate::pilot::hints::world_push(
                run.input_at(s.tick + 1),
                crate::pilot::hints::Facing::of(&s.camera),
            ),
            bits: run.input_at(s.tick + 1),
            quarter: crate::pilot::hints::Facing::of(&s.camera),
            doomed_in: None,
        });
        let before = seen.sim.events.len();
        seen.tick(0);
        for (_, e) in seen.sim.events.iter().skip(before) {
            match e {
                crate::sim::Event::Died => {
                    lost = true;
                    // nor its way in them: the ball was rolling off (a try that did not finish is followed to
                    // where it lost the ball, and the way on found from there)
                    let doomed = seen.sim.tick.saturating_sub(DOOMED_TICKS as u64);
                    while at_tick.last().is_some_and(|&t| t >= doomed) {
                        at_tick.pop();
                        path.pop();
                        sectors.pop();
                    }
                    gates.retain(|g| g.0 <= path.len());
                    deaths.push(path.len());
                    // the player's last seconds before losing a ball are no line to follow
                    let n = demo.ticks.len();
                    let from = n.saturating_sub(DOOMED_TICKS);
                    for (i, t) in demo.ticks.iter_mut().enumerate().skip(from) {
                        if t.alive {
                            t.doomed_in = u32::try_from(n - i).ok();
                        }
                        t.alive = false;
                    }
                }
                crate::sim::Event::NewBall => lost = false,
                crate::sim::Event::Transform(k) => {
                    let at = seen.sim.ball.position - glam::Vec3::Y * crate::sim::BALL_RADIUS;
                    gates.push((path.len(), at, *k))
                }
                _ => {}
            }
        }
        let b = &seen.sim.ball;
        // AUTOPILOT_TRACE_FOLLOW: the followed ball's place every half second, to find a stretch for a hint
        if std::env::var_os("AUTOPILOT_TRACE_FOLLOW").is_some() && seen.sim.tick.is_multiple_of(30)
        {
            let p = b.position;
            eprintln!(
                "follow t {} s {} ({:.1} {:.1} {:.1}) q {}",
                seen.sim.tick,
                seen.sim.sector + 1,
                p.x,
                p.y,
                p.z,
                seen.sim.camera.quarter
            );
        }
        let floor = b.position - glam::Vec3::Y * crate::sim::BALL_RADIUS;
        if !lost && b.on_ground && path.last().is_none_or(|p| p.distance(floor) >= 1.5) {
            path.push(floor);
            sectors.push(seen.sim.sector);
            at_tick.push(seen.sim.tick);
        }
    }
    let finished = seen.sim.finished;
    // the player's last seconds before a loss are kept to only where the run got on past that sector afterwards (a
    // loss that served the level), not in the sector a run that did not finish ended in
    let last = seen.sim.sector;
    for t in demo
        .ticks
        .iter_mut()
        .filter(|t| !finished && t.sector >= last)
    {
        t.doomed_in = None;
    }
    let (path, sectors, gates, deaths) = failed_tries_out(path, sectors, gates, deaths, finished);
    Ok(Watched {
        run,
        path,
        sectors,
        gates,
        demo,
        finished,
        deaths,
    })
}

/// A player's way without the tries that lost the ball: each one's stretch from where it came into the sector it was
/// lost in (the ball starts that sector again at its reset point) to where it was lost. The tries in the last sector
/// of a run that did not finish are kept: which got furthest is chosen later (way_on).
fn failed_tries_out(
    path: Vec<glam::Vec3>,
    sectors: Vec<usize>,
    gates: Vec<crate::pilot::autopilot::Gate>,
    deaths: Vec<usize>,
    finished: bool,
) -> (
    Vec<glam::Vec3>,
    Vec<usize>,
    Vec<crate::pilot::autopilot::Gate>,
    Vec<usize>,
) {
    let last = sectors.last().copied().unwrap_or(0);
    let mut keep = vec![true; path.len()];
    let mut try_start = 0;
    for &d in &deaths {
        let d = d.min(path.len());
        let s = d
            .checked_sub(1)
            .and_then(|i| sectors.get(i))
            .copied()
            .unwrap_or(0);
        if finished || s != last {
            let from = (try_start..d)
                .find(|&i| sectors.get(i) == Some(&s))
                .unwrap_or(d);
            keep.iter_mut().take(d).skip(from).for_each(|k| *k = false);
        }
        try_start = d;
    }
    // the new places of the points kept
    let mut at = Vec::with_capacity(path.len() + 1);
    let mut n = 0;
    for &k in &keep {
        at.push(n);
        n += usize::from(k);
    }
    at.push(n);
    let new = |i: usize| at.get(i).copied().unwrap_or(n);
    let gates = gates
        .into_iter()
        .filter(|g| keep.get(g.0).copied().unwrap_or(true))
        .map(|g| (new(g.0), g.1, g.2));
    let in_last = |d: usize| sectors.get(d.saturating_sub(1)) == Some(&last);
    let deaths = deaths
        .iter()
        .filter(|&&d| d >= keep.len() || !finished && in_last(d));
    let deaths: Vec<usize> = deaths.map(|&d| new(d)).collect();
    let path = path
        .into_iter()
        .zip(&keep)
        .filter(|p| *p.1)
        .map(|p| p.0)
        .collect();
    let sectors = sectors
        .into_iter()
        .zip(&keep)
        .filter(|p| *p.1)
        .map(|p| p.0)
        .collect();
    (path, sectors, gates.collect(), deaths)
}

/// The test player plays the level from where the run begins: it saves the run at --record (or as an attempt) and
/// prints how far it got.
pub fn autopilot(game: Game, data: &Data, o: &Options) -> Result<(), String> {
    let start = game.sim.ball.position;
    let (route, gates) = found_way(
        &game,
        o,
        start,
        game.sim.sector,
        &[],
        &[],
        game.sim.ball.kind,
    )?;
    println!(
        "route: {} points, stages at {:?}",
        route.points.len(),
        route.marks
    );
    if std::env::var_os("AUTOPILOT_FLOOR").is_some() {
        for p in route.floor_points() {
            eprintln!("floor {:.1} {:.1} {:.1}", p.x, p.y, p.z);
        }
    }
    if std::env::var_os("AUTOPILOT_DEBUG").is_some() {
        for (i, p) in route.points.iter().enumerate() {
            println!("route {i} {:.2} {:.2} {:.2}", p.x, p.y, p.z);
        }
    }
    println!("transformers on the way: {:?}", gates);
    pilot_run(
        game,
        data,
        o,
        crate::pilot::autopilot::Pilot::new(route, gates),
    )
}

/// The way the test player finds by itself from `start` in `sector` (from 0) through the rest of the level's
/// checkpoints to its end, and the transformers on it (its gates). A plan is followed from the start of the sector,
/// but for the objects of it a player's way `passed` already came by (within 20: those and all before them; a
/// transformer only when the player's ball went into it, at one of `took`).
fn found_way(
    game: &Game,
    o: &Options,
    start: glam::Vec3,
    sector: usize,
    passed: &[glam::Vec3],
    took: &[glam::Vec3],
    kind: crate::sim::BallKind,
) -> Result<
    (
        crate::pilot::autopilot::Route,
        Vec<crate::pilot::autopilot::Gate>,
    ),
    String,
> {
    let level = &game.level.level;
    let stages: Vec<glam::Vec3> = game.sim.stages().into_iter().skip(sector).collect();
    // The transformers the way passes (within 30) are part of the way, in its order: a player rolls into them
    // rather than around them.
    let base = crate::pilot::autopilot::Route::find(
        level,
        &game.modules,
        &game.sim.updrafts(),
        start,
        &stages,
    );
    let mut waypoints: Vec<(usize, glam::Vec3, bool)> = Vec::new();
    for t in game.sim.transformers() {
        let near = base
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    i,
                    glam::Vec3::new(p.x - t.x, 0.0, p.z - t.z).length() + (p.y - t.y).abs(),
                )
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = near.filter(|n| n.1 < 30.0) {
            waypoints.push((i, t, true));
        }
    }
    for (k, &m) in base.marks.iter().enumerate() {
        if let Some(&s) = stages.get(k) {
            waypoints.push((m, s, false));
        }
    }
    waypoints.sort_by_key(|w| w.0);
    // A plan (--plan) names the objects to pass through, in order, as a player who knows the level does: it
    // replaces the found order (from the checkpoint the sector begins at on).
    if let Some(plan) = &o.plan {
        let mut planned = read_plan(plan, level, sector)?;
        // (up to the sector's checkpoint: what comes after it is not passed yet)
        let end = stages
            .first()
            .and_then(|c| planned.iter().position(|p| p.0.distance(*c) < 1.0));
        let taken = |p: &(glam::Vec3, bool)| took.iter().any(|q| q.distance(p.0) < 8.0);
        let came = planned
            .iter()
            .take(end.unwrap_or(planned.len()))
            // (a transformer is passed when the player's ball went into it, not by)
            .rposition(|p| {
                if p.1 {
                    taken(p)
                } else {
                    passed.iter().any(|q| q.distance(p.0) < 20.0)
                }
            });
        if let Some(k) = came {
            // but a transformer after the last one the player took, not taken, is still to go into (the player's
            // ball went by it, or did not get to it before it was lost)
            let last_taken = planned.iter().take(k + 1).rposition(|p| p.1 && taken(p));
            let mut i = 0;
            planned.retain(|p| {
                i += 1;
                i > k + 1 || (p.1 && !taken(p) && last_taken.is_none_or(|t| i > t + 1))
            });
        }
        waypoints = planned
            .into_iter()
            .map(|(p, trafo)| (0, p, trafo))
            .collect();
    }
    let fans = game.sim.updrafts();
    // the transformers' places on the way: their gates
    // (each with the ball its transformer makes: the level's transformer at that place)
    let kind_at = |p: glam::Vec3| {
        game.sim
            .trafos()
            .min_by(|a, b| a.0.distance(p).total_cmp(&b.0.distance(p)))
            .map(|t| t.1)
    };
    let mut tries = 0;
    // the stretches of the way (by goal) found without riding a fan
    let mut grounded: Vec<usize> = Vec::new();
    loop {
        let stages: Vec<glam::Vec3> = waypoints.iter().map(|w| w.1).collect();
        // the transformers not on the way are kept off (rolling over one makes the ball over)
        let shun: Vec<glam::Vec3> = game
            .sim
            .transformers()
            .into_iter()
            .filter(|t| stages.iter().all(|s| s.distance(*t) > 1.0))
            .collect();
        let route = crate::pilot::autopilot::Route::find_shunning(
            level,
            &game.modules,
            &fans,
            start,
            &stages,
            &shun,
            &grounded,
        );
        let gates: Vec<crate::pilot::autopilot::Gate> = route
            .marks
            .iter()
            .zip(&waypoints)
            .filter(|(_, w)| w.2)
            .filter_map(|(&m, w)| Some((m, w.1, kind_at(w.1)?)))
            .collect();
        // Only a paper ball rides an updraft: a lift on the way taken as another ball is preceded by the paper
        // transformer nearest the fan, and the way is found again (a few times at most).
        let ball_at = |i: usize| {
            gates
                .iter()
                .take_while(|g| g.0 <= i)
                .last()
                .map_or(kind, |g| g.2)
        };
        let wrong = route
            .lifts
            .iter()
            .find(|&&(i, _)| ball_at(i) != crate::sim::BallKind::Paper);
        let paper = wrong.and_then(|&(i, f)| {
            let base = fans.get(f)?.0;
            let t = game
                .sim
                .trafos()
                .filter(|t| t.1 == crate::sim::BallKind::Paper)
                .min_by(|a, b| a.0.distance(base).total_cmp(&b.0.distance(base)))?;
            (t.0.distance(base) < FAN_PAPER_NEAR).then_some((i, t.0))
        });
        tries += 1;
        let Some((i, at)) = paper.filter(|_| tries <= FAN_PAPER_TRIES) else {
            return Ok((route, gates));
        };
        let before = route
            .marks
            .iter()
            .position(|&m| m >= i)
            .unwrap_or(waypoints.len());
        if waypoints.iter().any(|w| w.1.distance(at) < 1.0) {
            // the paper transformer is on the way already, after this lift: the stretch the lift is on is found
            // again over the floors only (its goal may be that very transformer, up where the lift would lead)
            if grounded.contains(&before) {
                return Ok((route, gates));
            }
            println!(
                "the way rides a fan at {i} not as paper: that stretch is found again without lifts"
            );
            grounded.push(before);
            continue;
        }
        println!(
            "the way rides a fan at {i} not as paper: into the paper transformer at {at:?} first"
        );
        waypoints.insert(before, (0, at, true));
    }
}

/// How often the way is found again to take a paper transformer before a fan, and how far from the fan one may be.
const FAN_PAPER_TRIES: usize = 4;
const FAN_PAPER_NEAR: f32 = 150.0;

/// A plan: one level object a line (`NAME  # note`, or a place `X Y Z`), in the order the way goes through them; blank lines and
/// `#` comments are skipped. Returns where each is and whether it is a transformer, from the checkpoint sector
/// `sector` (from 0) begins at on (PC_TwoFlames_0N; all of it for the first).
fn read_plan(
    path: &Path,
    level: &level::Level,
    sector: usize,
) -> Result<Vec<(glam::Vec3, bool)>, String> {
    let text = crate::files::read_utf8(path, crate::pilot::hints::MAX_FILE)?;
    let mut out = Vec::new();
    let begins = format!("PC_TwoFlames_{sector:02}");
    for line in text.lines() {
        let name = line
            .split('#')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_end_matches('?')
            .trim();
        if name.is_empty() {
            continue;
        }
        // a place on the way written as `X Y Z` (a rail to take rather than another, a point the way must come by)
        let xyz: Vec<f32> = name
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .filter(|v: &f32| v.is_finite())
            .collect();
        if let [x, y, z] = xyz[..]
            && name.split_whitespace().count() == 3
        {
            out.push((glam::Vec3::new(x, y, z), false));
            continue;
        }
        let i = level
            .objects
            .iter()
            .position(|ob| ob.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("{}: no object {name}", path.display()))?;
        if sector > 0 && name.eq_ignore_ascii_case(&begins) {
            out.clear();
            continue;
        }
        let trafo = level.groups_of(i).any(|g| g.starts_with("P_Trafo"));
        out.push((
            glam::Vec3::from(level.objects.get(i).map_or([0.0; 3], |o| o.position())),
            trafo,
        ));
    }
    Ok(out)
}

/// The level's hint file: --hints, else hints/<level>.txt in the data folder.
pub fn hints_path(game: &Game, o: &Options) -> PathBuf {
    o.hints.clone().unwrap_or_else(|| {
        crate::levels::data_dir()
            .join("hints")
            .join(format!("{}.txt", level_stem(game)))
    })
}

/// The level's zones file (zones.rs): `<level>.zones` beside its hint file.
fn zones_path(game: &Game, o: &Options) -> PathBuf {
    let hints = hints_path(game, o);
    let dir = hints.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    dir.join(format!("{}.zones", level_stem(game)))
}

/// The learned table's file name, beside the hints: one for every level.
const LEARNED: &str = "learned.txt";

/// The learned table, the lines left out said.
pub fn load_learned(path: &Path) -> crate::pilot::learned::Learned {
    let (l, notes) = crate::pilot::learned::load(path);
    for n in notes {
        eprintln!("learned: {n}");
    }
    if !l.is_empty() {
        println!("{} learned rules from {}", l.rules.len(), path.display());
    }
    l
}

/// The zones of the level, the lines left out said.
pub fn load_zones(path: &Path) -> crate::pilot::zones::Zones {
    let (zones, notes) = crate::pilot::zones::load(path);
    for n in notes {
        eprintln!("zones: {n}");
    }
    if !zones.is_empty() {
        println!("{} zones from {}", zones.zones.len(), path.display());
    }
    zones
}

/// The hints of the level, a replay stretch read into the view's quarter at its start and its inputs (the replay is
/// played to its first tick to know the quarter).
fn load_hints(
    game: &Game,
    data: &Data,
    o: &Options,
) -> Result<Vec<crate::pilot::hints::Hint>, String> {
    let path = hints_path(game, o);
    let mut hints = crate::pilot::hints::load(&path)?;
    let dir = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    // each replay the trace steps follow is played once, as far as the furthest of them goes
    let mut reach: std::collections::BTreeMap<PathBuf, u64> = std::collections::BTreeMap::new();
    for step in hints.iter().flat_map(|h| &h.steps) {
        if let crate::pilot::hints::Step::Trace(file, _, to, _) = step {
            let r = reach.entry(dir.join(file)).or_default();
            *r = (*r).max(*to);
        }
    }
    let mut played = std::collections::BTreeMap::new();
    for h in &mut hints {
        for step in &mut h.steps {
            if let crate::pilot::hints::Step::Trace(file, from, to, gain) = step {
                let file = dir.join(&*file);
                let run = match played.entry(file.clone()) {
                    std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
                    std::collections::btree_map::Entry::Vacant(e) => {
                        let until = reach.get(&file).copied().unwrap_or(*to);
                        e.insert(play_track(&file, until, data, o)?)
                    }
                };
                let track = load_track(&file, run, *from, *to, *gain);
                *step = crate::pilot::hints::Step::Track(Box::new(track));
                continue;
            }
            let crate::pilot::hints::Step::Replay(file, from, to) = step else {
                continue;
            };
            let file = dir.join(&*file);
            let bytes = crate::files::read_plain(&file, crate::files::MAX_REPLAY_FILE)?;
            let text =
                String::from_utf8(bytes).map_err(|_| format!("{}: not UTF-8", file.display()))?;
            let run = Replay::parse(&text).map_err(|e| format!("{}: {e}", file.display()))?;
            let mut watch = watch_options(o, data, &run);
            watch.replay = Some(file.clone());
            watch.follow = None;
            watch.autopilot = false;
            let mut seen = Game::new(&watch, data)?;
            while seen.sim.tick < *from && !seen.sim.game_over {
                seen.tick(0);
            }
            // the input of each tick of the stretch, in one pass over the replay's changes
            let mut inputs = Vec::new();
            let mut changes = run.changes.iter().peekable();
            let mut input = 0u8;
            for t in 0..=*to {
                while let Some(&&(_, i)) = changes.peek().filter(|c| c.0 <= t) {
                    input = i;
                    changes.next();
                }
                if t > *from {
                    inputs.push(input);
                }
            }
            *step = crate::pilot::hints::Step::Inputs(
                crate::pilot::hints::Facing::of(&seen.sim.camera),
                inputs,
            );
        }
    }
    if !hints.is_empty() {
        println!("{} hints from {}", hints.len(), path.display());
    }
    Ok(hints)
}

/// A replay played (headless, from its own start) for the trace steps that follow it: at each tick from its first to
/// the furthest a step needs, the ball's place and speed, the push of the player's arrows, whether the view was
/// lifted, the body carrying the ball and the loose bodies' places.
struct Played {
    track: crate::pilot::hints::Track,
    bodies: Vec<Vec<(usize, glam::Vec3)>>,
}

fn play_track(file: &Path, until: u64, data: &Data, o: &Options) -> Result<Played, String> {
    let bytes = crate::files::read_plain(file, crate::files::MAX_REPLAY_FILE)?;
    let text = String::from_utf8(bytes).map_err(|_| format!("{}: not UTF-8", file.display()))?;
    let run = Replay::parse(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut watch = watch_options(o, data, &run);
    watch.replay = Some(file.to_path_buf());
    watch.follow = None;
    watch.autopilot = false;
    let mut seen = Game::new(&watch, data)?;
    let mut played = Played {
        track: crate::pilot::hints::Track::default(),
        bodies: Vec::new(),
    };
    let track = &mut played.track;
    let mut changes = run.changes.iter().peekable();
    let mut input = 0u8;
    for t in 0..until {
        // the input of tick t + 1 is the one the replay plays from the state at tick t (as for a replay step)
        while let Some(&&(_, i)) = changes.peek().filter(|c| c.0 <= t + 1) {
            input = i;
            changes.next();
        }
        if seen.sim.game_over || seen.sim.finished {
            break;
        }
        let cam = &seen.sim.camera;
        track.pos.push(seen.sim.ball.position);
        track.vel.push(seen.sim.ball.velocity);
        track.push.push(crate::pilot::hints::push_of(
            input,
            cam.push_forward(),
            cam.push_right(),
        ));
        track.lift.push(input & crate::sim::input::LIFT != 0);
        track.on.push(crate::pilot::hints::carrier(&seen.sim));
        let world = seen.sim.world();
        played.bodies.push(
            seen.sim
                .loose()
                .iter()
                .filter_map(|&(o, b)| world.state(b).map(|st| (o, st.pos)))
                .collect::<Vec<_>>(),
        );
        seen.tick(0);
    }
    Ok(played)
}

/// A trace step's run: ticks `from` to `to` of the replay `file` as played (`play_track`).
fn load_track(
    file: &Path,
    played: &Played,
    from: u64,
    to: u64,
    gain: f32,
) -> crate::pilot::hints::Track {
    let all = &played.track;
    let at = usize::try_from(from)
        .unwrap_or(usize::MAX)
        .min(all.pos.len());
    let end = usize::try_from(to)
        .unwrap_or(usize::MAX)
        .min(all.pos.len())
        .max(at);
    let mut track = crate::pilot::hints::Track {
        pos: all.pos[at..end].to_vec(),
        vel: all.vel[at..end].to_vec(),
        push: all.push[at..end].to_vec(),
        lift: all.lift[at..end].to_vec(),
        on: all.on[at..end].to_vec(),
        meet: Vec::new(),
        gain,
    };
    let bodies = &played.bodies[at..end];
    crate::pilot::hints::fill_meet(&mut track, bodies);
    if std::env::var_os("TRACE_LOG").is_some() {
        let mut near = std::collections::BTreeMap::new();
        for (i, b) in bodies.iter().enumerate() {
            for &(o, p) in b {
                let d = track.pos.get(i).map_or(f32::INFINITY, |q| q.distance(p));
                let e = near.entry(o).or_insert((f32::INFINITY, 0usize));
                if d < e.0 {
                    *e = (d, i);
                }
            }
        }
        eprintln!("trace bodies nearest the ball (object: distance, tick): {near:?}");
    }
    println!(
        "trace {} {from}..{to}: {} ticks, {} on a body, {} before a body touches the ball ({} loose bodies)",
        file.display(),
        track.pos.len(),
        track.on.iter().filter(|o| o.is_some()).count(),
        track.meet.iter().filter(|o| o.is_some()).count(),
        bodies.first().map_or(0, Vec::len)
    );
    track
}

/// A report of where the test player is stuck, for whoever writes its hints: what it knows of the place, and two
/// pictures, its own view and the place from above.
pub fn stuck_report(
    game: &mut Game,
    data: &Data,
    o: &Options,
    pilot: &crate::pilot::autopilot::Pilot,
    dir: &Path,
) -> Result<(), String> {
    use std::fmt::Write as _;
    let s = &game.sim;
    let b = s.ball.position;
    // a new folder of its own for each report (never one already there, so nothing in it is written through)
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let stem = format!("{}-t{}", level_stem(game), s.tick);
    let mut folder = None;
    for k in 0..100u32 {
        let f = dir.join(if k == 0 {
            stem.clone()
        } else {
            format!("{stem}-{k}")
        });
        match std::fs::create_dir(&f) {
            Ok(()) => {
                folder = Some(f);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", f.display())),
        }
    }
    let folder =
        folder.ok_or_else(|| format!("{}: no new folder for the report", dir.display()))?;
    let mut t = String::new();
    let _ = writeln!(
        t,
        "level {}  tick {} ({})  sector {}  lives {}",
        game.level.name,
        s.tick,
        clock(s.tick),
        s.sector + 1,
        s.lives
    );
    let v = s.ball.velocity;
    let _ = writeln!(
        t,
        "ball {:?} at ({:.2} {:.2} {:.2}) velocity ({:.2} {:.2} {:.2}) on ground {}",
        s.ball.kind, b.x, b.y, b.z, v.x, v.y, v.z, s.ball.on_ground
    );
    let _ = writeln!(
        t,
        "view quarter {} (up pushes along {:?})",
        s.camera.quarter,
        s.camera.push_forward()
    );
    let _ = writeln!(
        t,
        "route {} / {}; ahead:",
        pilot.reached(),
        pilot.route.points.len()
    );
    for p in pilot
        .route
        .points
        .iter()
        .skip(pilot.reached())
        .step_by(5)
        .take(12)
    {
        let _ = writeln!(t, "  ({:.1} {:.1} {:.1})", p.x, p.y, p.z);
    }
    let _ = writeln!(t, "transformers:");
    for (p, k) in s.trafos() {
        if p.distance(b) < 80.0 {
            let _ = writeln!(
                t,
                "  {k:?} at ({:.1} {:.1} {:.1}) {:.1} away",
                p.x,
                p.y,
                p.z,
                p.distance(b)
            );
        }
    }
    let level = &game.level.level;
    let _ = writeln!(t, "loose bodies:");
    for &(obj, body) in s.loose() {
        let (Some(o), Some(st)) = (level.objects.get(obj), s.world().state(body)) else {
            continue;
        };
        if st.pos.distance(b) < 60.0 {
            let p = st.pos;
            let _ = writeln!(
                t,
                "  {} at ({:.1} {:.1} {:.1}) {:.1} away",
                o.name,
                p.x,
                p.y,
                p.z,
                p.distance(b)
            );
        }
    }
    let _ = writeln!(t, "objects within 40:");
    let mut near: Vec<(f32, String)> = level
        .objects
        .iter()
        .enumerate()
        .filter(|(_, ob)| ob.mesh.is_some())
        .filter_map(|(i, ob)| {
            let p = glam::Vec3::from(ob.position());
            let d = p.distance(b);
            (d < 40.0).then(|| {
                let groups: Vec<&str> = level
                    .groups_of(i)
                    .filter(|g| !g.starts_with("Sector"))
                    .take(4)
                    .collect();
                (
                    d,
                    format!(
                        "  {} at ({:.1} {:.1} {:.1}) {:.1} away {:?}",
                        ob.name, p.x, p.y, p.z, d, groups
                    ),
                )
            })
        })
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, l) in near.iter().take(60) {
        let _ = writeln!(t, "{l}");
    }
    std::fs::write(folder.join("report.txt"), &t).map_err(|e| e.to_string())?;
    let mut shot = o.clone();
    shot.ticks = Some(game.sim.tick);
    shot.size = (960, 720);
    shot.look = None;
    screenshot(game, data, &shot, &folder.join("view.png"))?;
    for (name, h) in [("top.png", 45.0f32), ("wide.png", 120.0)] {
        shot.look = Some([b.x, b.y + h, b.z + 0.01, b.x, b.y, b.z]);
        screenshot(game, data, &shot, &folder.join(name))?;
    }
    println!("stuck: report in {}", folder.display());
    Ok(())
}

/// A state the test player goes back to: the run, its record, the player, how far it got and since when, the hint
/// at, how near the edges it rolled.
type Saved = (
    Sim,
    Replay,
    crate::pilot::autopilot::Pilot,
    usize,
    u64,
    Option<(usize, usize)>,
    crate::pilot::edges::EdgeStats,
);

/// A hint marked `retry N` (hints.rs): the run saved just before it fired, to try it again from there when it
/// fails, and how often it has been tried again so far.
struct RetrySave {
    hint: usize,
    /// The step it is tried again from (0: its start, a `retry N` before its `:`; or the step after a `retry` step),
    /// and how often at most.
    from: usize,
    most: u32,
    /// How far across its points may be moved (`Runner::vary`).
    width: f32,
    state: Saved,
    tries: u32,
    /// Whether it has fired (from the save, it fires on the next tick but for a varied opening of the player's).
    fired: bool,
}

/// The test player's run, tick by tick (headless in `pilot_run`, or in the window with --watch): its hints and
/// reports, going back when the ball is lost, giving up when the way stands still.
pub struct PilotRun {
    pub pilot: crate::pilot::autopilot::Pilot,
    pub report: PathBuf,
    limit: u64,
    give_up: u64,
    best: usize,
    since: u64,
    heard: usize,
    hint_at: Option<(usize, usize)>,
    dead: bool,
    pub rewinds: crate::pilot::autopilot::Rewinds<Saved>,
    pub edges: Arc<crate::pilot::edges::Edges>,
    pub near_edges: crate::pilot::edges::EdgeStats,
    /// The last time it went back, for whoever watches: what failed, where, and back to when.
    pub went_back: Option<String>,
    /// The watcher's zones file beside the hints (zones.rs), and the learned table shared by the levels (learned.rs).
    pub zones_path: PathBuf,
    pub learned_path: PathBuf,
    /// The hint marked `retry` that runs now, and the run saved before it.
    retry: Option<RetrySave>,
    /// Each hint marked `retry` that ran: its line, the tries it took and whether the last got through, in order.
    retried: Vec<(usize, u32, bool)>,
}

impl PilotRun {
    pub fn new(
        game: &Game,
        data: &Data,
        o: &Options,
        mut pilot: crate::pilot::autopilot::Pilot,
    ) -> Result<PilotRun, String> {
        pilot.hints = crate::pilot::hints::Runner::new(load_hints(game, data, o)?);
        pilot.style = o.style;
        let zones_path = zones_path(game, o);
        pilot.zones = Arc::new(load_zones(&zones_path));
        let learned_path = zones_path.with_file_name(LEARNED);
        pilot.learned = Arc::new(load_learned(&learned_path));
        let level = &game.level.level;
        pilot.modules = Arc::new(
            (0..level.objects.len())
                .filter_map(|i| {
                    let name = crate::scene::module_of(level, i)?;
                    let ob = level.objects.get(i)?;
                    Some((glam::Vec3::from(ob.position()), name.to_owned()))
                })
                .collect(),
        );
        let level = &game.level.level;
        pilot.bridges = (0..level.objects.len())
            .filter(|&i| crate::scene::module_of(level, i) == Some("P_Modul_25"))
            .filter_map(|i| {
                level
                    .objects
                    .get(i)
                    .map(|ob| glam::Vec3::from(ob.position()))
            })
            .collect();
        if game.start.is_some() {
            pilot.take_up(&game.sim);
        }
        let hint_at = pilot.hints.at();
        // how near the floors' edges the ball rolls, in the run kept (edges.rs): the summary's last line
        let edges = pilot.route.edges.clone();
        Ok(PilotRun {
            pilot,
            // the player's own data folder, not a shared temporary one
            report: o
                .report
                .clone()
                .unwrap_or_else(|| crate::levels::data_dir().join("stuck")),
            limit: o.ticks.unwrap_or(60 * 60 * 20),
            // a dead run: the ball neither falls nor gets on along the way for --give-up seconds (it stands or goes
            // to and fro, not knowing where to go); its report is written and the run stops, for whoever writes the
            // hints. A ball that falls is an ordinary mistake: each fall starts the wait again.
            give_up: o.give_up.saturating_mul(60),
            best: 0,
            since: game.sim.tick,
            heard: game.sim.events.len(),
            hint_at,
            dead: false,
            // the states it goes back to when the ball is lost or the way stands still (autopilot.rs, Rewinds)
            rewinds: crate::pilot::autopilot::Rewinds::new(o.rewinds),
            edges,
            near_edges: crate::pilot::edges::EdgeStats::default(),
            went_back: None,
            zones_path,
            learned_path,
            retry: None,
            retried: Vec::new(),
        })
    }

    /// Whether the run goes on.
    pub fn going(&self, game: &Game) -> bool {
        !self.dead && game.sim.tick < self.limit && !game.sim.finished && !game.sim.game_over
    }

    /// Before the player decides: a state saved to go back to, when one is due.
    pub fn before(&mut self, game: &Game) {
        let s = &game.sim;
        if self.rewinds.due(s) {
            let state = (
                s.clone(),
                game.replay_out.clone(),
                self.pilot.clone(),
                self.best,
                self.since,
                self.hint_at,
                self.near_edges,
            );
            self.rewinds.save(s.tick, state);
        }
        // a hint marked `retry` about to fire: the run as it is before it, to try it again from (not again for the
        // same hint while it is being tried)
        if self.rewinds.enabled()
            && let Some(i) = self.pilot.hints.would_fire(s)
            && self.pilot.hints.retry_of(i) > 0
            && self.retry.as_ref().is_none_or(|r| r.hint != i)
        {
            self.retry = Some(RetrySave {
                hint: i,
                from: 0,
                most: self.pilot.hints.retry_of(i),
                width: crate::pilot::hints::VARY_POINT,
                state: (
                    s.clone(),
                    game.replay_out.clone(),
                    self.pilot.clone(),
                    self.best,
                    self.since,
                    self.hint_at,
                    self.near_edges,
                ),
                tries: 0,
                fired: false,
            });
        }
    }

    /// A hint marked `retry` that failed (`why`) is tried again from the run saved before it, varied, while it has
    /// tries left: whether it went back.
    fn retry_hint(&mut self, game: &mut Game, why: &str) -> bool {
        let Some(r) = self.retry.as_mut() else {
            return false;
        };
        let line = self.pilot.hints.line_of(r.hint);
        if r.tries >= r.most || !self.rewinds.take_one() {
            println!(
                "{} hint (line {line}) {why}: no tries left ({} tries), taken as it comes",
                clock(game.sim.tick),
                r.tries + 1
            );
            self.retried.push((line, r.tries + 1, false));
            self.retry = None;
            return false;
        }
        r.tries += 1;
        r.fired = false;
        let (sim, replay, p, b, sn, h, ne) = r.state.clone();
        let msg = format!(
            "{} hint (line {line}) {why} at ({:.1} {:.1} {:.1}): back to {} (try {} of {}, {} rewinds)",
            clock(game.sim.tick),
            game.sim.ball.position.x,
            game.sim.ball.position.y,
            game.sim.ball.position.z,
            clock(sim.tick),
            r.tries + 1,
            r.most + 1,
            self.rewinds.total()
        );
        println!("{msg}");
        self.went_back = Some(msg);
        // (seeded by the hint's line, the try and the tick: reproducible)
        let seed = (line as u32)
            .wrapping_mul(0x9E37_79B9)
            .wrapping_add(r.tries.wrapping_mul(0x85EB_CA6B))
            ^ (sim.tick as u32).wrapping_mul(0xC2B2_AE35);
        let (hint, from, tries, width) = (r.hint, r.from, r.tries, r.width);
        game.sim = sim;
        game.replay_out = replay;
        let (zones, learned) = (self.pilot.zones.clone(), self.pilot.learned.clone());
        self.pilot = p;
        self.pilot.zones = zones;
        self.pilot.learned = learned;
        self.pilot.hints.vary(hint, from, tries, seed, width);
        (self.best, self.since, self.hint_at, self.near_edges) = (b, game.sim.tick.max(sn), h, ne);
        self.heard = game.sim.events.len();
        // (the states saved on the failed try are not gone back to)
        self.rewinds.drop_after(game.sim.tick);
        true
    }

    /// One tick with input `i` (the player's), and what follows from it: back to a saved state when the ball was
    /// lost or the way stood still, a new ball's place on the way, a report when the run is dead.
    pub fn after(&mut self, game: &mut Game, data: &Data, o: &Options, i: u8) {
        game.tick(i);
        self.near_edges.tick(&self.edges, &game.sim.ball);
        let pilot = &mut self.pilot;
        if pilot.reached() > self.best {
            (self.best, self.since) = (pilot.reached(), game.sim.tick);
        }
        let heard = self.heard;
        // (a transformer rolled over by a slip, making the ball another kind than the way's, counts as a fall)
        let stray = game.sim.events.iter().skip(heard).any(|(_, e)| {
            matches!(e, crate::sim::Event::Transform(k) if pilot.stray_transform(*k, game.sim.ball.position))
        });
        let fell = stray
            || game
                .sim
                .events
                .iter()
                .skip(heard)
                .any(|(_, e)| matches!(e, crate::sim::Event::Died));
        // (a ball lost as the player lost one there is not gone back from: it may be what the level needs)
        let as_the_player = fell && !stray && pilot.lost_as_the_player(&game.sim);
        // lost, or standing still too long: back to a state saved before, varied (a try of its own, reproducible)
        let still =
            game.sim.tick - self.since > self.give_up.min(crate::pilot::autopilot::REWIND_STILL);
        // a hint marked `retry` that failed (its ball lost, stuck or off course while it runs, or its check not
        // met): tried again from its start, varied
        let failed_check = pilot.hints.take_failed();
        // a `retry` step reached: the run saved here, as it is after this tick, to try the rest of the hint again
        // from (the stretch before it got through)
        if let Some((hint, most, width)) = pilot.hints.take_mark()
            && self.rewinds.enabled()
        {
            if let Some(r) = self.retry.take()
                && r.fired
            {
                self.retried
                    .push((self.pilot.hints.line_of(r.hint), r.tries + 1, true));
            }
            let from = self.pilot.hints.at().map_or(0, |a| a.1);
            self.retry = Some(RetrySave {
                hint,
                from,
                most,
                width,
                state: (
                    game.sim.clone(),
                    game.replay_out.clone(),
                    self.pilot.clone(),
                    self.best,
                    self.since,
                    self.hint_at,
                    self.near_edges,
                ),
                tries: 0,
                fired: true,
            });
        }
        let pilot = &mut self.pilot;
        let running_hint = pilot.hints.running_hint();
        let off_course = pilot.hints.off_course();
        if let Some(hint) = self.retry.as_ref().map(|r| r.hint) {
            let running = running_hint == Some(hint);
            let why = if failed_check == Some(hint) {
                Some("check not met")
            } else if running && fell {
                Some("lost")
            } else if running && off_course {
                Some("off course")
            } else if running && still {
                Some("stuck")
            } else {
                None
            };
            if let Some(why) = why {
                if self.retry_hint(game, why) {
                    return;
                }
            } else if let Some(r) = self.retry.as_mut() {
                if running {
                    r.fired = true;
                } else {
                    // (ended and through, or it did not fire after all)
                    if r.fired {
                        let line = self.pilot.hints.line_of(hint);
                        self.retried.push((line, r.tries + 1, true));
                    }
                    self.retry = None;
                }
            }
        }
        let pilot = &mut self.pilot;
        // (standing still: from where it last got on, a state saved there too)
        let failed_at = if as_the_player {
            None
        } else if fell {
            Some(game.sim.tick)
        } else {
            still.then_some(self.since + 60)
        };
        if let Some(at) = failed_at
            && let Some((state, attempt)) = self.rewinds.back(
                at,
                (game.sim.sector, pilot.reached()),
                game.sim.ball.position,
                fell,
            )
        {
            let (sim, replay, p, b, sn, h, ne) = state;
            let (failed_sector, failed_at) = (game.sim.sector, self.pilot.reached());
            let line = format!(
                "{} {} at ({:.1} {:.1} {:.1}): back to {} (try {attempt}, {} rewinds)",
                clock(game.sim.tick),
                if fell { "lost" } else { "stuck" },
                game.sim.ball.position.x,
                game.sim.ball.position.y,
                game.sim.ball.position.z,
                clock(sim.tick),
                self.rewinds.total()
            );
            println!("{line}");
            self.went_back = Some(line);
            game.sim = sim;
            game.replay_out = replay;
            // (the zones as they are now: the watcher may have marked one since that state was saved)
            let (zones, learned) = (self.pilot.zones.clone(), self.pilot.learned.clone());
            self.pilot = p;
            self.pilot.zones = zones;
            self.pilot.learned = learned;
            self.pilot.perturb(&game.sim, attempt);
            self.pilot.fall_back(attempt, failed_sector, failed_at);
            (self.best, self.since, self.hint_at, self.near_edges) =
                (b, game.sim.tick.max(sn), h, ne);
            self.heard = game.sim.events.len();
            return;
        }
        // (a stray transformer taken as it came: back into one making the way's kind, as for a new ball)
        if stray
            || game
                .sim
                .events
                .iter()
                .skip(heard)
                .any(|(_, e)| matches!(e, crate::sim::Event::NewBall))
        {
            pilot.new_ball(&game.sim);
            // back at the reset point: getting on is counted from there again
            self.best = pilot.reached();
        }
        self.heard = game.sim.events.len();
        // a hint going on to its next step gets on too (its steps lead off the way: round, back for a run-up), and so
        // does one playing a recorded run; not one gone off course (its ball fell to a floor beneath a step's point)
        let now_at = pilot.hints.at();
        let hint_on = now_at != self.hint_at && now_at.is_some() && !pilot.hints.off_course();
        if fell || hint_on || pilot.hints.replaying() {
            self.since = game.sim.tick;
        }
        self.hint_at = now_at;
        if game.sim.tick - self.since > self.give_up {
            let b = game.sim.ball.position;
            println!(
                "dead run: ball at ({:.1} {:.1} {:.1}), not falling and the way not on for {} s",
                b.x,
                b.y,
                b.z,
                (game.sim.tick - self.since) / 60
            );
            if let Err(e) = stuck_report(game, data, o, &self.pilot, &self.report) {
                eprintln!("stuck report: {e}");
            }
            self.dead = true;
            return;
        }
        if game.sim.tick.is_multiple_of(600) {
            let b = game.sim.ball.position;
            println!(
                "{} sector {} lives {} route {} / {} ball ({:.1} {:.1} {:.1})",
                clock(game.sim.tick),
                game.sim.sector + 1,
                game.sim.lives,
                pilot.reached(),
                pilot.route.points.len(),
                b.x,
                b.y,
                b.z
            );
        }
    }

    /// The watcher drove since tick `began` and hands the ball back: the way is taken up from here, getting on is
    /// counted from here, and the states before are not gone back to (what the hands did is kept).
    pub fn resume(&mut self, game: &Game, began: u64) {
        self.pilot.resume(&game.sim, began);
        self.rewinds.forget();
        (self.best, self.since) = (self.pilot.reached(), game.sim.tick);
        self.heard = game.sim.events.len();
        self.hint_at = self.pilot.hints.at();
    }

    /// The summary of the run, and its record saved at --record.
    pub fn summary(&self, game: &Game, o: &Options) -> Result<(), String> {
        let s = &game.sim;
        let what = if s.finished {
            "finished"
        } else if s.game_over {
            "game over"
        } else if self.dead {
            "dead run"
        } else {
            "not finished"
        };
        println!(
            "result: {what} after {} ({}), sector {}, lives {}, route {} / {}",
            s.tick,
            clock(s.tick),
            s.sector + 1,
            s.lives,
            self.pilot.reached(),
            self.pilot.route.points.len()
        );
        if !self.retried.is_empty() {
            println!("{}", retries_line(&self.retried));
        }
        println!("{}", self.near_edges.line());
        if let Some(path) = &o.record {
            std::fs::write(path, game.replay_out.to_text())
                .map_err(|e| format!("{}: {e}", path.display()))?;
            println!("saved {}", path.display());
        }
        Ok(())
    }
}

/// The tries of the hints marked `retry` (`PilotRun::retried`), for the summary: `line L: N tries` each.
fn retries_line(retried: &[(usize, u32, bool)]) -> String {
    let each: Vec<String> = retried
        .iter()
        .map(|(l, n, ok)| {
            format!(
                "line {l}: {n} {}{}",
                if *n == 1 { "try" } else { "tries" },
                if *ok { "" } else { " (failed)" }
            )
        })
        .collect();
    format!("retries: {}", each.join(", "))
}

/// Runs the test player to --ticks (or the finish), prints how far it got and saves its run at --record. Where it
/// is stuck it writes a report (--report), and it follows the level's hints. With --watch it plays in the window.
fn pilot_run(
    mut game: Game,
    data: &Data,
    o: &Options,
    pilot: crate::pilot::autopilot::Pilot,
) -> Result<(), String> {
    let mut run = PilotRun::new(&game, data, o, pilot)?;
    if o.watch {
        let watch = super::watch::PilotWatch::new(Box::new(run));
        return crate::app::run_window(game, data.clone(), o, Some(Box::new(watch)));
    }
    while run.going(&game) {
        run.before(&game);
        let i = run.pilot.next(&game.sim);
        run.after(&mut game, data, o, i);
    }
    run.summary(&game, o)
}

/// For the level statistics (levelstats.rs): the level and its modules as a run loads it, and the ball's way
/// through it: a replay (`o.replay`) played without a window, tick by tick (`runway::play`), or else the way the
/// test player would find through the objects of a plan (`plan`), with no run behind it.
pub fn stats_run(
    o: &Options,
    data: &Data,
    plan: Option<&Path>,
) -> Result<crate::runway::Run, String> {
    let Some(plan) = plan else {
        return crate::runway::play(o, data);
    };
    let game = Game::new(o, data)?;
    let mut run = crate::runway::Run {
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
    let start = game.sim.ball.position;
    let mut stages: Vec<glam::Vec3> = read_plan(plan, &game.level.level, 0)?
        .into_iter()
        .map(|p| p.0)
        .collect();
    if let Some(&end) = run.checkpoints.last() {
        stages.push(end);
    }
    let route = crate::pilot::autopilot::Route::find(
        &game.level.level,
        &game.modules,
        &game.sim.updrafts(),
        start,
        &stages,
    );
    run.route = Some(route.points.clone());
    Ok(run)
}

#[cfg(test)]
mod follow_tests {
    use super::failed_tries_out;
    use glam::Vec3;

    /// A run that lost its ball in sector 2 and then got through it: the lost try's stretch in sector 2 goes, the
    /// way through sector 1 and the try that got through stay, and the transformers move with their points.
    #[test]
    fn a_lost_try_is_left_out_of_the_way_followed() {
        let path: Vec<Vec3> = (0..10).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        // sector 1 (0..3), the try lost in sector 2 (3..6), the new ball's try through sector 2 (6..10)
        let sectors = vec![0, 0, 0, 1, 1, 1, 1, 1, 1, 1];
        let gates = vec![
            (4, Vec3::ZERO, crate::sim::BallKind::Paper),
            (7, Vec3::X, crate::sim::BallKind::Stone),
        ];
        let (p, s, g, d) = failed_tries_out(path, sectors, gates, vec![6], true);
        assert_eq!(
            p.iter().map(|v| v.x as i32).collect::<Vec<_>>(),
            vec![0, 1, 2, 6, 7, 8, 9]
        );
        assert_eq!(s.len(), 7);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].0, 4, "the transformer at old 7 is now at 4");
        assert!(d.is_empty());
        // unfinished, the tries in its last sector are kept for way_on to choose from
        let path: Vec<Vec3> = (0..6).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let (p, _, _, d) =
            failed_tries_out(path, vec![0, 0, 1, 1, 1, 1], Vec::new(), vec![4], false);
        assert_eq!(p.len(), 6);
        assert_eq!(d, vec![4]);
    }
}
