# Solidify

**English** · [Français](README.fr.md)

**Write TypeScript or JavaScript, get Rust.** Solidify is a transpiler written in Rust, with no dependency, that reads a TypeScript (or JavaScript) program, the files it imports included, and writes the Rust you would have written by hand: structs, enums, `Option`, `Result`, iterators and ownership. No virtual machine and no garbage collector in the output, a plain Rust program whose output matches what Node.js prints.

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
solidify main.ts -o app.rs     # follows the imports of main.ts, one Rust file out
solidify < app.ts > app.rs     # or one file through the standard input
solidify --check main.ts       # only report errors
rustc --edition 2024 app.rs && ./app
```

As a library:

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

```rust
let options = solidify::Options::default();
let rust: String = solidify::transpile(typescript, &options)?;
// A program of several files, from its entry file:
let rust: String = solidify::transpile_file(Path::new("main.ts"), &options)?;
```

Errors come with the file, a line, a column and the offending line:

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
2. **Infer.** Types come from annotations and from use, by unification. Numbers stay integers (`i64`) until a float literal, a division or JSON says otherwise (`f64`). `T | null` becomes `Option<T>`, `A | B` an enum. A function that throws, or calls one that does outside a `try`, returns `Result<T, String>`. Parameters whose type nothing constrains become type parameters; `any` is inferred from use.
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
| `Map`, `Set`, `Record<K, V>` | `HashMap`, `HashSet`, or `JsMap`, `JsSet` when the program observes the insertion order |
| `throw`, `try` / `catch` / `finally` | `Result<T, String>`, `?`, a closure for the `try` block |
| `map`, `filter`, `reduce`, `find`, `some`… | iterator chains |
| `for … of`, `for (;;)`, `while`, labels | `for x in &v`, `for i in a..b`, `loop`, `'label` |
| `switch` | `match`, with grouped cases |
| `async`, `await` | `async fn`, `.await`, a small executor for `main` |
| `function f<T>(x: T)`, `f<number>(x)` | `fn f<T: Clone + PartialEq>(x: T)`, bounds from use |
| `class Stack<T>`, `interface Page<T>`, `type Entry<K, V>`, `extends Stack<number>`, `<T extends HasId>` | `struct Stack<T>` + `impl<T: …>`, bounds found across the program, a `HasIdLike` trait for constraints |
| `interface Listener<T> { notify(v: T): void }`, `abstract class Shape<T>` | `trait ListenerLike<T>`, `Box<dyn ListenerLike<String>>` |
| methods returning `this`, `a.add(1).add(2)` | `&mut Self`: the calls chain on one object |
| `enum Level { … }`, `Level[Level.High]` | `enum` with `Display`, `Level::High.name()` |
| `console.log`, `Math`, `parseInt`… | `println!`, `f64` methods, small helpers appended to the file |
| recursive classes | `Option<Box<T>>` |
| `class B extends A`, `abstract`, `super`, `protected`, `instanceof` | flattened structs, a trait per base class, `Box<dyn ALike>` where an `A` is expected |
| `interface Shape { area(): number }`, `class C implements Shape`, `interface A extends B` | a trait per interface, `impl ShapeLike for C`, `Box<dyn ShapeLike>` / `&dyn ShapeLike` where a `Shape` is expected |
| `A \| B`, `[1, "two"]`, discriminated unions, `typeof x === "string"`, `"kind" in x` | `enum` with one variant per member, `match` |
| `function*`, `yield`, `yield*` | `JsGenerator<T>`, `for x in gen` |
| `setTimeout`, `setInterval`, `clearTimeout`, `new Promise`, `Promise.all` | an event loop run by `main`, `JsPromise` |
| `/regex/flags`, `new RegExp`, `test`, `exec`, `match`, `matchAll`, `replace`, `split`, `search`, `lastIndex`, `while ((m = re.exec(s)) !== null)` | `JsRegex`, a backtracking engine shipped in the generated file, `while let Some(m) = re.exec(&s)` |
| `JSON.stringify(v, null, 2)`, `JSON.parse` into an annotated type | the value's `JsValue`, a generated `JsFromJson` for the parsed type |
| `import`, `export`, `export default`, `import * as ns`, `export … from` | one `mod` per imported file, `pub` for exports |
| `console.log` of objects, arrays, maps, sets, nested values | the exact text Node.js prints (`util.inspect` rules) |
| `any`, `unknown` | inferred from use |
| `...rest` parameters, `f(...xs)`, `{ ...base, z }`, `[a, ...rest] = xs`, `({ x, y }: Point) =>` | slices, `vec!`, struct literals built field by field, `let rest = xs[1..].to_vec()` |
| `parseInt` | `Option<i64>` (`NaN` is `None`), narrowed by `isNaN` |
| `new Date(…)`, `getTime`, `getFullYear`…, `toISOString`, `later - earlier`, `Date.now()` | `JsDate` (milliseconds, UTC), `PartialOrd`, `get_time()` differences |
| `f?.()`, `obj?.method()`, fields of function type, `onClick?: () => void` | `if let Some(f) = …`, `.map(\|o\| o.method())`, `Rc<dyn Fn(…)>` fields |

What stays different, because Rust values are not JavaScript references: an object stored in two places is copied, and changing one copy leaves the other alone (variables changed inside closures that outlive their scope are shared through `Rc<RefCell<T>>`). Numbers are `i64` until a float is involved, so an integer that overflows panics where JavaScript would lose precision. Imports go between your own files only, not to packages. `JSON` replacers and revivers, `Symbol`, `Proxy`, `eval`, prototype manipulation and `arguments` have no Rust counterpart and are refused with a message that says what to write instead.

Four rules worth knowing:

- **Integers first.** `let n = 0` is an `i64`. Write `2.0`, divide, or read the number from JSON, and the whole family of that number becomes `f64`. `parseFloat` and `Number()` return `f64`, because they can return `NaN`; `parseInt` returns `Option<i64>`.
- **Null is `Option`.** Test nullable values with `!== null`, `!== undefined` or `??`; the generated Rust uses `if let`, `let … else` and `unwrap_or`. Reading a field of a value that may be null is an error, as in TypeScript's strict mode. `console.log` prints `undefined` or `null` as Node.js does, from where the missing value comes.
- **Throw is `Result`.** `throw new Error("…")` becomes `return Err("…".to_string())`; `catch (e)` receives the message as a `String`. `JSON.parse` throws the same `SyntaxError` messages as V8.
- **Files are modules.** Top-level statements become `main`, imported files first, as Node.js runs them; top-level `const`s holding literals become Rust `const`s; other top-level variables read inside functions become `static`s (`LazyLock`, with a `Mutex` when they change). Each imported file becomes a `mod`, its exports `pub`; the files share one namespace, so two files cannot declare the same top-level name.

`console.log` prints objects, arrays, maps and sets exactly as Node.js does, `Map` and `Set` keep their insertion order when the program observes it, and `arr.sort()` without comparator sorts numbers as strings, as JavaScript does.

## Repository

| Folder | Contents |
| --- | --- |
| [`crates/solidify`](crates/solidify) | The transpiler: lexer, parser, type inference, code generation, CLI |
| [`crates/solidify/tests/cases`](crates/solidify/tests/cases) | 33 programs translated, compiled with `rustc` and compared with what Node.js prints (a directory holds a program of several files) |
| [`site`](site) | The website: a playground running Solidify as WebAssembly, written with [active](https://github.com/maxlestage/Active) |

## Development

```sh
cargo test --workspace                     # the transpiler and the site
cargo run -p solidify -- crates/solidify/tests/cases/fib.ts
cargo run -p solidify-site                 # the website on http://localhost:3000
```

The test suite translates every `tests/cases/*.ts` (and every `tests/cases/*/main.ts` with the files it imports), compares the result with the `.rs` next to it, compiles it with `rustc` and runs it: the output must match the `.out` file, which holds what Node.js prints. `UPDATE_CASES=1 cargo test -p solidify --test cases` rewrites the expected files (the `.out` files when `node` is installed).

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

Every branch whose CI is green is merged into `master` by the [`ci.yml`](.github/workflows/ci.yml) workflow (branches named `wip/…` excepted). After every green CI run on `master`, the [`pages.yml`](.github/workflows/pages.yml) workflow switches GitHub Pages on if needed, exports the site with `solidify-site export` and publishes it at `https://maxlestage.github.io/Solidify/`. To publish right away: **Actions → Pages → Run workflow**.

## License

Solidify is proprietary software: © 2026 Maxime Nathan Lestage, all rights reserved. It is not open source. The source code is published on GitHub so that it can be read, audited and built, and you may use it freely as a dependency of your own programs, commercial ones included. Modifying it or redistributing its source needs the author's written permission. Full terms: [LICENSE](LICENSE).

## Credits

Designed and developed by [Maxime Nathan Lestage](https://github.com/maxlestage), as part of one family of Rust projects: [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) and [Solidify](https://maxlestage.github.io/Solidify/).
