// The physics of the original's modules (PH/P_Modul_NN.nmo): which of a module's objects its scripts physicalize,
// with which values and collision meshes, and the joints they make. The values are the original's (docs/physics.md,
// the PH module table; the meshes and joints as the C game makes them at level 1). A module without an entry here
// keeps its placeholder as a solid until its scripts are run (phase 4).

use crate::physics::Material;

/// One physicalized object of a module: its name in the module file, its values, and its convex collision meshes
/// (meshes of the module file, in the object's frame).
#[derive(Debug, Clone, Copy)]
pub struct PartSpec {
    pub object: &'static str,
    pub fixed: bool,
    pub material: Material,
    pub group: &'static str,
    /// Made asleep: it moves once something touches it.
    pub start_frozen: bool,
    pub meshes: &'static [&'static str],
}

/// A joint a module makes: a slider of a part along the axis between two of the module's objects, to the fixed
/// anchor FixCube (Set Physics Slider; the game never limits these).
#[derive(Debug, Clone, Copy)]
pub struct SliderSpec {
    pub part: &'static str,
    pub from: &'static str,
    pub to: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleSpec {
    pub name: &'static str,
    pub parts: &'static [PartSpec],
    pub sliders: &'static [SliderSpec],
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
                meshes: &["P_Modul_01_Rinne_01_Mesh", "P_Modul_01_Rinne_02_Mesh", "P_Modul_01_Rinne_03_Mesh"],
            },
            PartSpec {
                object: "P_Modul_01_Filler",
                fixed: true,
                material: material(0.7, 0.4, 1.0, 0.1, 0.1),
                group: "Floor",
                start_frozen: false,
                meshes: &["P_Modul_01_Filler_Mesh"],
            },
            PartSpec {
                object: "P_Modul_01_Pusher",
                fixed: false,
                material: material(0.6, 0.4, 3.0, 0.1, 1.0),
                group: "Floor",
                start_frozen: true,
                meshes: &["P_Modul_01_Col01_Mesh", "P_Modul_01_Col02_Mesh", "P_Modul_01_Col03_Mesh"],
            },
        ],
        sliders: &[],
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
                meshes: &["P_Modul_34_Kiste_Mesh"],
            },
            PartSpec {
                object: "P_Modul_34_Schiebestein",
                fixed: false,
                material: material(0.5, 0.4, 1.6, 0.1, 0.1),
                group: "",
                start_frozen: true,
                meshes: &["P_Modul_34_Schiebestein_Mesh"],
            },
        ],
        sliders: &[SliderSpec {
            part: "P_Modul_34_Schiebestein",
            from: "P_Modul_34_Slider_Frame01",
            to: "P_Modul_34_Slider_Frame02",
        }],
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
        assert!(m.sliders.iter().all(|s| m.parts.iter().any(|p| p.object == s.part)));
        assert!(spec("P_Modul_99").is_none());
    }
}
