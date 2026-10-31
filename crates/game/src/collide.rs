// The level's physical triangles: what the level's physical groups (floors, rails, modules) are made of, in world
// space. The physics world takes them as one static mesh.

use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}
