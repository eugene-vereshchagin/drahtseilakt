// The hook for an outside player of a run in the window: a tool of ours (built on this library) that decides the
// run's input tick by tick instead of the keyboard, takes keys of its own, and draws its own marks and words over
// the game. The game itself never makes one (its binary passes none); `app::run_window` takes it.

use crate::Options;
use crate::app::Game;
use crate::assets::Data;
use crate::scene::Snapshot;
use std::time::Instant;
pub use winit::keyboard::KeyCode;

/// What a controller may change while it plays: the run, the keys the player holds (when the controller gives the
/// keys back, it clears them), and the view's last tick (`prev`: the frame interpolates from it; set it before a
/// tick, clear it when the run jumped).
pub struct Ctx<'a> {
    pub game: &'a mut Game,
    pub data: &'a Data,
    pub opts: &'a Options,
    pub keys: &'a mut u8,
    pub prev: &'a mut Option<Snapshot>,
}

/// An outside player of the run in the window. While it is not `done` the game asks it, each frame, how fast the
/// run goes, and, while it `drives`, lets it play each tick itself; while it does not drive the player's keys play
/// and it hears of each tick after. It sees every key before the game's own bindings, adds to the world's
/// instances and to the interface. A done controller leaves the window playing as the game, still with the
/// original's rules (no mods) and no replay prompt, since the run is not the player's.
pub trait Controller {
    /// Its run is over: the window plays on as the game.
    fn done(&self) -> bool;
    /// It plays the ticks itself (false: the player's keys drive, e.g. taken over by hand).
    fn drives(&self) -> bool;
    /// How fast the run goes against real time (0: paused).
    fn speed(&self) -> f32;
    /// Plays one tick (started: when this frame's ticks began, for a time budget). False when no tick was played:
    /// the frame's ticks stop there.
    fn tick(&mut self, ctx: &mut Ctx, started: Instant) -> bool;
    /// After a tick the player's keys played.
    fn after_tick(&mut self, game: &Game);
    /// A key went down (`down`) or was let go; true when it was the controller's.
    fn key(&mut self, ctx: &mut Ctx, code: KeyCode, down: bool) -> bool;
    /// Adds its marks in the world to the frame's instances.
    fn instances(&self, game: &Game, out: &mut Vec<crate::render::Instance>);
    /// Its status, a line a fact (drawn in the debug font when the interface's font is missing).
    fn status(&self, game: &Game) -> String;
    /// Its interface over the frame, in the interface's font.
    fn overlay(
        &self,
        game: &Game,
        strings: &crate::lang::Strings,
        font: &crate::ui::Face,
        screen: crate::ui::Screen,
    ) -> Vec<crate::ui::Quad>;
    /// The window closed while it was not done: it ends the run as it stands.
    fn close(&mut self, game: &Game, o: &Options) -> Result<(), String>;
}
