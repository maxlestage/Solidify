//! Solidify: write TypeScript or JavaScript, get Rust.

pub mod ast;
pub mod codegen;
pub mod desugar;
pub mod error;
pub mod infer;
pub mod lexer;
pub mod parser;
pub mod project;
pub mod types;

pub use error::Error;
pub use project::{FileInfo, LoadError, Project};

/// Options of a translation.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The source file name, mentioned in the header of the output.
    pub file_name: Option<String>,
    /// Omit the header comment.
    pub no_header: bool,
}

/// Translates one TypeScript (or JavaScript) file to Rust. A file importing others is
/// translated with [`Project::load`], which follows the imports.
pub fn transpile(src: &str, options: &Options) -> Result<String, Error> {
    Project::single(options.file_name.as_deref(), src)?.transpile(options)
}

/// Translates the file at `path` and every file it imports to one Rust file.
pub fn transpile_file(path: &std::path::Path, options: &Options) -> Result<String, String> {
    let project = Project::load(path).map_err(|e| e.0)?;
    project.transpile(options).map_err(|e| project.pretty(&e))
}
