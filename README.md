# Solidify

**English** · [Français](README.fr.md) · [Español](README.es.md)

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

Nothing to install but Node.js 20 or later: `npx` runs Solidify, compiled to WebAssembly, on your files.

```sh
npx solidify-rs main.ts -o app.rs     # follows the imports of main.ts, one Rust file out
npx solidify-rs < app.ts > app.rs     # or one file through the standard input
npx solidify-rs --check main.ts       # only report errors
rustc --edition 2024 app.rs && ./app
```

The website is also an app: open it in Chrome, Edge or Safari and install it (the **Install the app** button, or *Add to Home Screen*); the playground then works offline.

As a Rust library:

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
2. **Infer.** Types come from annotations and from use, by unification. Numbers stay integers (`i64`) until a float literal, a division or JSON says otherwise (`f64`). `T | null` becomes `Option<T>`, `A | B` an enum. A function that throws, or calls one that does outside a `try`, returns `Result<T, String>`. Parameters whose type nothing constrains become type parameters; `any` and `unknown` are values known at run time (`JsValue`).
3. **Generate.** Every value gets an ownership: parameters are borrowed (`&str`, `&[T]`, `&T`) unless the function changes or keeps them; a variable is moved at its last use and cloned before that. Loops become ranges and iterators, `switch` becomes `match`, `if (x !== null)` becomes `if let Some(x)`, `if (x === null) return` becomes `let Some(x) = x else { return }`.

## What translates to what

| TypeScript | Rust |
| --- | --- |
| `` number `` | i64, or f64 when a float is involved or the value can outgrow i64 |
| `` bigint, 10n ** 30n `` | JsBigInt, integers of any size |
| `` string, template strings, tag`…` `` | String, &str for parameters, format!, a call of the tag |
| `` T[], [A, B] `` | Vec\<T>, &[T] for parameters, tuples |
| `` T \| null, x?: T, ??, ?. `` | Option\<T>, if let, unwrap_or, map |
| `` interface, object literal `` | struct with Debug, Clone, PartialEq |
| `` an object stored in two places, === `` | Shared\<T> (Rc\<RefCell\<T>>): one object, compared by identity |
| `` class, get, set, static, static { … } `` | struct, impl, new(), &self or &mut self, statics of the module |
| `` const A = class { … }, function F() { this.x = … }, F.prototype.m `` | classes, rebuilt as structs and impl blocks |
| `` enum, "a" \| "b" `` | enum with Display, match |
| `` Map, Set, Record<K, V>, { [k: string]: V } `` | HashMap, HashSet, or JsMap, JsSet in insertion order |
| `` A & B, keyof T, T["k"], typeof x, `id-${number}` `` | one struct with the fields of both, string, the field's type, the variable's type |
| `` throw, try / catch / finally `` | Result\<T, String>, ?, a closure for the try block |
| `` class NotFound extends Error, instanceof in catch `` | errors that keep their class, fields, message and name |
| `` map, filter, reduce, find, some, forEach… `` | iterator chains |
| `` for … of, for (;;), while, labels, for await `` | for x in &v, for i in a..b, loop, 'label, .await per item |
| `` switch, case without break `` | match with grouped cases, labeled blocks for the fallthrough |
| `` async, await, new Promise `` | fn … -> Promise\<T>: a task started at once, .await |
| `` then, catch, finally, Promise.all, allSettled, race, any `` | methods of Promise\<T>, microtasks in Node's order |
| `` setTimeout, setInterval `` | an event loop run by main, with Node's timings |
| `` function f<T>(x: T), f<number>(x) `` | fn f\<T: Clone + PartialEq>(x: T), bounds from use |
| `` class Stack<T>, interface Page<T>, <T extends HasId> `` | struct Stack\<T> + impl\<T: …>, bounds found across the program |
| `` class B extends A, abstract, super, instanceof `` | flattened structs, a trait per base class, Box\<dyn ALike> |
| `` interface Shape { area(): number }, implements `` | a trait per interface, impl for each class, Box\<dyn ShapeLike> |
| `` A \| B, discriminated unions, typeof, "kind" in x `` | enum with one variant per member, match |
| `` any, unknown `` | JsValue: a value known at run time, with JavaScript's operators |
| `` eval, new Function `` | the code inlined when it is known, a small interpreter otherwise |
| `` Proxy, Reflect, Symbol, arguments `` | JsProxy with its traps, JsSymbol, a rest parameter |
| `` function*, yield, yield* `` | JsGenerator\<T>, for x in gen |
| `` /regex/flags, lookbehind, test, exec, match, replace, split `` | JsRegex, a backtracking engine shipped in the file |
| `` JSON.stringify, JSON.parse, replacer, reviver `` | the value's JsValue, JsFromJson for the parsed type |
| `` import, export, import * as ns, require, module.exports `` | one mod per file, pub for exports |
| `` import fs from "node:fs", path, os, process, node_modules `` | modules shipped with Solidify, packages translated from their sources |
| `` console.log of objects, arrays, maps, sets `` | the exact text Node.js prints |
| `` ...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) => `` | slices, vec!, struct literals field by field, nested patterns taken apart |
| `` new Date(…), getFullYear, toISOString, later - earlier `` | JsDate in milliseconds (UTC), PartialOrd |
| `` f?.(), obj?.method(), fields of function type `` | if let Some(f) = …, .map(\|o\| o.method()), Rc\<dyn Fn(…)> fields |
| `` recursive classes `` | Option\<Box\<T>> |

**What stays different.** The generated program prints what Node.js prints; a few things cannot be the same. An error printed whole shows `Error: message`, without JavaScript's stack trace. Timers use the real clock: two timers a few milliseconds apart may swap under heavy load, as they can in Node. Code built at run time for `eval` or `new Function` runs in a small interpreter shipped in the file, slower than translated code. Of Node's own modules, `fs`, `fs/promises`, `path`, `os` and `process` are there; packages from `node_modules` are translated from their sources, so native add-ons and modules such as `http` or `child_process` are refused, like async generators (`async function*`), with a message that says why.

Four rules worth knowing:

- **Integers first.** `let n = 0` is an `i64`. Write `2.0`, divide, or read the number from JSON, and the whole family of that number becomes `f64`. `parseFloat` and `Number()` return `f64`, because they can return `NaN`; `parseInt` returns `Option<i64>`.
- **Null is `Option`.** Test nullable values with `!== null`, `!== undefined` or `??`; the generated Rust uses `if let`, `let … else` and `unwrap_or`. Reading a field of a value that may be null is an error, as in TypeScript's strict mode. `console.log` prints `undefined` or `null` as Node.js does, from where the missing value comes.
- **Throw is `Result`.** `throw new Error("…")` becomes `return Err("…".to_string())`; `catch (e)` receives the message as a `String`. `JSON.parse` throws the same `SyntaxError` messages as V8.
- **Files are modules.** Top-level statements become `main`, imported files first, as Node.js runs them; top-level `const`s holding literals become Rust `const`s; other top-level variables read inside functions become `static`s (`LazyLock`, read and written in place when they change). Each imported file becomes a `mod`, its exports `pub`; a name declared by two files is renamed in one of them.

`console.log` prints objects, arrays, maps and sets exactly as Node.js does, `Map` and `Set` keep their insertion order when the program observes it, and `arr.sort()` without comparator sorts numbers as strings, as JavaScript does.

## Repository

| Folder | Contents |
| --- | --- |
| [`crates/solidify`](crates/solidify) | The transpiler: lexer, parser, type inference, code generation, CLI |
| [`crates/solidify/tests/cases`](crates/solidify/tests/cases) | 54 programs translated, compiled with `rustc` and compared with what Node.js prints (a directory holds a program of several files) |
| [`npm`](npm) | The `solidify-rs` package for `npx`: a Node.js launcher for the transpiler compiled to WebAssembly (WASI), checked against the test programs by `cargo test -p solidify --test npm` |
| [`site`](site) | The website, written with [active](https://github.com/maxlestage/Active): a playground running Solidify as WebAssembly, three languages chosen automatically or by hand, an installable app that works offline, many animations. [`site/README.md`](site/README.md) documents all of it |

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
cargo build -p solidify-site-worker --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target no-modules --no-typescript --out-dir site/pkg/worker target/wasm32-unknown-unknown/wasm-release/solidify_site_worker.wasm
cargo run -p solidify-site                 # http://localhost:3000
cargo run -p solidify-site -- export dist  # static files for any host
```

## Publish the site on GitHub Pages

Every branch whose CI is green is merged into `master` by the [`ci.yml`](.github/workflows/ci.yml) workflow (branches named `wip/…` excepted). After every green CI run on `master`, the [`pages.yml`](.github/workflows/pages.yml) workflow switches GitHub Pages on if needed, exports the site with `solidify-site export` and publishes it at `https://maxlestage.github.io/Solidify/`. To publish right away: **Actions → Pages → Run workflow**.

## Publish on npm

The [`npm.yml`](.github/workflows/npm.yml) workflow builds the package after every merge into `master` and publishes it when its version (in [`npm/package.json`](npm/package.json)) is not yet on npm. It needs an npm token in the repository secret `NPM_TOKEN` (**Settings → Secrets and variables → Actions**). To try the package locally:

```sh
cargo build -p solidify --bin solidify --target wasm32-wasip1 --profile wasm-release
cp target/wasm32-wasip1/wasm-release/solidify.wasm LICENSE npm/
cargo test -p solidify --test npm # every test program, as the native build translates it
node npm/bin/solidify.js main.ts     # what `npx solidify-rs main.ts` runs
```

## License

Solidify is proprietary software: © 2026 Maxime Nathan Lestage, all rights reserved. It is not open source. The source code is published on GitHub so that it can be read, audited and built, and you may use it freely as a dependency of your own programs, commercial ones included. Modifying it or redistributing its source needs the author's written permission. Full terms: [LICENSE](LICENSE).

## Credits

Designed and developed by [Maxime Nathan Lestage](https://github.com/maxlestage), as part of one family of Rust projects: [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) and [Solidify](https://maxlestage.github.io/Solidify/).
