// The parameter operations as data (docs/ck-runtime.md, "Operations"): one row per (operation, output type,
// input 1 type, input 2 type) with its function, looked up as the original's parameter manager does — the exact
// signature first, then with the types' parents in a fixed order, then with the inputs swapped. A signature
// without a function is reported once per run; a graph's own operation then copies input 1 to its output, an
// Op block writes nothing. Each row says where its meaning comes from: the C reference (`src/ck`, written from
// ParameterOperations.dll) or the operation's registered name where the C lacks the function.

use crate::exec::{Host, Runtime};
use crate::types as t;
use crate::{Guid, Id, ObjKind, Value};

/// The longest string an operation builds.
pub const MAX_STRING: usize = 1 << 16;

/// The operations by GUID (named as the original registers them).
pub mod op {
    use crate::Guid;
    pub const ADD: Guid = Guid(0x33cc_6b49, 0x3589_282b);
    pub const SUB: Guid = Guid(0x6764_1171, 0x6499_077a);
    pub const MUL: Guid = Guid(0x3899_6b85, 0x334e_35c2);
    pub const DIV: Guid = Guid(0x748c_3502, 0x0fea_0cf1);
    pub const MOD: Guid = Guid(0x2a3f_4ab6, 0x753a_4a20);
    pub const CONVERT: Guid = Guid(0x4280_09b4, 0x1caa_5c78);
    pub const BY_NAME: Guid = Guid(0x5992_03f6, 0x23b0_6096);
    pub const NAME: Guid = Guid(0x5939_156e, 0x62ab_3472);
    pub const CLASS_ID: Guid = Guid(0x35aa_0ddb, 0x7a5e_3335);
    pub const POSITION: Guid = Guid(0x4bc8_7aea, 0x6b5b_643e);
    pub const CURRENT_MESH: Guid = Guid(0x6a5c_5061, 0x5efe_7428);
    pub const WORLD_MATRIX: Guid = Guid(0x4728_51ba, 0x5ccc_2831);
    pub const GET_X: Guid = Guid(0x1292_6657, 0x6228_322e);
    pub const GET_Y: Guid = Guid(0x389f_72bd, 0x7aef_3482);
    pub const GET_Z: Guid = Guid(0x093d_5158, 0x53ef_47a0);
    pub const GET_W: Guid = Guid(0x098d_8765, 0x53ea_46a0);
    pub const SET_X: Guid = Guid(0x48c2_0def, 0xfecd_00a1);
    pub const SET_Y: Guid = Guid(0x48c2_0dee, 0xfec2_00af);
    pub const SET_Z: Guid = Guid(0x48c2_0ded, 0xfec2_0da1);
    pub const GET_RIGHT: Guid = Guid(0x48c2_0ded, 0xf15a_5781);
    pub const GET_UP: Guid = Guid(0x48c2_0ded, 0xfed1_2457);
    pub const GET_DIR: Guid = Guid(0x48c2_0ded, 0xfed1_2252);
    pub const NORMALIZE: Guid = Guid(0x35cc_27a6, 0x25d2_209d);
    pub const OPPOSITE: Guid = Guid(0x599c_2cb6, 0x3e7d_4de8);
    pub const CROSS: Guid = Guid(0x3be0_5df3, 0x143a_49e0);
    pub const MAGNITUDE: Guid = Guid(0x68fd_46f9, 0x26e0_7e46);
    pub const ARC_COSINE: Guid = Guid(0x7eed_1d16, 0x345b_7184);
    pub const LENGTH: Guid = Guid(0x556a_69af, 0x076e_3f09);
    pub const ROW_COUNT: Guid = Guid(0xfb55_6b23, 0x69b2_2a75);
    pub const GROUP_COUNT: Guid = Guid(0x44df_6f61, 0x5f8f_55c8);
    pub const SCRIPT: Guid = Guid(0x2500_20d4, 0x10d5_1b91);
    pub const SCRIPT_COUNT: Guid = Guid(0x551c_20ba, 0x20da_6d50);
    pub const MATERIAL: Guid = Guid(0x6b8c_78bd, 0x7b84_6d3c);
    pub const MATERIAL_LIST: Guid = Guid(0x765f_7aa3, 0x33d6_79b2);
    pub const ELEMENT: Guid = Guid(0x660f_5c52, 0x0adf_7be9);
    pub const MESH: Guid = Guid(0x1309_5053, 0x7f9b_44e2);
    pub const DIFFUSE: Guid = Guid(0x0f9e_20cc, 0x6ea1_0a47);
    pub const EMISSIVE: Guid = Guid(0x1368_4596, 0x0ab5_7f34);
    /// The C reference's "box maximum" row; the same GUID as `MIN`, which the DLL registers as Min.
    pub const MAX: Guid = Guid(0x6ea2_14cb, 0x43be_6b56);
    /// Min (ParameterOperations.dll, `re/ops.txt`): Integer, Float, Vector, 2D Vector pairs; the C's box row reads
    /// the box's second corner (the DLL registers Min of a Box too).
    pub const MIN: Guid = Guid(0x6ea2_14cb, 0x43be_6b56);
    /// Max.
    pub const MAXIMUM: Guid = Guid(0x7c2e_01a8, 0x6e70_2206);
    /// Absolute Value (Integer, Float).
    pub const ABS: Guid = Guid(0x1ada_7470, 0x6209_7629);
    /// Get Distance (Float ← Vector, Vector / 3D Entity, 3D Entity; Vector ← 3D Entity, 3D Entity is the second
    /// minus the first).
    pub const DISTANCE: Guid = Guid(0x265c_7d00, 0x5c8f_1702);
    /// Contain String (Boolean ← String, String): whether input 1 contains input 2 (the DLL stores strstr's pointer).
    pub const CONTAIN_STRING: Guid = Guid(0x5e86_6012, 0x5860_306d);
    /// Inverse (Matrix, Vector, 2D Vector, Float, Color).
    pub const INVERSE: Guid = Guid(0x7c27_550b, 0x32a3_2e99);
    /// Is Derived From (Boolean ← Object, Object): CKIsChildClassOf(input 1, the class of input 2).
    pub const IS_DERIVED_FROM: Guid = Guid(0x6ab9_4f5a, 0x1a63_76a9);
    /// Equal.
    pub const EQUAL: Guid = Guid(0x6c62_476b, 0x7a92_2973);
    /// Xor (Integer, Boolean).
    pub const XOR: Guid = Guid(0x0144_05eb, 0x2e08_7eec);
    /// Is Visible (Boolean ← 3D / 2D Entity).
    pub const IS_VISIBLE: Guid = Guid(0x0a54_7387, 0x4108_51af);
    /// Get Children (Object Array ← 3D Entity).
    pub const CHILDREN: Guid = Guid(0x0dd6_35d5, 0x47e6_2a28);
    /// Get Parent (3D Entity, 2D Entity).
    pub const PARENT: Guid = Guid(0x5bf9_718f, 0x030b_698c);
    /// Get Width / Get Height (Integer ← nothing: the render area; ← Texture; ← 2D Entity).
    pub const WIDTH: Guid = Guid(0x5a4e_13fe, 0x628b_7ca2);
    pub const HEIGHT: Guid = Guid(0x37f9_040e, 0x52c0_5c4e);
    /// Set Alpha (Color ← Color, Float).
    pub const SET_ALPHA: Guid = Guid(0x48c1_0000, 0xfec1_00a0);
    /// Degre To Radian (Float ← Float).
    pub const DEG_TO_RAD: Guid = Guid(0x49c1_00e0, 0xfeb1_05a9);
    pub const BOUNDING_BOX: Guid = Guid(0x31a7_649f, 0x305e_5164);
}

/// An operation's function: (runtime, host, the output's type, input 1, input 2) → the output's new value, or
/// none to leave the output as it is (the original's functions return early on a missing object).
pub type OpFn = fn(&mut Runtime, &mut dyn Host, Guid, &Value, &Value) -> Option<Value>;

/// Where a row's meaning comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The C reference (the function is checked against it by the oracle where the oracle can build its inputs).
    C,
    /// The operation's registered name; the C has no function for it (it copies input 1).
    Name,
    /// ParameterOperations.dll's own function for it (read from the disassembly, `re/ops.txt`); the C has none.
    Dll,
}

#[derive(Debug, Clone, Copy)]
pub struct Row {
    pub op: Guid,
    pub res: Guid,
    pub p1: Guid,
    pub p2: Guid,
    pub f: OpFn,
    pub source: Source,
}

const fn r(op: Guid, res: Guid, p1: Guid, p2: Guid, f: OpFn, source: Source) -> Row {
    Row {
        op,
        res,
        p1,
        p2,
        f,
        source,
    }
}

use Source::{C, Dll as D, Name as N};
use t::{
    BEOBJECT, BOOL, BOX, COLOR, ENTITY, ENTITY2D, EULER, FLOAT, GROUP, INT, MATERIAL, MATRIX, MESH,
    NONE as NO, OBJECT, OBJECT_ARRAY, PERCENTAGE, QUATERNION, RECT, SCRIPT, STRING, TEXTURE, TIME,
    VECTOR, VECTOR2,
};

/// Every function the runtime has.
pub static TABLE: &[Row] = &[
    // arithmetic (C)
    r(
        op::ADD,
        INT,
        INT,
        INT,
        |_, _, _, a, b| Some(Value::Int(a.as_int().wrapping_add(b.as_int()))),
        C,
    ),
    r(
        op::ADD,
        FLOAT,
        FLOAT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Float(a.as_float() + b.as_float())),
        C,
    ),
    r(op::ADD, STRING, STRING, STRING, add_string, C),
    r(
        op::SUB,
        INT,
        INT,
        INT,
        |_, _, _, a, b| Some(Value::Int(a.as_int().wrapping_sub(b.as_int()))),
        C,
    ),
    r(
        op::SUB,
        FLOAT,
        FLOAT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Float(a.as_float() - b.as_float())),
        C,
    ),
    r(
        op::MUL,
        INT,
        INT,
        INT,
        |_, _, _, a, b| Some(Value::Int(a.as_int().wrapping_mul(b.as_int()))),
        C,
    ),
    r(
        op::MUL,
        FLOAT,
        FLOAT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Float(a.as_float() * b.as_float())),
        C,
    ),
    r(
        op::MUL,
        FLOAT,
        INT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Float(a.as_int() as f32 * b.as_float())),
        C,
    ),
    r(
        op::MUL,
        INT,
        FLOAT,
        INT,
        |_, _, _, a, b| Some(Value::Int(ftol(a.as_float() * b.as_int() as f32))),
        C,
    ),
    r(op::DIV, INT, INT, INT, div_int, C),
    r(op::DIV, FLOAT, FLOAT, FLOAT, div_float, C),
    r(op::MOD, INT, INT, INT, mod_int, C),
    // arithmetic the C lacks (by name)
    r(
        op::ADD,
        VECTOR,
        VECTOR,
        VECTOR,
        |_, _, _, a, b| Some(Value::Vec3(zip3(a, b, |x, y| x + y))),
        N,
    ),
    r(
        op::SUB,
        VECTOR,
        VECTOR,
        VECTOR,
        |_, _, _, a, b| Some(Value::Vec3(zip3(a, b, |x, y| x - y))),
        N,
    ),
    r(
        op::SUB,
        FLOAT,
        INT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Float(a.as_int() as f32 - b.as_float())),
        N,
    ),
    r(
        op::MUL,
        VECTOR,
        VECTOR,
        FLOAT,
        |_, _, _, a, b| Some(Value::Vec3(a.as_vec3().map(|x| x * b.as_float()))),
        N,
    ),
    r(op::MUL, VECTOR, MATRIX, VECTOR, mul_matrix_vector, N),
    r(op::DIV, VECTOR, VECTOR, FLOAT, div_vector, N),
    r(
        op::DIV,
        FLOAT,
        INT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Float(a.as_int() as f32 / nonzero(b.as_float()))),
        N,
    ),
    r(
        op::OPPOSITE,
        INT,
        INT,
        NO,
        |_, _, _, a, _| Some(Value::Int(a.as_int().wrapping_neg())),
        N,
    ),
    r(
        op::OPPOSITE,
        FLOAT,
        FLOAT,
        NO,
        |_, _, _, a, _| Some(Value::Float(-a.as_float())),
        N,
    ),
    r(
        op::OPPOSITE,
        VECTOR,
        VECTOR,
        NO,
        |_, _, _, a, _| Some(Value::Vec3(a.as_vec3().map(|x| -x))),
        N,
    ),
    r(
        op::ARC_COSINE,
        FLOAT,
        FLOAT,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_float().acos())),
        N,
    ),
    // Min, Max, Abs, Distance and the equality tests: named by their use in the maps (the C has none of them)
    r(
        op::MIN,
        INT,
        INT,
        INT,
        |_, _, _, a, b| Some(Value::Int(a.as_int().min(b.as_int()))),
        D,
    ),
    r(
        op::MIN,
        FLOAT,
        FLOAT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Float(fmin(a.as_float(), b.as_float()))),
        D,
    ),
    r(
        op::MAXIMUM,
        INT,
        INT,
        INT,
        |_, _, _, a, b| Some(Value::Int(a.as_int().max(b.as_int()))),
        D,
    ),
    r(
        op::MAXIMUM,
        FLOAT,
        FLOAT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Float(fmax(a.as_float(), b.as_float()))),
        D,
    ),
    r(
        op::ABS,
        INT,
        INT,
        NO,
        |_, _, _, a, _| Some(Value::Int(a.as_int().wrapping_abs())),
        D,
    ),
    r(
        op::ABS,
        FLOAT,
        FLOAT,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_float().abs())),
        D,
    ),
    r(
        op::DISTANCE,
        FLOAT,
        VECTOR,
        VECTOR,
        |_, _, _, a, b| Some(Value::Float(length(zip3(a, b, |x, y| x - y)))),
        D,
    ),
    r(op::DISTANCE, FLOAT, ENTITY, ENTITY, entity_distance, D),
    r(
        op::INVERSE,
        MATRIX,
        MATRIX,
        NO,
        |_, _, _, a, _| a.as_matrix().and_then(|m| invert(&m)).map(Value::Matrix),
        D,
    ),
    // the rest of ParameterOperations.dll's table (`re/ops.txt`); the C has none of these
    r(op::CONTAIN_STRING, BOOL, STRING, STRING, contain_string, D),
    r(
        op::IS_DERIVED_FROM,
        BOOL,
        OBJECT,
        OBJECT,
        is_derived_from,
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        STRING,
        STRING,
        |_, _, _, a, b| Some(Value::Bool(equal_strings(a, b))),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        INT,
        INT,
        |_, _, _, a, b| Some(Value::Bool(a.as_int() == b.as_int())),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        FLOAT,
        FLOAT,
        |_, _, _, a, b| {
            Some(Value::Bool(
                a.as_float().to_bits() == b.as_float().to_bits(),
            ))
        },
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        INT,
        FLOAT,
        |_, _, _, a, b| {
            Some(Value::Bool(
                f64::from(a.as_int()) == f64::from(b.as_float()),
            ))
        },
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        BOOL,
        BOOL,
        |_, _, _, a, b| Some(Value::Bool(a.as_bool() == b.as_bool())),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        OBJECT,
        OBJECT,
        |_, _, _, a, b| Some(Value::Bool(a.as_obj() == b.as_obj())),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        VECTOR2,
        VECTOR2,
        |_, _, _, a, b| Some(Value::Bool(bits_equal(a, b))),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        VECTOR,
        VECTOR,
        |_, _, _, a, b| Some(Value::Bool(bits_equal(a, b))),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        EULER,
        EULER,
        |_, _, _, a, b| Some(Value::Bool(bits_equal(a, b))),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        QUATERNION,
        QUATERNION,
        |_, _, _, a, b| Some(Value::Bool(bits_equal(a, b))),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        RECT,
        RECT,
        |_, _, _, a, b| Some(Value::Bool(bits_equal(a, b))),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        COLOR,
        COLOR,
        |_, _, _, a, b| Some(Value::Bool(bits_equal(a, b))),
        D,
    ),
    r(
        op::EQUAL,
        BOOL,
        MATRIX,
        MATRIX,
        |_, _, _, a, b| Some(Value::Bool(bits_equal(a, b))),
        D,
    ),
    r(
        op::XOR,
        INT,
        INT,
        INT,
        |_, _, _, a, b| Some(Value::Int(a.as_int() ^ b.as_int())),
        D,
    ),
    r(
        op::XOR,
        BOOL,
        BOOL,
        BOOL,
        |_, _, _, a, b| Some(Value::Bool(a.as_bool() != b.as_bool())),
        D,
    ),
    r(op::IS_VISIBLE, BOOL, ENTITY, NO, is_visible, D),
    r(op::IS_VISIBLE, BOOL, ENTITY2D, NO, is_visible, D),
    r(op::CHILDREN, OBJECT_ARRAY, ENTITY, NO, children, D),
    r(op::PARENT, ENTITY, ENTITY, NO, parent, D),
    r(op::PARENT, ENTITY2D, ENTITY2D, NO, parent, D),
    r(
        op::WIDTH,
        INT,
        NO,
        NO,
        |_, h, _, _, _| h.screen_size().map(|s| Value::Int(s.0)),
        D,
    ),
    r(
        op::HEIGHT,
        INT,
        NO,
        NO,
        |_, h, _, _, _| h.screen_size().map(|s| Value::Int(s.1)),
        D,
    ),
    r(
        op::WIDTH,
        INT,
        TEXTURE,
        NO,
        |rt, h, _, a, _| texture_size(rt, h, a, false),
        D,
    ),
    r(
        op::HEIGHT,
        INT,
        TEXTURE,
        NO,
        |rt, h, _, a, _| texture_size(rt, h, a, true),
        D,
    ),
    r(op::SET_ALPHA, COLOR, COLOR, FLOAT, set_alpha, D),
    r(op::MATERIAL, MATERIAL, t::SPRITE3D, NO, sprite_material, D),
    r(
        op::DEG_TO_RAD,
        FLOAT,
        FLOAT,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_float() * 0.017_453_292)),
        D,
    ),
    r(op::DISTANCE, VECTOR, ENTITY, ENTITY, entity_offset, D),
    r(op::DIV, QUATERNION, QUATERNION, QUATERNION, quat_div, D),
    // vectors and quaternions (by name)
    r(
        op::NORMALIZE,
        VECTOR,
        VECTOR,
        NO,
        |_, _, _, a, _| Some(Value::Vec3(normalize(a.as_vec3()))),
        N,
    ),
    r(
        op::CROSS,
        VECTOR,
        VECTOR,
        VECTOR,
        |_, _, _, a, b| Some(Value::Vec3(cross(a.as_vec3(), b.as_vec3()))),
        N,
    ),
    r(
        op::MAGNITUDE,
        FLOAT,
        VECTOR,
        NO,
        |_, _, _, a, _| Some(Value::Float(length(a.as_vec3()))),
        N,
    ),
    r(
        op::SET_X,
        VECTOR,
        VECTOR,
        FLOAT,
        |_, _, _, a, b| Some(Value::Vec3(set(a.as_vec3(), 0, b.as_float()))),
        N,
    ),
    r(
        op::SET_Y,
        VECTOR,
        VECTOR,
        FLOAT,
        |_, _, _, a, b| Some(Value::Vec3(set(a.as_vec3(), 1, b.as_float()))),
        N,
    ),
    r(
        op::SET_Z,
        VECTOR,
        VECTOR,
        FLOAT,
        |_, _, _, a, b| Some(Value::Vec3(set(a.as_vec3(), 2, b.as_float()))),
        N,
    ),
    r(
        op::GET_X,
        FLOAT,
        QUATERNION,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_vec4()[0])),
        N,
    ),
    r(
        op::GET_Y,
        FLOAT,
        QUATERNION,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_vec4()[1])),
        N,
    ),
    r(
        op::GET_Z,
        FLOAT,
        QUATERNION,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_vec4()[2])),
        N,
    ),
    r(
        op::GET_W,
        FLOAT,
        QUATERNION,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_vec4()[3])),
        N,
    ),
    r(
        op::GET_Y,
        FLOAT,
        VECTOR,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_vec3()[1])),
        C,
    ),
    r(
        op::GET_Y,
        FLOAT,
        ENTITY,
        NO,
        |rt, h, _, a, _| world(rt, h, a).map(|m| Value::Float(m[3][1])),
        C,
    ),
    r(
        op::MAX,
        VECTOR,
        BOX,
        NO,
        |_, _, _, a, _| Some(Value::Vec3(box_max(a))),
        C,
    ),
    // conversions (C)
    r(
        op::CONVERT,
        FLOAT,
        INT,
        NO,
        |_, _, _, a, _| Some(Value::Float(a.as_int() as f32)),
        C,
    ),
    r(
        op::CONVERT,
        INT,
        FLOAT,
        NO,
        |_, _, _, a, _| Some(Value::Int(ftol(a.as_float()))),
        C,
    ),
    r(
        op::CONVERT,
        VECTOR2,
        FLOAT,
        FLOAT,
        |_, _, _, a, b| Some(Value::Vec2([a.as_float(), b.as_float()])),
        C,
    ),
    r(op::CONVERT, INT, STRING, NO, from_string, C),
    r(op::CONVERT, OBJECT, STRING, NO, from_string, C),
    r(op::CONVERT, STRING, INT, NO, to_string, C),
    r(op::CONVERT, STRING, FLOAT, NO, to_string, C),
    r(op::CONVERT, STRING, BOOL, NO, to_string, C),
    r(op::CONVERT, STRING, TIME, NO, to_string, C),
    r(op::CONVERT, STRING, PERCENTAGE, NO, to_string, C),
    // conversions the C lacks (by name)
    r(
        op::CONVERT,
        INT,
        BOOL,
        NO,
        |_, _, _, a, _| Some(Value::Int(a.as_int())),
        N,
    ),
    r(
        op::CONVERT,
        QUATERNION,
        MATRIX,
        NO,
        |_, _, _, a, _| a.as_matrix().map(|m| Value::Quat(quat_of(&m))),
        N,
    ),
    // objects (C)
    r(op::BY_NAME, OBJECT, STRING, NO, from_string, C),
    r(
        op::LENGTH,
        INT,
        STRING,
        NO,
        |_, _, _, a, _| Some(Value::Int(a.as_string().len().min(i32::MAX as usize) as i32)),
        C,
    ),
    r(
        op::NAME,
        STRING,
        OBJECT,
        NO,
        |rt, _, _, a, _| Some(Value::Str(name(rt, a.as_obj()))),
        C,
    ),
    r(op::CLASS_ID, t::CLASS_ID, OBJECT, NO, class_id, C),
    r(op::ROW_COUNT, INT, t::ARRAY, NO, row_count, C),
    r(op::GROUP_COUNT, INT, GROUP, NO, group_count, C),
    r(op::SCRIPT, SCRIPT, BEOBJECT, STRING, script_by_name, D),
    r(op::SCRIPT, SCRIPT, BEOBJECT, INT, script_of, C),
    r(
        op::SCRIPT_COUNT,
        INT,
        BEOBJECT,
        NO,
        |rt, _, _, a, _| {
            Some(Value::Int(
                scripts(rt, a).len().min(i32::MAX as usize) as i32
            ))
        },
        C,
    ),
    r(op::CURRENT_MESH, MESH, ENTITY, NO, current_mesh, C),
    r(op::MATERIAL, MATERIAL, MESH, INT, material_of, C),
    r(
        op::MATERIAL,
        MATERIAL,
        ENTITY2D,
        NO,
        |_, _, _, _, _| Some(Value::Obj(None)),
        C,
    ),
    r(
        op::WORLD_MATRIX,
        MATRIX,
        ENTITY,
        NO,
        |rt, h, _, a, _| world(rt, h, a).map(Value::Matrix),
        C,
    ),
    r(op::POSITION, VECTOR, ENTITY, ENTITY, position, C),
    r(op::POSITION, VECTOR, ENTITY, NO, position, C),
    r(op::POSITION, VECTOR2, ENTITY2D, NO, |_, _, _, _, _| None, C),
    r(op::BOUNDING_BOX, BOX, ENTITY, NO, bounding_box, C),
    r(
        op::DIFFUSE,
        COLOR,
        MATERIAL,
        NO,
        |rt, h, _, a, _| colour(rt, h, a, false),
        C,
    ),
    r(
        op::EMISSIVE,
        COLOR,
        MATERIAL,
        NO,
        |rt, h, _, a, _| colour(rt, h, a, true),
        C,
    ),
    r(op::ELEMENT, OBJECT, OBJECT_ARRAY, INT, element, C),
    r(op::MATERIAL_LIST, OBJECT_ARRAY, MESH, NO, material_list, C),
    // entities the C lacks (by name)
    r(
        op::GET_RIGHT,
        VECTOR,
        ENTITY,
        NO,
        |rt, h, _, a, _| world(rt, h, a).map(|m| axis(&m, 0)),
        N,
    ),
    r(
        op::GET_UP,
        VECTOR,
        ENTITY,
        NO,
        |rt, h, _, a, _| world(rt, h, a).map(|m| axis(&m, 1)),
        N,
    ),
    r(
        op::GET_DIR,
        VECTOR,
        ENTITY,
        NO,
        |rt, h, _, a, _| world(rt, h, a).map(|m| axis(&m, 2)),
        N,
    ),
    r(op::MESH, MESH, ENTITY, INT, mesh_of, N),
];

/// Whether the table has any function of this operation.
pub fn known(op: Guid) -> bool {
    TABLE.iter().any(|r| r.op == op)
}

/// The function of a signature, if any: the exact row, else the parents of the inputs and the output tried in the
/// original's order (both inputs' parents, then input 1's, then input 2's, each with the output's parent first, then
/// the output's parent alone), recursively; else the same with the inputs swapped. Returns the row and whether the
/// inputs are swapped.
pub fn find(op: Guid, res: Guid, p1: Guid, p2: Guid) -> Option<(&'static Row, bool)> {
    lookup(op, res, p1, p2, 0)
        .map(|r| (r, false))
        .or_else(|| lookup(op, res, p2, p1, 0).map(|r| (r, true)))
}

fn exact(op: Guid, res: Guid, p1: Guid, p2: Guid) -> Option<&'static Row> {
    TABLE
        .iter()
        .find(|r| r.op == op && r.res == res && r.p1 == p1 && r.p2 == p2)
}

fn lookup(op: Guid, res: Guid, p1: Guid, p2: Guid, depth: usize) -> Option<&'static Row> {
    if depth > 16 {
        return None;
    }
    if let Some(r) = exact(op, res, p1, p2) {
        return Some(r);
    }
    let d = depth.saturating_add(1);
    let (rp, pp1, pp2) = (t::parent(res), t::parent(p1), t::parent(p2));
    let try_res = |a: Guid, b: Guid| -> Option<&'static Row> {
        rp.and_then(|rp| lookup(op, rp, a, b, d))
            .or_else(|| lookup(op, res, a, b, d))
    };
    if let Some(pp1) = pp1 {
        if let Some(pp2) = pp2
            && let Some(r) = try_res(pp1, pp2)
        {
            return Some(r);
        }
        if let Some(r) = try_res(pp1, p2) {
            return Some(r);
        }
    }
    if let Some(pp2) = pp2
        && let Some(r) = try_res(p1, pp2)
    {
        return Some(r);
    }
    rp.and_then(|rp| lookup(op, rp, p1, p2, d))
}

/// Runs an operation: its value for the output, or none to leave the output as it is. `copy_on_miss`: a graph's
/// own operation without a function copies input 1 (CKParameterOperation::DoOperation); an Op block writes nothing.
#[allow(clippy::too_many_arguments)]
pub fn run(
    rt: &mut Runtime,
    host: &mut dyn Host,
    guid: Guid,
    res: Guid,
    p1: Guid,
    p2: Guid,
    a: &Value,
    b: &Value,
    copy_on_miss: bool,
) -> Option<Value> {
    match rt.op_function(guid, res, p1, p2, host) {
        Some((row, false)) => (row.f)(rt, host, res, a, b),
        Some((row, true)) => (row.f)(rt, host, res, b, a),
        None if copy_on_miss => Some(a.clone()),
        None => None,
    }
}

// ---- the functions --------------------------------------------------------------------------------------------

/// A float to an integer as the original's x86 code converts (truncation toward zero; out of range or NaN gives
/// the "integer indefinite" `i32::MIN`).
pub fn ftol(f: f32) -> i32 {
    if f.is_nan() || f >= 2_147_483_648.0 || f < -2_147_483_648.0 {
        i32::MIN
    } else {
        f as i32
    }
}

fn fmin(a: f32, b: f32) -> f32 {
    if b < a { b } else { a }
}

fn fmax(a: f32, b: f32) -> f32 {
    if b > a { b } else { a }
}

/// The distance of two entities' world positions (nothing written without both).
fn entity_distance(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    b: &Value,
) -> Option<Value> {
    let (ma, mb) = (world(rt, host, a)?, world(rt, host, b)?);
    let d = [
        ma[3][0] - mb[3][0],
        ma[3][1] - mb[3][1],
        ma[3][2] - mb[3][2],
    ];
    Some(Value::Float(length(d)))
}

/// Whether input 1 contains input 2 (strstr; a missing string gives false, an empty one is found at once).
fn contain_string(
    _: &mut Runtime,
    _: &mut dyn Host,
    _: Guid,
    a: &Value,
    b: &Value,
) -> Option<Value> {
    Some(Value::Bool(match (a, b) {
        (Value::Str(x), Value::Str(y)) => x.contains(y.as_str()),
        _ => false,
    }))
}

/// Equal on two strings: strcmp, so case matters; a missing string is false.
fn equal_strings(a: &Value, b: &Value) -> bool {
    matches!((a, b), (Value::Str(x), Value::Str(y)) if x == y)
}

/// The words of a value as the original's `repz cmps` compares them (bit for bit, so 0.0 and -0.0 differ).
fn words(v: &Value) -> Vec<u32> {
    match v {
        Value::Vec2(x) => x.iter().map(|f| f.to_bits()).collect(),
        Value::Vec3(x) => x.iter().map(|f| f.to_bits()).collect(),
        Value::Quat(x) | Value::Rect(x) | Value::Color(x) => {
            x.iter().map(|f| f.to_bits()).collect()
        }
        Value::Matrix(m) => m.iter().flatten().map(|f| f.to_bits()).collect(),
        _ => Vec::new(),
    }
}

fn bits_equal(a: &Value, b: &Value) -> bool {
    words(a) == words(b)
}

/// The CK class tree of the classes a level holds: the class a class derives from.
fn class_parent(c: u32) -> Option<u32> {
    Some(match c {
        // 2 / 3 / 4 / 6 / 8 / 9 (parameters, links, behaviours, ios), 10 scene, 16 render context, 20 ...: roots below Object
        11 => 1,  // scene object -> object
        19 => 11, // be object -> scene object
        10 | 21 | 23 | 24 | 30 | 31 | 32 | 47 | 52 | 51 => 19,
        25 | 26 => 24, // wave, midi sound -> sound
        33 | 27 => 47, // 3D entity, 2D entity -> render object
        34 | 36 | 37 | 38 | 40 | 41 | 43 => 33,
        35 => 34, // target camera -> camera
        39 => 38, // target light -> light
        42 => 41, // body part -> 3D object
        28 => 27, // sprite -> 2D entity
        29 => 28, // sprite text -> sprite
        2..=9 | 12..=18 | 20 | 22 | 44..=46 | 48..=50 | 53.. => 1,
        _ => return None,
    })
}

/// Whether class `c` is `base` or derives from it (CKIsChildClassOf).
fn class_derives(c: u32, base: u32) -> bool {
    let mut at = Some(c);
    for _ in 0..16 {
        match at {
            Some(x) if x == base => return true,
            Some(x) => at = class_parent(x),
            None => return false,
        }
    }
    false
}

/// Is Derived From: the class of input 1 is, or derives from, the class of input 2 (nothing written when either object
/// is gone).
fn is_derived_from(
    rt: &mut Runtime,
    _: &mut dyn Host,
    _: Guid,
    a: &Value,
    b: &Value,
) -> Option<Value> {
    let class = |v: &Value| {
        v.as_obj()
            .and_then(|o| rt.graph.objects.get(o))
            .map(|o| o.class)
    };
    let (c1, c2) = (class(a)?, class(b)?);
    Some(Value::Bool(class_derives(c1, c2)))
}

fn is_visible(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    _: &Value,
) -> Option<Value> {
    let e = rt.ent(a.as_obj())?;
    host.is_visible(e).map(Value::Bool)
}

/// Get Parent: nothing written for an object without one.
fn parent(rt: &mut Runtime, host: &mut dyn Host, _: Guid, a: &Value, _: &Value) -> Option<Value> {
    let e = rt.ent(a.as_obj())?;
    let p = host.parent_of(e)?;
    let kind = match p {
        crate::Ent::Object(i) => ObjKind::Entity(i),
        crate::Ent::Ball => ObjKind::Ball,
    };
    object_of(rt, kind).map(|o| Value::Obj(Some(o)))
}

/// Get Children: the object's children as an Object Array (paid from the tick's work, one unit each).
fn children(rt: &mut Runtime, host: &mut dyn Host, _: Guid, a: &Value, _: &Value) -> Option<Value> {
    let e = rt.ent(a.as_obj())?;
    let kids = host.children_of(e);
    if !rt.pay(kids.len() as u64) {
        return None;
    }
    let list = kids
        .into_iter()
        .take(crate::MAX_CHILDREN)
        .map(|k| object_of(rt, ObjKind::Entity(k)))
        .collect();
    Some(Value::Objects(list))
}

fn texture_size(rt: &mut Runtime, host: &mut dyn Host, a: &Value, height: bool) -> Option<Value> {
    let o = rt.graph.objects.get(a.as_obj()?)?;
    let ObjKind::Texture(t) = o.kind else {
        return None;
    };
    let info = host.texture_info(t)?;
    Some(Value::Int(
        i32::try_from(if height { info.height } else { info.width }).unwrap_or(i32::MAX),
    ))
}

/// Set Alpha: the colour with its alpha set to the number, at most 1 (no lower bound).
fn set_alpha(_: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let [r, g, bl, _] = a.as_vec4();
    let f = b.as_float();
    Some(Value::Color([r, g, bl, if f > 1.0 { 1.0 } else { f }]))
}

/// Get Distance of two entities as a Vector: the second's world position minus the first's.
fn entity_offset(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    b: &Value,
) -> Option<Value> {
    let (ma, mb) = (world(rt, host, a)?, world(rt, host, b)?);
    Some(Value::Vec3([
        mb[3][0] - ma[3][0],
        mb[3][1] - ma[3][1],
        mb[3][2] - ma[3][2],
    ]))
}

/// Get Script with a name: the first script of the object whose name contains the string (nothing written if none).
fn script_by_name(
    rt: &mut Runtime,
    _: &mut dyn Host,
    _: Guid,
    a: &Value,
    b: &Value,
) -> Option<Value> {
    let key = b.as_string();
    let found = scripts(rt, a)
        .into_iter()
        .find(|&s| rt.graph.name(s).contains(key.as_str()))?;
    Some(Value::Obj(Some(found)))
}

/// Quaternion division, VxMath's `Vx3DQuaternionDivide(P, Q)`: conj(Q) * P (x, y, z, w).
fn quat_div(_: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let (p, q) = (a.as_vec4(), b.as_vec4());
    Some(Value::Quat([
        q[3] * p[0] - q[0] * p[3] - p[2] * q[1] + p[1] * q[2],
        p[1] * q[3] - p[3] * q[1] - p[0] * q[2] + p[2] * q[0],
        p[2] * q[3] - p[3] * q[2] - p[1] * q[0] + p[0] * q[1],
        p[2] * q[2] + p[1] * q[1] + p[0] * q[0] + q[3] * p[3],
    ]))
}

/// The inverse of a 4x4 matrix (rows as the world matrices keep them), `None` for a singular one.
pub(crate) fn invert(m: &[[f32; 4]; 4]) -> Option<[[f32; 4]; 4]> {
    let a: Vec<f64> = m.iter().flatten().map(|&x| f64::from(x)).collect();
    // Gauss-Jordan with partial pivoting on [a | 1]
    let mut w = [[0.0f64; 8]; 4];
    for r in 0..4 {
        for c in 0..4 {
            w[r][c] = a.get(r * 4 + c).copied().unwrap_or(0.0);
        }
        w[r][4 + r] = 1.0;
    }
    for col in 0..4 {
        let piv = (col..4).max_by(|&x, &y| w[x][col].abs().total_cmp(&w[y][col].abs()))?;
        if w[piv][col].abs() < 1e-12 || !w[piv][col].is_finite() {
            return None;
        }
        w.swap(col, piv);
        let d = w[col][col];
        for c in 0..8 {
            w[col][c] /= d;
        }
        for r in 0..4 {
            if r != col {
                let f = w[r][col];
                for c in 0..8 {
                    w[r][c] -= f * w[col][c];
                }
            }
        }
    }
    let mut out = [[0.0f32; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] = w[r][4 + c] as f32;
        }
    }
    Some(out)
}

fn nonzero(d: f32) -> f32 {
    if d == 0.0 { 1.0 } else { d }
}

/// The original's divide guard: a zero divisor becomes 1 (and is logged there).
fn div_int(_: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let d = match b.as_int() {
        0 => 1,
        d => d,
    };
    // i32::MIN / -1 overflows: it stays i32::MIN, as the wrapping machine division gives
    Some(Value::Int(a.as_int().checked_div(d).unwrap_or(i32::MIN)))
}

fn div_float(_: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    Some(Value::Float(a.as_float() / nonzero(b.as_float())))
}

fn div_vector(_: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let d = nonzero(b.as_float());
    Some(Value::Vec3(a.as_vec3().map(|x| x / d)))
}

/// The remainder takes the divisor's side when negative (`r < 0` → `r + d`); a divisor of 0 gives 0.
fn mod_int(_: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let d = b.as_int();
    if d == 0 {
        return Some(Value::Int(0));
    }
    let r = a.as_int().checked_rem(d).unwrap_or(0);
    Some(Value::Int(if r < 0 { r.wrapping_add(d) } else { r }))
}

fn add_string(_: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let x = a.as_string();
    // a string that doubles each time it is read would eat the memory: past the cap it stops growing
    if x.len() >= MAX_STRING {
        return Some(Value::Str(x));
    }
    Some(Value::Str(format!("{x}{}", b.as_string())))
}

fn zip3(a: &Value, b: &Value, f: fn(f32, f32) -> f32) -> [f32; 3] {
    let (x, y) = (a.as_vec3(), b.as_vec3());
    [f(x[0], y[0]), f(x[1], y[1]), f(x[2], y[2])]
}

fn set(mut v: [f32; 3], i: usize, x: f32) -> [f32; 3] {
    if let Some(c) = v.get_mut(i) {
        *c = x;
    }
    v
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// A unit vector; the zero vector stays zero.
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = length(v);
    if l > 0.0 { v.map(|x| x / l) } else { v }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn box_max(a: &Value) -> [f32; 3] {
    match a {
        Value::Box(b) => [b[3], b[4], b[5]],
        _ => [0.0; 3],
    }
}

/// Matrix × Vector: the point transformed (rotation, scale and translation).
#[allow(clippy::indexing_slicing)] // constant indices into fixed-size arrays
fn mul_matrix_vector(
    _: &mut Runtime,
    _: &mut dyn Host,
    _: Guid,
    a: &Value,
    b: &Value,
) -> Option<Value> {
    let m = a.as_matrix()?;
    let p = b.as_vec3();
    Some(Value::Vec3(std::array::from_fn(|i| {
        m[0][i] * p[0] + m[1][i] * p[1] + m[2][i] * p[2] + m[3][i]
    })))
}

/// A rotation matrix's quaternion (x, y, z, w), the largest-diagonal method; the matrix's scale is not removed.
#[allow(clippy::indexing_slicing)] // constant indices into fixed-size arrays
fn quat_of(m: &[[f32; 4]; 4]) -> [f32; 4] {
    // m[c][r]: column c, row r
    let e = |r: usize, c: usize| m[c][r];
    let tr = e(0, 0) + e(1, 1) + e(2, 2);
    if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        [
            (e(2, 1) - e(1, 2)) / s,
            (e(0, 2) - e(2, 0)) / s,
            (e(1, 0) - e(0, 1)) / s,
            0.25 * s,
        ]
    } else if e(0, 0) > e(1, 1) && e(0, 0) > e(2, 2) {
        let s = (1.0 + e(0, 0) - e(1, 1) - e(2, 2)).max(0.0).sqrt() * 2.0;
        let s = nonzero(s);
        [
            0.25 * s,
            (e(0, 1) + e(1, 0)) / s,
            (e(0, 2) + e(2, 0)) / s,
            (e(2, 1) - e(1, 2)) / s,
        ]
    } else if e(1, 1) > e(2, 2) {
        let s = nonzero((1.0 + e(1, 1) - e(0, 0) - e(2, 2)).max(0.0).sqrt() * 2.0);
        [
            (e(0, 1) + e(1, 0)) / s,
            0.25 * s,
            (e(1, 2) + e(2, 1)) / s,
            (e(0, 2) - e(2, 0)) / s,
        ]
    } else {
        let s = nonzero((1.0 + e(2, 2) - e(0, 0) - e(1, 1)).max(0.0).sqrt() * 2.0);
        [
            (e(0, 2) + e(2, 0)) / s,
            (e(1, 2) + e(2, 1)) / s,
            0.25 * s,
            (e(1, 0) - e(0, 1)) / s,
        ]
    }
}

/// A world matrix's axis (0 right, 1 up, 2 direction), as a unit vector.
fn axis(m: &[[f32; 4]; 4], i: usize) -> Value {
    let c = m.get(i).copied().unwrap_or_default();
    Value::Vec3(normalize([c[0], c[1], c[2]]))
}

/// C's `%g`: six significant digits, trailing zeros dropped, an exponent below 1e-4 and from 1e6 on.
pub fn fmt_g(x: f32) -> String {
    let x = f64::from(x);
    if x.is_nan() {
        return if x.is_sign_negative() { "-nan" } else { "nan" }.into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.into();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0" } else { "0" }.into();
    }
    // the exponent after rounding to six significant digits
    let sci = format!("{x:.5e}");
    let (mant, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let strip = |s: String| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    };
    if !(-4..6).contains(&exp) {
        let m = strip(mant.to_string());
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{m}e{sign}{:02}", exp.unsigned_abs())
    } else {
        let decimals = usize::try_from(5i32.saturating_sub(exp)).unwrap_or(0);
        strip(format!("{x:.decimals$}"))
    }
}

/// Anything → String through the input's type's string function: Integer `%d`, Float (and its children) `%g`,
/// Boolean `TRUE`/`FALSE`, an object its name (`NULL` for none), a string itself.
fn to_string(rt: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, _: &Value) -> Option<Value> {
    Some(Value::Str(match a {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => fmt_g(*f),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.into(),
        Value::Str(s) => s.clone(),
        Value::Obj(o) => o.map_or_else(|| "NULL".into(), |o| rt.graph.name(o).to_string()),
        v => v.as_string(),
    }))
}

/// The leading number of a string as C's `sscanf("%d")` reads it (0 when there is none).
fn scan_int(s: &str) -> i32 {
    let s = s.trim_start();
    let (neg, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, s.get(1..).unwrap_or_default()),
        Some(b'+') => (false, s.get(1..).unwrap_or_default()),
        _ => (false, s),
    };
    let mut v: i64 = 0;
    for c in digits.bytes().take_while(u8::is_ascii_digit) {
        v = v
            .saturating_mul(10)
            .saturating_add(i64::from(c.saturating_sub(b'0')));
    }
    let v = if neg { v.saturating_neg() } else { v };
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// The leading decimal number of a string as C's `sscanf("%f")` reads it (0 when there is none): sign, digits,
/// a point, an exponent; the longest prefix that is a number (a long run of digits is cut at 64 characters).
fn scan_float(s: &str) -> f32 {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .take(64)
        .take_while(|&(_, c)| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E'))
        .last()
        .map_or(0, |(i, c)| i.saturating_add(c.len_utf8()));
    let mut prefix = s.get(..end).unwrap_or_default();
    while !prefix.is_empty() {
        if let Ok(f) = prefix.parse::<f32>() {
            return f;
        }
        prefix = prefix
            .get(..prefix.len().saturating_sub(1))
            .unwrap_or_default();
    }
    0.0
}

/// String → the output's type through its string function: a number, `TRUE` (any case) for a Boolean, an object
/// of the output type's class by name (as Get Object By Name: the first of that name, any case; a pseudo-object
/// of the game is of every class).
fn from_string(
    rt: &mut Runtime,
    _: &mut dyn Host,
    res: Guid,
    a: &Value,
    _: &Value,
) -> Option<Value> {
    text_to_value(rt, res, a.as_string())
}

/// A text as a value of a type, through the type's string function (as Convert String → that type).
pub(crate) fn text_to_value(rt: &Runtime, res: Guid, s: String) -> Option<Value> {
    Some(match t::kind(res) {
        Some(t::Kind::Int | t::Kind::Enum) => Value::Int(scan_int(&s)),
        Some(t::Kind::Float) => Value::Float(scan_float(&s)),
        Some(t::Kind::Bool) => Value::Bool(s.eq_ignore_ascii_case("true")),
        Some(t::Kind::Str | t::Kind::Message | t::Kind::Attribute) => Value::Str(s),
        Some(t::Kind::Object(class)) => Value::Obj(
            rt.graph
                .objects
                .iter()
                .position(|o| o.name.eq_ignore_ascii_case(&s) && t::class_derives(o.class, class)),
        ),
        _ => return None,
    })
}

/// A value of a type as text, through the type's string function as the C reference has it (CK2.dll: Integer `%d`,
/// Float `%g`, Boolean `TRUE` / `FALSE`, a string itself, an object its name or `NULL`, a message its name); the
/// other types give an empty text, as there.
pub(crate) fn value_text(rt: &Runtime, guid: Guid, v: &Value) -> String {
    match t::kind(guid) {
        Some(t::Kind::Int | t::Kind::Enum) => v.as_int().to_string(),
        Some(t::Kind::Float) => fmt_g(v.as_float()),
        Some(t::Kind::Bool) => if v.as_int() != 0 { "TRUE" } else { "FALSE" }.into(),
        Some(t::Kind::Str | t::Kind::Message) => v.as_string(),
        Some(t::Kind::Object(_)) => v
            .as_obj()
            .map_or_else(|| "NULL".into(), |o| rt.graph.name(o).to_string()),
        _ => String::new(),
    }
}

fn name(rt: &Runtime, o: Option<Id>) -> String {
    o.map(|i| rt.graph.name(i).to_string()).unwrap_or_default()
}

fn world(rt: &mut Runtime, host: &mut dyn Host, a: &Value) -> Option<[[f32; 4]; 4]> {
    let e = rt.ent(a.as_obj())?;
    host.matrix(e)
}

fn class_id(rt: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, _: &Value) -> Option<Value> {
    let c = a
        .as_obj()
        .and_then(|o| rt.graph.objects.get(o))
        .map_or(0, |o| o.class);
    Some(Value::Int(i32::try_from(c).unwrap_or(0)))
}

fn row_count(rt: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, _: &Value) -> Option<Value> {
    let n = match a
        .as_obj()
        .and_then(|o| rt.graph.objects.get(o))
        .map(|o| o.kind)
    {
        Some(ObjKind::Array(i)) => rt.rows(i).len(),
        _ => 0,
    };
    Some(Value::Int(n.min(i32::MAX as usize) as i32))
}

fn group_count(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    _: &Value,
) -> Option<Value> {
    let n = match a
        .as_obj()
        .and_then(|o| rt.graph.objects.get(o))
        .map(|o| o.kind)
    {
        Some(ObjKind::Group(g)) => rt.group_ids(g, host).len(),
        _ => 0,
    };
    Some(Value::Int(n.min(i32::MAX as usize) as i32))
}

fn scripts(rt: &Runtime, a: &Value) -> Vec<Id> {
    a.as_obj().map(|o| rt.scripts_of(o)).unwrap_or_default()
}

fn script_of(rt: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let i = usize::try_from(b.as_int()).ok();
    Some(Value::Obj(i.and_then(|i| scripts(rt, a).get(i).copied())))
}

/// The graph object standing for a level object of this kind.
fn object_of(rt: &Runtime, kind: ObjKind) -> Option<Id> {
    rt.graph.objects.iter().position(|o| o.kind == kind)
}

fn current_mesh(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    _: &Value,
) -> Option<Value> {
    let e = rt.ent(a.as_obj());
    let mesh = e.and_then(|e| host.mesh_of(e));
    Some(Value::Obj(
        mesh.and_then(|m| object_of(rt, ObjKind::Mesh(m))),
    ))
}

/// Get Mesh (entity, index): the entity's meshes are only its current one here (index 0).
fn mesh_of(
    rt: &mut Runtime,
    host: &mut dyn Host,
    res: Guid,
    a: &Value,
    b: &Value,
) -> Option<Value> {
    if b.as_int() != 0 {
        return Some(Value::Obj(None));
    }
    current_mesh(rt, host, res, a, b)
}

fn materials(rt: &Runtime, host: &dyn Host, a: &Value) -> Vec<Option<Id>> {
    match a
        .as_obj()
        .and_then(|o| rt.graph.objects.get(o))
        .map(|o| o.kind)
    {
        Some(ObjKind::Mesh(m)) => host
            .mesh_materials(m)
            .into_iter()
            .map(|x| object_of(rt, ObjKind::Material(x)))
            .collect(),
        _ => Vec::new(),
    }
}

fn material_of(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    b: &Value,
) -> Option<Value> {
    let i = usize::try_from(b.as_int()).ok();
    Some(Value::Obj(i.and_then(|i| {
        materials(rt, host, a).get(i).copied().flatten()
    })))
}

/// Get Material of a 3D sprite: its mesh's first material (nothing written without one).
fn sprite_material(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    _: &Value,
) -> Option<Value> {
    let e = rt.ent(a.as_obj())?;
    let m = *host.mesh_materials(host.mesh_of(e)?).first()?;
    object_of(rt, ObjKind::Material(m)).map(|o| Value::Obj(Some(o)))
}

/// Get Material List: nothing written for what is not a mesh.
fn material_list(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    _: &Value,
) -> Option<Value> {
    let is_mesh = matches!(
        a.as_obj()
            .and_then(|o| rt.graph.objects.get(o))
            .map(|o| o.kind),
        Some(ObjKind::Mesh(_))
    );
    is_mesh.then(|| Value::Objects(materials(rt, host, a)))
}

fn element(_: &mut Runtime, _: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let i = usize::try_from(b.as_int()).ok();
    Some(Value::Obj(match a {
        Value::Objects(v) => i.and_then(|i| v.get(i).copied().flatten()),
        _ => None,
    }))
}

/// Get Position: the entity's place, in the referential's frame when input 2 is an entity.
#[allow(clippy::indexing_slicing)] // constant indices into fixed-size arrays
fn position(rt: &mut Runtime, host: &mut dyn Host, _: Guid, a: &Value, b: &Value) -> Option<Value> {
    let m = world(rt, host, a)?;
    let p = [m[3][0], m[3][1], m[3][2]];
    let Some(r) = world(rt, host, b) else {
        return Some(Value::Vec3(p));
    };
    // into the referential's frame: the inverse of its rotation-scale applied to p - its origin
    let d = [p[0] - r[3][0], p[1] - r[3][1], p[2] - r[3][2]];
    let cols = [0, 1, 2].map(|c| [r[c][0], r[c][1], r[c][2]]);
    let det = cols[0][0] * (cols[1][1] * cols[2][2] - cols[2][1] * cols[1][2])
        - cols[1][0] * (cols[0][1] * cols[2][2] - cols[2][1] * cols[0][2])
        + cols[2][0] * (cols[0][1] * cols[1][2] - cols[1][1] * cols[0][2]);
    if det.abs() < 1e-20 {
        return Some(Value::Vec3(d));
    }
    // Cramer's rule: solve cols * x = d
    let solve = |k: usize| {
        let mut m = cols;
        if let Some(c) = m.get_mut(k) {
            *c = d;
        }
        (m[0][0] * (m[1][1] * m[2][2] - m[2][1] * m[1][2])
            - m[1][0] * (m[0][1] * m[2][2] - m[2][1] * m[0][2])
            + m[2][0] * (m[0][1] * m[1][2] - m[1][1] * m[0][2]))
            / det
    };
    Some(Value::Vec3([solve(0), solve(1), solve(2)]))
}

fn bounding_box(
    rt: &mut Runtime,
    host: &mut dyn Host,
    _: Guid,
    a: &Value,
    _: &Value,
) -> Option<Value> {
    let e = rt.ent(a.as_obj())?;
    let (lo, hi) = host.bounds(e)?;
    Some(Value::Box([lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]]))
}

fn colour(rt: &mut Runtime, host: &mut dyn Host, a: &Value, emissive: bool) -> Option<Value> {
    match a
        .as_obj()
        .and_then(|o| rt.graph.objects.get(o))
        .map(|o| o.kind)
    {
        Some(ObjKind::Material(m)) => host.material_color(m, emissive).map(Value::Color),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Each signature against values worked out by hand (not by running the code): the C's rows and the rows
    /// taken from the operations' names.
    #[test]
    fn hand_computed_values() {
        use crate::exec::Quiet;
        let mut rt = Runtime::new(std::sync::Arc::new(crate::Graph::default()));
        let mut run = |o: Guid, res: Guid, p1: Guid, p2: Guid, a: Value, b: Value| {
            super::run(&mut rt, &mut Quiet, o, res, p1, p2, &a, &b, false)
        };
        let (i, f, s, v) = (
            Value::Int,
            Value::Float,
            |x: &str| Value::Str(x.into()),
            Value::Vec3,
        );
        let n = Value::None;
        let cases: Vec<(Option<Value>, Option<Value>)> = vec![
            (
                run(op::ADD, INT, INT, INT, i(i32::MAX), i(1)),
                Some(i(i32::MIN)),
            ),
            (run(op::DIV, INT, INT, INT, i(7), i(0)), Some(i(7))),
            (run(op::DIV, INT, INT, INT, i(-7), i(2)), Some(i(-3))),
            (run(op::MOD, INT, INT, INT, i(-7), i(3)), Some(i(2))),
            (run(op::MOD, INT, INT, INT, i(7), i(-3)), Some(i(1))),
            (run(op::MOD, INT, INT, INT, i(5), i(0)), Some(i(0))),
            (
                run(op::DIV, FLOAT, FLOAT, FLOAT, f(3.0), f(0.0)),
                Some(f(3.0)),
            ),
            (run(op::MUL, INT, FLOAT, INT, f(2.75), i(2)), Some(i(5))),
            (run(op::MUL, FLOAT, FLOAT, INT, f(2.5), i(2)), Some(f(5.0))),
            (
                run(op::ADD, TIME, TIME, TIME, f(16.5), f(0.5)),
                Some(f(17.0)),
            ),
            (
                run(op::CONVERT, INT, FLOAT, NO, f(-2.9), n.clone()),
                Some(i(-2)),
            ),
            (
                run(op::CONVERT, INT, FLOAT, NO, f(3e9), n.clone()),
                Some(i(i32::MIN)),
            ),
            (
                run(op::CONVERT, FLOAT, BOOL, NO, Value::Bool(true), n.clone()),
                Some(f(1.0)),
            ),
            (
                run(op::CONVERT, INT, BOOL, NO, Value::Bool(true), n.clone()),
                Some(i(1)),
            ),
            (
                run(op::CONVERT, STRING, FLOAT, NO, f(0.5), n.clone()),
                Some(s("0.5")),
            ),
            (
                run(op::CONVERT, STRING, BOOL, NO, Value::Bool(false), n.clone()),
                Some(s("FALSE")),
            ),
            (
                run(op::CONVERT, STRING, INT, NO, i(-12), n.clone()),
                Some(s("-12")),
            ),
            (
                run(op::CONVERT, INT, STRING, NO, s(" 31 apples"), n.clone()),
                Some(i(31)),
            ),
            (
                run(op::CONVERT, BOOL, STRING, NO, s("True"), n.clone()),
                Some(Value::Bool(true)),
            ),
            (
                run(op::CONVERT, VECTOR2, FLOAT, FLOAT, f(1.0), f(2.0)),
                Some(Value::Vec2([1.0, 2.0])),
            ),
            (
                run(op::ADD, STRING, STRING, STRING, s("ab"), s("c")),
                Some(s("abc")),
            ),
            (
                run(op::LENGTH, INT, STRING, NO, s("four"), n.clone()),
                Some(i(4)),
            ),
            (
                run(
                    op::ADD,
                    VECTOR,
                    VECTOR,
                    VECTOR,
                    v([1.0, 2.0, 3.0]),
                    v([1.0, 1.0, 1.0]),
                ),
                Some(v([2.0, 3.0, 4.0])),
            ),
            (
                run(op::MUL, VECTOR, VECTOR, FLOAT, v([1.0, -2.0, 0.5]), f(2.0)),
                Some(v([2.0, -4.0, 1.0])),
            ),
            (
                run(op::DIV, VECTOR, VECTOR, FLOAT, v([2.0, 4.0, 6.0]), f(0.0)),
                Some(v([2.0, 4.0, 6.0])),
            ),
            (
                run(
                    op::CROSS,
                    VECTOR,
                    VECTOR,
                    VECTOR,
                    v([1.0, 0.0, 0.0]),
                    v([0.0, 1.0, 0.0]),
                ),
                Some(v([0.0, 0.0, 1.0])),
            ),
            (
                run(
                    op::NORMALIZE,
                    VECTOR,
                    VECTOR,
                    NO,
                    v([3.0, 0.0, 4.0]),
                    n.clone(),
                ),
                Some(v([0.6, 0.0, 0.8])),
            ),
            (
                run(op::NORMALIZE, VECTOR, VECTOR, NO, v([0.0; 3]), n.clone()),
                Some(v([0.0; 3])),
            ),
            (
                run(
                    op::MAGNITUDE,
                    FLOAT,
                    VECTOR,
                    NO,
                    v([3.0, 0.0, 4.0]),
                    n.clone(),
                ),
                Some(f(5.0)),
            ),
            (
                run(op::SET_Y, VECTOR, VECTOR, FLOAT, v([1.0, 2.0, 3.0]), f(9.0)),
                Some(v([1.0, 9.0, 3.0])),
            ),
            (
                run(op::OPPOSITE, INT, INT, NO, i(i32::MIN), n.clone()),
                Some(i(i32::MIN)),
            ),
            (
                run(
                    op::OPPOSITE,
                    VECTOR,
                    VECTOR,
                    NO,
                    v([1.0, -2.0, 0.0]),
                    n.clone(),
                ),
                Some(v([-1.0, 2.0, -0.0])),
            ),
            (
                run(op::ARC_COSINE, FLOAT, FLOAT, NO, f(-1.0), n.clone()),
                Some(f(std::f32::consts::PI)),
            ),
            (
                run(
                    op::GET_W,
                    FLOAT,
                    QUATERNION,
                    NO,
                    Value::Quat([0.1, 0.2, 0.3, 0.4]),
                    n.clone(),
                ),
                Some(f(0.4)),
            ),
            (
                run(
                    op::MAX,
                    VECTOR,
                    BOX,
                    NO,
                    Value::Box([0.0, 1.0, 2.0, 3.0, 4.0, 5.0]),
                    n.clone(),
                ),
                Some(v([3.0, 4.0, 5.0])),
            ),
            (
                run(
                    op::MUL,
                    VECTOR,
                    MATRIX,
                    VECTOR,
                    Value::Matrix([
                        [0.0, 1.0, 0.0, 0.0],
                        [-1.0, 0.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [10.0, 0.0, 0.0, 1.0],
                    ]),
                    v([1.0, 2.0, 3.0]),
                ),
                Some(v([8.0, 1.0, 3.0])),
            ),
            // Degre To Radian: x * 0.017453292
            (
                run(op::DEG_TO_RAD, FLOAT, FLOAT, NO, f(-2.0), n.clone()),
                Some(f(-0.034_906_584)),
            ),
            (
                run(
                    op::INVERSE,
                    MATRIX,
                    MATRIX,
                    NO,
                    Value::Matrix([
                        [0.0, 2.0, 0.0, 0.0],
                        [-2.0, 0.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [4.0, 6.0, 8.0, 1.0],
                    ]),
                    n.clone(),
                ),
                Some(Value::Matrix([
                    [0.0, -0.5, 0.0, 0.0],
                    [0.5, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [-3.0, 2.0, -8.0, 1.0],
                ])),
            ),
            (
                run(
                    op::INVERSE,
                    MATRIX,
                    MATRIX,
                    NO,
                    Value::Matrix([[0.0; 4]; 4]),
                    n.clone(),
                ),
                None,
            ),
            // Abs, Min, Max (the clamp of Landfill in the Clouds), Distance and the equality tests
            (
                run(op::ABS, FLOAT, FLOAT, NO, f(-2.5), n.clone()),
                Some(f(2.5)),
            ),
            (run(op::ABS, INT, INT, NO, i(-7), n.clone()), Some(i(7))),
            (
                run(op::ABS, INT, INT, NO, i(i32::MIN), n.clone()),
                Some(i(i32::MIN)),
            ),
            (
                run(op::MIN, FLOAT, FLOAT, FLOAT, f(3.0), f(-1.0)),
                Some(f(-1.0)),
            ),
            (run(op::MIN, INT, INT, INT, i(3), i(9)), Some(i(3))),
            (
                run(op::MAXIMUM, FLOAT, FLOAT, FLOAT, f(3.0), f(-1.0)),
                Some(f(3.0)),
            ),
            (
                run(op::MAXIMUM, TIME, TIME, TIME, f(1.5), f(1000.0)),
                Some(f(1000.0)),
            ),
            (
                run(
                    op::DISTANCE,
                    FLOAT,
                    VECTOR,
                    VECTOR,
                    v([1.0, 2.0, 3.0]),
                    v([4.0, 6.0, 3.0]),
                ),
                Some(f(5.0)),
            ),
            (
                run(
                    op::CONTAIN_STRING,
                    BOOL,
                    STRING,
                    STRING,
                    s("PE_Balloon_01"),
                    s("Balloon"),
                ),
                Some(Value::Bool(true)),
            ),
            (
                run(
                    op::CONTAIN_STRING,
                    BOOL,
                    STRING,
                    STRING,
                    s("PE_Balloon"),
                    s("balloon"),
                ),
                Some(Value::Bool(false)),
            ),
            (
                run(op::CONTAIN_STRING, BOOL, STRING, STRING, s("PE"), s("")),
                Some(Value::Bool(true)),
            ),
            (
                run(
                    op::EQUAL,
                    BOOL,
                    STRING,
                    STRING,
                    s("PE_Balloon"),
                    s("PE_Balloon"),
                ),
                Some(Value::Bool(true)),
            ),
            (
                run(
                    op::EQUAL,
                    BOOL,
                    STRING,
                    STRING,
                    s("PE_Balloon"),
                    s("pe_balloon"),
                ),
                Some(Value::Bool(false)),
            ),
            (
                run(op::EQUAL, BOOL, FLOAT, FLOAT, f(0.0), f(-0.0)),
                Some(Value::Bool(false)),
            ),
            (
                run(op::EQUAL, BOOL, INT, FLOAT, i(3), f(3.0)),
                Some(Value::Bool(true)),
            ),
            (
                run(op::EQUAL, BOOL, INT, INT, i(3), i(4)),
                Some(Value::Bool(false)),
            ),
            (
                run(
                    op::EQUAL,
                    BOOL,
                    VECTOR,
                    VECTOR,
                    v([1.0, 2.0, 3.0]),
                    v([1.0, 2.0, 3.0]),
                ),
                Some(Value::Bool(true)),
            ),
            (
                run(
                    op::EQUAL,
                    BOOL,
                    OBJECT,
                    OBJECT,
                    Value::Obj(Some(3)),
                    Value::Obj(None),
                ),
                Some(Value::Bool(false)),
            ),
            (run(op::XOR, INT, INT, INT, i(6), i(3)), Some(i(5))),
            (
                run(
                    op::XOR,
                    BOOL,
                    BOOL,
                    BOOL,
                    Value::Bool(true),
                    Value::Bool(true),
                ),
                Some(Value::Bool(false)),
            ),
            (
                run(
                    op::SET_ALPHA,
                    COLOR,
                    COLOR,
                    FLOAT,
                    Value::Color([0.1, 0.2, 0.3, 0.4]),
                    f(2.0),
                ),
                Some(Value::Color([0.1, 0.2, 0.3, 1.0])),
            ),
            (
                run(
                    op::SET_ALPHA,
                    COLOR,
                    COLOR,
                    FLOAT,
                    Value::Color([0.1, 0.2, 0.3, 0.4]),
                    f(-0.5),
                ),
                Some(Value::Color([0.1, 0.2, 0.3, -0.5])),
            ),
            (
                run(
                    op::DIV,
                    QUATERNION,
                    QUATERNION,
                    QUATERNION,
                    Value::Quat([1.0, 0.0, 0.0, 0.0]),
                    Value::Quat([0.0, 1.0, 0.0, 0.0]),
                ),
                Some(Value::Quat([0.0, 0.0, 1.0, 0.0])),
            ),
            (run(op::WIDTH, INT, NO, NO, n.clone(), n.clone()), None),
        ];
        for (k, (got, want)) in cases.into_iter().enumerate() {
            assert_eq!(got, want, "case {k}");
        }
    }

    #[test]
    fn degrees_to_radians() {
        use crate::exec::Quiet;
        let mut rt = Runtime::new(std::sync::Arc::new(crate::Graph::default()));
        let got = super::run(
            &mut rt,
            &mut Quiet,
            op::DEG_TO_RAD,
            FLOAT,
            FLOAT,
            NO,
            &Value::Float(180.0),
            &Value::None,
            false,
        );
        assert!((got.unwrap().as_float() - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn the_class_tree() {
        // 3D object (41) -> 3D entity (33) -> render object (47) -> be object (19) -> scene object (11) -> object (1)
        assert!(class_derives(41, 33) && class_derives(41, 1) && class_derives(41, 41));
        assert!(class_derives(35, 34)); // target camera is a camera
        assert!(!class_derives(33, 41)); // not the other way
        assert!(!class_derives(27, 33)); // a 2D entity is no 3D entity
        assert!(class_derives(29, 27)); // sprite text is a 2D entity
        assert!(!class_derives(31, 32)); // texture and mesh are siblings
    }

    #[test]
    fn every_row_is_one_signature() {
        for (i, a) in TABLE.iter().enumerate() {
            assert!(
                TABLE
                    .iter()
                    .skip(i + 1)
                    .all(|b| (b.op, b.res, b.p1, b.p2) != (a.op, a.res, a.p1, a.p2)),
                "{} {} {} {}",
                a.op,
                a.res,
                a.p1,
                a.p2
            );
        }
    }

    #[test]
    fn parents_are_tried_for_the_inputs_and_the_output() {
        // Time + Time -> Time is Float addition; String -> Array is the object string function
        let (row, swap) = find(op::ADD, TIME, TIME, TIME).unwrap();
        assert_eq!((row.res, row.p1, swap), (FLOAT, FLOAT, false));
        let (row, _) = find(op::CONVERT, t::ARRAY, STRING, NO).unwrap();
        assert_eq!(row.res, OBJECT);
        let (row, _) = find(op::NAME, STRING, t::OBJECT3D, NO).unwrap();
        assert_eq!(row.p1, OBJECT);
        // Boolean -> Float through Integer -> Float
        let (row, _) = find(op::CONVERT, FLOAT, BOOL, NO).unwrap();
        assert_eq!(row.p1, INT);
        // Float x Integer -> Float: the Integer x Float row with the inputs swapped
        let (row, swap) = find(op::MUL, FLOAT, FLOAT, INT).unwrap();
        assert_eq!((row.p1, row.p2, swap), (INT, FLOAT, true));
        assert!(find(op::ADD, INT, Guid(1, 2), INT).is_none());
    }

    #[test]
    fn g_formats_as_c_does() {
        for (x, s) in [
            (0.0, "0"),
            (1.0, "1"),
            (-2.5, "-2.5"),
            (0.1, "0.1"),
            (1.0 / 3.0, "0.333333"),
            (123456.0, "123456"),
            (1234567.0, "1.23457e+06"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (999999.5, "1e+06"),
            (16.666666, "16.6667"),
        ] {
            assert_eq!(fmt_g(x), s, "{x}");
        }
    }

    #[test]
    fn scan_int_reads_the_leading_number() {
        assert_eq!(scan_int(" 42abc"), 42);
        assert_eq!(scan_int("-7"), -7);
        assert_eq!(scan_int("x1"), 0);
        assert_eq!(scan_int("99999999999"), i32::MAX);
        assert_eq!(scan_float(" 2.5abc"), 2.5);
        assert_eq!(scan_float("-1e2x"), -100.0);
        assert_eq!(scan_float("1e"), 1.0);
        assert_eq!(scan_float("x"), 0.0);
    }

    #[test]
    fn a_rotation_matrix_gives_its_quaternion() {
        // 90 degrees about Y (column-major, m[c][r]): x axis -> -z
        let m = [
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let q = quat_of(&m);
        let h = std::f32::consts::FRAC_1_SQRT_2;
        for (a, b) in q.iter().zip([0.0, h, 0.0, h]) {
            assert!((a - b).abs() < 1e-6, "{q:?}");
        }
    }
}
