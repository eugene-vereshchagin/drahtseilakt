// Blocks whose prototype GUID is in none of the player's libraries (docs/drahtseilakt/scripts.md, "Not in the
// player"): the original has no function for them, so a map's block of this kind never does anything there. They
// are declared so a map using them is not reported as unknown, and run as named no-ops: inputs cleared, no
// output, reported once. Object Rename (this GUID; the player's Narratives.dll has another block of that name)
// keeps the effect its author meant behind the `absent-as-authored` feature, for a later optional mod; the
// default build follows the original.

use super::*;

/// Known, and nothing happens: the inputs are taken so the graph does not spin on the block.
pub(crate) fn no_op(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let mut any = false;
    for i in 0..b.inputs.len() {
        any |= input(rt, b, i);
    }
    if any {
        rt.report_once(
            proto,
            host,
            &format!(
                "script block not in the player's libraries (does nothing, as in the original): {}",
                b.name
            ),
        );
    }
    Flow::Done
}

/// Object Rename as its author meant it: the object answers to the new name from then on.
#[cfg(feature = "absent-as-authored")]
pub(crate) fn object_rename(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let o = pin(rt, b, "Object", 0, host).as_obj();
        let name = pin(rt, b, "Name", 1, host).as_string();
        if let Some(o) = o {
            rt.rename(o, &name);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

#[cfg(feature = "absent-as-authored")]
const RENAME: Run = object_rename;
#[cfg(not(feature = "absent-as-authored"))]
const RENAME: Run = no_op;

/// A named no-op's declaration: its interface is not known (the player never had it).
const fn absent(guid: Guid, name: &'static str) -> Proto {
    Proto {
        guid,
        name,
        library: Library::NotInPlayer,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::NoOp,
        layouts: &[],
        run: no_op,
        event: None,
    }
}

pub(super) const PROTOS: &[Proto] = &[
    absent(Guid(0x002d_010a, 0x003d_010a), "Explode"),
    absent(Guid(0x843c_b43a, 0xa12d_ac48), "Midi Player"),
    // Virtools 3.5+ scripting: one map's editor-time texture tool (blocks named Renamer, Is Same Texture...)
    absent(Guid(0x021e_6fa7, 0x6fe7_4610), "Run VSL"),
    absent(Guid(0x7a1d_593e, 0x12c5_648d), "Add Mesh"),
    absent(Guid(0x0002_145e, 0x6e1b_2592), "Remove Mesh"),
    absent(Guid(0x4389_0176, 0x58b2_244e), "Select Mesh"),
    absent(Guid(0x4321_abcd, 0x0eea_abdd), "Get Nearest Object"),
    Proto {
        pins: &["Object", "Name"],
        run: RENAME,
        ..absent(proto::OBJECT_RENAME, "Object Rename")
    },
];
