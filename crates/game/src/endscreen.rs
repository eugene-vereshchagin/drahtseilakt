// The level's end screen as the original's Menu_Score shows it (measured in the C game, which was sent "End Level"):
// a dark field with the lines Level Bonus, Time Points, Extra Lives and, under a rule, Score, in GameFont_04 with its
// shadow; a yellow bar on the line being counted. The level bonus comes first, then the points left on the counter
// move over to Time Points (one a frame, then five, then faster), then 200 for each life, one life at a time.

use crate::sim::Sim;
use crate::ui::{ALIGN_RIGHT, Anchor, Font, Quad, Screen, Style, TEXT_SCREEN, homogeneous, place};

/// Ticks from the finish to the original's "End Level" (the balloon carries the ball off meanwhile).
const END_DELAY: u64 = 180;
/// Ticks after "End Level": the screen shows, the bonus is added, the time points begin to move.
const SHOWN: u64 = 10;
const BONUS: u64 = 80;
const TIME: u64 = 134;
/// Ticks from the last time point to the first life, and between lives.
const FIRST_LIFE: u64 = 60;
const EACH_LIFE: u64 = 40;
/// Points a life is worth on the screen.
const LIFE: i64 = 200;

const WHITE: [f32; 4] = [1.0; 4];
/// The lines' tops (fractions of the screen), and the label and number columns.
const ROWS: [f32; 4] = [0.30, 0.36, 0.42, 0.50];
const LABELS: [&str; 4] = ["Level Bonus:", "Time Points:", "Extra Lives:", "Score:"];

/// GameFont_04: scale 0.6, right aligned, a shadow 2 off at 120 degrees in black at one half.
const FONT_04: Style = Style {
    scale: [0.6, 0.6],
    spacing: [0.0, 0.0],
    align: ALIGN_RIGHT,
    flags: TEXT_SCREEN,
    margins: [2.0; 4],
    color: WHITE,
    v2: true,
    shadow: Some(([1.0, 1.732_05], [0.0, 0.0, 0.0, 0.502])),
};

/// What the end screen shows at a tick: the four numbers, the line counted, the points left on the counter, and
/// whether the counting is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tally {
    pub numbers: [i64; 4],
    pub row: usize,
    pub counter: i64,
    /// Lives still shown as balls (each counted one leaves them).
    pub lives: u32,
    pub done: bool,
}

/// The points moved to Time Points after `t` ticks of moving: one a tick for 80, five a tick for 76, nine a tick
/// for 10, then 25 (as the C game counted 969 points).
fn moved(t: u64) -> i64 {
    let t = t as i64;
    let phases = [(80, 1), (76, 5), (10, 9), (i64::MAX, 25)];
    let (mut left, mut sum) = (t, 0i64);
    for (len, step) in phases {
        let n = left.min(len);
        sum = sum.saturating_add(n.saturating_mul(step));
        left -= n;
        if left <= 0 {
            break;
        }
    }
    sum
}

/// The tally `t` ticks after "End Level", for a run that ended with `points`, `bonus` and `lives`.
pub fn tally(t: u64, points: i64, bonus: i64, lives: u32) -> Option<Tally> {
    if t < SHOWN {
        return None;
    }
    let points = points.max(0);
    let shown_bonus = if t >= BONUS { bonus } else { 0 };
    let time = if t >= TIME {
        moved(t - TIME).min(points)
    } else {
        0
    };
    // the tick the last point moved
    let time_done = (0..)
        .map(|k| TIME + k)
        .find(|&k| moved(k - TIME) >= points)
        .unwrap_or(TIME);
    let lives_shown = if t >= time_done + FIRST_LIFE {
        (((t - time_done - FIRST_LIFE) / EACH_LIFE) + 1).min(u64::from(lives)) as i64
    } else {
        0
    };
    let extra = lives_shown * LIFE;
    let row = if t < TIME {
        0
    } else if time < points || t < time_done + FIRST_LIFE / 2 {
        1
    } else {
        2
    };
    Some(Tally {
        numbers: [shown_bonus, time, extra, shown_bonus + time + extra],
        row,
        counter: points - time,
        lives: lives.saturating_sub(lives_shown as u32),
        done: lives_shown == i64::from(lives) && time == points && t >= BONUS,
    })
}

/// The tally of a finished run now, if the screen is up.
pub fn of(sim: &Sim) -> Option<Tally> {
    if !sim.finished {
        return None;
    }
    let t = sim.tick.checked_sub(sim.finished_at + END_DELAY)?;
    tally(t, sim.points, sim.level_bonus, sim.lives)
}

/// The end screen's quads.
pub fn interface(t: &Tally, font: &Font, screen: Screen) -> Vec<Quad> {
    let quad = |r: [f32; 4], uv: [f32; 4], color: [f32; 4], texture: &'static str| Quad {
        rect: screen.rect(homogeneous(r), Anchor::Center),
        uv,
        top: color,
        bottom: color,
        texture,
    };
    const SPECIAL: &str = "Button01_special";
    let mut out = vec![quad(
        [0.30, 0.27, 0.70, 0.57],
        [0.0, 0.99, 1.0, 1.0],
        [0.0, 0.0, 0.0, 0.43],
        SPECIAL,
    )];
    if let Some(&y) = ROWS.get(t.row) {
        out.push(quad(
            [0.3134, y, 0.687_53, y + 0.056_37],
            [0.6, 0.52, 0.9, 0.64],
            [0.98, 1.0, 0.0, 0.35],
            SPECIAL,
        ));
    }
    // the rule over the score: untextured white
    out.push(quad(
        [0.314_08, 0.481_59, 0.686_18, 0.486_12],
        [0.0; 4],
        WHITE,
        "",
    ));
    for (k, (&y, label)) in ROWS.iter().zip(LABELS).enumerate() {
        let text = |out: &mut Vec<Quad>, s: &str, x0: f32, x1: f32| {
            out.extend(place(
                font.layout(s.as_bytes(), homogeneous([x0, y, x1, y + 0.05]), &FONT_04),
                screen,
                Anchor::Center,
            ));
        };
        text(&mut out, label, 0.32, 0.57);
        text(
            &mut out,
            &t.numbers.get(k).copied().unwrap_or(0).to_string(),
            0.58,
            0.68,
        );
    }
    if t.done {
        // the keys (the original goes on to its highscore entry here)
        let keys = Style {
            align: 0,
            ..FONT_04
        };
        let r = homogeneous([0.25, 0.60, 0.75, 0.65]);
        out.extend(place(
            font.layout(b"Enter: next level    Esc: menu", r, &keys),
            screen,
            Anchor::Center,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The C game's level 1 run sent "End Level" with 969 points, a bonus of 100 and three lives: the numbers it showed
    /// (DUMPTEXT every 10 frames from the message).
    #[test]
    fn the_tally_counts_as_the_c_game_did() {
        let at = |t| tally(t, 969, 100, 3).expect("shown");
        assert!(tally(5, 969, 100, 3).is_none());
        assert_eq!(at(70).numbers, [0, 0, 0, 0]);
        assert_eq!(at(80).numbers, [100, 0, 0, 100]);
        assert_eq!(at(140).numbers[1], 6);
        assert_eq!(at(210).numbers[1], 76);
        assert_eq!(at(290).numbers[1], 460);
        assert_eq!(at(320).numbers, [100, 969, 0, 1069]);
        assert_eq!(at(380).numbers[2], 200);
        assert_eq!(at(420).numbers[2], 400);
        let end = at(460);
        assert_eq!(end.numbers, [100, 969, 600, 1669]);
        assert!(end.done && end.counter == 0);
        assert_eq!((at(200).row, at(500).row), (1, 2));
    }
}
