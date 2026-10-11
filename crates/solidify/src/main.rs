//! `solidify input.ts [-o output.rs]`: writes Rust for a TypeScript or JavaScript file.
//!
//! Without a file, reads the standard input. `--check` only reports errors. `--no-header`
//! omits the generated-by comment.

use std::io::{Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut check = false;
    let mut no_header = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
                if output.is_none() {
                    eprintln!("error: `-o` needs a file name");
                    return ExitCode::from(2);
                }
            }
            "--check" => check = true,
            "--no-header" => no_header = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("solidify {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            flag if flag.starts_with('-') && flag != "-" => {
                eprintln!("error: unknown option `{flag}`\n{USAGE}");
                return ExitCode::from(2);
            }
            file => input = Some(file.to_string()),
        }
        i += 1;
    }
    let (source, name) = match input.as_deref() {
        Some("-") | None => {
            let mut s = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut s) {
                eprintln!("error: cannot read the standard input: {e}");
                return ExitCode::from(2);
            }
            (s, None)
        }
        Some(path) => match std::fs::read_to_string(path) {
            Ok(s) => (s, Some(path.to_string())),
            Err(e) => {
                eprintln!("error: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        },
    };
    let options = solidify::Options {
        file_name: name.as_deref().map(|n| {
            std::path::Path::new(n)
                .file_name()
                .map_or(n.to_string(), |f| f.to_string_lossy().into_owned())
        }),
        no_header,
    };
    match solidify::transpile(&source, &options) {
        Ok(rust) => {
            if check {
                return ExitCode::SUCCESS;
            }
            match output {
                Some(path) => {
                    if let Err(e) = std::fs::write(&path, rust) {
                        eprintln!("error: cannot write {path}: {e}");
                        return ExitCode::from(2);
                    }
                }
                None => {
                    let _ = std::io::stdout().write_all(rust.as_bytes());
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            let where_ = name.as_deref().unwrap_or("<stdin>");
            eprintln!(
                "{}",
                e.pretty(&source)
                    .replacen("  --> ", &format!("  --> {where_}:"), 1)
            );
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "usage: solidify [input.ts] [-o output.rs] [--check] [--no-header]

Translates one TypeScript or JavaScript file to Rust. Reads the standard input
when no file is given, writes to the standard output when -o is not given.";
