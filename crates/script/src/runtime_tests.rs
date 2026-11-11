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
use std::sync::Arc;

/// A host that records what the scripts asked of it; two level objects (0 and 1) and the ball.
#[derive(Default)]
struct Rec {
    matrices: Vec<[[f32; 4]; 4]>,
    visible: Vec<bool>,
    notes: Vec<String>,
    messages: Vec<String>,
    ball: [f32; 3],
    in_play: bool,
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
    fn sound(&mut self, _: &str, _: bool) {}
    fn text(&mut self, _: &str, text: &str, _: bool) {
        self.notes.push(format!("text {text}"));
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
    let limit = std::time::Duration::from_secs(10);
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
    // blocks and Set Row among them)
    assert_eq!(all.len(), 101);
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
