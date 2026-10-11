//! Every `tests/cases/<name>.ts` is translated, compared with `<name>.rs` (the expected
//! output), compiled with `rustc`, and run: its output must match `<name>.out`, which holds
//! what Node.js prints for the TypeScript file. A directory `tests/cases/<name>/` holds a
//! program of several files, entered at `main.ts`.
//!
//! `UPDATE_CASES=1 cargo test --test cases` rewrites the `.rs` files, and the `.out` files
//! when `node` is installed.

use std::path::{Path, PathBuf};
use std::process::Command;

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases")
}

fn cases() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(cases_dir())
        .expect("tests/cases")
        .flatten()
        .map(|e| e.path())
        .filter_map(|p| {
            if p.extension().is_some_and(|e| e == "ts") {
                Some(p)
            } else if p.is_dir() && p.join("main.ts").is_file() {
                Some(p.join("main.ts"))
            } else {
                None
            }
        })
        .collect();
    files.sort();
    files
}

fn case_name(path: &Path) -> String {
    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
    if stem == "main" {
        // A program of several files: named after its directory.
        return path
            .parent()
            .and_then(|d| d.file_name())
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or(stem);
    }
    stem
}

fn transpile(path: &Path) -> Result<String, String> {
    let options = solidify::Options {
        file_name: path.file_name().map(|f| f.to_string_lossy().into_owned()),
        no_header: true,
    };
    solidify::transpile_file(path, &options)
}

#[test]
fn every_case_translates_compiles_and_runs() {
    let update = std::env::var_os("UPDATE_CASES").is_some();
    let out_dir = std::env::temp_dir().join(format!("solidify-cases-{}", std::process::id()));
    std::fs::create_dir_all(&out_dir).expect("temp dir");
    let mut failures = Vec::new();
    for case in cases() {
        let name = case_name(&case);
        let expected_rs = case.with_extension("rs");
        let expected_out = case.with_extension("out");
        let rust = match transpile(&case) {
            Ok(r) => r,
            Err(e) => {
                failures.push(format!("{name}: translation failed:\n{e}"));
                continue;
            }
        };
        if update {
            std::fs::write(&expected_rs, &rust).expect("write .rs");
            if let Ok(node) = Command::new("node")
                .arg("--experimental-transform-types")
                .arg("--no-warnings")
                .arg(&case)
                .output()
                && node.status.success()
            {
                std::fs::write(&expected_out, &node.stdout).expect("write .out");
            }
        } else if let Ok(expected) = std::fs::read_to_string(&expected_rs)
            && expected != rust
        {
            failures.push(format!(
                "{name}: the Rust output changed (run with UPDATE_CASES=1 to accept):\n--- expected\n{expected}\n--- got\n{rust}"
            ));
            continue;
        }
        // Compile.
        let rs_path = out_dir.join(format!("{name}.rs"));
        std::fs::write(&rs_path, &rust).expect("write temp .rs");
        let is_bin = rust.contains("\nfn main(");
        let bin_path = out_dir.join(&name);
        let mut rustc = Command::new("rustc");
        rustc
            .arg("--edition")
            .arg("2024")
            .arg("--crate-type")
            .arg(if is_bin { "bin" } else { "lib" })
            .arg("-A")
            .arg("warnings")
            .arg("-o")
            .arg(if is_bin {
                bin_path.clone()
            } else {
                out_dir.join(format!("lib{name}.rlib"))
            })
            .arg(&rs_path);
        let compiled = rustc.output().expect("run rustc");
        if !compiled.status.success() {
            failures.push(format!(
                "{name}: rustc rejected the output:\n{}\n--- source\n{rust}",
                String::from_utf8_lossy(&compiled.stderr)
            ));
            continue;
        }
        if !is_bin {
            continue;
        }
        let run = Command::new(&bin_path).output().expect("run binary");
        let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
        if let Ok(expected) = std::fs::read_to_string(&expected_out)
            && expected != stdout
        {
            failures.push(format!(
                "{name}: the program printed something else than Node.js:\n--- expected\n{expected}\n--- got\n{stdout}\n--- stderr\n{}",
                String::from_utf8_lossy(&run.stderr)
            ));
        }
    }
    let _ = std::fs::remove_dir_all(&out_dir);
    if !failures.is_empty() {
        panic!(
            "{} case(s) failed:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}
