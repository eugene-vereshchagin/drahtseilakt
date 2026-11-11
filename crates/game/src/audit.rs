// The map audit (`--audit FILE|DIR`): what a map asks of the game and what the game does not give it. For every
// map, with the player's copy and without it: each body the physics makes of a level object that is not drawn
// (an "invisible blocker" gets a name, its groups and the reason), the stand-ins no module is made for, the
// textures nobody has, the objects of kinds the loader leaves out, the materials' modes the renderer does not
// follow, the script blocks not known. One CSV row per map, kind of issue and copy; a summary ranked by the
// number of maps hit. Maps are strangers' files: read through the same loaders as the game, nothing else.

use crate::assets::{Data, Source};
use crate::scene::{self, Modules, Placement};
use crate::sim::Sim;
use level::Level;
use std::path::{Path, PathBuf};

/// How deep a folder of maps is walked, and how many maps one run takes.
const MAX_DEPTH: usize = 6;
const MAX_MAPS: usize = 5000;
/// How many names a row lists (the count is always whole).
const MAX_NAMES: usize = 40;

/// One kind of issue in one map: how many, and the names (objects, textures, modules, blocks).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub kind: &'static str,
    pub count: usize,
    pub names: Vec<String>,
}

/// The kinds, with what each means (the CSV's legend and the summary's).
pub const KINDS: [(&str, &str); 24] = [
    ("load-failed", "the map does not load"),
    (
        "load-warning",
        "objects of the file that could not be decoded",
    ),
    (
        "body-no-material",
        "a floor, rail or stopper collides but is not drawn",
    ),
    (
        "body-plain",
        "a floor, rail or stopper whose mesh has no material at all: drawn plain grey",
    ),
    (
        "body-transparent",
        "a floor or rail collides but every material of it is fully transparent",
    ),
    (
        "body-hidden",
        "a floor or rail hidden in the file collides with the ball: an invisible collider, solid in the original too",
    ),
    (
        "stopper-hidden",
        "a stopper hidden in the file (stops boxes and loose balls, never the player's ball)",
    ),
    (
        "standin-solid",
        "a module's stand-in left as a solid of its own mesh: no module of its group is there",
    ),
    (
        "standin-hidden",
        "a module's stand-in hidden in the file: no module is made at it",
    ),
    (
        "module-missing",
        "a module the map names that neither the game nor the copy has",
    ),
    (
        "loose-not-drawn",
        "a box, ball or dome has a body but nothing is drawn for it",
    ),
    (
        "texture-missing",
        "a texture a material draws with that nobody has (drawn in the material's colour, toned down)",
    ),
    (
        "texture-found",
        "textures drawn: the map's own pixels, ours or the copy's by name (count only)",
    ),
    (
        "texture-by-name",
        "textures without a picture or a file, taken for the game's texture their name means (count only)",
    ),
    (
        "frame-drawn",
        "2D frames of the map drawn over the picture, as the original draws them (count only)",
    ),
    (
        "frame-not-drawn",
        "2D frames not drawn: hidden or without a texture (a script's), or with a picture saved in the map (its bitmap is not read right yet)",
    ),
    (
        "sound-unplayed",
        "sounds of the map (read; its scripts play them, and the files are not in the map): not played",
    ),
    (
        "curve-unused",
        "curves and their points nothing in the map reads (an exporter's shapes): not drawn (count only)",
    ),
    (
        "part-no-material",
        "a drawn mesh has faces without a material (drawn plain grey; the original draws them white)",
    ),
    (
        "class-skipped",
        "objects of classes the loader does not read (curves a script reads, patch meshes, MIDI, 2D sprites, ...)",
    ),
    (
        "animation-played",
        "object animations in a map whose blocks play one: not played here, the objects stand still",
    ),
    (
        "animation-unplayed",
        "object animations no block of the map plays (an exporter's bake): still in the original too (count only)",
    ),
    (
        "material-mode",
        "materials whose alpha test is not 'greater' (the renderer tests 'greater' only)",
    ),
    (
        "script-block-unknown",
        "script blocks this engine does not know: the map's scripts do not run",
    ),
];

fn push(out: &mut Vec<Issue>, kind: &'static str, name: String) {
    let i = match out.iter().position(|i| i.kind == kind) {
        Some(i) => i,
        None => {
            out.push(Issue {
                kind,
                count: 0,
                names: Vec::new(),
            });
            out.len().saturating_sub(1)
        }
    };
    if let Some(issue) = out.get_mut(i) {
        issue.count = issue.count.saturating_add(1);
        if issue.names.len() < MAX_NAMES && !issue.names.contains(&name) {
            issue.names.push(name);
        }
    }
}

/// A level object as a row names it: its name and where it is.
fn named(level: &Level, i: usize) -> String {
    level.objects.get(i).map_or_else(String::new, |o| {
        let [x, y, z] = o.position();
        format!("{} @{x:.0},{y:.0},{z:.0}", o.name)
    })
}

/// The issues of a loaded level: its bodies against what is drawn, its stand-ins, its textures.
pub fn level_issues(
    level: &Level,
    modules: &Modules,
    sim: &Sim,
    placements: &[Placement],
    data: &Data,
    open: bool,
) -> Vec<Issue> {
    let mut out = Vec::new();
    let mut drawn = vec![false; level.objects.len()];
    for p in placements {
        if let Some(d) = drawn.get_mut(p.object) {
            *d = true;
        }
    }
    let is_drawn = |i: usize| drawn.get(i).copied().unwrap_or(false);
    let loaded = |i: usize| scene::module_of(level, i).filter(|n| modules.get(n).is_some());
    let not_solid: Vec<usize> = (0..level.objects.len())
        .filter(|&i| loaded(i).is_some())
        .collect();
    for (i, body) in crate::sim::static_objects(level, &not_solid) {
        let Some(o) = level.objects.get(i) else {
            continue;
        };
        let Some(mesh) = o.mesh.and_then(|m| level.meshes.get(m)) else {
            continue;
        };
        let stand_in = level.module_group(i).is_some();
        let for_ball = body.group != crate::physics::GROUP_BALL;
        let clear = |p: &level::Part| {
            p.material
                .and_then(|m| level.materials.get(m))
                .is_some_and(|m| m.blend.is_some() && m.diffuse[3] <= 0.02)
        };
        let kind = if stand_in {
            "standin-solid"
        } else if !o.visible {
            if for_ball {
                "body-hidden"
            } else {
                "stopper-hidden"
            }
        } else if !is_drawn(i) {
            "body-no-material"
        } else if !mesh.parts.is_empty() && mesh.parts.iter().all(clear) {
            "body-transparent"
        } else if mesh.parts.iter().all(|p| p.material.is_none()) {
            "body-plain"
        } else {
            continue;
        };
        push(&mut out, kind, named(level, i));
    }
    for &(i, _) in sim.loose() {
        if !is_drawn(i) {
            push(&mut out, "loose-not-drawn", named(level, i));
        }
    }
    // the stand-ins: a member of a module's group at which no module is made
    for (i, o) in level.objects.iter().enumerate() {
        let Some(group) = level.module_group(i) else {
            continue;
        };
        if scene::is_marker_group(group) || o.mesh.is_none() {
            continue;
        }
        match scene::module_of(level, i) {
            None if !o.visible => push(&mut out, "standin-hidden", named(level, i)),
            None => {}
            Some(name) if modules.get(name).is_none() => {
                push(&mut out, "module-missing", name.to_owned());
            }
            Some(_) => {}
        }
    }
    // the textures the materials of drawn meshes use
    let mut used = vec![false; level.textures.len()];
    for (i, o) in level.objects.iter().enumerate() {
        if !is_drawn(i) {
            continue;
        }
        let Some(mesh) = o.mesh.and_then(|m| level.meshes.get(m)) else {
            continue;
        };
        if mesh.parts.iter().any(|p| p.material.is_none())
            && mesh.parts.iter().any(|p| p.material.is_some())
        {
            push(&mut out, "part-no-material", named(level, i));
        }
        let materials = mesh
            .parts
            .iter()
            .filter_map(|p| p.material)
            .chain(mesh.channels.iter().map(|c| c.material));
        for t in materials.filter_map(|m| level.materials.get(m).and_then(|m| m.texture)) {
            if let Some(u) = used.get_mut(t) {
                *u = true;
            }
        }
    }
    for (t, _) in level.textures.iter().zip(&used).filter(|(_, u)| **u) {
        // (a pack's picture put in place of a texture at load is pixels by now)
        let found = match &t.source {
            level::TextureSource::Pixels { .. } => true,
            level::TextureSource::None => false,
            _ => data.texture(t).is_some(),
        };
        // a `.nmo` map names the game's textures by file; one named outright is a texture that had neither a file
        // nor pixels and was taken by its name (assets::name_meant_textures)
        if let (true, false, level::TextureSource::Game(meant)) = (found, open, &t.source) {
            push(&mut out, "texture-by-name", format!("{} = {meant}", t.name));
        }
        if found {
            push(&mut out, "texture-found", t.name.clone());
        } else {
            let file = match &t.source {
                level::TextureSource::File(f) => f.as_str(),
                _ => "no file named",
            };
            push(&mut out, "texture-missing", format!("{} <{file}>", t.name));
        }
    }
    if let Some(graph) = &level.scripts {
        for (guid, name, n) in graph.prototypes() {
            if !script::blocks::known(guid) {
                for _ in 0..n {
                    push(&mut out, "script-block-unknown", name.clone());
                }
            }
        }
    }
    out
}

/// What the file itself holds that the level model leaves out: classes not read, alpha tests not followed.
fn file_issues(path: &Path, out: &mut Vec<Issue>) {
    let Some(bytes) = crate::assets::read_game_file(path) else {
        return;
    };
    let Ok(doc) = nmo::read(&bytes).and_then(|f| nmo::Document::parse(&f)) else {
        return;
    };
    doc_issues(&doc, out);
}

/// The building blocks that play an animation: Play Animation 3D Entity and Play Global Animation (3DTransfo;
/// docs/animation.md 5.5 and 5.6).
const ANIMATION_PLAYERS: [nmo::behavior::Guid; 2] = [
    nmo::behavior::Guid(0x6422_1225, 0x769a_143f),
    nmo::behavior::Guid(0x1c92_36e1, 0x42f4_0996),
];

fn is_animation(class_id: u32) -> bool {
    matches!(class_id, 15 | 16 | 18)
}

/// Whether anything in the file can play an animation: one of the blocks that do, or a parameter whose value is
/// an animation of the file. Nothing else moves an object by its animation: Virtools plays none by itself.
fn plays_animations(doc: &nmo::Document) -> bool {
    doc.objects.iter().any(|o| {
        let Some(chunk) = &o.chunk else { return false };
        match o.class_id {
            nmo::behavior::class::BEHAVIOR => nmo::behavior::behavior(chunk)
                .ok()
                .and_then(|b| b.proto)
                .is_some_and(|(guid, _)| ANIMATION_PLAYERS.contains(&guid)),
            3 | 45 => matches!(
                nmo::behavior::parameter(chunk).map(|p| p.value),
                Ok(nmo::behavior::Saved::Object(Some(i)))
                    if doc.get(i).is_some_and(|a| is_animation(a.class_id))
            ),
            _ => false,
        }
    })
}

/// Whether an object animation's keys move its object at all: some linear position, rotation or scale key differs
/// from the first (a quaternion and its negative are one rotation).
fn animation_moves(chunk: &nmo::chunk::Chunk) -> bool {
    const EPS: f32 = 1e-4;
    let Ok(Some(a)) = nmo::scene::object_animation(chunk) else {
        return false;
    };
    fn differ<const N: usize>(keys: &[(f32, [f32; N])], signed: bool) -> bool {
        let Some((_, first)) = keys.first() else {
            return false;
        };
        let off =
            |k: &[f32; N], sign: f32| k.iter().zip(first).any(|(x, y)| (x * sign - y).abs() > EPS);
        keys.iter()
            .any(|(_, k)| off(k, 1.0) && (!signed || off(k, -1.0)))
    }
    differ(&a.position, false) || differ(&a.rotation, true) || differ(&a.scale, false)
}

/// The issues of a file's objects: the classes the loader leaves out, the animations, the materials' modes.
fn doc_issues(doc: &nmo::Document, out: &mut Vec<Issue>) {
    use nmo::class;
    // what the loader reads, and what carries nothing to draw (the level, groups, arrays, behaviours and their
    // parameters, scenes, attributes' and messages' managers)
    let read = [
        class::LEVEL,
        class::DATA_ARRAY,
        class::GROUP,
        class::MATERIAL,
        class::TEXTURE,
        class::MESH,
        class::ENTITY_3D,
        class::CAMERA,
        class::TARGET_CAMERA,
        class::LIGHT,
        class::TARGET_LIGHT,
        class::OBJECT_3D,
        class::SPRITE_3D,
        nmo::behavior::class::BEHAVIOR,
    ];
    let logic = [
        2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 20, 22, 45, 46, 48, 55,
    ];
    let played = plays_animations(doc);
    // what a script of the file can reach: a parameter's value, a behaviour's target
    let mut reached = vec![false; doc.objects.len()];
    for o in &doc.objects {
        let Some(chunk) = &o.chunk else { continue };
        let i = match o.class_id {
            nmo::behavior::class::BEHAVIOR => {
                nmo::behavior::behavior(chunk).ok().and_then(|b| b.target)
            }
            3 | 45 => match nmo::behavior::parameter(chunk).map(|p| p.value) {
                Ok(nmo::behavior::Saved::Object(i)) => i,
                _ => None,
            },
            _ => None,
        };
        if let Some(r) = i.and_then(|i| reached.get_mut(i)) {
            *r = true;
        }
    }
    // a curve is read when it or a point of it is reached; its points are then read with it
    let mut curve_read = reached.clone();
    for (i, o) in doc.of_class(class::CURVE) {
        let points = o
            .chunk
            .as_ref()
            .and_then(|c| nmo::scene::curve(c).ok().flatten())
            .map(|c| c.points)
            .unwrap_or_default();
        let read = reached.get(i).copied().unwrap_or(false)
            || points
                .iter()
                .any(|p| reached.get(*p).copied().unwrap_or(false));
        for p in points.into_iter().chain([i]) {
            if let Some(r) = curve_read.get_mut(p) {
                *r = read;
            }
        }
    }
    for (i, o) in doc.objects.iter().enumerate() {
        if read.contains(&o.class_id) || logic.contains(&o.class_id) {
            continue;
        }
        match o.class_id {
            class::ENTITY_2D => {
                // drawn as hud::level_frames draws: shown in the file, with a material that has a texture
                let chunk = o.chunk.as_ref();
                let shown =
                    chunk.is_some_and(|c| c.section(0x4).is_none() && c.section(0x18).is_none());
                let textured = chunk
                    .and_then(|c| nmo::scene::entity_2d(c).ok())
                    .and_then(|e| doc.get(e.material?)?.chunk.as_ref())
                    .and_then(|c| nmo::scene::material(c).ok())
                    .and_then(|m| doc.get(m.texture?)?.chunk.as_ref())
                    .and_then(|c| nmo::scene::texture(c).ok())
                    // (a picture saved inside the map is not drawn yet: hud::level_frames)
                    .is_some_and(|t| t.embedded.is_none());
                let kind = if shown && textured {
                    "frame-drawn"
                } else {
                    "frame-not-drawn"
                };
                push(out, kind, o.name_lossy());
                continue;
            }
            class::WAVE_SOUND => {
                let file = o
                    .chunk
                    .as_ref()
                    .and_then(|c| nmo::scene::wave_sound(c).ok())
                    .and_then(|s| s.file_name)
                    .map(|f| String::from_utf8_lossy(&f).into_owned())
                    .unwrap_or_default();
                push(
                    out,
                    "sound-unplayed",
                    format!("{} <{file}>", o.name_lossy()),
                );
                continue;
            }
            class::CURVE | class::CURVE_POINT if !curve_read.get(i).copied().unwrap_or(false) => {
                push(out, "curve-unused", o.name_lossy());
                continue;
            }
            _ => {}
        }
        if is_animation(o.class_id) {
            // An animation is a loss only where a block plays it. The exporters bake one for every object, with
            // keys that never leave the object's place; one that would move is named so.
            let moves = o.chunk.as_ref().is_some_and(animation_moves);
            let kind = if played {
                "animation-played"
            } else {
                "animation-unplayed"
            };
            let note = if moves { " (its keys move)" } else { "" };
            push(out, kind, format!("{}{note}", o.name_lossy()));
            continue;
        }
        let what = match o.class_id {
            24..=26 => "sound",
            27..=29 => "2D entity",
            36 | 43 => "curve",
            40 | 42 => "character",
            50 | 51 => "grid",
            53 => "patch mesh",
            _ => "other",
        };
        push(
            out,
            "class-skipped",
            format!("{what} (class {}) {}", o.class_id, o.name_lossy()),
        );
    }
    for (_, o) in doc.of_class(class::MATERIAL) {
        let Some(m) = o.chunk.as_ref().and_then(|c| nmo::scene::material(c).ok()) else {
            continue;
        };
        // VXCMP_GREATER is 5, GREATEREQUAL 7: what the renderer's cutoff does
        if m.alpha_test && !matches!(m.alpha_func, 5 | 7) {
            push(
                out,
                "material-mode",
                format!("{} alpha func {}", o.name_lossy(), m.alpha_func),
            );
        }
    }
}

/// The issues of the map at `path` with the game's files `data`.
pub fn map_issues(path: &Path, data: &Data) -> Vec<Issue> {
    let mut out = Vec::new();
    // a folder is a level in the open format (maps() lists it when it holds level.gltf)
    let open = path.is_dir();
    let source = if open {
        Source::Open(path.to_path_buf())
    } else {
        Source::Nmo(path.to_path_buf())
    };
    let data = data.for_source(&source);
    let loaded = match data.load(&source) {
        Ok(l) => l,
        Err(e) => {
            push(&mut out, "load-failed", e);
            return out;
        }
    };
    for w in &loaded.warnings {
        push(&mut out, "load-warning", w.clone());
    }
    let level = &loaded.level;
    let data = data.with_parts(level);
    let modules = scene::load_modules(level, &data);
    let start = level
        .start_ball
        .as_deref()
        .and_then(crate::sim::BallKind::from_name)
        .unwrap_or(crate::sim::BallKind::Wood);
    let sim = Sim::with_rules(level, start, &modules, false, crate::replay::VERSION);
    let placements = scene::placements(level, &modules, sim.loose(), sim.module_bodies());
    out.extend(level_issues(
        level,
        &modules,
        &sim,
        &placements,
        &data,
        open,
    ));
    if !open {
        file_issues(path, &mut out);
    }
    out
}

/// The `.nmo` files and the open-format level folders (those holding a `level.gltf`) of `root` (a file or a folder
/// walked a few levels deep, links not followed), sorted.
pub(crate) fn maps(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut todo = vec![(root.to_path_buf(), 0usize)];
    while let Some((path, depth)) = todo.pop() {
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_file() {
            let nmo = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("nmo"));
            if nmo && out.len() < MAX_MAPS {
                out.push(path);
            }
        } else if meta.is_dir()
            && std::fs::symlink_metadata(path.join("level.gltf")).is_ok_and(|m| m.is_file())
        {
            if out.len() < MAX_MAPS {
                out.push(path);
            }
        } else if meta.is_dir() && depth < MAX_DEPTH {
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };
            for e in entries.flatten().take(MAX_MAPS) {
                todo.push((e.path(), depth.saturating_add(1)));
            }
        }
    }
    out.sort();
    out
}

fn csv_field(s: &str) -> String {
    // a name may hold anything; nothing a spreadsheet would take for a formula starts a field
    let s: String = s.chars().filter(|c| !c.is_control()).collect();
    let s = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s
    };
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Audits every map under each of `roots`, with the copy (when `data` has one) and without it; writes the CSV when
/// asked, and prints the summary: the kinds of issue by the number of maps they hit.
pub fn run(
    roots: &[PathBuf],
    data: &Data,
    csv: Option<&Path>,
    prepare: &dyn Fn(bool),
) -> Result<(), String> {
    let files: Vec<PathBuf> = roots.iter().flat_map(|r| maps(r)).collect();
    if files.is_empty() {
        return Err("--audit: no .nmo file or open-format level there".into());
    }
    let passes: Vec<(&str, Data)> = if data.has_copy() {
        vec![("with", data.clone()), ("without", Data::new(None))]
    } else {
        vec![("without", data.clone())]
    };
    let mut rows = String::from("map,copy,kind,count,names\n");
    // per pass and kind: the maps hit and the items in all
    let mut totals: Vec<(&str, &str, usize, usize)> = Vec::new();
    for (copy, data) in &passes {
        // the texture packs and the interface as the game has them with this copy, or with none
        prepare(data.has_copy());
        for file in &files {
            let shown = file.display().to_string();
            let issues = map_issues(file, data);
            for i in &issues {
                rows.push_str(&format!(
                    "{},{copy},{},{},{}\n",
                    csv_field(&shown),
                    i.kind,
                    i.count,
                    csv_field(&i.names.join("; "))
                ));
                match totals.iter_mut().find(|t| t.0 == *copy && t.1 == i.kind) {
                    Some(t) => {
                        t.2 = t.2.saturating_add(1);
                        t.3 = t.3.saturating_add(i.count);
                    }
                    None => totals.push((copy, i.kind, 1, i.count)),
                }
                if csv.is_none() {
                    println!(
                        "{shown} [{copy} the copy] {} x{}: {}",
                        i.kind,
                        i.count,
                        i.names.join("; ")
                    );
                }
            }
        }
    }
    if let Some(path) = csv {
        crate::hints::write_plain(path, &rows)?;
    }
    println!("audit: {} maps", files.len());
    for (copy, _) in &passes {
        println!("{copy} the copy, kinds of issue by the maps they hit:");
        let mut t: Vec<_> = totals.iter().filter(|t| t.0 == *copy).collect();
        t.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.cmp(b.1)));
        for (_, kind, maps, items) in t {
            let what = KINDS.iter().find(|k| k.0 == *kind).map_or("", |k| k.1);
            println!("  {maps:4} maps {items:6} items  {kind}: {what}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use level::{Group, Material, Mesh, Object, Part};

    fn material() -> Material {
        Material {
            name: "m".into(),
            diffuse: [1.0; 4],
            ambient: [1.0; 3],
            emissive: [0.0; 3],
            specular: [0.0; 3],
            power: 0.0,
            texture: None,
            normal: None,
            parallax: 0.0,
            blend: None,
            alpha_cutoff: None,
            z_write: true,
            two_sided: false,
            unlit: false,
            texgen: None,
        }
    }

    /// A square of two triangles at `at`, with the level's material 0 or without any, in the given groups.
    fn add(
        level: &mut Level,
        name: &str,
        at: [f32; 3],
        with_material: bool,
        visible: bool,
        groups: &[&str],
    ) -> usize {
        let mesh = level.meshes.len();
        level.meshes.push(Mesh {
            name: format!("{name}_Mesh"),
            positions: vec![
                [-5.0, 0.0, -5.0],
                [5.0, 0.0, -5.0],
                [5.0, 0.0, 5.0],
                [-5.0, 0.0, 5.0],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            uvs: vec![[0.0; 2]; 4],
            parts: vec![Part {
                material: with_material.then_some(0),
                indices: vec![0, 2, 1, 0, 3, 2],
            }],
            channels: Vec::new(),
            look: None,
        });
        let object = level.objects.len();
        level.objects.push(Object {
            name: name.into(),
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [at[0], at[1], at[2], 1.0],
            ],
            mesh: Some(mesh),
            visible,
            sprite: None,
        });
        for g in groups {
            match level.groups.iter_mut().find(|x| x.name == *g) {
                Some(x) => x.members.push(object),
                None => level.groups.push(Group {
                    name: (*g).into(),
                    members: vec![object],
                }),
            }
        }
        object
    }

    fn level() -> Level {
        let mut level = Level::default();
        level.materials.push(material());
        add(
            &mut level,
            "Floor",
            [0.0, 0.0, 0.0],
            true,
            true,
            &["Phys_Floors"],
        );
        level
    }

    fn issues(level: &Level) -> Vec<Issue> {
        let modules = Modules::default();
        let sim = Sim::new(level, crate::sim::BallKind::Wood);
        let placements = scene::placements(level, &modules, sim.loose(), sim.module_bodies());
        level_issues(level, &modules, &sim, &placements, &Data::new(None), false)
    }

    fn count(found: &[Issue], kind: &str) -> usize {
        found.iter().find(|i| i.kind == kind).map_or(0, |i| i.count)
    }

    #[test]
    fn a_hidden_floor_is_reported_and_a_hidden_stopper_apart() {
        let mut level = level();
        add(
            &mut level,
            "Wall",
            [20.0, 0.0, 0.0],
            true,
            false,
            &["Phys_Floors"],
        );
        add(
            &mut level,
            "Stop",
            [40.0, 0.0, 0.0],
            true,
            false,
            &["Phys_FloorStopper"],
        );
        let found = issues(&level);
        assert_eq!(count(&found, "body-hidden"), 1);
        assert_eq!(count(&found, "stopper-hidden"), 1);
        assert_eq!(count(&found, "body-no-material"), 0);
        let hidden = found
            .iter()
            .find(|i| i.kind == "body-hidden")
            .expect("reported");
        assert_eq!(hidden.names, vec!["Wall @20,0,0".to_owned()]);
    }

    /// The original physicalizes every member of `Phys_Floors`, seen or not: its own levels keep invisible
    /// colliders there (`A02_inv_Coll_00`, `A07_Floor_Invisible_Col`...), so a hidden floor is a body.
    #[test]
    fn a_hidden_floor_is_solid_as_the_originals_invisible_colliders() {
        let mut level = level();
        let wall = add(
            &mut level,
            "A02_inv_Coll_00",
            [20.0, 0.0, 0.0],
            true,
            false,
            &["Phys_Floors"],
        );
        let bodies = crate::sim::static_objects(&level, &[]);
        assert!(bodies.iter().any(|(object, _)| *object == wall));
    }

    /// A file object saved for a file (references are indices), with one section of words.
    fn file_object(class_id: u32, name: &str, section: u32, fields: &[u32]) -> nmo::Object {
        let mut words = vec![section, 0];
        words.extend_from_slice(fields);
        nmo::Object {
            id: 0,
            class_id,
            name: name.as_bytes().to_vec(),
            chunk: Some(nmo::chunk::Chunk {
                data_version: 10,
                class_id: class_id as u8,
                chunk_version: 7,
                options: nmo::chunk::OPTION_FILE,
                words,
            }),
        }
    }

    /// An object animation of `entity` with linear position keys at the given x.
    fn animation(name: &str, entity: u32, xs: &[f32]) -> nmo::Object {
        let mut fields = vec![0; 7];
        fields.extend([0, entity, 100.0f32.to_bits()]);
        fields.extend([0x637c_4301, 1 + 4 * xs.len() as u32, xs.len() as u32]);
        for (k, x) in xs.iter().enumerate() {
            fields.extend([(k as f32).to_bits(), x.to_bits(), 0, 0]);
        }
        fields.push(0);
        file_object(15, name, 0x0400_0000, &fields)
    }

    fn doc_found(objects: Vec<nmo::Object>) -> Vec<Issue> {
        let mut out = Vec::new();
        doc_issues(&nmo::Document { objects }, &mut out);
        out
    }

    /// Virtools plays no animation by itself: the bake an exporter leaves on every object moves nothing in the
    /// original, so it is counted, not reported as a loss; one whose keys would move is named so.
    /// A frame is drawn as the game draws it (shown, a material with a texture); a hidden one or one without a
    /// texture waits for a script. A sound is read but not played. A curve nothing reaches is an exporter's shape;
    /// one a parameter names stays among the classes not read.
    #[test]
    fn frames_sounds_and_curves_are_told_apart() {
        let f = f32::to_bits;
        let material =
            |texture: u32| file_object(30, "m", 0x1000, &[!0, !0, 0, 0, 0, texture, 0, 0, 0]);
        let frame = |name: &str, material: u32| {
            let rect = [f(0.0), f(0.0), f(80.0), f(60.0)];
            let mut fields = vec![0x4_0000];
            fields.extend(rect);
            fields.push(material);
            file_object(27, name, 0x0010_f000, &fields)
        };
        let mut hidden = frame("hidden", 1);
        if let Some(c) = &mut hidden.chunk {
            // the hidden mark before the frame's part
            let mut words = vec![0x4, 2];
            words.extend(c.words.iter().copied());
            c.words = words;
        }
        let found = doc_found(vec![
            file_object(31, "t", 0x1, &[]),
            material(0),
            material(u32::MAX),
            frame("signature", 1),
            frame("plain", 2),
            hidden,
            file_object(
                25,
                "bgm",
                0x1000,
                &[0, 5, u32::from_le_bytes(*b"a.wa"), u32::from(b'v')],
            ),
            file_object(43, "Line01", 0xffc0_0000, &[1, 8, f(0.0), 0, 1]),
            file_object(36, "", 0x1, &[]),
            file_object(43, "Path", 0xffc0_0000, &[0, f(0.0), 0, 1]),
            file_object(45, "curve", 0x40, &[0, 0, 2, 9]),
        ]);
        assert_eq!(count(&found, "frame-drawn"), 1);
        assert_eq!(count(&found, "frame-not-drawn"), 2);
        assert_eq!(count(&found, "sound-unplayed"), 1);
        assert!(
            found
                .iter()
                .any(|i| i.names.contains(&"bgm <a.wav>".to_string()))
        );
        assert_eq!(count(&found, "curve-unused"), 2);
        assert_eq!(count(&found, "class-skipped"), 1);
    }

    #[test]
    fn an_animation_nothing_plays_is_only_counted() {
        let found = doc_found(vec![
            animation("Still", 0, &[5.0, 5.0, 5.0]),
            animation("Lift", 0, &[5.0, 9.0]),
        ]);
        assert_eq!(count(&found, "animation-unplayed"), 2);
        assert_eq!(count(&found, "animation-played"), 0);
        assert_eq!(count(&found, "class-skipped"), 0);
        let names = &found.first().expect("one kind").names;
        assert_eq!(
            names,
            &["Still".to_owned(), "Lift (its keys move)".to_owned()]
        );
    }

    /// A block that reads an animation (a parameter holding one) plays it in the original: here it stands still.
    #[test]
    fn an_animation_a_block_reads_is_a_loss() {
        // a local parameter of the object-animation type whose value is object 0
        let holder = file_object(45, "Animation", 0x40, &[0x4347_6550, 0x11e5_2c0e, 2, 0]);
        let found = doc_found(vec![animation("Lift", 0, &[5.0, 9.0]), holder]);
        assert_eq!(count(&found, "animation-played"), 1);
        assert_eq!(count(&found, "animation-unplayed"), 0);
    }

    /// The original draws any visible object, with or without a material: a floor the ball runs on is seen.
    #[test]
    fn a_floor_without_a_material_is_drawn_plain() {
        let mut level = level();
        let plain = add(
            &mut level,
            "Loft01",
            [20.0, 0.0, 0.0],
            false,
            true,
            &["Phys_Floors"],
        );
        let sim = Sim::new(&level, crate::sim::BallKind::Wood);
        let placements = scene::placements(
            &level,
            &Modules::default(),
            sim.loose(),
            sim.module_bodies(),
        );
        assert!(
            placements
                .iter()
                .any(|p| p.object == plain && p.marker.is_none())
        );
        let found = issues(&level);
        assert_eq!(count(&found, "body-no-material"), 0);
        assert_eq!(count(&found, "body-plain"), 1);
    }

    /// The original makes a module at every member of the module's group, whatever the member is called, looks like
    /// or whether it is hidden; the stand-in itself is never seen.
    #[test]
    fn a_member_of_a_modules_group_is_a_stand_in_whatever_it_looks_like() {
        let mut level = level();
        let blender = add(
            &mut level,
            "P_Modul_26_01_.001",
            [20.0, 0.0, 0.0],
            true,
            true,
            &["P_Modul_26", "Sector_01"],
        );
        let hidden = add(
            &mut level,
            "Anything",
            [40.0, 0.0, 0.0],
            false,
            false,
            &["P_Box", "Sector_01"],
        );
        let own = add(
            &mut level,
            "P_Box_03",
            [60.0, 0.0, 0.0],
            false,
            true,
            &["Sector_01"],
        );
        let floor = add(
            &mut level,
            "P_Box_04",
            [80.0, 0.0, 0.0],
            true,
            true,
            &["Phys_Floors"],
        );
        assert_eq!(scene::module_of(&level, blender), Some("P_Modul_26"));
        assert_eq!(scene::module_of(&level, hidden), Some("P_Box"));
        assert_eq!(
            scene::module_of(&level, own),
            Some("P_Box"),
            "the original's levels: by the name"
        );
        assert_eq!(
            scene::module_of(&level, floor),
            None,
            "a textured object in no module's group"
        );
        let sim = Sim::new(&level, crate::sim::BallKind::Wood);
        let placements = scene::placements(
            &level,
            &Modules::default(),
            sim.loose(),
            sim.module_bodies(),
        );
        assert!(
            placements.iter().any(|p| p.object == blender),
            "no module there: the stand-in is what is seen"
        );
        let modules = Modules {
            models: vec![Level::default()],
            names: vec!["P_Modul_26".into()],
        };
        let placements = scene::placements(&level, &modules, sim.loose(), sim.module_bodies());
        assert!(
            !placements.iter().any(|p| p.object == blender),
            "its module is there: the stand-in is not drawn"
        );
        assert!(placements.iter().any(|p| p.object == floor));
    }

    /// Only the game's own modules are made: a group a mod reads (`P_Modul_Portal`) is nothing to the level init,
    /// so its members are never bodies; a module whose file is missing stays solid as its stand-in.
    #[test]
    fn a_group_of_no_module_of_the_game_makes_no_body() {
        let mut level = level();
        let portal = add(
            &mut level,
            "Portal_In",
            [20.0, 0.0, 0.0],
            false,
            true,
            &["P_Modul_Portal"],
        );
        let module = add(
            &mut level,
            "P_Modul_03_01",
            [40.0, 0.0, 0.0],
            false,
            true,
            &["P_Modul_03"],
        );
        let bodies: Vec<usize> = crate::sim::static_objects(&level, &[])
            .into_iter()
            .map(|s| s.0)
            .collect();
        assert!(!bodies.contains(&portal));
        assert!(bodies.contains(&module));
        let sim = Sim::new(&level, crate::sim::BallKind::Wood);
        let placements = scene::placements(
            &level,
            &Modules::default(),
            sim.loose(),
            sim.module_bodies(),
        );
        assert!(
            placements.iter().any(|p| p.object == module),
            "a solid is seen, plain when it has no material"
        );
        let found = issues(&level);
        assert_eq!(count(&found, "standin-solid"), 1);
        assert_eq!(count(&found, "module-missing"), 2);
    }

    #[test]
    fn a_csv_field_is_quoted_and_starts_no_formula() {
        assert_eq!(csv_field("a \"b\", c"), "\"a \"\"b\"\", c\"");
        assert_eq!(csv_field("=1+1"), "\"'=1+1\"");
        assert_eq!(csv_field("a\nb\r"), "\"ab\"");
    }

    #[test]
    fn a_folder_of_maps_is_walked_without_following_links() {
        let dir = std::env::temp_dir().join(format!("drahtseilakt-audit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a/b")).expect("made");
        std::fs::write(dir.join("a/b/One.NMO"), b"x").expect("written");
        std::fs::write(dir.join("a/two.nmo"), b"x").expect("written");
        std::fs::write(dir.join("a/notes.txt"), b"x").expect("written");
        // an open-format level is one entry, its files are not looked into
        std::fs::create_dir_all(dir.join("a/open")).expect("made");
        std::fs::write(dir.join("a/open/level.gltf"), b"{}").expect("written");
        std::fs::write(dir.join("a/open/Inner.nmo"), b"x").expect("written");
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.join("a"), dir.join("link")).expect("linked");
        let found = maps(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.len(), 3, "{found:?}");
    }
}
