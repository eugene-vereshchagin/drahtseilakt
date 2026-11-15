// The converter's audit (`--convert-audit FILE|DIR`): every map is loaded from its `.nmo`, exported to the open
// format, loaded again from the export and the two levels compared — objects, meshes, materials and their pictures,
// groups, settings, frames, texture slots and the map's own scripts — and then both are played without a window for
// `--ticks` ticks (3000 by default) with the same fixed keys, comparing what is where after every tick (the ball, the
// rules' state, the objects the scripts move, the loose bodies, the modules' parts). Original and converted copies of
// the same level double as a test of the converter (decisions.md): a map that differs is a loss of the converter.
// One tab-separated line a map (`convert-audit` first), the first difference named; the export is removed after.

use crate::assets::{Data, Source};
use level::Level;
use std::path::{Path, PathBuf};

/// How many differences a map lists (the first ones; the count is whole).
const MAX_LISTED: usize = 6;

/// One map's verdict.
pub struct Verdict {
    pub converts: Result<(), String>,
    pub loads: Result<(), String>,
    /// The differences of the two levels' contents (empty: the same).
    pub content: Vec<String>,
    /// The first tick whose state differs, and what differed; none: the same for all ticks played.
    pub behaviour: Result<Option<(u64, String)>, String>,
    pub ticks: u64,
}

/// The keys of tick `t`: a fixed sequence that rolls the ball about (forward mostly, turning and backing off, the
/// camera turned now and then), the same for both copies.
pub fn keys(t: u64) -> u8 {
    use crate::sim::input::*;
    // a fixed pseudo-random choice every half second (splitmix of the half second's number)
    let mut z = (t / 30).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    const CHOICES: [u8; 10] = [
        UP,
        UP,
        UP,
        UP | LEFT,
        UP | RIGHT,
        LEFT,
        RIGHT,
        DOWN,
        0,
        SHIFT | LEFT,
    ];
    CHOICES[(z % CHOICES.len() as u64) as usize]
}

/// What is compared each tick, by part: the ball; the rules (sector, lives, points, camera, the end); the objects the
/// scripts move or hide; the loose bodies; the modules' parts.
pub const PARTS: [&str; 5] = [
    "ball",
    "rules",
    "script objects",
    "loose bodies",
    "module parts",
];

fn first_differences(a: &Level, b: &Level, data_a: &Data, data_b: &Data) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut note = |s: String| out.push(s);
    // objects, in order
    if a.objects.len() != b.objects.len() {
        note(format!(
            "objects: {} -> {}",
            a.objects.len(),
            b.objects.len()
        ));
    }
    let bits3 = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    for (i, (x, y)) in a.objects.iter().zip(&b.objects).enumerate() {
        if x.name != y.name {
            note(format!("object {i} name {:?} -> {:?}", x.name, y.name));
            continue;
        }
        let mx: Vec<f32> = x.matrix.iter().flatten().copied().collect();
        let my: Vec<f32> = y.matrix.iter().flatten().copied().collect();
        // -0 and 0 are the same place
        if mx
            .iter()
            .zip(&my)
            .any(|(p, q)| p != q && !(p.is_nan() && q.is_nan()))
        {
            note(format!("object {:?} matrix", x.name));
        }
        if x.visible != y.visible {
            note(format!("object {:?} visible", x.name));
        }
        if x.sprite != y.sprite {
            note(format!("object {:?} sprite", x.name));
        }
        if x.parent != y.parent {
            note(format!(
                "object {:?} parent {:?} -> {:?}",
                x.name, x.parent, y.parent
            ));
        }
        match (mesh(a, x.mesh), mesh(b, y.mesh)) {
            (None, None) => {}
            (Some(p), Some(q)) => {
                let same = bits3(p.positions.as_flattened()) == bits3(q.positions.as_flattened())
                    && bits3(p.normals.as_flattened()) == bits3(q.normals.as_flattened())
                    && bits3(p.uvs.as_flattened()) == bits3(q.uvs.as_flattened());
                if !same {
                    note(format!("object {:?} mesh vertices", x.name));
                }
                let parts = |m: &level::Mesh| {
                    m.parts
                        .iter()
                        .filter(|p| !p.indices.is_empty())
                        .map(|p| (p.material, p.indices.clone()))
                        .collect::<Vec<_>>()
                };
                if parts(p) != parts(q) {
                    note(format!("object {:?} mesh faces or materials", x.name));
                }
                if p.channels != q.channels {
                    note(format!("object {:?} mesh channels", x.name));
                }
                if p.look.is_some() != q.look.is_some() {
                    note(format!("object {:?} mesh look", x.name));
                }
            }
            (p, q) => note(format!(
                "object {:?} mesh {} -> {}",
                x.name,
                p.is_some(),
                q.is_some()
            )),
        }
    }
    // materials, with the pictures their textures draw
    if a.materials.len() != b.materials.len() {
        note(format!(
            "materials: {} -> {}",
            a.materials.len(),
            b.materials.len()
        ));
    }
    let mut pics: std::collections::HashMap<
        (bool, usize),
        Option<(u32, u32, u64, Option<[u8; 3]>)>,
    > = std::collections::HashMap::new();
    let mut picture = |side: bool,
                       l: &Level,
                       t: Option<usize>|
     -> Option<Option<(u32, u32, u64, Option<[u8; 3]>)>> {
        let t = t?;
        Some(
            pics.entry((side, t))
                .or_insert_with(|| {
                    let tex = l.textures.get(t)?;
                    let data = if side { data_b } else { data_a };
                    let (w, h, rgba) = data.texture(tex)?;
                    Some((w, h, gamedata::fnv1a(&rgba), tex.color_key))
                })
                .clone(),
        )
    };
    for (x, y) in a.materials.iter().zip(&b.materials) {
        let fields = |m: &level::Material| {
            format!(
                "{:?}",
                (
                    (
                        &m.name, m.diffuse, m.ambient, m.emissive, m.specular, m.power
                    ),
                    (
                        m.blend,
                        m.alpha_cutoff,
                        m.z_write,
                        m.two_sided,
                        m.unlit,
                        m.texgen,
                        m.parallax
                    )
                )
            )
        };
        if fields(x) != fields(y) {
            note(format!("material {:?} fields", x.name));
        }
        if picture(false, a, x.texture) != picture(true, b, y.texture) {
            note(format!("material {:?} texture picture", x.name));
        }
    }
    // groups
    let groups = |l: &Level| {
        l.groups
            .iter()
            .map(|g| (g.name.clone(), g.members.clone()))
            .collect::<Vec<_>>()
    };
    if groups(a) != groups(b) {
        note(format!("groups ({} -> {})", a.groups.len(), b.groups.len()));
    }
    if a.sky != b.sky || a.start_ball != b.start_ball {
        note("sky or start ball".into());
    }
    if a.settings != b.settings {
        note("settings".into());
    }
    if a.frames != b.frames {
        note(format!(
            "2D frames: {} -> {}",
            a.frames.len(),
            b.frames.len()
        ));
    }
    let slots = |l: &Level| l.texture_slots.iter().map(|s| s.1.len()).sum::<usize>();
    if slots(a) != slots(b) {
        note(format!("texture slots: {} -> {}", slots(a), slots(b)));
    }
    // the scripts
    match (&a.scripts, &b.scripts) {
        (None, None) => {}
        (Some(x), Some(y)) => {
            for d in script_differences(x, y) {
                note(d);
            }
        }
        (x, y) => note(format!("scripts {} -> {}", x.is_some(), y.is_some())),
    }
    out
}

fn mesh(l: &Level, m: Option<usize>) -> Option<&level::Mesh> {
    m.and_then(|k| l.meshes.get(k))
        .filter(|m| level::open::written(m))
}

/// What differs between two scripts' graphs: their census (blocks, links, parameters, operations) and each part.
fn script_differences(x: &script::Graph, y: &script::Graph) -> Vec<String> {
    use script::{Node, ObjKind};
    let mut out = Vec::new();
    let census = |g: &script::Graph| {
        let mut c = [0usize; 5];
        for n in &g.nodes {
            let k = match n {
                Node::Block(_) => 0,
                Node::Link(_) => 1,
                Node::Io(_) => 2,
                Node::Param(_) => 3,
                Node::Op(_) => 4,
                Node::None => continue,
            };
            c[k] += 1;
        }
        c
    };
    let (cx, cy) = (census(x), census(y));
    if cx != cy {
        out.push(format!(
            "script census (blocks, links, ios, params, ops) {cx:?} -> {cy:?}"
        ));
    }
    // object kinds: a mesh or texture index may be another in the export (empty meshes and repeated pictures)
    let kind = |k: &ObjKind| std::mem::discriminant(k);
    let objs = |g: &script::Graph| {
        g.objects
            .iter()
            .map(|o| (o.name.clone(), o.class, kind(&o.kind)))
            .collect::<Vec<_>>()
    };
    let (ox, oy) = (objs(x), objs(y));
    if ox != oy {
        let first = ox.iter().zip(&oy).position(|(p, q)| p != q);
        out.push(match first {
            Some(i) => format!(
                "script objects (first {i} {:?}: {:?} -> {:?})",
                x.objects[i].name, x.objects[i].kind, y.objects[i].kind
            ),
            None => format!("script objects ({} -> {})", ox.len(), oy.len()),
        });
    }
    if x.nodes != y.nodes {
        let first = x
            .nodes
            .iter()
            .zip(&y.nodes)
            .position(|(p, q)| p != q)
            .unwrap_or(x.nodes.len().min(y.nodes.len()));
        let what = match x.nodes.get(first) {
            Some(Node::Block(b)) => format!("block {:?}", b.name),
            Some(Node::Param(p)) => format!("param {:?}", p.name),
            Some(Node::Link(_)) => "link".into(),
            Some(Node::Io(_)) => "io".into(),
            Some(Node::Op(_)) => "op".into(),
            Some(Node::None) | None => "count".into(),
        };
        out.push(format!("script nodes (first {first}: {what})"));
    }
    if x.arrays != y.arrays {
        out.push("script arrays".into());
    }
    if x.scripts != y.scripts {
        out.push("script list".into());
    }
    if x.links_from != y.links_from {
        out.push("script links".into());
    }
    if x.priorities != y.priorities {
        out.push(format!(
            "script priorities ({} -> {})",
            x.priorities.len(),
            y.priorities.len()
        ));
    }
    if x.start_off != y.start_off {
        out.push(format!(
            "script start-off ({} -> {})",
            x.start_off.len(),
            y.start_off.len()
        ));
    }
    if x.groups != y.groups {
        out.push("script groups".into());
    }
    if x.attributes != y.attributes {
        out.push("script attributes".into());
    }
    if x.waves != y.waves {
        out.push(format!(
            "script waves ({} -> {})",
            x.waves.len(),
            y.waves.len()
        ));
    }
    out
}

/// Converts, reloads, compares and plays one map; `out` is the folder its export goes to (removed after unless
/// `keep`).
pub fn check(
    path: &Path,
    data: &Data,
    out: &Path,
    ticks: u64,
    keep: bool,
    o: &crate::Options,
) -> Verdict {
    let mut v = Verdict {
        converts: Ok(()),
        loads: Ok(()),
        content: Vec::new(),
        behaviour: Ok(None),
        ticks,
    };
    let src = Source::Nmo(path.to_path_buf());
    let data_a = data.for_source(&src);
    let a = match data_a.load(&src) {
        Ok(l) => l,
        Err(e) => {
            v.converts = Err(format!("the map does not load: {e}"));
            return v;
        }
    };
    let stem = path
        .file_stem()
        .map_or("map".into(), |s| s.to_string_lossy().into_owned());
    let dir = out.join(&stem);
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(e) = crate::assets::export(&data_a, &a, &dir, false) {
        v.converts = Err(e);
        return v;
    }
    let osrc = Source::Open(dir.clone());
    let data_b = data.for_source(&osrc);
    let b = match data_b.load(&osrc) {
        Ok(l) => l,
        Err(e) => {
            v.loads = Err(e);
            if !keep {
                let _ = std::fs::remove_dir_all(&dir);
            }
            return v;
        }
    };
    v.content = first_differences(&a.level, &b.level, &data_a, &data_b);
    drop((a, b));
    if ticks > 0 {
        let run = |source: Source| {
            let mut o = o.clone();
            o.source = source;
            o.chosen = true;
            o.headless = true;
            o.replay = None;
            o.record = None;
            crate::app::states(&o, data, ticks, &keys)
        };
        let played = (run(src), run(osrc));
        if let (Ok(x), _) = &played {
            v.ticks = x.len() as u64;
        }
        v.behaviour = match played {
            (Ok(x), Ok(y)) => Ok(x
                .iter()
                .zip(&y)
                .position(|(p, q)| p != q)
                .map(|t| {
                    let (p, q) = (&x[t], &y[t]);
                    let parts: Vec<&str> = PARTS
                        .iter()
                        .zip(p.iter().zip(q))
                        .filter(|(_, (m, n))| m != n)
                        .map(|(name, _)| *name)
                        .collect();
                    (t as u64 + 1, parts.join(", "))
                })
                .or_else(|| {
                    (x.len() != y.len()).then(|| (x.len().min(y.len()) as u64, "run length".into()))
                })),
            (Err(e), _) => Err(format!("the map does not play: {e}")),
            (_, Err(e)) => Err(format!("the export does not play: {e}")),
        };
    }
    if !keep {
        let _ = std::fs::remove_dir_all(&dir);
    }
    v
}

fn clean(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// Runs the audit over every map under `roots`, one line each.
pub fn run(
    roots: &[PathBuf],
    data: &Data,
    out: &Path,
    ticks: u64,
    o: &crate::Options,
) -> Result<(), String> {
    let files: Vec<PathBuf> = roots
        .iter()
        .flat_map(|r| crate::audit::maps(r))
        .filter(|p| p.is_file())
        .collect();
    if files.is_empty() {
        return Err("--convert-audit: no .nmo file there".into());
    }
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let keep = std::env::var_os("DRAHTSEILAKT_KEEP_EXPORT").is_some();
    for f in &files {
        let t0 = std::time::Instant::now();
        let v = check(f, data, out, ticks, keep, o);
        let r = |x: &Result<(), String>| match x {
            Ok(()) => "yes".to_owned(),
            Err(e) => format!("no: {}", clean(e)),
        };
        let content = if v.content.is_empty() {
            "same".to_owned()
        } else {
            let mut s: Vec<String> = v
                .content
                .iter()
                .take(MAX_LISTED)
                .map(|d| clean(d))
                .collect();
            if v.content.len() > MAX_LISTED {
                s.push(format!("... {} in all", v.content.len()));
            }
            s.join("; ")
        };
        let behaviour = match &v.behaviour {
            _ if v.converts.is_err() || v.loads.is_err() => "-".to_owned(),
            Ok(None) => format!("same for {} ticks", v.ticks),
            Ok(Some((t, what))) => format!("differs at tick {t}: {what}"),
            Err(e) => format!("not played: {}", clean(e)),
        };
        println!(
            "convert-audit\t{}\t{}\t{}\t{}\t{}\t{:.1}s",
            f.display(),
            r(&v.converts),
            r(&v.loads),
            content,
            behaviour,
            t0.elapsed().as_secs_f64()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_keys_are_a_fixed_sequence_that_moves() {
        let a: Vec<u8> = (0..3000).map(super::keys).collect();
        let b: Vec<u8> = (0..3000).map(super::keys).collect();
        assert_eq!(a, b);
        assert!(
            a.iter()
                .filter(|&&k| k & crate::sim::input::UP != 0)
                .count()
                > 1000
        );
        assert_eq!(a[0], a[29], "one choice holds half a second");
    }
}
