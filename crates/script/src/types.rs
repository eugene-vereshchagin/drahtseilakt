// The parameter types a graph's parameters carry, as the original's parameter manager has them
// (docs/ck-runtime.md, "Parameter types"): GUID → name, kind (how a value is held), parent type (3D Entity is a
// Behavioral Object is an Object; Boolean is an Integer; Time, Angle and Percentage are Floats), size and default.
// The operation table (ops.rs) tries the parents of a signature's types; blocks read values by their kind. A type
// the registry does not know is held as its saved bytes (`Value::Raw`).

use crate::{Guid, Value};

pub const NONE: Guid = Guid(0x1cb1_0760, 0x419f_50c5);
pub const INT: Guid = Guid(0x5a57_16fd, 0x44e2_76d7);
pub const FLOAT: Guid = Guid(0x4788_4c3f, 0x432c_2c20);
pub const BOOL: Guid = Guid(0x1ad5_2a8e, 0x5e74_1920);
pub const STRING: Guid = Guid(0x6bd0_10e2, 0x1156_17ea);
pub const VECTOR: Guid = Guid(0x4882_4eae, 0x2fe4_7960);
pub const VECTOR2: Guid = Guid(0x4efc_b34a, 0x6079_e42f);
pub const MATRIX: Guid = Guid(0x643f_046e, 0x6521_1b71);
pub const COLOR: Guid = Guid(0x57d4_2fee, 0x7cbb_3b91);
pub const TIME: Guid = Guid(0x54b4_422b, 0x730f_0f4f);
pub const ANGLE: Guid = Guid(0x1126_2cf5, 0x30b0_233a);
pub const PERCENTAGE: Guid = Guid(0xf3c8_4b4e, 0x0ffa_cc34);
pub const EULER: Guid = Guid(0x13b0_1b3c, 0x1942_583e);
pub const QUATERNION: Guid = Guid(0x06c4_39ee, 0x45b5_0fc2);
pub const RECT: Guid = Guid(0x7ab2_0d20, 0x6930_44a9);
pub const BOX: Guid = Guid(0x6686_49c8, 0x283e_2ee1);
pub const MESSAGE: Guid = Guid(0x0388_1e12, 0x5ba3_4e2b);
pub const ATTRIBUTE: Guid = Guid(0x3ea3_4ee9, 0x09fa_5366);
pub const CLASS_ID: Guid = Guid(0x1964_4e43, 0x4d6f_6123);
pub const OBJECT_ARRAY: Guid = Guid(0x71df_7142, 0xc437_133a);
pub const OBJECT: Guid = Guid(0x30ec_20ab, 0x6df6_517d);
pub const BEOBJECT: Guid = Guid(0x71d8_0779, 0x402f_42f3);
pub const SCRIPT: Guid = Guid(0x7ea4_176d, 0x1b40_5d30);
pub const ENTITY: Guid = Guid(0x5b8a_05d5, 0x31ea_28d4);
pub const OBJECT3D: Guid = Guid(0x362e_4df8, 0x1744_3539);
pub const CAMERA: Guid = Guid(0x3cf2_4d6f, 0x2162_04f9);
pub const LIGHT: Guid = Guid(0x4b6d_412f, 0x5d1e_1416);
pub const ENTITY2D: Guid = Guid(0x1816_71e3, 0x1bdc_1503);
pub const MATERIAL: Guid = Guid(0x55ab_12cd, 0x22ae_6a8b);
pub const TEXTURE: Guid = Guid(0x155b_2870, 0x1836_79f8);
pub const MESH: Guid = Guid(0x2453_5345, 0x65d1_5229);
pub const GROUP: Guid = Guid(0x5c0f_151b, 0x6f05_47fd);
pub const ARRAY: Guid = Guid(0x024d_52f1, 0x6782_23b2);
pub const SOUND: Guid = Guid(0x4019_4410, 0x3a77_3f80);
pub const WAVE_SOUND: Guid = Guid(0x4bf7_4e5e, 0x45f4_09ef);
pub const LEVEL: Guid = Guid(0x1058_4787, 0x7693_2f77);

/// How a type's value is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    None,
    Int,
    Float,
    Bool,
    Str,
    Vector,
    Vector2,
    Quat,
    Matrix,
    Color,
    Rect,
    Box,
    /// A message type, held by name.
    Message,
    /// An attribute type, held by name.
    Attribute,
    /// An object of this class or one derived from it (CKCID).
    Object(u32),
    ObjectArray,
    /// A number naming one of a set (a class id, an enumeration): held as an Integer.
    Enum,
}

/// One registered type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeInfo {
    pub guid: Guid,
    pub name: &'static str,
    pub kind: Kind,
    pub parent: Option<Guid>,
    /// The size of its value in the original, in bytes (a string's is its buffer's at creation).
    pub size: u32,
}

const fn t(
    guid: Guid,
    name: &'static str,
    kind: Kind,
    parent: Option<Guid>,
    size: u32,
) -> TypeInfo {
    TypeInfo {
        guid,
        name,
        kind,
        parent,
        size,
    }
}

const fn obj(guid: Guid, name: &'static str, parent: Guid, class: u32) -> TypeInfo {
    t(guid, name, Kind::Object(class), Some(parent), 4)
}

/// Every type the runtime knows.
pub const TYPES: &[TypeInfo] = &[
    t(NONE, "None", Kind::None, None, 0),
    t(INT, "Integer", Kind::Int, None, 4),
    t(BOOL, "Boolean", Kind::Bool, Some(INT), 4),
    t(CLASS_ID, "Class ID", Kind::Enum, Some(INT), 4),
    t(FLOAT, "Float", Kind::Float, None, 4),
    t(TIME, "Time", Kind::Float, Some(FLOAT), 4),
    t(ANGLE, "Angle", Kind::Float, Some(FLOAT), 4),
    t(PERCENTAGE, "Percentage", Kind::Float, Some(FLOAT), 4),
    t(STRING, "String", Kind::Str, None, 64),
    t(VECTOR, "Vector", Kind::Vector, None, 12),
    t(EULER, "Euler Angles", Kind::Vector, Some(VECTOR), 12),
    t(VECTOR2, "2D Vector", Kind::Vector2, None, 8),
    t(QUATERNION, "Quaternion", Kind::Quat, None, 16),
    t(MATRIX, "Matrix", Kind::Matrix, None, 64),
    t(COLOR, "Color", Kind::Color, None, 16),
    t(RECT, "Rectangle", Kind::Rect, None, 16),
    t(BOX, "Box", Kind::Box, None, 24),
    t(MESSAGE, "Message", Kind::Message, None, 4),
    t(ATTRIBUTE, "Attribute", Kind::Attribute, None, 4),
    t(OBJECT_ARRAY, "Object Array", Kind::ObjectArray, None, 0),
    t(OBJECT, "Object", Kind::Object(1), None, 4),
    obj(BEOBJECT, "Behavioral Object", OBJECT, 19),
    obj(SCRIPT, "Script", OBJECT, 8),
    obj(ENTITY, "3D Entity", BEOBJECT, 33),
    obj(OBJECT3D, "3D Object", ENTITY, 41),
    obj(CAMERA, "Camera", ENTITY, 34),
    obj(LIGHT, "Light", ENTITY, 38),
    obj(ENTITY2D, "2D Entity", BEOBJECT, 27),
    obj(MATERIAL, "Material", BEOBJECT, 30),
    obj(TEXTURE, "Texture", BEOBJECT, 31),
    obj(MESH, "Mesh", BEOBJECT, 32),
    obj(GROUP, "Group", BEOBJECT, 23),
    obj(ARRAY, "Array", BEOBJECT, 52),
    obj(SOUND, "Sound", BEOBJECT, 24),
    obj(WAVE_SOUND, "Wave Sound", SOUND, 25),
    obj(LEVEL, "Level", BEOBJECT, 21),
];

/// The registered type of a GUID.
pub fn info(g: Guid) -> Option<&'static TypeInfo> {
    TYPES.iter().find(|t| t.guid == g)
}

pub fn kind(g: Guid) -> Option<Kind> {
    info(g).map(|t| t.kind)
}

/// The type a type derives from; none for a root type or one not registered.
pub fn parent(g: Guid) -> Option<Guid> {
    info(g).and_then(|t| t.parent)
}

/// Whether `g` is `base` or derives from it (the chain is short; a cycle cannot occur in the table, and the walk
/// is bounded anyway).
pub fn derives(g: Guid, base: Guid) -> bool {
    let mut at = Some(g);
    for _ in 0..16 {
        match at {
            Some(x) if x == base => return true,
            Some(x) => at = parent(x),
            None => return false,
        }
    }
    false
}

/// The type's name, or its GUID when it is not registered.
pub fn name(g: Guid) -> String {
    info(g).map_or_else(|| g.to_string(), |t| t.name.to_string())
}

/// Whether the type holds an object reference.
pub fn is_object(g: Guid) -> bool {
    matches!(kind(g), Some(Kind::Object(_)))
}

/// The class a Virtools class derives from (CKCID values; docs/ck-runtime.md, "Parameter types"): the object classes
/// a graph names. 0 is no class: the game's pseudo-objects (`CurrentLevel`...), which any class accepts.
pub fn class_parent(cid: u32) -> Option<u32> {
    // (class, parent): Scene Object 11 and the parameters, behaviours, links under Object 1; Behavioral Object 19
    // under Scene Object; Render Object 47 under it; 2D Entity 27 and 3D Entity 33 under Render Object.
    const PARENTS: &[(u32, u32)] = &[
        (2, 1),
        (3, 1),
        (4, 1),
        (6, 1),
        (8, 1),
        (9, 1),
        (11, 1),
        (45, 1),
        (19, 11),
        (10, 19),
        (21, 19),
        (23, 19),
        (24, 19),
        (30, 19),
        (31, 19),
        (32, 19),
        (47, 19),
        (52, 19),
        (25, 24),
        (26, 24),
        (27, 47),
        (33, 47),
        (28, 27),
        (29, 28),
        (22, 33),
        (34, 33),
        (36, 33),
        (37, 33),
        (38, 33),
        (41, 33),
        (43, 33),
        (50, 33),
        (35, 34),
        (39, 38),
        (40, 41),
        (53, 32),
    ];
    PARENTS.iter().find(|(c, _)| *c == cid).map(|&(_, p)| p)
}

/// Whether an object of class `cid` is of class `base` (a pseudo-object of class 0 is of every class).
pub fn class_derives(cid: u32, base: u32) -> bool {
    if cid == 0 || base == 1 {
        return true;
    }
    let mut at = Some(cid);
    for _ in 0..16 {
        match at {
            Some(c) if c == base => return true,
            Some(c) => at = class_parent(c),
            None => return false,
        }
    }
    false
}

/// A new parameter's value: zero, empty, identity, no object.
pub fn default(g: Guid) -> Value {
    match kind(g) {
        None | Some(Kind::None) => Value::None,
        Some(Kind::Int | Kind::Enum) => Value::Int(0),
        Some(Kind::Bool) => Value::Bool(false),
        Some(Kind::Float) => Value::Float(0.0),
        Some(Kind::Str | Kind::Message | Kind::Attribute) => Value::Str(String::new()),
        Some(Kind::Vector) => Value::Vec3([0.0; 3]),
        Some(Kind::Vector2) => Value::Vec2([0.0; 2]),
        Some(Kind::Quat) => Value::Quat([0.0, 0.0, 0.0, 1.0]),
        Some(Kind::Matrix) => Value::Matrix(std::array::from_fn(|c| {
            std::array::from_fn(|r| if r == c { 1.0 } else { 0.0 })
        })),
        Some(Kind::Color) => Value::Color([0.0; 4]),
        Some(Kind::Rect) => Value::Rect([0.0; 4]),
        Some(Kind::Box) => Value::Box([0.0; 6]),
        Some(Kind::Object(_)) => Value::Obj(None),
        Some(Kind::ObjectArray) => Value::Objects(Vec::new()),
    }
}

#[cfg(test)]
#[allow(clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn parents_chain_to_the_root() {
        assert!(derives(OBJECT3D, ENTITY));
        assert!(derives(OBJECT3D, BEOBJECT));
        assert!(derives(OBJECT3D, OBJECT));
        assert!(derives(WAVE_SOUND, BEOBJECT));
        assert!(derives(BOOL, INT));
        assert!(derives(TIME, FLOAT));
        assert!(!derives(FLOAT, TIME));
        assert!(!derives(ENTITY, MESH));
        assert!(!derives(Guid(1, 2), OBJECT));
        assert_eq!(parent(ENTITY), Some(BEOBJECT));
        assert_eq!(parent(OBJECT), None);
        assert!(class_derives(41, 33));
        assert!(class_derives(35, 19));
        assert!(class_derives(29, 27));
        assert!(class_derives(0, 32));
        assert!(!class_derives(32, 33));
        assert!(!class_derives(33, 32));
    }

    #[test]
    fn the_table_has_each_type_once_and_every_parent_registered() {
        for (i, a) in TYPES.iter().enumerate() {
            assert!(
                TYPES.iter().skip(i + 1).all(|b| b.guid != a.guid),
                "{}",
                a.name
            );
            if let Some(p) = a.parent {
                assert!(info(p).is_some(), "parent of {}", a.name);
            }
        }
    }

    #[test]
    fn defaults_follow_the_kind() {
        assert_eq!(default(INT), Value::Int(0));
        assert_eq!(default(TIME), Value::Float(0.0));
        assert_eq!(default(MESH), Value::Obj(None));
        assert_eq!(default(QUATERNION), Value::Quat([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(default(Guid(7, 7)), Value::None);
        let Value::Matrix(m) = default(MATRIX) else {
            panic!()
        };
        assert_eq!(m[2][2], 1.0);
        assert_eq!(m[3][2], 0.0);
    }
}
