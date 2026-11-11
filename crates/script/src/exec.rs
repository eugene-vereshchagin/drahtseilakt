// The runtime: runs a map's scripts one tick at a time the way the original's behaviour manager does
// (docs/ck-runtime.md, "Execution"): each active script is executed; a graph fires the links from its active
// IOs (a zero-delay link in the same tick, a delayed one that many ticks later), runs its active sub-behaviours
// from a priority stack, and stays active while links are queued or sub-behaviours active. Building blocks run
// through blocks.rs. Messages sent in a tick reach their waiters in the next. All order is by index, so a run is
// deterministic. The state is small and cloned with the run (rewind, replays); the graph is shared.

use crate::blocks::{self, BlockEvent, Flow};
use crate::{Graph, Id, Node, ObjKind, Value};
use std::sync::Arc;

/// An entity of the engine a block acts on: a level object by index, or the ball.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ent {
    Object(usize),
    Ball,
}

/// The engine's side of the scripts: what blocks read and change. Positions are world coordinates; matrices are
/// column-major with the position in the last column, as the level's objects.
pub trait Host {
    /// The tick's length in milliseconds (the original's delta time).
    fn delta_ms(&self) -> f32;
    fn matrix(&self, e: Ent) -> Option<[[f32; 4]; 4]>;
    /// The entity's matrix as the level was loaded (its initial conditions).
    fn initial_matrix(&self, e: Ent) -> Option<[[f32; 4]; 4]>;
    fn set_matrix(&mut self, e: Ent, m: [[f32; 4]; 4]);
    fn set_visible(&mut self, e: Ent, on: bool);
    /// The world-axis bounding box of the entity.
    fn bounds(&self, e: Ent) -> Option<([f32; 3], [f32; 3])>;
    fn delete(&mut self, e: Ent);
    fn physicalize(&mut self, e: Ent, on: bool);
    fn particles(&mut self, e: Ent, on: bool);
    /// The level mesh of an entity, by index.
    fn mesh_of(&self, e: Ent) -> Option<usize>;
    /// The members of a level group, by index.
    fn group_members(&self, group: usize) -> Vec<usize>;
    /// A cell of one of the game's own arrays (`CurrentLevel`...), which live in the engine, not in the map.
    fn game_cell(&self, array: &str, row: usize, column: usize) -> Option<Value>;
    /// A message a script sends (to an entity, or to all): the engine may act on the game's own messages.
    fn message(&mut self, name: &str, to: Option<Ent>);
    fn sound(&mut self, name: &str, on: bool);
    fn text(&mut self, name: &str, text: &str, on: bool);
    /// A line for the log: what a script did, or what the runtime could not do.
    fn note(&mut self, text: &str);
    /// A copy of a level object under a new name (Object Copy): the new object's index, or none when the
    /// engine makes no copy.
    fn copy(&mut self, _e: Ent, _name: &str) -> Option<usize> {
        None
    }
    /// A level group gains or loses a member, or all of them (Add To Group, Remove From Group, Group Clear).
    fn group_add(&mut self, _group: usize, _object: usize) {}
    fn group_remove(&mut self, _group: usize, _object: usize) {}
    fn group_clear(&mut self, _group: usize) {}
}

/// Per-block state a block keeps between ticks.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Scratch {
    #[default]
    None,
    /// An iterator's next index.
    Index(i64),
    /// A delayer's or timer's time left or elapsed, in ms.
    Time(f32),
    /// A proximity test's last state: inside, and ticks until the next test.
    Proximity {
        inside: Option<bool>,
        wait: i64,
    },
    /// A message waiter: received.
    Received(bool),
    Flag(bool),
    /// A progression's time so far (ms, or frames) and the value it gave last.
    Progress {
        elapsed: f32,
        last: f32,
    },
    /// A timer that may be paused: its time, and whether it runs.
    Clock {
        elapsed: f32,
        running: bool,
    },
}

#[derive(Debug, Clone, Default)]
struct BlockState {
    active: bool,
    stacked: bool,
    scratch: Scratch,
    /// Reset at least once (the original's flags are no longer all zero): activating it again without asking for
    /// a reset keeps its state.
    reset_done: bool,
    /// Pending activation requests of a script (`REQ_*`), applied at the start of the next tick.
    request: u8,
    /// Deleted (Object Delete of the behaviour or of its owner): never runs again.
    gone: bool,
    /// The tick in which Execute Script last ran it inside its caller's frame (the manager skips it then).
    ran_inline: u64,
}

const REQ_ACTIVATE: u8 = 1;
const REQ_DEACTIVATE: u8 = 2;
const REQ_RESET: u8 = 4;

/// An object's state in the scene: asleep (deactivated in the scene: its scripts are not executed) or deleted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ObjState {
    asleep: bool,
    gone: bool,
}

/// The most trace lines a run keeps (the trace is a tool's, never the game's).
const MAX_TRACE: usize = 1 << 20;
/// How deep Execute Script may nest scripts run inside their caller's frame.
const MAX_INLINE: usize = 8;

#[derive(Debug, Clone, Default)]
struct LinkState {
    current: u16,
    queued: bool,
}

#[derive(Debug, Clone)]
pub struct Runtime {
    pub graph: Arc<Graph>,
    blocks: Vec<BlockState>,
    links: Vec<LinkState>,
    io: Vec<bool>,
    pub values: Vec<Value>,
    /// The maps' arrays' rows, which Set Cell and Add Row change (the graph's are the file's).
    arrays: Vec<Vec<Vec<Value>>>,
    /// Messages sent this tick, delivered at the next: (name, receiver or all).
    outbox: Vec<(String, Option<Id>)>,
    inbox: Vec<(String, Option<Id>)>,
    pub tick: u64,
    /// Prototypes met that the runtime does not know, reported once each.
    unknown: Vec<crate::Guid>,
    /// Work left this tick: every activation, execution, parameter read and write spends one, so a graph that
    /// fans out (a sub-behaviour listed a thousand times, an operation reading one parameter twice, round and
    /// round) cannot take the game's time, whatever file it came from.
    budget: u64,
    /// Whether the budget ran out (said once).
    starved: bool,
    /// The random blocks' generator: a fixed seed stepped by each draw, so a run repeats exactly.
    rng: u64,
    /// Objects copied so far (Object Copy), capped.
    copies: usize,
    /// The scripts in the order the behaviour manager runs them: objects by priority (higher first, file order
    /// among equals), each object's scripts in its list's order.
    order: Vec<Id>,
    /// Each graph object's state in the scene.
    objects: Vec<ObjState>,
    /// Whether any activation request waits (most ticks none does: the requests pass is skipped).
    requested: bool,
    /// Scripts being run inside a caller's frame (Execute Script), innermost last.
    inline: Vec<Id>,
    /// The trace (frames, behaviours run, IOs fired, parameters written), when asked for.
    trace: Option<Vec<String>>,
}

/// The most objects a run's scripts may copy.
pub const MAX_COPIES: usize = 256;

/// The most pops of the execution stack in one graph in one tick (the original's BehaviorMaxIteration).
const MAX_ITERATIONS: usize = 8_000;
const MAX_DEPTH: usize = 48;
/// The work one tick of all the scripts may do (a map's own take a few thousand).
const TICK_BUDGET: u64 = 300_000;

impl Runtime {
    /// A runtime as the scripts of a loaded level start: each script reset and active, unless its saved activity
    /// (or its object's) starts it deactivated (the saved behaviour flags say nothing of it: most scripts that
    /// run have no active bit there).
    pub fn new(graph: Arc<Graph>) -> Runtime {
        let n = graph.nodes.len();
        let values = graph
            .nodes
            .iter()
            .map(|n| match n {
                Node::Param(p) => p.value.clone(),
                _ => Value::None,
            })
            .collect();
        let mut rt = Runtime {
            blocks: vec![BlockState::default(); n],
            links: vec![LinkState::default(); n],
            io: vec![false; n],
            values,
            arrays: graph.arrays.iter().map(|a| a.rows.clone()).collect(),
            graph,
            outbox: Vec::new(),
            inbox: Vec::new(),
            tick: 0,
            unknown: Vec::new(),
            budget: TICK_BUDGET,
            starved: false,
            rng: 0x9e37_79b9_7f4a_7c15,
            copies: 0,
            order: Vec::new(),
            objects: Vec::new(),
            requested: false,
            inline: Vec::new(),
            trace: None,
        };
        rt.objects = vec![ObjState::default(); rt.graph.objects.len()];
        for &o in &rt.graph.start_off {
            if let Some(st) = rt.objects.get_mut(o)
                && rt.graph.block(o).is_none()
            {
                st.asleep = true;
            }
        }
        rt.order = run_order(&rt.graph);
        let scripts = rt.graph.scripts.clone();
        // the original's launch resets every block (CKM_BEHAVIORRESET) before the engine runs: nothing to tell
        let mut quiet = Quiet;
        let off: std::collections::HashSet<Id> = rt.graph.start_off.iter().copied().collect();
        for s in scripts {
            rt.bind_owner(s);
            let active = !off.contains(&s);
            rt.activate(s, active, true, 0);
            rt.tree_event(s, BlockEvent::Reset, &mut quiet, 0);
        }
        rt.budget = TICK_BUDGET;
        rt
    }

    /// Turns the trace on (or off): from then on each tick's frame, behaviours run, IOs fired and parameters
    /// written are kept for `take_trace`.
    pub fn set_trace(&mut self, on: bool) {
        self.trace = on.then(Vec::new);
    }

    /// The trace lines kept since the last call.
    pub fn take_trace(&mut self) -> Vec<String> {
        self.trace.as_mut().map(std::mem::take).unwrap_or_default()
    }

    fn trace_line(&mut self, line: impl FnOnce(&Runtime) -> String) {
        if self.trace.as_ref().is_some_and(|t| t.len() < MAX_TRACE) {
            let l = line(self);
            if let Some(t) = self.trace.as_mut() {
                t.push(l);
            }
        }
    }

    /// The scripts in the order a tick runs them.
    pub fn order(&self) -> &[Id] {
        &self.order
    }

    /// Puts the script's owner where its "This" parameters stand.
    fn bind_owner(&mut self, script: Id) {
        let Some(owner) = self.graph.block(script).and_then(|b| b.owner) else {
            return;
        };
        for id in self.graph.script_nodes(script).unwrap_or_default() {
            if matches!(self.graph.param(id), Some(p) if p.value == Value::Owner)
                && let Some(v) = self.values.get_mut(id)
            {
                *v = Value::Obj(Some(owner));
            }
        }
    }

    /// The next random number in [0, 1): the same sequence in every run (xorshift), so replays repeat.
    pub fn random(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x.wrapping_shl(13);
        x ^= x.wrapping_shr(7);
        x ^= x.wrapping_shl(17);
        self.rng = x;
        (x.wrapping_shr(40) as f32) / 16_777_216.0
    }

    /// Object Copy: a copy of a level object with copies of the scripts attached to it, bound to the copy and
    /// in the state the originals are in. The copy gets a name of its own (the original's and a number), as
    /// the scripts that copy a template tell their copies apart by name. None when the object is no level
    /// object, the engine makes no copy, or the run has copied its fill.
    pub fn copy_object(&mut self, original: Id, host: &mut dyn Host) -> Option<Id> {
        let Some(Ent::Object(index)) = self.ent(Some(original)) else {
            return None;
        };
        // every try counts, made or refused: each walks the scripts once, and a map may ask every tick
        if self.copies >= MAX_COPIES {
            if self.copies == MAX_COPIES {
                self.copies = MAX_COPIES.saturating_add(1);
                host.note("script copies stopped: too many objects copied");
            }
            return None;
        }
        let number = self.copies.saturating_add(1);
        self.copies = number;
        let scripts: Vec<Id> = self
            .graph
            .scripts
            .iter()
            .copied()
            .filter(|&s| {
                self.graph
                    .block(s)
                    .is_some_and(|b| b.owner == Some(original))
            })
            .collect();
        // The copy's work is the nodes it copies. Walking them is paid from the tick's budget as it goes,
        // whether the copy is made or not: a copy that does not fit in what is left is refused whole (and
        // the tick's work is spent), and the run's graph stays under its cap.
        let mut work = 0u64;
        for &s in &scripts {
            let Some(nodes) = self.graph.script_nodes(s) else {
                host.note("script copy refused: a script of the object is too large");
                return None;
            };
            let cost = (nodes.len() as u64).saturating_add(1);
            if cost > self.budget {
                self.budget = 0;
                self.starved = true;
                host.note(
                    "script copy refused: the object's scripts are more than a tick may copy",
                );
                return None;
            }
            self.budget = self.budget.saturating_sub(cost);
            work = work.saturating_add(cost);
        }
        let room = crate::MAX_OBJECTS.saturating_sub(self.graph.nodes.len()) as u64;
        if work.saturating_add(1) > room {
            host.note("script copy refused: the run's scripts would be more than a map may hold");
            return None;
        }
        let name = format!("{}{number:03}", self.graph.name(original));
        let new_index = host.copy(Ent::Object(index), &name)?;
        let g = Arc::make_mut(&mut self.graph);
        let new_id = g.add_object(&name, ObjKind::Entity(new_index));
        let mut made = Vec::new();
        for s in scripts {
            if let Some((root, pairs)) = g.clone_script(s, new_id) {
                made.push((s, root, pairs));
            }
        }
        let n = self.graph.nodes.len();
        self.blocks.resize(n, BlockState::default());
        self.links.resize(n, LinkState::default());
        self.io.resize(n, false);
        self.values.resize(n, Value::None);
        self.objects
            .resize(self.graph.objects.len(), ObjState::default());
        for (old_root, root, pairs) in made {
            self.order.push(root);
            for (old, new) in pairs {
                let v = if matches!(self.graph.param(new), Some(p) if p.value == Value::Owner) {
                    Value::Obj(Some(new_id))
                } else {
                    self.values.get(old).cloned().unwrap_or_default()
                };
                if let Some(slot) = self.values.get_mut(new) {
                    *slot = v;
                }
            }
            let active = self.is_active(old_root);
            self.activate(root, active, true, 0);
            self.tree_event(root, BlockEvent::Copy, host, 0);
        }
        Some(new_id)
    }

    /// A line for the log, said once per key (a prototype) in a run.
    pub(crate) fn report_once(&mut self, key: crate::Guid, host: &mut dyn Host, text: &str) {
        if !self.unknown.contains(&key) {
            self.unknown.push(key);
            host.note(text);
        }
    }

    /// Spends one unit of the tick's work; false when none is left.
    fn spend(&mut self) -> bool {
        match self.budget.checked_sub(1) {
            Some(b) => {
                self.budget = b;
                true
            }
            None => {
                self.starved = true;
                false
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.graph.scripts.is_empty()
    }

    pub fn is_active(&self, block: Id) -> bool {
        self.blocks.get(block).is_some_and(|b| b.active)
    }

    pub fn io_active(&self, io: Id) -> bool {
        self.io.get(io).copied().unwrap_or(false)
    }

    pub fn set_io(&mut self, io: Id, on: bool) {
        if let Some(x) = self.io.get_mut(io) {
            let rising = on && !*x;
            *x = on;
            if rising {
                self.trace_line(|rt| {
                    let owner = rt.graph.io(io).and_then(|i| i.owner);
                    let side = owner.and_then(|o| rt.graph.block(o)).and_then(|b| {
                        b.inputs
                            .iter()
                            .position(|&x| x == io)
                            .map(|k| format!("in{k}"))
                            .or_else(|| {
                                b.outputs
                                    .iter()
                                    .position(|&x| x == io)
                                    .map(|k| format!("out{k}"))
                            })
                    });
                    format!(
                        "io {} {}",
                        owner.map_or_else(
                            || "?".to_string(),
                            |o| format!("{o} {}", rt.graph.name(o))
                        ),
                        side.unwrap_or_default()
                    )
                });
            }
        }
    }

    pub fn scratch(&self, block: Id) -> &Scratch {
        static NONE: Scratch = Scratch::None;
        self.blocks.get(block).map_or(&NONE, |b| &b.scratch)
    }

    pub fn set_scratch(&mut self, block: Id, s: Scratch) {
        if let Some(b) = self.blocks.get_mut(block) {
            b.scratch = s;
        }
    }

    /// `CKBehavior::Activate`: `reset` (or a behaviour never reset) puts the behaviour back to its start (links,
    /// IOs, sub-behaviours), and a script's first input is activated. Activating a behaviour that was reset before
    /// without asking for a reset keeps its state (it resumes).
    pub fn activate(&mut self, block: Id, active: bool, reset: bool, depth: usize) {
        if depth > MAX_DEPTH || !self.spend() {
            return;
        }
        let done = self.blocks.get(block).is_some_and(|b| b.reset_done);
        if reset || !done {
            self.reset(block, depth);
        }
        if let Some(st) = self.blocks.get_mut(block) {
            st.active = active;
        }
    }

    /// FUN_24005dc8 (docs/ck-runtime.md): links back to their delays, sub-behaviours inactive and reset, IOs off,
    /// requests cleared; a script's input 0 on. The blocks are not told (the original sends CKM_BEHAVIORRESET only
    /// when the play starts); their scratch state is cleared.
    fn reset(&mut self, block: Id, depth: usize) {
        if depth > MAX_DEPTH || !self.spend() {
            return;
        }
        let Some(b) = self.graph.block(block).cloned() else {
            return;
        };
        for &l in &b.links {
            if let (Some(link), Some(st)) = (self.graph.link(l), self.links.get_mut(l)) {
                st.current = link.delay;
                st.queued = false;
            }
        }
        for &s in &b.sub {
            if let Some(st) = self.blocks.get_mut(s) {
                st.active = false;
            }
            self.reset(s, depth.saturating_add(1));
        }
        for &io in b.inputs.iter().chain(&b.outputs) {
            self.set_io(io, false);
        }
        if let Some(st) = self.blocks.get_mut(block) {
            st.scratch = Scratch::None;
            st.stacked = false;
            st.reset_done = true;
            st.request = 0;
        }
        if b.parent.is_none()
            && let Some(&first) = b.inputs.first()
        {
            self.set_io(first, true);
        }
    }

    /// Activate Script / Deactivate Script: a request on the script, applied by the next tick before any script
    /// runs (the original's behaviour manager). Activation wins over a deactivation asked in the same tick only if
    /// asked after it.
    pub fn request(&mut self, script: Id, active: bool, reset: bool) {
        self.requested = true;
        if let Some(st) = self.blocks.get_mut(script) {
            st.request = if active {
                (st.request & !REQ_DEACTIVATE) | REQ_ACTIVATE | if reset { REQ_RESET } else { 0 }
            } else {
                (st.request & !(REQ_ACTIVATE | REQ_RESET)) | REQ_DEACTIVATE
            };
        }
    }

    /// Tells every building block of a behaviour's tree of an event.
    pub(crate) fn tree_event(
        &mut self,
        root: Id,
        e: BlockEvent,
        host: &mut dyn Host,
        depth: usize,
    ) {
        if depth > MAX_DEPTH || !self.spend() {
            return;
        }
        let Some(b) = self.graph.block(root).cloned() else {
            return;
        };
        if b.proto.is_some() {
            blocks::event(self, root, &b, e, host);
        }
        for &s in &b.sub {
            self.tree_event(s, e, host, depth.saturating_add(1));
        }
    }

    /// The scripts attached to an object.
    fn scripts_of(&self, object: Id) -> Vec<Id> {
        self.order
            .iter()
            .copied()
            .filter(|&s| self.graph.block(s).is_some_and(|b| b.owner == Some(object)))
            .collect()
    }

    /// Whether the object is active in the scene (its scripts run). The level and objects the graph does not
    /// hold are.
    pub fn awake(&self, object: Option<Id>) -> bool {
        object
            .and_then(|o| self.objects.get(o))
            .is_none_or(|o| !o.asleep && !o.gone)
    }

    /// Activate Object / Deactivate Object (CKScene::Activate / DeActivate): the object wakes in the scene or
    /// falls asleep; its scripts' blocks are told (ACTIVATESCRIPT / DEACTIVATESCRIPT). Waking also asks for its
    /// scripts' activation (with reset if asked), applied at the next tick as any activation.
    pub fn activate_object(&mut self, object: Id, active: bool, reset: bool, host: &mut dyn Host) {
        let Some(st) = self.objects.get(object).copied() else {
            return;
        };
        if st.gone {
            return;
        }
        let scripts = self.scripts_of(object);
        if active {
            if st.asleep {
                if let Some(o) = self.objects.get_mut(object) {
                    o.asleep = false;
                }
                for &s in &scripts {
                    self.tree_event(s, BlockEvent::ActivateScript, host, 0);
                }
            }
            for s in scripts {
                self.request(s, true, reset);
            }
        } else if !st.asleep {
            if let Some(o) = self.objects.get_mut(object) {
                o.asleep = true;
            }
            for s in scripts {
                self.tree_event(s, BlockEvent::DeactivateScript, host, 0);
            }
        }
    }

    /// Wakes the object a script is attached to if it sleeps (Activate Script's "Awake Object").
    pub(crate) fn wake_owner(&mut self, script: Id, host: &mut dyn Host) {
        let owner = self.graph.block(script).and_then(|b| b.owner);
        if let Some(o) = owner
            && !self.awake(Some(o))
        {
            self.activate_object(o, true, false, host);
        }
    }

    /// Object Delete of a behaviour: its blocks are told, and it never runs again.
    pub fn delete_behavior(&mut self, block: Id, host: &mut dyn Host) {
        if self.blocks.get(block).is_none_or(|b| b.gone) {
            return;
        }
        self.tree_event(block, BlockEvent::Delete, host, 0);
        if let Some(st) = self.blocks.get_mut(block) {
            st.gone = true;
            st.active = false;
            st.request = 0;
        }
    }

    /// Object Delete of an object: its scripts are deleted with it (the manager no longer runs them).
    pub fn delete_object(&mut self, object: Id, host: &mut dyn Host) {
        if self.objects.get(object).is_none_or(|o| o.gone) {
            return;
        }
        for s in self.scripts_of(object) {
            self.delete_behavior(s, host);
        }
        if let Some(o) = self.objects.get_mut(object) {
            o.gone = true;
        }
    }

    /// Execute Script: the script is activated (reset if asked) and run at once, inside the caller's frame; the
    /// manager does not run it again in this tick. False when it cannot be (deleted, already running inside
    /// this frame, nested too deep).
    pub(crate) fn execute_now(&mut self, script: Id, reset: bool, host: &mut dyn Host) -> bool {
        if self.inline.len() >= MAX_INLINE
            || self.inline.contains(&script)
            || self.blocks.get(script).is_none_or(|b| b.gone)
        {
            return false;
        }
        self.activate(script, true, reset, 0);
        self.inline.push(script);
        let keep = self.execute(script, host, 0);
        self.inline.pop();
        let tick = self.tick;
        if let Some(st) = self.blocks.get_mut(script) {
            st.active = keep;
            st.ran_inline = tick;
        }
        true
    }

    pub fn send(&mut self, name: &str, to: Option<Id>) {
        if self.outbox.len() < 4_096 {
            self.outbox.push((name.to_string(), to));
        }
    }

    /// Whether a message of this name reached the block's owner (or everyone) this tick.
    pub fn received(&self, name: &str, owner: Option<Id>) -> bool {
        self.inbox
            .iter()
            .any(|(n, to)| n.eq_ignore_ascii_case(name) && (to.is_none() || *to == owner))
    }

    /// One tick, as the original's behaviour manager runs a frame: first every pending activation request of a
    /// script of an object active in the scene is applied, then every active script of such an object is run, in
    /// the manager's order.
    pub fn tick(&mut self, host: &mut dyn Host) {
        self.tick = self.tick.saturating_add(1);
        self.inbox = std::mem::take(&mut self.outbox);
        self.budget = TICK_BUDGET;
        let tick = self.tick;
        self.trace_line(|_| format!("frame {tick}"));
        let order = self.order.clone();
        // a request asked while the pass runs (a block's callback) waits for the next tick, as in the original
        let pending = std::mem::take(&mut self.requested);
        for &s in order.iter().filter(|_| pending) {
            let Some(st) = self.blocks.get(s) else {
                continue;
            };
            let (req, was, gone) = (st.request, st.active, st.gone);
            if req & (REQ_ACTIVATE | REQ_DEACTIVATE) == 0 || gone {
                continue;
            }
            let owner = self.graph.block(s).and_then(|b| b.owner);
            if !self.awake(owner) {
                // it waits for its object to wake
                self.requested = true;
                continue;
            }
            let on = req & REQ_ACTIVATE != 0;
            self.activate(s, on, req & REQ_RESET != 0, 0);
            if let Some(st) = self.blocks.get_mut(s) {
                st.request = 0;
            }
            self.trace_line(|rt| {
                format!(
                    "script {s} {} {}",
                    rt.graph.name(s),
                    if on { "activated" } else { "deactivated" }
                )
            });
            if was != on {
                let e = if on {
                    BlockEvent::ActivateScript
                } else {
                    BlockEvent::DeactivateScript
                };
                self.tree_event(s, e, host, 0);
            }
        }
        for s in order {
            let owner = self.graph.block(s).and_then(|b| b.owner);
            let runs = self
                .blocks
                .get(s)
                .is_some_and(|b| b.active && !b.gone && b.ran_inline != tick);
            if runs && self.awake(owner) {
                let keep = self.execute(s, host, 0);
                if let Some(st) = self.blocks.get_mut(s) {
                    st.active = keep;
                }
            }
        }
        if self.starved {
            self.starved = false;
            host.note("script work stopped: a tick took more than the budget");
        }
    }

    /// `CKBehavior::Execute`: a building block runs its prototype; a graph fires links and runs its stack.
    /// Returns whether the behaviour stays active.
    fn execute(&mut self, block: Id, host: &mut dyn Host, depth: usize) -> bool {
        if depth > MAX_DEPTH || !self.spend() {
            return false;
        }
        let Some(b) = self.graph.block(block).cloned() else {
            return false;
        };
        if self.blocks.get(block).is_some_and(|st| st.gone) {
            return false;
        }
        self.trace_line(|rt| {
            format!(
                "{} {block} {}",
                if b.proto.is_some() { "run" } else { "graph" },
                rt.graph.name(block)
            )
        });
        if let Some(proto) = b.proto {
            return match blocks::run(self, block, &b, proto, host) {
                Flow::Done => false,
                Flow::Continue => true,
                Flow::Unknown => {
                    if !self.unknown.contains(&proto) {
                        self.unknown.push(proto);
                        host.note(&format!(
                            "script block not known: {} ({proto}) in {}",
                            b.name,
                            self.graph.name(block)
                        ));
                    }
                    // Its inputs are cleared so the graph does not spin on it; nothing comes out.
                    for &io in &b.inputs {
                        self.set_io(io, false);
                    }
                    false
                }
            };
        }
        // Prepare: fire the links from active IOs, then stack the active sub-behaviours (first on top).
        let mut sources = Vec::new();
        let mut to_activate = Vec::new();
        for &l in &b.links {
            let Some(link) = self.graph.link(l) else {
                continue;
            };
            if self.io_active(link.src) {
                sources.push(link.src);
                if link.delay == 0 {
                    to_activate.push(link.dst);
                } else {
                    self.queue(l, link.delay);
                }
            }
        }
        for s in sources {
            self.set_io(s, false);
        }
        for dst in to_activate {
            self.activate_io(dst, block);
        }
        let mut stack: Vec<Id> = Vec::new();
        for &s in b.sub.iter().rev() {
            if self.is_active(s) {
                stack.push(s);
                if let Some(st) = self.blocks.get_mut(s) {
                    st.stacked = true;
                }
            }
        }
        // Loop: run the top, propagate its outputs, until the stack is empty.
        let mut iterations = 0usize;
        while let Some(top) = stack.pop() {
            iterations = iterations.saturating_add(1);
            if iterations > MAX_ITERATIONS || self.budget == 0 {
                host.note(&format!("script loop stopped in {}", b.name));
                break;
            }
            if let Some(st) = self.blocks.get_mut(top) {
                st.stacked = false;
            }
            let keep = self.execute(top, host, depth.saturating_add(1));
            if let Some(st) = self.blocks.get_mut(top) {
                st.active = keep;
            }
            let Some(tb) = self.graph.block(top) else {
                continue;
            };
            let outputs = tb.outputs.clone();
            for out in outputs {
                if !self.io_active(out) {
                    continue;
                }
                self.set_io(out, false);
                let links = self.graph.links_from.get(out).cloned().unwrap_or_default();
                for l in links {
                    let Some(link) = self.graph.link(l) else {
                        continue;
                    };
                    // only this graph's links carry the activation on
                    if !b.links.contains(&l) {
                        continue;
                    }
                    if link.delay == 0 {
                        if let Some(c) = self.activate_io(link.dst, block) {
                            let stacked = self.blocks.get(c).is_some_and(|s| s.stacked);
                            if !stacked {
                                self.push_by_priority(&mut stack, c);
                            }
                        }
                    } else {
                        self.queue(l, link.delay);
                    }
                }
            }
        }
        // End: delayed links count down; the graph stays active while any is queued or a sub-behaviour is.
        let mut keep = false;
        for &l in &b.links {
            let Some(link) = self.graph.link(l) else {
                continue;
            };
            let Some(st) = self.links.get_mut(l) else {
                continue;
            };
            if !st.queued {
                continue;
            }
            st.current = st.current.saturating_sub(1);
            if st.current == 0 {
                st.queued = false;
                self.activate_io(link.dst, block);
            } else {
                keep = true;
            }
        }
        keep || b.sub.iter().any(|&s| self.is_active(s))
    }

    fn queue(&mut self, link: Id, delay: u16) {
        if let Some(st) = self.links.get_mut(link) {
            if st.current == 0 {
                st.current = delay;
            }
            st.queued = true;
        }
    }

    /// Activates an IO; an input of a sub-behaviour activates that behaviour too, which is returned.
    fn activate_io(&mut self, io: Id, graph: Id) -> Option<Id> {
        self.set_io(io, true);
        let info = self.graph.io(io)?;
        let owner = info.owner?;
        if info.is_input && owner != graph {
            if let Some(st) = self.blocks.get_mut(owner) {
                st.active = true;
            }
            return Some(owner);
        }
        None
    }

    /// Inserts a behaviour into the stack above every entry of a priority not above its own.
    fn push_by_priority(&mut self, stack: &mut Vec<Id>, block: Id) {
        let prio = self.graph.block(block).map_or(0, |b| b.priority);
        let mut at = stack.len();
        while at > 0 {
            let below = stack.get(at.wrapping_sub(1)).copied();
            let p = below
                .and_then(|x| self.graph.block(x))
                .map_or(0, |b| b.priority);
            if p <= prio {
                break;
            }
            at = at.wrapping_sub(1);
        }
        stack.insert(at.min(stack.len()), block);
        if let Some(st) = self.blocks.get_mut(block) {
            st.stacked = true;
        }
    }

    // ---- arrays -----------------------------------------------------------------------------------------------

    pub fn rows(&self, array: usize) -> &[Vec<Value>] {
        self.arrays.get(array).map_or(&[], Vec::as_slice)
    }

    pub fn cell(&self, array: usize, row: usize, column: usize) -> Option<Value> {
        self.arrays.get(array)?.get(row)?.get(column).cloned()
    }

    pub fn set_cell(&mut self, array: usize, row: usize, column: usize, v: Value) {
        if let Some(c) = self
            .arrays
            .get_mut(array)
            .and_then(|a| a.get_mut(row))
            .and_then(|r| r.get_mut(column))
        {
            *c = v;
        }
    }

    pub fn add_row(&mut self, array: usize, row: Vec<Value>) {
        if let Some(a) = self.arrays.get_mut(array)
            && a.len() < crate::MAX_ARRAY_ROWS
        {
            a.push(row);
        }
    }

    /// Removes a row; false when there is none at the index.
    pub fn remove_row(&mut self, array: usize, row: usize) -> bool {
        match self.arrays.get_mut(array) {
            Some(a) if row < a.len() => {
                a.remove(row);
                true
            }
            _ => false,
        }
    }

    /// Object Rename as its author meant it (only behind the `absent-as-authored` feature: the player has no such
    /// block, docs/drahtseilakt/scripts.md): the object answers to the new name from now on (a name is kept to 256 bytes).
    #[cfg_attr(not(feature = "absent-as-authored"), allow(dead_code))]
    pub(crate) fn rename(&mut self, object: Id, name: &str) {
        if object >= self.graph.objects.len() {
            return;
        }
        let mut end = name.len().min(256);
        while !name.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        if let Some(o) = Arc::make_mut(&mut self.graph).objects.get_mut(object) {
            o.name = name.get(..end).unwrap_or_default().to_string();
        }
    }

    pub fn clear_rows(&mut self, array: usize) {
        if let Some(a) = self.arrays.get_mut(array) {
            a.clear();
        }
    }

    // ---- parameters -------------------------------------------------------------------------------------------

    /// Reads a parameter: an input follows its source (running the operation that computes it first).
    pub fn read(&mut self, param: Id, host: &mut dyn Host) -> Value {
        self.read_depth(param, host, 0)
    }

    fn read_depth(&mut self, param: Id, host: &mut dyn Host, depth: usize) -> Value {
        if depth > MAX_DEPTH || !self.spend() {
            return Value::None;
        }
        let Some(p) = self.graph.param(param).cloned() else {
            return Value::None;
        };
        if let Some(src) = p.source {
            return self.read_depth(src, host, depth.saturating_add(1));
        }
        if let Some(op) = p.op {
            self.run_op(op, host, depth.saturating_add(1));
        }
        self.values.get(param).cloned().unwrap_or_default()
    }

    /// Writes a parameter and copies it on to its destinations.
    pub fn write(&mut self, param: Id, v: Value) {
        self.write_depth(param, v, 0);
    }

    fn write_depth(&mut self, param: Id, v: Value, depth: usize) {
        if depth > 8 || !self.spend() {
            return;
        }
        let Some(slot) = self.values.get_mut(param) else {
            return;
        };
        *slot = v.clone();
        self.trace_line(|_| format!("param {param} = {v:?}"));
        let dests = self
            .graph
            .param(param)
            .map(|p| p.destinations.clone())
            .unwrap_or_default();
        for d in dests {
            self.write_depth(d, v.clone(), depth.saturating_add(1));
        }
    }

    fn run_op(&mut self, op: Id, host: &mut dyn Host, depth: usize) {
        let Some(Node::Op(o)) = self.graph.nodes.get(op).cloned() else {
            return;
        };
        let a = o.inputs[0].map_or(Value::None, |i| self.read_depth(i, host, depth));
        let b = o.inputs[1].map_or(Value::None, |i| self.read_depth(i, host, depth));
        let out_guid = o.output.and_then(|x| self.graph.param(x)).map(|p| p.guid);
        let a_guid = o.inputs[0]
            .and_then(|x| self.graph.param(x))
            .map(|p| p.guid);
        let r = blocks::operation(self, host, o.guid, out_guid, a_guid, &a, &b);
        if let Some(out) = o.output {
            self.write(out, r);
        }
    }

    /// The pin of a block by name (case-insensitively), else by index.
    pub fn pin(&self, block: &crate::Block, name: &str, index: usize) -> Option<Id> {
        block
            .pins
            .iter()
            .copied()
            .find(|&p| {
                self.graph
                    .param(p)
                    .is_some_and(|q| q.name.eq_ignore_ascii_case(name))
            })
            .or_else(|| block.pins.get(index).copied())
    }

    pub fn pout(&self, block: &crate::Block, name: &str, index: usize) -> Option<Id> {
        block
            .pouts
            .iter()
            .copied()
            .find(|&p| {
                self.graph
                    .param(p)
                    .is_some_and(|q| q.name.eq_ignore_ascii_case(name))
            })
            .or_else(|| block.pouts.get(index).copied())
    }

    /// The entity a block acts on: its target parameter, else the script's owner.
    pub fn target(&mut self, block: &crate::Block, host: &mut dyn Host) -> Option<Id> {
        match block.target {
            Some(t) => self.read(t, host).as_obj(),
            None => block.owner,
        }
    }

    /// An object of the graph as an engine entity, when it is one.
    pub fn ent(&self, id: Option<Id>) -> Option<Ent> {
        match self.graph.objects.get(id?)?.kind {
            ObjKind::Entity(i) => Some(Ent::Object(i)),
            ObjKind::Ball => Some(Ent::Ball),
            _ => None,
        }
    }
}

/// The behaviour manager's order: objects with scripts by priority, higher first (stable, so the file's order
/// among equals), each object's scripts in the order the graph lists them; scripts without an owner last.
fn run_order(g: &Graph) -> Vec<Id> {
    let mut owners: Vec<Option<Id>> = Vec::new();
    let mut by_owner: std::collections::HashMap<Option<Id>, Vec<Id>> =
        std::collections::HashMap::new();
    for &s in &g.scripts {
        let owner = g.block(s).and_then(|b| b.owner);
        let list = by_owner.entry(owner).or_default();
        if list.is_empty() {
            owners.push(owner);
        }
        list.push(s);
    }
    let prio = |o: &Option<Id>| match o {
        Some(o) => g
            .priorities
            .iter()
            .find(|(id, _)| id == o)
            .map_or(0, |&(_, p)| p),
        None => i32::MIN,
    };
    owners.sort_by_key(|o| std::cmp::Reverse(prio(o)));
    owners
        .into_iter()
        .flat_map(|o| by_owner.remove(&o).unwrap_or_default())
        .collect()
}

/// A host that is told nothing and has no objects: the runtime's start, before the engine runs, and tools that run
/// a graph alone (script-dump --run).
pub struct Quiet;

impl Host for Quiet {
    fn delta_ms(&self) -> f32 {
        0.0
    }
    fn matrix(&self, _: Ent) -> Option<[[f32; 4]; 4]> {
        None
    }
    fn initial_matrix(&self, _: Ent) -> Option<[[f32; 4]; 4]> {
        None
    }
    fn set_matrix(&mut self, _: Ent, _: [[f32; 4]; 4]) {}
    fn set_visible(&mut self, _: Ent, _: bool) {}
    fn bounds(&self, _: Ent) -> Option<([f32; 3], [f32; 3])> {
        None
    }
    fn delete(&mut self, _: Ent) {}
    fn physicalize(&mut self, _: Ent, _: bool) {}
    fn particles(&mut self, _: Ent, _: bool) {}
    fn mesh_of(&self, _: Ent) -> Option<usize> {
        None
    }
    fn group_members(&self, _: usize) -> Vec<usize> {
        Vec::new()
    }
    fn game_cell(&self, _: &str, _: usize, _: usize) -> Option<Value> {
        None
    }
    fn message(&mut self, _: &str, _: Option<Ent>) {}
    fn sound(&mut self, _: &str, _: bool) {}
    fn text(&mut self, _: &str, _: &str, _: bool) {}
    fn note(&mut self, _: &str) {}
}
