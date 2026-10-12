# Démarrage rapide

D'un fichier TypeScript à un binaire Rust qui tourne, en quatre commandes. Il vous faut Node.js 20 ou plus récent et Rust ([installation](installation.md)).

## Écrire un programme

Enregistrez ceci sous le nom `tasks.ts`. Node.js affiche `1 left: Ship Rust` ; le Rust aussi.

```ts
interface Task {
  title: string;
  done: boolean;
}

const tasks: Task[] = [
  { title: "Write TypeScript", done: true },
  { title: "Ship Rust", done: false },
];

const left = tasks.filter((t) => !t.done).map((t) => t.title);
console.log(`${left.length} left: ${left.join(", ")}`);
```
```rust
#[derive(Debug, Clone, PartialEq)]
struct Task {
    pub title: String,
    pub done: bool,
}

fn main() {
    let tasks = vec![Task { title: "Write TypeScript".to_string(), done: true }, Task { title: "Ship Rust".to_string(), done: false }];
    let left = tasks.iter().filter(|t| !t.done).cloned().collect::<Vec<_>>().into_iter().map(|t| t.title.clone()).collect::<Vec<_>>();
    println!("{} left: {}", left.len(), left.join(", "));
}
```

L'interface devient un struct, le tableau un `Vec`, `filter` et `map` une chaîne d'itérateurs, le gabarit de chaîne un `println!`.

## Le traduire

```sh
npx solidify-rs tasks.ts -o tasks.rs
```

`tasks.rs` contient maintenant le Rust ci-dessus. Sans `-o`, il va sur la sortie standard.

## Le compiler et le lancer

```sh
rustc --edition 2024 -O tasks.rs -o tasks
./tasks
```

Il affiche `1 left: Ship Rust`, comme Node.js.

## Un programme de plusieurs fichiers

Solidify suit les imports du fichier que vous lui donnez, et écrit un seul fichier Rust avec un module par fichier importé. Avec `math.ts` :

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

`npx solidify-rs main.ts -o app.rs` écrit :

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

[Modules et programmes](modules.md) explique comment les fichiers, les exports et le code de niveau supérieur sont traduits.

## Quand quelque chose ne se traduit pas

`--check` signale les erreurs sans rien écrire. Solidify lit le TypeScript strictement : lire un champ d'une valeur qui peut être `null` est une erreur, comme dans le mode strict de TypeScript.

```ts error
function initial(name: string | null): string {
  return name.charAt(0);
}

console.log(initial("Ada"));
```
```text
error: `name` may be null here: test it with `if (… !== null)` first
  --> app.ts:2:10
   |
  2|   return name.charAt(0);
   |          ^^^^
```

Testez d'abord la valeur (`if (name !== null)`), ou donnez une valeur par défaut avec `??`, et le programme se traduit. [Erreurs](errors.md) liste les messages et ce qu'il faut faire pour chacun.

## Ensuite

- Essayez vos propres programmes dans le bac à sable, sur la page d'accueil : il traduit à mesure que vous tapez.
- Lisez [le fonctionnement](how-it-works.md), puis la page du sujet qui vous intéresse.
