# La biblioteca JavaScript

Un programa usa más que el lenguaje: `console.log`, los métodos de las cadenas y de los arrays, `Map`, `JSON`, las expresiones regulares, las fechas. Cada uno se convierte en el equivalente propio de Rust cuando existe, y si no en una función auxiliar escrita en la salida, que se comporta como Node.js.

## console.log

`console.log` imprime exactamente lo que imprime Node.js: números, cadenas, objetos, arrays, maps y sets, con los mismos espacios y las mismas comillas. Los argumentos se separan con espacios; `console.error` imprime en la salida de error estándar.

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

Imprime `{ name: 'Ada', langs: [ 'en', 'fr' ], admin: true }`, luego `[ 2, 4, 6 ]`, luego `Map(1) { 'a' => 1 } Set(2) { 1, 2 }`.

## Cadenas

Los métodos de las cadenas corresponden a los de Rust (`toUpperCase` es `to_uppercase`, `trim` es `trim`, `split` es `split`, `includes` es `contains`, `startsWith` es `starts_with`…), y los que le faltan a Rust son funciones auxiliares: `charAt`, `padStart`, `slice` con índices negativos, `localeCompare`. Los índices y las longitudes cuentan caracteres como lo hace JavaScript.

## Arrays

`map`, `filter`, `reduce`, `find`, `some`, `every`, `forEach`, `flatMap`, `includes`, `indexOf`, `join` se convierten en cadenas de iteradores o en métodos de `Vec`; `push`, `pop`, `splice`, `sort`, `reverse` modifican el vector en su sitio. `sort()` sin comparador ordena los números como cadenas, como hace JavaScript. El spread (`[...a, ...b]`) construye un vector nuevo.

## Map y Set

`Map` y `Set` se convierten en `JsMap` y `JsSet`: tablas hash que conservan su orden de inserción, como las de JavaScript, cuando el programa puede observarlo (al recorrerlas o imprimirlas).

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

`Record<K, V>` y los objetos usados como diccionarios también son mapas.

## JSON

`JSON.parse` lee el texto en el tipo que espera el programa (`JsFromJson`), y lanza los mensajes `SyntaxError` de V8 cuando no puede. `JSON.stringify` escribe cualquier valor, con el replacer y la indentación cuando se indican.

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

Un número leído de JSON es un `f64`, como en JavaScript.

## Expresiones regulares

Un literal de expresión regular se convierte en un `JsRegex`: la sintaxis de JavaScript (grupos, grupos con nombre, lookahead y lookbehind, flags `g`, `i`, `m`, `s`, `u`, `y`), evaluada por un motor con retroceso escrito en el archivo. `test`, `exec`, `match`, `matchAll`, `replace`, `replaceAll`, `split` y `search` funcionan con él.

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

## Fechas

`new Date(…)` se convierte en un `JsDate`: milisegundos desde 1970, en UTC. Sus getters, `toISOString`, `getTime`, `Date.now()` y la resta de dos fechas funcionan como en JavaScript.

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
> Las fechas se leen y se muestran en UTC. `Date.UTC` no se admite: escribe la fecha como una cadena ISO.

## Generadores

Una función generadora (`function*`) devuelve un `JsGenerator<T>`, un iterador: `for … of` lo ejecuta, `yield*` delega en otro.

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

## Math y números

`Math` son los métodos de `f64` de Rust (`Math.sqrt(x)` es `x.sqrt()`, `Math.max` es `max`), y `Math.random` un pequeño generador escrito en el archivo. `toFixed`, `toString(radix)`, `Number.isInteger` y los demás métodos de los números imprimen como lo hace JavaScript.

## Código dinámico

Lo que solo JavaScript puede hacer sigue funcionando: `Proxy` y `Reflect` (`JsProxy` con sus trampas), `Symbol` (`JsSymbol`), `arguments` (un parámetro rest). `eval` y `new Function` integran el código cuando se conoce en el momento de traducir, y si no lo ejecutan en un pequeño intérprete escrito en el archivo, más lento que el código traducido.
