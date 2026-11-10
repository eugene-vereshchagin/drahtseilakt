// nmo-dump: prints what a Virtools file holds. Default: the container; --objects: every object with its class;
// --check: decode every scene object and print only what fails (for runs over whole map collections);
// --arrays: every data array with its rows; --params: every parameter with its value or source;
// --scripts: the behaviour graphs' building blocks by prototype, with counts (the survey of community maps).

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

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args.first().map(String::as_str) {
        Some("--objects") | Some("--check") | Some("--scripts") | Some("--arrays")
        | Some("--params") => args.remove(0),
        _ => String::new(),
    };
    if args.is_empty() {
        eprintln!(
            "usage: nmo-dump [--objects | --check | --scripts | --arrays | --params] FILE..."
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
                                            .chunks_exact(4)
                                            .filter_map(|c| {
                                                c.try_into().ok().map(f32::from_le_bytes)
                                            })
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
