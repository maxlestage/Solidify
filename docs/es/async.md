# Async y temporizadores

Una función `async` devuelve una promesa que arranca de inmediato, como en JavaScript. Solidify incluye un pequeño bucle de eventos en la salida: las promesas, las microtareas y los temporizadores se ejecutan en el mismo orden que en Node.js.

## async y await

Una función `async` que devuelve `T` se convierte en una función que devuelve `Promise<T>`; su cuerpo se ejecuta como una tarea lanzada de inmediato, y `await` espera a otra promesa. `new Promise((resolve, reject) => …)` es un `Promise::new`:

```ts
function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchScore(name: string): Promise<number> {
  await wait(10);
  return name.length * 10;
}

async function main(): Promise<void> {
  const scores = await Promise.all([fetchScore("Ada"), fetchScore("Grace")]);
  console.log(scores);
}

main();
```
```rust fragment
fn wait(ms: i64) -> Promise<()> {
    Promise::new(move |resolve, _| { js_set_timeout(move || resolve(()), ms as f64); })
}

fn fetch_score(name: String) -> Promise<i64> {
    Promise::spawn(async move {
        wait(10).await;
        (name.chars().count() * 10) as i64
    })
}

fn main_() -> Promise<()> {
    Promise::spawn(async move {
        let scores = Promise::all(vec![fetch_score("Ada".to_string()), fetch_score("Grace".to_string())]).await;
        println!("{}", js_inspect(&scores.js_value()));
    })
}

fn main() {
    main_();
    js_run_timers();
}

// …
```

Una función llamada `main` pasa a llamarse `main_`: el `main` de Rust ejecuta el código de nivel superior del programa y después el bucle de eventos, hasta que no queda nada por hacer.

## then, catch y los demás

`Promise<T>` tiene los métodos de las promesas de JavaScript:

- `then`, `catch` y `finally`, con sus callbacks ejecutados como microtareas;
- `Promise.resolve`, `Promise.reject`;
- `Promise.all`, `Promise.allSettled`, `Promise.race` y `Promise.any`;
- `for await (const x of promises)`, que espera cada elemento por turno.

Una función async que lanza un error rechaza su promesa; un `await` dentro de un `try` lo captura como lo haría `catch`.

## Temporizadores

`setTimeout`, `setInterval`, `clearTimeout` y `clearInterval` añaden temporizadores al bucle de eventos. Primero se ejecuta el código del programa, luego las microtareas y después los temporizadores, en orden:

```ts
setTimeout(() => console.log("later"), 20);
setTimeout(() => console.log("sooner"), 10);
Promise.resolve().then(() => console.log("microtask"));
console.log("now");
```
```rust fragment
fn main() {
    js_set_timeout(move || println!("later"), 20.0);
    js_set_timeout(move || println!("sooner"), 10.0);
    Promise::resolved(()).then(move |_| println!("microtask"));
    println!("now");
    js_run_timers();
}

/// A timer made by `setTimeout` or `setInterval`.
struct JsTimer {
    id: i64,
    due: std::time::Instant,
    every: Option<std::time::Duration>,
    callback: Box<dyn FnMut()>,
}

thread_local! {
    static JS_TIMERS: std::cell::RefCell<Vec<JsTimer>> = const { std::cell::RefCell::new(Vec::new()) };
    static JS_TIMER_ID: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
    /// The interval being run was cleared from inside its callback.
    static JS_CLEARED: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

// …
```

Imprime `now`, `microtask`, `sooner`, `later`, como Node.js.

> [!NOTE]
> Los temporizadores usan el reloj real. Dos temporizadores separados por pocos milisegundos pueden intercambiarse con mucha carga, como puede ocurrir en Node.js. Los generadores asíncronos (`async function*`) no se admiten.
