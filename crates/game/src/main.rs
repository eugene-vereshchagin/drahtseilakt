// Drahtseilakt: the game. Loads a level (its own, from the player's copy of the original, or any .nmo map), plays
// it in a window, records and replays runs, and can run without a window (replay checks, screenshots).

// A release build on Windows is a windowed program: no console window opens beside the game. Its messages then go to
// a log file (log_without_console); a debug build stays a console program.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use drahtseilakt::args;
use std::process::ExitCode;

/// Set in the game started again with its output going to the log file.
#[cfg(windows)]
const LOGGED: &str = "DRAHTSEILAKT_LOGGED";

/// A windowed program started from Explorer (or from a console without redirection) has no output at all: it starts
/// itself again with its output and errors going to `drahtseilakt.log` in the data folder, waits for that one and ends
/// with its exit code. With output to a file or a pipe (`2> log.txt`, `| more`) it prints there as usual. If the log
/// cannot be made the game plays on without one.
#[cfg(windows)]
fn log_without_console() {
    use std::os::windows::io::AsRawHandle;
    // null: the parent gave no handle (std's documented value for a missing standard handle)
    if !std::io::stderr().as_raw_handle().is_null() || std::env::var_os(LOGGED).is_some() {
        return;
    }
    let dir = gamedata::profile::data_dir();
    // the last start's log is kept beside (a crash's log survives the next start)
    let log = dir.join("drahtseilakt.log");
    let _ = std::fs::rename(&log, dir.join("drahtseilakt.old.log"));
    let Ok(out) = std::fs::create_dir_all(&dir).and_then(|()| std::fs::File::create(&log)) else {
        return;
    };
    let (Ok(err), Ok(exe)) = (out.try_clone(), std::env::current_exe()) else {
        return;
    };
    let status = std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .env(LOGGED, "1")
        .stdin(std::process::Stdio::null())
        .stdout(out)
        .stderr(err)
        .status();
    if let Ok(s) = status {
        std::process::exit(s.code().unwrap_or(1));
    }
}

fn main() -> ExitCode {
    #[cfg(windows)]
    log_without_console();
    // the game's own options only: no program built on the library adds any here
    let result = args::parse(std::env::args().skip(1), args::USAGE, &mut |_, _, _| {
        Ok(false)
    })
    .and_then(|o| {
        println!("{}", args::paths_line());
        drahtseilakt::run(o)
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("drahtseilakt: {e}");
            ExitCode::FAILURE
        }
    }
}
