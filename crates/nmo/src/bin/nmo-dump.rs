// nmo-dump: prints what a Virtools file holds. For now the container: header fields and part sizes.

use std::process::ExitCode;

fn main() -> ExitCode {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: nmo-dump FILE...");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for path in &paths {
        match std::fs::read(path) {
            Err(e) => {
                eprintln!("{path}: {e}");
                failed = true;
            }
            Ok(bytes) => match nmo::read(&bytes) {
                Err(e) => {
                    eprintln!("{path}: {e}");
                    failed = true;
                }
                Ok(file) => {
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
            },
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
