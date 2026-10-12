# Types

Chaque valeur du programme reçoit un type Rust, de son annotation ou de son usage. Cette page passe en revue chaque sorte de valeur ; le [tableau](mapping.md) les liste toutes d'un coup.

## Nombres

Un `number` est un `i64` tant qu'il ne contient que des nombres entiers. Un littéral flottant (`2.0`), une division, `Math.sqrt`, `parseFloat`, `Number()` ou du JSON en font un `f64`, et avec lui tout nombre qui en reçoit la valeur.

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

Un nombre qui peut dépasser `i64` est aussi un `f64`, pour qu'il déborde comme en JavaScript. `bigint` et les littéraux comme `10n` deviennent `JsBigInt`, un entier de toute taille.

`parseFloat` et `Number()` renvoient un `f64`, puisqu'ils peuvent renvoyer `NaN` ; `parseInt` renvoie un `Option<i64>`, où `None` tient lieu de `NaN`.

## Chaînes

Un `string` est une `String` ; un paramètre qui est seulement lu est un `&str`. Les gabarits de chaînes deviennent `format!`, `+=` devient `push_str`, et `length` compte les caractères comme le fait JavaScript.

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

## Tableaux et tuples

`T[]` est un `Vec<T>`, emprunté en `&[T]` par les fonctions qui ne font que le lire. Un type tuple `[A, B]` est un tuple Rust, et la déstructuration le décompose.

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

## Objets et interfaces

Une `interface`, un `type` d'objet et un littéral objet deviennent tous un `struct` avec `Debug`, `Clone` et `PartialEq`. Un littéral objet sans type déclaré reçoit un struct nommé d'après sa variable (`const user = { … }` donne `struct User`). Un champ optionnel (`label?: string`) est un `Option`.

`A & B` est un seul struct avec les champs des deux ; `keyof T` est une chaîne ; `T["k"]` est le type du champ ; `typeof x` est le type de la variable. Un `Record<K, V>` ou une signature d'index (`{ [k: string]: V }`) est une map.

## null et undefined

`T | null`, `T | undefined` et les champs optionnels sont des `Option<T>`. Solidify les lit comme le fait le mode strict de TypeScript : lire une valeur qui peut manquer est une erreur tant qu'elle n'a pas été testée.

- `if (x !== null)` et `if (x)` deviennent `if let Some(x)`.
- `if (x === null) return` devient `let Some(x) = x else { return }`.
- `x ?? fallback` devient `unwrap_or` ; `x?.field` devient `map` ; `x!` prend la valeur.

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

`console.log` affiche `undefined` ou `null` comme Node.js, selon d'où vient la valeur absente.

## Unions

Une union de littéraux de chaînes est un enum, affiché avec ses chaînes :

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

Une union de types objets, distingués par un champ (une union discriminée), est un enum avec une variante par membre, et un `switch` sur ce champ est un `match` :

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

Les autres unions (`number | string`) deviennent aussi un enum, et `typeof x === "string"`, `"kind" in x` ou `instanceof` choisissent la variante.

## any et unknown

`any` et `unknown` contiennent un `JsValue` : une valeur connue seulement à l'exécution, avec les opérateurs, les comparaisons et l'affichage de JavaScript. La tester avec `typeof` la lit comme le ferait JavaScript.

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

Le reste du programme garde ses types précis : `JsValue` n'apparaît que là où le programme l'a demandé.
