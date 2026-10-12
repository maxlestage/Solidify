# Módulos y programas

Un programa es un archivo y los archivos que importa. Solidify sigue los imports desde el archivo que le das y escribe un único archivo Rust: cada archivo importado se convierte en un `mod`, y el código de nivel superior se convierte en `main`.

## Código de nivel superior

Las instrucciones del nivel superior del programa se convierten en `main`, primero las de los archivos importados, en el orden en que Node.js las ejecuta. Las declaraciones de nivel superior se convierten en elementos del módulo:

- una `const` que contiene un literal es una `const` de Rust;
- una variable de nivel superior que las funciones leen o modifican es un `static`, leído y escrito en su sitio;
- las funciones, las clases y las interfaces son funciones, structs y traits.

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

## Varios archivos

`import` y `export` funcionan como en TypeScript: exports con nombre, `export default`, `import * as ns`, y `require` con `module.exports` para los archivos CommonJS. Cada archivo se convierte en un `mod`, sus exports son `pub`, y un nombre declarado por dos archivos se renombra en uno de ellos.

Con `math.ts`:

```ts fragment
export function square(n: number): number {
  return n * n;
}

export const UNIT = "m²";
```

y `main.ts`:

```ts fragment
import { square, UNIT } from "./math";

console.log(`${square(12)} ${UNIT}`);
```

`npx solidify-rs main.ts` escribe:

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

Importar un archivo que no existe es un error, y también lo es un ciclo de imports (`circular import between b.ts and a.ts`).

## Los módulos de Node

De los módulos propios de Node, `fs`, `fs/promises`, `path`, `os` y `process` vienen incluidos con Solidify, escritos en TypeScript y traducidos con tu programa. `node:fs` y `fs` son el mismo módulo.

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

`readFileSync` lanza un error cuando falta el archivo, con el mensaje de Node, así que el `main` del programa devuelve un `Result`. `process.argv`, `process.cwd()` y `process.exit(code)` funcionan como en Node.js.

## Paquetes

Un import de un paquete (`import { chunk } from "lodash-es"`) se busca en las carpetas `node_modules` por encima del archivo, como hace Node.js, y el paquete se traduce desde sus fuentes junto con el programa: su `package.json` decide el archivo de entrada (`exports`, `module`, `main`).

Un paquete que necesita algo que Solidify no traduce se rechaza, con un mensaje que dice qué: módulos nativos, o módulos de Node distintos de los cinco anteriores (`http`, `child_process`, `net`…).
