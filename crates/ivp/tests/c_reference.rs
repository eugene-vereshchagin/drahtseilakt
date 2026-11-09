// A synthetic scene run against the C reference physics (OpenBallance src/phys, the upstream this crate ports): a
// floor of two-triangle tiles with seams between them, a ramp and a wall as bodies of their own, and a ball pushed
// along both axes and left alone. The positions it must reach were printed by the C physics for the same scene,
// built and stepped the same way (scenario text from `scene()`, run by a small C program over phys_body_create,
// phys_force_create and ivp_time_simulate_until; IVP_SCENARIO_OUT=<file> writes the text). No game data is involved.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use ivp::body::{BodyDesc, Shape};
use ivp::core::World;
use ivp::ledge::{self, Surface};
use std::fmt::Write as _;

/// One static body: its triangles in its own frame, its position and its group.
struct Static {
    tris: Vec<[[f32; 3]; 3]>,
    pos: [f32; 3],
    group: &'static str,
}

/// The scene: a 60 x 60 floor of 5 x 5 tiles, a ramp rising along +x and a wall across +z.
fn scene() -> Vec<Static> {
    let mut floor = Vec::new();
    for i in 0..12 {
        for k in 0..12 {
            let (x0, z0) = (-30.0 + 5.0 * i as f32, -30.0 + 5.0 * k as f32);
            let (x1, z1) = (x0 + 5.0, z0 + 5.0);
            floor.push([[x0, 0.0, z0], [x1, 0.0, z1], [x1, 0.0, z0]]);
            floor.push([[x0, 0.0, z0], [x0, 0.0, z1], [x1, 0.0, z1]]);
        }
    }
    let ramp = vec![
        [[0.0, 0.0, -5.0], [10.0, 3.0, 5.0], [10.0, 3.0, -5.0]],
        [[0.0, 0.0, -5.0], [0.0, 0.0, 5.0], [10.0, 3.0, 5.0]],
    ];
    let wall = vec![
        [[-30.0, 0.0, 0.0], [30.0, 6.0, 0.0], [30.0, 0.0, 0.0]],
        [[-30.0, 0.0, 0.0], [-30.0, 6.0, 0.0], [30.0, 6.0, 0.0]],
    ];
    vec![
        Static {
            tris: floor,
            pos: [0.0, 0.0, 0.0],
            group: "Floor",
        },
        Static {
            tris: ramp,
            pos: [8.0, 0.0, -10.0],
            group: "Floor",
        },
        Static {
            tris: wall,
            pos: [0.0, 0.0, 22.0],
            group: "",
        },
    ]
}

/// The force on the ball at a tick (the wooden ball's 0.43 along world axes, two at once on the diagonal), and
/// whether the arrows changed then (the game wakes the ball).
fn input(t: u32) -> ([f32; 3], bool) {
    let f = match t {
        1..=39 => [0.0, 0.0, 0.0],
        40..=99 => [0.43, 0.0, 0.0],
        100..=169 => [0.0, 0.0, 0.43],
        170..=229 => [-0.43, 0.0, 0.43],
        230..=319 => [0.0, 0.0, 0.0],
        320..=339 => [-0.43, 0.0, 0.0],
        340..=399 => [0.0, 0.0, 0.0],
        400..=459 => [0.0, 0.0, -0.43],
        _ => [0.0, 0.0, 0.0],
    };
    (f, matches!(t, 40 | 100 | 170 | 230 | 320 | 340 | 400 | 460))
}

const TICKS: u32 = 600;

/// The scene as the C program reads it.
fn scenario_text() -> String {
    let mut out = String::new();
    for s in scene() {
        let g = if s.group.is_empty() { "-" } else { s.group };
        let _ = writeln!(
            out,
            "static {} {} {} {} 0 0 0 1 {g}",
            s.tris.len(),
            s.pos[0],
            s.pos[1],
            s.pos[2]
        );
        for t in &s.tris {
            let _ = writeln!(
                out,
                "tri {} {} {} {} {} {} {} {} {}",
                t[0][0], t[0][1], t[0][2], t[1][0], t[1][1], t[1][2], t[2][0], t[2][1], t[2][2]
            );
        }
    }
    let _ = writeln!(out, "ball -6 2.5 -12");
    for t in 1..=TICKS {
        let (f, wake) = input(t);
        let _ = writeln!(
            out,
            "tick {t} {} {} {} {}",
            f[0],
            f[1],
            f[2],
            u8::from(wake)
        );
    }
    out
}

fn group(name: &str) -> [u8; 8] {
    let mut g = [0u8; 8];
    for (d, s) in g.iter_mut().zip(name.bytes()) {
        *d = s;
    }
    g
}

/// The ball's position after each tick.
fn run() -> Vec<[f64; 3]> {
    let identity = ivp::math::Quat {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };
    let mut w = World::new();
    w.time_start();
    w.set_gravity([0.0, -20.0, 0.0]);
    for s in scene() {
        let v = |p: [f32; 3]| [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])];
        let ledges = s
            .tris
            .iter()
            .filter_map(|t| ledge::triangle(v(t[0]), v(t[1]), v(t[2])))
            .collect();
        let desc = BodyDesc {
            fixed: true,
            friction: 0.7,
            elasticity: 0.3,
            mass: 1.0,
            lin_damp: 0.1,
            rot_damp: 0.1,
            group: group(s.group),
            ..BodyDesc::default()
        };
        w.add_body(
            desc,
            Shape::Surface(Surface::compile(ledges).expect("a surface")),
            v(s.pos),
            identity,
        );
    }
    let desc = BodyDesc {
        mass: 1.9,
        friction: 0.8,
        elasticity: 0.2,
        lin_damp: 0.9,
        rot_damp: 0.1,
        group: group("Ball"),
        ..BodyDesc::default()
    };
    let ball = w.add_body(desc, Shape::Sphere(2.0), [-6.0, 2.5, -12.0], identity);
    w.wake_body(ball);
    let (mut force, mut cur) = (None, [0.0f32; 3]);
    let mut out = Vec::new();
    for t in 1..=TICKS {
        let (f, wake) = input(t);
        if f != cur {
            if let Some(c) = force.take() {
                w.force_destroy(c);
            }
            if f != [0.0; 3] {
                force = Some(w.force_create(ball, f, [0.0; 3], true));
            }
            cur = f;
        }
        if wake {
            w.wake_body(ball);
        }
        w.advance(1.0 / 30.0);
        out.push(w.body_pose(ball).0);
    }
    out
}

/// The ball's position printed by the C physics for this scene: on the ramp, at the wall, along it, back across the
/// seams, and rolling out.
const C_TRACK: [(u32, [f64; 3]); 6] = [
    (120, [13.435885, 3.733458, -9.713441]),
    (200, [11.471280, 2.145008, 19.883689]),
    (300, [-8.732321, 2.017367, 19.889609]),
    (400, [-19.860302, 2.018282, 19.861649]),
    (500, [-22.072786, 2.014250, -5.001870]),
    (600, [-22.306095, 2.017888, -11.185948]),
];

#[test]
fn the_ball_takes_the_c_reference_path() {
    if let Ok(path) = std::env::var("IVP_SCENARIO_OUT") {
        std::fs::write(path, scenario_text()).expect("the scenario file");
    }
    let track = run();
    for (t, c) in C_TRACK {
        let p = track[t as usize - 1];
        let d = ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2)).sqrt();
        assert!(
            d < 1e-3,
            "tick {t}: the port's ball at {p:?}, the C ball at {c:?}"
        );
    }
}
