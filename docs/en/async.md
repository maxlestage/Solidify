# Async and timers

An `async` function returns a promise that starts at once, as in JavaScript. Solidify ships a small event loop in the output: promises, microtasks and timers run in the same order as in Node.js.

## async and await

An `async` function returning `T` becomes a function returning `Promise<T>`; its body runs as a task started at once, and `await` waits for another promise. `new Promise((resolve, reject) => …)` is a `Promise::new`:

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

A function named `main` is renamed `main_`: Rust's `main` runs the program's top-level code, then the event loop until nothing is left to do.

## then, catch and the others

`Promise<T>` has the methods of JavaScript's promises:

- `then`, `catch` and `finally`, with their callbacks run as microtasks;
- `Promise.resolve`, `Promise.reject`;
- `Promise.all`, `Promise.allSettled`, `Promise.race` and `Promise.any`;
- `for await (const x of promises)`, which awaits each item in turn.

An async function that throws rejects its promise; `await` in a `try` catches it as `catch` would.

## Timers

`setTimeout`, `setInterval`, `clearTimeout` and `clearInterval` add timers to the event loop. The program's code runs first, then the microtasks, then the timers in order:

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

It prints `now`, `microtask`, `sooner`, `later`, as Node.js does.

> [!NOTE]
> Timers use the real clock. Two timers a few milliseconds apart may swap under heavy load, as they can in Node.js. Async generators (`async function*`) are not supported.
