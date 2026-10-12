# Solidify

[English](README.md) · [Français](README.fr.md) · **Español**

**Escribe TypeScript o JavaScript, obtén Rust.** Solidify es un transpilador escrito en Rust, sin dependencias, que lee un programa TypeScript (o JavaScript), con los archivos que importa, y escribe el Rust que habrías escrito a mano: structs, enums, `Option`, `Result`, iteradores y propiedad. Ni máquina virtual ni recolector de basura en la salida: un programa Rust normal cuya salida es la de Node.js.

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

se convierte en

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

📚 **Sitio web, con un playground que ejecuta Solidify en el navegador**: el sitio está en [`site/`](site), escrito a su vez con [active](https://github.com/maxlestage/Active) y publicado en https://maxlestage.github.io/Solidify/. Arráncalo con `cargo run -p solidify-site`.

📖 **Documentación**: 16 páginas, de la [introducción](docs/es/introduction.md) y el [inicio rápido](docs/es/quick-start.md) a la [línea de comandos](docs/es/cli.md), la [biblioteca Rust](docs/es/library.md) y la tabla completa de [qué se traduce en qué](docs/es/mapping.md). En [`docs/`](docs) en español, inglés y francés, y en el sitio en https://maxlestage.github.io/Solidify/es/docs/ con un buscador. Cada ejemplo se comprueba con el transpilador.

## Instalar

Nada que instalar salvo Node.js 20 o posterior: `npx` ejecuta Solidify, compilado a WebAssembly, sobre tus archivos.

```sh
npx solidify-rs main.ts -o app.rs     # sigue los imports de main.ts, un solo archivo Rust de salida
npx solidify-rs < app.ts > app.rs     # o un archivo por la entrada estándar
npx solidify-rs --check main.ts       # solo informa de los errores
rustc --edition 2024 app.rs && ./app
```

El sitio también es una aplicación: ábrelo en Chrome, Edge o Safari e instálalo (el botón **Instalar la app**, o *Añadir a pantalla de inicio*); el playground funciona entonces sin conexión.

Como biblioteca de Rust:

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

```rust
let options = solidify::Options::default();
let rust: String = solidify::transpile(typescript, &options)?;
// Un programa de varios archivos, desde su archivo de entrada:
let rust: String = solidify::transpile_file(Path::new("main.ts"), &options)?;
```

Los errores llegan con el archivo, una línea, una columna y la línea culpable:

```text
error: `x` may be null here (number | null): test it with `if (… !== null)`, or use `?.` or `!`
  --> app.ts:4:15
   |
  4|   return x.toFixed(2);
   |          ^^^^^^^^^
```

## Cómo funciona

Tres pasadas, todas deterministas:

1. **Analizar.** Un parser escrito a mano lee el TypeScript moderno: clases, interfaces, enums, funciones flecha, plantillas de texto, desestructuración, async/await. JavaScript es TypeScript sin anotaciones.
2. **Inferir.** Los tipos vienen de las anotaciones y del uso, por unificación. Los números siguen siendo enteros (`i64`) mientras un literal decimal, una división o JSON no digan lo contrario (`f64`). `T | null` se convierte en `Option<T>`, `A | B` en un enum. Una función que lanza un error, o que llama a otra que lo hace fuera de un `try`, devuelve `Result<T, String>`. Los parámetros cuyo tipo nada restringe se vuelven parámetros de tipo; `any` y `unknown` son valores conocidos en ejecución (`JsValue`).
3. **Generar.** Cada valor recibe una propiedad: los parámetros se toman prestados (`&str`, `&[T]`, `&T`) salvo si la función los modifica o los guarda; una variable se mueve en su último uso y se clona antes. Los bucles se convierten en rangos e iteradores, `switch` en `match`, `if (x !== null)` en `if let Some(x)`, `if (x === null) return` en `let Some(x) = x else { return }`.

## Qué se traduce, y en qué

| TypeScript | Rust |
| --- | --- |
| `` number `` | i64, o f64 en cuanto interviene un decimal o el valor puede superar i64 |
| `` bigint, 10n ** 30n `` | JsBigInt, enteros de cualquier tamaño |
| `` string, template strings, tag`…` `` | String, &str como parámetro, format!, una llamada a la etiqueta |
| `` T[], [A, B] `` | Vec\<T>, &[T] como parámetro, tuplas |
| `` T \| null, x?: T, ??, ?. `` | Option\<T>, if let, unwrap_or, map |
| `` interface, object literal `` | struct con Debug, Clone, PartialEq |
| `` an object stored in two places, === `` | Shared\<T> (Rc\<RefCell\<T>>): un solo objeto, comparado por identidad |
| `` class, get, set, static, static { … } `` | struct, impl, new(), &self o &mut self, statics del módulo |
| `` const A = class { … }, function F() { this.x = … }, F.prototype.m `` | clases, reconstruidas como structs y bloques impl |
| `` enum, "a" \| "b" `` | enum con Display, match |
| `` Map, Set, Record<K, V>, { [k: string]: V } `` | HashMap, HashSet, o JsMap, JsSet en orden de inserción |
| `` A & B, keyof T, T["k"], typeof x, `id-${number}` `` | un struct con los campos de ambos, string, el tipo del campo, el de la variable |
| `` throw, try / catch / finally `` | Result\<T, String>, ?, un closure para el bloque try |
| `` class NotFound extends Error, instanceof in catch `` | errores que conservan su clase, campos, mensaje y nombre |
| `` map, filter, reduce, find, some, forEach… `` | cadenas de iteradores |
| `` for … of, for (;;), while, labels, for await `` | for x in &v, for i in a..b, loop, 'label, .await por elemento |
| `` switch, case without break `` | match con casos agrupados, bloques etiquetados para los casos encadenados |
| `` async, await, new Promise `` | fn … -> Promise\<T>: una tarea lanzada al instante, .await |
| `` then, catch, finally, Promise.all, allSettled, race, any `` | métodos de Promise\<T>, microtareas en el orden de Node |
| `` setTimeout, setInterval `` | un bucle de eventos lanzado por main, con los tiempos de Node |
| `` function f<T>(x: T), f<number>(x) `` | fn f\<T: Clone + PartialEq>(x: T), límites deducidos del uso |
| `` class Stack<T>, interface Page<T>, <T extends HasId> `` | struct Stack\<T> + impl\<T: …>, límites hallados en todo el programa |
| `` class B extends A, abstract, super, instanceof `` | structs aplanados, un trait por clase base, Box\<dyn ALike> |
| `` interface Shape { area(): number }, implements `` | un trait por interfaz, impl para cada clase, Box\<dyn ShapeLike> |
| `` A \| B, discriminated unions, typeof, "kind" in x `` | enum con una variante por miembro, match |
| `` any, unknown `` | JsValue: un valor conocido en ejecución, con los operadores de JavaScript |
| `` eval, new Function `` | el código integrado cuando se conoce, un pequeño intérprete si no |
| `` Proxy, Reflect, Symbol, arguments `` | JsProxy con sus trampas, JsSymbol, un parámetro rest |
| `` function*, yield, yield* `` | JsGenerator\<T>, for x in gen |
| `` /regex/flags, lookbehind, test, exec, match, replace, split `` | JsRegex, un motor con retroceso incluido en el archivo |
| `` JSON.stringify, JSON.parse, replacer, reviver `` | el JsValue del valor, JsFromJson para el tipo leído |
| `` import, export, import * as ns, require, module.exports `` | un mod por archivo, pub para los exports |
| `` import fs from "node:fs", path, os, process, node_modules `` | módulos incluidos en Solidify, paquetes traducidos desde sus fuentes |
| `` console.log of objects, arrays, maps, sets `` | el texto exacto que imprime Node.js |
| `` ...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) => `` | slices, vec!, literales de struct campo a campo, patrones anidados descompuestos |
| `` new Date(…), getFullYear, toISOString, later - earlier `` | JsDate en milisegundos (UTC), PartialOrd |
| `` f?.(), obj?.method(), fields of function type `` | if let Some(f) = …, .map(\|o\| o.method()), campos Rc\<dyn Fn(…)> |
| `` recursive classes `` | Option\<Box\<T>> |

**Qué sigue siendo distinto.** El programa generado imprime lo que imprime Node.js; algunas cosas no pueden ser idénticas. Un error impreso entero muestra `Error: message`, sin la pila de llamadas de JavaScript. Los temporizadores siguen el reloj real: dos temporizadores separados por pocos milisegundos pueden intercambiarse con mucha carga, como en Node. El código construido en ejecución para `eval` o `new Function` corre en un pequeño intérprete incluido en el archivo, más lento que el código traducido. De los módulos propios de Node están `fs`, `fs/promises`, `path`, `os` y `process`; los paquetes de `node_modules` se traducen desde sus fuentes, así que los módulos nativos y módulos como `http` o `child_process` se rechazan, como los generadores asíncronos (`async function*`), con un mensaje que explica por qué.

Cuatro reglas que conviene conocer:

- **Primero los enteros.** `let n = 0` es un `i64`. Escribe `2.0`, divide o lee el número desde JSON, y toda la familia de ese número pasa a `f64`. `parseFloat` y `Number()` devuelven `f64`, porque pueden devolver `NaN`; `parseInt` devuelve `Option<i64>`.
- **Null es `Option`.** Comprueba los valores que pueden faltar con `!== null`, `!== undefined` o `??`; el Rust generado usa `if let`, `let … else` y `unwrap_or`. Leer un campo de un valor que puede ser nulo es un error, como en el modo estricto de TypeScript. `console.log` imprime `undefined` o `null` como Node.js, según de dónde venga el valor que falta.
- **Throw es `Result`.** `throw new Error("…")` se convierte en `return Err("…".to_string())`; `catch (e)` recibe el error, que se escribe `Error: message` como en JavaScript. `JSON.parse` lanza los mismos mensajes de `SyntaxError` que V8.
- **Los archivos son módulos.** Las instrucciones de primer nivel se convierten en `main`, primero los archivos importados, como los ejecuta Node.js; las `const` de primer nivel que guardan un literal se vuelven `const` de Rust; las demás variables de primer nivel leídas desde una función se vuelven `static` (`LazyLock`, leídas y escritas en su sitio cuando cambian). Cada archivo importado se vuelve un `mod`, sus exports `pub`; un nombre declarado por dos archivos se renombra en uno de ellos.

`console.log` imprime objetos, arrays, maps y sets exactamente como Node.js, `Map` y `Set` conservan su orden de inserción cuando el programa lo observa, y `arr.sort()` sin comparador ordena los números como cadenas, como JavaScript.

## Repositorio

| Carpeta | Contenido |
| --- | --- |
| [`crates/solidify`](crates/solidify) | El transpilador: lexer, parser, inferencia de tipos, generación de código, línea de comandos |
| [`crates/solidify/tests/cases`](crates/solidify/tests/cases) | 54 programas traducidos, compilados con `rustc` y comparados con lo que imprime Node.js (una carpeta contiene un programa de varios archivos) |
| [`npm`](npm) | El paquete `solidify-rs` para `npx`: un lanzador Node.js del transpilador compilado a WebAssembly (WASI), comprobado contra los programas de prueba con `cargo test -p solidify --test npm` |
| [`docs`](docs) | La documentación, en inglés, francés y español (`docs/{en,fr,es}/*.md`): el sitio la muestra, y un test comprueba cada ejemplo con el transpilador |
| [`site`](site) | El sitio web, escrito con [active](https://github.com/maxlestage/Active): un playground que ejecuta Solidify en WebAssembly, la documentación con un menú y un buscador, tres idiomas elegidos automáticamente o a mano, una app instalable que funciona sin conexión, muchas animaciones. [`site/README.md`](site/README.md) lo documenta todo |

## Desarrollo

```sh
cargo test --workspace                     # el transpilador y el sitio
cargo run -p solidify -- crates/solidify/tests/cases/fib.ts
cargo run -p solidify-site                 # el sitio en http://localhost:3000
```

La batería de pruebas traduce cada `tests/cases/*.ts` (y cada `tests/cases/*/main.ts` con los archivos que importa), compara el resultado con el `.rs` de al lado, lo compila con `rustc` y lo ejecuta: la salida debe coincidir con el archivo `.out`, que contiene lo que imprime Node.js. `UPDATE_CASES=1 cargo test -p solidify --test cases` reescribe los archivos esperados (los `.out` cuando `node` está instalado).

El paquete del navegador del sitio se construye así (el [`Dockerfile`](Dockerfile) y [`ci.yml`](.github/workflows/ci.yml) hacen lo mismo):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p solidify-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/solidify_site.wasm
cargo build -p solidify-site-worker --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target no-modules --no-typescript --out-dir site/pkg/worker target/wasm32-unknown-unknown/wasm-release/solidify_site_worker.wasm
cargo run -p solidify-site                 # http://localhost:3000
cargo run -p solidify-site -- export dist  # archivos estáticos para cualquier alojamiento
```

## Publicar el sitio en GitHub Pages

Cada rama cuya CI está en verde se fusiona en `master` mediante el workflow [`ci.yml`](.github/workflows/ci.yml) (salvo las ramas llamadas `wip/…`). Tras cada ejecución en verde de la CI en `master`, el workflow [`pages.yml`](.github/workflows/pages.yml) activa GitHub Pages si hace falta, exporta el sitio con `solidify-site export` y lo publica en `https://maxlestage.github.io/Solidify/`. Para publicar al momento: **Actions → Pages → Run workflow**.

## Publicar en npm

El workflow [`npm.yml`](.github/workflows/npm.yml) construye el paquete tras cada fusión en `master` y lo publica cuando su versión (en [`npm/package.json`](npm/package.json)) aún no está en npm. Necesita un token de npm en el secreto del repositorio `NPM_TOKEN` (**Settings → Secrets and variables → Actions**). Para probar el paquete en local:

```sh
cargo build -p solidify --bin solidify --target wasm32-wasip1 --profile wasm-release
cp target/wasm32-wasip1/wasm-release/solidify.wasm LICENSE npm/
cargo test -p solidify --test npm # cada programa de prueba, como lo traduce la versión nativa
node npm/bin/solidify.js main.ts     # lo que ejecuta `npx solidify-rs main.ts`
```

## Licencia

Solidify es software propietario: © 2026 Maxime Nathan Lestage, todos los derechos reservados. No es código abierto. El código fuente se publica en GitHub para poder leerlo, auditarlo y compilarlo, y puedes usarlo libremente como dependencia de tus propios programas, incluidos los comerciales. Modificarlo o redistribuir su código fuente requiere el permiso por escrito del autor. Texto completo: [LICENSE](LICENSE).

## Créditos

Diseñado y desarrollado por [Maxime Nathan Lestage](https://github.com/maxlestage), dentro de una misma familia de proyectos Rust: [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) y [Solidify](https://maxlestage.github.io/Solidify/).
