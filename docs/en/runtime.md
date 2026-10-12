# The JavaScript library

A program uses more than the language: `console.log`, the methods of strings and arrays, `Map`, `JSON`, regular expressions, dates. Each becomes Rust's own equivalent when there is one, and otherwise a helper written into the output, which behaves as Node.js does.

## console.log

`console.log` prints exactly what Node.js prints: numbers, strings, objects, arrays, maps and sets, with the same spacing and quotes. Arguments are separated by spaces; `console.error` prints on the standard error.

```ts
const user = { name: "Ada", langs: ["en", "fr"], admin: true };
console.log(user);
console.log([1, 2, 3].map((n) => n * 2));
console.log(new Map([["a", 1]]), new Set([1, 2]));
```
```rust fragment
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub name: String,
    pub langs: Vec<String>,
    pub admin: bool,
}

fn main() {
    let user = User { name: "Ada".to_string(), langs: vec!["en".to_string(), "fr".to_string()], admin: true };
    println!("{}", js_inspect(&user.js_value()));
    println!("{}", js_inspect(&(vec![1, 2, 3].into_iter().map(|n| n * 2).collect::<Vec<_>>()).js_value()));
    println!("{} {}", js_inspect(&(vec![("a".to_string(), 1)].into_iter().collect::<JsMap<_, _>>()).js_value()), js_inspect(&(vec![1, 2].into_iter().collect::<JsSet<_>>()).js_value()));
}
// …
```

It prints `{ name: 'Ada', langs: [ 'en', 'fr' ], admin: true }`, then `[ 2, 4, 6 ]`, then `Map(1) { 'a' => 1 } Set(2) { 1, 2 }`.

## Strings

The methods of strings map to Rust's (`toUpperCase` is `to_uppercase`, `trim` is `trim`, `split` is `split`, `includes` is `contains`, `startsWith` is `starts_with`…), and those Rust lacks are helpers: `charAt`, `padStart`, `slice` with negative indices, `localeCompare`. Indices and lengths count characters as JavaScript does.

## Arrays

`map`, `filter`, `reduce`, `find`, `some`, `every`, `forEach`, `flatMap`, `includes`, `indexOf`, `join` become iterator chains or `Vec` methods; `push`, `pop`, `splice`, `sort`, `reverse` change the vector in place. `sort()` without a comparator sorts numbers as strings, as JavaScript does. Spread (`[...a, ...b]`) builds a new vector.

## Map and Set

`Map` and `Set` become `JsMap` and `JsSet`: hash maps that keep their insertion order, like JavaScript's, when the program can observe it (by iterating or printing).

```ts
const counts = new Map<string, number>();
for (const word of "the cat and the hat".split(" ")) {
  counts.set(word, (counts.get(word) ?? 0) + 1);
}
for (const [word, n] of counts) {
  console.log(word, n);
}
```
```rust fragment
fn main() {
    let mut counts: JsMap<String, i64> = JsMap::new();
    for word in "the cat and the hat".split(" ").map(String::from).collect::<Vec<String>>() {
        counts.insert(word.clone(), counts.get(&word).copied().unwrap_or(0) + 1);
    }
    for (word, &n) in &counts {
        println!("{word} {n}");
    }
}
// …
```

`Record<K, V>` and objects used as dictionaries are maps too.

## JSON

`JSON.parse` reads text into the type the program expects (`JsFromJson`), and throws V8's `SyntaxError` messages when it cannot. `JSON.stringify` writes any value, with the replacer and the indentation when given.

```ts
interface Config {
  port: number;
  hosts: string[];
}

const config: Config = JSON.parse('{"port": 8080, "hosts": ["a", "b"]}');
console.log(config.port + 1, config.hosts.length);
console.log(JSON.stringify({ ok: true, port: config.port }));
```
```rust fragment
#[derive(Debug, Clone, PartialEq)]
struct Config {
    pub port: f64,
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub ok: bool,
    pub port: f64,
}

fn main() -> Result<(), String> {
    let config = js_json_parse_as::<Config>("{\"port\": 8080, \"hosts\": [\"a\", \"b\"]}")?;
    println!("{} {}", js_number_text(config.port + 1.0), config.hosts.len());
    println!("{}", js_json_stringify(&(Object { ok: true, port: config.port }).js_value(), ""));
    Ok(())
}
// …
```

A number read from JSON is an `f64`, as in JavaScript.

## Regular expressions

A regular expression literal becomes a `JsRegex`: JavaScript's syntax (groups, named groups, lookahead and lookbehind, flags `g`, `i`, `m`, `s`, `u`, `y`), matched by a backtracking engine written into the file. `test`, `exec`, `match`, `matchAll`, `replace`, `replaceAll`, `split` and `search` work on it.

```ts
const line = "2026-10-12 ERROR disk full";
const match = /^(\d{4})-(\d{2})-(\d{2}) (\w+)/.exec(line);
if (match !== null) {
  console.log(match[4], match[1]);
}
console.log("a1b22c333".replace(/\d+/g, "#"));
```
```rust fragment
const LINE: &str = "2026-10-12 ERROR disk full";

fn main() {
    let r#match = JsRegex::new("^(\\d{4})-(\\d{2})-(\\d{2}) (\\w+)", "").exec(LINE);
    if let Some(r#match) = &r#match {
        println!("{} {}", r#match.get(4), r#match.get(1));
    }
    println!("{}", JsRegex::new("\\d+", "g").replace("a1b22c333", "#"));
}
// …
```

## Dates

`new Date(…)` becomes a `JsDate`: milliseconds since 1970, in UTC. Its getters, `toISOString`, `getTime`, `Date.now()` and the subtraction of two dates work as in JavaScript.

```ts
const start = new Date("2026-01-31T00:00:00Z");
const later = new Date(start.getTime() + 24 * 60 * 60 * 1000);
console.log(later.toISOString(), later.getMonth());
```
```rust fragment
fn main() {
    let start = JsDate::parse("2026-01-31T00:00:00Z");
    let later = JsDate::from_ms(start.get_time() + 24 * 60 * 60 * 1000);
    println!("{} {}", later.to_iso_string(), later.get_month());
}
// …
```

> [!NOTE]
> Dates are read and shown in UTC. `Date.UTC` is not supported: write the date as an ISO string instead.

## Generators

A generator function (`function*`) returns a `JsGenerator<T>`, an iterator: `for … of` runs it, `yield*` delegates to another.

```ts
function* range(from: number, to: number) {
  for (let i = from; i < to; i++) {
    yield i;
  }
}

let sum = 0;
for (const n of range(1, 5)) {
  sum += n;
}
console.log(sum);
```
```rust fragment
fn range(from: i64, to: i64) -> JsGenerator<i64> {
    JsGenerator::new(move |y| {
        for i in from..to {
            y.yield_(i);
        }
    })
}

fn main() {
    let mut sum = 0;
    for n in range(1, 5) {
        sum += n;
    }
    println!("{sum}");
}
// …
```

## Math and numbers

`Math` is Rust's `f64` methods (`Math.sqrt(x)` is `x.sqrt()`, `Math.max` is `max`), `Math.random` a small generator written into the file. `toFixed`, `toString(radix)`, `Number.isInteger` and the other methods of numbers print as JavaScript does.

## Dynamic code

What only JavaScript can do still runs: `Proxy` and `Reflect` (`JsProxy` with its traps), `Symbol` (`JsSymbol`), `arguments` (a rest parameter). `eval` and `new Function` inline the code when it is known when translating, and otherwise run it in a small interpreter written into the file, slower than translated code.
