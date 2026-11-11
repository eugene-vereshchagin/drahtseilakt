// The runtime: runs a map's scripts one tick at a time the way the original's behaviour manager does
// (docs/ck-runtime.md, "Execution"): each active script is executed; a graph fires the links from its active
// IOs (a zero-delay link in the same tick, a delayed one that many ticks later), runs its active sub-behaviours
// from a priority stack, and stays active while links are queued or sub-behaviours active. Building blocks run
// through blocks.rs. Messages sent in a tick reach their waiters in the next. All order is by index, so a run is
// deterministic. The state is small and cloned with the run (rewind, replays); the graph is shared.

use crate::blocks::{self, Flow};
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
}

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
}

/// The most objects a run's scripts may copy.
pub const MAX_COPIES: usize = 256;

/// The most pops of the execution stack in one graph in one tick (the original's BehaviorMaxIteration).
const MAX_ITERATIONS: usize = 8_000;
const MAX_DEPTH: usize = 48;
/// The work one tick of all the scripts may do (a map's own take a few thousand).
const TICK_BUDGET: u64 = 300_000;

impl Runtime {
    /// A runtime with every script active and reset, as scripts of a loaded level start.
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
        };
        let scripts = rt.graph.scripts.clone();
        for s in scripts {
            rt.bind_owner(s);
            rt.activate(s, true, true, 0);
        }
        rt
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
        for (old_root, root, pairs) in made {
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
        }
        Some(new_id)
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
            *x = on;
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

    /// `CKBehavior::Activate`: `reset` puts the behaviour back to its start (links, IOs, sub-behaviours), and a
    /// script's first input is activated.
    pub fn activate(&mut self, block: Id, active: bool, reset: bool, depth: usize) {
        if depth > MAX_DEPTH || !self.spend() {
            return;
        }
        let Some(b) = self.graph.block(block).cloned() else {
            return;
        };
        let was_off = !self.is_active(block);
        if reset || was_off {
            for &l in &b.links {
                if let (Some(link), Some(st)) = (self.graph.link(l), self.links.get_mut(l)) {
                    st.current = link.delay;
                    st.queued = false;
                }
            }
            for &s in &b.sub {
                self.activate(s, false, true, depth.saturating_add(1));
            }
            for &io in b.inputs.iter().chain(&b.outputs) {
                self.set_io(io, false);
            }
            if let Some(st) = self.blocks.get_mut(block) {
                st.scratch = Scratch::None;
                st.stacked = false;
            }
            if b.parent.is_none()
                && let Some(&first) = b.inputs.first()
            {
                self.set_io(first, true);
            }
        }
        if let Some(st) = self.blocks.get_mut(block) {
            st.active = active;
        }
    }

    /// Every script's scripts of an object (its graphs attached): activated or deactivated.
    pub fn activate_object(&mut self, object: Id, active: bool, reset: bool) {
        let scripts: Vec<Id> = self
            .graph
            .scripts
            .iter()
            .copied()
            .filter(|&s| self.graph.block(s).is_some_and(|b| b.owner == Some(object)))
            .collect();
        for s in scripts {
            self.activate(s, active, reset, 0);
        }
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

    /// One tick of every active script, in file order.
    pub fn tick(&mut self, host: &mut dyn Host) {
        self.tick = self.tick.saturating_add(1);
        self.inbox = std::mem::take(&mut self.outbox);
        self.budget = TICK_BUDGET;
        let scripts = self.graph.scripts.clone();
        for s in scripts {
            if self.is_active(s) {
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

    /// Object Rename: the object answers to the new name from now on (a name is kept to 256 bytes).
    pub fn rename(&mut self, object: Id, name: &str) {
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
