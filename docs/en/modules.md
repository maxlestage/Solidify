# Modules and programs

A program is a file and the files it imports. Solidify follows the imports from the file you give it and writes one Rust file: each imported file becomes a `mod`, and the top-level code becomes `main`.

## Top-level code

The statements at the top level of the program become `main`, imported files first, in the order Node.js runs them. Top-level declarations become items of the module:

- a `const` holding a literal is a Rust `const`;
- a top-level variable that functions read or change is a `static`, read and written in place;
- functions, classes and interfaces are functions, structs and traits.

```ts
const LIMIT = 3;
let calls = 0;

function track(): number {
  calls += 1;
  return calls;
}

for (let i = 0; i < LIMIT; i++) {
  track();
}
console.log(calls);
```
```rust fragment
use std::sync::LazyLock;

const LIMIT: i64 = 3;
static CALLS: LazyLock<JsStatic<i64>> = LazyLock::new(|| JsStatic::new(0));

fn track() -> i64 {
    (*CALLS.write()) += 1;
    (*CALLS.read())
}

fn main() {
    for _ in 0..LIMIT {
        track();
    }
    println!("{}", (*CALLS.read()));
}

// …
```

## Several files

`import` and `export` work as in TypeScript: named exports, `export default`, `import * as ns`, and `require` with `module.exports` for CommonJS files. Each file becomes a `mod`, its exports are `pub`, and a name declared by two files is renamed in one of them.

With `math.ts`:

```ts fragment
export function square(n: number): number {
  return n * n;
}

export const UNIT = "m²";
```

and `main.ts`:

```ts fragment
import { square, UNIT } from "./math";

console.log(`${square(12)} ${UNIT}`);
```

`npx solidify-rs main.ts` writes:

```rust
mod math {
    use super::*;

    pub const UNIT: &str = "m²";

    pub fn square(n: i64) -> i64 {
        n * n
    }
}

pub use math::*;

fn main() {
    println!("{} {UNIT}", square(12));
}
```

An import of a file that does not exist is an error, and so is a cycle of imports (`circular import between b.ts and a.ts`).

## Node's modules

Of Node's own modules, `fs`, `fs/promises`, `path`, `os` and `process` are shipped with Solidify, written in TypeScript and translated with your program. `node:fs` and `fs` are the same module.

```ts
import { readFileSync, writeFileSync } from "node:fs";

writeFileSync("note.txt", "hello\nworld\n");
const lines = readFileSync("note.txt", "utf8").trim().split("\n");
console.log(lines.length, process.argv.length > 0);
```
```rust fragment
// …
mod node_fs {
    use super::*;
// …
    pub fn read_file_sync(path: &str, encoding: Option<&str>) -> Result<String, String> {
        if !js_node_fs_exists(path) {
            return Err(format!("ENOENT: no such file or directory, open '{path}'"));
        }
        if js_node_fs_is_dir(path) {
            return Err("EISDIR: illegal operation on a directory, read".to_string());
        }
        Ok(js_node_fs_read(path))
    }
// …
pub use node_fs::*;

fn main() -> Result<(), String> {
    write_file_sync("note.txt", "hello\nworld\n");
    let lines = read_file_sync("note.txt", Some("utf8"))?.trim().to_string().split("\n").map(String::from).collect::<Vec<String>>();
    println!("{} {}", lines.len(), std::env::args().collect::<Vec<String>>().len() > 0);
    js_run_timers();
    Ok(())
}
// …
```

`readFileSync` throws when the file is missing, with Node's message, so the program's `main` returns a `Result`. `process.argv`, `process.cwd()` and `process.exit(code)` work as in Node.js.

## Packages

An import of a package (`import { chunk } from "lodash-es"`) is looked up in the `node_modules` folders above the file, as Node.js does, and the package is translated from its sources with the program: its `package.json` decides the entry file (`exports`, `module`, `main`).

A package that needs what Solidify does not translate is refused, with a message naming what: native add-ons, or Node's modules other than the five above (`http`, `child_process`, `net`…).
