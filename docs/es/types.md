# Tipos

Cada valor del programa recibe un tipo Rust, de su anotación o de su uso. Esta página recorre cada tipo de valor; la [tabla](mapping.md) los reúne todos a la vez.

## Números

Un `number` es un `i64` mientras solo contenga números enteros. Un literal decimal (`2.0`), una división, `Math.sqrt`, `parseFloat`, `Number()` o JSON lo convierten en un `f64`, y con él a cada número al que fluye.

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

Un número que puede superar `i64` también es un `f64`, para que desborde como lo hace JavaScript. `bigint` y los literales como `10n` se convierten en `JsBigInt`, un entero de cualquier tamaño.

`parseFloat` y `Number()` devuelven un `f64`, ya que pueden devolver `NaN`; `parseInt` devuelve un `Option<i64>`, donde `None` representa `NaN`.

## Cadenas

Un `string` es un `String`; un parámetro que solo se lee es un `&str`. Las plantillas de cadenas se convierten en `format!`, `+=` se convierte en `push_str`, y `length` cuenta los caracteres como lo hace JavaScript.

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

## Arrays y tuplas

`T[]` es un `Vec<T>`, prestado como `&[T]` a las funciones que solo lo leen. Un tipo tupla `[A, B]` es una tupla de Rust, y la desestructuración la descompone.

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

## Objetos e interfaces

Una `interface`, un `type` de objeto y un literal de objeto se convierten todos en un `struct` con `Debug`, `Clone` y `PartialEq`. Un literal de objeto sin tipo declarado recibe un struct con el nombre de su variable (`const user = { … }` da `struct User`). Un campo opcional (`label?: string`) es un `Option`.

`A & B` es un único struct con los campos de ambos; `keyof T` es una cadena; `T["k"]` es el tipo del campo; `typeof x` es el tipo de la variable. Un `Record<K, V>` o una firma de índice (`{ [k: string]: V }`) es un mapa.

## null y undefined

`T | null`, `T | undefined` y los campos opcionales son `Option<T>`. Solidify los lee como lo hace el modo estricto de TypeScript: leer un valor que puede faltar es un error hasta que se comprueba.

- `if (x !== null)` y `if (x)` se convierten en `if let Some(x)`.
- `if (x === null) return` se convierte en `let Some(x) = x else { return }`.
- `x ?? fallback` se convierte en `unwrap_or`; `x?.field` se convierte en `map`; `x!` toma el valor.

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

`console.log` imprime `undefined` o `null` como lo hace Node.js, según de dónde venga el valor que falta.

## Uniones

Una unión de literales de cadena es un enum, que se imprime con sus cadenas:

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

Una unión de tipos de objeto que se distinguen por un campo (una unión discriminada) es un enum con una variante por miembro, y un `switch` sobre ese campo es un `match`:

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

Las demás uniones (`number | string`) también se convierten en un enum, y `typeof x === "string"`, `"kind" in x` o `instanceof` eligen la variante.

## any y unknown

`any` y `unknown` contienen un `JsValue`: un valor que solo se conoce en tiempo de ejecución, con los operadores, las comparaciones y la forma de imprimir de JavaScript. Comprobarlo con `typeof` lo lee como lo haría JavaScript.

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

El resto del programa conserva sus tipos precisos: `JsValue` solo aparece donde el programa lo pidió.
