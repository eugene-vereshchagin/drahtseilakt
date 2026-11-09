// The building blocks the runtime knows, by prototype GUID, and the parameter operations. Each block reads its
// active inputs (clearing them), its pins by name, acts through the host and activates its outputs. The list and
// each block's standing: docs/drahtseilakt/scripts.md. A block met that is not here is reported by the runtime.

use crate::exec::{Ent, Host, Runtime, Scratch};
use crate::{Block, Guid, Id, ObjKind, Value, types};

/// What a block returns: done this tick, stays active for the next, or not known to the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Done,
    Continue,
    Unknown,
}

pub mod proto {
    use super::Guid;
    pub const OP: Guid = Guid(0x2d5d_6d01, 0x6a35_3eb0);
    pub const OBJECT_DELETE: Guid = Guid(0x7412_0ded, 0x7652_4673);
    pub const CHANGE_TEXTURE_VIDEO_FORMAT: Guid = Guid(0xf56d_92ef, 0x41eb_20c1);
    pub const ACTIVATE_SCRIPT: Guid = Guid(0x4c7e_7bc3, 0x0b69_3155);
    pub const DEACTIVATE_SCRIPT: Guid = Guid(0x1436_7c05, 0x635b_24f9);
    pub const EXECUTE_SCRIPT: Guid = Guid(0x706c_5a40, 0x5bb3_1a0b);
    pub const POINT_PARTICLE_SYSTEM: Guid = Guid(0x506b_40f7, 0x3085_2e46);
    pub const PLANAR_PARTICLE_SYSTEM: Guid = Guid(0x4995_7bfe, 0x0ff2_7ffc);
    pub const TT_SCALEABLE_PROXIMITY: Guid = Guid(0x2a2a_63ca, 0x0082_6175);
    pub const PROXIMITY: Guid = Guid(0x5321_cacb, 0xdcdc_5213);
    pub const GET_CELL: Guid = Guid(0x33b9_9f51, 0x07d9_5c45);
    pub const SET_CELL: Guid = Guid(0x30ed_1c6d, 0x4a3b_7067);
    pub const ADD_ROW: Guid = Guid(0x1c7e_5dc6, 0x3f64_23c2);
    pub const GET_KEY_ROW: Guid = Guid(0x4906_4205, 0x10e7_2f7a);
    pub const GET_ROW: Guid = Guid(0x33b7_7f41, 0x07b9_5c45);
    pub const CLEAR_ARRAY: Guid = Guid(0x35c9_352f, 0x7b1a_193b);
    pub const BINARY_SWITCH: Guid = Guid(0xeb50_6901, 0x984a_fccc);
    pub const WAIT_MESSAGE: Guid = Guid(0x4587_ffee, 0x4587_ffdd);
    pub const SEND_MESSAGE_TO_GROUP: Guid = Guid(0x5f90_6952, 0x6df1_1649);
    pub const SEND_MESSAGE: Guid = Guid(0xa20e_8d5b, 0xdf00_2150);
    pub const GROUP_ITERATOR: Guid = Guid(0x6050_252f, 0x3aa8_2d40);
    pub const COLLECTION_ITERATOR: Guid = Guid(0x4196_02ec, 0x7822_779e);
    pub const ITERATOR: Guid = Guid(0x198f_0af9, 0x0268_249f);
    pub const PHYSICALIZE: Guid = Guid(0x7522_370e, 0x37ec_15ec);
    pub const BOX_BOX_INTERSECTION: Guid = Guid(0x6415_4401, 0x76cf_37af);
    pub const SET_POSITION: Guid = Guid(0xe456_e78a, 0x4567_89aa);
    pub const SET_ORIENTATION: Guid = Guid(0x6258_74aa, 0xaa69_4132);
    pub const SET_WORLD_MATRIX: Guid = Guid(0xaa4a_a6f0, 0xddef_def4);
    pub const ROTATE: Guid = Guid(0xffff_ffee, 0xeeff_ffff);
    pub const RESTORE_IC: Guid = Guid(0x766e_4e44, 0x4fac_6d52);
    pub const TT_RESTORE_IC: Guid = Guid(0x7e0f_2f58, 0x232d_3109);
    pub const SET_IC: Guid = Guid(0x7267_1a24, 0x161a_2347);
    pub const KEEP_ACTIVE: Guid = Guid(0x7160_133a, 0x1f25_32fe);
    pub const HIDE: Guid = Guid(0x31d9_7d82, 0x78d5_4d98);
    pub const SHOW: Guid = Guid(0xa85a_213a, 0xef78_d52a);
    pub const SWITCH_ON_PARAMETER: Guid = Guid(0x4c42_aace, 0x1da4_5635);
    pub const DELAYER: Guid = Guid(0x15d4_72a5, 0x3bea_409f);
    pub const TIMER: Guid = Guid(0xa2a5_a63a, 0xe4e7_e8e5);
    pub const ACTIVATE_OBJECT: Guid = Guid(0x5f00_3fba, 0x5e57_4f60);
    pub const DEACTIVATE_OBJECT: Guid = Guid(0x160f_4b7d, 0x67de_224e);
    pub const OBJECT_COPY: Guid = Guid(0x3f6b_0ac7, 0x47d2_0f78);
    pub const TEST: Guid = Guid(0x17d6_6d26, 0x726b_7dec);
    pub const GET_CURRENT_CAMERA: Guid = Guid(0x368f_22b1, 0x2229_57e4);
    pub const TEXT_DISPLAY: Guid = Guid(0xf22d_010a, 0xf2cd_010a);
    pub const NOP: Guid = Guid(0x3025_61c4, 0x0d28_2980);
    pub const WAVE_PLAYER: Guid = Guid(0x5bde_0e45, 0x2e21_07d5);
    pub const IDENTITY: Guid = Guid(0x1515_1652, 0xaeef_ffd5);
    pub const PARAMETER_SELECTOR: Guid = Guid(0x63eb_2b0c, 0x27e2_767e);
    pub const KEY_EVENT: Guid = Guid(0x1af2_274b, 0x6b8c_1524);
    pub const PER_SECOND: Guid = Guid(0x448e_54ce, 0x75a6_55c5);
    pub const SET_PHYSICS_FORCE: Guid = Guid(0x56e2_0c57, 0x0b92_6068);
    pub const CHANGE_TEXTURE_SIZE: Guid = Guid(0x7a7c_4d7b, 0x468d_0f33);
    pub const SET_TEXTURE: Guid = Guid(0xeb12_3eb5, 0x5be3_21be);
    pub const SET_DIFFUSE: Guid = Guid(0xe1e1_e1e1, 0x1e1e_1e1e);
    pub const SET_EMISSIVE: Guid = Guid(0xd1d1_d1d1, 0x3030_3030);
    pub const IS_IN_GROUP: Guid = Guid(0x58a1_210b, 0x14e5_715c);
    pub const COUNTER: Guid = Guid(0x998f_000f, 0xf000_f899);
    pub const BINARY_MEMORY: Guid = Guid(0xd02d_67dd, 0x1021_1fdd);
    pub const GET_DELTA_TIME: Guid = Guid(0x47dc_3232, 0x64bf_203a);
    pub const CHRONO: Guid = Guid(0x5e2f_1788, 0x4f96_5a44);
    pub const WAIT_FOR_ALL: Guid = Guid(0xc044_a999, 0xfdfe_faf7);
    pub const STREAMING_EVENT: Guid = Guid(0x1f0b_52bf, 0x4c33_42dd);
    pub const SEQUENCER: Guid = Guid(0x4253_0844, 0x257b_6053);
}

/// Whether the runtime knows this prototype.
pub fn known(guid: Guid) -> bool {
    KNOWN.contains(&guid)
}

const KNOWN: &[Guid] = &[
    proto::OP,
    proto::OBJECT_DELETE,
    proto::CHANGE_TEXTURE_VIDEO_FORMAT,
    proto::ACTIVATE_SCRIPT,
    proto::DEACTIVATE_SCRIPT,
    proto::EXECUTE_SCRIPT,
    proto::POINT_PARTICLE_SYSTEM,
    proto::PLANAR_PARTICLE_SYSTEM,
    proto::TT_SCALEABLE_PROXIMITY,
    proto::PROXIMITY,
    proto::GET_CELL,
    proto::SET_CELL,
    proto::ADD_ROW,
    proto::GET_KEY_ROW,
    proto::GET_ROW,
    proto::CLEAR_ARRAY,
    proto::BINARY_SWITCH,
    proto::WAIT_MESSAGE,
    proto::SEND_MESSAGE_TO_GROUP,
    proto::SEND_MESSAGE,
    proto::GROUP_ITERATOR,
    proto::COLLECTION_ITERATOR,
    proto::ITERATOR,
    proto::PHYSICALIZE,
    proto::BOX_BOX_INTERSECTION,
    proto::SET_POSITION,
    proto::SET_ORIENTATION,
    proto::SET_WORLD_MATRIX,
    proto::ROTATE,
    proto::RESTORE_IC,
    proto::TT_RESTORE_IC,
    proto::SET_IC,
    proto::KEEP_ACTIVE,
    proto::HIDE,
    proto::SHOW,
    proto::SWITCH_ON_PARAMETER,
    proto::DELAYER,
    proto::TIMER,
    proto::ACTIVATE_OBJECT,
    proto::DEACTIVATE_OBJECT,
    proto::OBJECT_COPY,
    proto::TEST,
    proto::GET_CURRENT_CAMERA,
    proto::TEXT_DISPLAY,
    proto::NOP,
    proto::WAVE_PLAYER,
    proto::IDENTITY,
    proto::PARAMETER_SELECTOR,
    proto::KEY_EVENT,
    proto::PER_SECOND,
    proto::SET_PHYSICS_FORCE,
    proto::CHANGE_TEXTURE_SIZE,
    proto::SET_TEXTURE,
    proto::SET_DIFFUSE,
    proto::SET_EMISSIVE,
    proto::IS_IN_GROUP,
    proto::COUNTER,
    proto::BINARY_MEMORY,
    proto::GET_DELTA_TIME,
    proto::CHRONO,
    proto::WAIT_FOR_ALL,
    proto::STREAMING_EVENT,
    proto::SEQUENCER,
];

/// Reads and clears input `i`.
fn input(rt: &mut Runtime, b: &Block, i: usize) -> bool {
    let Some(&io) = b.inputs.get(i) else {
        return false;
    };
    let on = rt.io_active(io);
    if on {
        rt.set_io(io, false);
    }
    on
}

fn out(rt: &mut Runtime, b: &Block, i: usize) {
    if let Some(&io) = b.outputs.get(i) {
        rt.set_io(io, true);
    }
}

fn pin(rt: &mut Runtime, b: &Block, name: &str, index: usize, host: &mut dyn Host) -> Value {
    match rt.pin(b, name, index) {
        Some(p) => rt.read(p, host),
        None => Value::None,
    }
}

fn set_pout(rt: &mut Runtime, b: &Block, name: &str, index: usize, v: Value) {
    if let Some(p) = rt.pout(b, name, index) {
        rt.write(p, v);
    }
}

fn local(rt: &Runtime, b: &Block, index: usize) -> Value {
    b.locals
        .get(index)
        .and_then(|&l| rt.values.get(l))
        .cloned()
        .unwrap_or_default()
}

/// The target as an engine entity.
fn target_ent(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> Option<Ent> {
    let t = rt.target(b, host);
    rt.ent(t)
}

/// A map array by object id: an index into the graph's arrays.
fn array_of(rt: &Runtime, id: Option<Id>) -> Option<usize> {
    match rt.graph.objects.get(id?)?.kind {
        ObjKind::Array(i) => Some(i),
        _ => None,
    }
}

type Mat = [[f32; 4]; 4];

fn mat_mul(a: &Mat, b: &Mat) -> Mat {
    b.map(|bc| {
        let mut out = [0.0f32; 4];
        for (ak, &bk) in a.iter().zip(bc.iter()) {
            for (o, &av) in out.iter_mut().zip(ak.iter()) {
                *o += av * bk;
            }
        }
        out
    })
}

fn transform_dir(m: &Mat, p: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (col, &pk) in m.iter().take(3).zip(p.iter()) {
        for (o, &c) in out.iter_mut().zip(col.iter()) {
            *o += c * pk;
        }
    }
    out
}

fn transform_point(m: &Mat, p: [f32; 3]) -> [f32; 3] {
    let d = transform_dir(m, p);
    [d[0] + m[3][0], d[1] + m[3][1], d[2] + m[3][2]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 1e-12 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        v
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn scale_of(m: &Mat) -> [f32; 4] {
    m.map(|c| (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt())
}

/// Rotation by `angle` about `axis` (a unit vector), as a matrix.
fn rotation(axis: [f32; 3], angle: f32) -> Mat {
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    let [x, y, z] = axis;
    [
        [t * x * x + c, t * x * y + s * z, t * x * z - s * y, 0.0],
        [t * x * y - s * z, t * y * y + c, t * y * z + s * x, 0.0],
        [t * x * z + s * y, t * y * z - s * x, t * z * z + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// The referential's matrix, or the world's.
fn referential(
    rt: &mut Runtime,
    b: &Block,
    name: &str,
    index: usize,
    host: &mut dyn Host,
) -> Option<Mat> {
    let r = pin(rt, b, name, index, host).as_obj();
    let e = rt.ent(r)?;
    host.matrix(e)
}

fn distance_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

/// Compares as the original's comparison operator: 1 ==, 2 !=, 3 <, 4 <=, 5 >, 6 >=.
fn compare(op: i32, a: &Value, b: &Value) -> bool {
    match op {
        1 => a.same(b),
        2 => !a.same(b),
        3 => a.as_float() < b.as_float(),
        4 => a.as_float() <= b.as_float(),
        5 => a.as_float() > b.as_float(),
        6 => a.as_float() >= b.as_float(),
        _ => false,
    }
}

fn ent_name(rt: &Runtime, e: Option<Id>) -> String {
    e.map(|i| rt.graph.name(i).to_string()).unwrap_or_default()
}

/// Runs one building block. `b` is the block's definition; its state lives in the runtime.
pub fn run(rt: &mut Runtime, id: Id, b: &Block, proto: Guid, host: &mut dyn Host) -> Flow {
    match proto {
        proto::OP => {
            if !input(rt, b, 0) {
                return Flow::Done;
            }
            // The operation's GUID lies in the block's second and third locals (the first is a function pointer).
            let guid = Guid(
                local(rt, b, 1).as_int() as u32,
                local(rt, b, 2).as_int() as u32,
            );
            let a = pin(rt, b, "p1", 0, host);
            let c = pin(rt, b, "p2", 1, host);
            let out_guid = rt
                .pout(b, "res", 0)
                .and_then(|p| rt.graph.param(p))
                .map(|p| p.guid);
            let a_guid = rt
                .pin(b, "p1", 0)
                .and_then(|p| rt.graph.param(p))
                .map(|p| p.guid);
            let r = operation(rt, host, guid, out_guid, a_guid, &a, &c);
            set_pout(rt, b, "res", 0, r);
            out(rt, b, 0);
            Flow::Done
        }
        proto::NOP | proto::WAIT_FOR_ALL => {
            let mut any = false;
            for i in 0..b.inputs.len() {
                any |= input(rt, b, i);
            }
            if any {
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::IDENTITY => {
            input(rt, b, 0);
            for i in 0..b.pins.len() {
                let v = rt
                    .pin(b, "", i)
                    .map(|p| rt.read(p, host))
                    .unwrap_or_default();
                if let Some(p) = b.pouts.get(i).copied() {
                    rt.write(p, v);
                }
            }
            out(rt, b, 0);
            Flow::Done
        }
        proto::PARAMETER_SELECTOR => {
            for i in 0..b.inputs.len() {
                if input(rt, b, i) {
                    let v = b
                        .pins
                        .get(i)
                        .copied()
                        .map(|p| rt.read(p, host))
                        .unwrap_or_default();
                    set_pout(rt, b, "Selected", 0, v);
                    out(rt, b, 0);
                }
            }
            Flow::Done
        }
        proto::BINARY_SWITCH => {
            if input(rt, b, 0) {
                let c = pin(rt, b, "Condition", 0, host).as_bool();
                out(rt, b, usize::from(!c));
            }
            Flow::Done
        }
        proto::TEST => {
            if input(rt, b, 0) {
                let op = pin(rt, b, "Test", 0, host).as_int();
                let a = pin(rt, b, "A", 1, host);
                let c = pin(rt, b, "B", 2, host);
                out(rt, b, usize::from(!compare(op, &a, &c)));
            }
            Flow::Done
        }
        proto::SWITCH_ON_PARAMETER => {
            if input(rt, b, 0) {
                let t = pin(rt, b, "Test", 0, host);
                let mut hit = 0;
                for i in 1..b.pins.len() {
                    let v = b
                        .pins
                        .get(i)
                        .copied()
                        .map(|p| rt.read(p, host))
                        .unwrap_or_default();
                    if v.same(&t) {
                        hit = i;
                        break;
                    }
                }
                out(rt, b, hit);
            }
            Flow::Done
        }
        proto::ACTIVATE_SCRIPT | proto::EXECUTE_SCRIPT | proto::DEACTIVATE_SCRIPT => {
            if input(rt, b, 0) {
                let script = pin(
                    rt,
                    b,
                    "Script",
                    if proto == proto::DEACTIVATE_SCRIPT {
                        0
                    } else {
                        1
                    },
                    host,
                )
                .as_obj();
                if let Some(s) = script
                    && rt.graph.block(s).is_some()
                {
                    if proto == proto::DEACTIVATE_SCRIPT {
                        rt.activate(s, false, false, 0);
                    } else {
                        let reset = pin(rt, b, "Reset ?", 0, host).as_bool();
                        rt.activate(s, true, reset, 0);
                    }
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::ACTIVATE_OBJECT | proto::DEACTIVATE_OBJECT => {
            if input(rt, b, 0) {
                let o = pin(rt, b, "Object", 0, host).as_obj();
                if let Some(o) = o {
                    let reset = pin(rt, b, "Reset Scripts", 3, host).as_bool()
                        || pin(rt, b, "Reset ?", 1, host).as_bool();
                    rt.activate_object(o, proto == proto::ACTIVATE_OBJECT, reset);
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::KEEP_ACTIVE => {
            // Reset stops it; In 1 keeps Out 1 firing every tick.
            if input(rt, b, 0) {
                rt.set_scratch(id, Scratch::Flag(false));
                out(rt, b, 0);
                return Flow::Done;
            }
            if input(rt, b, 1) {
                rt.set_scratch(id, Scratch::Flag(true));
            }
            if matches!(rt.scratch(id), Scratch::Flag(true)) {
                out(rt, b, 1);
                Flow::Continue
            } else {
                Flow::Done
            }
        }
        proto::DELAYER | proto::TIMER => {
            let restart = input(rt, b, 0);
            let looped = input(rt, b, 1);
            let wait = pin(
                rt,
                b,
                if proto == proto::DELAYER {
                    "Time to Wait"
                } else {
                    "Duration"
                },
                0,
                host,
            )
            .as_float();
            let mut elapsed = match rt.scratch(id) {
                Scratch::Time(t) if !restart => *t,
                _ => 0.0,
            };
            if !restart && !looped && !matches!(rt.scratch(id), Scratch::Time(_)) {
                return Flow::Done;
            }
            if !restart {
                elapsed += host.delta_ms();
            }
            set_pout(rt, b, "Elapsed Time", 0, Value::Float(elapsed));
            set_pout(rt, b, "Delta Time", 1, Value::Float(host.delta_ms()));
            if elapsed >= wait {
                rt.set_scratch(id, Scratch::None);
                out(rt, b, 0);
                Flow::Done
            } else {
                rt.set_scratch(id, Scratch::Time(elapsed));
                if proto == proto::TIMER {
                    out(rt, b, 1);
                }
                Flow::Continue
            }
        }
        proto::CHRONO => {
            if input(rt, b, 0) {
                rt.set_scratch(id, Scratch::Time(0.0));
                out(rt, b, 0);
            }
            if input(rt, b, 1) {
                let t = match rt.scratch(id) {
                    Scratch::Time(t) => *t,
                    _ => 0.0,
                };
                set_pout(rt, b, "Elapsed Time", 0, Value::Float(t));
                rt.set_scratch(id, Scratch::None);
                out(rt, b, 1);
                return Flow::Done;
            }
            match rt.scratch(id) {
                Scratch::Time(t) => {
                    let t = *t + host.delta_ms();
                    rt.set_scratch(id, Scratch::Time(t));
                    set_pout(rt, b, "Elapsed Time", 0, Value::Float(t));
                    Flow::Continue
                }
                _ => Flow::Done,
            }
        }
        proto::GET_DELTA_TIME => {
            if input(rt, b, 0) {
                set_pout(rt, b, "Delta Time", 0, Value::Float(host.delta_ms()));
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::PER_SECOND => {
            if input(rt, b, 0) {
                let x = pin(rt, b, "X", 0, host).as_float();
                set_pout(rt, b, "Y", 0, Value::Float(x * host.delta_ms() / 1000.0));
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::COUNTER => {
            let start = input(rt, b, 0);
            let looped = input(rt, b, 1);
            if !start && !looped {
                return Flow::Done;
            }
            let count = pin(rt, b, "Count", 0, host).as_int();
            let first = pin(rt, b, "Start Index", 1, host).as_int();
            let step = pin(rt, b, "Step", 2, host).as_int().max(1);
            let i = if start {
                0
            } else {
                match rt.scratch(id) {
                    Scratch::Index(i) => *i,
                    _ => 0,
                }
            };
            if i >= i64::from(count) {
                rt.set_scratch(id, Scratch::None);
                out(rt, b, 0);
            } else {
                set_pout(
                    rt,
                    b,
                    "Value",
                    0,
                    Value::Int(first.saturating_add((i as i32).saturating_mul(step))),
                );
                rt.set_scratch(id, Scratch::Index(i.saturating_add(1)));
                out(rt, b, 1);
            }
            Flow::Done
        }
        proto::BINARY_MEMORY => {
            if input(rt, b, 0) {
                rt.set_scratch(id, Scratch::Flag(false));
                set_pout(rt, b, "State", 0, Value::Bool(false));
                out(rt, b, 0);
            }
            if input(rt, b, 1) {
                rt.set_scratch(id, Scratch::Flag(true));
                set_pout(rt, b, "State", 0, Value::Bool(true));
                out(rt, b, 1);
            }
            Flow::Done
        }
        proto::STREAMING_EVENT | proto::SEQUENCER => {
            for i in 0..b.inputs.len() {
                if input(rt, b, i) {
                    out(rt, b, i);
                }
            }
            Flow::Done
        }
        proto::WAIT_MESSAGE => {
            if input(rt, b, 1) {
                rt.set_scratch(id, Scratch::None);
                return Flow::Done;
            }
            let started = input(rt, b, 0);
            if started {
                rt.set_scratch(id, Scratch::Received(false));
            }
            if !matches!(rt.scratch(id), Scratch::Received(_)) {
                return Flow::Done;
            }
            let name = pin(rt, b, "Message", 0, host).as_string();
            if !started && rt.received(&name, b.owner) {
                rt.set_scratch(id, Scratch::None);
                out(rt, b, 0);
                return Flow::Done;
            }
            Flow::Continue
        }
        proto::SEND_MESSAGE_TO_GROUP | proto::SEND_MESSAGE => {
            if input(rt, b, 0) {
                let name = pin(rt, b, "Message", 0, host).as_string();
                let dest = pin(
                    rt,
                    b,
                    if proto == proto::SEND_MESSAGE {
                        "Dest"
                    } else {
                        "Group"
                    },
                    1,
                    host,
                )
                .as_obj();
                let to_group = dest.and_then(|d| rt.graph.objects.get(d)).map(|o| o.kind);
                match to_group {
                    Some(ObjKind::Group(g)) => {
                        for m in host.group_members(g) {
                            let gid = rt
                                .graph
                                .objects
                                .iter()
                                .position(|o| o.kind == ObjKind::Entity(m));
                            rt.send(&name, gid);
                            host.message(&name, Some(Ent::Object(m)));
                        }
                    }
                    Some(ObjKind::Entity(_)) | Some(ObjKind::Ball) => {
                        rt.send(&name, dest);
                        let e = rt.ent(dest);
                        host.message(&name, e);
                    }
                    _ => {
                        rt.send(&name, None);
                        host.message(&name, None);
                    }
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::GET_CELL | proto::SET_CELL => {
            if !input(rt, b, 0) {
                return Flow::Done;
            }
            let row = pin(rt, b, "Row Index", 0, host).as_int();
            let col = pin(rt, b, "Column Index", 1, host).as_int();
            let (Ok(row), Ok(col)) = (usize::try_from(row), usize::try_from(col)) else {
                out(rt, b, 1);
                return Flow::Done;
            };
            let t = rt.target(b, host);
            let cell = match array_of(rt, t) {
                Some(a) => {
                    if proto == proto::SET_CELL {
                        let v = pin(rt, b, "Value", 2, host);
                        let slot = rt
                            .graph
                            .arrays
                            .get(a)
                            .and_then(|arr| arr.rows.get(row))
                            .and_then(|r| r.get(col))
                            .is_some();
                        if slot {
                            // the arrays live in the shared graph; a map's own table is edited in the runtime's copy
                            rt.set_cell(a, row, col, v);
                        }
                        slot.then_some(Value::None)
                    } else {
                        rt.cell(a, row, col)
                    }
                }
                None => {
                    let name = ent_name(rt, t);
                    if proto == proto::SET_CELL {
                        None
                    } else {
                        host.game_cell(&name, row, col)
                    }
                }
            };
            match cell {
                Some(v) => {
                    if proto == proto::GET_CELL {
                        set_pout(rt, b, "Cell Value", 0, v);
                    }
                    out(rt, b, 0);
                }
                None => out(rt, b, 1),
            }
            Flow::Done
        }
        proto::GET_KEY_ROW | proto::GET_ROW => {
            if !input(rt, b, 0) {
                return Flow::Done;
            }
            let t = rt.target(b, host);
            let Some(a) = array_of(rt, t) else {
                out(rt, b, 1);
                return Flow::Done;
            };
            let row = if proto == proto::GET_ROW {
                usize::try_from(pin(rt, b, "Row Index", 0, host).as_int()).ok()
            } else {
                let key = pin(rt, b, "Key Value", 0, host);
                rt.rows(a)
                    .iter()
                    .position(|r| r.first().is_some_and(|c| c.same(&key)))
            };
            match row.and_then(|r| rt.rows(a).get(r).cloned()) {
                Some(cells) => {
                    for (i, c) in cells.into_iter().enumerate() {
                        if let Some(p) = b.pouts.get(i).copied() {
                            rt.write(p, c);
                        }
                    }
                    out(rt, b, 0);
                }
                None => out(rt, b, 1),
            }
            Flow::Done
        }
        proto::ADD_ROW => {
            if input(rt, b, 0) {
                let t = rt.target(b, host);
                if let Some(a) = array_of(rt, t) {
                    let mut row = Vec::new();
                    for i in 0..b.pins.len() {
                        row.push(
                            b.pins
                                .get(i)
                                .copied()
                                .map(|p| rt.read(p, host))
                                .unwrap_or_default(),
                        );
                    }
                    let unique = local(rt, b, 0).as_int();
                    let present = usize::try_from(unique).ok().is_some_and(|k| {
                        rt.rows(a)
                            .iter()
                            .any(|r| r.get(k).zip(row.get(k)).is_some_and(|(x, y)| x.same(y)))
                    });
                    if present {
                        out(rt, b, 1);
                    } else {
                        rt.add_row(a, row);
                        out(rt, b, 0);
                    }
                } else {
                    out(rt, b, 0);
                }
            }
            Flow::Done
        }
        proto::CLEAR_ARRAY => {
            if input(rt, b, 0) {
                let t = rt.target(b, host);
                if let Some(a) = array_of(rt, t) {
                    rt.clear_rows(a);
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::ITERATOR => {
            let start = input(rt, b, 0);
            let looped = input(rt, b, 1);
            if !start && !looped {
                return Flow::Done;
            }
            let t = rt.target(b, host);
            let rows = array_of(rt, t).map(|a| rt.rows(a).len()).unwrap_or(0) as i64;
            let first = i64::from(pin(rt, b, "Starting Row", 0, host).as_int());
            let last = i64::from(pin(rt, b, "Ending Row", 1, host).as_int());
            let last = if last < 0 {
                rows.saturating_sub(1)
            } else {
                last.min(rows.saturating_sub(1))
            };
            let backward = pin(rt, b, "Backward", 2, host).as_bool();
            let i = if start {
                if backward { last } else { first }
            } else {
                match rt.scratch(id) {
                    Scratch::Index(i) => *i,
                    _ => first,
                }
            };
            let done = if backward { i < first } else { i > last };
            if done || i < 0 {
                rt.set_scratch(id, Scratch::None);
                out(rt, b, 0);
                return Flow::Done;
            }
            set_pout(rt, b, "Row Index", 0, Value::Int(i as i32));
            let cells = array_of(rt, t)
                .and_then(|a| rt.rows(a).get(i as usize).cloned())
                .unwrap_or_default();
            for (k, c) in cells.into_iter().enumerate() {
                if let Some(p) = b.pouts.get(k.saturating_add(1)).copied() {
                    rt.write(p, c);
                }
            }
            rt.set_scratch(
                id,
                Scratch::Index(if backward {
                    i.saturating_sub(1)
                } else {
                    i.saturating_add(1)
                }),
            );
            out(rt, b, 1);
            Flow::Done
        }
        proto::GROUP_ITERATOR | proto::COLLECTION_ITERATOR => {
            let start = input(rt, b, 0);
            let looped = input(rt, b, 1);
            if !start && !looped {
                return Flow::Done;
            }
            let g = pin(
                rt,
                b,
                if proto == proto::GROUP_ITERATOR {
                    "Group"
                } else {
                    "Collection"
                },
                0,
                host,
            )
            .as_obj();
            let members: Vec<Id> = match g.and_then(|x| rt.graph.objects.get(x)).map(|o| o.kind) {
                Some(ObjKind::Group(gi)) => host
                    .group_members(gi)
                    .into_iter()
                    .filter_map(|m| {
                        rt.graph
                            .objects
                            .iter()
                            .position(|o| o.kind == ObjKind::Entity(m))
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let i = if start {
                0
            } else {
                match rt.scratch(id) {
                    Scratch::Index(i) => *i,
                    _ => 0,
                }
            };
            match usize::try_from(i)
                .ok()
                .and_then(|k| members.get(k).copied())
            {
                Some(m) => {
                    set_pout(rt, b, "Element", 0, Value::Obj(Some(m)));
                    set_pout(rt, b, "Index", 1, Value::Int(i as i32));
                    rt.set_scratch(id, Scratch::Index(i.saturating_add(1)));
                    out(rt, b, 1);
                }
                None => {
                    rt.set_scratch(id, Scratch::None);
                    out(rt, b, 0);
                }
            }
            Flow::Done
        }
        proto::IS_IN_GROUP => {
            if input(rt, b, 0) {
                let o = pin(rt, b, "Object", 0, host).as_obj();
                let g = pin(rt, b, "Group", 1, host).as_obj();
                let e = rt.ent(o);
                let hit = match (e, g.and_then(|x| rt.graph.objects.get(x)).map(|o| o.kind)) {
                    (Some(Ent::Object(i)), Some(ObjKind::Group(gi))) => {
                        host.group_members(gi).contains(&i)
                    }
                    _ => false,
                };
                out(rt, b, usize::from(!hit));
            }
            Flow::Done
        }
        proto::HIDE | proto::SHOW => {
            if input(rt, b, 0) {
                if let Some(e) = target_ent(rt, b, host) {
                    host.set_visible(e, proto == proto::SHOW);
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::OBJECT_DELETE => {
            if input(rt, b, 0) {
                let o = pin(rt, b, "Object", 0, host).as_obj();
                match o.and_then(|x| rt.graph.objects.get(x)).map(|i| i.kind) {
                    Some(ObjKind::Entity(_)) | Some(ObjKind::Ball) => {
                        if let Some(e) = rt.ent(o) {
                            host.delete(e);
                        }
                    }
                    Some(ObjKind::Group(gi)) => {
                        // deleting a group deletes the group object only; its members stay (as the original)
                        host.note(&format!("script deletes group {} (members kept)", gi));
                    }
                    Some(ObjKind::Behavior) => {
                        if let Some(x) = o {
                            rt.activate(x, false, false, 0);
                        }
                    }
                    _ => {}
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::PHYSICALIZE => {
            let on = input(rt, b, 0);
            let off = input(rt, b, 1);
            if on || off {
                if let Some(e) = target_ent(rt, b, host) {
                    host.physicalize(e, on);
                }
                out(rt, b, usize::from(!on));
            }
            Flow::Done
        }
        proto::BOX_BOX_INTERSECTION => {
            if input(rt, b, 0) {
                let a = pin(rt, b, "Entity 0", 0, host).as_obj();
                let c = pin(rt, b, "Entity 1", 1, host).as_obj();
                let (ea, ec) = (rt.ent(a), rt.ent(c));
                let hit = match (
                    ea.and_then(|e| host.bounds(e)),
                    ec.and_then(|e| host.bounds(e)),
                ) {
                    (Some((amin, amax)), Some((cmin, cmax))) => amin
                        .iter()
                        .zip(amax.iter())
                        .zip(cmin.iter().zip(cmax.iter()))
                        .all(|((a0, a1), (c0, c1))| a0 <= c1 && c0 <= a1),
                    _ => false,
                };
                out(rt, b, usize::from(!hit));
            }
            Flow::Done
        }
        proto::TT_SCALEABLE_PROXIMITY | proto::PROXIMITY => {
            let off = proto == proto::TT_SCALEABLE_PROXIMITY && input(rt, b, 1);
            if off {
                rt.set_scratch(id, Scratch::None);
                return Flow::Done;
            }
            let started = input(rt, b, 0);
            if started {
                rt.set_scratch(
                    id,
                    Scratch::Proximity {
                        inside: None,
                        wait: 0,
                    },
                );
            }
            let Scratch::Proximity { inside, wait } = rt.scratch(id).clone() else {
                return Flow::Done;
            };
            if wait > 0 {
                rt.set_scratch(
                    id,
                    Scratch::Proximity {
                        inside,
                        wait: wait.saturating_sub(1),
                    },
                );
                return Flow::Continue;
            }
            let distance = pin(rt, b, "Distance", 0, host).as_float();
            let a = pin(
                rt,
                b,
                if proto == proto::PROXIMITY {
                    "Object A"
                } else {
                    "ObjectA"
                },
                1,
                host,
            )
            .as_obj();
            let c = pin(
                rt,
                b,
                if proto == proto::PROXIMITY {
                    "Object B"
                } else {
                    "ObjectB"
                },
                2,
                host,
            )
            .as_obj();
            let pos = |rt: &Runtime, host: &dyn Host, o: Option<Id>| {
                rt.ent(o)
                    .and_then(|e| host.matrix(e))
                    .map(|m| [m[3][0], m[3][1], m[3][2]])
            };
            let (Some(pa), Some(pc)) = (pos(rt, host, a), pos(rt, host, c)) else {
                return Flow::Continue;
            };
            let d2 = distance_sq(pa, pc);
            set_pout(rt, b, "Current Distance", 0, Value::Float(d2.sqrt()));
            let now = d2 <= distance * distance;
            // The test runs every N ticks, N going from the minimum frame delay at the near exactness to the
            // maximum at the far one (both compared squared), as the original's.
            let mut next_wait = 0i64;
            if proto == proto::TT_SCALEABLE_PROXIMITY {
                let near = pin(rt, b, "Exactness min. Distance", 4, host).as_float();
                let far = pin(rt, b, "Exactness max. Distance", 5, host).as_float();
                let fmin = pin(rt, b, "Minimum Framedelay", 6, host).as_float();
                let fmax = pin(rt, b, "Maximum Framedelay", 7, host).as_float();
                let (n2, f2) = (near * near, far * far);
                let t = if f2 > n2 {
                    ((d2 - n2) / (f2 - n2)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                next_wait = ((fmin + (fmax - fmin) * t).floor().max(1.0) as i64).saturating_sub(1);
            }
            rt.set_scratch(
                id,
                Scratch::Proximity {
                    inside: Some(now),
                    wait: next_wait,
                },
            );
            match (inside, now) {
                (Some(false) | None, true) => out(rt, b, 0),
                (Some(true), false) => out(rt, b, 1),
                _ => {
                    if proto == proto::PROXIMITY {
                        out(rt, b, 2);
                    }
                }
            }
            if proto == proto::PROXIMITY {
                Flow::Done
            } else {
                Flow::Continue
            }
        }
        proto::SET_POSITION => {
            if input(rt, b, 0) {
                let p = pin(rt, b, "Position", 0, host).as_vec3();
                let r = referential(rt, b, "Referential", 1, host);
                if let Some(e) = target_ent(rt, b, host)
                    && let Some(mut m) = host.matrix(e)
                {
                    let world = match r {
                        Some(rm) => transform_point(&rm, p),
                        None => p,
                    };
                    m[3] = [world[0], world[1], world[2], 1.0];
                    host.set_matrix(e, m);
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::SET_ORIENTATION => {
            if input(rt, b, 0) {
                let dir = pin(rt, b, "Dir", 0, host).as_vec3();
                let up = pin(rt, b, "Up", 1, host).as_vec3();
                let r = referential(rt, b, "Referential", 3, host);
                if let Some(e) = target_ent(rt, b, host)
                    && let Some(mut m) = host.matrix(e)
                {
                    let (dir, up) = match r {
                        Some(rm) => (transform_dir(&rm, dir), transform_dir(&rm, up)),
                        None => (dir, up),
                    };
                    let z = normalize(dir);
                    let x = normalize(cross(up, z));
                    let y = cross(z, x);
                    let s = scale_of(&m);
                    m[0] = [x[0] * s[0], x[1] * s[0], x[2] * s[0], 0.0];
                    m[1] = [y[0] * s[1], y[1] * s[1], y[2] * s[1], 0.0];
                    m[2] = [z[0] * s[2], z[1] * s[2], z[2] * s[2], 0.0];
                    host.set_matrix(e, m);
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::ROTATE => {
            if input(rt, b, 0) {
                let axis = pin(rt, b, "Axis of Rotation", 0, host).as_vec3();
                let angle = pin(rt, b, "Angle of Rotation", 1, host).as_float();
                let r = referential(rt, b, "Referential of the Axis", 2, host);
                if let Some(e) = target_ent(rt, b, host)
                    && let Some(m) = host.matrix(e)
                {
                    let axis = normalize(match r {
                        Some(rm) => transform_dir(&rm, axis),
                        None => axis,
                    });
                    let pos = m[3];
                    let mut rotated = mat_mul(&rotation(axis, angle), &m);
                    rotated[3] = pos;
                    host.set_matrix(e, rotated);
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::SET_WORLD_MATRIX => {
            if input(rt, b, 0) {
                if let Some(m) = pin(rt, b, "World Matrix", 0, host).as_matrix()
                    && let Some(e) = target_ent(rt, b, host)
                {
                    host.set_matrix(e, m);
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::RESTORE_IC | proto::TT_RESTORE_IC => {
            if input(rt, b, 0) {
                if let Some(e) = target_ent(rt, b, host)
                    && let Some(m) = host.initial_matrix(e)
                {
                    host.set_matrix(e, m);
                }
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::SET_IC
        | proto::CHANGE_TEXTURE_SIZE
        | proto::SET_TEXTURE
        | proto::SET_DIFFUSE
        | proto::SET_EMISSIVE => {
            // Looks only, or initial conditions kept as loaded: accepted, nothing to do in this engine.
            if input(rt, b, 0) {
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::CHANGE_TEXTURE_VIDEO_FORMAT => {
            if input(rt, b, 0) {
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::POINT_PARTICLE_SYSTEM | proto::PLANAR_PARTICLE_SYSTEM => {
            let on = input(rt, b, 0);
            let off = input(rt, b, 1);
            let freeze = input(rt, b, 2);
            let e = rt.ent(b.owner);
            if on {
                if let Some(e) = e {
                    host.particles(e, true);
                }
                rt.set_scratch(id, Scratch::Flag(true));
                out(rt, b, 0);
            }
            if off || freeze {
                if let Some(e) = e {
                    host.particles(e, false);
                }
                rt.set_scratch(id, Scratch::None);
                out(rt, b, if off { 1 } else { 2 });
                return Flow::Done;
            }
            if matches!(rt.scratch(id), Scratch::Flag(true)) {
                Flow::Continue
            } else {
                Flow::Done
            }
        }
        proto::WAVE_PLAYER => {
            let t = rt.target(b, host);
            let name = ent_name(rt, t);
            if input(rt, b, 0) {
                host.sound(&name, true);
                out(rt, b, 0);
            }
            if input(rt, b, 1) {
                host.sound(&name, false);
                out(rt, b, 1);
            }
            if input(rt, b, 2) {
                out(rt, b, 2);
            }
            Flow::Done
        }
        proto::TEXT_DISPLAY => {
            let t = rt.target(b, host);
            let name = ent_name(rt, t);
            if input(rt, b, 0) {
                let text = pin(rt, b, "Text", 4, host).as_string();
                host.text(&name, &text, true);
                out(rt, b, 0);
            }
            if input(rt, b, 1) {
                host.text(&name, "", false);
                out(rt, b, 1);
            }
            Flow::Done
        }
        proto::GET_CURRENT_CAMERA => {
            if input(rt, b, 0) {
                set_pout(rt, b, "Current Camera", 0, Value::Obj(None));
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::OBJECT_COPY => {
            if input(rt, b, 0) {
                let o = pin(rt, b, "Original", 1, host).as_obj();
                host.note(&format!(
                    "script copies {} (copies are not made yet; the copy is nothing)",
                    ent_name(rt, o)
                ));
                set_pout(rt, b, "Copy", 0, Value::Obj(None));
                out(rt, b, 0);
            }
            Flow::Done
        }
        proto::SET_PHYSICS_FORCE => {
            if input(rt, b, 0) {
                host.note("script sets a physics force (not applied yet)");
                out(rt, b, 0);
            }
            if input(rt, b, 1) {
                out(rt, b, 1);
            }
            Flow::Done
        }
        proto::KEY_EVENT => {
            // Keys other than the game's are not in a run's input: the event never fires.
            if input(rt, b, 1) {
                return Flow::Done;
            }
            input(rt, b, 0);
            Flow::Continue
        }
        _ => Flow::Unknown,
    }
}

pub mod ops {
    use super::Guid;
    pub const ADD: Guid = Guid(0x33cc_6b49, 0x3589_282b);
    pub const SUB: Guid = Guid(0x6764_1171, 0x6499_077a);
    pub const MUL: Guid = Guid(0x3899_6b85, 0x334e_35c2);
    pub const DIV: Guid = Guid(0x748c_3502, 0x0fea_0cf1);
    pub const MOD: Guid = Guid(0x2a3f_4ab6, 0x753a_4a20);
    pub const CONVERT: Guid = Guid(0x4280_09b4, 0x1caa_5c78);
    pub const BY_NAME: Guid = Guid(0x5992_03f6, 0x23b0_6096);
    pub const NAME: Guid = Guid(0x5939_156e, 0x62ab_3472);
    pub const POSITION: Guid = Guid(0x4bc8_7aea, 0x6b5b_643e);
    pub const CURRENT_MESH: Guid = Guid(0x6a5c_5061, 0x5efe_7428);
    pub const WORLD_MATRIX: Guid = Guid(0x4728_51ba, 0x5ccc_2831);
    pub const GET_Y: Guid = Guid(0x389f_72bd, 0x7aef_3482);
    pub const LENGTH: Guid = Guid(0x556a_69af, 0x076e_3f09);
    pub const ROW_COUNT: Guid = Guid(0xfb55_6b23, 0x69b2_2a75);
    pub const GROUP_COUNT: Guid = Guid(0x44df_6f61, 0x5f8f_55c8);
}

fn is_float(g: Option<Guid>, v: &Value) -> bool {
    matches!(g, Some(types::FLOAT) | Some(types::TIME)) || matches!(v, Value::Float(_))
}

fn arith(
    f: fn(f32, f32) -> f32,
    i: fn(i32, i32) -> i32,
    a: &Value,
    b: &Value,
    floaty: bool,
) -> Value {
    match (a, b) {
        (Value::Vec3(x), Value::Vec3(y)) => {
            Value::Vec3([f(x[0], y[0]), f(x[1], y[1]), f(x[2], y[2])])
        }
        (Value::Vec3(x), s) | (s, Value::Vec3(x)) => {
            let k = s.as_float();
            Value::Vec3([f(x[0], k), f(x[1], k), f(x[2], k)])
        }
        (Value::Str(x), y) => Value::Str(format!("{x}{}", y.as_string())),
        _ if floaty => Value::Float(f(a.as_float(), b.as_float())),
        _ => Value::Int(i(a.as_int(), b.as_int())),
    }
}

/// A parameter operation: the Op block's and the graphs' own. `out` and `a` are the output's and first input's
/// types when known; a conversion goes by the output's type.
pub fn operation(
    rt: &mut Runtime,
    host: &mut dyn Host,
    guid: Guid,
    out: Option<Guid>,
    a_guid: Option<Guid>,
    a: &Value,
    b: &Value,
) -> Value {
    let floaty = is_float(out, a) || is_float(a_guid, a) || matches!(b, Value::Float(_));
    match guid {
        ops::ADD => arith(|x, y| x + y, i32::wrapping_add, a, b, floaty),
        ops::SUB => arith(|x, y| x - y, i32::wrapping_sub, a, b, floaty),
        ops::MUL => arith(|x, y| x * y, i32::wrapping_mul, a, b, floaty),
        ops::DIV => {
            // the original makes a zero divisor one and says so
            let safe_f = |x: f32, y: f32| if y == 0.0 { x } else { x / y };
            let safe_i = |x: i32, y: i32| x.checked_div(y).unwrap_or(x);
            arith(safe_f, safe_i, a, b, floaty)
        }
        ops::MOD => {
            let d = b.as_int();
            Value::Int(a.as_int().checked_rem_euclid(d).unwrap_or(0))
        }
        ops::CONVERT | ops::BY_NAME => match out {
            Some(types::FLOAT) | Some(types::TIME) => Value::Float(a.as_float()),
            Some(types::INT) => Value::Int(a.as_int()),
            Some(types::BOOL) => Value::Bool(a.as_bool()),
            Some(types::STRING) => Value::Str(a.as_string()),
            _ => match a {
                Value::Str(name) => Value::Obj(rt.graph.find(name)),
                v => v.clone(),
            },
        },
        ops::NAME => Value::Str(ent_name(rt, a.as_obj())),
        ops::POSITION => {
            let e = rt.ent(a.as_obj());
            match e.and_then(|e| host.matrix(e)) {
                Some(m) => Value::Vec3([m[3][0], m[3][1], m[3][2]]),
                None => Value::Vec3([0.0; 3]),
            }
        }
        ops::WORLD_MATRIX => {
            let e = rt.ent(a.as_obj());
            e.and_then(|e| host.matrix(e))
                .map_or(Value::None, Value::Matrix)
        }
        ops::CURRENT_MESH => {
            let e = rt.ent(a.as_obj());
            let mesh = e.and_then(|e| host.mesh_of(e));
            Value::Obj(mesh.and_then(|m| {
                rt.graph
                    .objects
                    .iter()
                    .position(|o| o.kind == ObjKind::Mesh(m))
            }))
        }
        ops::GET_Y => match a {
            Value::Vec3(v) => Value::Float(v[1]),
            Value::Obj(o) => {
                let e = rt.ent(*o);
                Value::Float(e.and_then(|e| host.matrix(e)).map_or(0.0, |m| m[3][1]))
            }
            v => Value::Float(v.as_float()),
        },
        ops::LENGTH => Value::Int(a.as_string().len() as i32),
        ops::ROW_COUNT => {
            Value::Int(array_of(rt, a.as_obj()).map_or(0, |i| rt.rows(i).len()) as i32)
        }
        ops::GROUP_COUNT => {
            let n = match a
                .as_obj()
                .and_then(|x| rt.graph.objects.get(x))
                .map(|o| o.kind)
            {
                Some(ObjKind::Group(g)) => host.group_members(g).len(),
                _ => 0,
            };
            Value::Int(n as i32)
        }
        _ => {
            host.note(&format!(
                "script operation not known: {guid}; its first input is passed on"
            ));
            a.clone()
        }
    }
}
