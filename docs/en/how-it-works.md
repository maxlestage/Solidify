# How it works

Solidify translates in three passes: it parses the program, infers a type for every value, then generates Rust. Each pass is deterministic: the same program always gives the same Rust, and no pass guesses.

## 1. Parse

A hand-written parser reads modern TypeScript: classes, interfaces, enums, type aliases, generics, arrow functions, template strings, destructuring, spread, optional chaining, `async`/`await`, generators, labels, regular expressions. JavaScript is TypeScript without the annotations, and is read by the same parser.

When the program imports other files, they are read too, in the order Node.js would run them: the imported files first. A syntax error stops here, with the file, the line and the column.

## 2. Infer

Annotations are a start, not the whole story: every value gets a type, from its annotation when there is one, and from its use everywhere else. The types are solved together, by unification, across the whole program.

- **Numbers stay integers** (`i64`) until a float literal, a division, `Math`, `parseFloat` or JSON says otherwise; then the number, and every number it flows into, is a `f64`.
- **`T | null`** and optional fields become `Option<T>`; **`A | B`** becomes an enum.
- **A function that throws**, or calls one that does outside a `try`, returns `Result<T, String>`.
- **A parameter nothing constrains** becomes a type parameter, with the bounds its uses need.
- **`any` and `unknown`** are values known at run time (`JsValue`).

Here `total` only adds whole numbers, while `half` divides:

```ts
function total(values: number[]): number {
  let sum = 0;
  for (const v of values) {
    sum += v;
  }
  return sum;
}

function half(n: number): number {
  return n / 2;
}

console.log(total([1, 2, 3]), half(5));
```
```rust fragment
fn total(values: &[i64]) -> i64 {
    let mut sum = 0;
    for &v in values {
        sum += v;
    }
    sum
}

fn half(n: i64) -> f64 {
    n as f64 / 2.0
}

fn main() {
    println!("{} {}", total(&[1, 2, 3]), js_number_text(half(5)));
}

// …
```

`js_number_text`, one of the helpers written after `main`, prints `2.5` the way JavaScript does.

## 3. Generate

The last pass writes Rust, deciding the ownership of every value as it goes:

- **Parameters are borrowed** (`&str`, `&[T]`, `&T`) unless the function changes them (`&mut`) or keeps them.
- **A variable is moved at its last use**, and cloned before that when it is still needed.
- **Loops** become ranges (`for i in 1..=10`) and iterators; `map`, `filter`, `reduce` become iterator chains.
- **`switch`** becomes `match`; **`if (x !== null)`** becomes `if let Some(x)`; **`if (x === null) return`** becomes `let Some(x) = x else { return }`.

```ts
function find(names: string[], prefix: string): string | null {
  for (const name of names) {
    if (name.startsWith(prefix)) {
      return name;
    }
  }
  return null;
}

const found = find(["Ada", "Grace", "Linus"], "Gr");
console.log(found ?? "nobody");
```
```rust
fn find(names: &[String], prefix: &str) -> Option<String> {
    for name in names {
        if name.starts_with(prefix) {
            return Some(name.clone());
        }
    }
    None
}

fn main() {
    let found = find(&["Ada".to_string(), "Grace".to_string(), "Linus".to_string()], "Gr");
    println!("{}", found.unwrap_or("nobody".to_string()));
}
```

## The output

- **One file**, for Rust's 2024 edition, that `rustc` compiles alone: no crate to add.
- **Names follow Rust's conventions:** `minMax` becomes `min_max`, a top-level `const line` becomes `const LINE`, a variable named after a Rust keyword is escaped (`r#match`).
- **Comments are kept**, where they were.
- **Helpers come last.** What Rust's standard library lacks (printing numbers as JavaScript does, `console.log` of objects, regular expressions, dates, promises and timers) is written after `main`, and only what the program uses.
- **A header** names the source file; `--no-header` leaves it out.

The [types](types.md) page goes through each kind of value, and [functions and ownership](ownership.md) through borrowing, moves and closures.
