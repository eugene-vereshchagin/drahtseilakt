// The physics of the original's modules (PH/P_Modul_NN.nmo, PH/PE_Balloon.nmo): which of a module's objects its
// scripts physicalize, with which values and collision meshes, and the joints they make, as the C game makes them
// (docs/physics.md, the PH module table). The modules' scripted behaviour (forces, timers) is not here; a module
// without an entry has no physics at all in the original, so its placeholder is not solid either.

use crate::physics::Material;

/// One physicalized object of a module: its name in the module file, its values, and its convex collision meshes
/// (meshes of the module file, in the object's frame, scaled by its scale).
#[derive(Debug, Clone, Copy)]
pub struct PartSpec {
    pub object: &'static str,
    pub fixed: bool,
    pub material: Material,
    pub group: &'static str,
    /// Made asleep: it moves once something touches it.
    pub start_frozen: bool,
    /// In the collision system at all (a rope or a balloon is not).
    pub collide: bool,
    /// The mass centre in the object's frame (the scripts never use "Auto Mass Center").
    pub shift: [f32; 3],
    pub meshes: &'static [&'static str],
}

/// A joint a module's scripts make, between two of its parts or a part and the fixed anchor FixCube (`a` is the
/// script's target, `b` its Object2). Points and axes come from the module's frame objects, at their places in the
/// level: a slider along the axis from `from` to `to`; a hinge at `frame` about its Z axis; a ball joint at `pos` in
/// `frame`; a spring between `pos_a` in `frame_a` and `pos_b` in `frame_b`.
#[derive(Debug, Clone, Copy)]
pub enum JointSpec {
    Slider {
        a: &'static str,
        b: &'static str,
        from: &'static str,
        to: &'static str,
        limits: bool,
        lo: f32,
        hi: f32,
    },
    Hinge {
        a: &'static str,
        b: &'static str,
        frame: &'static str,
    },
    Ball {
        a: &'static str,
        b: &'static str,
        frame: &'static str,
        pos: [f32; 3],
    },
    Spring {
        a: &'static str,
        pos_a: [f32; 3],
        frame_a: &'static str,
        b: &'static str,
        pos_b: [f32; 3],
        frame_b: &'static str,
        length: f32,
        constant: f32,
        lin_damp: f32,
        glob_damp: f32,
    },
}

/// The fixed anchor of the modules' joints.
pub const FIXCUBE: &str = "FixCube";

/// A module's wake-up on approach: its script starts a TT Scaleable Proximity between the ball's position and
/// `near` (a part, or the module itself when it names the module's root `_MF` object) once its joints are made, and
/// when the ball comes within `distance` runs Physics WakeUp on `part`, once. A part made asleep would otherwise stay
/// asleep with nothing touching it (the stone of module 34 above its box).
#[derive(Debug, Clone, Copy)]
pub struct WakeSpec {
    pub part: &'static str,
    pub near: &'static str,
    pub distance: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleSpec {
    pub name: &'static str,
    pub parts: &'static [PartSpec],
    pub joints: &'static [JointSpec],
    pub wake: Option<WakeSpec>,
}

const fn material(friction: f32, elasticity: f32, mass: f32, lin_damp: f32, rot_damp: f32) -> Material {
    Material {
        friction,
        elasticity,
        mass,
        lin_damp,
        rot_damp,
    }
}

/// Every module with physics in levels 1 to 12, as the C game physicalizes them at a level's load (its PHYSICALIZE,
/// SLIDER, HINGE, BALLJOINT and SPRING records, printed from its building blocks). The finish (PE_Balloon) is not
/// here: its balloons are held up by its script, and without it they would pull its bridge down under the ball.
const MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        name: "P_Modul_01",
        parts: &[
            PartSpec {
                object: "P_Modul_01_Rinne",
                fixed: true,
                material: material(0.7, 0.4, 1.0, 0.1, 0.1),
                group: "Ball",
                start_frozen: false,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_01_Rinne_01_Mesh", "P_Modul_01_Rinne_02_Mesh", "P_Modul_01_Rinne_03_Mesh"],
            },
            PartSpec {
                object: "P_Modul_01_Filler",
                fixed: true,
                material: material(0.7, 0.4, 1.0, 0.1, 0.1),
                group: "Floor",
                start_frozen: false,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_01_Filler_Mesh"],
            },
            PartSpec {
                object: "P_Modul_01_Pusher",
                fixed: false,
                material: material(0.6, 0.4, 3.0, 0.1, 1.0),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_01_Col01_Mesh", "P_Modul_01_Col02_Mesh", "P_Modul_01_Col03_Mesh"],
            },
        ],
        joints: &[
        ],
        wake: Some(WakeSpec { part: "P_Modul_01_Pusher", near: "P_Modul_01_Pusher", distance: 50.0 }),
    },
    ModuleSpec {
        name: "P_Modul_03",
        parts: &[
            PartSpec {
                object: "P_Modul_03_Gate",
                fixed: false,
                material: material(0.4, 0.0, 2.0, 0.5, 1.0),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Gate_Coll01_Mesh", "P_Modul_03_Gate_Coll02_Mesh", "P_Modul_03_Gate_Coll03_Mesh"],
            },
            PartSpec {
                object: "P_Modul_03_Wall01",
                fixed: false,
                material: material(0.4, 0.01, 2.0, 0.5, 1.5),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Wall_Coll_Mesh"],
            },
            PartSpec {
                object: "P_Modul_03_Wall02",
                fixed: false,
                material: material(0.4, 0.01, 2.0, 0.5, 1.5),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Wall_Coll_Mesh"],
            },
            PartSpec {
                object: "P_Modul_03_Wall03",
                fixed: false,
                material: material(0.4, 0.01, 2.0, 0.5, 1.5),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Wall_Coll_Mesh"],
            },
            PartSpec {
                object: "P_Modul_03_Wall04",
                fixed: false,
                material: material(0.4, 0.01, 2.0, 0.5, 1.5),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Wall_Coll_Mesh"],
            },
            PartSpec {
                object: "P_Modul_03_Wall05",
                fixed: false,
                material: material(0.4, 0.01, 2.0, 0.5, 1.5),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Wall_Coll_Mesh"],
            },
            PartSpec {
                object: "P_Modul_03_Wall06",
                fixed: false,
                material: material(0.4, 0.01, 2.0, 0.5, 1.5),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Wall_Coll_Mesh"],
            },
            PartSpec {
                object: "P_Modul_03_Wall07",
                fixed: false,
                material: material(0.4, 0.01, 2.0, 0.5, 1.5),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Wall_Coll_Mesh"],
            },
            PartSpec {
                object: "P_Modul_03_Floor",
                fixed: false,
                material: material(0.7, 0.0, 3.0, 1.0, 3.0),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_03_Floor_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Slider { a: "P_Modul_03_Floor", b: "FixCube", from: "P_Modul_03_frame_low", to: "P_Modul_03_frame_high", limits: false, lo: -1.0, hi: 1.0 },
            JointSpec::Spring { a: "P_Modul_03_Floor", pos_a: [0.0, 0.0, 0.0], frame_a: "P_Modul_03_frame_low", b: "FixCube", pos_b: [0.0, 0.0, 0.0], frame_b: "P_Modul_03_frame_high", length: 0.0, constant: 15.0, lin_damp: 0.1, glob_damp: 1.0 },
        ],
        wake: Some(WakeSpec { part: "P_Modul_03_Floor", near: "P_Modul_03_MF", distance: 35.0 }),
    },
    ModuleSpec {
        name: "P_Modul_08",
        parts: &[
            PartSpec {
                object: "P_Modul_08_Schaukel",
                fixed: false,
                material: material(0.7, 0.4, 10.0, 0.4, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_08_Col1_Mesh", "P_Modul_08_Col2_Mesh", "P_Modul_08_Col3_Mesh", "P_Modul_08_Col4_Mesh", "P_Modul_08_Col5_Mesh", "P_Modul_08_Col6_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Hinge { a: "P_Modul_08_Schaukel", b: "FixCube", frame: "P_Modul_08_HingeFrame" },
        ],
        wake: None,
    },
    ModuleSpec {
        name: "P_Modul_17",
        parts: &[
            PartSpec {
                object: "Modul17_Dreharme",
                fixed: false,
                material: material(0.7, 0.4, 3.0, 3.0, 0.005),
                group: "Floor",
                start_frozen: false,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_17_Col01_Mesh", "P_Modul_17_Col02_Mesh", "P_Modul_17_Col03_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Hinge { a: "Modul17_Dreharme", b: "FixCube", frame: "P_Modul_17_HingeFrame" },
            JointSpec::Spring { a: "Modul17_Dreharme", pos_a: [0.0, 4.0, 0.0], frame_a: "P_Modul_17_HingeFrame", b: "FixCube", pos_b: [0.0, 0.0, -4.0], frame_b: "Modul17_Dreharme", length: 0.0, constant: 0.32, lin_damp: 0.1, glob_damp: 0.1 },
        ],
        wake: None,
    },
    ModuleSpec {
        name: "P_Modul_19",
        parts: &[
            PartSpec {
                object: "P_Modul_19_Flaps",
                fixed: false,
                material: material(0.7, 0.4, 3.0, 1.0, 0.05),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [1.0, 1.0, 0.0],
                meshes: &["P_Modul_19_Col1_Mesh", "P_Modul_19_Col2_Mesh", "P_Modul_19_Col3_Mesh", "P_Modul_19_Col4_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Hinge { a: "P_Modul_19_Flaps", b: "FixCube", frame: "P_Modul_19_HingeFrame" },
        ],
        wake: Some(WakeSpec { part: "P_Modul_19_Flaps", near: "P_Modul_19_MF", distance: 50.0 }),
    },
    ModuleSpec {
        name: "P_Modul_25",
        parts: &[
            PartSpec {
                object: "P_Modul_25_Bridge",
                fixed: false,
                material: material(0.7, 1.0, 3.0, 1.0, 0.05),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [-2.5, 0.2, 0.0],
                meshes: &["P_Modul_25_Col01_Mesh", "P_Modul_25_Col02_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Hinge { a: "P_Modul_25_Bridge", b: "FixCube", frame: "P_Modul_25_HingeFrame" },
        ],
        wake: Some(WakeSpec { part: "P_Modul_25_Bridge", near: "P_Modul_25_MF", distance: 50.0 }),
    },
    ModuleSpec {
        name: "P_Modul_26",
        parts: &[
            PartSpec {
                object: "P_Modul_26_Sack",
                fixed: false,
                material: material(0.7, 0.4, 10.0, 0.1, 0.1),
                group: "Floor",
                start_frozen: false,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_26_Sack_Mesh"],
            },
            PartSpec {
                object: "P_Modul_26_Rope",
                fixed: false,
                material: material(0.7, 0.4, 1.0, 0.1, 0.1),
                group: "",
                start_frozen: false,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_26_Rope_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Ball { a: "P_Modul_26_Rope", b: "FixCube", frame: "P_Modul_26_Balljoint_oben", pos: [0.0, 0.0, 0.0] },
            JointSpec::Ball { a: "P_Modul_26_Rope", b: "P_Modul_26_Sack", frame: "P_Modul_26_Balljoint_unten", pos: [0.0, 0.0, 0.0] },
        ],
        wake: None,
    },
    ModuleSpec {
        name: "P_Modul_29",
        parts: &[
            PartSpec {
                object: "P_Modul_29_Platte01",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte01_Mesh"],
            },
            PartSpec {
                object: "P_Modul_29_Platte02",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte02_Mesh"],
            },
            PartSpec {
                object: "P_Modul_29_Platte03",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte03_Mesh"],
            },
            PartSpec {
                object: "P_Modul_29_Platte04",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte04_Mesh"],
            },
            PartSpec {
                object: "P_Modul_29_Platte05",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte05_Mesh"],
            },
            PartSpec {
                object: "P_Modul_29_Platte06",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte06_Mesh"],
            },
            PartSpec {
                object: "P_Modul_29_Platte07",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte07_Mesh"],
            },
            PartSpec {
                object: "P_Modul_29_Platte08",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte08_Mesh"],
            },
            PartSpec {
                object: "P_Modul_29_Platte09",
                fixed: false,
                material: material(0.7, 0.4, 1.0, 0.1, 0.3),
                group: "Modul29",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_29_Platte09_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Hinge { a: "P_Modul_29_Platte01", b: "FixCube", frame: "P_Modul_29_HingeFrame01" },
            JointSpec::Hinge { a: "P_Modul_29_Platte02", b: "P_Modul_29_Platte01", frame: "P_Modul_29_HingeFrame02" },
            JointSpec::Hinge { a: "P_Modul_29_Platte03", b: "P_Modul_29_Platte02", frame: "P_Modul_29_HingeFrame03" },
            JointSpec::Hinge { a: "P_Modul_29_Platte04", b: "P_Modul_29_Platte03", frame: "P_Modul_29_HingeFrame04" },
            JointSpec::Hinge { a: "P_Modul_29_Platte05", b: "P_Modul_29_Platte04", frame: "P_Modul_29_HingeFrame05" },
            JointSpec::Hinge { a: "P_Modul_29_Platte06", b: "P_Modul_29_Platte05", frame: "P_Modul_29_HingeFrame06" },
            JointSpec::Hinge { a: "P_Modul_29_Platte08", b: "P_Modul_29_Platte07", frame: "P_Modul_29_HingeFrame08" },
            JointSpec::Hinge { a: "P_Modul_29_Platte09", b: "P_Modul_29_Platte08", frame: "P_Modul_29_HingeFrame09" },
            JointSpec::Hinge { a: "P_Modul_29_Platte09", b: "FixCube", frame: "P_Modul_29_HingeFrame10" },
            JointSpec::Hinge { a: "P_Modul_29_Platte07", b: "P_Modul_29_Platte06", frame: "P_Modul_29_HingeFrame07" },
        ],
        wake: Some(WakeSpec { part: "P_Modul_29_Platte04", near: "P_Modul_29_MF", distance: 80.0 }),
    },
    ModuleSpec {
        name: "P_Modul_30",
        parts: &[
            PartSpec {
                object: "P_Modul_30_Wippe",
                fixed: false,
                material: material(0.7, 0.4, 3.0, 1.0, 1.0),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 4.0, 0.0],
                meshes: &["P_Modul_30_Col1_Mesh", "P_Modul_30_Col2_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Hinge { a: "P_Modul_30_Wippe", b: "FixCube", frame: "P_Modul_30_HingeFrame" },
        ],
        wake: Some(WakeSpec { part: "P_Modul_30_Wippe", near: "P_Modul_30_MF", distance: 50.0 }),
    },
    ModuleSpec {
        name: "P_Modul_34",
        parts: &[
            PartSpec {
                object: "P_Modul_34_Kiste",
                fixed: false,
                material: material(0.8, 0.4, 1.4, 0.1, 0.1),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_34_Kiste_Mesh"],
            },
            PartSpec {
                object: "P_Modul_34_Schiebestein",
                fixed: false,
                material: material(0.5, 0.4, 1.6, 0.1, 0.1),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_34_Schiebestein_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Slider { a: "P_Modul_34_Schiebestein", b: "FixCube", from: "P_Modul_34_Slider_Frame01", to: "P_Modul_34_Slider_Frame02", limits: false, lo: -1.0, hi: 1.0 },
        ],
        wake: Some(WakeSpec { part: "P_Modul_34_Schiebestein", near: "P_Modul_34_Schiebestein", distance: 50.0 }),
    },
    ModuleSpec {
        name: "P_Modul_37",
        parts: &[
            PartSpec {
                object: "P_Modul_37_Bridge",
                fixed: false,
                material: material(0.7, 1.0, 3.0, 1.0, 0.05),
                group: "",
                start_frozen: true,
                collide: true,
                shift: [-7.5, 0.0, 0.0],
                meshes: &["P_Modul_37_Col1_Mesh", "P_Modul_37_Col2_Mesh", "P_Modul_37_Col3_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Hinge { a: "P_Modul_37_Bridge", b: "FixCube", frame: "P_Modul_37_HingeFrame" },
        ],
        wake: Some(WakeSpec { part: "P_Modul_37_Bridge", near: "P_Modul_37_MF", distance: 50.0 }),
    },
    ModuleSpec {
        name: "P_Modul_41",
        parts: &[
            PartSpec {
                object: "P_Modul_41",
                fixed: false,
                material: material(0.7, 0.4, 1.0, 0.1, 0.1),
                group: "Floor",
                start_frozen: false,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["P_Modul_41_Col1_Mesh", "P_Modul_41_Col2_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Hinge { a: "P_Modul_41", b: "FixCube", frame: "P_Modul_41_HingeFrame" },
        ],
        wake: None,
    },
];

/// The physics of a module, by its name (`P_Modul_34`), if the port has it.
pub fn spec(name: &str) -> Option<&'static ModuleSpec> {
    MODULES.iter().find(|m| m.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_is_found_by_name_and_its_slider_names_a_part() {
        let m = spec("p_modul_34").expect("module 34");
        let named = |n: &str| n == FIXCUBE || m.parts.iter().any(|p| p.object == n);
        assert!(m.joints.iter().all(|j| match j {
            JointSpec::Slider { a, b, .. } | JointSpec::Hinge { a, b, .. } | JointSpec::Ball { a, b, .. } => {
                named(a) && named(b)
            }
            JointSpec::Spring { a, b, .. } => named(a) && named(b),
        }));
        assert!(spec("P_Modul_99").is_none());
    }
}
