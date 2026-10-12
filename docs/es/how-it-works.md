# Cómo funciona

Solidify traduce en tres pasadas: analiza el programa, infiere un tipo para cada valor y después genera Rust. Cada pasada es determinista: el mismo programa da siempre el mismo Rust, y ninguna pasada adivina.

## 1. Analizar

Un analizador escrito a mano lee TypeScript moderno: clases, interfaces, enums, alias de tipo, genéricos, funciones flecha, plantillas de cadenas, desestructuración, spread, encadenamiento opcional, `async`/`await`, generadores, etiquetas, expresiones regulares. JavaScript es TypeScript sin las anotaciones, y lo lee el mismo analizador.

Cuando el programa importa otros archivos, también se leen, en el orden en que Node.js los ejecutaría: primero los archivos importados. Un error de sintaxis detiene todo aquí, con el archivo, la línea y la columna.

## 2. Inferir

Las anotaciones son un punto de partida, no toda la historia: cada valor recibe un tipo, de su anotación cuando la tiene, y de su uso en los demás casos. Los tipos se resuelven juntos, por unificación, en todo el programa.

- **Los números siguen siendo enteros** (`i64`) hasta que un literal decimal, una división, `Math`, `parseFloat` o JSON digan lo contrario; entonces el número, y cada número al que fluye, es un `f64`.
- **`T | null`** y los campos opcionales se convierten en `Option<T>`; **`A | B`** se convierte en un enum.
- **Una función que lanza un error**, o que llama a otra que lo hace fuera de un `try`, devuelve `Result<T, String>`.
- **Un parámetro que nada restringe** se convierte en un parámetro de tipo, con los límites que necesitan sus usos.
- **`any` y `unknown`** son valores conocidos en tiempo de ejecución (`JsValue`).

Aquí `total` solo suma números enteros, mientras que `half` divide:

```ts
function total(values: number[]): number {
  let sum = 0;
  for (const v of values) {
    sum += v;
  }
  return sum;
}

function half(n: number): number {
  return n / 2;
}

console.log(total([1, 2, 3]), half(5));
```
```rust fragment
fn total(values: &[i64]) -> i64 {
    let mut sum = 0;
    for &v in values {
        sum += v;
    }
    sum
}

fn half(n: i64) -> f64 {
    n as f64 / 2.0
}

fn main() {
    println!("{} {}", total(&[1, 2, 3]), js_number_text(half(5)));
}

// …
```

`js_number_text`, una de las funciones auxiliares escritas después de `main`, imprime `2.5` como lo hace JavaScript.

## 3. Generar

La última pasada escribe Rust y decide sobre la marcha la propiedad de cada valor:

- **Los parámetros se toman prestados** (`&str`, `&[T]`, `&T`) salvo que la función los modifique (`&mut`) o los guarde.
- **Una variable se mueve en su último uso**, y se clona antes si todavía hace falta.
- **Los bucles** se convierten en rangos (`for i in 1..=10`) e iteradores; `map`, `filter`, `reduce` se convierten en cadenas de iteradores.
- **`switch`** se convierte en `match`; **`if (x !== null)`** se convierte en `if let Some(x)`; **`if (x === null) return`** se convierte en `let Some(x) = x else { return }`.

```ts
function find(names: string[], prefix: string): string | null {
  for (const name of names) {
    if (name.startsWith(prefix)) {
      return name;
    }
  }
  return null;
}

const found = find(["Ada", "Grace", "Linus"], "Gr");
console.log(found ?? "nobody");
```
```rust
fn find(names: &[String], prefix: &str) -> Option<String> {
    for name in names {
        if name.starts_with(prefix) {
            return Some(name.clone());
        }
    }
    None
}

fn main() {
    let found = find(&["Ada".to_string(), "Grace".to_string(), "Linus".to_string()], "Gr");
    println!("{}", found.unwrap_or("nobody".to_string()));
}
```

## La salida

- **Un único archivo**, para la edición 2024 de Rust, que `rustc` compila por sí solo: ningún crate que añadir.
- **Los nombres siguen las convenciones de Rust:** `minMax` se convierte en `min_max`, un `const line` de nivel superior se convierte en `const LINE`, y una variable que se llama como una palabra clave de Rust se escapa (`r#match`).
- **Los comentarios se conservan**, donde estaban.
- **Las funciones auxiliares van al final.** Lo que le falta a la biblioteca estándar de Rust (imprimir números como lo hace JavaScript, `console.log` de objetos, expresiones regulares, fechas, promesas y temporizadores) se escribe después de `main`, y solo lo que usa el programa.
- **Un encabezado** nombra el archivo fuente; `--no-header` lo omite.

La página de [tipos](types.md) recorre cada tipo de valor, y [funciones y propiedad](ownership.md) los préstamos, los movimientos y las closures.
