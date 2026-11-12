// Unit tests of the runtime and its blocks on graphs built by hand (no game data): link delays and the stack,
// parameter pull through operations and destinations, iterators, messages, the proximity test, Hide and Set
// Position through a recording host.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::panic
)]

use crate::blocks::proto;
use crate::exec::{Ent, Host};
use crate::{Block, Graph, Guid, Id, Io, Link, Node, ObjKind, Param, Runtime, Value, types};
use crate::{CameraChange, CameraState};
use std::sync::Arc;

/// A host that records what the scripts asked of it; two level objects (0 and 1) and the ball.
#[derive(Default)]
struct Rec {
    matrices: Vec<[[f32; 4]; 4]>,
    visible: Vec<bool>,
    notes: Vec<String>,
    messages: Vec<String>,
    /// The sound commands: "key file Command".
    sounds: Vec<String>,
    playing: bool,
    ball: [f32; 3],
    in_play: bool,
    frame: crate::InputFrame,
    before: crate::InputFrame,
    cursor: Option<bool>,
    /// The camera changes the scripts asked for.
    cams: Vec<CameraChange>,
    /// The light, fog and sky changes the scripts asked for.
    env: Vec<crate::EnvChange>,
    /// The material and texture changes the scripts asked for.
    mats: Vec<(usize, crate::MatChange)>,
    texs: Vec<(usize, crate::TexChange)>,
    /// Every text box call: the block and its spec (none: taken away).
    boxes: Vec<(usize, Option<crate::TextSpec>)>,
    /// Every emitter call: block, entity, values, state.
    emitters: Vec<(usize, Ent, crate::EmitterSpec, crate::EmitState)>,
}

fn ident(x: f32, y: f32, z: f32) -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [x, y, z, 1.0],
    ]
}

impl Rec {
    fn new() -> Rec {
        Rec {
            matrices: vec![ident(0.0, 0.0, 0.0), ident(10.0, 0.0, 0.0)],
            visible: vec![true, true],
            in_play: true,
            ..Rec::default()
        }
    }
}

impl Host for Rec {
    fn delta_ms(&self) -> f32 {
        1000.0 / 60.0
    }
    fn matrix(&self, e: Ent) -> Option<[[f32; 4]; 4]> {
        match e {
            Ent::Ball => Some(ident(self.ball[0], self.ball[1], self.ball[2])),
            Ent::Object(i) => self.matrices.get(i).copied(),
        }
    }
    fn initial_matrix(&self, e: Ent) -> Option<[[f32; 4]; 4]> {
        match e {
            Ent::Object(i) => Some(ident(10.0 * i as f32, 0.0, 0.0)),
            Ent::Ball => None,
        }
    }
    fn set_matrix(&mut self, e: Ent, m: [[f32; 4]; 4]) {
        match e {
            Ent::Ball => self.ball = [m[3][0], m[3][1], m[3][2]],
            Ent::Object(i) => self.matrices[i] = m,
        }
    }
    fn set_visible(&mut self, e: Ent, on: bool) {
        if let Ent::Object(i) = e {
            self.visible[i] = on;
        }
    }
    fn bounds(&self, e: Ent) -> Option<([f32; 3], [f32; 3])> {
        let m = self.matrix(e)?;
        let p = [m[3][0], m[3][1], m[3][2]];
        Some((
            [p[0] - 1.0, p[1] - 1.0, p[2] - 1.0],
            [p[0] + 1.0, p[1] + 1.0, p[2] + 1.0],
        ))
    }
    fn delete(&mut self, e: Ent) {
        self.set_visible(e, false);
    }
    fn physicalize(&mut self, _: Ent, _: bool) {}
    fn particles(&mut self, _: Ent, on: bool) {
        self.notes.push(format!("particles {on}"));
    }
    fn emitter(&mut self, key: usize, e: Ent, spec: &crate::EmitterSpec, state: crate::EmitState) {
        self.emitters.push((key, e, spec.clone(), state));
    }
    fn mesh_of(&self, _: Ent) -> Option<usize> {
        None
    }
    fn group_members(&self, _: usize) -> Vec<usize> {
        vec![0, 1]
    }
    fn game_cell(&self, array: &str, row: usize, column: usize) -> Option<Value> {
        (array == "CurrentLevel" && row == 0 && column == 4).then_some(Value::Bool(self.in_play))
    }
    fn message(&mut self, name: &str, _: Option<Ent>) {
        self.messages.push(name.to_string());
    }
    fn camera_get(&self) -> CameraState {
        CameraState {
            position: [1.0, 2.0, 3.0],
            object: Some(7),
            ..CameraState::default()
        }
    }
    fn camera_set(&mut self, change: CameraChange) {
        self.cams.push(change);
    }
    fn env_set(&mut self, change: crate::EnvChange) {
        self.env.push(change);
    }
    fn material_set(&mut self, material: usize, change: crate::MatChange) {
        self.mats.push((material, change));
    }
    fn texture_set(&mut self, texture: usize, change: crate::TexChange) {
        self.texs.push((texture, change));
    }
    fn texture_info(&self, texture: usize) -> Option<crate::TexInfo> {
        (texture == 0).then(|| crate::TexInfo {
            file: "wall.bmp".into(),
            width: 64,
            height: 32,
            slot: 3,
        })
    }
    fn sound(&mut self, key: usize, file: &str, cmd: crate::SoundCmd) {
        self.sounds.push(format!("{key} {file} {cmd:?}"));
    }
    fn sound_playing(&self, _: usize) -> bool {
        self.playing
    }
    fn text_box(&mut self, key: usize, spec: Option<crate::TextSpec>) {
        if let Some(s) = &spec {
            if self
                .boxes
                .last()
                .and_then(|(k, p)| (*k == key).then_some(p))
                != Some(&spec)
            {
                self.notes.push(format!("text {}", s.text.trim_end()));
            }
        }
        self.boxes.push((key, spec));
    }
    fn note(&mut self, text: &str) {
        self.notes.push(text.to_string());
    }
    fn copy(&mut self, e: Ent, name: &str) -> Option<usize> {
        let m = self.matrix(e)?;
        self.matrices.push(m);
        self.visible.push(true);
        self.notes.push(format!("copy {name}"));
        Some(self.matrices.len() - 1)
    }
    fn input(&self) -> crate::InputFrame {
        self.frame
    }
    fn input_before(&self) -> crate::InputFrame {
        self.before
    }
    fn show_cursor(&mut self, on: bool) {
        self.cursor = Some(on);
    }
    fn group_add(&mut self, group: usize, object: usize) {
        self.notes.push(format!("group {group} + {object}"));
    }
    fn group_remove(&mut self, group: usize, object: usize) {
        self.notes.push(format!("group {group} - {object}"));
    }
}

/// Builds graphs node by node; ids are handed out in order.
struct Build {
    g: Graph,
}

impl Build {
    fn new() -> Build {
        let mut g = Graph::default();
        // objects 0 and 1 are the level's entities A and B, 2 a group, 3 the ball frame, 4 CurrentLevel
        for (name, kind) in [
            ("A", ObjKind::Entity(0)),
            ("B", ObjKind::Entity(1)),
            ("G", ObjKind::Group(0)),
            ("Ball_Pos_Frame", ObjKind::Ball),
            ("CurrentLevel", ObjKind::Other),
        ] {
            g.add_object(name, kind);
        }
        Build { g }
    }

    fn node(&mut self, name: &str, n: Node) -> Id {
        let id = self.g.add_object(name, ObjKind::Other);
        self.g.nodes[id] = n;
        id
    }

    fn io(&mut self, name: &str) -> Id {
        self.node(name, Node::Io(Io::default()))
    }

    fn local(&mut self, name: &str, guid: Guid, v: Value) -> Id {
        self.node(
            name,
            Node::Param(Param {
                name: name.into(),
                guid,
                value: v,
                ..Param::default()
            }),
        )
    }

    fn pin(&mut self, name: &str, guid: Guid, source: Id) -> Id {
        self.node(
            name,
            Node::Param(Param {
                name: name.into(),
                guid,
                source: Some(source),
                ..Param::default()
            }),
        )
    }

    fn pout(&mut self, name: &str, guid: Guid) -> Id {
        self.local(name, guid, Value::None)
    }

    /// A building block with `ins` inputs and `outs` outputs named as given.
    fn bb(
        &mut self,
        name: &str,
        proto: Guid,
        ins: &[&str],
        outs: &[&str],
        pins: Vec<Id>,
        pouts: Vec<Id>,
        locals: Vec<Id>,
        target: Option<Id>,
    ) -> Id {
        let inputs: Vec<Id> = ins.iter().map(|n| self.io(n)).collect();
        let outputs: Vec<Id> = outs.iter().map(|n| self.io(n)).collect();
        self.node(
            name,
            Node::Block(Block {
                name: name.into(),
                proto: Some(proto),
                inputs,
                outputs,
                pins,
                pouts,
                locals,
                target,
                ..Block::default()
            }),
        )
    }

    fn link(&mut self, src: Id, dst: Id, delay: u16) -> Id {
        self.node("link", Node::Link(Link { delay, src, dst }))
    }

    /// A script (top-level graph) owned by object `owner`, with the one input `start`, holding `sub` and `links`.
    fn script(
        &mut self,
        name: &str,
        owner: Option<Id>,
        start: Id,
        sub: Vec<Id>,
        links: Vec<Id>,
    ) -> Id {
        let id = self.node(
            name,
            Node::Block(Block {
                name: name.into(),
                inputs: vec![start],
                sub,
                links,
                flags: 0x2,
                ..Block::default()
            }),
        );
        self.g.nodes[id] = match self.g.nodes[id].clone() {
            Node::Block(b) => Node::Block(Block { owner, ..b }),
            n => n,
        };
        id
    }

    fn done(mut self) -> Arc<Graph> {
        self.g.wire_for_tests();
        Arc::new(self.g)
    }
}

impl Graph {
    fn wire_for_tests(&mut self) {
        self.links_from = vec![Vec::new(); self.nodes.len()];
        self.wire();
        self.scripts = (0..self.nodes.len())
            .filter(|&i| matches!(&self.nodes[i], Node::Block(b) if b.parent.is_none()))
            .collect();
        let roots = self.scripts.clone();
        for r in roots {
            let owner = self.block(r).and_then(|b| b.owner);
            self.set_owner(r, owner, 0);
        }
    }
}

fn in_of(g: &Graph, block: Id, i: usize) -> Id {
    g.block(block).unwrap().inputs[i]
}
fn out_of(g: &Graph, block: Id, i: usize) -> Id {
    g.block(block).unwrap().outputs[i]
}

#[test]
fn a_hide_fires_in_the_tick_of_its_link_and_a_delayed_link_later() {
    let mut b = Build::new();
    let hier = b.local("Hierarchy", types::BOOL, Value::Bool(false));
    let target_a = b.local("Target", types::ENTITY, Value::Obj(Some(0)));
    let target_b = b.local("Target", types::ENTITY, Value::Obj(Some(1)));
    let hide_a = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![hier],
        vec![],
        vec![],
        Some(target_a),
    );
    let hide_b = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![hier],
        vec![],
        vec![],
        Some(target_b),
    );
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, hide_a, 0), 0);
    let l2 = b.link(out_of(&b.g, hide_a, 0), in_of(&b.g, hide_b, 0), 3);
    let script = b.node(
        "Script",
        Node::Block(Block {
            name: "S".into(),
            inputs: vec![start],
            sub: vec![hide_a, hide_b],
            links: vec![l1, l2],
            ..Block::default()
        }),
    );
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![false, true],
        "A hidden in the first tick"
    );
    assert!(
        rt.is_active(script),
        "the script stays active while a link is queued"
    );
    rt.tick(&mut host);
    rt.tick(&mut host);
    assert_eq!(host.visible, vec![false, true], "B waits for the delay");
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![false, false],
        "B hidden three ticks after A"
    );
    rt.tick(&mut host);
    assert!(!rt.is_active(script), "nothing left to do");
}

#[test]
fn get_cell_on_the_game_table_feeds_a_binary_switch_and_op_finds_objects_by_name() {
    let mut b = Build::new();
    // Op: p1 "CurrentLevel" -> res (object); the operation GUID in locals 1 and 2
    let p1 = b.local("p1", types::STRING, Value::Str("CurrentLevel".into()));
    let p1_pin = b.pin("p1", types::STRING, p1);
    let p2_pin = b.pin("p2", types::NONE, p1);
    let res = b.pout("res", types::OBJECT);
    let fp = b.local("", Guid(0, 0), Value::Raw(vec![0; 8]));
    let ga = b.local(
        "",
        Guid(0, 0),
        Value::Raw(0x4280_09b4u32.to_le_bytes().to_vec()),
    );
    let gb = b.local(
        "",
        Guid(0, 0),
        Value::Raw(0x1caa_5c78u32.to_le_bytes().to_vec()),
    );
    let op = b.bb(
        "Op",
        proto::OP,
        &["In"],
        &["Out"],
        vec![p1_pin, p2_pin],
        vec![res],
        vec![fp, ga, gb],
        None,
    );
    // Get Cell row 0 column 4 of the Op's result
    let row = b.local("Row Index", types::INT, Value::Int(0));
    let col = b.local("Column Index", types::INT, Value::Int(4));
    let row_pin = b.pin("Row Index", types::INT, row);
    let col_pin = b.pin("Column Index", types::INT, col);
    let cell = b.pout("Cell Value", types::BOOL);
    let target = b.pin("Target", types::ARRAY, res);
    let get = b.bb(
        "Get Cell",
        proto::GET_CELL,
        &["In"],
        &["Found", "Not Found"],
        vec![row_pin, col_pin],
        vec![cell],
        vec![],
        Some(target),
    );
    let cond = b.pin("Condition", types::BOOL, cell);
    let sw = b.bb(
        "Binary Switch",
        proto::BINARY_SWITCH,
        &["In"],
        &["True", "False"],
        vec![cond],
        vec![],
        vec![],
        None,
    );
    let hier = b.local("Hierarchy", types::BOOL, Value::Bool(false));
    let hide = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![hier],
        vec![],
        vec![],
        None,
    );
    let start = b.io("Start");
    let links = vec![
        b.link(start, in_of(&b.g, op, 0), 0),
        b.link(out_of(&b.g, op, 0), in_of(&b.g, get, 0), 0),
        b.link(out_of(&b.g, get, 0), in_of(&b.g, sw, 0), 0),
        b.link(out_of(&b.g, sw, 0), in_of(&b.g, hide, 0), 0),
    ];
    // the script belongs to object B: Hide without a target hides B
    let _script = b.node(
        "Script",
        Node::Block(Block {
            name: "S".into(),
            inputs: vec![start],
            sub: vec![op, get, sw, hide],
            links,
            owner: Some(1),
            ..Block::default()
        }),
    );
    let g = b.done();
    let mut rt = Runtime::new(g.clone());
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(rt.values[res], Value::Obj(g.find("CurrentLevel")));
    assert_eq!(rt.values[cell], Value::Bool(true));
    assert_eq!(
        host.visible,
        vec![true, false],
        "in play: the owner B is hidden"
    );

    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    host.in_play = false;
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![true, true],
        "not in play: the False branch, nothing hidden"
    );
}

#[test]
fn an_iterator_loops_over_a_map_array_and_set_position_moves_the_rows_objects() {
    let mut b = Build::new();
    b.g.arrays.push(crate::Array {
        name: "Moves".into(),
        columns: vec![("Object".into(), 4), ("Position".into(), 5)],
        rows: vec![
            vec![Value::Obj(Some(0)), Value::Vec3([1.0, 2.0, 3.0])],
            vec![Value::Obj(Some(1)), Value::Vec3([4.0, 5.0, 6.0])],
        ],
        key: None,
    });
    let arr = b.g.add_object("Moves", ObjKind::Array(0));
    let arr_param = b.local("Target (Array)", types::ARRAY, Value::Obj(Some(arr)));
    let first = b.local("Starting Row", types::INT, Value::Int(0));
    let last = b.local("Ending Row", types::INT, Value::Int(-1));
    let back = b.local("Backward", types::BOOL, Value::Bool(false));
    let pins = vec![
        b.pin("Starting Row", types::INT, first),
        b.pin("Ending Row", types::INT, last),
        b.pin("Backward", types::BOOL, back),
    ];
    let row_out = b.pout("Row Index", types::INT);
    let obj_out = b.pout("Object", types::ENTITY);
    let pos_out = b.pout("Position", types::VECTOR);
    let it = b.bb(
        "Iterator",
        proto::ITERATOR,
        &["In", "Loop In"],
        &["Out", "Loop Out"],
        pins,
        vec![row_out, obj_out, pos_out],
        vec![],
        Some(arr_param),
    );
    let pos_pin = b.pin("Position", types::VECTOR, pos_out);
    let none = b.local("Referential", types::ENTITY, Value::Obj(None));
    let ref_pin = b.pin("Referential", types::ENTITY, none);
    let target = b.pin("Target", types::ENTITY, obj_out);
    let set = b.bb(
        "Set Position",
        proto::SET_POSITION,
        &["In"],
        &["Out"],
        vec![pos_pin, ref_pin],
        vec![],
        vec![],
        Some(target),
    );
    let start = b.io("Start");
    let links = vec![
        b.link(start, in_of(&b.g, it, 0), 0),
        b.link(out_of(&b.g, it, 1), in_of(&b.g, set, 0), 0),
        b.link(out_of(&b.g, set, 0), in_of(&b.g, it, 1), 0),
    ];
    let script = b.node(
        "Script",
        Node::Block(Block {
            name: "S".into(),
            inputs: vec![start],
            sub: vec![it, set],
            links,
            ..Block::default()
        }),
    );
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(host.matrices[0][3], [1.0, 2.0, 3.0, 1.0]);
    assert_eq!(host.matrices[1][3], [4.0, 5.0, 6.0, 1.0]);
    // docs/gameplay_bbs.md, "Iterator": the step past the last row wraps Row Index to 0, the stop row
    assert_eq!(rt.values[row_out], Value::Int(0));
    assert!(!rt.is_active(script), "both rows done within one tick");
}

#[test]
fn a_message_sent_in_one_tick_wakes_its_waiter_in_the_next() {
    let mut b = Build::new();
    let msg = b.local("Message", types::MESSAGE, Value::Str("Go".into()));
    let msg_pin = b.pin("Message", types::MESSAGE, msg);
    let wait = b.bb(
        "Wait Message",
        proto::WAIT_MESSAGE,
        &["In", "Stop"],
        &["Out"],
        vec![msg_pin],
        vec![],
        vec![],
        None,
    );
    let hier = b.local("Hierarchy", types::BOOL, Value::Bool(false));
    let hide = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![hier],
        vec![],
        vec![],
        None,
    );
    let start = b.io("Start");
    let l = vec![
        b.link(start, in_of(&b.g, wait, 0), 0),
        b.link(out_of(&b.g, wait, 0), in_of(&b.g, hide, 0), 0),
    ];
    let waiter = b.script("Waiter", Some(0), start, vec![wait, hide], l);
    let msg2_pin = b.pin("Message", types::MESSAGE, msg);
    let dest = b.local("Group", types::GROUP, Value::Obj(Some(2)));
    let dest_pin = b.pin("Group", types::GROUP, dest);
    let send = b.bb(
        "Send Message To Group",
        proto::SEND_MESSAGE_TO_GROUP,
        &["In"],
        &["Out"],
        vec![msg2_pin, dest_pin],
        vec![],
        vec![],
        None,
    );
    let start2 = b.io("Start");
    let delayed = b.local("Time to Wait", types::TIME, Value::Float(40.0));
    let wait_pin = b.pin("Time to Wait", types::TIME, delayed);
    // the delayer counts in its Elapsed Time output (docs/gameplay_bbs.md, "Delayer")
    let elapsed = b.pout("Elapsed Time", types::TIME);
    let delayer = b.bb(
        "Delayer",
        proto::DELAYER,
        &["In"],
        &["Out"],
        vec![wait_pin],
        vec![elapsed],
        vec![],
        None,
    );
    let l2 = vec![
        b.link(start2, in_of(&b.g, delayer, 0), 0),
        b.link(out_of(&b.g, delayer, 0), in_of(&b.g, send, 0), 0),
    ];
    let sender = b.node(
        "Sender",
        Node::Block(Block {
            name: "Sender".into(),
            inputs: vec![start2],
            sub: vec![delayer, send],
            links: l2,
            owner: Some(1),
            ..Block::default()
        }),
    );
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    // docs/gameplay_bbs.md, "Delayer": the tick In fires in counts, so 40 ms is reached in the third run
    for _ in 0..2 {
        rt.tick(&mut host);
        assert_eq!(host.visible, vec![true, true], "the delayer still waits");
    }
    rt.tick(&mut host);
    assert_eq!(
        host.messages,
        vec!["Go", "Go"],
        "sent to both members of the group"
    );
    assert_eq!(host.visible, vec![true, true], "received at the next tick");
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![false, true],
        "the waiter, owned by A, hides A"
    );
    assert!(!rt.is_active(sender) && !rt.is_active(waiter));
}

#[test]
fn the_proximity_test_enters_and_leaves_and_box_intersection_tests_overlap() {
    let mut b = Build::new();
    let dist = b.local("Distance", types::FLOAT, Value::Float(5.0));
    let a = b.local("ObjectA", types::ENTITY, Value::Obj(Some(3)));
    let c = b.local("ObjectB", types::ENTITY, Value::Obj(Some(0)));
    let zero = b.local("x", types::FLOAT, Value::Float(0.0));
    let one = b.local("x", types::FLOAT, Value::Float(1.0));
    let pins = vec![
        b.pin("Distance", types::FLOAT, dist),
        b.pin("ObjectA", types::ENTITY, a),
        b.pin("ObjectB", types::ENTITY, c),
        b.pin("Barycenter?", types::BOOL, zero),
        b.pin("Exactness min. Distance", types::FLOAT, one),
        b.pin("Exactness max. Distance", types::FLOAT, one),
        b.pin("Minimum Framedelay", types::INT, one),
        b.pin("Maximum Framedelay", types::INT, one),
    ];
    let cur = b.pout("Current Distance", types::FLOAT);
    let prox = b.bb(
        "TT Scaleable Proximity",
        proto::TT_SCALEABLE_PROXIMITY,
        &["In", "Off"],
        &["EnterRange", "ExitRange"],
        pins,
        vec![cur],
        vec![],
        None,
    );
    let hier = b.local("Hierarchy", types::BOOL, Value::Bool(false));
    let hide = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![hier],
        vec![],
        vec![],
        None,
    );
    let show = b.bb(
        "Show",
        proto::SHOW,
        &["In"],
        &["Out"],
        vec![hier],
        vec![],
        vec![],
        None,
    );
    let e0 = b.pin("Entity 0", types::ENTITY, a);
    let e1 = b.pin("Entity 1", types::ENTITY, c);
    let bbi = b.bb(
        "Box Box Intersection",
        proto::BOX_BOX_INTERSECTION,
        &["In"],
        &["True", "False"],
        vec![e0, e1],
        vec![],
        vec![],
        None,
    );
    let text = b.local("Text", types::STRING, Value::Str("hit".into()));
    let tpins = vec![
        b.pin("Offset", types::VECTOR2, zero),
        b.pin("Color", types::COLOR, zero),
        b.pin("Alignment", types::INT, zero),
        b.pin("Size", types::INT, zero),
        b.pin("Text", types::STRING, text),
    ];
    let disp = b.bb(
        "Text Display",
        proto::TEXT_DISPLAY,
        &["On", "Off"],
        &["Exit On", "Exit Off"],
        tpins,
        vec![],
        vec![],
        None,
    );
    let start = b.io("Start");
    let links = vec![
        b.link(start, in_of(&b.g, prox, 0), 0),
        b.link(out_of(&b.g, prox, 0), in_of(&b.g, hide, 0), 0),
        b.link(out_of(&b.g, prox, 1), in_of(&b.g, show, 0), 0),
        b.link(start, in_of(&b.g, bbi, 0), 0),
        b.link(out_of(&b.g, bbi, 0), in_of(&b.g, disp, 0), 0),
    ];
    b.script(
        "S",
        Some(1),
        start,
        vec![prox, hide, show, bbi, disp],
        links,
    );
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    host.ball = [1.5, 0.0, 0.0];
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![true, false],
        "within 5 of A: EnterRange hides the owner B"
    );
    assert_eq!(rt.values[cur], Value::Float(1.5));
    assert_eq!(
        host.notes,
        vec!["text hit"],
        "boxes of half-size 1 at 0 and 1.5 overlap"
    );
    host.ball = [20.0, 0.0, 0.0];
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![true, true],
        "out of range: ExitRange shows B again"
    );
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![true, true],
        "no new event while it stays out"
    );
}

// ---- a stranger's graph may fan out: the tick's work budget ends it ----------------------------------------------

fn bare(nodes: Vec<Node>) -> Graph {
    let n = nodes.len();
    Graph {
        objects: (0..n)
            .map(|i| crate::ObjInfo {
                name: format!("o{i}"),
                class: 0,
                kind: ObjKind::Other,
            })
            .collect(),
        nodes,
        arrays: Vec::new(),
        scripts: vec![0],
        links_from: vec![Vec::new(); n],
        priorities: Vec::new(),
        start_off: Vec::new(),
        groups: Vec::new(),
        attributes: Vec::new(),
        waves: Vec::new(),
    }
}

#[test]
fn a_block_listing_itself_a_thousand_times_does_not_stall_the_runtime() {
    let block = Block {
        name: "loop".into(),
        sub: vec![0; 1000],
        ..Block::default()
    };
    // activation recurses through the sub-behaviours: without a budget this is 1000^48 steps
    let mut rt = Runtime::new(Arc::new(bare(vec![Node::Block(block)])));
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert!(
        host.notes.iter().any(|n| n.contains("budget")),
        "{:?}",
        host.notes
    );
}

#[test]
fn an_operation_reading_its_own_output_twice_does_not_stall_the_runtime() {
    let p = Param {
        name: "p".into(),
        guid: types::INT,
        op: Some(1),
        ..Param::default()
    };
    let op = crate::Op {
        guid: Guid(1, 2),
        inputs: [Some(0), Some(0)],
        output: Some(0),
    };
    let mut rt = Runtime::new(Arc::new(bare(vec![Node::Param(p), Node::Op(op)])));
    let mut host = Rec::new();
    // 2^48 reads without a budget
    let _ = rt.read(0, &mut host);
}

#[test]
fn a_parameter_copying_to_itself_a_thousand_times_does_not_stall_the_runtime() {
    let p = Param {
        name: "p".into(),
        guid: types::INT,
        destinations: vec![0; 1000],
        ..Param::default()
    };
    let mut rt = Runtime::new(Arc::new(bare(vec![Node::Param(p)])));
    rt.write(0, Value::Int(1));
}

#[test]
fn a_string_that_doubles_each_time_stops_growing() {
    let big = Value::Str("x".repeat(1 << 16));
    let mut rt = Runtime::new(Arc::new(bare(vec![])));
    let r = crate::ops::run(
        &mut rt,
        &mut Rec::new(),
        crate::ops::op::ADD,
        types::STRING,
        types::STRING,
        types::STRING,
        &big,
        &big,
        true,
    )
    .unwrap();
    assert!(r.as_string().len() <= (1 << 16) + 8);
}

/// A template object A with a script that hides "This"; another script copies A and moves the copy. The copy
/// gets its own name and its own copy of the script, bound to the copy: the copy's script hides the copy.
#[test]
fn object_copy_copies_the_scripts_and_binds_this_to_the_copy() {
    let mut b = Build::new();
    // the maker: Start -> Object Copy(A) -> Set Position(Copy, 5 0 0)
    let dep = b.local("Dependency Options", types::NONE, Value::None);
    let orig = b.local("Original", types::ENTITY, Value::Obj(Some(0)));
    let orig_pin = b.pin("Original", types::ENTITY, orig);
    let copy_out = b.pout("Copy", types::ENTITY);
    let copy = b.bb(
        "Object Copy",
        proto::OBJECT_COPY,
        &["In"],
        &["Out"],
        vec![dep, orig_pin],
        vec![copy_out],
        vec![],
        None,
    );
    let pos = b.local("Position", types::VECTOR, Value::Vec3([5.0, 0.0, 0.0]));
    let pos_pin = b.pin("Position", types::VECTOR, pos);
    let set = b.bb(
        "Set Position",
        proto::SET_POSITION,
        &["In"],
        &["Out"],
        vec![pos_pin],
        vec![],
        vec![],
        Some(copy_out),
    );
    let start2 = b.io("Start");
    let l1 = b.link(start2, in_of(&b.g, copy, 0), 0);
    let l2 = b.link(out_of(&b.g, copy, 0), in_of(&b.g, set, 0), 0);
    // the maker on B, which has the higher priority: the manager runs it before the template
    b.script("Maker", Some(1), start2, vec![copy, set], vec![l1, l2]);
    b.g.priorities = vec![(1, 1)];
    // the template's script on A (object 0): Start -> Hide(This)
    let this = b.local("This", types::BEOBJECT, Value::Owner);
    let target = b.pin("Target", types::ENTITY, this);
    let hide = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![],
        vec![],
        vec![],
        Some(target),
    );
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, hide, 0), 0);
    let template = b.script("Template", Some(0), start, vec![hide], vec![l]);
    let g = b.done();
    let nodes_before = g.nodes.len();
    let mut rt = Runtime::new(g.clone());
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![false, true, true],
        "the template hides itself; the copy is made"
    );
    assert_eq!(
        host.matrices[2][3],
        [5.0, 0.0, 0.0, 1.0],
        "the copy is moved, not the original"
    );
    assert_eq!(host.matrices[0][3], [0.0, 0.0, 0.0, 1.0]);
    assert!(
        host.notes.contains(&"copy A001".to_string()),
        "{:?}",
        host.notes
    );
    assert_eq!(
        g.nodes.len(),
        nodes_before,
        "the level's graph is not touched: the run has its own"
    );
    assert_eq!(
        rt.graph.scripts.len(),
        3,
        "the copy has the template's script"
    );
    let copied = *rt.graph.scripts.last().unwrap();
    assert_ne!(copied, template);
    assert_eq!(
        rt.graph
            .name(rt.graph.block(copied).unwrap().owner.unwrap()),
        "A001"
    );
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![false, true, false],
        "the copy's script hides the copy"
    );
    // a rewound run copies again and gets the same
    let mut again = Runtime::new(g);
    let mut host2 = Rec::new();
    again.tick(&mut host2);
    again.tick(&mut host2);
    assert_eq!(host2.visible, host.visible);
}

/// Object Copy in a loop stops at the cap: the run does not grow without end.
#[test]
fn copies_are_capped() {
    let mut b = Build::new();
    let dep = b.local("Dependency Options", types::NONE, Value::None);
    let orig = b.local("Original", types::ENTITY, Value::Obj(Some(0)));
    let orig_pin = b.pin("Original", types::ENTITY, orig);
    let copy_out = b.pout("Copy", types::ENTITY);
    let copy = b.bb(
        "Object Copy",
        proto::OBJECT_COPY,
        &["In"],
        &["Out"],
        vec![dep, orig_pin],
        vec![copy_out],
        vec![],
        None,
    );
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, copy, 0), 0);
    let l2 = b.link(out_of(&b.g, copy, 0), in_of(&b.g, copy, 0), 0);
    b.script("Maker", None, start, vec![copy], vec![l1, l2]);
    let mut rt = Runtime::new(b.done());
    let mut host = Rec::new();
    for _ in 0..3 {
        rt.tick(&mut host);
    }
    assert_eq!(host.matrices.len(), 2 + crate::exec::MAX_COPIES);
    assert_eq!(
        host.notes.iter().filter(|n| n.contains("too many")).count(),
        1,
        "said once"
    );
}

/// Linear Progression goes from A to B over its time, a Loop In a tick; Interpolator and Set Component follow it.
#[test]
fn a_progression_runs_from_a_to_b_and_ends() {
    let mut b = Build::new();
    let f = |b: &mut Build, name: &str, v: f32| {
        let l = b.local(name, types::FLOAT, Value::Float(v));
        b.pin(name, types::FLOAT, l)
    };
    let (time, a, c) = (
        f(&mut b, "Time", 100.0),
        f(&mut b, "A", 2.0),
        f(&mut b, "B", 12.0),
    );
    let outs: Vec<Id> = ["Elapsed Time", "Value", "Delta", "Progression"]
        .iter()
        .map(|n| b.pout(n, types::FLOAT))
        .collect();
    let (value, progression) = (outs[1], outs[3]);
    let time_based = b.local("Time Based", types::BOOL, Value::Bool(true));
    let prog = b.bb(
        "Linear Progression",
        proto::LINEAR_PROGRESSION,
        &["In", "Loop In"],
        &["Out", "Loop Out"],
        vec![time, a, c],
        outs,
        vec![time_based],
        None,
    );
    // Interpolator: (0 0 0)..(10 20 30) at the progression
    let va = b.local("A", types::VECTOR, Value::Vec3([0.0; 3]));
    let vb = b.local("B", types::VECTOR, Value::Vec3([10.0, 20.0, 30.0]));
    let (pa, pb) = (b.pin("A", types::VECTOR, va), b.pin("B", types::VECTOR, vb));
    let pv = b.pin("Value", types::FLOAT, progression);
    let mixed = b.pout("C", types::VECTOR);
    let mix = b.bb(
        "Interpolator",
        proto::INTERPOLATOR,
        &["In"],
        &["Out"],
        vec![pa, pb, pv],
        vec![mixed],
        vec![],
        None,
    );
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, prog, 0), 0);
    let l2 = b.link(out_of(&b.g, prog, 1), in_of(&b.g, prog, 1), 1);
    let l3 = b.link(out_of(&b.g, prog, 1), in_of(&b.g, mix, 0), 0);
    let script = b.script("S", None, start, vec![prog, mix], vec![l1, l2, l3]);
    let mut rt = Runtime::new(b.done());
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(rt.values[value], Value::Float(2.0), "it starts at A");
    let mut last = 2.0;
    for _ in 0..3 {
        rt.tick(&mut host);
        let v = rt.values[value].as_float();
        assert!(v > last && v < 12.0, "{v}");
        last = v;
    }
    // 3 ticks of 16.67 ms of 100: half way
    assert!((last - 7.0).abs() < 0.01, "{last}");
    assert!((rt.values[mixed].as_vec3()[1] - 10.0).abs() < 0.01);
    for _ in 0..4 {
        rt.tick(&mut host);
    }
    assert_eq!(rt.values[value], Value::Float(12.0), "it ends at B");
    assert!(!rt.is_active(script), "and stops");
}

fn line_curve(y0: f32, y1: f32) -> Value {
    use nmo::curve2d::{Curve2d, Point};
    let p = |x: f32, y: f32| Point {
        pos: [x, y],
        flags: 2,
        ..Point::default()
    };
    Value::Curve(Arc::new(Curve2d::new(vec![p(0.0, y0), p(1.0, y1)])))
}

const CURVE2D: Guid = Guid(0x20ad_345d, 0x1afb_25b1);

/// Bezier Progression follows the original: Elapsed Time grows by the tick on the first run too, Value is
/// A + (B - A) * curve(elapsed / duration), Out comes once the elapsed time exceeds the duration.
#[test]
fn a_bezier_progression_follows_its_curve_and_ends() {
    let mut b = Build::new();
    let f = |b: &mut Build, name: &str, v: f32| {
        let l = b.local(name, types::FLOAT, Value::Float(v));
        b.pin(name, types::FLOAT, l)
    };
    let (time, a, c) = (
        f(&mut b, "Duration", 100.0),
        f(&mut b, "A", 2.0),
        f(&mut b, "B", 12.0),
    );
    // the curve falls from 1 to 0: the value goes from B to A
    let cl = b.local("Progression Curve", CURVE2D, line_curve(1.0, 0.0));
    let cp = b.pin("Progression Curve", CURVE2D, cl);
    let outs: Vec<Id> = ["Elapsed Time", "Value", "Delta", "Progression"]
        .iter()
        .map(|n| b.pout(n, types::FLOAT))
        .collect();
    let (elapsed, value, delta, progress) = (outs[0], outs[1], outs[2], outs[3]);
    let prog = b.bb(
        "Bezier Progression",
        proto::BEZIER_PROGRESSION,
        &["In", "Loop In"],
        &["Out", "Loop Out"],
        vec![time, a, c, cp],
        outs,
        vec![],
        None,
    );
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, prog, 0), 0);
    let l2 = b.link(out_of(&b.g, prog, 1), in_of(&b.g, prog, 1), 1);
    let script = b.script("S", None, start, vec![prog], vec![l1, l2]);
    let mut rt = Runtime::new(b.done());
    let mut host = Rec::new();
    rt.tick(&mut host);
    let tick = 1000.0 / 60.0;
    // the first run: elapsed = one tick, progress 1/6, value = 2 + 10 * (1 - 1/6)
    assert!((rt.values[elapsed].as_float() - tick).abs() < 1e-3);
    assert!((rt.values[progress].as_float() - 1.0 / 6.0).abs() < 1e-4);
    assert!((rt.values[value].as_float() - (2.0 + 10.0 * (1.0 - 1.0 / 6.0))).abs() < 1e-3);
    // delta: from the value at 0 (B = 12) to the first value
    assert!((rt.values[delta].as_float() - (rt.values[value].as_float() - 12.0)).abs() < 1e-3);
    for _ in 0..8 {
        rt.tick(&mut host);
    }
    assert_eq!(rt.values[progress], Value::Float(1.0));
    assert_eq!(rt.values[value], Value::Float(2.0), "it ends at A, the curve's 0");
    assert!(!rt.is_active(script), "Out fired and nothing looped back");
}

/// Bezier Transform: Y = bottom + curve((X - left) / (right - left)) * (top - bottom).
#[test]
fn a_bezier_transform_maps_x_through_the_curve_and_the_boundaries() {
    let run = |x: f32, curve: Value, rect: [f32; 4]| -> f32 {
        let mut b = Build::new();
        let xl = b.local("X", types::FLOAT, Value::Float(x));
        let xp = b.pin("X", types::FLOAT, xl);
        let cl = b.local("Progression Curve", CURVE2D, curve);
        let cp = b.pin("Progression Curve", CURVE2D, cl);
        let rl = b.local("Boundaries", types::RECT, Value::Rect(rect));
        let rp = b.pin("Boundaries", types::RECT, rl);
        let y = b.pout("Y", types::FLOAT);
        let local = b.local("Use 4 Floats As Boundaries", types::BOOL, Value::Bool(false));
        let t = b.bb(
            "Bezier Transform",
            proto::BEZIER_TRANSFORM,
            &["In"],
            &["Out"],
            vec![xp, cp, rp],
            vec![y],
            vec![local],
            None,
        );
        let start = b.io("Start");
        let l1 = b.link(start, in_of(&b.g, t, 0), 0);
        b.script("S", None, start, vec![t], vec![l1]);
        let mut rt = Runtime::new(b.done());
        rt.tick(&mut Rec::new());
        rt.values[y].as_float()
    };
    // a straight curve: the boundaries alone
    assert_eq!(run(60.0, line_curve(0.0, 1.0), [0.0, 120.0, 120.0, 0.0]), 60.0);
    assert_eq!(run(0.8, line_curve(0.0, 1.0), [0.0, 1.25, 1.0, 0.0]), 1.0);
    // a falling curve: Y = 120 * (1 - 0.25)
    assert_eq!(run(30.0, line_curve(1.0, 0.0), [0.0, 120.0, 120.0, 0.0]), 90.0);
    // X outside the range is the curve's end value; a rectangle of no width reads the curve at 0
    assert_eq!(run(500.0, line_curve(0.0, 1.0), [0.0, 10.0, 100.0, 0.0]), 10.0);
    assert_eq!(run(-9.0, line_curve(0.0, 1.0), [0.0, 10.0, 100.0, 2.0]), 2.0);
    assert_eq!(run(5.0, line_curve(0.0, 1.0), [3.0, 10.0, 3.0, 0.0]), 0.0);
    // no curve in the file: the identity shape
    assert_eq!(run(25.0, Value::None, [0.0, 1.0, 100.0, 0.0]), 0.25);
}

/// Random gives the same numbers in every run, within its bounds; the group blocks reach the engine.
#[test]
fn random_repeats_and_groups_change() {
    let build = || {
        let mut b = Build::new();
        let lo = b.local("Min", types::FLOAT, Value::Float(3.0));
        let hi = b.local("Max", types::FLOAT, Value::Float(4.0));
        let (plo, phi) = (
            b.pin("Min", types::FLOAT, lo),
            b.pin("Max", types::FLOAT, hi),
        );
        let rand = b.pout("Rand", types::FLOAT);
        let random = b.bb(
            "Random",
            proto::RANDOM,
            &["In"],
            &["Out"],
            vec![plo, phi],
            vec![rand],
            vec![],
            None,
        );
        let group = b.local("Group", types::GROUP, Value::Obj(Some(2)));
        let pg = b.pin("Group", types::GROUP, group);
        let t = b.local("Target", types::BEOBJECT, Value::Obj(Some(1)));
        let add = b.bb(
            "Add To Group",
            proto::ADD_TO_GROUP,
            &["In"],
            &["Out"],
            vec![pg],
            vec![],
            vec![],
            Some(t),
        );
        let start = b.io("Start");
        let l1 = b.link(start, in_of(&b.g, random, 0), 0);
        let l2 = b.link(out_of(&b.g, random, 0), in_of(&b.g, random, 0), 1);
        let l3 = b.link(start, in_of(&b.g, add, 0), 0);
        b.script("S", None, start, vec![random, add], vec![l1, l2, l3]);
        (b.done(), rand)
    };
    let run = || {
        let (g, rand) = build();
        let mut rt = Runtime::new(g);
        let mut host = Rec::new();
        let mut seen = Vec::new();
        for _ in 0..20 {
            rt.tick(&mut host);
            seen.push(rt.values[rand].as_float());
        }
        (seen, host.notes)
    };
    let (a, notes) = run();
    let (b2, _) = run();
    assert_eq!(a, b2, "the same numbers in every run");
    assert!(a.iter().all(|x| (3.0..4.0).contains(x)), "{a:?}");
    assert!(
        a.windows(2).any(|w| w[0] != w[1]),
        "and they differ from draw to draw"
    );
    assert!(notes.contains(&"group 0 + 1".to_string()), "{notes:?}");
}

/// A hostile map: an object with tens of thousands of scripts. Loading is linear in the map, and copying the
/// object is paid from the tick's budget: a copy larger than a tick may make is refused, whole, so neither
/// the load nor a tick takes the game's time (the bounds are generous: the work is milliseconds).
#[test]
fn an_object_with_very_many_scripts_neither_stalls_the_load_nor_a_copy() {
    let build = |n: usize, maker: bool| {
        let mut b = Build::new();
        if maker {
            let dep = b.local("Dependency Options", types::NONE, Value::None);
            let orig = b.local("Original", types::ENTITY, Value::Obj(Some(0)));
            let orig_pin = b.pin("Original", types::ENTITY, orig);
            let copy_out = b.pout("Copy", types::ENTITY);
            let copy = b.bb(
                "Object Copy",
                proto::OBJECT_COPY,
                &["In"],
                &["Out"],
                vec![dep, orig_pin],
                vec![copy_out],
                vec![],
                None,
            );
            let start = b.io("Start");
            let l1 = b.link(start, in_of(&b.g, copy, 0), 0);
            let l2 = b.link(out_of(&b.g, copy, 0), in_of(&b.g, copy, 0), 1);
            b.script("Maker", None, start, vec![copy], vec![l1, l2]);
        }
        for _ in 0..n {
            let s = b.io("Start");
            b.script("S", Some(0), s, vec![], vec![]);
        }
        b.done()
    };
    // generous for a debug build on a busy machine: what it catches is a blow-up (quadratic walks), not seconds
    let limit = std::time::Duration::from_secs(30);
    // the load: 300 000 scripts on one object
    let g = build(300_000, false);
    let t = std::time::Instant::now();
    let _rt = Runtime::new(g);
    assert!(t.elapsed() < limit, "load took {:?}", t.elapsed());
    // a copy every tick of an object with 20 000 scripts: the graph stays under its cap, the ticks are short
    let g = build(20_000, true);
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    let t = std::time::Instant::now();
    for _ in 0..300 {
        rt.tick(&mut host);
    }
    assert!(t.elapsed() < limit, "300 ticks took {:?}", t.elapsed());
    assert!(rt.graph.nodes.len() <= crate::MAX_OBJECTS);
    assert!(host.matrices.len() > 2, "copies that fit are made");
    assert!(
        host.notes.iter().any(|n| n.contains("copy refused")),
        "and one that does not fit is refused"
    );
    // straight calls, as the reviewer's: 256 tries on 60 000 scripts
    let g = build(60_000, false);
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    let t = std::time::Instant::now();
    for _ in 0..256 {
        rt.copy_object(0, &mut host);
    }
    assert!(t.elapsed() < limit, "256 copies took {:?}", t.elapsed());
    assert!(rt.graph.nodes.len() <= crate::MAX_OBJECTS);
}

#[test]
fn the_registry_declares_each_prototype_once_in_its_library() {
    use crate::blocks::{Library, Standing, find, registry};
    let all = registry();
    for w in all.windows(2) {
        assert_ne!(w[0].guid, w[1].guid, "{} declared twice", w[0].name);
    }
    // the 80 kinds known before the registry, the 7 absent ones besides Object Rename, and stage 0b's (the ten array
    // blocks and Set Row among them), the sounds, the input blocks and the cameras (Set As Active Camera,
    // Set Clipping Planes, Set FOV)
    assert_eq!(all.len(), 154); // and the text blocks (six), the spherical emitter, Physics Impulse and WakeUp, the five environment and the 11 material blocks
    for p in all {
        assert_eq!(find(p.guid).map(|q| q.name), Some(p.name));
        assert_eq!(
            p.library == Library::NotInPlayer,
            p.standing == Standing::NoOp,
            "{}",
            p.name
        );
    }
    assert_eq!(
        find(proto::OBJECT_RENAME).map(|p| p.standing),
        Some(Standing::NoOp)
    );
    assert_eq!(
        find(proto::DELAYER).map(|p| p.library.dll()),
        Some("Logics.dll")
    );
    assert!(find(Guid(1, 2)).is_none());
}

/// A script on `owner` whose start hides `target` (Start -> Hide).
fn hider(b: &mut Build, name: &str, owner: Id, target: Id) -> Id {
    let t = b.local("Target", types::ENTITY, Value::Obj(Some(target)));
    let hide = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![],
        vec![],
        vec![],
        Some(t),
    );
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, hide, 0), 0);
    b.script(name, Some(owner), start, vec![hide], vec![l])
}

/// A script on object 0 whose start runs one block `proto` with pins `pins` (each a local of the given value).
fn one_block(b: &mut Build, proto: Guid, pins: Vec<(Guid, Value)>) -> Id {
    let mut ps = Vec::new();
    for (g, v) in pins {
        let l = b.local("p", g, v);
        ps.push(b.pin("p", g, l));
    }
    let blk = b.bb("blk", proto, &["In"], &["Out"], ps, vec![], vec![], None);
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, blk, 0), 0);
    b.script("Caller", Some(0), start, vec![blk], vec![l])
}

#[test]
fn an_activation_asked_in_a_tick_is_applied_at_the_start_of_the_next() {
    // B's script (saved inactive) hides B; A's script activates it. A's object has the higher priority, so the
    // manager runs A first: with an activation applied at once, B would hide in the same tick.
    let mut b = Build::new();
    let target = hider(&mut b, "B hides", 1, 1);
    b.g.start_off.push(target);
    one_block(
        &mut b,
        proto::ACTIVATE_SCRIPT,
        vec![
            (types::BOOL, Value::Bool(true)),
            (types::SCRIPT, Value::Obj(Some(target))),
        ],
    );
    b.g.priorities = vec![(0, 100)];
    let g = b.done();
    let mut rt = Runtime::new(g);
    assert_eq!(
        rt.order().first().copied(),
        Some(rt.graph.scripts[1]),
        "A's object first"
    );
    assert!(
        !rt.is_active(target),
        "saved to start deactivated, starts inactive"
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![true, true],
        "asked in tick 1, not yet run"
    );
    rt.tick(&mut host);
    assert_eq!(host.visible, vec![true, false], "applied and run in tick 2");
}

#[test]
fn a_script_resumed_without_reset_keeps_its_state() {
    // Start -(delay 3)-> Hide A; deactivated in tick 1 and activated again without reset, the delayed link goes
    // on counting where it was: the hide comes on tick 3 + the ticks it was off, never restarted.
    let mut b = Build::new();
    let t = b.local("Target", types::ENTITY, Value::Obj(Some(0)));
    let hide = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![],
        vec![],
        vec![],
        Some(t),
    );
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, hide, 0), 3);
    let s = b.script("Late", Some(0), start, vec![hide], vec![l]);
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.tick(&mut host); // the link is queued: 2 left
    rt.request(s, false, false);
    rt.tick(&mut host); // off
    rt.request(s, true, false);
    rt.tick(&mut host); // on again: 1 left
    rt.tick(&mut host); // fires at the end of the tick: Hide runs in the next
    assert_eq!(host.visible, vec![true, true]);
    rt.tick(&mut host);
    assert_eq!(
        host.visible,
        vec![false, true],
        "resumed, not restarted (a reset would wait 3 more)"
    );
}

#[test]
fn a_sleeping_object_runs_no_scripts_and_resumes_when_woken() {
    let mut b = Build::new();
    let t = b.local("Target", types::ENTITY, Value::Obj(Some(1)));
    let hide = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![],
        vec![],
        vec![],
        Some(t),
    );
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, hide, 0), 2);
    b.script("On B", Some(1), start, vec![hide], vec![l]);
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.activate_object(1, false, false, &mut host);
    for _ in 0..5 {
        rt.tick(&mut host);
    }
    assert_eq!(host.visible, vec![true, true], "asleep: nothing runs");
    rt.activate_object(1, true, false, &mut host);
    for _ in 0..3 {
        rt.tick(&mut host);
    }
    assert_eq!(host.visible, vec![true, false], "woken: the script goes on");
}

#[test]
fn execute_script_runs_the_script_inside_the_callers_frame() {
    let mut b = Build::new();
    let target = hider(&mut b, "B hides", 1, 1);
    b.g.start_off.push(target);
    one_block(
        &mut b,
        proto::EXECUTE_SCRIPT,
        vec![
            (types::BOOL, Value::Bool(false)),
            (types::SCRIPT, Value::Obj(Some(target))),
        ],
    );
    b.g.priorities = vec![(0, 100)];
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(host.visible, vec![true, false], "in the same tick");
}

#[test]
fn deleting_an_object_ends_its_scripts() {
    let mut b = Build::new();
    let t = b.local("Target", types::ENTITY, Value::Obj(Some(1)));
    let hide = b.bb(
        "Hide",
        proto::HIDE,
        &["In"],
        &["Out"],
        vec![],
        vec![],
        vec![],
        Some(t),
    );
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, hide, 0), 2);
    let s = b.script("On A", Some(0), start, vec![hide], vec![l]);
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.delete_object(0, &mut host);
    rt.request(s, true, true);
    for _ in 0..4 {
        rt.tick(&mut host);
    }
    assert_eq!(host.visible, vec![true, true]);
    assert!(!rt.is_active(s));
}

#[test]
fn a_group_iterator_keeps_its_index_in_its_index_output() {
    let mut b = Build::new();
    let grp = b.local("Group", types::GROUP, Value::Obj(Some(2)));
    let gpin = b.pin("Group", types::GROUP, grp);
    let elem = b.pout("Element", types::ENTITY);
    let index = b.pout("Index", types::INT);
    let it = b.bb(
        "Group Iterator",
        proto::GROUP_ITERATOR,
        &["In", "Loop In"],
        &["Out", "Loop Out"],
        vec![gpin],
        vec![elem, index],
        vec![],
        None,
    );
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, it, 0), 0);
    let l2 = b.link(out_of(&b.g, it, 1), in_of(&b.g, it, 1), 1);
    b.script("Iter", Some(0), start, vec![it], vec![l1, l2]);
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(rt.values[index], Value::Int(0));
    rt.tick(&mut host);
    assert_eq!(
        rt.values[index],
        Value::Int(1),
        "the second member, from the stored index"
    );
    assert_eq!(rt.values[elem], Value::Obj(Some(1)));
}

#[test]
fn the_trace_shows_frames_runs_ios_and_parameters() {
    let mut b = Build::new();
    hider(&mut b, "Hides", 0, 0);
    let g = b.done();
    let mut rt = Runtime::new(g);
    rt.set_trace(true);
    let mut host = Rec::new();
    rt.tick(&mut host);
    let t = rt.take_trace();
    assert_eq!(t.first().map(String::as_str), Some("frame 1"));
    assert!(
        t.iter()
            .any(|l| l.starts_with("graph ") && l.ends_with("Hides")),
        "{t:?}"
    );
    assert!(
        t.iter()
            .any(|l| l.starts_with("run ") && l.ends_with("Hide")),
        "{t:?}"
    );
    assert!(
        t.iter().any(|l| l.starts_with("io ") && l.ends_with("in0")),
        "{t:?}"
    );
    assert!(rt.take_trace().is_empty(), "taken");
}

#[test]
fn the_census_counts_kinds_links_and_delays_with_the_registrys_standing() {
    let mut b = Build::new();
    hider(&mut b, "Hides", 0, 0);
    let unknown = b.bb(
        "Mystery",
        Guid(7, 7),
        &["In"],
        &[],
        vec![],
        vec![],
        vec![],
        None,
    );
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, unknown, 0), 2);
    b.script("Odd", Some(1), start, vec![unknown], vec![l]);
    let g = b.done();
    let mut c = crate::census::Census::default();
    c.add(&g);
    c.add(&Graph::default());
    assert_eq!(
        (c.files, c.with_graphs, c.scripts, c.blocks, c.links),
        (2, 1, 2, 2, 2)
    );
    assert_eq!(c.delays, [1, 0, 1, 0]);
    let csv = c.csv();
    assert!(
        csv.contains("block,Hide,31d97d82:78d54d98,Visuals.dll,full,1,1"),
        "{csv}"
    );
    assert!(
        csv.contains("block,Mystery,00000007:00000007,?,unknown,1,1"),
        "{csv}"
    );
}

/// Groups hold any object of the graph, not only the level's entities (docs/gameplay_bbs.md, "Fill Group By Class"):
/// the file's own members of every kind seed a group, Fill Group By Class collects by class (the class itself and
/// the number after it when Derived Classes is off, every derived class when on), and Is In Group sees them.
#[test]
fn groups_hold_any_object_and_fill_by_class() {
    let mut b = Build::new();
    // ids 5.. : a mesh (class 32), two materials (30), a texture (31)
    let mesh = b.g.add_object("Mesh", ObjKind::Mesh(0));
    let m1 = b.g.add_object("Mat1", ObjKind::Material(0));
    let m2 = b.g.add_object("Mat2", ObjKind::Material(1));
    let tex = b.g.add_object("Tex", ObjKind::Texture(0));
    for (i, c) in [(mesh, 32), (m1, 30), (m2, 30), (tex, 31)] {
        b.g.objects[i].class = c;
    }
    b.g.groups = vec![vec![mesh, 0]];
    let class = b.local("Class", types::INT, Value::Int(30));
    let derived = b.local("Derived Classes", types::BOOL, Value::Bool(false));
    let pins = vec![
        b.pin("Class", types::INT, class),
        b.pin("Derived Classes", types::BOOL, derived),
    ];
    let t = b.local("Target", types::GROUP, Value::Obj(Some(2)));
    let fill = b.bb(
        "Fill Group By Class",
        proto::FILL_GROUP_BY_CLASS,
        &["In"],
        &["Out"],
        pins,
        vec![],
        vec![],
        Some(t),
    );
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, fill, 0), 0);
    b.script("S", None, start, vec![fill], vec![l]);
    let g = b.done();
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    assert_eq!(
        rt.group_ids(0, &host),
        vec![mesh, 0],
        "the file's members, of any kind"
    );
    rt.tick(&mut host);
    assert_eq!(
        rt.group_ids(0, &host),
        vec![mesh, 0, m1, m2],
        "the materials are added, the texture (class 31) is not"
    );
    // a copy of the run keeps its own members
    let mut other = rt.clone();
    other.group_clear(0, &host);
    assert!(other.group_ids(0, &host).is_empty());
    assert_eq!(rt.group_ids(0, &host).len(), 4);
}

/// Attributes (docs/gameplay_bbs.md, "Attributes"): the file's seed the lists in order, Set Attribute adds an object
/// and its value, Has Attribute finds it and gives the value, Remove Attribute drops it; a copy of the run keeps its
/// own, and a bad index is refused.
#[test]
fn attributes_are_set_found_and_listed_in_order() {
    let mut b = Build::new();
    b.g.attributes = vec![(0, 5, None), (1, 6, None)];
    let target = b.local("Target", types::BEOBJECT, Value::Obj(Some(1)));
    let attr = b.local("Attribute", types::INT, Value::Int(5));
    let val = b.local("Attribute Value", types::INT, Value::Int(7));
    let pins = vec![
        b.pin("Attribute", types::INT, attr),
        b.pin("Attribute Value", types::INT, val),
    ];
    let set = b.bb(
        "Set Attribute",
        proto::SET_ATTRIBUTE,
        &["In"],
        &["Out"],
        pins,
        vec![],
        vec![],
        Some(target),
    );
    let shown = b.pout("Attribute Value", types::INT);
    let pin_has = b.pin("Attribute", types::INT, attr);
    let has = b.bb(
        "Has Attribute",
        proto::HAS_ATTRIBUTE,
        &["In"],
        &["True", "False"],
        vec![pin_has],
        vec![shown],
        vec![],
        Some(target),
    );
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, set, 0), 0);
    let l2 = b.link(out_of(&b.g, set, 0), in_of(&b.g, has, 0), 0);
    b.script("S", None, start, vec![set, has], vec![l1, l2]);
    let g = b.done();
    let mut rt = Runtime::new(g);
    assert_eq!(
        rt.with_attribute(5, 0).map(|x| x.0),
        Some(0),
        "the file's first"
    );
    assert!(rt.attribute(1, 5).is_none());
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(rt.attribute(1, 5), Some(&Value::Int(7)));
    assert_eq!(
        rt.values[shown],
        Value::Int(7),
        "Has Attribute gives the value"
    );
    assert_eq!(
        rt.with_attribute(5, 1).map(|x| x.0),
        Some(1),
        "in the order they were set"
    );
    assert!(rt.with_attribute(5, 2).is_none());
    let mut other = rt.clone();
    other.remove_attribute(1, 5);
    assert!(other.attribute(1, 5).is_none() && rt.attribute(1, 5).is_some());
    assert!(
        !rt.set_attribute(1, -1, None),
        "a negative index is no attribute"
    );
    assert!(
        !rt.set_attribute(999_999, 5, None),
        "nor an object that is not there"
    );
    for a in 0..80 {
        rt.set_attribute(0, a + 10, None);
    }
    assert!(
        rt.attrs_of(0) <= crate::MAX_ATTRIBUTES_PER_OBJECT,
        "an object carries a bounded number"
    );
}

// ---- sounds (docs/gameplay_bbs.md, "Sounds"): the commands go to the host; expected values from those notes ----

/// A graph with a wave sound object (class 25, id 5, file "Wind.wav") and one sound block of `proto` that has
/// `ins` inputs, `outs` outputs and pins `(name, type, value)` set, targeted at the wave sound and owned by object
/// 0. Returns the runtime, the script and block ids and the block's pout (if `pout` is named).
fn sound_script(
    proto: Guid,
    ins: &[&str],
    outs: &[&str],
    pins: &[(&str, Guid, Value)],
    pout: Option<&str>,
) -> (Runtime, Id, Id, Option<Id>) {
    sound_script_watch(proto, ins, outs, pins, pout, &[])
}

/// As `sound_script`, with a Hide behind each output in `watch` (the first hides A, the second B): an output that
/// fired shows as a hidden object in the host.
fn sound_script_watch(
    proto: Guid,
    ins: &[&str],
    outs: &[&str],
    pins: &[(&str, Guid, Value)],
    pout: Option<&str>,
    watch: &[usize],
) -> (Runtime, Id, Id, Option<Id>) {
    let mut b = Build::new();
    let wave = b.g.add_object("Wind", ObjKind::Other);
    b.g.objects[wave].class = 25;
    b.g.waves.push(crate::Wave {
        object: wave,
        file: "Sounds\\Wind.wav".into(),
        looped: false,
        gain: 1.0,
        pan: 0.0,
        pitch: 1.0,
    });
    let target = b.local("Target", types::OBJECT, Value::Obj(Some(wave)));
    let pins: Vec<Id> = pins
        .iter()
        .map(|(n, g, v)| {
            let l = b.local(n, *g, v.clone());
            b.pin(n, *g, l)
        })
        .collect();
    let pouts: Vec<Id> = pout.map(|n| b.pout(n, types::FLOAT)).into_iter().collect();
    let first_out = pouts.first().copied();
    let blk = b.bb("Sound", proto, ins, outs, pins, pouts, vec![], Some(target));
    let start = b.io("Start");
    let mut sub = vec![blk];
    let mut links = vec![b.link(start, in_of(&b.g, blk, 0), 0)];
    let hier = b.local("Hierarchy", types::BOOL, Value::Bool(false));
    for (n, &o) in watch.iter().enumerate() {
        let t = b.local("Target", types::ENTITY, Value::Obj(Some(n)));
        let hide = b.bb(
            "Hide",
            proto::HIDE,
            &["In"],
            &["Out"],
            vec![hier],
            vec![],
            vec![],
            Some(t),
        );
        links.push(b.link(out_of(&b.g, blk, o), in_of(&b.g, hide, 0), 0));
        sub.push(hide);
    }
    let script = b.script("S", Some(0), start, sub, links);
    (Runtime::new(b.done()), script, blk, first_out)
}

#[test]
fn a_looping_wave_player_plays_and_finishes_its_run() {
    let (mut rt, script, blk, _) = sound_script_watch(
        proto::WAVE_PLAYER,
        &["Play", "Stop", "Pause/Resume"],
        &["Start Playing", "End Playing", "Paused/Resumed"],
        &[
            ("Fade In", types::FLOAT, Value::Float(0.0)),
            ("Fade Out", types::FLOAT, Value::Float(0.0)),
            ("Loop", types::BOOL, Value::Bool(true)),
        ],
        None,
        &[0, 1],
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(
        host.sounds,
        vec!["5 Sounds\\Wind.wav Play { looping: true, fade_in: 0.0 }"]
    );
    assert_eq!(
        host.visible,
        vec![false, true],
        "Start Playing fired, End Playing did not"
    );
    assert!(!rt.is_active(blk), "a looping Play returns at once");
    let _ = script;
}

#[test]
fn a_one_shot_wave_player_waits_for_the_sound_and_then_reports_the_end() {
    let (mut rt, _, blk, _) = sound_script_watch(
        proto::WAVE_PLAYER,
        &["Play", "Stop", "Pause/Resume"],
        &["Start Playing", "End Playing", "Paused/Resumed"],
        &[
            ("Fade In", types::FLOAT, Value::Float(0.5)),
            ("Fade Out", types::FLOAT, Value::Float(0.0)),
            ("Loop", types::BOOL, Value::Bool(false)),
        ],
        None,
        &[1],
    );
    let mut host = Rec::new();
    host.playing = true;
    rt.tick(&mut host);
    assert_eq!(
        host.sounds,
        vec!["5 Sounds\\Wind.wav Play { looping: false, fade_in: 0.5 }"]
    );
    rt.tick(&mut host);
    assert!(rt.is_active(blk), "stays active while the sound plays");
    assert_eq!(host.visible, vec![true, true], "no End Playing yet"); // (the one watched output hides A)
    host.playing = false;
    rt.tick(&mut host);
    assert_eq!(host.visible, vec![false, true], "End Playing fired");
    assert!(!rt.is_active(blk));
    assert_eq!(host.sounds.len(), 1, "no more commands");
}

#[test]
fn a_script_going_inactive_silences_its_wave_player() {
    let (mut rt, _, _, _) = sound_script(
        proto::WAVE_PLAYER,
        &["Play", "Stop", "Pause/Resume"],
        &["Start Playing", "End Playing", "Paused/Resumed"],
        &[("Loop", types::BOOL, Value::Bool(true))],
        None,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    rt.activate_object(0, false, false, &mut host);
    assert_eq!(
        host.sounds.last().map(String::as_str),
        Some("5 Sounds\\Wind.wav Stop { fade_out: 0.0 }")
    );
}

fn control(proto: Guid, name: &str, v: f32) -> Vec<String> {
    let (mut rt, _, _, _) = sound_script_watch(
        proto,
        &["In"],
        &["Out"],
        &[(name, types::FLOAT, Value::Float(v))],
        None,
        &[0],
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert!(!host.visible[0], "Out fired");
    host.sounds
}

#[test]
fn volume_pitch_and_panning_controls_set_their_value() {
    assert_eq!(
        control(proto::VOLUME_CONTROL, "Gain", 0.25),
        ["5 Sounds\\Wind.wav Gain(0.25)"]
    );
    assert_eq!(
        control(proto::PITCH_CONTROL, "Pitch", 1.5),
        ["5 Sounds\\Wind.wav Pitch(1.5)"]
    );
    assert_eq!(
        control(proto::PANNING_CONTROL, "Pan", -1.0),
        ["5 Sounds\\Wind.wav Pan(-1.0)"]
    );
}

fn instance(flat: bool) -> Vec<String> {
    let (mut rt, _, _, _) = sound_script_watch(
        proto::PLAY_SOUND_INSTANCE,
        &["In"],
        &["Out"],
        &[
            ("2D", types::BOOL, Value::Bool(flat)),
            ("Object", types::ENTITY, Value::Obj(Some(1))),
            ("Position", types::VECTOR, Value::Vec3([0.0, 2.0, 0.0])),
            ("Direction", types::VECTOR, Value::Vec3([0.0; 3])),
            ("Min Delay", types::FLOAT, Value::Float(0.0)),
            ("Volume", types::FLOAT, Value::Float(0.5)),
        ],
        None,
        &[0],
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert!(!host.visible[0], "Out fired");
    host.sounds
}

#[test]
fn a_sound_instance_is_flat_or_placed_at_the_object() {
    assert_eq!(
        instance(true),
        ["5 Sounds\\Wind.wav Instance { volume: 0.5, at: None }"]
    );
    // object B (1) sits at x = 10; the position pin is in its frame
    assert_eq!(
        instance(false),
        ["5 Sounds\\Wind.wav Instance { volume: 0.5, at: Some([10.0, 2.0, 0.0]) }"]
    );
}

#[test]
fn the_linear_volume_curve_follows_the_notes() {
    for (v, want) in [
        (2.0, 1.0),
        (1.0, 1.0),
        (0.5, 0.141_421),
        (0.01, 0.0),
        (0.0, 0.0),
        (0.02, 0.02 * 50f32.powf(0.02)),
    ] {
        let (mut rt, _, _, pout) = sound_script_watch(
            proto::TT_LINEAR_VOLUME,
            &["In"],
            &["Out"],
            &[("Vol. in Decibel", types::FLOAT, Value::Float(v))],
            Some("Linear Volume"),
            &[0],
        );
        let mut host = Rec::new();
        rt.tick(&mut host);
        assert!(!host.visible[0], "Out fired");
        let got = rt.values[pout.unwrap()].as_float();
        assert!((got - want).abs() < 1e-4, "v {v}: {got} vs {want}");
    }
}

#[test]
fn a_proximity_volume_follows_the_distance_and_ends_on_off() {
    let gain = |near: f32, far: f32| {
        let (mut rt, script, blk, _) = sound_script(
            proto::TT_PROXIMITY_VOLUME_CONTROL,
            &["On", "Off"],
            &["On", "Off"],
            &[
                ("Object 1", types::ENTITY, Value::Obj(Some(0))),
                ("Object 2", types::ENTITY, Value::Obj(Some(1))),
                ("Near", types::FLOAT, Value::Float(near)),
                ("Far", types::FLOAT, Value::Float(far)),
                ("Sound", types::SOUND, Value::Obj(Some(5))),
            ],
            None,
        );
        let mut host = Rec::new();
        rt.tick(&mut host);
        assert!(rt.is_active(blk) && rt.is_active(script), "stays on");
        host.sounds.clone()
    };
    // B is 10 units from A: inside Near the sound is at full volume, past Far at 0.02 (not silent), between the
    // squared distances linear: 1 - (100 - 25) / (400 - 25) = 0.8
    assert_eq!(gain(12.0, 20.0), ["5 Sounds\\Wind.wav Gain(1.0)"]);
    assert_eq!(gain(1.0, 5.0), ["5 Sounds\\Wind.wav Gain(0.02)"]);
    let want = 0.02 * 50f32.powf(0.8);
    let got = gain(5.0, 20.0);
    let got: f32 = got[0]
        .trim_start_matches("5 Sounds\\Wind.wav Gain(")
        .trim_end_matches(')')
        .parse()
        .unwrap();
    assert!((got - want).abs() < 1e-5, "{got} vs {want}");
}

// ---- input blocks (docs/gameplay_bbs.md, "Input blocks"): keys and mouse come from the host's tick input ----

/// A script on object 0 starting one input block through its input 0; returns the block and the runtime.
struct InputRig {
    rt: Runtime,
    host: Rec,
    blk: Id,
    g: Arc<Graph>,
}

impl InputRig {
    fn new(
        proto: Guid,
        ins: &[&str],
        outs: &[&str],
        pins: Vec<(Guid, Value)>,
        pouts: &[(&str, Guid)],
        locals: Vec<(Guid, Value)>,
    ) -> (InputRig, Vec<Id>) {
        let mut b = Build::new();
        let mut ps = Vec::new();
        for (g, v) in pins {
            let l = b.local("p", g, v);
            ps.push(b.pin("p", g, l));
        }
        let po: Vec<Id> = pouts.iter().map(|(n, g)| b.pout(n, *g)).collect();
        let ls: Vec<Id> = locals
            .into_iter()
            .map(|(g, v)| b.local("l", g, v))
            .collect();
        let blk = b.bb("blk", proto, ins, outs, ps, po.clone(), ls, None);
        let start = b.io("Start");
        let mut subs = vec![blk];
        let mut links = vec![b.link(start, in_of(&b.g, blk, 0), 0)];
        // each output sends the message "o<i>" to object 0, which the host records
        for i in 0..outs.len() {
            let m = b.local("m", types::MESSAGE, Value::Str(format!("o{i}")));
            let mp = b.pin("Message", types::MESSAGE, m);
            let d = b.local("d", types::ENTITY, Value::Obj(Some(0)));
            let dp = b.pin("Dest", types::ENTITY, d);
            let send = b.bb(
                "send",
                proto::SEND_MESSAGE,
                &["In"],
                &["Out"],
                vec![mp, dp],
                vec![],
                vec![],
                None,
            );
            links.push(b.link(out_of(&b.g, blk, i), in_of(&b.g, send, 0), 0));
            subs.push(send);
        }
        b.script("S", Some(0), start, subs, links);
        let g = b.done();
        let rig = InputRig {
            rt: Runtime::new(g.clone()),
            host: Rec::new(),
            blk,
            g,
        };
        (rig, po)
    }

    /// One tick with `keys` down (the previous tick's keys are the last call's).
    fn tick(&mut self, keys: &[u8], mouse: [i16; 2], buttons: u8) {
        self.host.before = self.host.frame;
        self.host.messages.clear();
        let mut f = crate::InputFrame::default();
        for &k in keys {
            f.set(k, true);
        }
        f.mouse = mouse;
        f.buttons = buttons;
        self.host.frame = f;
        self.rt.tick(&mut self.host);
    }

    /// Whether output `out` fired in the last tick.
    fn fired(&self, out: usize) -> bool {
        self.host.messages.contains(&format!("o{out}"))
    }
}

#[test]
fn key_event_fires_pressed_and_released_on_the_keys_of_the_tick() {
    let (mut r, po) = InputRig::new(
        proto::KEY_EVENT,
        &["On", "Off"],
        &["Pressed", "Released"],
        vec![
            (types::INT, Value::Int(0x1d)),
            (types::INT, Value::Int(0x39)),
        ],
        &[("Key State", types::BOOL)],
        vec![],
    );
    r.tick(&[0x1d], [0, 0], 0);
    assert!(!r.fired(0), "only one of the two keys is down");
    r.tick(&[0x1d, 0x39], [0, 0], 0);
    assert!(r.fired(0), "both down: Pressed");
    assert_eq!(r.rt.values[po[0]], Value::Bool(true));
    r.tick(&[0x1d, 0x39], [0, 0], 0);
    assert!(!r.fired(0), "Pressed fires once");
    r.tick(&[0x39], [0, 0], 0);
    assert!(r.fired(1), "a key let go: Released");
    assert_eq!(r.rt.values[po[0]], Value::Bool(false));
}

#[test]
fn key_waiter_waits_for_its_key_or_for_any_key_just_pressed() {
    let (mut r, _) = InputRig::new(
        proto::KEY_WAITER,
        &["In"],
        &["Out"],
        vec![(types::INT, Value::Int(0x1c))],
        &[("Key", types::INT)],
        vec![(types::BOOL, Value::Bool(false))],
    );
    r.tick(&[0x39], [0, 0], 0);
    assert!(!r.fired(0));
    r.tick(&[0x1c], [0, 0], 0);
    assert!(r.fired(0), "its key is down");
    let (mut r, po) = InputRig::new(
        proto::KEY_WAITER,
        &["In"],
        &["Out"],
        vec![(types::INT, Value::Int(0))],
        &[("Key", types::INT)],
        vec![(types::BOOL, Value::Bool(true))],
    );
    r.tick(&[0x39], [0, 0], 0);
    assert!(r.fired(0), "any key");
    assert_eq!(r.rt.values[po[0]], Value::Int(0x39));
    let (mut r, _) = InputRig::new(
        proto::KEY_WAITER,
        &["In"],
        &["Out"],
        vec![(types::INT, Value::Int(0))],
        &[("Key", types::INT)],
        vec![(types::BOOL, Value::Bool(true))],
    );
    r.host.frame.set(0x10, true);
    r.tick(&[0x10], [0, 0], 0);
    assert!(!r.fired(0), "a key already held is not a press");
}

#[test]
fn switch_on_key_fires_the_output_of_the_key_just_pressed() {
    let (mut r, _) = InputRig::new(
        proto::SWITCH_ON_KEY,
        &["On", "Off"],
        &["A", "B"],
        vec![
            (types::INT, Value::Int(0x1e)),
            (types::INT, Value::Int(0x30)),
        ],
        &[],
        vec![],
    );
    r.tick(&[], [0, 0], 0);
    r.tick(&[0x30], [0, 0], 0);
    assert!(!r.fired(0) && r.fired(1));
    r.tick(&[0x30], [0, 0], 0);
    assert!(!r.fired(1), "held, not pressed again");
}

#[test]
fn mouse_waiter_fires_move_and_button_edges_in_its_output_order() {
    // mask: Move, left pressed (bit 1), left released (bit 2) -> outputs 0, 1, 2
    let (mut r, po) = InputRig::new(
        proto::MOUSE_WAITER,
        &["On", "Off"],
        &["Move", "Left Pressed", "Left Released"],
        vec![],
        &[
            ("Mouse Position", types::VECTOR2),
            ("Wheel Direction", types::FLOAT),
            ("Left Button", types::BOOL),
            ("Middle Button", types::BOOL),
            ("Right Button", types::BOOL),
        ],
        vec![
            (types::BOOL, Value::Bool(true)),
            (types::INT, Value::Int(0b111)),
        ],
    );
    r.tick(&[], [0, 0], 0);
    assert!(!r.fired(0), "no move from the origin");
    r.tick(&[], [320, 240], 0);
    assert!(r.fired(0));
    assert_eq!(r.rt.values[po[0]], Value::Vec2([320.0, 240.0]));
    r.tick(&[], [320, 240], crate::input::LEFT);
    assert!(r.fired(1) && !r.fired(0));
    assert_eq!(r.rt.values[po[2]], Value::Bool(true));
    r.tick(&[], [320, 240], 0);
    assert!(r.fired(2));
}

fn typed(r: &mut InputRig, key: u8) {
    r.tick(&[], [0, 0], 0);
    r.tick(&[key], [0, 0], 0);
}

#[test]
fn input_string_types_edits_and_ends_on_return() {
    let (mut r, po) = InputRig::new(
        proto::INPUT_STRING,
        &["On", "Off"],
        &["On", "Off", "Updated String"],
        vec![
            (types::STRING, Value::Str("ab".into())),
            (types::INT, Value::Int(0)),
        ],
        &[("String", types::STRING), ("Key", types::INT)],
        vec![
            (types::INT, Value::Int(4)),
            (types::BOOL, Value::Bool(false)),
            (types::BOOL, Value::Bool(false)),
            (types::INT, Value::Int(0)),
            (types::BOOL, Value::Bool(false)),
        ],
    );
    r.tick(&[], [0, 0], 0);
    assert!(r.fired(0), "On");
    assert_eq!(r.rt.values[po[0]], Value::Str("ab".into()));
    typed(&mut r, 0x2e); // c
    assert_eq!(r.rt.values[po[0]], Value::Str("abc".into()));
    assert!(r.fired(2), "Updated String");
    assert_eq!(r.rt.values[po[1]], Value::Int(0x2e));
    typed(&mut r, 0x0e); // backspace
    assert_eq!(r.rt.values[po[0]], Value::Str("ab".into()));
    // Shift gives the capital
    r.tick(&[], [0, 0], 0);
    r.tick(&[0x2a, 0x20], [0, 0], 0);
    assert_eq!(r.rt.values[po[0]], Value::Str("abD".into()));
    // full at 4: the fifth character at the end is refused
    typed(&mut r, 0x21);
    typed(&mut r, 0x22);
    assert_eq!(r.rt.values[po[0]], Value::Str("abDf".into()));
    typed(&mut r, 0x1c); // Return
    assert!(r.fired(1), "Off on Return");
    assert_eq!(r.rt.values[po[0]], Value::Str("abDf".into()));
}

#[test]
fn input_string_is_bounded_at_256_characters() {
    let long = "x".repeat(1000);
    let (mut r, po) = InputRig::new(
        proto::INPUT_STRING,
        &["On", "Off"],
        &["On", "Off", "Updated String"],
        vec![
            (types::STRING, Value::Str(long)),
            (types::INT, Value::Int(0)),
        ],
        &[("String", types::STRING), ("Key", types::INT)],
        vec![(types::INT, Value::Int(100000))],
    );
    r.tick(&[], [0, 0], 0);
    assert_eq!(r.rt.values[po[0]], Value::Str("x".repeat(256)));
}

#[test]
fn the_cursor_blocks_only_show_or_hide_the_system_cursor() {
    for p in [
        proto::SHOW_MOUSE_CURSOR,
        proto::MOUSE_CURSOR,
        proto::MOUSE_CURSOR_SYSTEM,
    ] {
        let (mut r, _) = InputRig::new(
            p,
            &["In"],
            &["Out"],
            vec![(types::BOOL, Value::Bool(true))],
            &[],
            vec![],
        );
        r.tick(&[], [0, 0], 0);
        assert!(r.fired(0));
        assert_eq!(r.host.cursor, Some(true));
    }
}
// ---- cameras (docs/gameplay_bbs.md, "Cameras"): the changes go to the host; the reading comes from it ----

/// A graph with two objects (0 owns the script, 1 is a camera) and one camera block of `proto` with `pins` set and
/// `pouts` declared, targeted at the camera when `target` is set. Returns the runtime, the block and its pouts.
fn camera_script(
    proto: Guid,
    ins: &[&str],
    outs: &[&str],
    pins: &[(&str, Guid, Value)],
    pouts: &[(&str, Guid)],
    target: bool,
) -> (Runtime, Id, Vec<Id>) {
    let mut b = Build::new();
    let owner = b.g.add_object("Owner", ObjKind::Other);
    let cam = b.g.add_object("Cam", ObjKind::Other);
    b.g.objects[cam].class = 34;
    b.g.objects[cam].kind = ObjKind::Entity(1);
    let t = b.local("Target", types::OBJECT, Value::Obj(Some(cam)));
    let pins: Vec<Id> = pins
        .iter()
        .map(|(n, g, v)| {
            let l = b.local(n, *g, v.clone());
            b.pin(n, *g, l)
        })
        .collect();
    let pouts: Vec<Id> = pouts.iter().map(|(n, g)| b.pout(n, *g)).collect();
    let blk = b.bb(
        "Camera",
        proto,
        ins,
        outs,
        pins,
        pouts.clone(),
        vec![],
        target.then_some(t),
    );
    let start = b.io("Start");
    let links = vec![b.link(start, in_of(&b.g, blk, 0), 0)];
    b.script("S", Some(owner), start, vec![blk], links);
    (Runtime::new(b.done()), blk, pouts)
}

#[test]
fn get_current_camera_writes_the_host_camera_to_its_outputs() {
    let (mut rt, _, pouts) = camera_script(
        proto::GET_CURRENT_CAMERA,
        &["In"],
        &["Out"],
        &[],
        &[
            ("Current Camera", types::OBJECT),
            ("Position", types::VECTOR),
        ],
        false,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(rt.values[pouts[0]], Value::Obj(Some(7)));
    assert_eq!(rt.values[pouts[1]], Value::Vec3([1.0, 2.0, 3.0]));
    assert!(host.cams.is_empty(), "reading the camera changes nothing");
}

#[test]
fn set_as_active_camera_attaches_the_view_to_the_target_object() {
    let (mut rt, _, _) = camera_script(
        proto::SET_AS_ACTIVE_CAMERA,
        &["In"],
        &["Out"],
        &[],
        &[],
        true,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(host.cams, vec![CameraChange::Active(1)]);
}

#[test]
fn set_clipping_planes_applies_a_near_below_a_far_and_ignores_the_rest() {
    let planes = |near: f32, far: f32| {
        camera_script(
            proto::SET_CLIPPING_PLANES,
            &["In"],
            &["Out"],
            &[
                ("Near", types::FLOAT, Value::Float(near)),
                ("Far", types::FLOAT, Value::Float(far)),
            ],
            &[],
            false,
        )
    };
    let (mut rt, _, _) = planes(0.5, 100.0);
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(
        host.cams,
        vec![CameraChange::Clip {
            near: 0.5,
            far: 100.0
        }]
    );
    for (near, far) in [(0.0, 100.0), (-1.0, 100.0), (5.0, 5.0), (f32::NAN, 100.0)] {
        let (mut rt, _, _) = planes(near, far);
        let mut host = Rec::new();
        rt.tick(&mut host);
        assert!(host.cams.is_empty(), "{near} {far} is ignored");
    }
}

#[test]
fn set_fov_gives_the_angle_in_degrees_and_ignores_a_non_finite_one() {
    let fov = |a: f32| {
        camera_script(
            proto::SET_FOV,
            &["In"],
            &["Out"],
            &[("Angle", types::FLOAT, Value::Float(a))],
            &[],
            false,
        )
    };
    let (mut rt, _, _) = fov(1.012_291);
    let mut host = Rec::new();
    rt.tick(&mut host);
    match host.cams.as_slice() {
        [CameraChange::Fov(d)] => assert!((d - 58.0).abs() < 0.01, "{d}"),
        other => panic!("{other:?}"),
    }
    let (mut rt, _, _) = fov(f32::NAN);
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert!(host.cams.is_empty());
}

// ---- materials and textures (Materials.dll): the changes go to the host for this run ------------------------------

/// A graph with material 0 (the target), material 1, and texture 0, and one block of `proto` with `pins`
/// (object values name them by `Value::Obj(Some(..))`): returns the runtime and the ids of the three objects.
fn material_script(
    proto: Guid,
    pins: Vec<(&str, Guid, Value)>,
    pouts: &[(&str, Guid)],
    outs: &[&str],
    target: Option<usize>,
) -> (Runtime, [Id; 3], Vec<Id>) {
    let mut b = Build::new();
    let owner = b.g.add_object("Owner", ObjKind::Other);
    let m0 = b.g.add_object("M0", ObjKind::Material(0));
    let m1 = b.g.add_object("M1", ObjKind::Material(1));
    let t0 = b.g.add_object("T0", ObjKind::Texture(0));
    let ids = [m0, m1, t0];
    let t = target.map(|i| b.local("Target", types::OBJECT, Value::Obj(Some(ids[i]))));
    let pins: Vec<Id> = pins
        .iter()
        .map(|(n, g, v)| {
            let l = b.local(n, *g, v.clone());
            b.pin(n, *g, l)
        })
        .collect();
    let pouts: Vec<Id> = pouts.iter().map(|(n, g)| b.pout(n, *g)).collect();
    let mut all = vec!["In"];
    all.truncate(1);
    let blk = b.bb("Mat", proto, &all, outs, pins, pouts.clone(), vec![], t);
    let start = b.io("Start");
    let links = vec![b.link(start, in_of(&b.g, blk, 0), 0)];
    b.script("S", Some(owner), start, vec![blk], links);
    (Runtime::new(b.done()), ids, pouts)
}

fn run_material(
    proto: Guid,
    pins: Vec<(&str, Guid, Value)>,
    target: usize,
) -> Vec<(usize, crate::MatChange)> {
    let (mut rt, _, _) = material_script(proto, pins, &[], &["Out"], Some(target));
    let mut host = Rec::new();
    rt.tick(&mut host);
    host.mats
}

#[test]
fn the_material_blocks_hand_their_change_to_the_host() {
    use crate::MatChange as C;
    let color = Value::Color([0.1, 0.2, 0.3, 0.4]);
    assert_eq!(
        run_material(
            proto::SET_DIFFUSE,
            vec![
                ("Diffuse Color", types::COLOR, color.clone()),
                ("Keep Alpha Component", types::BOOL, Value::Bool(true)),
            ],
            1
        ),
        vec![(
            1,
            C::Diffuse {
                color: [0.1, 0.2, 0.3, 0.4],
                keep_alpha: true
            }
        )]
    );
    assert_eq!(
        run_material(
            proto::SET_EMISSIVE,
            vec![("Emissive Color", types::COLOR, color.clone())],
            0
        ),
        vec![(0, C::Emissive([0.1, 0.2, 0.3]))]
    );
    assert_eq!(
        run_material(
            proto::SET_AMBIENT,
            vec![("Ambient Color", types::COLOR, color.clone())],
            0
        ),
        vec![(0, C::Ambient([0.1, 0.2, 0.3]))]
    );
    assert_eq!(
        run_material(
            proto::SET_SPECULAR,
            vec![
                ("Specular Color", types::COLOR, color),
                ("Specular Power", types::FLOAT, Value::Float(8.0)),
            ],
            0
        ),
        vec![(
            0,
            C::Specular {
                color: [0.1, 0.2, 0.3],
                power: 8.0
            }
        )]
    );
    assert_eq!(
        run_material(
            proto::SET_BLEND_MODES,
            vec![
                ("Blend", types::BOOL, Value::Bool(true)),
                ("Source Blend", types::INT, Value::Int(5)),
                ("Destination Blend", types::INT, Value::Int(6)),
            ],
            0
        ),
        vec![(
            0,
            C::Blend {
                on: true,
                src: 5,
                dst: 6
            }
        )]
    );
    assert_eq!(
        run_material(
            proto::SET_MATERIAL_Z_BUFFER,
            vec![
                ("Z Buffer Write", types::BOOL, Value::Bool(false)),
                ("Z Compare Function", types::INT, Value::Int(4)),
            ],
            0
        ),
        vec![(
            0,
            C::ZBuffer {
                write: false,
                func: 4
            }
        )]
    );
    assert_eq!(
        run_material(
            proto::SET_ALPHA_TEST,
            vec![
                ("Alpha Test Enable", types::BOOL, Value::Bool(true)),
                ("Alpha Test Function", types::INT, Value::Int(7)),
                ("Alpha Reference Value", types::INT, Value::Int(999)),
            ],
            0
        ),
        vec![(
            0,
            C::AlphaTest {
                on: true,
                func: 7,
                reference: 255
            }
        )]
    );
}

#[test]
fn set_texture_names_a_level_texture_or_clears_it() {
    use crate::MatChange as C;
    let (mut rt, ids, _) = material_script(
        proto::SET_TEXTURE,
        vec![("Texture", types::OBJECT, Value::Obj(None))],
        &[],
        &["Out"],
        Some(0),
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(host.mats, vec![(0, C::Texture(None))]);
    // the texture object (index 0) is a Texture; a material object in that pin is not one
    let tex = ids[2];
    let (mut rt, _, _) = material_script(
        proto::SET_TEXTURE,
        vec![("Texture", types::OBJECT, Value::Obj(Some(tex)))],
        &[],
        &["Out"],
        Some(1),
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(host.mats, vec![(1, C::Texture(Some(0)))]);
}

#[test]
fn set_material_copies_the_new_material_over_the_replaced_one() {
    use crate::MatChange as C;
    // ids: the target material 0 is not what is replaced; "Material" is material 1, "to replace" material 0
    let mut b = Build::new();
    let owner = b.g.add_object("Owner", ObjKind::Other);
    let m0 = b.g.add_object("M0", ObjKind::Material(0));
    let m1 = b.g.add_object("M1", ObjKind::Material(1));
    let pins: Vec<Id> = [("Material", m1), ("Material to replace", m0)]
        .iter()
        .map(|(n, o)| {
            let l = b.local(n, types::OBJECT, Value::Obj(Some(*o)));
            b.pin(n, types::OBJECT, l)
        })
        .collect();
    let blk = b.bb(
        "SetMat",
        proto::SET_MATERIAL,
        &["In"],
        &["Out"],
        pins,
        vec![],
        vec![],
        None,
    );
    let start = b.io("Start");
    let links = vec![b.link(start, in_of(&b.g, blk, 0), 0)];
    b.script("S", Some(owner), start, vec![blk], links);
    let mut rt = Runtime::new(b.done());
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(host.mats, vec![(0, C::CopyFrom(1))]);
}

#[test]
fn texture_blocks_record_size_and_slot_and_report_info() {
    use crate::TexChange as T;
    let (mut rt, _, _) = material_script(
        proto::CHANGE_TEXTURE_SIZE,
        vec![
            ("Size", types::VECTOR2, Value::Vec2([128.0, 64.0])),
            ("Clear", types::BOOL, Value::Bool(true)),
        ],
        &[],
        &["Out"],
        Some(2),
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(
        host.texs,
        vec![(
            0,
            T::Size {
                width: 128,
                height: 64,
                clear: true
            }
        )]
    );
    let (mut rt, _, _) = material_script(
        proto::SET_CURRENT_SLOT,
        vec![("Slot Index", types::INT, Value::Int(2))],
        &[],
        &["Out"],
        Some(2),
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(host.texs, vec![(0, T::Slot(2))]);
    let (mut rt, _, pouts) = material_script(
        proto::GET_TEXTURE_INFO,
        vec![],
        &[("FileName", types::STRING), ("Priority", types::INT)],
        &["Exist", "Empty"],
        Some(2),
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(rt.values[pouts[0]], Value::Str("wall.bmp".into()));
    // a material is not a texture: "Empty" fires and nothing is written
    let (mut rt, _, pouts) = material_script(
        proto::GET_TEXTURE_INFO,
        vec![],
        &[("FileName", types::STRING), ("Priority", types::INT)],
        &["Exist", "Empty"],
        Some(0),
    );
    rt.tick(&mut host);
    assert_eq!(rt.values[pouts[0]], Value::None);
}

// ---- text: Text Display, 2D Text and the font blocks ------------------------------------------------------------

/// One block of `proto` with the given pins (name, type, value), its inputs and outputs, run from a start input
/// that is linked to input `first_in`; returns the runtime, the block's id and the graph builder's output.
fn text_rig(
    proto: Guid,
    ins: &[&str],
    outs: &[&str],
    pins: &[(&str, Guid, Value)],
    pouts: &[(&str, Guid)],
    target_class: u32,
) -> (Runtime, Id) {
    let mut b = Build::new();
    let pin_ids: Vec<Id> = pins
        .iter()
        .map(|(n, g, v)| {
            let l = b.local(n, *g, v.clone());
            b.pin(n, *g, l)
        })
        .collect();
    let pout_ids: Vec<Id> = pouts.iter().map(|(n, g)| b.pout(n, *g)).collect();
    let frame = b.g.add_object("Frame", ObjKind::Other);
    b.g.objects[frame].class = target_class;
    let target = b.local("Target", types::ENTITY, Value::Obj(Some(frame)));
    let blk = b.bb(
        "B",
        proto,
        ins,
        outs,
        pin_ids,
        pout_ids,
        vec![],
        Some(target),
    );
    let start = b.io("Start");
    let links = vec![b.link(start, in_of(&b.g, blk, 0), 0)];
    b.script("S", Some(frame), start, vec![blk], links);
    (Runtime::new(b.done()), blk)
}

fn last_box(host: &Rec) -> Option<crate::TextSpec> {
    host.boxes.last().and_then(|(_, s)| s.clone())
}

#[test]
fn text_display_sends_its_box_and_joins_its_text_pins() {
    let pins = [
        ("Offset", types::VECTOR2, Value::Vec2([10.0, 20.0])),
        ("Color", types::COLOR, Value::Color([1.0, 0.5, 0.0, 1.0])),
        ("Alignment", types::INT, Value::Int(2 | 16)),
        ("Size", types::INT, Value::Int(15)),
        ("Text", types::STRING, Value::Str("Score".into())),
        ("Value", types::STRING, Value::Str("42".into())),
    ];
    let (mut rt, _) = text_rig(
        proto::TEXT_DISPLAY,
        &["On", "Off"],
        &["Exit On", "Exit Off"],
        &pins,
        &[],
        27,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    let s = last_box(&host).expect("a box");
    assert_eq!(
        s.text, "Score 42 ",
        "every text pin's string followed by a space"
    );
    assert_eq!(
        (s.x, s.y, s.w, s.h),
        (10.0, 20.0, 320.0, 32.0),
        "offset and the default sprite size"
    );
    assert_eq!(s.size, 20.0, "15 points are 20 pixels");
    assert_eq!(s.color, [1.0, 0.5, 0.0, 1.0]);
    assert_eq!(
        s.align,
        text_align_bits(true, false, false, true),
        "Left and Bottom"
    );
}

fn text_align_bits(left: bool, right: bool, top: bool, bottom: bool) -> u8 {
    use crate::text_align::*;
    [(left, LEFT), (right, RIGHT), (top, TOP), (bottom, BOTTOM)]
        .iter()
        .filter(|(on, _)| *on)
        .fold(0, |a, (_, b)| a | b)
}

#[test]
fn text_display_off_takes_the_box_away_and_a_deactivated_script_too() {
    let pins = [
        ("Offset", types::VECTOR2, Value::Vec2([0.0, 0.0])),
        ("Color", types::COLOR, Value::Color([1.0; 4])),
        ("Alignment", types::INT, Value::Int(2)),
        ("Size", types::INT, Value::Int(10)),
        ("Text", types::STRING, Value::Str("x".into())),
    ];
    let (mut rt, blk) = text_rig(
        proto::TEXT_DISPLAY,
        &["On", "Off"],
        &["Exit On", "Exit Off"],
        &pins,
        &[],
        27,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert!(last_box(&host).is_some());
    let b = rt.graph.block(blk).unwrap().clone();
    crate::blocks::event(
        &mut rt,
        blk,
        &b,
        crate::blocks::BlockEvent::DeactivateScript,
        &mut host,
    );
    assert_eq!(
        host.boxes.last(),
        Some(&(blk, None)),
        "deactivating the script removes the box"
    );
    // Off input
    let off = in_of(&rt.graph, blk, 1);
    rt.set_io(off, true);
    host.boxes.clear();
    rt.tick(&mut host);
    assert_eq!(
        host.boxes.first(),
        Some(&(blk, None)),
        "Off removes the box"
    );
}

#[test]
fn text_2d_uses_the_font_margins_and_alignment_and_counts_lines() {
    let pins = [
        ("Font", types::INT, Value::Int(1)),
        ("Text", types::STRING, Value::Str("one\ntwo".into())),
        ("Alignment", types::INT, Value::Int(1 | 4)),
        ("Margins", types::RECT, Value::Rect([2.0, 3.0, 4.0, 5.0])),
        ("Offset", types::VECTOR2, Value::Vec2([1.0, 1.0])),
    ];
    let (mut rt, blk) = text_rig(
        proto::TEXT_2D,
        &["On", "Off"],
        &["Exit On", "Exit Off"],
        &pins,
        &[("Text Extents", types::RECT), ("Line Count", types::INT)],
        27,
    );
    rt.fonts.push(crate::Font {
        name: "f".into(),
        scale: [0.5, 0.25],
        color: [0.0, 1.0, 0.0, 1.0],
        bold: true,
    });
    let mut host = Rec::new();
    rt.tick(&mut host);
    let s = last_box(&host).expect("a box");
    assert_eq!(
        (s.x, s.y, s.w, s.h),
        (3.0, 4.0, 634.0, 472.0),
        "margins inside the frame plus the offset"
    );
    assert_eq!(s.size, 8.0, "the font's scale 0.25 of a 32 pixel cell");
    assert_eq!(s.color, [0.0, 1.0, 0.0, 1.0]);
    assert!(s.bold);
    assert_eq!(s.align, text_align_bits(true, false, true, false));
    let lc = rt.graph.block(blk).unwrap().pouts[1];
    assert_eq!(rt.values[lc], Value::Int(2));
}

#[test]
fn text_2d_on_something_that_is_not_a_2d_entity_draws_nothing() {
    let pins = [("Text", types::STRING, Value::Str("x".into()))];
    let (mut rt, _) = text_rig(
        proto::TEXT_2D,
        &["On", "Off"],
        &["Exit On", "Exit Off"],
        &pins,
        &[],
        33,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert!(host.boxes.is_empty());
}

#[test]
fn font_blocks_make_fonts_and_set_their_properties() {
    // TT CreateFontEx without a texture makes no font
    let (mut rt, _) = text_rig(
        proto::TT_CREATE_FONT_EX,
        &["In"],
        &["Out"],
        &[
            ("Font Name", types::STRING, Value::Str("F".into())),
            ("Font Texture", types::TEXTURE, Value::Obj(None)),
        ],
        &[("Font", types::INT)],
        27,
    );
    rt.tick(&mut Rec::new());
    assert!(rt.fonts.is_empty());
    // with a texture it does, and a second font of the name replaces it
    let (mut rt, blk) = text_rig(
        proto::TT_CREATE_FONT_EX,
        &["In"],
        &["Out"],
        &[
            ("Font Name", types::STRING, Value::Str("F".into())),
            ("Font Texture", types::TEXTURE, Value::Obj(Some(0))),
        ],
        &[("Font", types::INT)],
        27,
    );
    rt.tick(&mut Rec::new());
    assert_eq!(rt.fonts.len(), 1);
    let f = rt.graph.block(blk).unwrap().pouts[0];
    assert_eq!(rt.values[f], Value::Int(1));
    // Create System Font: bold from weight 700
    let (mut rt, blk) = text_rig(
        proto::CREATE_SYSTEM_FONT,
        &["In"],
        &["Success", "Error"],
        &[
            ("Font Name", types::STRING, Value::Str("S".into())),
            ("System Font Name", types::INT, Value::Int(0)),
            ("Font Weight", types::INT, Value::Int(700)),
        ],
        &[("Font Created", types::INT)],
        27,
    );
    rt.tick(&mut Rec::new());
    assert!(rt.fonts[0].bold);
    let f = rt.graph.block(blk).unwrap().pouts[0];
    assert_eq!(rt.values[f], Value::Int(1));
    // Set Font Properties
    let (mut rt, _) = text_rig(
        proto::SET_FONT_PROPERTIES,
        &["In"],
        &["Out"],
        &[
            ("Font", types::INT, Value::Int(1)),
            ("Space", types::VECTOR2, Value::Vec2([0.0; 2])),
            ("Scale", types::VECTOR2, Value::Vec2([0.4, 0.5])),
            ("Italic Offset", types::FLOAT, Value::Float(0.0)),
            ("Color", types::COLOR, Value::Color([0.1, 0.2, 0.3, 1.0])),
        ],
        &[],
        27,
    );
    rt.fonts.push(crate::Font {
        name: "f".into(),
        scale: [1.0; 2],
        color: [1.0; 4],
        bold: false,
    });
    rt.tick(&mut Rec::new());
    assert_eq!(rt.fonts[0].scale, [0.4, 0.5]);
    assert_eq!(rt.fonts[0].color, [0.1, 0.2, 0.3, 1.0]);
}

#[test]
fn three_d_text_and_movie_player_are_named_no_ops() {
    for p in [proto::TEXT_3D, proto::MOVIE_PLAYER] {
        let (mut rt, _) = text_rig(p, &["In"], &["Out"], &[], &[], 27);
        let mut host = Rec::new();
        rt.tick(&mut host);
        assert!(host.boxes.is_empty());
        assert!(
            host.notes.iter().any(|n| n.contains("does nothing")),
            "said once"
        );
    }
}

/// A planar emitter block on object 0 with the given pin values, its three inputs and a script that starts it.
fn emitter_script(shape_proto: Guid) -> (Arc<Graph>, Id, [Id; 3]) {
    let mut b = Build::new();
    let mut pins = Vec::new();
    for (name, v) in [
        ("Emission Delay", Value::Float(20.0)),
        ("Emission Delay Variance", Value::Float(0.0)),
        ("Yaw Variance", Value::Float(0.1)),
        ("Pitch Variance", Value::Float(0.2)),
        ("Speed", Value::Float(0.0055)),
        ("Speed Variance", Value::Float(0.001)),
        ("Angular Speed/Spreading", Value::Float(0.0)),
        (
            "Angular Speed Variance/Spreading Variation",
            Value::Float(0.0),
        ),
        ("Lifespan", Value::Float(800.0)),
        ("Lifespan Variance", Value::Float(25.0)),
        ("Maximum Number", Value::Int(80)),
        ("Emission", Value::Int(3)),
        ("Emission Variance", Value::Int(1)),
        ("Initial Size", Value::Float(0.7)),
        ("Initial Size Variance", Value::Float(0.0)),
        ("Ending Size", Value::Float(0.2)),
    ] {
        let l = b.local(name, types::NONE, v);
        pins.push(b.pin(name, types::NONE, l));
    }
    let mut locals = Vec::new();
    for v in [
        Value::Raw(vec![0; 4]),
        Value::Int(1),
        Value::Float(0.0),
        Value::Int(40),
        Value::Int(3),
        Value::Int(5),
        Value::Int(2),
        Value::Obj(None),
        Value::Int(3),
        Value::Int(12),
    ] {
        locals.push(b.local("setting", types::NONE, v));
    }
    let block = b.bb(
        "PlanarParticleSystem",
        shape_proto,
        &["On", "Off", "Freeze"],
        &["Exit On", "Exit Off", "Exit Freeze"],
        pins,
        vec![],
        locals,
        None,
    );
    let ins = [0, 1, 2].map(|i| in_of(&b.g, block, i));
    let start = b.io("Start");
    let l = b.link(start, ins[0], 0);
    let script = b.script("S", Some(0), start, vec![block], vec![l]);
    let _ = script;
    (b.done(), block, ins)
}

#[test]
fn a_planar_emitter_reads_its_pins_and_follows_on_freeze_off() {
    use crate::EmitState::{Frozen, Off, On};
    let (g, block, ins) = emitter_script(proto::PLANAR_PARTICLE_SYSTEM);
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.tick(&mut host);
    let (key, ent, spec, state) = host.emitters.last().cloned().expect("On reaches the host");
    assert_eq!((key, ent, state), (block, Ent::Object(0), On));
    assert_eq!(spec.shape, crate::EmitShape::Planar);
    assert_eq!((spec.delay, spec.speed, spec.life), (20.0, 0.0055, 800.0));
    assert_eq!((spec.emission, spec.emission_var), (3, 1));
    assert_eq!(spec.max, 40, "the pool-size setting caps the pin's 80");
    assert_eq!((spec.src_blend, spec.dst_blend, spec.render), (5, 2, 3));
    assert_eq!(spec.variances, 12);
    // freeze toggles: frozen, then back on
    rt.set_io(ins[2], true);
    rt.tick(&mut host);
    assert_eq!(host.emitters.last().map(|e| e.3), Some(Frozen));
    rt.tick(&mut host);
    assert_eq!(
        host.emitters.last().map(|e| e.3),
        Some(Frozen),
        "stays frozen"
    );
    rt.set_io(ins[2], true);
    rt.tick(&mut host);
    assert_eq!(host.emitters.last().map(|e| e.3), Some(On));
    rt.set_io(ins[1], true);
    rt.tick(&mut host);
    assert_eq!(host.emitters.last().map(|e| e.3), Some(Off));
}

#[test]
fn a_spherical_emitter_has_its_own_shape() {
    let (g, _, _) = emitter_script(proto::SPHERICAL_PARTICLE_SYSTEM);
    let mut rt = Runtime::new(g);
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(
        host.emitters.last().map(|e| e.2.shape),
        Some(crate::EmitShape::Spherical)
    );
}

// ---- the scene's look: Set Light Color, Set Fog, TT Sky, Edit 2D Entity, Set Pickable ---------------------------

#[test]
fn set_light_color_names_the_target_light_and_clamps_the_colour() {
    let pins = [(
        "Color",
        types::COLOR,
        Value::Color([0.25, 2.0, f32::NAN, 1.0]),
    )];
    let (mut rt, _) = text_rig(
        crate::blocks::environment::SET_LIGHT_COLOR,
        &["In"],
        &["Out"],
        &pins,
        &[],
        38,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(
        host.env,
        vec![crate::EnvChange::LightColor {
            light: "Frame".into(),
            color: [0.25, 1.0, 0.0, 1.0]
        }]
    );
}

fn fog_pins(mode: i32, start: f32, end: f32) -> Vec<(&'static str, Guid, Value)> {
    vec![
        ("Fog Mode", types::INT, Value::Int(mode)),
        ("Fog Start", types::FLOAT, Value::Float(start)),
        ("Fog End", types::FLOAT, Value::Float(end)),
        ("Fog Density", types::FLOAT, Value::Float(0.5)),
        (
            "Fog Color",
            types::COLOR,
            Value::Color([0.1, 0.2, 0.3, 1.0]),
        ),
    ]
}

#[test]
fn set_fog_sets_a_linear_fog_and_mode_none_or_a_bad_range_switches_it_off() {
    let run = |mode, start, end| {
        let (mut rt, _) = text_rig(
            crate::blocks::environment::SET_FOG,
            &["In"],
            &["Out"],
            &fog_pins(mode, start, end),
            &[],
            0,
        );
        let mut host = Rec::new();
        rt.tick(&mut host);
        host.env
    };
    assert_eq!(
        run(3, 35.0, 60.0),
        vec![crate::EnvChange::Fog(Some(crate::FogSpec {
            mode: 3,
            start: 35.0,
            end: 60.0,
            density: 0.5,
            color: [0.1, 0.2, 0.3, 1.0]
        }))]
    );
    assert_eq!(run(0, 35.0, 60.0), vec![crate::EnvChange::Fog(None)]);
    assert_eq!(
        run(3, 60.0, 35.0),
        vec![crate::EnvChange::Fog(None)],
        "end before start"
    );
    assert!(matches!(run(2, 0.0, 0.0)[0], crate::EnvChange::Fog(Some(f)) if f.mode == 2));
}

#[test]
fn tt_sky_stays_on_until_off_and_off_takes_it_away() {
    let mut pins = vec![
        ("Distortion", types::FLOAT, Value::Float(0.15)),
        ("Vertex Color", types::COLOR, Value::Color([1.0; 4])),
        ("Orientation Object", types::ENTITY, Value::Obj(None)),
        ("Radius", types::FLOAT, Value::Float(100.0)),
        ("Quadratic SideFaces?", types::BOOL, Value::Bool(true)),
        ("or SideFace-Heigth", types::FLOAT, Value::Float(10.0)),
        ("Y-Position of Sky", types::FLOAT, Value::Float(0.0)),
    ];
    for _ in 0..5 {
        pins.push(("Side-Mat", types::MATERIAL, Value::Obj(Some(1))));
    }
    let (mut rt, blk) = text_rig(
        crate::blocks::environment::TT_SKY,
        &["On", "Off"],
        &["Exit On", "Exit Off"],
        &pins,
        &[],
        0,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    rt.tick(&mut host);
    assert!(host.env.len() >= 2, "every tick while on");
    match &host.env[0] {
        crate::EnvChange::Sky(Some(s)) => {
            assert_eq!(
                s.sides.len(),
                4,
                "four sides without a Side Materials local"
            );
            assert_eq!(s.radius, 100.0);
            assert_eq!(s.distortion, 0.15);
        }
        other => panic!("{other:?}"),
    }
    let off = in_of(&rt.graph, blk, 1);
    rt.set_io(off, true);
    host.env.clear();
    rt.tick(&mut host);
    assert_eq!(host.env, vec![crate::EnvChange::Sky(None)]);
}

#[test]
fn edit_2d_entity_keeps_what_it_edits_and_2d_text_lays_out_inside_it() {
    let pins = [
        ("Size X", types::FLOAT, Value::Float(0.5)),
        ("Size Y", types::FLOAT, Value::Float(0.25)),
    ];
    let (mut rt, _) = text_rig(
        crate::blocks::environment::EDIT_2D_ENTITY,
        &["In"],
        &["Out"],
        &pins,
        &[],
        27,
    );
    rt.tick(&mut Rec::new());
    assert_eq!(rt.ents2d.len(), 1);
    assert_eq!(rt.ents2d[0].1.size, Some([0.5, 0.25]));
    assert_eq!(rt.ents2d[0].1.pos, None, "position not edited");
}

#[test]
fn set_pickable_only_passes_on() {
    let pins = [("Pickable", types::BOOL, Value::Bool(false))];
    let (mut rt, blk) = text_rig(
        crate::blocks::environment::SET_PICKABLE,
        &["In"],
        &["Out"],
        &pins,
        &[],
        0,
    );
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert!(host.env.is_empty());
    let inp = rt.graph.block(blk).unwrap().inputs[0];
    assert!(!rt.io_active(inp), "the input was taken");
}
