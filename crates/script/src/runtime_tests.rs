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
    assert_eq!(rt.values[row_out], Value::Int(1));
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
    let delayer = b.bb(
        "Delayer",
        proto::DELAYER,
        &["In"],
        &["Out"],
        vec![wait_pin],
        vec![],
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
    for _ in 0..3 {
        rt.tick(&mut host);
        assert_eq!(
            host.visible,
            vec![true, true],
            "40 ms is three ticks at 60 Hz, the message comes after"
        );
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
    let r = crate::blocks::arith(|x, y| x + y, i32::wrapping_add, &big, &big, false);
    assert!(r.as_string().len() <= (1 << 16) + 8);
}
