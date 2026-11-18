// The game's facade for a map's scripts: the arrays of the original game (`CurrentLevel`, `IngameParameter`,
// `Energy`, `Physicalize_GameBall`) with the original's columns, their cells filled from the run, and the messages the
// engine emits at the native rules' events (docs/gameplay_bbs.md, "The game's facade"). Read-only for the rules: a
// script's write to a cell is kept apart in the run's scripts and never reaches the simulation.

use crate::sim::{BallKind, LIFE_BONUS, POINTS_EVERY, START_POINTS};
use script::Value;

/// Column names and kinds (1 int, 2 float, 3 string, 4 object, 5 parameter) of the game's arrays, as saved in
/// Gameplay.nmo and Balls.nmo.
pub const CURRENT_LEVEL: [(&str, u32); 6] = [
    ("Level ID", 1),
    ("ActiveBall", 4),
    ("Ball_Pos_Frame", 4),
    ("CurrentResetpoint", 5),
    ("Activation Phase?", 5),
    ("Points", 1),
];
pub const INGAME_PARAMETER: [(&str, u32); 5] = [
    ("Activetrafo", 4),
    ("Activate Sector", 1),
    ("Deactivate Sector", 1),
    ("Tutorial activ?", 5),
    ("RollSound activate?", 5),
];
pub const ENERGY: [(&str, u32); 6] = [
    ("Points", 1),
    ("Lifes", 1),
    ("StartPoints", 1),
    ("StartLifes", 1),
    ("Timefactor", 5),
    ("LifeBonus", 1),
];
pub const PHYSICALIZE_GAMEBALL: [(&str, u32); 8] = [
    ("Ballname", 3),
    ("Friction", 2),
    ("Elasticity", 2),
    ("Mass", 2),
    ("CollGroup", 3),
    ("Linear Damp", 2),
    ("Rot Damp", 2),
    ("Force", 2),
];
/// The rows of Balls.nmo's `Physicalize_GameBall`: name, friction, elasticity, mass, collision group, linear damping,
/// rotational damping, force.
type BallRow = (&'static str, f32, f32, f32, &'static str, f32, f32, f32);
const GAME_BALLS: [BallRow; 3] = [
    ("Ball_Paper", 0.5, 0.4, 0.2, "Ball", 1.5, 0.1, 0.065),
    ("Ball_Stone", 0.5, 0.1, 10.0, "Ball", 0.3, 0.1, 0.92),
    ("Ball_Wood", 0.8, 0.2, 1.9, "Ball", 0.9, 0.1, 0.43),
];

/// The columns of a game array, by name (case-insensitive).
pub fn columns(array: &str) -> Option<&'static [(&'static str, u32)]> {
    let a = array.to_ascii_lowercase();
    Some(match a.as_str() {
        "currentlevel" => &CURRENT_LEVEL,
        "ingameparameter" => &INGAME_PARAMETER,
        "energy" => &ENERGY,
        "physicalize_gameball" => &PHYSICALIZE_GAMEBALL,
        _ => return None,
    })
}

/// What the cells are filled from: the run at a tick, and the ids of the ball objects in the map's graph.
pub struct Fill {
    pub in_play: bool,
    pub points: i64,
    pub lives: u32,
    pub sector: usize,
    /// The sector left last (0 before the first change).
    pub left_sector: usize,
    /// `Ball_Pos_Frame`, and the object of the ball's kind (`Ball_Wood`...), if the graph has them.
    pub frame: Option<script::Id>,
    pub kind_object: Option<script::Id>,
}

fn int(v: i64) -> Value {
    Value::Int(i32::try_from(v).unwrap_or(i32::MAX))
}

/// A cell of a game array as the original holds it at the start of a level, with the run's numbers in.
pub fn cell(array: &str, row: usize, column: usize, f: &Fill) -> Option<Value> {
    let a = array.to_ascii_lowercase();
    if a == "physicalize_gameball" {
        let r = GAME_BALLS.get(row)?;
        return Some(match column {
            0 => Value::Str(r.0.into()),
            1 => Value::Float(r.1),
            2 => Value::Float(r.2),
            3 => Value::Float(r.3),
            4 => Value::Str(r.4.into()),
            5 => Value::Float(r.5),
            6 => Value::Float(r.6),
            7 => Value::Float(r.7),
            _ => return None,
        });
    }
    if row != 0 {
        return None;
    }
    Some(match (a.as_str(), column) {
        // an unnumbered map: the level table's number is the game's, a map has none
        ("currentlevel", 0) => Value::Int(0),
        ("currentlevel", 1) => Value::Obj(f.kind_object.or(f.frame)),
        ("currentlevel", 2) => Value::Obj(f.frame),
        ("currentlevel", 3) => Value::None,
        ("currentlevel", 4) => Value::Bool(f.in_play),
        ("currentlevel", 5) => int(f.points),
        // the transformer last hit is not an object a script can name
        ("ingameparameter", 0) => Value::Obj(None),
        ("ingameparameter", 1) => int(i64::try_from(f.sector).unwrap_or(i64::MAX)),
        ("ingameparameter", 2) => int(i64::try_from(f.left_sector).unwrap_or(i64::MAX)),
        ("ingameparameter", 3) => Value::Bool(false),
        ("ingameparameter", 4) => Value::Bool(true),
        ("energy", 0) => int(f.points),
        ("energy", 1) => int(i64::from(f.lives)),
        ("energy", 2) => int(START_POINTS),
        ("energy", 3) => int(i64::from(crate::sim::START_LIVES)),
        // milliseconds between two points counted off (the counter's period, 30 ticks of 1/60 s)
        ("energy", 4) => Value::Float(POINTS_EVERY as f32 * 1000.0 / 60.0),
        ("energy", 5) => int(LIFE_BONUS),
        _ => return None,
    })
}

/// The messages the original's pause sends: `Pause Level` when the pause menu opens; `Unpause Level` and `Refresh`
/// (the gameplay script's refresh of the time factor and the sky) when the run goes on.
pub fn pause_events(paused: bool) -> &'static [&'static str] {
    if paused {
        &["Pause Level"]
    } else {
        &["Unpause Level", "Refresh"]
    }
}

/// The run's numbers an event is seen from, one tick to the next.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seen {
    pub sector: usize,
    pub lives: u32,
    pub points: i64,
    pub kind: BallKind,
    pub finished: bool,
    /// The ball is dying (falling out of the level), and the last checkpoint has been passed (a level with
    /// checkpoints).
    pub dying: bool,
    pub last_stage: bool,
}

/// The game's messages for what changed between two ticks, in the order the original sends them. The table
/// "game event -> message name" (docs/gameplay_bbs.md): level start `Start Music` (Init Ingame), sector reached
/// `Checkpoint reached`, a life lost `Ball Off` then `Sub Life` (and `Game Over` with the last one), an extra life
/// `Life_Up`, an extra point `Extrapoint`, the level finished `Level_Finish` then `End Level`, a new ball `Trafo`;
/// also the last checkpoint reached `last Checkpoint reached` (after `Checkpoint reached`), the ball starting to
/// die `Dead`, and at the start `Part_Loaded` (the level's parts are in) before `Start Music`. The pause's own
/// messages come from `pause_events`.
pub fn events(prev: Option<&Seen>, now: &Seen) -> Vec<&'static str> {
    let Some(p) = prev else {
        return vec!["Part_Loaded", "Start Music"];
    };
    let mut out = Vec::new();
    if now.sector > p.sector {
        out.push("Checkpoint reached");
        if now.last_stage && !p.last_stage {
            out.push("last Checkpoint reached");
        }
    }
    if now.dying && !p.dying {
        out.push("Dead");
    }
    if now.lives < p.lives {
        out.push("Ball Off");
        out.push("Sub Life");
        if now.lives == 0 {
            out.push("Game Over");
        }
    }
    if now.lives > p.lives {
        out.push("Life_Up");
    }
    if now.finished && !p.finished {
        out.push("Level_Finish");
        out.push("End Level");
    } else if now.points > p.points && !now.finished && now.lives == p.lives {
        out.push("Extrapoint");
    }
    if now.kind != p.kind {
        out.push("Trafo");
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn fill() -> Fill {
        Fill {
            in_play: true,
            points: 970,
            lives: 2,
            sector: 3,
            left_sector: 2,
            frame: Some(7),
            kind_object: Some(9),
        }
    }

    fn seen() -> Seen {
        Seen {
            sector: 1,
            lives: 3,
            points: 1000,
            kind: BallKind::Wood,
            finished: false,
            dying: false,
            last_stage: false,
        }
    }

    #[test]
    fn layouts_are_the_originals() {
        // Gameplay.nmo's arrays, column by column
        let names = |a: &str| -> Vec<&str> { columns(a).unwrap().iter().map(|c| c.0).collect() };
        assert_eq!(
            names("CurrentLevel"),
            [
                "Level ID",
                "ActiveBall",
                "Ball_Pos_Frame",
                "CurrentResetpoint",
                "Activation Phase?",
                "Points"
            ]
        );
        assert_eq!(
            names("IngameParameter"),
            [
                "Activetrafo",
                "Activate Sector",
                "Deactivate Sector",
                "Tutorial activ?",
                "RollSound activate?"
            ]
        );
        assert_eq!(
            names("Energy"),
            [
                "Points",
                "Lifes",
                "StartPoints",
                "StartLifes",
                "Timefactor",
                "LifeBonus"
            ]
        );
        assert_eq!(columns("physicalize_gameball").unwrap().len(), 8);
        assert!(columns("Checkpoints").is_none());
    }

    #[test]
    fn cells_follow_the_run() {
        let f = fill();
        assert_eq!(cell("CurrentLevel", 0, 1, &f), Some(Value::Obj(Some(9))));
        assert_eq!(cell("currentlevel", 0, 2, &f), Some(Value::Obj(Some(7))));
        assert_eq!(cell("CurrentLevel", 0, 4, &f), Some(Value::Bool(true)));
        assert_eq!(cell("CurrentLevel", 0, 5, &f), Some(Value::Int(970)));
        assert_eq!(cell("CurrentLevel", 1, 0, &f), None, "one row");
        assert_eq!(cell("CurrentLevel", 0, 6, &f), None, "six columns");
        assert_eq!(cell("IngameParameter", 0, 1, &f), Some(Value::Int(3)));
        assert_eq!(cell("IngameParameter", 0, 2, &f), Some(Value::Int(2)));
        assert_eq!(cell("Energy", 0, 0, &f), Some(Value::Int(970)));
        assert_eq!(cell("Energy", 0, 1, &f), Some(Value::Int(2)));
        assert_eq!(
            cell("Energy", 0, 2, &f),
            Some(Value::Int(1000)),
            "StartPoints"
        );
        assert_eq!(cell("Energy", 0, 3, &f), Some(Value::Int(3)), "StartLifes");
        assert_eq!(cell("Energy", 0, 5, &f), Some(Value::Int(200)), "LifeBonus");
        assert_eq!(
            cell("Physicalize_GameBall", 2, 0, &f),
            Some(Value::Str("Ball_Wood".into()))
        );
        assert_eq!(
            cell("Physicalize_GameBall", 1, 3, &f),
            Some(Value::Float(10.0))
        );
        assert_eq!(cell("Physicalize_GameBall", 3, 0, &f), None);
    }

    #[test]
    fn each_event_has_its_message() {
        assert_eq!(events(None, &seen()), ["Part_Loaded", "Start Music"]);
        let s = seen();
        assert!(events(Some(&s), &s).is_empty());
        let t = |f: &dyn Fn(&mut Seen)| {
            let mut n = s;
            f(&mut n);
            events(Some(&s), &n)
        };
        assert_eq!(t(&|n| n.sector = 2), ["Checkpoint reached"]);
        assert_eq!(t(&|n| n.lives = 2), ["Ball Off", "Sub Life"]);
        assert_eq!(
            t(&|n| {
                n.sector = 5;
                n.last_stage = true;
            }),
            ["Checkpoint reached", "last Checkpoint reached"]
        );
        assert_eq!(t(&|n| n.dying = true), ["Dead"]);
        assert_eq!(pause_events(true), ["Pause Level"]);
        assert_eq!(pause_events(false), ["Unpause Level", "Refresh"]);
        assert_eq!(
            events(Some(&Seen { lives: 1, ..s }), &Seen { lives: 0, ..s }),
            ["Ball Off", "Sub Life", "Game Over"]
        );
        assert_eq!(t(&|n| n.lives = 4), ["Life_Up"]);
        assert_eq!(t(&|n| n.points = 1100), ["Extrapoint"]);
        assert_eq!(
            t(&|n| n.points = 999),
            Vec::<&str>::new(),
            "the counter ticking down is no event"
        );
        assert_eq!(
            t(&|n| n.lives = 2),
            ["Ball Off", "Sub Life"],
            "the points a respawn resets are no extra point"
        );
        assert_eq!(
            t(&|n| {
                n.finished = true;
                n.points = 1800;
            }),
            ["Level_Finish", "End Level"]
        );
        assert_eq!(t(&|n| n.kind = BallKind::Paper), ["Trafo"]);
    }
}
