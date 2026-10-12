# Errores

Aquí conviven dos tipos de errores: los que el programa lanza mientras se ejecuta, que se convierten en el `Result` de Rust, y los que Solidify señala cuando no puede traducir un programa, con el archivo, la línea y la columna.

## throw se convierte en Result

Una función que lanza un error, o que llama a otra que lo hace fuera de un `try`, devuelve `Result<T, String>`: `throw new Error("…")` se convierte en `return Err(…)`, y quienes la llaman propagan el error con `?`. Un bloque `try` se convierte en una closure cuyo error recibe `catch`, como mensaje:

```ts
function withdraw(balance: number, amount: number): number {
  if (amount > balance) {
    throw new Error(`only ${balance} left`);
  }
  return balance - amount;
}

try {
  console.log(withdraw(10, 4));
  console.log(withdraw(10, 40));
} catch (e) {
  console.log(`refused: ${e.message}`);
}
```
```rust
fn withdraw(balance: i64, amount: i64) -> Result<i64, String> {
    if amount > balance {
        return Err(format!("only {balance} left"));
    }
    Ok(balance - amount)
}

fn main() {
    let result_: Result<(), String> = (|| {
        println!("{}", withdraw(10, 4)?);
        println!("{}", withdraw(10, 40)?);
        Ok(())
    })();
    if let Err(e) = result_ {
        println!("refused: {e}");
    }
}
```

Un error que nadie captura llega a `main`, que lo devuelve: el programa se detiene con el código de salida 1 e imprime el mensaje en la salida de error estándar (`Error: "only 10 left"`), donde Node.js imprimiría una pila de llamadas.

## Clases de error

Una clase que extiende `Error` conserva sus campos, su mensaje y su nombre a través del `Result`; `instanceof` en `catch` la vuelve a encontrar:

```ts
class NotFound extends Error {
  constructor(public key: string) {
    super(`no ${key}`);
    this.name = "NotFound";
  }
}

function lookup(key: string): string {
  if (key !== "a") {
    throw new NotFound(key);
  }
  return "found";
}

try {
  lookup("b");
} catch (e) {
  if (e instanceof NotFound) {
    console.log(`${e.name}: ${e.key}`);
  }
}
```
```rust fragment
#[derive(Debug, Clone, PartialEq)]
struct NotFound {
    pub key: String,
    pub message: String,
    pub name: String,
}

impl NotFound {
    pub fn new(key: String) -> Self {
        Self { key: key.clone(), message: format!("no {key}"), name: "NotFound".to_string() }
    }
}

fn lookup(key: &str) -> Result<String, String> {
    if key != "a" {
        return Err(js_error_encode(&NotFound::new(key.to_string()).js_value(), "NotFound,Error"));
    }
    Ok("found".to_string())
}

fn main() {
    let result_: Result<(), String> = (|| {
        lookup("b")?;
        Ok(())
    })();
    if let Err(e) = result_ {
        if let Some(e) = js_error_as::<NotFound>(&e, "NotFound") {
            println!("{}: {}", e.name, e.key);
        }
    }
}

// …
```

`JSON.parse` lanza los mismos mensajes `SyntaxError` que V8, así que un programa que los imprime imprime el mismo texto.

## Los mensajes de Solidify

Cuando un programa no se puede traducir, Solidify dice por qué y dónde: el mensaje, luego el archivo, la línea y la columna, la propia línea y una marca bajo el lugar. `npx solidify-rs --check` solo los muestra, sin escribir nada.

### Un valor que puede ser null

Leer un valor que puede ser `null` o `undefined` es un error, como en el modo estricto de TypeScript:

```ts error
const names: string[] = ["Ada"];
const first = names.find((n) => n.startsWith("A"));
console.log(first.length);
```
```text
error: `first` may be null here (string | null): test it with `if (… !== null)`, or use `?.` or `!`
  --> app.ts:3:13
   |
  3| console.log(first.length);
   |             ^^^^^^^^^^^^
```

Compruébalo primero (`if (first !== undefined)`), dale un valor por defecto (`first ?? ""`), léelo con `?.`, o afírmalo con `!` cuando sepas que está ahí.

### Un tipo que no encaja

Una variable conserva un único tipo. Darle un valor de otro tipo es un error:

```ts error
let count = 0;
count = "many";
```
```text
error: expected number (integer), found string
  --> app.ts:2:9
   |
  2| count = "many";
   |         ^^^^^^
```

Usa dos variables, o declara la unión que quieres decir (`let count: number | string = 0`).

### Lo que no se admite

Algunas cosas se rechazan, cada una con un mensaje que la nombra:

```ts error
async function* ticks() {
  yield 1;
}
```
```text
error: async generators are not supported
  --> app.ts:1:7
   |
  1| async function* ticks() {
   |       ^^^^^^^^^^^^^^^^^^^
```

Los [límites](limits.md) las enumeran todas.

## En la biblioteca Rust

`solidify::transpile` devuelve un `solidify::Error`: su `message` y un `span` (posiciones en bytes dentro del código fuente). `pretty(source)` lo formatea como arriba; la página de la [biblioteca Rust](library.md#errores) muestra cómo.
