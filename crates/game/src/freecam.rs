// The free camera, for debugging: with the run paused, fly anywhere through the level with the keyboard. Only
// in debug mode (--debug, or the switch on the menu's Debug page); F2 takes it up and puts it down.

use glam::Vec3;

/// The keys that fly it, as bits.
pub mod keys {
    pub const FORWARD: u16 = 1;
    pub const BACK: u16 = 2;
    pub const TURN_LEFT: u16 = 4;
    pub const TURN_RIGHT: u16 = 8;
    pub const LEFT: u16 = 16;
    pub const RIGHT: u16 = 32;
    pub const UP: u16 = 64;
    pub const DOWN: u16 = 128;
    pub const LOOK_UP: u16 = 256;
    pub const LOOK_DOWN: u16 = 512;
    pub const FAST: u16 = 1024;
}

/// Units a second, and radians a second for turning.
const SPEED: f32 = 40.0;
const FAST: f32 = 4.0;
const TURN: f32 = 1.6;

#[derive(Clone, Copy, Debug)]
pub struct FreeCam {
    pub pos: Vec3,
    /// About y, 0 looking along +x.
    pub yaw: f32,
    /// Up from the horizon, within ±85°.
    pub pitch: f32,
}

impl FreeCam {
    /// Taken up where a camera is, looking where it looks.
    pub fn at(eye: Vec3, target: Vec3) -> FreeCam {
        let d = (target - eye).normalize_or(Vec3::X);
        FreeCam {
            pos: eye,
            yaw: d.z.atan2(d.x),
            pitch: d.y.clamp(-1.0, 1.0).asin(),
        }
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos())
    }

    /// Flies `dt` seconds with the keys held.
    pub fn fly(&mut self, held: u16, dt: f32) {
        let on = |k: u16| if held & k != 0 { 1.0 } else { 0.0 };
        self.yaw += (on(keys::TURN_RIGHT) - on(keys::TURN_LEFT)) * TURN * dt;
        let limit = 85f32.to_radians();
        self.pitch = (self.pitch + (on(keys::LOOK_UP) - on(keys::LOOK_DOWN)) * TURN * dt).clamp(-limit, limit);
        let speed = SPEED * if held & keys::FAST != 0 { FAST } else { 1.0 } * dt;
        let ahead = self.forward();
        let side = Vec3::new(-self.yaw.sin(), 0.0, self.yaw.cos());
        self.pos += ahead * (on(keys::FORWARD) - on(keys::BACK)) * speed
            + side * (on(keys::RIGHT) - on(keys::LEFT)) * speed
            + Vec3::Y * (on(keys::UP) - on(keys::DOWN)) * speed;
    }

    /// The eye and the point it looks at.
    pub fn view(&self) -> (Vec3, Vec3) {
        (self.pos, self.pos + self.forward())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_looks_where_it_was_taken_up_and_flies_that_way() {
        let mut c = FreeCam::at(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0));
        let (eye, target) = c.view();
        assert!((target - eye - Vec3::Z).length() < 1e-5);
        c.fly(keys::FORWARD, 1.0);
        assert!((c.pos - Vec3::Z * SPEED).length() < 1e-3);
        c.fly(keys::UP | keys::FAST, 0.5);
        assert!((c.pos.y - SPEED * FAST * 0.5).abs() < 1e-3);
    }
}
