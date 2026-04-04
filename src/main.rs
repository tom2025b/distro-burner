//! distro-burner (Rust wrapper)
//!
//! This binary is a thin launcher. All logic lives in `distro_burner.py`.
//! The wrapper locates the Python script, then replaces itself with:
//!
//!   python3 distro_burner.py <all original args>
//!
//! Script resolution order:
//!   1. Same directory as this binary  (installed / release builds)
//!   2. Cargo workspace root           (cargo run / dev builds)
//!
//! Why Rust as the entry point?
//!   - Single distributable binary users can put in PATH
//!   - Fails fast with a clear message if Python 3 or the script is missing
//!   - Lets you swap in the pure-Rust branch without changing the CLI surface

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let script = find_script().unwrap_or_else(|| {
        eprintln!(
            "distro-burner: cannot find distro_burner.py\n\
             Looked next to this binary and in the repo root.\n\
             Make sure distro_burner.py is co-located with the binary."
        );
        std::process::exit(1);
    });

    // Collect every argument after argv[0] and pass them through unchanged.
    let forwarded: Vec<String> = std::env::args().skip(1).collect();

    let status = Command::new("python3")
        .arg(&script)
        .args(&forwarded)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("distro-burner: failed to launch python3 — {e}");
            eprintln!("Install Python 3: https://python.org");
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}

/// Locate `distro_burner.py` using two strategies.
fn find_script() -> Option<PathBuf> {
    // Strategy 1: adjacent to the compiled binary (typical installed layout).
    if let Ok(exe) = std::env::current_exe() {
        let candidate = exe.parent()?.join("distro_burner.py");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // Strategy 2: repo root, baked in at compile time (cargo run / dev).
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("distro_burner.py");
    if dev.exists() {
        return Some(dev);
    }

    None
}
