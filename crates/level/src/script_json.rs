// A map's behaviour graphs (the `script` crate's `Graph`) as the open format's JSON, so a converted level keeps its
// scripts: `scenes[0].extras.drahtseilakt.scripts`. Only the primary data is written (objects, nodes, arrays, the
// top-level graphs with their owners); the back references are derived again on reading (`Graph::assemble`), which
// also checks every index. The file is a stranger's: every shape is checked, a malformed one refuses the level.

use script::{Array, Block, Graph, Guid, Io, Link, Node, ObjInfo, ObjKind, Op, Param, Value};
use serde_json::{Value as J, json};

fn guid(g: Guid) -> J {
    json!([g.0, g.1])
}

fn guid_of(v: &J) -> Result<Guid, String> {
    let a = v.as_array().ok_or("script guid is not a pair")?;
    let w = |i: usize| {
        a.get(i)
            .and_then(J::as_u64)
            .and_then(|x| u32::try_from(x).ok())
            .ok_or("script guid word is not a number")
    };
    Ok(Guid(w(0)?, w(1)?))
}

fn num(x: f32) -> J {
    json!(if x.is_finite() { x } else { 0.0 })
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return Err("script bytes are not hex".into());
    }
    s.as_bytes()
        .chunks(2)
        .map(|p| {
            std::str::from_utf8(p)
                .ok()
                .and_then(|p| u8::from_str_radix(p, 16).ok())
                .ok_or_else(|| "script bytes are not hex".to_owned())
        })
        .collect()
}

fn value(v: &Value) -> J {
    match v {
        Value::None => J::Null,
        Value::Int(i) => json!(["i", i]),
        Value::Float(f) => json!(["f", num(*f)]),
        Value::Bool(b) => json!(["b", b]),
        Value::Str(s) => json!(["s", s]),
        Value::Obj(o) => json!(["o", o]),
        Value::Vec3(a) => json!(["v3", a.map(num)]),
        Value::Vec2(a) => json!(["v2", a.map(num)]),
        Value::Color(a) => json!(["c", a.map(num)]),
        Value::Matrix(m) => json!(["m", m.iter().flatten().map(|x| num(*x)).collect::<Vec<_>>()]),
        Value::Quat(a) => json!(["q", a.map(num)]),
        Value::Rect(a) => json!(["rect", a.map(num)]),
        Value::Box(a) => json!(["box", a.map(num)]),
        Value::Objects(v) => json!(["oa", v]),
        Value::Raw(b) => json!(["r", hex(b)]),
        Value::Curve(c) => json!([
            "curve",
            c.points
                .iter()
                .map(|p| json!([
                    p.flags,
                    num(p.pos[0]),
                    num(p.pos[1]),
                    num(p.tension),
                    num(p.continuity),
                    num(p.bias),
                    num(p.tan_in[0]),
                    num(p.tan_in[1]),
                    num(p.tan_out[0]),
                    num(p.tan_out[1]),
                ]))
                .collect::<Vec<_>>()
        ]),
        Value::Owner => json!(["this", 0]),
    }
}

fn floats<const N: usize>(v: &J) -> Result<[f32; N], String> {
    let a = v
        .as_array()
        .filter(|a| a.len() == N)
        .ok_or("script vector has the wrong length")?;
    let mut out = [0.0f32; N];
    for (o, x) in out.iter_mut().zip(a) {
        *o = x.as_f64().ok_or("script number expected")? as f32;
    }
    Ok(out)
}

fn index(v: &J) -> Result<usize, String> {
    v.as_u64()
        .and_then(|x| usize::try_from(x).ok())
        .ok_or_else(|| "script index expected".to_owned())
}

fn opt_index(v: &J) -> Result<Option<usize>, String> {
    if v.is_null() {
        Ok(None)
    } else {
        index(v).map(Some)
    }
}

fn indices(v: &J) -> Result<Vec<usize>, String> {
    v.as_array()
        .ok_or("script index list expected")?
        .iter()
        .map(index)
        .collect()
}

fn value_of(j: &J) -> Result<Value, String> {
    if j.is_null() {
        return Ok(Value::None);
    }
    let a = j
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or("script value is malformed")?;
    let tag = a.first().and_then(J::as_str).unwrap_or_default();
    let x = a.get(1).ok_or("script value is malformed")?;
    Ok(match tag {
        "i" => Value::Int(
            i32::try_from(x.as_i64().ok_or("script int expected")?).map_err(|e| e.to_string())?,
        ),
        "f" => Value::Float(x.as_f64().ok_or("script float expected")? as f32),
        "b" => Value::Bool(x.as_bool().ok_or("script bool expected")?),
        "s" => Value::Str(x.as_str().ok_or("script string expected")?.to_owned()),
        "o" => Value::Obj(opt_index(x)?),
        "v3" => Value::Vec3(floats(x)?),
        "v2" => Value::Vec2(floats(x)?),
        "c" => Value::Color(floats(x)?),
        "m" => {
            let f = floats::<16>(x)?;
            Value::Matrix(std::array::from_fn(|r| {
                std::array::from_fn(|c| {
                    f.get(r.saturating_mul(4).saturating_add(c))
                        .copied()
                        .unwrap_or(0.0)
                })
            }))
        }
        "q" => Value::Quat(floats(x)?),
        "rect" => Value::Rect(floats(x)?),
        "box" => Value::Box(floats(x)?),
        "oa" => Value::Objects(
            x.as_array()
                .ok_or("script object array expected")?
                .iter()
                .take(script::MAX_ARRAY_ROWS)
                .map(opt_index)
                .collect::<Result<_, _>>()?,
        ),
        "this" => Value::Owner,
        "curve" => {
            let pts = x.as_array().ok_or("script curve expected")?;
            if pts.len() > nmo::curve2d::MAX_POINTS {
                return Err("script curve has too many points".into());
            }
            let mut points = Vec::new();
            for p in pts {
                let f = floats::<9>(&json!(
                    p.as_array().and_then(|a| a.get(1..)).unwrap_or_default()
                ))?;
                let flags = p
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(J::as_u64)
                    .and_then(|x| u32::try_from(x).ok())
                    .ok_or("script curve flags expected")?;
                points.push(nmo::curve2d::Point {
                    flags,
                    pos: [f[0], f[1]],
                    tension: f[2],
                    continuity: f[3],
                    bias: f[4],
                    tan_in: [f[5], f[6]],
                    tan_out: [f[7], f[8]],
                });
            }
            Value::Curve(std::sync::Arc::new(nmo::curve2d::Curve2d::new(points)))
        }
        "r" => Value::Raw(unhex(x.as_str().ok_or("script bytes expected")?)?),
        _ => return Err("unknown script value".into()),
    })
}

fn kind(
    k: ObjKind,
    mesh: &dyn Fn(usize) -> Option<usize>,
    tex: &dyn Fn(usize) -> Option<usize>,
) -> J {
    match k {
        ObjKind::Entity(i) => json!(["entity", i]),
        ObjKind::Group(i) => json!(["group", i]),
        ObjKind::Mesh(i) => mesh(i).map_or(json!(["other", 0]), |m| json!(["mesh", m])),
        ObjKind::Material(i) => json!(["material", i]),
        ObjKind::Texture(i) => tex(i).map_or(json!(["other", 0]), |t| json!(["texture", t])),
        ObjKind::Array(i) => json!(["array", i]),
        ObjKind::Behavior => json!(["behavior", 0]),
        ObjKind::Ball => json!(["ball", 0]),
        ObjKind::Other => json!(["other", 0]),
    }
}

/// The graphs as JSON. `mesh` and `tex` say where the level's mesh and texture of an index went in the file (a
/// mesh without geometry, a texture dropped: nowhere, and the object is then nothing the engine knows).
pub fn to_json(
    g: &Graph,
    mesh: &dyn Fn(usize) -> Option<usize>,
    tex: &dyn Fn(usize) -> Option<usize>,
) -> J {
    let objects: Vec<J> = g
        .objects
        .iter()
        .map(|o| json!([o.name, o.class, kind(o.kind, mesh, tex)]))
        .collect();
    let nodes: Vec<J> = g
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            let body = match n {
                Node::None => return None,
                Node::Block(b) => json!({"t": "block", "name": b.name, "proto": b.proto.map(guid),
                    "flags": b.flags, "priority": b.priority, "target": b.target,
                    "inputs": b.inputs, "outputs": b.outputs, "pins": b.pins, "pouts": b.pouts,
                    "locals": b.locals, "sub": b.sub, "links": b.links, "ops": b.ops}),
                Node::Link(l) => json!({"t": "link", "delay": l.delay, "src": l.src, "dst": l.dst}),
                Node::Io(io) => json!({"t": "io", "input": io.is_input}),
                Node::Param(p) => json!({"t": "param", "name": p.name, "guid": guid(p.guid),
                    "value": value(&p.value), "source": p.source, "shared": p.shared,
                    "dest": p.destinations}),
                Node::Op(o) => {
                    json!({"t": "op", "guid": guid(o.guid), "in": o.inputs, "out": o.output})
                }
            };
            Some(json!([i, body]))
        })
        .collect();
    let arrays: Vec<J> = g
        .arrays
        .iter()
        .map(|a| {
            json!({"name": a.name, "columns": a.columns, "key": a.key,
                "rows": a.rows.iter().map(|r| r.iter().map(value).collect::<Vec<_>>()).collect::<Vec<_>>()})
        })
        .collect();
    let scripts: Vec<J> = g
        .scripts
        .iter()
        .map(|&s| {
            let owner = match g.nodes.get(s) {
                Some(Node::Block(b)) => b.owner,
                _ => None,
            };
            json!([s, owner])
        })
        .collect();
    let waves: Vec<J> = g
        .waves
        .iter()
        .map(|w| {
            json!({"object": w.object, "file": w.file, "looped": w.looped, "gain": num(w.gain), "pan": num(w.pan),
                "pitch": num(w.pitch)})
        })
        .collect();
    json!({"objects": objects, "nodes": nodes, "arrays": arrays, "scripts": scripts, "groups": g.groups,
        "attributes": g.attributes, "priorities": g.priorities, "start_off": g.start_off, "waves": waves})
}

/// The most objects and arrays a level's scripts may have (a map's own are a few thousand).
const MAX_OBJECTS: usize = 1 << 18;
const MAX_ARRAYS: usize = 1 << 12;
/// The longest file name a wave sound keeps (a path as saved; only its last part is used).
const MAX_WAVE_NAME: usize = 4096;

/// The graphs from JSON. `counts` are how many entities, groups, meshes, materials and textures the level has: an
/// object naming one beyond them is nothing the engine knows.
pub fn from_json(j: &J, counts: [usize; 5]) -> Result<Graph, String> {
    let too_many = || "the scripts hold more than a level's can".to_owned();
    let list = |k: &str| -> Result<&Vec<J>, String> {
        j.get(k)
            .and_then(J::as_array)
            .ok_or_else(|| format!("scripts: `{k}` expected"))
    };
    // the counts are checked before anything is built from the lists (a stranger's file may claim any)
    let (objs, node_list, array_list) = (list("objects")?, list("nodes")?, list("arrays")?);
    if objs.len() > MAX_OBJECTS || node_list.len() > objs.len() || array_list.len() > MAX_ARRAYS {
        return Err(too_many());
    }
    let mut objects = Vec::new();
    for o in objs {
        let a = o
            .as_array()
            .filter(|a| a.len() == 3)
            .ok_or("script object is malformed")?;
        let k = a
            .get(2)
            .and_then(J::as_array)
            .filter(|k| k.len() == 2)
            .ok_or("script kind is malformed")?;
        let i = index(k.get(1).ok_or("script kind is malformed")?)?;
        let within =
            |c: usize, make: fn(usize) -> ObjKind| if i < c { make(i) } else { ObjKind::Other };
        let kind = match k.first().and_then(J::as_str).unwrap_or_default() {
            "entity" => within(counts[0], ObjKind::Entity),
            "group" => within(counts[1], ObjKind::Group),
            "mesh" => within(counts[2], ObjKind::Mesh),
            "material" => within(counts[3], ObjKind::Material),
            "texture" => within(counts[4], ObjKind::Texture),
            "array" => ObjKind::Array(i),
            "behavior" => ObjKind::Behavior,
            "ball" => ObjKind::Ball,
            _ => ObjKind::Other,
        };
        objects.push(ObjInfo {
            name: a.first().and_then(J::as_str).unwrap_or_default().to_owned(),
            class: u32::try_from(index(a.get(1).ok_or("script object is malformed")?)?)
                .map_err(|e| e.to_string())?,
            kind,
        });
    }
    let mut nodes = Vec::new();
    let mut taken = vec![false; objects.len()];
    for n in node_list {
        let a = n
            .as_array()
            .filter(|a| a.len() == 2)
            .ok_or("script node is malformed")?;
        let id = index(a.first().ok_or("script node is malformed")?)?;
        match taken.get_mut(id) {
            Some(t) if !*t => *t = true,
            _ => return Err("a script object has two nodes, or none".into()),
        }
        let b = a.get(1).ok_or("script node is malformed")?;
        let f = |k: &str| b.get(k).unwrap_or(&J::Null);
        let node = match f("t").as_str().unwrap_or_default() {
            "block" => Node::Block(Block {
                name: f("name").as_str().unwrap_or_default().to_owned(),
                proto: if f("proto").is_null() {
                    None
                } else {
                    Some(guid_of(f("proto"))?)
                },
                flags: u32::try_from(f("flags").as_u64().unwrap_or(0)).unwrap_or(0),
                priority: i32::try_from(f("priority").as_i64().unwrap_or(0)).unwrap_or(0),
                target: opt_index(f("target"))?,
                inputs: indices(f("inputs"))?,
                outputs: indices(f("outputs"))?,
                pins: indices(f("pins"))?,
                pouts: indices(f("pouts"))?,
                locals: indices(f("locals"))?,
                sub: indices(f("sub"))?,
                links: indices(f("links"))?,
                ops: indices(f("ops"))?,
                parent: None,
                owner: None,
            }),
            "link" => Node::Link(Link {
                delay: u16::try_from(f("delay").as_u64().unwrap_or(0)).unwrap_or(0),
                src: index(f("src"))?,
                dst: index(f("dst"))?,
            }),
            "io" => Node::Io(Io {
                owner: None,
                is_input: f("input").as_bool().unwrap_or(false),
            }),
            "param" => Node::Param(Param {
                name: f("name").as_str().unwrap_or_default().to_owned(),
                guid: guid_of(f("guid"))?,
                value: value_of(f("value"))?,
                source: opt_index(f("source"))?,
                shared: f("shared").as_bool().unwrap_or(false),
                destinations: indices(f("dest"))?,
                op: None,
                block: None,
            }),
            "op" => {
                let i = f("in")
                    .as_array()
                    .filter(|a| a.len() == 2)
                    .ok_or("script operation is malformed")?;
                Node::Op(Op {
                    guid: guid_of(f("guid"))?,
                    inputs: [
                        opt_index(i.first().unwrap_or(&J::Null))?,
                        opt_index(i.get(1).unwrap_or(&J::Null))?,
                    ],
                    output: opt_index(f("out"))?,
                })
            }
            _ => return Err("unknown script node".into()),
        };
        nodes.push((id, node));
    }
    let mut arrays = Vec::new();
    for a in array_list {
        let columns = a
            .get("columns")
            .and_then(J::as_array)
            .ok_or("script array without columns")?
            .iter()
            .map(|c| {
                let p = c
                    .as_array()
                    .filter(|p| p.len() == 2)
                    .ok_or("script column is malformed")?;
                Ok((
                    p.first().and_then(J::as_str).unwrap_or_default().to_owned(),
                    u32::try_from(index(p.get(1).ok_or("script column is malformed")?)?)
                        .map_err(|e| e.to_string())?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let rows = a
            .get("rows")
            .and_then(J::as_array)
            .ok_or("script array without rows")?
            .iter()
            .map(|r| {
                r.as_array()
                    .ok_or_else(|| "script row is malformed".to_owned())?
                    .iter()
                    .map(value_of)
                    .collect::<Result<Vec<_>, String>>()
            })
            .collect::<Result<Vec<_>, String>>()?;
        arrays.push(Array {
            name: a
                .get("name")
                .and_then(J::as_str)
                .unwrap_or_default()
                .to_owned(),
            // absent or null in files written before the key column was kept: none
            key: match a.get("key") {
                None | Some(J::Null) => None,
                Some(k) => Some(index(k)?),
            },
            columns,
            rows,
        });
    }
    let mut scripts = Vec::new();
    for s in list("scripts")? {
        let p = s
            .as_array()
            .filter(|p| p.len() == 2)
            .ok_or("script entry is malformed")?;
        scripts.push((
            index(p.first().ok_or("script entry is malformed")?)?,
            opt_index(p.get(1).unwrap_or(&J::Null))?,
        ));
    }
    // the members the file gave each group, of any kind (absent in files written before: the level's own are used)
    let mut groups = Vec::new();
    if let Some(list) = j.get("groups").and_then(J::as_array) {
        if list.len() > MAX_OBJECTS {
            return Err(too_many());
        }
        for m in list {
            let ids = indices(m)?;
            if ids.len() > MAX_OBJECTS || ids.iter().any(|&i| i >= objs.len()) {
                return Err("a script group names an object that does not exist".into());
            }
            groups.push(ids);
        }
    }
    // the attributes the file's objects carry: (object, attribute index, parameter), absent in older files
    let mut attributes = Vec::new();
    if let Some(list) = j.get("attributes").and_then(J::as_array) {
        if list.len() > script::MAX_ATTRIBUTES {
            return Err(too_many());
        }
        for a in list {
            let t = a
                .as_array()
                .filter(|t| t.len() == 3)
                .ok_or("script attribute is malformed")?;
            let o = index(t.first().ok_or("script attribute is malformed")?)?;
            let n = i32::try_from(index(t.get(1).ok_or("script attribute is malformed")?)?)
                .map_err(|e| e.to_string())?;
            let p = opt_index(t.get(2).unwrap_or(&J::Null))?;
            if o >= objs.len()
                || n >= script::MAX_ATTRIBUTE_TYPES
                || p.is_some_and(|p| p >= objs.len())
            {
                return Err("a script attribute names an object that does not exist".into());
            }
            attributes.push((o, n, p));
        }
    }
    // the objects' priorities, the scripts and objects the scene starts deactivated and the wave sounds: absent in
    // files written before (every object then has priority 0, starts active, and no sound is known)
    let mut priorities = Vec::new();
    if let Some(list) = j.get("priorities").and_then(J::as_array) {
        if list.len() > objs.len() {
            return Err(too_many());
        }
        for p in list {
            let t = p
                .as_array()
                .filter(|t| t.len() == 2)
                .ok_or("script priority is malformed")?;
            let o = index(t.first().ok_or("script priority is malformed")?)?;
            let n = t
                .get(1)
                .and_then(J::as_i64)
                .and_then(|n| i32::try_from(n).ok())
                .ok_or("script priority is malformed")?;
            if o >= objs.len() {
                return Err("a script priority names an object that does not exist".into());
            }
            priorities.push((o, n));
        }
    }
    let start_off = match j.get("start_off") {
        Some(v) if !v.is_null() => {
            let ids = indices(v)?;
            if ids.len() > objs.len() || ids.iter().any(|&i| i >= objs.len()) {
                return Err("a script started deactivated does not exist".into());
            }
            ids
        }
        _ => Vec::new(),
    };
    let mut waves = Vec::new();
    if let Some(list) = j.get("waves").and_then(J::as_array) {
        if list.len() > script::MAX_WAVES {
            return Err(too_many());
        }
        for w in list {
            let object = index(&w["object"])?;
            if object >= objs.len() {
                return Err("a script wave sound names an object that does not exist".into());
            }
            let file = w["file"].as_str().ok_or("script wave sound is malformed")?;
            if file.len() > MAX_WAVE_NAME {
                return Err("a script wave sound's file name is too long".into());
            }
            let f = |k: &str| -> Result<f32, String> {
                Ok(w[k].as_f64().ok_or("script wave sound is malformed")? as f32)
            };
            waves.push(script::Wave {
                object,
                file: file.to_owned(),
                looped: w["looped"]
                    .as_bool()
                    .ok_or("script wave sound is malformed")?,
                gain: f("gain")?,
                pan: f("pan")?,
                pitch: f("pitch")?,
            });
        }
    }
    let mut g = Graph::assemble(objects, nodes, arrays, scripts)?;
    g.groups = groups;
    g.attributes = attributes;
    g.priorities = priorities;
    g.start_off = start_off;
    g.waves = waves;
    Ok(g)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    fn graph() -> Graph {
        let obj = |n: &str, kind| ObjInfo {
            name: n.into(),
            class: 0,
            kind,
        };
        let objects = vec![
            obj("Script", ObjKind::Behavior),
            obj("Thing", ObjKind::Entity(0)),
            obj("In", ObjKind::Other),
            obj("Param", ObjKind::Other),
        ];
        let block = Block {
            name: "Script".into(),
            proto: Some(Guid(1, 2)),
            flags: 2,
            inputs: vec![2],
            locals: vec![3],
            ..Block::default()
        };
        let param = Param {
            name: "P".into(),
            guid: Guid(3, 4),
            value: Value::Vec3([1.0, 2.5, -3.0]),
            ..Param::default()
        };
        Graph::assemble(
            objects,
            vec![(0, Node::Block(block)), (3, Node::Param(param))],
            vec![Array {
                name: "T".into(),
                columns: vec![("c".into(), 1)],
                rows: vec![
                    vec![Value::Int(7)],
                    vec![Value::Raw(vec![1, 255])],
                    vec![Value::Quat([0.0, 0.5, -1.0, 1.0])],
                    vec![Value::Rect([1.0, 2.0, 3.0, 4.0])],
                    vec![Value::Box([0.0, 1.0, 2.0, 3.0, 4.0, 5.0])],
                    vec![Value::Objects(vec![Some(0), None])],
                    vec![Value::Curve(std::sync::Arc::new(
                        nmo::curve2d::Curve2d::new(vec![
                            nmo::curve2d::Point {
                                pos: [0.0, 0.0],
                                flags: 3,
                                tan_out: [1.0, 0.5],
                                ..nmo::curve2d::Point::default()
                            },
                            nmo::curve2d::Point {
                                pos: [1.0, 1.0],
                                tension: 0.5,
                                ..nmo::curve2d::Point::default()
                            },
                        ]),
                    ))],
                ],
                key: Some(0),
            }],
            vec![(0, Some(1))],
        )
        .expect("graph")
    }

    #[test]
    fn a_graph_survives_the_open_format() {
        let mut g = graph();
        g.groups = vec![vec![0, 1], vec![]];
        g.attributes = vec![(0, 5, None), (1, 6, Some(0))];
        g.priorities = vec![(1, -3), (0, 7)];
        g.start_off = vec![0];
        g.waves = vec![script::Wave {
            object: 1,
            file: "Sounds\\Hit.wav".into(),
            looped: true,
            gain: 0.5,
            pan: -0.25,
            pitch: 1.5,
        }];
        let j = to_json(&g, &Some, &Some);
        let back = from_json(&j, [1, 0, 0, 0, 0]).expect("read");
        assert_eq!(back, g);
        // a group, priority, deactivated script or sound naming an object that is not there refuses the graph
        for (key, v) in [
            ("groups", serde_json::json!([[4000000000u64]])),
            ("priorities", serde_json::json!([[9, 1]])),
            ("priorities", serde_json::json!([[0, 1u64 << 40]])),
            ("start_off", serde_json::json!([9])),
            (
                "waves",
                serde_json::json!([{"object": 9, "file": "", "looped": false, "gain": 1, "pan": 0, "pitch": 1}]),
            ),
            ("waves", serde_json::json!([{"object": 0, "file": 3}])),
        ] {
            let mut bad = j.clone();
            bad[key] = v;
            assert!(from_json(&bad, [1, 0, 0, 0, 0]).is_err(), "{key}");
        }
        // a file written before these were kept reads as before
        let mut old = j.clone();
        for key in ["priorities", "start_off", "waves"] {
            old.as_object_mut().map(|o| o.remove(key));
        }
        let back = from_json(&old, [1, 0, 0, 0, 0]).expect("read");
        assert!(back.priorities.is_empty() && back.start_off.is_empty() && back.waves.is_empty());
    }

    #[test]
    fn an_index_outside_the_objects_refuses_the_graph() {
        let mut j = to_json(&graph(), &Some, &Some);
        j["scripts"] = serde_json::json!([[99, null]]);
        assert!(from_json(&j, [1, 0, 0, 0, 0]).is_err());
        let mut j = to_json(&graph(), &Some, &Some);
        j["nodes"][0][1]["inputs"] = serde_json::json!([4000000000u64]);
        assert!(from_json(&j, [1, 0, 0, 0, 0]).is_err());
    }

    // ---- hostile shapes: each is refused quickly, none builds a graph that stalls the runtime ----

    fn refused(edit: impl FnOnce(&mut serde_json::Value)) {
        let mut j = to_json(&graph(), &Some, &Some);
        edit(&mut j);
        assert!(from_json(&j, [1, 0, 0, 0, 0]).is_err());
    }

    #[test]
    fn a_block_that_is_its_own_sub_behaviour_is_refused() {
        refused(|j| j["nodes"][0][1]["sub"] = serde_json::json!([0, 0]));
    }

    #[test]
    fn a_block_in_two_sub_lists_is_refused() {
        refused(|j| {
            j["nodes"][0][1]["sub"] = serde_json::json!([1, 1]);
        });
    }

    #[test]
    fn a_parameter_copying_round_a_loop_or_twice_is_refused() {
        refused(|j| j["nodes"][2][1]["dest"] = serde_json::json!([3]));
        refused(|j| j["nodes"][2][1]["dest"] = serde_json::json!([0, 0]));
    }

    #[test]
    fn a_parameter_reading_itself_is_refused() {
        refused(|j| j["nodes"][2][1]["source"] = serde_json::json!(3));
    }

    #[test]
    fn more_nodes_than_objects_or_a_node_twice_is_refused() {
        refused(|j| {
            let n = j["nodes"][1].clone();
            j["nodes"] =
                serde_json::json!([j["nodes"][0].clone(), n.clone(), n.clone(), n.clone(), n]);
        });
        refused(|j| {
            let n = j["nodes"][0].clone();
            j["nodes"] = serde_json::json!([n.clone(), n]);
        });
    }

    #[test]
    fn a_script_twice_or_inside_another_graph_is_refused() {
        refused(|j| j["scripts"] = serde_json::json!([[0, 1], [0, 1]]));
        refused(|j| {
            j["nodes"][0][1]["sub"] = serde_json::json!([2]);
            j["nodes"].as_array_mut().expect("array").push(serde_json::json!([
                2,
                {"t": "block", "name": "inner", "proto": null, "flags": 0, "priority": 0, "target": null,
                 "inputs": [], "outputs": [], "pins": [], "pouts": [], "locals": [], "sub": [], "links": [], "ops": []}
            ]));
            j["scripts"] = serde_json::json!([[0, 1], [2, 1]]);
        });
    }
}
