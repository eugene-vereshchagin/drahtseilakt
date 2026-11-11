// nmo-dump: prints what a Virtools file holds. Default: the container; --objects: every object with its class;
// --check: decode every scene object and print only what fails (for runs over whole map collections);
// --arrays: every data array with its rows; --params: every parameter with its value or source;
// --scripts: the behaviour graphs' building blocks by prototype, with counts (the survey of community maps);
// --extras: the 2D entities, sounds, curves and patch meshes of a file, each with what it holds and who names it.

use nmo::{Document, class, scene};
use std::process::ExitCode;

fn decode(doc: &Document) -> Vec<String> {
    let mut errors = Vec::new();
    for (i, o) in doc.objects.iter().enumerate() {
        let Some(chunk) = &o.chunk else { continue };
        let result = match o.class_id {
            class::MESH => scene::mesh(chunk).map(|_| ()),
            class::MATERIAL => scene::material(chunk).map(|_| ()),
            class::TEXTURE => scene::texture(chunk).map(|_| ()),
            class::GROUP => scene::group(chunk).map(|_| ()),
            class::OBJECT_3D
            | class::ENTITY_3D
            | class::CAMERA
            | class::TARGET_CAMERA
            | class::LIGHT
            | class::TARGET_LIGHT => scene::entity(chunk).map(|_| ()),
            class::SPRITE_3D => scene::entity(chunk)
                .and_then(|_| scene::sprite(chunk))
                .map(|_| ()),
            _ => Ok(()),
        };
        if let Err(e) = result {
            errors.push(format!(
                "object {i} (class {}) {:?}: {e}",
                o.class_id,
                o.name_lossy()
            ));
        }
    }
    errors
}

/// The objects around the scene (2D entities, sounds, curves, patch meshes), each with what it holds, whether the
/// file hides it, and who refers to it: the parameters whose value it is, the behaviours it is the target of, the
/// groups it is in. The survey behind docs/drahtseilakt/community-maps.md, "2D frames, curves and sounds".
fn extras(doc: &Document) {
    use nmo::behavior;
    let name = |i: usize| doc.get(i).map_or_else(String::new, |o| o.name_lossy());
    // the textures with no pixels in the file: the name they ask for and the sections they do hold
    for (i, o) in doc.of_class(class::TEXTURE) {
        let Some(chunk) = &o.chunk else { continue };
        let Ok(t) = scene::texture(chunk) else {
            println!("  {i:5} texture {}: unreadable", o.name_lossy());
            continue;
        };
        if t.embedded.is_some() {
            continue;
        }
        let mut sections = Vec::new();
        let mut at = 0usize;
        while let (Some(&id), Some(&next)) =
            (chunk.words.get(at), chunk.words.get(at.saturating_add(1)))
        {
            let end = if next == 0 {
                chunk.words.len()
            } else {
                next as usize
            };
            let head: Vec<String> = chunk
                .words
                .get(at.saturating_add(2)..end.min(at.saturating_add(14)))
                .unwrap_or_default()
                .iter()
                .map(|w| format!("{w:x}"))
                .collect();
            sections.push(format!(
                "{id:#x}[{}: {}]",
                end.saturating_sub(at).saturating_sub(2),
                head.join(" ")
            ));
            if next == 0 || end <= at {
                break;
            }
            at = end;
        }
        for (_, m) in doc.of_class(class::MATERIAL) {
            if let Some(mat) = m.chunk.as_ref().and_then(|c| scene::material(c).ok())
                && mat.texture == Some(i)
            {
                sections.push(format!(
                    "USED BY {} diffuse {:?}",
                    m.name_lossy(),
                    mat.diffuse
                ));
            }
        }
        println!(
            "  {i:5} texture {:24} file {:?} dv {} | {}",
            o.name_lossy(),
            t.file_name
                .map(|f| String::from_utf8_lossy(&f).into_owned()),
            chunk.data_version,
            sections.join(" ")
        );
    }
    // who refers to each object, gathered in one pass over the file
    let mut referrers: Vec<Vec<String>> = vec![Vec::new(); doc.objects.len()];
    for (k, u) in doc.objects.iter().enumerate() {
        let Some(c) = &u.chunk else { continue };
        let mut refer = |to: usize, what: String| {
            if let Some(r) = referrers.get_mut(to).filter(|r| r.len() < 16) {
                r.push(what);
            }
        };
        match u.class_id {
            behavior::class::BEHAVIOR => {
                if let Some(t) = behavior::behavior(c).ok().and_then(|b| b.target) {
                    refer(t, format!("target of {k} {}", u.name_lossy()));
                }
            }
            3 | 45 => {
                if let Ok(behavior::Saved::Object(Some(x))) =
                    behavior::parameter(c).map(|p| p.value)
                {
                    refer(x, format!("parameter {k} {}", u.name_lossy()));
                }
            }
            class::GROUP => {
                for m in scene::group(c).unwrap_or_default() {
                    refer(m, format!("group {}", u.name_lossy()));
                }
            }
            _ => {}
        }
    }
    for (i, o) in doc.objects.iter().enumerate() {
        if !matches!(o.class_id, 24..=29 | 43 | 53) {
            continue;
        }
        let Some(chunk) = &o.chunk else {
            println!(
                "  {i:5} class {:2} {}: no chunk",
                o.class_id,
                o.name_lossy()
            );
            continue;
        };
        let hidden = chunk.section(0x4).is_some() || chunk.section(0x18).is_some();
        let what = match o.class_id {
            27..=29 => match scene::entity_2d(chunk) {
                Ok(e) => {
                    let mat = e.material.map(|m| {
                        let tex = doc
                            .get(m)
                            .and_then(|o| o.chunk.as_ref())
                            .and_then(|c| scene::material(c).ok())
                            .and_then(|m| m.texture)
                            .map(name);
                        format!("{} (texture {tex:?})", name(m))
                    });
                    format!(
                        "rect {:?} homogeneous {} flags {:#x} z {} material {mat:?}",
                        e.rect,
                        e.homogeneous(),
                        e.flags,
                        e.z_order
                    )
                }
                Err(e) => format!("unreadable: {e}"),
            },
            25 => match scene::wave_sound(chunk) {
                Ok(s) => format!(
                    "file {:?} loop {} gain {}",
                    s.file_name
                        .map(|f| String::from_utf8_lossy(&f).into_owned()),
                    s.looped,
                    s.gain
                ),
                Err(e) => format!("unreadable: {e}"),
            },
            43 => match scene::curve(chunk) {
                Ok(Some(c)) => format!("{} points, open {}", c.points.len(), c.open),
                Ok(None) => "no curve part".into(),
                Err(e) => format!("unreadable: {e}"),
            },
            _ => String::new(),
        };
        let mut users = referrers.get(i).cloned().unwrap_or_default();
        let own = behavior::scripts(chunk);
        if !own.is_empty() {
            users.push(format!("has {} scripts", own.len()));
        }
        println!(
            "  {i:5} class {:2} {:28} {}{what} | {}",
            o.class_id,
            o.name_lossy(),
            if hidden { "HIDDEN " } else { "" },
            users.join(", ")
        );
    }
}

/// The object animations of a file (class 15), each with the entity it moves, its length in frames, its
/// controllers (kind and key count) and how far its linear position, rotation and scale keys go from the first; then the keyed
/// animations (class 18) and every parameter of the object-animation type, which is what a block that plays one
/// reads. The survey behind docs/drahtseilakt/community-maps.md, "Object animations".
fn anims(doc: &Document) {
    for (i, o) in doc.of_class(class::OBJECT_ANIMATION) {
        let Some(chunk) = &o.chunk else { continue };
        let name = o.name_lossy();
        let Some(mut w) = chunk.section(0x0400_0000) else {
            println!("  {i:5} anim {name}: no key section");
            continue;
        };
        let mut kinds = Vec::new();
        let head = (|| {
            w.take(7)?;
            let flags = w.u32()?;
            let entity = w.u32()?;
            let length = w.f32()?;
            if flags & 0x80 != 0 {
                w.take(3)?;
            }
            for _ in 0..64 {
                let kind = w.u32()?;
                if kind == 0 {
                    break;
                }
                let size = w.u32()? as usize;
                let block = w.take(size)?;
                kinds.push((kind, block.first().copied().unwrap_or(0)));
            }
            nmo::Result::Ok((flags, entity, length))
        })();
        let Ok((flags, entity, length)) = head else {
            println!("  {i:5} anim {name}: malformed");
            continue;
        };
        // a reference is an index into the file's objects only in a chunk saved for a file
        let target = chunk
            .references_are_indices()
            .then(|| doc.get(entity as usize))
            .flatten()
            .map_or_else(|| "?".to_string(), |e| e.name_lossy());
        // how far the keys go from the first one: position, rotation (either sign of the quaternion), scale
        let spread = |keys: &[Vec<f32>], signed: bool| -> f32 {
            let Some(first) = keys.first() else {
                return 0.0;
            };
            let dist = |k: &[f32], sign: f32| -> f32 {
                k.iter()
                    .zip(first)
                    .map(|(a, b)| (a * sign - b).powi(2))
                    .sum::<f32>()
                    .sqrt()
            };
            keys.iter()
                .map(|k| {
                    if signed {
                        dist(k, 1.0).min(dist(k, -1.0))
                    } else {
                        dist(k, 1.0)
                    }
                })
                .fold(0.0, f32::max)
        };
        let (travel, turn, stretch) =
            scene::object_animation(chunk)
                .ok()
                .flatten()
                .map_or((0.0, 0.0, 0.0), |a| {
                    let p: Vec<Vec<f32>> = a.position.iter().map(|k| k.1.to_vec()).collect();
                    let r: Vec<Vec<f32>> = a.rotation.iter().map(|k| k.1.to_vec()).collect();
                    let s: Vec<Vec<f32>> = a.scale.iter().map(|k| k.1.to_vec()).collect();
                    (spread(&p, false), spread(&r, true), spread(&s, false))
                });
        let kinds: Vec<String> = kinds.iter().map(|(k, n)| format!("{k:#x}x{n}")).collect();
        println!(
            "  {i:5} anim {name} -> {target}: flags {flags:#x} length {length} travel {travel:.3} turn {turn:.3} stretch {stretch:.3} [{}]",
            kinds.join(" ")
        );
    }
    for (i, o) in doc.objects.iter().enumerate() {
        if o.class_id == 18 || o.class_id == 16 {
            println!(
                "  {i:5} keyed animation (class {}) {}",
                o.class_id,
                o.name_lossy()
            );
        }
        if o.class_id == 40 {
            println!("  {i:5} character {}", o.name_lossy());
        }
        let Some(chunk) = &o.chunk else { continue };
        if matches!(o.class_id, 2 | 3 | 45) {
            let guid = if o.class_id == 2 {
                nmo::behavior::parameter_in(chunk).ok().map(|p| p.guid)
            } else {
                nmo::behavior::parameter(chunk).ok().map(|p| p.guid)
            };
            if guid == Some(nmo::behavior::Guid(0x4347_6550, 0x11e5_2c0e)) {
                println!("  {i:5} animation parameter {}", o.name_lossy());
            }
        }
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args.first().map(String::as_str) {
        Some("--objects") | Some("--check") | Some("--scripts") | Some("--arrays")
        | Some("--params") | Some("--anims") | Some("--extras") => args.remove(0),
        _ => String::new(),
    };
    if args.is_empty() {
        eprintln!(
            "usage: nmo-dump [--objects | --check | --scripts | --arrays | --params | --anims] FILE..."
        );
        return ExitCode::from(2);
    }
    let mut failed = false;
    for path in &args {
        let result = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                let file = nmo::read(&bytes).map_err(|e| e.to_string())?;
                let doc = Document::parse(&file).map_err(|e| e.to_string())?;
                Ok((file, doc))
            });
        let (file, doc) = match result {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{path}: {e}");
                failed = true;
                continue;
            }
        };
        match mode.as_str() {
            "--check" => {
                for e in decode(&doc) {
                    eprintln!("{path}: {e}");
                    failed = true;
                }
            }
            "--scripts" => {
                let mut protos: Vec<(nmo::behavior::Guid, String, usize)> = Vec::new();
                let mut scripts = 0usize;
                for o in &doc.objects {
                    let Some(chunk) = &o.chunk else { continue };
                    if o.class_id != nmo::behavior::class::BEHAVIOR {
                        continue;
                    }
                    let Ok(b) = nmo::behavior::behavior(chunk) else {
                        continue;
                    };
                    if b.flags & nmo::behavior::FLAG_SCRIPT != 0 {
                        scripts = scripts.saturating_add(1);
                    }
                    let Some((guid, _)) = b.proto else { continue };
                    match protos.iter_mut().find(|p| p.0 == guid) {
                        Some(p) => p.2 = p.2.saturating_add(1),
                        None => protos.push((guid, o.name_lossy(), 1)),
                    }
                }
                protos.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
                println!("{path}: {scripts} scripts, {} block kinds", protos.len());
                for (guid, name, n) in protos {
                    println!("  {n:5} {guid} {name}");
                }
            }
            "--anims" => {
                println!("{path}");
                anims(&doc);
            }
            "--arrays" => {
                println!("{path}");
                for (_, o) in doc.of_class(nmo::class::DATA_ARRAY) {
                    let Some(Ok(a)) = o.chunk.as_ref().map(nmo::scene::data_array) else {
                        continue;
                    };
                    let cols: Vec<String> = a
                        .columns
                        .iter()
                        .map(|c| String::from_utf8_lossy(c).into_owned())
                        .collect();
                    println!("  array {}: {}", o.name_lossy(), cols.join(" | "));
                    for row in &a.rows {
                        println!("    {row:?}");
                    }
                }
            }
            "--params" => {
                println!("{path}");
                for (i, o) in doc.objects.iter().enumerate() {
                    let Some(chunk) = &o.chunk else { continue };
                    match o.class_id {
                        2 => {
                            if let Ok(p) = nmo::behavior::parameter_in(chunk) {
                                println!(
                                    "  {i:5} in    {:24} <- {:?}{}",
                                    o.name_lossy(),
                                    p.source,
                                    if p.shared { " (shared)" } else { "" }
                                );
                            }
                        }
                        3 | 45 => {
                            if let Ok(p) = nmo::behavior::parameter(chunk) {
                                let v = match &p.value {
                                    nmo::behavior::Saved::Bytes(b) => {
                                        let txt = String::from_utf8_lossy(b);
                                        let floats: Vec<f32> = b
                                            .as_chunks::<4>()
                                            .0
                                            .iter()
                                            .map(|c| f32::from_le_bytes(*c))
                                            .collect();
                                        format!("{b:02x?} {txt:?} {floats:?}")
                                    }
                                    other => format!("{other:?}"),
                                };
                                println!(
                                    "  {i:5} {} {:24} = {v} -> {:?}",
                                    if o.class_id == 3 { "out  " } else { "local" },
                                    o.name_lossy(),
                                    p.destinations
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            "--extras" => {
                println!("{path}");
                extras(&doc);
            }
            "--objects" => {
                println!("{path}");
                for (i, o) in doc.objects.iter().enumerate() {
                    println!("  {i:5} class {:3} {}", o.class_id, o.name_lossy());
                }
            }
            _ => {
                let h = &file.header;
                println!("{path}");
                println!(
                    "  ck version {:#010x}, file version {}, product build {:#010x}",
                    h.ck_version, h.file_version, h.product_build
                );
                println!(
                    "  objects {}, managers {}, max id {}",
                    h.object_count, h.manager_count, h.max_id_saved
                );
                let sum = if file.checksum_matches() {
                    "matches"
                } else {
                    "does not match"
                };
                println!("  checksum {:#010x} {sum}", h.checksum);
                println!(
                    "  header 1: {} bytes ({} packed)",
                    file.header1.len(),
                    h.header1_packed
                );
                println!(
                    "  data:     {} bytes ({} packed)",
                    file.data.len(),
                    h.data_packed
                );
                for inc in &file.included {
                    println!(
                        "  included: {} ({} bytes)",
                        String::from_utf8_lossy(&inc.name),
                        inc.data.len()
                    );
                }
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
