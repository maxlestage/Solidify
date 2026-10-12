# Introduction

Solidify reads a TypeScript or JavaScript program, the files it imports included, and writes the Rust you would have written by hand: structs, enums, `Option`, `Result`, iterators and ownership. The output is one plain Rust file, with no virtual machine and no garbage collector, and it prints what Node.js prints.

## What it does

Give it this TypeScript:

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

Every choice in that output comes from the program itself:

- `Point` is an interface, so it becomes a `struct`; the optional `label` becomes an `Option<String>`.
- `x` and `y` only ever hold whole numbers, so they are `i64`. A float literal, a division or JSON would have made them `f64`.
- `describe` only reads its argument, so it borrows it (`&Point`) instead of taking it.
- `p.label !== undefined` is a test of the `Option`: it becomes `if let Some(label)`.
- The statements at the top of the file become `main`.

## What it is for

- **Moving a program to Rust** without rewriting it from scratch: translate it, then read and keep the Rust.
- **Learning Rust from code you know:** write what you mean in TypeScript and see how Rust says it.
- **Shipping one binary** where you had a Node.js script: `rustc` compiles the output, no runtime to install.

> [!NOTE]
> Solidify translates programs, not libraries for the browser: there is no DOM, no `fetch` and no `http` server. The [limits](limits.md) list what stays different from Node.js, and what is refused with a message.

## How it is checked

The repository holds 54 test programs. Each one is translated, compiled with `rustc` and run; its output must be exactly what Node.js prints for the same program. Every example on these pages is checked the same way against the transpiler: what you read here is what Solidify writes.

## How to read these pages

- **Getting started:** [install it](installation.md), then follow the [quick start](quick-start.md) from a `.ts` file to a running binary.
- **The translation:** [how it works](how-it-works.md), then one page per subject: [types](types.md), [functions and ownership](ownership.md), [classes and interfaces](classes.md), [errors](errors.md), [async and timers](async.md), [modules and programs](modules.md), and [the JavaScript library](runtime.md) (`console.log`, strings, arrays, JSON, regular expressions, dates).
- **Reference:** the [command line](cli.md), the [Rust library](library.md), the full table of [what translates to what](mapping.md), the [limits](limits.md) and the [FAQ](faq.md).

Press `/` anywhere on the site to search these pages.
