//! Solidify: write TypeScript or JavaScript, get Rust.

pub mod ast;
pub mod codegen;
pub mod error;
pub mod infer;
pub mod lexer;
pub mod parser;
pub mod types;

pub use error::Error;

/// Options of a translation.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The source file name, mentioned in the header of the output.
    pub file_name: Option<String>,
    /// Omit the header comment.
    pub no_header: bool,
}

/// Translates a TypeScript (or JavaScript) module to Rust.
pub fn transpile(src: &str, options: &Options) -> Result<String, Error> {
    let module = parser::parse(src)?;
    let program = infer::infer(&module, src)?;
    let opts = codegen::Options {
        file_name: options.file_name.clone(),
        no_header: options.no_header,
    };
    codegen::generate(&module, &program, &opts)
}
