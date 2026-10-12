# Quick start

From a TypeScript file to a running Rust binary, in four commands. You need Node.js 20 or later and Rust ([installation](installation.md)).

## Write a program

Save this as `tasks.ts`. Node.js prints `1 left: Ship Rust`; so will the Rust.

```ts
interface Task {
  title: string;
  done: boolean;
}

const tasks: Task[] = [
  { title: "Write TypeScript", done: true },
  { title: "Ship Rust", done: false },
];

const left = tasks.filter((t) => !t.done).map((t) => t.title);
console.log(`${left.length} left: ${left.join(", ")}`);
```
```rust
#[derive(Debug, Clone, PartialEq)]
struct Task {
    pub title: String,
    pub done: bool,
}

fn main() {
    let tasks = vec![Task { title: "Write TypeScript".to_string(), done: true }, Task { title: "Ship Rust".to_string(), done: false }];
    let left = tasks.iter().filter(|t| !t.done).cloned().collect::<Vec<_>>().into_iter().map(|t| t.title.clone()).collect::<Vec<_>>();
    println!("{} left: {}", left.len(), left.join(", "));
}
```

The interface is a struct, the array a `Vec`, `filter` and `map` an iterator chain, the template string a `println!`.

## Translate it

```sh
npx solidify-rs tasks.ts -o tasks.rs
```

`tasks.rs` now holds the Rust above. Without `-o`, it goes to the standard output.

## Compile and run it

```sh
rustc --edition 2024 -O tasks.rs -o tasks
./tasks
```

It prints `1 left: Ship Rust`, like Node.js.

## A program of several files

Solidify follows the imports of the file you give it, and writes one Rust file with a module per imported file. With `math.ts`:

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

`npx solidify-rs main.ts -o app.rs` writes:

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

[Modules and programs](modules.md) tells how files, exports and top-level code are translated.

## When something does not translate

`--check` reports errors without writing anything. Solidify reads TypeScript strictly: reading a field of a value that may be `null` is an error, as in TypeScript's strict mode.

```ts error
function initial(name: string | null): string {
  return name.charAt(0);
}

console.log(initial("Ada"));
```
```text
error: `name` may be null here: test it with `if (… !== null)` first
  --> app.ts:2:10
   |
  2|   return name.charAt(0);
   |          ^^^^
```

Test the value first (`if (name !== null)`), or give a default with `??`, and the program translates. [Errors](errors.md) lists the messages and what to do about them.

## Next

- Try your own programs in the playground, on the home page: it translates as you type.
- Read [how it works](how-it-works.md), then the page of the subject you need.
