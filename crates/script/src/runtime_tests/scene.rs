// Scene-changing blocks on hand-built graphs: Object Create, Delete Dynamic Objects, Object Delete of made
// objects, Set Parent, Attach Script, Set IC, Set Physics Slider, Iterator If, Shuffle Order.

use super::*;

/// Wires `blocks` in a row (Start -> first.In, each Out -> next In) in one script on no object.
fn chain(b: &mut Build, blocks: &[Id]) -> Id {
    let start = b.io("Start");
    let mut links = vec![b.link(start, in_of(&b.g, blocks[0], 0), 0)];
    for w in blocks.windows(2) {
        links.push(b.link(out_of(&b.g, w[0], 0), in_of(&b.g, w[1], 0), 0));
    }
    b.script("S", None, start, blocks.to_vec(), links)
}

/// An Object Create block of `class` and `name`; returns (block, the Object output).
fn create(b: &mut Build, class: i32, name: &str, dynamic: bool) -> (Id, Id) {
    let c = b.local("Class", types::CLASS_ID, Value::Int(class));
    let cp = b.pin("Class", types::CLASS_ID, c);
    let n = b.local("Name", types::STRING, Value::Str(name.into()));
    let np = b.pin("Name", types::STRING, n);
    let o = b.pout("Object", types::BEOBJECT);
    let d = b.local("Dynamic", types::BOOL, Value::Bool(dynamic));
    let blk = b.bb(
        "Object Create",
        proto::OBJECT_CREATE,
        &["In"],
        &["Out"],
        vec![cp, np],
        vec![o],
        vec![d],
        None,
    );
    (blk, o)
}

fn delete_dynamic(b: &mut Build) -> Id {
    let scene = b.local("Scene", types::BEOBJECT, Value::Obj(None));
    let p = b.pin("Scene", types::BEOBJECT, scene);
    b.bb(
        "Delete Dynamic Objects",
        proto::DELETE_DYNAMIC_OBJECTS,
        &["In"],
        &["Out"],
        vec![p],
        vec![],
        vec![],
        None,
    )
}

fn object_delete(b: &mut Build, of: Id) -> Id {
    let p = b.pin("Object", types::BEOBJECT, of);
    b.bb(
        "Object Delete",
        proto::OBJECT_DELETE,
        &["In"],
        &["Out"],
        vec![p],
        vec![],
        vec![],
        None,
    )
}

fn run(b: Build, ticks: usize) -> (Runtime, Rec) {
    let mut rt = Runtime::new(b.done());
    let mut host = Rec::new();
    for _ in 0..ticks {
        rt.tick(&mut host);
    }
    (rt, host)
}

fn obj(rt: &Runtime, o: Id) -> Id {
    rt.values[o].as_obj().expect("an object was made")
}

#[test]
fn object_create_makes_a_group_an_array_an_entity_and_an_object() {
    let mut b = Build::new();
    let (g, go) = create(&mut b, 23, "Trafos", true);
    let (a, ao) = create(&mut b, 52, "Cleaner", true);
    let (e, eo) = create(&mut b, 41, "Frame", true);
    let (s, so) = create(&mut b, 37, "Sprite", true);
    let (m, mo) = create(&mut b, 30, "Material", true);
    chain(&mut b, &[g, a, e, s, m]);
    let (mut rt, host) = run(b, 2);
    let (gid, aid, eid, sid, mid) = (
        obj(&rt, go),
        obj(&rt, ao),
        obj(&rt, eo),
        obj(&rt, so),
        obj(&rt, mo),
    );
    assert_eq!(rt.graph.objects[gid].kind, ObjKind::Group(1), "after the level's group 0");
    assert_eq!(rt.graph.objects[gid].name, "Trafos");
    assert_eq!(rt.graph.objects[gid].class, 23);
    assert!(rt.group_ids(1, &host).is_empty(), "a new group is empty");
    assert_eq!(rt.graph.objects[aid].kind, ObjKind::Array(0));
    assert!(rt.array(0).is_some_and(|a| a.rows.is_empty() && a.columns.is_empty()));
    assert_eq!(rt.graph.objects[eid].kind, ObjKind::Entity(2), "a row of the host's table");
    assert_eq!(rt.graph.objects[sid].kind, ObjKind::Entity(3), "a 3D sprite is an entity");
    assert_eq!(rt.graph.objects[mid].kind, ObjKind::Other);
    assert_eq!(host.matrices.len(), 4, "two rows were made in the host");
}

#[test]
fn object_create_refuses_a_level_and_stops_at_the_cap() {
    let mut b = Build::new();
    let (lvl, lo) = create(&mut b, 21, "Level", true);
    chain(&mut b, &[lvl]);
    let (rt, host) = run(b, 2);
    assert_eq!(rt.values[lo], Value::None, "no level is made");
    assert_eq!(host.matrices.len(), 2);

    let mut b = Build::new();
    let (c, _) = create(&mut b, 41, "X", true);
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, c, 0), 0);
    let l2 = b.link(out_of(&b.g, c, 0), in_of(&b.g, c, 0), 0);
    b.script("Maker", None, start, vec![c], vec![l1, l2]);
    let (_, host) = run(b, 3);
    assert_eq!(host.matrices.len(), 2 + crate::exec::MAX_COPIES);
    assert_eq!(
        host.notes.iter().filter(|n| n.contains("too many")).count(),
        1,
        "said once"
    );
}

#[test]
fn a_made_group_takes_members_and_object_delete_empties_it() {
    let mut b = Build::new();
    let (c, go) = create(&mut b, 23, "G2", true);
    let gp = b.pin("Group", types::GROUP, go);
    let t = b.local("Target", types::BEOBJECT, Value::Obj(Some(1)));
    let add = b.bb(
        "Add To Group",
        proto::ADD_TO_GROUP,
        &["In"],
        &["Out"],
        vec![gp],
        vec![],
        vec![],
        Some(t),
    );
    chain(&mut b, &[c, add]);
    let (mut rt, host) = run(b, 2);
    let gid = obj(&rt, go);
    let gi = match rt.graph.objects[gid].kind {
        ObjKind::Group(gi) => gi,
        k => panic!("{k:?}"),
    };
    assert_eq!(rt.group_ids(gi, &host), vec![1]);

    // Object Delete of the made group
    let mut b = Build::new();
    let (c, go) = create(&mut b, 23, "G2", true);
    let del = object_delete(&mut b, go);
    chain(&mut b, &[c, del]);
    let (rt, host) = run(b, 2);
    let gid = obj(&rt, go);
    let ObjKind::Group(gi) = rt.graph.objects[gid].kind else {
        panic!()
    };
    let mut rt = rt;
    assert!(rt.group_ids(gi, &host).is_empty());
}

#[test]
fn delete_dynamic_objects_deletes_the_dynamic_ones_only() {
    let mut b = Build::new();
    let (d, _) = create(&mut b, 41, "Dyn", true);
    let (k, _) = create(&mut b, 41, "Keep", false);
    let (ga, gao) = create(&mut b, 52, "Arr", true);
    let dd = delete_dynamic(&mut b);
    chain(&mut b, &[d, k, ga, dd]);
    let (rt, host) = run(b, 2);
    assert_eq!(host.deleted, vec![Ent::Object(2)], "only the dynamic entity is deleted");
    assert!(host.notes.iter().any(|n| n.contains("create Dyn")));
    let aid = obj(&rt, gao);
    assert!(!rt.awake(Some(aid)), "the made array is gone");
}

#[test]
fn object_delete_of_a_made_entity_deletes_its_row() {
    let mut b = Build::new();
    let (c, eo) = create(&mut b, 33, "Frame", true);
    let del = object_delete(&mut b, eo);
    chain(&mut b, &[c, del]);
    let (_, host) = run(b, 2);
    assert_eq!(host.deleted, vec![Ent::Object(2)]);
}

/// A Set Parent block on target `t` with the Parent pin from `parent`.
fn set_parent(b: &mut Build, t: Id, parent: Id) -> Id {
    let pp = b.pin("Parent", types::ENTITY, parent);
    let tp = b.pin("Target", types::ENTITY, t);
    b.bb(
        "Set Parent",
        proto::SET_PARENT,
        &["In"],
        &["Out"],
        vec![pp],
        vec![],
        vec![],
        Some(tp),
    )
}

#[test]
fn set_parent_links_unparents_and_refuses_itself() {
    let mut b = Build::new();
    let tb = b.local("Target", types::ENTITY, Value::Obj(Some(1)));
    let pa = b.local("Parent", types::ENTITY, Value::Obj(Some(0)));
    let none = b.local("Parent", types::ENTITY, Value::Obj(None));
    let me = b.local("Parent", types::ENTITY, Value::Obj(Some(1)));
    let s1 = set_parent(&mut b, tb, pa);
    let s2 = set_parent(&mut b, tb, none);
    let s3 = set_parent(&mut b, tb, me);
    let hide = {
        let hier = b.local("Hierarchy", types::BOOL, Value::Bool(false));
        b.bb("Hide", proto::HIDE, &["In"], &["Out"], vec![hier], vec![], vec![], Some(tb))
    };
    chain(&mut b, &[s1, s2, s3, hide]);
    let (_, host) = run(b, 2);
    assert_eq!(
        host.parents,
        vec![
            (Ent::Object(1), Some(Ent::Object(0))),
            (Ent::Object(1), None),
        ],
        "itself is not asked of the table"
    );
    assert_eq!(host.visible, vec![true, false], "Out came after the last one too");
}

#[test]
fn set_parent_without_an_entity_target_does_nothing() {
    let mut b = Build::new();
    let tb = b.local("Target", types::ENTITY, Value::Obj(None));
    let pa = b.local("Parent", types::ENTITY, Value::Obj(Some(0)));
    let s1 = set_parent(&mut b, tb, pa);
    chain(&mut b, &[s1]);
    let (_, host) = run(b, 2);
    assert!(host.parents.is_empty());
}

/// A script on A (object 0) that hides its owner when it starts: the template Attach Script copies.
fn template(b: &mut Build) -> Id {
    let this = b.local("This", types::BEOBJECT, Value::Owner);
    let target = b.pin("Target", types::ENTITY, this);
    let hide = b.bb("Hide", proto::HIDE, &["In"], &["Out"], vec![], vec![], vec![], Some(target));
    let start = b.io("Start");
    let l = b.link(start, in_of(&b.g, hide, 0), 0);
    b.script("Template", Some(0), start, vec![hide], vec![l])
}

/// An Attach Script block: the script, on the target object, with the three flags; returns (block, Created Script).
fn attach(b: &mut Build, script: Id, target: Id, dynamic: bool, activate: bool, reset: bool) -> (Id, Id) {
    let mk = |b: &mut Build, n: &str, v: bool| {
        let l = b.local(n, types::BOOL, Value::Bool(v));
        b.pin(n, types::BOOL, l)
    };
    let d = mk(b, "Dynamic?", dynamic);
    let sl = b.local("Script", types::SCRIPT, Value::Obj(Some(script)));
    let sp = b.pin("Script", types::SCRIPT, sl);
    let a = mk(b, "Activate?", activate);
    let r = mk(b, "Reset?", reset);
    let o = b.pout("Created Script", types::SCRIPT);
    let tp = b.local("Target", types::ENTITY, Value::Obj(Some(target)));
    let blk = b.bb(
        "Attach Script",
        proto::ATTACH_SCRIPT,
        &["In"],
        &["Out"],
        vec![d, sp, a, r],
        vec![o],
        vec![],
        Some(tp),
    );
    (blk, o)
}

#[test]
fn attach_script_copies_the_script_to_the_target_and_starts_it() {
    let mut b = Build::new();
    let t = template(&mut b);
    // the template is on A, but not started: the attaching script puts a copy on B
    let (at, out) = attach(&mut b, t, 1, false, true, true);
    chain(&mut b, &[at]);
    let g = b.done();
    let nodes = g.nodes.len();
    let mut rt = Runtime::new(g.clone());
    let mut host = Rec::new();
    rt.tick(&mut host);
    let created = obj(&rt, out);
    assert_ne!(created, t);
    assert_eq!(rt.graph.block(created).unwrap().owner, Some(1), "bound to B");
    assert_eq!(g.nodes.len(), nodes, "the level's graph is untouched");
    assert_eq!(host.visible, vec![false, true], "A's own template ran; B not yet (starts next tick)");
    rt.tick(&mut host);
    assert_eq!(host.visible, vec![false, false], "the copy hid its owner B");
}

#[test]
fn attach_script_without_activation_waits_and_is_capped() {
    let mut b = Build::new();
    let t = template(&mut b);
    let (at, out) = attach(&mut b, t, 1, false, false, true);
    chain(&mut b, &[at]);
    let (mut rt, mut host) = run(b, 3);
    let created = obj(&rt, out);
    assert!(!rt.is_active(created));
    assert_eq!(host.visible, vec![false, true], "an inactive copy does nothing");
    rt.activate(created, true, true, 0);
    rt.tick(&mut host);
    assert_eq!(host.visible, vec![false, false]);

    // in a loop the attaches stop at the cap, and say so once
    let mut b = Build::new();
    let t = template(&mut b);
    let (at, _) = attach(&mut b, t, 1, false, false, true);
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, at, 0), 0);
    let l2 = b.link(out_of(&b.g, at, 0), in_of(&b.g, at, 0), 0);
    b.script("Maker", None, start, vec![at], vec![l1, l2]);
    let (rt, host) = run(b, 3);
    assert_eq!(
        rt.graph.scripts.len(),
        2 + crate::exec::MAX_ATTACHED,
        "the template, the maker and the cap"
    );
    assert_eq!(host.notes.iter().filter(|n| n.contains("too many scripts")).count(), 1);
}

#[test]
fn a_dynamic_attached_script_goes_with_delete_dynamic_objects() {
    let mut b = Build::new();
    let t = template(&mut b);
    let (at, out) = attach(&mut b, t, 1, true, true, true);
    let dd = delete_dynamic(&mut b);
    chain(&mut b, &[at, dd]);
    let (rt, host) = run(b, 3);
    let created = obj(&rt, out);
    assert!(!rt.is_active(created), "deleted before it ran");
    assert_eq!(host.visible, vec![false, true], "the copy never hid B");
}

#[test]
fn set_ic_saves_the_state_and_restore_ic_puts_it_back() {
    let mut b = Build::new();
    let tb = b.local("Target", types::ENTITY, Value::Obj(Some(1)));
    let set = b.bb("Set IC", proto::SET_IC, &["In"], &["Out"], vec![], vec![], vec![], Some(tb));
    let restore = b.bb("Restore IC", proto::RESTORE_IC, &["In"], &["Out"], vec![], vec![], vec![], Some(tb));
    chain(&mut b, &[set, restore]);
    let mut rt = Runtime::new(b.done());
    let mut host = Rec::new();
    host.ic_shown = Some(true);
    host.visible[1] = false;
    rt.tick(&mut host);
    assert_eq!(host.ics, vec![Ent::Object(1)]);
    assert_eq!(host.visible[1], true, "the saved visibility comes back");
    assert_eq!(host.matrices[1][3], [10.0, 0.0, 0.0, 1.0]);
}

/// A Set Physics Slider block.
fn slider(b: &mut Build, target: Option<usize>, object2: Option<usize>, points: [Option<usize>; 2], limits: Option<(f32, f32)>) -> Id {
    let ent = |b: &mut Build, n: &str, v: Option<usize>| {
        let l = b.local(n, types::ENTITY, Value::Obj(v));
        b.pin(n, types::ENTITY, l)
    };
    let o2 = ent(b, "Object2", object2);
    let p0 = ent(b, "Axis first Point", points[0]);
    let p1 = ent(b, "Axis second Point", points[1]);
    let lim = b.local("Limitations (meter)", types::BOOL, Value::Bool(limits.is_some()));
    let limp = b.pin("Limitations (meter)", types::BOOL, lim);
    let (lo, hi) = limits.unwrap_or((-1.0, 1.0));
    let lol = b.local("Lower Limit", types::FLOAT, Value::Float(lo));
    let lop = b.pin("Lower Limit", types::FLOAT, lol);
    let hil = b.local("Upper Limit", types::FLOAT, Value::Float(hi));
    let hip = b.pin("Upper Limit", types::FLOAT, hil);
    let tl = b.local("Target", types::ENTITY, Value::Obj(target));
    b.bb(
        "Set Physics Slider",
        proto::SET_PHYSICS_SLIDER,
        &["Create", "Destroy"],
        &["Out1", "Out2"],
        vec![o2, p0, p1, limp, lop, hip],
        vec![],
        vec![],
        Some(tl),
    )
}

#[test]
fn set_physics_slider_creates_with_the_axis_and_limits_and_destroys() {
    let mut b = Build::new();
    let s = slider(&mut b, Some(0), Some(1), [Some(0), Some(1)], Some((-4.5, 4.5)));
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, s, 0), 0);
    let l2 = b.link(start, in_of(&b.g, s, 1), 2);
    let script = b.script("Create", None, start, vec![s], vec![l1, l2]);
    let mut rt = Runtime::new(b.done());
    let mut host = Rec::new();
    rt.tick(&mut host);
    assert_eq!(host.sliders.len(), 1);
    let (e, e2, key, spec) = host.sliders[0].clone();
    assert_eq!((e, e2), (Ent::Object(0), Ent::Object(1)));
    let spec = spec.expect("created");
    assert_eq!(spec.p0, [0.0, 0.0, 0.0]);
    assert_eq!(spec.p1, [10.0, 0.0, 0.0]);
    assert_eq!(spec.limits, Some((-4.5, 4.5)));
    // Destroy input two ticks later: the same key
    let sb = rt.graph.block(script).unwrap().sub[0];
    assert_eq!(key, sb);
    rt.tick(&mut host);
    rt.tick(&mut host);
    assert_eq!(host.sliders.last().unwrap().3, None, "destroyed");
    assert_eq!(host.sliders.last().unwrap().2, key);
}

#[test]
fn set_physics_slider_without_an_object2_or_a_point_makes_none() {
    for (o2, pts) in [
        (None, [Some(0), Some(1)]),
        (Some(1), [None, Some(1)]),
        (Some(1), [Some(0), None]),
    ] {
        let mut b = Build::new();
        let s = slider(&mut b, Some(0), o2, pts, None);
        chain(&mut b, &[s]);
        let (_, host) = run(b, 2);
        assert!(host.sliders.is_empty(), "{o2:?} {pts:?}");
    }
    let mut b = Build::new();
    let s = slider(&mut b, None, Some(1), [Some(0), Some(1)], None);
    chain(&mut b, &[s]);
    // no target: the owner (none here) is no entity
    let (_, host) = run(b, 2);
    assert!(host.sliders.is_empty());
}

/// An array "T" of an integer column with `values`, and an Iterator If block over it; the Object column is not
/// there: the cells go to the Value output.
fn if_array(b: &mut Build, values: &[i32]) -> (Id, Id, Id) {
    b.g.arrays.push(crate::Array {
        name: "T".into(),
        columns: vec![("Value".into(), 1)],
        rows: values.iter().map(|&v| vec![Value::Int(v)]).collect(),
        key: None,
    });
    let arr = b.g.add_object("T", ObjKind::Array(0));
    let ap = b.local("Target (Array)", types::ARRAY, Value::Obj(Some(arr)));
    let col = b.local("Column", types::INT, Value::Int(0));
    let colp = b.pin("Column", types::INT, col);
    let op = b.local("Operator", types::INT, Value::Int(5));
    let opp = b.pin("Operator", types::INT, op);
    let rf = b.local("Reference Value", types::INT, Value::Int(3));
    let rfp = b.pin("Reference Value", types::INT, rf);
    let index = b.pout("Index", types::INT);
    let val = b.pout("Value", types::INT);
    let it = b.bb(
        "Iterator If",
        proto::ITERATOR_IF,
        &["In", "Loop In"],
        &["Out", "Loop Out"],
        vec![colp, opp, rfp],
        vec![index, val],
        vec![],
        Some(ap),
    );
    (it, index, val)
}

#[test]
fn iterator_if_walks_the_rows_that_satisfy_the_test() {
    let mut b = Build::new();
    let (it, index, val) = if_array(&mut b, &[1, 5, 2, 7, 3, 9]);
    // each loop turn hides nobody, it only comes back: count the turns with the host's notes via Hide? keep to values
    let start = b.io("Start");
    let l1 = b.link(start, in_of(&b.g, it, 0), 0);
    let l2 = b.link(out_of(&b.g, it, 1), in_of(&b.g, it, 1), 1);
    let script = b.script("S", None, start, vec![it], vec![l1, l2]);
    let mut rt = Runtime::new(b.done());
    let mut host = Rec::new();
    let mut seen = Vec::new();
    for _ in 0..8 {
        rt.tick(&mut host);
        if rt.values[index] != Value::None {
            let pair = (rt.values[index].as_int(), rt.values[val].as_int());
            if seen.last() != Some(&pair) {
                seen.push(pair);
            }
        }
    }
    assert_eq!(seen, vec![(1, 5), (3, 7), (5, 9)], "rows with a value above 3, in order");
    assert!(!rt.is_active(script), "Out after the last one");
}

#[test]
fn iterator_if_with_no_match_or_no_rows_goes_out_at_once() {
    for values in [vec![], vec![1, 2, 3]] {
        let mut b = Build::new();
        let (it, index, _) = if_array(&mut b, &values);
        let start = b.io("Start");
        let l1 = b.link(start, in_of(&b.g, it, 0), 0);
        let after = {
            let hier = b.local("Hierarchy", types::BOOL, Value::Bool(false));
            let t = b.local("Target", types::ENTITY, Value::Obj(Some(1)));
            b.bb("Hide", proto::HIDE, &["In"], &["Out"], vec![hier], vec![], vec![], Some(t))
        };
        let l2 = b.link(out_of(&b.g, it, 0), in_of(&b.g, after, 0), 0);
        b.script("S", None, start, vec![it, after], vec![l1, l2]);
        let (rt, host) = run(b, 2);
        assert_eq!(host.visible, vec![true, false], "{values:?}");
        assert_eq!(rt.values[index], Value::None);
    }
}

fn shuffle_run(n: i32) -> Vec<i32> {
    let mut b = Build::new();
    b.g.arrays.push(crate::Array {
        name: "T".into(),
        columns: vec![("Value".into(), 1)],
        rows: (0..n).map(|v| vec![Value::Int(v)]).collect(),
        key: None,
    });
    let arr = b.g.add_object("T", ObjKind::Array(0));
    let ap = b.local("Target (Array)", types::ARRAY, Value::Obj(Some(arr)));
    let sh = b.bb("Shuffle Order", proto::SHUFFLE_ORDER, &["In"], &["Out"], vec![], vec![], vec![], Some(ap));
    chain(&mut b, &[sh]);
    let (rt, _) = run(b, 2);
    rt.rows(0).iter().map(|r| r[0].as_int()).collect()
}

#[test]
fn shuffle_order_permutes_the_rows_the_same_way_every_run() {
    let a = shuffle_run(12);
    let mut sorted = a.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..12).collect::<Vec<_>>(), "no row lost or doubled");
    assert_ne!(a, (0..12).collect::<Vec<_>>(), "the order changed");
    assert_eq!(a, shuffle_run(12), "the run's own generator: the same each time");
    assert_eq!(shuffle_run(1), vec![0]);
    assert!(shuffle_run(0).is_empty());
}
