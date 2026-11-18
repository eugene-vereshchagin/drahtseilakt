// The sky a map's TT Sky block builds (scripts: `SkySpec`): walls round the camera with the level's own materials,
// a bottom (and a top) closing them. Built here as one mesh centred on the origin; the frame draws it at the eye,
// behind everything, replacing the level's own sky while it is on.

use level::{Mesh, Part};
use script::SkySpec;

/// Walls drawn at most (the block has up to 16 sides).
const MAX_SIDES: usize = 16;

/// The sky's mesh, with a part per material; none when nothing of it has a material or its size is nothing.
pub fn build(spec: &SkySpec) -> Option<Mesh> {
    let n = spec.sides.len().clamp(4, MAX_SIDES);
    let r = spec.radius;
    // NaN-preserving: a NaN radius is refused, which `r <= 1e-3` would let through
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(r > 1e-3) {
        return None;
    }
    let step = std::f32::consts::TAU / n as f32;
    let half_w = r * (step / 2.0).tan();
    let half_h = if spec.quadratic {
        half_w
    } else {
        (spec.side_height / 2.0).max(1e-3)
    };
    let (lo, hi) = (spec.y - half_h, spec.y + half_h);
    let mut mesh = Mesh {
        name: "ScriptSky".to_string(),
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        parts: Vec::new(),
        channels: Vec::new(),
        look: None,
    };
    let add = |mesh: &mut Mesh, material: usize, quad: &[([f32; 3], [f32; 2])]| {
        let base = mesh.positions.len() as u32;
        for (p, uv) in quad {
            mesh.positions.push(*p);
            mesh.normals.push([0.0, 1.0, 0.0]);
            mesh.uvs.push(*uv);
        }
        let k = quad.len() as u32;
        let tris: Vec<u32> = (1..k.saturating_sub(1))
            .flat_map(|i| {
                [
                    base,
                    base.saturating_add(i),
                    base.saturating_add(i).saturating_add(1),
                ]
            })
            .collect();
        match mesh.parts.iter_mut().find(|p| p.material == Some(material)) {
            Some(p) => p.indices.extend(tris),
            None => mesh.parts.push(Part {
                material: Some(material),
                indices: tris,
            }),
        }
    };
    // a wall k faces the direction (sin, cos) of its angle, front +z then right +x; left to right is (cos, -sin)
    for (k, m) in spec.sides.iter().take(n).enumerate() {
        let Some(m) = *m else { continue };
        let a = step * k as f32;
        let (d, rt) = ((a.sin(), a.cos()), (a.cos(), -a.sin()));
        let at = |s: f32, y: f32| [d.0 * r + rt.0 * s, y, d.1 * r + rt.1 * s];
        // texture rows run top to bottom; the inset keeps edge texels from wrapping around
        add(
            &mut mesh,
            m,
            &[
                (at(-half_w, lo), [0.002, 0.998]),
                (at(half_w, lo), [0.998, 0.998]),
                (at(half_w, hi), [0.998, 0.002]),
                (at(-half_w, hi), [0.002, 0.002]),
            ],
        );
    }
    // the lid and the floor: a fan over the walls' corners
    let corner = r / (step / 2.0).cos();
    let lid = |y: f32| -> Vec<([f32; 3], [f32; 2])> {
        (0..n)
            .map(|k| {
                let a = step * (k as f32 + 0.5);
                let (x, z) = (a.sin() * corner, a.cos() * corner);
                (
                    [x, y, z],
                    [0.5 + x / (2.0 * corner), 0.5 + z / (2.0 * corner)],
                )
            })
            .collect()
    };
    if let Some(m) = spec.bottom {
        add(&mut mesh, m, &lid(lo));
    }
    if let Some(m) = spec.top {
        add(&mut mesh, m, &lid(hi));
    }
    (!mesh.parts.is_empty()).then_some(mesh)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn spec() -> SkySpec {
        SkySpec {
            color: [1.0; 4],
            radius: 100.0,
            distortion: 0.0,
            quadratic: true,
            side_height: 10.0,
            y: 0.0,
            sides: vec![Some(3), Some(4), None, Some(3)],
            bottom: Some(5),
            top: None,
        }
    }

    #[test]
    fn four_walls_a_bottom_and_a_part_per_material() {
        let m = build(&spec()).unwrap();
        // three walls (one has no material) and the bottom's four corners
        assert_eq!(m.positions.len(), 3 * 4 + 4);
        let mats: Vec<_> = m.parts.iter().map(|p| p.material).collect();
        assert_eq!(mats, vec![Some(3), Some(4), Some(5)]);
        assert_eq!(m.parts[0].indices.len(), 12, "two walls of one material");
        // square walls: a hundred out, a hundred each way
        let front = &m.positions[0..4];
        assert!(front.iter().all(|p| (p[2] - 100.0).abs() < 1e-3));
        assert!(front.iter().all(|p| (p[1].abs() - 100.0).abs() < 1e-3));
        assert!(
            m.parts
                .iter()
                .all(|p| p.indices.iter().all(|&i| (i as usize) < m.positions.len()))
        );
    }

    #[test]
    fn nothing_to_draw_without_materials_or_size() {
        let mut s = spec();
        s.radius = 0.0;
        assert!(build(&s).is_none());
        let mut s = spec();
        s.sides = vec![None; 4];
        s.bottom = None;
        assert!(build(&s).is_none());
    }
}
