//! The npm package (`npx solidify-rs`): every test program, translated by the transpiler
//! compiled to WebAssembly (WASI) and run by Node.js, must come out exactly as the native
//! build writes it (the `.rs` files of `tests/cases`).
//!
//! Needs `npm/solidify.wasm` (see README, "Publish on npm") and `node`; skipped without
//! them, so that `cargo test` runs anywhere. CI builds the package first.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_package_translates_like_the_native_build() {
    let launcher = root().join("npm/bin/solidify.js");
    if !root().join("npm/solidify.wasm").is_file()
        || Command::new("node").arg("--version").output().is_err()
    {
        eprintln!("skipped: npm/solidify.wasm or node is missing");
        return;
    }
    let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    let mut programs = Vec::new();
    for entry in std::fs::read_dir(&cases).expect("tests/cases").flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "ts") {
            programs.push((path.clone(), path.with_extension("rs")));
        } else if path.join("main.ts").is_file() {
            programs.push((path.join("main.ts"), path.join("main.rs")));
        }
    }
    programs.sort();
    let mut differ = Vec::new();
    for (source, expected) in &programs {
        let run = Command::new("node")
            .arg(&launcher)
            .arg(source)
            .arg("--no-header")
            .output()
            .expect("node runs");
        let expected = std::fs::read_to_string(expected).expect("expected Rust");
        if !run.status.success() || String::from_utf8_lossy(&run.stdout) != expected {
            differ.push(format!(
                "{}\n{}",
                source.display(),
                String::from_utf8_lossy(&run.stderr)
            ));
        }
    }
    assert!(
        differ.is_empty(),
        "{} of {} programs differ:\n{}",
        differ.len(),
        programs.len(),
        differ.join("\n")
    );
}
