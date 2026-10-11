# Solidify

**English** · [Français](README.fr.md)

**Write TypeScript or JavaScript, get Rust.** Solidify is a transpiler written in Rust, with no dependency, that reads one TypeScript (or JavaScript) file and writes the Rust you would have written by hand: structs, enums, `Option`, `Result`, iterators and ownership. No virtual machine and no garbage collector in the output, a plain Rust program.

```ts
interface Point { x: number; y: number; label?: string }

function describe(p: Point): string {
  if (p.label !== undefined) {
    return p.label;
  }
  return `(${p.x}, ${p.y})`;
}

const points: Point[] = [{ x: 1, y: 2 }, { x: 3, y: 4, label: "far" }];
for (const p of points) {
  console.log(describe(p));
}
```

becomes

```rust
#[derive(Debug, Clone, PartialEq)]
struct Point {
    pub x: i64,
    pub y: i64,
    pub label: Option<String>,
}

fn describe(p: &Point) -> String {
    if let Some(label) = &p.label {
        return label.clone();
    }
    format!("({}, {})", p.x, p.y)
}

fn main() {
    let points = vec![Point { x: 1, y: 2, label: None }, Point { x: 3, y: 4, label: Some("far".to_string()) }];
    for p in &points {
        println!("{}", describe(p));
    }
}
```

📚 **Website, with a playground running Solidify in the browser**: the site in [`site/`](site), itself written with [active](https://github.com/maxlestage/Active) and published at https://maxlestage.github.io/Solidify/. Run it with `cargo run -p solidify-site`.

## Install

```sh
cargo install --git https://github.com/maxlestage/Solidify solidify
solidify app.ts -o app.rs      # one file in, one file out
solidify < app.ts > app.rs     # or through the standard input
solidify --check app.ts        # only report errors
rustc --edition 2024 app.rs && ./app
```

As a library:

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

```rust
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

Errors come with a line, a column and the offending line:

```text
error: `x` may be null here (number | null): test it with `if (… !== null)`, or use `?.` or `!`
  --> app.ts:4:15
   |
  4|   return x.toFixed(2);
   |          ^^^^^^^^^
```

## How it works

Three passes, all deterministic:

1. **Parse.** A hand-written parser reads modern TypeScript: classes, interfaces, enums, arrow functions, template strings, destructuring, async/await. JavaScript is TypeScript without the annotations.
2. **Infer.** Types come from annotations and from use, by unification. Numbers stay integers (`i64`) until a float literal or a division says otherwise (`f64`). `T | null` becomes `Option<T>`. A function that throws, or calls one that does outside a `try`, returns `Result<T, String>`. Parameters whose type nothing constrains become type parameters.
3. **Generate.** Every value gets an ownership: parameters are borrowed (`&str`, `&[T]`, `&T`) unless the function changes or keeps them; a variable is moved at its last use and cloned before that. Loops become ranges and iterators, `switch` becomes `match`, `if (x !== null)` becomes `if let Some(x)`, `if (x === null) return` becomes `let Some(x) = x else { return }`.

## What translates to what

| TypeScript | Rust |
| --- | --- |
| `number` | `i64`, or `f64` when a float is involved |
| `string`, template strings | `String`, `&str` for parameters, `format!` |
| `T[]`, `[A, B]` | `Vec<T>`, `&[T]` for parameters, tuples |
| `T \| null`, `x?: T`, `??`, `?.` | `Option<T>`, `if let`, `unwrap_or`, `map` |
| `interface`, object literals | `struct` with `Debug`, `Clone`, `PartialEq` |
| `class`, `get`, `set`, `static` | `struct` + `impl`, `new()`, `&self` or `&mut self`, consts |
| `enum`, `"a" \| "b"` | `enum` with `Display`, `match` |
| `Map`, `Set`, `Record<K, V>` | `HashMap`, `HashSet` |
| `throw`, `try` / `catch` / `finally` | `Result<T, String>`, `?`, a closure for the `try` block |
| `map`, `filter`, `reduce`, `find`, `some`… | iterator chains |
| `for … of`, `for (;;)`, `while`, labels | `for x in &v`, `for i in a..b`, `loop`, `'label` |
| `switch` | `match`, with grouped cases |
| `async`, `await` | `async fn`, `.await`, a small executor for `main` |
| `function f<T>(x: T)` | `fn f<T: Clone + PartialEq>(x: T)`, bounds from use |
| `console.log`, `Math`, `parseInt`… | `println!`, `f64` methods, small helpers appended to the file |
| recursive classes | `Option<Box<T>>` |

Not yet: inheritance (`extends`), `any`, regular expressions, JSON, timers, imports between files, unions of different types, `typeof` and `instanceof` tests, generators. Each one is refused with a message that says what to write instead.

Four rules worth knowing:

- **Integers first.** `let n = 0` is an `i64`. Write `2.0`, or divide, and the whole family of that number becomes `f64`. `parseInt`, `parseFloat` and `Number()` return `f64`, because they can return `NaN`.
- **Null is `Option`.** Test nullable values with `!== null`, `!== undefined` or `??`; the generated Rust uses `if let`, `let … else` and `unwrap_or`. Reading a field of a value that may be null is an error, as in TypeScript's strict mode.
- **Throw is `Result`.** `throw new Error("…")` becomes `return Err("…".to_string())`; `catch (e)` receives the message as a `String`.
- **One file at a time.** Top-level statements become `main`; top-level `const`s holding literals become Rust `const`s; other top-level variables cannot be read from inside functions.

`console.log` of an object prints Rust's `Debug` form, `Map` iteration follows `HashMap` order, and `arr.sort()` without comparator sorts numbers as strings, as JavaScript does.

## Repository

| Folder | Contents |
| --- | --- |
| [`crates/solidify`](crates/solidify) | The transpiler: lexer, parser, type inference, code generation, CLI |
| [`crates/solidify/tests/cases`](crates/solidify/tests/cases) | Programs translated, compiled with `rustc` and compared with what Node.js prints |
| [`site`](site) | The website: a playground running Solidify as WebAssembly, written with [active](https://github.com/maxlestage/Active) |

## Development

```sh
cargo test --workspace                     # the transpiler and the site
cargo run -p solidify -- crates/solidify/tests/cases/fib.ts
cargo run -p solidify-site                 # the website on http://localhost:3000
```

The test suite translates every `tests/cases/*.ts`, compares the result with the `.rs` next to it, compiles it with `rustc` and runs it: the output must match the `.out` file, which holds what Node.js prints. `UPDATE_CASES=1 cargo test -p solidify --test cases` rewrites the expected files (the `.out` files when `node` is installed).

The website's browser bundle is built like this (the [`Dockerfile`](Dockerfile) and [`ci.yml`](.github/workflows/ci.yml) do the same):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p solidify-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/solidify_site.wasm
cargo run -p solidify-site                 # http://localhost:3000
cargo run -p solidify-site -- export dist  # static files for any host
```

## Publish the site on GitHub Pages

1. On GitHub, open **Settings → Pages** and set **Source** to **GitHub Actions** (once).
2. After every green CI run on `master`, the [`pages.yml`](.github/workflows/pages.yml) workflow exports the site with `solidify-site export` and publishes it at `https://maxlestage.github.io/Solidify/`. To publish right away: **Actions → Pages → Run workflow**.

## License

Solidify is proprietary software: © 2026 Maxime Nathan Lestage, all rights reserved. It is not open source. The source code is published on GitHub so that it can be read, audited and built, and you may use it freely as a dependency of your own programs, commercial ones included. Modifying it or redistributing its source needs the author's written permission. Full terms: [LICENSE](LICENSE).

## Credits

Designed and developed by [Maxime Nathan Lestage](https://github.com/maxlestage), as part of one family of Rust projects: [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) and [Solidify](https://maxlestage.github.io/Solidify/).
