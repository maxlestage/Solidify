# Installation

There is nothing to install but Node.js: `npx` runs Solidify, compiled to WebAssembly, on your files. You can also use it in the browser, offline, or as a Rust library.

## With npx

You need Node.js 20 or later. Then, in the folder of your program:

```sh
npx solidify-rs main.ts -o app.rs
```

The first run downloads the `solidify-rs` package (about 2 MB, the transpiler itself); the next ones start at once. To keep it in a project instead:

```sh
npm install --save-dev solidify-rs
npx solidify-rs main.ts -o app.rs
```

The [command line](cli.md) page lists every option.

## Compiling the output

The output is one Rust file for the 2024 edition. Install Rust with [rustup](https://rustup.rs), then:

```sh
rustc --edition 2024 -O app.rs -o app
./app
```

It needs no crate: the few helpers a program uses (printing numbers the way JavaScript does, regular expressions, dates…) are written into the file itself.

## In the browser

The playground on the home page runs Solidify in your browser, compiled to WebAssembly: nothing leaves your machine. The site is also an app: open it in Chrome, Edge or Safari and choose **Install the app** (on an iPhone, *Share → Add to Home Screen*). It then works offline, playground included.

## As a Rust library

Add it to your `Cargo.toml`:

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

Then call it from Rust:

```rust
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

The [Rust library](library.md) page shows the whole API.

## From the sources

```sh
git clone https://github.com/maxlestage/Solidify
cd Solidify
cargo build --release -p solidify
./target/release/solidify main.ts -o app.rs
```

The `solidify` binary takes the same options as `npx solidify-rs`. Solidify has no dependency: building it only needs Rust.
