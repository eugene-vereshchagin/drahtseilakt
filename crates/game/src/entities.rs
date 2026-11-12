// The run's entity table: one list that the renderer and the map's scripts read (and the collision will, with
// task 3.2). Rows 0..N are the level's objects in the level's order; rows after them are what scripts made while
// the run goes: copies (Object Copy) and created objects. A row has its world transform, its parent (a child
// follows a moved parent), whether it is shown, its mesh and materials, its class and whether it is alive.
// It is part of the run's state: it clones and rewinds with it, and a level without scripts never has one (the
// renderer then takes the old path untouched).

use glam::{Mat4, Vec3};
use level::Level;

/// The most rows a run holds (level objects and what scripts add).
pub const MAX_ENTITIES: usize = crate::scripting::MAX_OBJECTS;
/// The most copies and created objects drawn in one frame; the rows past it stay in the table, undrawn.
pub const MAX_DRAWN_EXTRA: usize = 4_096;
/// How far up a parent chain is followed (a loop or a very deep chain stops here).
const MAX_DEPTH: usize = 32;

/// Where a row came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// An object of the level file.
    Level,
    /// A copy of the row `source` (a level row or another copy).
    Copy { source: usize },
    /// An object a script made out of nothing (Object Create); drawn only if it has a mesh.
    Created,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    #[allow(dead_code)]
    // read by the tests; the 3.2 collision will tell a copy from a level object by it
    pub class: Class,
    pub parent: Option<usize>,
    /// The world matrix as made or loaded.
    initial: Mat4,
    /// The transform now: world for a row with no parent, relative to the parent for a child.
    mat: Mat4,
    /// A script set `mat` (a row nobody moved keeps its loaded matrix exactly).
    moved: bool,
    /// A script hid it (or deleted it); for a level row, as before: the file's own hidden mark is not here.
    pub hidden: bool,
    /// A script's Set 3DSprite Mode (a sprite's own mode otherwise).
    sprite_mode: Option<u32>,
    pub mesh: Option<usize>,
    /// The materials of its mesh, in order.
    pub materials: Vec<usize>,
    pub alive: bool,
    /// Whether a copy of it is drawn as a plain mesh (not a billboard sprite, not a cloud layer).
    drawable: bool,
    bounds: Option<(Vec3, Vec3)>,
    /// The state Set IC saved: the world matrix and whether the row was shown. Restore IC goes back to it; none:
    /// the row's initial state is the level's own.
    ic: Option<(Mat4, bool)>,
}

#[derive(Debug, Clone, Default)]
pub struct Entities {
    rows: Vec<Entity>,
    /// Rows added by scripts (past the level's objects) that are not deleted: what the draw cap counts.
    level_rows: usize,
}

impl Entities {
    pub fn new(level: &Level) -> Entities {
        let rows = level
            .objects
            .iter()
            .map(|o| {
                let m = Mat4::from_cols_array_2d(&o.matrix);
                let bounds = o.mesh.and_then(|k| level.meshes.get(k)).and_then(|mesh| {
                    let mut lo = Vec3::splat(f32::INFINITY);
                    let mut hi = Vec3::splat(f32::NEG_INFINITY);
                    for p in &mesh.positions {
                        lo = lo.min(Vec3::from(*p));
                        hi = hi.max(Vec3::from(*p));
                    }
                    (lo.is_finite() && hi.is_finite()).then_some((lo, hi))
                });
                Entity {
                    name: o.name.clone(),
                    class: Class::Level,
                    parent: None,
                    initial: m,
                    mat: m,
                    moved: false,
                    hidden: false,
                    sprite_mode: None,
                    mesh: o.mesh,
                    materials: o
                        .mesh
                        .and_then(|k| level.meshes.get(k))
                        .map(|mesh| mesh.parts.iter().filter_map(|p| p.material).collect())
                        .unwrap_or_default(),
                    alive: true,
                    drawable: o.sprite.is_none() && o.name != "SkyLayer",
                    bounds,
                    ic: None,
                }
            })
            .collect::<Vec<_>>();
        let mut t = Entities {
            level_rows: rows.len(),
            rows,
        };
        for (i, o) in level.objects.iter().enumerate() {
            if let Some(p) = o.parent {
                t.link(i, Some(p));
            }
        }
        t
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn get(&self, i: usize) -> Option<&Entity> {
        self.rows.get(i)
    }

    pub fn name(&self, i: usize) -> &str {
        self.rows.get(i).map_or("?", |r| r.name.as_str())
    }

    pub fn mesh(&self, i: usize) -> Option<usize> {
        self.rows.get(i)?.mesh
    }

    pub fn local_bounds(&self, i: usize) -> Option<Option<(Vec3, Vec3)>> {
        self.rows.get(i).map(|r| r.bounds)
    }

    pub fn initial(&self, i: usize) -> Option<Mat4> {
        self.rows.get(i).map(|r| r.initial)
    }

    /// The matrix Restore IC puts a row back to: the one Set IC saved, else the level's.
    pub fn ic_matrix(&self, i: usize) -> Option<Mat4> {
        let r = self.rows.get(i)?;
        Some(r.ic.map_or(r.initial, |(m, _)| m))
    }

    /// Whether the row was shown when Set IC saved it (none: it never did).
    pub fn ic_shown(&self, i: usize) -> Option<bool> {
        self.rows.get(i)?.ic.map(|(_, hidden)| !hidden)
    }

    /// Set IC: the row's world matrix and visibility now are its initial condition.
    pub fn save_ic(&mut self, i: usize) -> bool {
        let Some(world) = self.world(i) else {
            return false;
        };
        let Some(r) = self.rows.get_mut(i) else {
            return false;
        };
        r.ic = Some((world, r.hidden));
        true
    }

    pub fn is_hidden(&self, i: usize) -> bool {
        self.rows.get(i).is_some_and(|r| r.hidden || !r.alive)
    }

    /// Whether a row or one of its parents has been moved by a script.
    fn affected(&self, i: usize, depth: usize) -> bool {
        let Some(r) = self.rows.get(i) else {
            return false;
        };
        r.moved || (depth < MAX_DEPTH && r.parent.is_some_and(|p| self.affected(p, depth + 1)))
    }

    fn world_at(&self, i: usize, depth: usize) -> Option<Mat4> {
        let r = self.rows.get(i)?;
        if !self.affected(i, 0) {
            return Some(r.initial);
        }
        match r.parent {
            Some(p) if depth < MAX_DEPTH => Some(self.world_at(p, depth + 1)? * r.mat),
            _ => Some(r.mat),
        }
    }

    /// The world transform of a row now: as loaded until it or a parent is moved.
    pub fn world(&self, i: usize) -> Option<Mat4> {
        self.world_at(i, 0)
    }

    /// Whether a script moved this row itself (not through a parent).
    pub fn is_moved(&self, i: usize) -> bool {
        self.rows.get(i).is_some_and(|r| r.moved)
    }

    /// Sets a row's world transform (a script's Set Position and the others); a child keeps its parent and its
    /// transform relative to it follows.
    pub fn set_world(&mut self, i: usize, world: Mat4) {
        let parent_world = self
            .rows
            .get(i)
            .and_then(|r| r.parent)
            .and_then(|p| self.world(p));
        let Some(r) = self.rows.get_mut(i) else {
            return;
        };
        r.mat = match parent_world {
            Some(pw) => pw.inverse() * world,
            None => world,
        };
        r.moved = true;
    }

    /// Makes `parent` the row's parent (None: no parent), keeping its world place. A loop is refused.
    pub fn link(&mut self, i: usize, parent: Option<usize>) -> bool {
        if i >= self.rows.len() || parent.is_some_and(|p| p >= self.rows.len()) {
            return false;
        }
        let mut at = parent;
        for _ in 0..=MAX_DEPTH {
            match at {
                Some(a) if a == i => return false,
                Some(a) => at = self.rows[a].parent,
                None => break,
            }
        }
        if at.is_some() {
            return false; // a chain deeper than we follow
        }
        let Some(world) = self.world(i) else {
            return false;
        };
        let rel = match parent {
            Some(p) => {
                let Some(pw) = self.world(p) else {
                    return false;
                };
                let inv = pw.inverse();
                if !inv.is_finite() {
                    return false;
                }
                inv * world
            }
            None => world,
        };
        let moved_now = self.affected(i, 0);
        let r = &mut self.rows[i];
        r.parent = parent;
        r.mat = rel;
        // freed from its parent it must keep the place the chain gave it
        if parent.is_none() {
            r.moved = moved_now;
        }
        true
    }

    /// Set 3DSprite Mode for a row; whether it changed.
    pub fn set_sprite_mode(&mut self, i: usize, mode: u32) -> bool {
        match self.rows.get_mut(i) {
            Some(r) if r.sprite_mode != Some(mode) => {
                r.sprite_mode = Some(mode);
                true
            }
            _ => false,
        }
    }

    /// The mode a script gave a row's sprite.
    pub fn sprite_mode(&self, i: usize) -> Option<u32> {
        self.rows.get(i)?.sprite_mode
    }

    pub fn set_hidden(&mut self, i: usize, hidden: bool) -> bool {
        match self.rows.get_mut(i) {
            Some(r) if r.hidden != hidden => {
                r.hidden = hidden;
                true
            }
            _ => false,
        }
    }

    /// Deletes a row: not drawn, not found by the scripts' reads again. Its slot stays (rows are never renumbered).
    pub fn delete(&mut self, i: usize) {
        if let Some(r) = self.rows.get_mut(i) {
            r.alive = false;
            r.hidden = true;
        }
    }

    /// A copy of row `i` under `name`: the source's place, mesh, materials and box; shown as the source is. Its
    /// index, or none at the cap or for a row that does not exist.
    pub fn copy(&mut self, i: usize, name: &str) -> Option<usize> {
        if self.rows.len() >= MAX_ENTITIES {
            return None;
        }
        let world = self.world(i)?;
        let src = self.rows.get(i)?;
        let row = Entity {
            name: name.to_string(),
            class: Class::Copy { source: i },
            parent: None,
            initial: world,
            mat: world,
            moved: false,
            hidden: src.hidden || !src.alive,
            sprite_mode: src.sprite_mode,
            mesh: src.mesh,
            materials: src.materials.clone(),
            alive: true,
            drawable: src.drawable,
            bounds: src.bounds,
            ic: None,
        };
        self.rows.push(row);
        Some(self.rows.len() - 1)
    }

    /// A new object of a class (Object Create): at `matrix`, with a mesh to draw or none (it then only exists).
    pub fn create(
        &mut self,
        name: &str,
        mesh: Option<usize>,
        materials: Vec<usize>,
        bounds: Option<(Vec3, Vec3)>,
        matrix: Mat4,
    ) -> Option<usize> {
        if self.rows.len() >= MAX_ENTITIES || !matrix.is_finite() {
            return None;
        }
        self.rows.push(Entity {
            name: name.to_string(),
            class: Class::Created,
            parent: None,
            initial: matrix,
            mat: matrix,
            moved: false,
            hidden: false,
            sprite_mode: None,
            mesh,
            materials,
            alive: true,
            drawable: true,
            bounds,
            ic: None,
        });
        Some(self.rows.len() - 1)
    }

    /// How many rows a script hid or deleted, and how many it moved (the run's reports).
    #[allow(dead_code)] // the headless reports of the tests
    pub fn hidden_count(&self) -> usize {
        self.rows.iter().filter(|r| r.hidden).count()
    }

    #[allow(dead_code)]
    pub fn moved_count(&self) -> usize {
        self.rows.iter().filter(|r| r.moved).count()
    }

    /// Where a drawn thing of level object `i` goes, `placed` being its place as the level draws it: as it is
    /// when nothing moved it; a script's matrix for a moved object with no parent (the old rule); for a child,
    /// `placed` carried along by what its parents did.
    pub fn draw_matrix(&self, i: usize, placed: Mat4) -> Mat4 {
        let Some(r) = self.rows.get(i) else {
            return placed;
        };
        if !self.affected(i, 0) {
            return placed;
        }
        if r.parent.is_none() {
            return r.mat;
        }
        match self.world(i) {
            Some(w) => w * r.initial.inverse() * placed,
            None => placed,
        }
    }

    /// The copies and created objects to draw: each with the row and its world matrix; shown, alive, with a mesh
    /// and a material to draw it with, at most `MAX_DRAWN_EXTRA`. The mesh is the source's level mesh, so the
    /// renderer draws it as it draws the level's own (a mesh with no material, a collider, is not seen).
    pub fn drawn_extra(&self) -> impl Iterator<Item = (usize, &Entity, Mat4)> + '_ {
        self.rows
            .iter()
            .enumerate()
            .skip(self.level_rows)
            .filter(|(_, r)| {
                r.alive && !r.hidden && r.drawable && r.mesh.is_some() && !r.materials.is_empty()
            })
            .take(MAX_DRAWN_EXTRA)
            .filter_map(|(i, r)| Some((i, r, self.world(i)?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn table(n: usize) -> Entities {
        let rows = (0..n)
            .map(|k| {
                let m = Mat4::from_translation(Vec3::new(k as f32 * 10.0, 0.0, 0.0));
                Entity {
                    name: format!("o{k}"),
                    class: Class::Level,
                    parent: None,
                    initial: m,
                    mat: m,
                    moved: false,
                    hidden: false,
                    sprite_mode: None,
                    mesh: Some(k),
                    materials: vec![k + 100],
                    alive: true,
                    drawable: true,
                    bounds: None,
                    ic: None,
                }
            })
            .collect::<Vec<_>>();
        Entities {
            level_rows: rows.len(),
            rows,
        }
    }

    fn at(t: &Entities, i: usize) -> Vec3 {
        t.world(i).unwrap().w_axis.truncate()
    }

    #[test]
    fn a_copy_is_a_new_row_drawn_with_the_sources_mesh_and_materials() {
        let mut t = table(3);
        let c = t.copy(1, "copy").unwrap();
        assert_eq!(c, 3);
        assert_eq!(t.mesh(c), Some(1));
        assert_eq!(t.get(c).unwrap().materials, vec![101]);
        assert_eq!(t.get(c).unwrap().class, Class::Copy { source: 1 });
        let drawn: Vec<_> = t.drawn_extra().map(|(i, _, w)| (i, w)).collect();
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].0, 3);
        assert_eq!(drawn[0].1.w_axis.x, 10.0);
        // moving the copy leaves the source where it was
        t.set_world(c, Mat4::from_translation(Vec3::new(5.0, 6.0, 7.0)));
        assert_eq!(at(&t, c), Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(at(&t, 1), Vec3::new(10.0, 0.0, 0.0));
        // a copy of a copy has the copy's place
        let cc = t.copy(c, "again").unwrap();
        assert_eq!(t.get(cc).unwrap().class, Class::Copy { source: c });
        assert_eq!(at(&t, cc), Vec3::new(5.0, 6.0, 7.0));
    }

    #[test]
    fn a_deleted_row_is_not_drawn_and_a_hidden_one_neither() {
        let mut t = table(2);
        let a = t.copy(0, "a").unwrap();
        let b = t.copy(0, "b").unwrap();
        t.delete(a);
        t.set_hidden(b, true);
        assert_eq!(t.drawn_extra().count(), 0);
        t.set_hidden(b, false);
        assert_eq!(t.drawn_extra().count(), 1);
        t.delete(1);
        assert!(t.is_hidden(1));
        assert_eq!(t.len(), 4, "rows are never renumbered");
    }

    #[test]
    fn a_created_object_is_drawn_only_with_a_mesh() {
        let mut t = table(1);
        let bare = t
            .create("bare", None, vec![], None, Mat4::IDENTITY)
            .unwrap();
        let shown = t
            .create("shown", Some(7), vec![3], None, Mat4::IDENTITY)
            .unwrap();
        assert_eq!(t.get(bare).unwrap().class, Class::Created);
        let drawn: Vec<_> = t.drawn_extra().collect();
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].0, shown);
    }

    #[test]
    fn set_ic_saves_the_place_and_visibility_restore_uses() {
        let mut t = table(2);
        // without Set IC the initial state is the level's own
        assert_eq!(t.ic_matrix(1), t.initial(1));
        assert_eq!(t.ic_shown(1), None);
        t.set_world(1, Mat4::from_translation(Vec3::new(3.0, 4.0, 5.0)));
        t.set_hidden(1, true);
        assert!(t.save_ic(1));
        t.set_world(1, Mat4::from_translation(Vec3::new(9.0, 9.0, 9.0)));
        t.set_hidden(1, false);
        assert_eq!(
            t.ic_matrix(1).unwrap().w_axis.truncate(),
            Vec3::new(3.0, 4.0, 5.0)
        );
        assert_eq!(t.ic_shown(1), Some(false));
        // the level's own initial matrix (what bodies follow) is unchanged
        assert_eq!(t.initial(1).unwrap().w_axis.x, 10.0);
        // a child saves its world place
        t.set_world(0, Mat4::from_translation(Vec3::new(0.0, 5.0, 0.0)));
        assert!(t.link(1, Some(0)));
        assert!(t.save_ic(1));
        assert_eq!(
            t.ic_matrix(1).unwrap().w_axis.truncate(),
            Vec3::new(9.0, 9.0, 9.0)
        );
        assert!(!t.save_ic(7));
    }

    #[test]
    fn a_child_follows_a_moved_parent() {
        let mut t = table(3);
        assert!(t.link(1, Some(0)));
        assert!(t.link(2, Some(1)));
        // nothing moved: every row is exactly where it was loaded
        assert_eq!(at(&t, 2), Vec3::new(20.0, 0.0, 0.0));
        t.set_world(0, Mat4::from_translation(Vec3::new(0.0, 5.0, 0.0)));
        assert_eq!(at(&t, 1), Vec3::new(10.0, 5.0, 0.0));
        assert_eq!(at(&t, 2), Vec3::new(20.0, 5.0, 0.0));
        // a turned parent swings its child about itself
        t.set_world(0, Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let p = at(&t, 1);
        assert!((p - Vec3::new(0.0, 0.0, -10.0)).length() < 1e-4, "{p:?}");
        // the drawn mesh of an unmoved child is carried along too
        let d = t.draw_matrix(1, Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)));
        assert!((d.w_axis.truncate() - p).length() < 1e-4);
        // moving the child itself sets its place in the world
        t.set_world(1, Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(at(&t, 1), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn a_parent_loop_is_refused_and_a_row_without_a_parent_is_as_before() {
        let mut t = table(2);
        assert!(t.link(1, Some(0)));
        assert!(!t.link(0, Some(1)));
        assert!(!t.link(0, Some(0)));
        let m = Mat4::from_translation(Vec3::new(3.0, 3.0, 3.0));
        t.set_world(0, m);
        // the old rule for an unparented moved object: its matrix replaces the placement's
        let mut u = table(2);
        u.set_world(1, m);
        assert_eq!(u.draw_matrix(1, Mat4::IDENTITY), m);
        assert_eq!(u.draw_matrix(0, Mat4::IDENTITY), Mat4::IDENTITY);
    }

    #[test]
    fn a_clone_rewinds_the_table() {
        let mut t = table(2);
        let saved = t.clone();
        t.copy(0, "c").unwrap();
        t.set_world(1, Mat4::from_translation(Vec3::new(9.0, 9.0, 9.0)));
        t.delete(0);
        assert_eq!(saved.len(), 2);
        assert_eq!(at(&saved, 1), Vec3::new(10.0, 0.0, 0.0));
        assert!(!saved.is_hidden(0));
        assert_eq!(saved.drawn_extra().count(), 0);
        let again = saved.clone();
        assert_eq!(again.len(), 2);
    }

    #[test]
    fn the_table_stops_at_its_caps() {
        let mut t = table(1);
        for k in 0..(MAX_DRAWN_EXTRA + 10) {
            t.copy(0, &format!("c{k}")).unwrap();
        }
        assert_eq!(t.drawn_extra().count(), MAX_DRAWN_EXTRA);
    }
}
