# Rust library

The transpiler is a Rust crate, `solidify`, without dependency. The command line and the website's playground are built on the same API.

## Adding it

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

## Translating a source

`transpile` translates one TypeScript or JavaScript source, given as a string:

```rust
let typescript = "const n: number = 6 * 7;\nconsole.log(n);";
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

It returns the Rust as a `String`, or a `solidify::Error`. The source cannot import other files: use `transpile_file` for a program on disk.

## Translating a program

`transpile_file` reads the entry file at `path`, follows its imports (relative files, packages of `node_modules`, Node's modules) and translates the whole program to one Rust file:

```rust
use std::path::Path;

let rust: String = solidify::transpile_file(Path::new("main.ts"), &solidify::Options::default())?;
std::fs::write("app.rs", rust)?;
```

Its error is a `String`, already formatted with the file, the line and the column.

## Options

```rust
pub struct Options {
    /// The source file name, mentioned in the header of the output.
    pub file_name: Option<String>,
    /// Omit the header comment.
    pub no_header: bool,
}
```

`Options::default()` writes a header without a file name. To name the source, or to leave the header out:

```rust
let options = solidify::Options {
    file_name: Some("main.ts".to_string()),
    no_header: false,
};
```

## Errors

A `solidify::Error` has a `message` and a `span`, the byte offsets of the problem in the source. It implements `std::error::Error` and `Display` (the message alone), and formats itself with the source it came from:

- `error.position(source)`: the line and the column, from 1;
- `error.display(source)`: `line:column: message`;
- `error.pretty(source)`: the message, the place, the line and a mark under it, as the command line prints them.

```rust
let source = std::fs::read_to_string("app.ts")?;
match solidify::transpile(&source, &solidify::Options::default()) {
    Ok(rust) => println!("{rust}"),
    Err(error) => eprintln!("{}", error.pretty(&source)),
}
```

## Project

`Project` is what the functions above use: the files of a program, loaded and parsed. `Project::load(path)` reads an entry file and everything it imports (a `LoadError` names the file that failed), `Project::single(name, source)` takes one source, and `project.transpile(&options)` translates. `project.pretty(&error)` formats an error with the file it is in, for programs of several files.

```rust
use solidify::{Options, Project};

let project = Project::load(std::path::Path::new("main.ts"))?;
match project.transpile(&Options::default()) {
    Ok(rust) => std::fs::write("app.rs", rust)?,
    Err(error) => eprintln!("{}", project.pretty(&error)),
}
```

## In the browser

The crate compiles to WebAssembly as it is: the playground on the home page calls `transpile` in your browser, and the npm package is the command line compiled for WASI.
