# Modules et programmes

Un programme, c'est un fichier et les fichiers qu'il importe. Solidify suit les imports depuis le fichier que vous lui donnez et écrit un seul fichier Rust : chaque fichier importé devient un `mod`, et le code de niveau supérieur devient `main`.

## Code de niveau supérieur

Les instructions au niveau supérieur du programme deviennent `main`, fichiers importés d'abord, dans l'ordre où Node.js les exécute. Les déclarations de niveau supérieur deviennent des éléments du module :

- un `const` qui contient un littéral est un `const` Rust ;
- une variable de niveau supérieur que des fonctions lisent ou modifient est un `static`, lu et écrit sur place ;
- les fonctions, classes et interfaces sont des fonctions, des structs et des traits.

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

## Plusieurs fichiers

`import` et `export` fonctionnent comme en TypeScript : exports nommés, `export default`, `import * as ns`, et `require` avec `module.exports` pour les fichiers CommonJS. Chaque fichier devient un `mod`, ses exports sont `pub`, et un nom déclaré par deux fichiers est renommé dans l'un d'eux.

Avec `math.ts` :

```ts fragment
export function square(n: number): number {
  return n * n;
}

export const UNIT = "m²";
```

et `main.ts` :

```ts fragment
import { square, UNIT } from "./math";

console.log(`${square(12)} ${UNIT}`);
```

`npx solidify-rs main.ts` écrit :

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

L'import d'un fichier qui n'existe pas est une erreur, tout comme un cycle d'imports (`circular import between b.ts and a.ts`).

## Les modules de Node

Parmi les modules propres à Node, `fs`, `fs/promises`, `path`, `os` et `process` sont livrés avec Solidify, écrits en TypeScript et traduits avec votre programme. `node:fs` et `fs` sont le même module.

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

`readFileSync` lève une erreur quand le fichier manque, avec le message de Node, donc le `main` du programme renvoie un `Result`. `process.argv`, `process.cwd()` et `process.exit(code)` fonctionnent comme dans Node.js.

## Paquets

L'import d'un paquet (`import { chunk } from "lodash-es"`) est cherché dans les dossiers `node_modules` au-dessus du fichier, comme le fait Node.js, et le paquet est traduit depuis ses sources avec le programme : son `package.json` décide du fichier d'entrée (`exports`, `module`, `main`).

Un paquet qui a besoin de ce que Solidify ne traduit pas est refusé, avec un message qui dit quoi : modules natifs, ou modules de Node autres que les cinq ci-dessus (`http`, `child_process`, `net`…).
