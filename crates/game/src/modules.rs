// The physics of the original's modules (PH/P_Modul_NN.nmo, PH/PE_Balloon.nmo): which of a module's objects its
// scripts physicalize, with which values and collision meshes, and the joints they make, as the C game makes them
// (docs/physics.md, the PH module table), and the parts of their scripts the port runs: forces that start when the ball
// comes near, and the finish's balloon. Other scripted behaviour (timers, the other modules' forces) is not here yet;
// a module without an entry has no physics at all in the original, so its placeholder is not solid either.

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

/// A force a module's script puts on a part (SetPhysicsForce): `value` along `dir` turned by `dir_frame`'s axes and
/// normalised, at `pos` in `pos_frame`, both taken where the frames are when the force is made. The force keeps its
/// world direction; it is an impulse at every PSI while the part is awake (docs/physics.md, 2.2).
#[derive(Debug, Clone, Copy)]
pub struct ForceSpec {
    pub part: &'static str,
    pub pos: [f32; 3],
    pub pos_frame: &'static str,
    pub dir: [f32; 3],
    pub dir_frame: &'static str,
    pub value: f32,
}

/// The finish's trigger (PE_Balloon): when the ball comes within `distance` of part `near` (TT Scaleable Proximity
/// with exactness 5 to 10, every 1 to 10 frames), the script pushes with `force`, breaks the hinges at the frames
/// `breaks` and tells the game the level is finished.
#[derive(Debug, Clone, Copy)]
pub struct FinishSpec {
    pub near: &'static str,
    pub distance: f32,
    pub force: ForceSpec,
    pub breaks: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleSpec {
    pub name: &'static str,
    pub parts: &'static [PartSpec],
    pub joints: &'static [JointSpec],
    pub wake: Option<WakeSpec>,
    /// Forces made when the wake-up fires.
    pub forces: &'static [ForceSpec],
    pub finish: Option<FinishSpec>,
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
/// SLIDER, HINGE, BALLJOINT and SPRING records, printed from its building blocks), and the finish (PE_Balloon) from its
/// script's blocks: its balloons are held up by their forces, which start with its wake-up.
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
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
        forces: &[],
        finish: None,
    },
    ModuleSpec {
        name: "PE_Balloon",
        parts: &[
            PartSpec {
                object: "PE_Balloon_Platform",
                fixed: false,
                material: material(0.7, 0.4, 4.0, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 2.0, 0.0],
                meshes: &["PE_Balloon_Col_01_Mesh", "PE_Balloon_Col_02_Mesh", "PE_Balloon_Col_03_Mesh", "PE_Balloon_Col_04_Mesh", "PE_Balloon_Col_05_Mesh", "PE_Balloon_Col_06_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Platte01",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Platte01_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Platte02",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Platte02_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Platte03",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Platte03_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Platte04",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Platte04_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Platte05",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Platte05_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Platte06",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Platte06_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Platte07",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Platte07_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Platte08",
                fixed: false,
                material: material(0.7, 0.4, 0.5, 0.1, 0.1),
                group: "Floor",
                start_frozen: true,
                collide: true,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Platte08_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Ballon_Seil01",
                fixed: false,
                material: material(0.7, 0.4, 0.2, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Ballon_Seil01_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Ballon_Seil02",
                fixed: false,
                material: material(0.7, 0.4, 0.2, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Ballon_Seil02_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Ballon_Seil03",
                fixed: false,
                material: material(0.7, 0.4, 0.2, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Ballon_Seil03_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Ballon_Seil04",
                fixed: false,
                material: material(0.7, 0.4, 0.2, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Ballon_Seil04_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Ballon01",
                fixed: false,
                material: material(0.7, 0.4, 0.2, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Ballon01_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Ballon02",
                fixed: false,
                material: material(0.7, 0.4, 0.2, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Ballon02_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Ballon03",
                fixed: false,
                material: material(0.7, 0.4, 0.2, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Ballon03_Mesh"],
            },
            PartSpec {
                object: "PE_Balloon_Ballon04",
                fixed: false,
                material: material(0.7, 0.4, 0.2, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["PE_Balloon_Ballon04_Mesh"],
            },
            PartSpec {
                object: "PE_Box_slide",
                fixed: false,
                material: material(0.7, 0.4, 4.0, 1.0, 1.0),
                group: "",
                start_frozen: true,
                collide: false,
                shift: [0.0, 0.0, 0.0],
                meshes: &["Box_slide_Mesh"],
            },
        ],
        joints: &[
            JointSpec::Slider {
                a: "PE_Box_slide",
                b: "FixCube",
                from: "PE_Balloon_Slideframe_close",
                to: "PE_Balloon_Slideframe_far",
                limits: true,
                lo: -30000.0,
                hi: 0.0,
            },
            JointSpec::Slider {
                a: "PE_Balloon_Platform",
                b: "PE_Box_slide",
                from: "PE_Balloon_Slideframe_high",
                to: "PE_Balloon_Slideframe_low",
                limits: false,
                lo: -2.0,
                hi: 2.0,
            },
            JointSpec::Spring {
                a: "PE_Balloon_Platform",
                pos_a: [0.0, 0.0, 0.0],
                frame_a: "PE_Balloon_Slideframe_high",
                b: "PE_Box_slide",
                pos_b: [0.0, 0.0, 0.0],
                frame_b: "PE_Balloon_Slideframe_low",
                length: 15.0,
                constant: 20.0,
                lin_damp: 0.6,
                glob_damp: 1.0,
            },
            JointSpec::Hinge { a: "PE_Balloon_Platform", b: "PE_Balloon_Ballon_Seil01", frame: "PE_Balloon_Ballon01_hinge_a" },
            JointSpec::Hinge { a: "PE_Balloon_Ballon_Seil01", b: "PE_Balloon_Ballon01", frame: "PE_Balloon_Ballon01_hinge_b" },
            JointSpec::Hinge { a: "PE_Balloon_Platform", b: "PE_Balloon_Ballon_Seil02", frame: "PE_Balloon_Ballon02_hinge_a" },
            JointSpec::Hinge { a: "PE_Balloon_Ballon_Seil02", b: "PE_Balloon_Ballon02", frame: "PE_Balloon_Ballon02_hinge_b" },
            JointSpec::Hinge { a: "PE_Balloon_Platform", b: "PE_Balloon_Ballon_Seil03", frame: "PE_Balloon_Ballon03_hinge_a" },
            JointSpec::Hinge { a: "PE_Balloon_Ballon_Seil03", b: "PE_Balloon_Ballon03", frame: "PE_Balloon_Ballon03_hinge_b" },
            JointSpec::Hinge { a: "PE_Balloon_Platform", b: "PE_Balloon_Ballon_Seil04", frame: "PE_Balloon_Ballon04_hinge_a" },
            JointSpec::Hinge { a: "PE_Balloon_Ballon_Seil04", b: "PE_Balloon_Ballon04", frame: "PE_Balloon_Ballon04_hinge_b" },
            JointSpec::Hinge { a: "PE_Balloon_Platform", b: "PE_Balloon_Platte01", frame: "PE_Balloon_Hinge01" },
            JointSpec::Hinge { a: "PE_Balloon_Platte01", b: "PE_Balloon_Platte02", frame: "PE_Balloon_Hinge02" },
            JointSpec::Hinge { a: "PE_Balloon_Platte02", b: "PE_Balloon_Platte03", frame: "PE_Balloon_Hinge03" },
            JointSpec::Hinge { a: "PE_Balloon_Platte03", b: "PE_Balloon_Platte04", frame: "PE_Balloon_Hinge04" },
            JointSpec::Hinge { a: "PE_Balloon_Platte04", b: "PE_Balloon_Platte05", frame: "PE_Balloon_Hinge05" },
            JointSpec::Hinge { a: "PE_Balloon_Platte05", b: "PE_Balloon_Platte06", frame: "PE_Balloon_Hinge06" },
            JointSpec::Hinge { a: "PE_Balloon_Platte06", b: "PE_Balloon_Platte07", frame: "PE_Balloon_Hinge07" },
            JointSpec::Hinge { a: "PE_Balloon_Platte07", b: "PE_Balloon_Platte08", frame: "PE_Balloon_Hinge08" },
            JointSpec::Hinge { a: "PE_Balloon_Platte08", b: "FixCube", frame: "PE_Balloon_Hinge09" },
        ],
        // The platform is woken when the ball comes within 70 (exactness 75 to 100, every 10 to 60 frames), and the
        // balloons' forces start then.
        wake: Some(WakeSpec { part: "PE_Balloon_Platform", near: "PE_Balloon_Platform", distance: 70.0 }),
        forces: &[
            ForceSpec {
                part: "PE_Balloon_Ballon01",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Balloon_Ballon01",
                dir: [0.0, 1.0, 0.0],
                dir_frame: "PE_Balloon_Ballon01_hinge_b",
                value: 0.3,
            },
            ForceSpec {
                part: "PE_Balloon_Ballon_Seil01",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Balloon_Ballon01_hinge_b",
                dir: [-0.2, 0.0, 0.0],
                dir_frame: "PE_Balloon_Ballon01_hinge_b",
                value: 0.3,
            },
            ForceSpec {
                part: "PE_Balloon_Ballon02",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Balloon_Ballon02",
                dir: [0.0, 1.0, 0.0],
                dir_frame: "PE_Balloon_Ballon02_hinge_b",
                value: 0.3,
            },
            ForceSpec {
                part: "PE_Balloon_Ballon_Seil02",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Balloon_Ballon02_hinge_b",
                dir: [-0.2, 0.0, 0.0],
                dir_frame: "PE_Balloon_Ballon02_hinge_b",
                value: 0.3,
            },
            ForceSpec {
                part: "PE_Balloon_Ballon03",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Balloon_Ballon03",
                dir: [0.0, 1.0, 0.0],
                dir_frame: "PE_Balloon_Ballon03_hinge_b",
                value: 0.3,
            },
            ForceSpec {
                part: "PE_Balloon_Ballon_Seil03",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Balloon_Ballon03_hinge_b",
                dir: [-0.2, 0.0, 0.0],
                dir_frame: "PE_Balloon_Ballon03_hinge_b",
                value: 0.3,
            },
            ForceSpec {
                part: "PE_Balloon_Ballon04",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Balloon_Ballon04",
                dir: [0.0, 1.0, 0.0],
                dir_frame: "PE_Balloon_Ballon04_hinge_b",
                value: 0.3,
            },
            ForceSpec {
                part: "PE_Balloon_Ballon_Seil04",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Balloon_Ballon04_hinge_b",
                dir: [-0.2, 0.0, 0.0],
                dir_frame: "PE_Balloon_Ballon04_hinge_b",
                value: 0.3,
            },
        ],
        finish: Some(FinishSpec {
            near: "PE_Balloon_Platform",
            distance: 1.0,
            force: ForceSpec {
                part: "PE_Box_slide",
                pos: [0.0, 0.0, 0.0],
                pos_frame: "PE_Box_slide",
                dir: [-1.0, 0.0, 1.0],
                dir_frame: "PE_Box_slide",
                value: 0.2,
            },
            breaks: &["PE_Balloon_Hinge01", "PE_Balloon_Hinge02", "PE_Balloon_Hinge03", "PE_Balloon_Hinge04", "PE_Balloon_Hinge05", "PE_Balloon_Hinge06", "PE_Balloon_Hinge07", "PE_Balloon_Hinge08", "PE_Balloon_Hinge09"],
        }),
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
    fn a_module_is_found_by_name_and_its_joints_and_forces_name_its_parts() {
        assert!(spec("p_modul_34").is_some());
        assert!(spec("P_Modul_99").is_none());
        for m in MODULES {
            let named = |n: &str| n == FIXCUBE || m.parts.iter().any(|p| p.object == n);
            assert!(m.joints.iter().all(|j| match j {
                JointSpec::Slider { a, b, .. } | JointSpec::Hinge { a, b, .. } | JointSpec::Ball { a, b, .. } => {
                    named(a) && named(b)
                }
                JointSpec::Spring { a, b, .. } => named(a) && named(b),
            }), "{}", m.name);
            assert!(m.forces.iter().all(|f| named(f.part)), "{}", m.name);
            if let Some(f) = &m.finish {
                assert!(named(f.near) && named(f.force.part), "{}", m.name);
                // every hinge it breaks is one of the module's
                let hinge = |fr: &str| m.joints.iter().any(|j| matches!(j, JointSpec::Hinge { frame, .. } if *frame == fr));
                assert!(f.breaks.iter().all(|b| hinge(b)), "{}", m.name);
            }
        }
    }
}
