# Inicio rápido

De un archivo TypeScript a un binario Rust que se ejecuta, en cuatro comandos. Necesitas Node.js 20 o posterior y Rust ([instalación](installation.md)).

## Escribe un programa

Guarda esto como `tasks.ts`. Node.js imprime `1 left: Ship Rust`; el Rust también.

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

La interfaz es un struct, el array un `Vec`, `filter` y `map` una cadena de iteradores, la plantilla de cadena un `println!`.

## Tradúcelo

```sh
npx solidify-rs tasks.ts -o tasks.rs
```

`tasks.rs` contiene ahora el Rust de arriba. Sin `-o`, va a la salida estándar.

## Compílalo y ejecútalo

```sh
rustc --edition 2024 -O tasks.rs -o tasks
./tasks
```

Imprime `1 left: Ship Rust`, como Node.js.

## Un programa de varios archivos

Solidify sigue los imports del archivo que le das y escribe un único archivo Rust, con un módulo por cada archivo importado. Con `math.ts`:

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

`npx solidify-rs main.ts -o app.rs` escribe:

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

[Módulos y programas](modules.md) explica cómo se traducen los archivos, los exports y el código de nivel superior.

## Cuando algo no se traduce

`--check` informa de los errores sin escribir nada. Solidify lee TypeScript de forma estricta: leer un campo de un valor que puede ser `null` es un error, como en el modo estricto de TypeScript.

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

Comprueba primero el valor (`if (name !== null)`), o dale un valor por defecto con `??`, y el programa se traduce. [Errores](errors.md) enumera los mensajes y qué hacer con ellos.

## Siguientes pasos

- Prueba tus propios programas en la zona de pruebas, en la página de inicio: traduce mientras escribes.
- Lee [cómo funciona](how-it-works.md) y después la página del tema que necesites.
