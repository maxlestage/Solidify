# Types

Every value of the program gets a Rust type, from its annotation or from its use. This page goes through each kind of value; the [table](mapping.md) lists them all at once.

## Numbers

A `number` is an `i64` while it only holds whole numbers. A float literal (`2.0`), a division, `Math.sqrt`, `parseFloat`, `Number()` or JSON makes it an `f64`, and with it every number it flows into.

```ts
let count = 0;
for (let i = 1; i <= 10; i++) {
  count += i;
}
const average = count / 10;
console.log(count, average);
```
```rust fragment
fn main() {
    let mut count = 0;
    for i in 1..=10 {
        count += i;
    }
    let average = count as f64 / 10.0;
    println!("{count} {}", js_number_text(average));
}
// …
```

A number that may outgrow `i64` is an `f64` too, so that it overflows as JavaScript does. `bigint` and literals like `10n` become `JsBigInt`, an integer of any size.

`parseFloat` and `Number()` return an `f64`, since they can return `NaN`; `parseInt` returns an `Option<i64>`, `None` standing for `NaN`.

## Strings

A `string` is a `String`; a parameter that is only read is a `&str`. Template strings become `format!`, `+=` becomes `push_str`, and `length` counts characters as JavaScript does.

```ts
function shout(word: string): string {
  return word.toUpperCase() + "!";
}

let greeting = "hello";
greeting += ", world";
console.log(shout(greeting), greeting.length);
```
```rust
fn shout(word: &str) -> String {
    format!("{}!", word.to_uppercase())
}

fn main() {
    let mut greeting = "hello".to_string();
    greeting.push_str(", world");
    println!("{} {}", shout(&greeting), greeting.chars().count());
}
```

## Arrays and tuples

`T[]` is a `Vec<T>`, borrowed as `&[T]` by the functions that only read it. A tuple type `[A, B]` is a Rust tuple, and destructuring takes it apart.

```ts
function minMax(xs: number[]): [number, number] {
  let lo = xs[0];
  let hi = xs[0];
  for (const x of xs) {
    if (x < lo) lo = x;
    if (x > hi) hi = x;
  }
  return [lo, hi];
}

const [lo, hi] = minMax([4, 8, 1]);
console.log(lo, hi);
```
```rust
fn min_max(xs: &[i64]) -> (i64, i64) {
    let mut lo = xs[0];
    let mut hi = xs[0];
    for &x in xs {
        if x < lo {
            lo = x;
        }
        if x > hi {
            hi = x;
        }
    }
    (lo, hi)
}

fn main() {
    let (lo, hi) = min_max(&[4, 8, 1]);
    println!("{lo} {hi}");
}
```

## Objects and interfaces

An `interface`, a `type` of an object, and an object literal all become a `struct` with `Debug`, `Clone` and `PartialEq`. An object literal without a declared type gets a struct named after its variable (`const user = { … }` gives `struct User`). An optional field (`label?: string`) is an `Option`.

`A & B` is one struct with the fields of both; `keyof T` is a string; `T["k"]` is the field's type; `typeof x` is the variable's type. A `Record<K, V>` or an index signature (`{ [k: string]: V }`) is a map.

## null and undefined

`T | null`, `T | undefined` and optional fields are `Option<T>`. Solidify reads them as TypeScript's strict mode does: reading a value that may be missing is an error until it is tested.

- `if (x !== null)` and `if (x)` become `if let Some(x)`.
- `if (x === null) return` becomes `let Some(x) = x else { return }`.
- `x ?? fallback` becomes `unwrap_or`; `x?.field` becomes `map`; `x!` takes the value.

```ts
function initials(first: string, last?: string): string {
  if (last === undefined) {
    return first.charAt(0);
  }
  return first.charAt(0) + last.charAt(0);
}

console.log(initials("Ada"), initials("Grace", "Hopper"));
```
```rust
fn initials(first: &str, last: Option<&str>) -> String {
    let Some(last) = last else {
        return js_char_at(first, 0);
    };
    format!("{}{}", js_char_at(first, 0), js_char_at(last, 0))
}

fn main() {
    println!("{} {}", initials("Ada", None), initials("Grace", Some("Hopper")));
}

/// `s.charAt(i)` / `s[i]`: one character, or an empty string.
fn js_char_at(s: &str, i: i64) -> String {
    if i < 0 {
        return String::new();
    }
    s.chars().nth(i as usize).map(String::from).unwrap_or_default()
}
```

`console.log` prints `undefined` or `null` as Node.js does, from where the missing value comes.

## Unions

A union of string literals is an enum, printed with its strings:

```ts
type Direction = "up" | "down";

function flip(d: Direction): Direction {
  return d === "up" ? "down" : "up";
}

console.log(flip("up"));
```
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    Up,
    Down,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Direction::Up => "up",
            Direction::Down => "down",
        })
    }
}

fn flip(d: Direction) -> Direction {
    if d == Direction::Up { Direction::Down } else { Direction::Up }
}

fn main() {
    println!("{}", flip(Direction::Up));
}
```

A union of object types, told apart by a field (a discriminated union), is an enum with one variant per member, and `switch` on that field is a `match`:

```ts
type Event =
  | { kind: "click"; x: number; y: number }
  | { kind: "key"; key: string };

function describe(event: Event): string {
  switch (event.kind) {
    case "click":
      return `click at ${event.x},${event.y}`;
    case "key":
      return `key ${event.key}`;
  }
}

console.log(describe({ kind: "key", key: "Enter" }));
```
```rust
#[derive(Debug, Clone, PartialEq)]
enum Event {
    Click(Click),
    Key(Key),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Click {
    pub kind: String,
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub kind: String,
    pub key: String,
}

fn describe(event: &Event) -> String {
    match event {
        Event::Click(event) => {
            return format!("click at {},{}", event.x, event.y);
        }
        Event::Key(event) => {
            return format!("key {}", event.key);
        }
    }
}

fn main() {
    println!("{}", describe(&Event::Key(Key { kind: "key".to_string(), key: "Enter".to_string() })));
}
```

Other unions (`number | string`) become an enum too, and `typeof x === "string"`, `"kind" in x` or `instanceof` choose the variant.

## any and unknown

`any` and `unknown` hold a `JsValue`: a value known only at run time, with JavaScript's operators, comparisons and printing. Testing it with `typeof` reads it as JavaScript would.

```ts
function show(value: unknown): string {
  if (typeof value === "string") {
    return `text: ${value}`;
  }
  return `something else: ${value}`;
}

console.log(show("hi"));
console.log(show(42));
```
```rust fragment
fn show(value: &JsValue) -> String {
    if js_typeof(&value) == "string" {
        return format!("text: {value}");
    }
    format!("something else: {value}")
}

fn main() {
    println!("{}", show(&JsValue::Str("hi".to_string())));
    println!("{}", show(&JsValue::Int(42)));
}

// …
```

The rest of the program keeps its precise types: `JsValue` is only where the program asked for it.
