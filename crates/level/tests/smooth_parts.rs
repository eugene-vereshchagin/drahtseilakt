// The shared parts (balls, modules) are drawn round whichever model pack is chosen: a part without a look, or with a
// look that is only the part's own mesh again (`models original` puts the copy's meshes as looks), is rounded; a finer
// look from a pack is kept.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use level::{Level, Mesh, Part};

/// A flat-shaded icosphere of `20 * 4^depth` faces: one normal per face, like the original's low-poly balls.
fn flat_sphere(depth: u32) -> Mesh {
    use glam::Vec3;
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let mut v: Vec<Vec3> = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ]
    .iter()
    .map(|&(x, y, z)| Vec3::new(x, y, z).normalize())
    .collect();
    let mut tris: Vec<[usize; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..depth {
        let mut next = vec![];
        let mid = |v: &mut Vec<Vec3>, a: usize, b: usize| {
            v.push(((v[a] + v[b]) / 2.0).normalize());
            v.len() - 1
        };
        for [a, b, c] in tris {
            let (ab, bc, ca) = (mid(&mut v, a, b), mid(&mut v, b, c), mid(&mut v, c, a));
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        tris = next;
    }
    let (mut positions, mut normals, mut uvs, mut indices) = (vec![], vec![], vec![], vec![]);
    for [a, b, c] in tris {
        let n = (v[a] + v[b] + v[c]).normalize().to_array();
        let base = positions.len() as u32;
        for i in [a, b, c] {
            positions.push(v[i].to_array());
            normals.push(n);
            uvs.push([0.0, 0.0]);
        }
        indices.extend([base, base + 1, base + 2]);
    }
    Mesh {
        name: "Ball_Test_Mesh".into(),
        positions,
        normals,
        uvs,
        parts: vec![Part {
            material: None,
            indices,
        }],
        ..Mesh::default()
    }
}

fn level_of(m: Mesh) -> Level {
    Level {
        meshes: vec![m],
        ..Level::default()
    }
}

fn drawn_facets(l: &Level) -> f32 {
    level::smooth::facets(level::smooth::drawn(&l.meshes[0]))
}

#[test]
fn a_part_without_a_look_is_rounded() {
    let mut l = level_of(flat_sphere(2));
    let src = drawn_facets(&l);
    assert!(src > 20.0);
    assert_eq!(level::smooth::add_looks_cached(&mut l), 1);
    assert!(
        drawn_facets(&l) < src / 2.0,
        "{} of {src}",
        drawn_facets(&l)
    );
}

#[test]
fn a_look_no_finer_than_the_mesh_is_rounded_too() {
    // `models original`: the copy's mesh is put as the part's look
    let mut m = flat_sphere(2);
    m.look = Some(Box::new(m.clone()));
    let mut l = level_of(m);
    let src = drawn_facets(&l);
    assert!(src > 20.0);
    assert_eq!(level::smooth::add_looks_cached(&mut l), 1);
    assert!(
        drawn_facets(&l) < src / 2.0,
        "{} of {src}",
        drawn_facets(&l)
    );
}

#[test]
fn a_finer_look_is_kept() {
    let mut m = flat_sphere(2);
    let fine = flat_sphere(4);
    let tris = |m: &Mesh| m.parts[0].indices.len();
    m.look = Some(Box::new(fine));
    let before = m.look.as_ref().map(|l| tris(l));
    let mut l = level_of(m);
    assert_eq!(level::smooth::add_looks_cached(&mut l), 0);
    assert_eq!(l.meshes[0].look.as_ref().map(|l| tris(l)), before);
}
