# Async et minuteries

Une fonction `async` renvoie une promesse qui démarre tout de suite, comme en JavaScript. Solidify livre une petite boucle d'événements dans la sortie : promesses, microtâches et minuteries s'exécutent dans le même ordre que dans Node.js.

## async et await

Une fonction `async` qui renvoie `T` devient une fonction qui renvoie `Promise<T>` ; son corps s'exécute comme une tâche lancée tout de suite, et `await` attend une autre promesse. `new Promise((resolve, reject) => …)` est un `Promise::new` :

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

Une fonction nommée `main` est renommée `main_` : le `main` de Rust exécute le code de niveau supérieur du programme, puis la boucle d'événements jusqu'à ce qu'il ne reste plus rien à faire.

## then, catch et les autres

`Promise<T>` a les méthodes des promesses de JavaScript :

- `then`, `catch` et `finally`, dont les callbacks s'exécutent comme des microtâches ;
- `Promise.resolve`, `Promise.reject` ;
- `Promise.all`, `Promise.allSettled`, `Promise.race` et `Promise.any` ;
- `for await (const x of promises)`, qui attend chaque élément à son tour.

Une fonction async qui lève une erreur rejette sa promesse ; `await` dans un `try` l'attrape comme le ferait `catch`.

## Minuteries

`setTimeout`, `setInterval`, `clearTimeout` et `clearInterval` ajoutent des minuteries à la boucle d'événements. Le code du programme s'exécute d'abord, puis les microtâches, puis les minuteries dans l'ordre :

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

Il affiche `now`, `microtask`, `sooner`, `later`, comme Node.js.

> [!NOTE]
> Les minuteries suivent l'horloge réelle. Deux minuteries à quelques millisecondes d'écart peuvent s'inverser sous forte charge, comme dans Node.js. Les générateurs asynchrones (`async function*`) ne sont pas pris en charge.
