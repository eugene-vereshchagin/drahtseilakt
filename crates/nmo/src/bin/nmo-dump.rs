// nmo-dump: prints what a Virtools file holds. Default: the container; --objects: every object with its class;
// --check: decode every scene object and print only what fails (for runs over whole map collections).

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
        Some("--objects") | Some("--check") => args.remove(0),
        _ => String::new(),
    };
    if args.is_empty() {
        eprintln!("usage: nmo-dump [--objects | --check] FILE...");
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
