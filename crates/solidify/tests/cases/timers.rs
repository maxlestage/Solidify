use std::cell::RefCell;
use std::rc::Rc;

/// Timers and promises run on an event loop once the script is done, like in Node.
async fn sleep(ms: i64) {
    JsPromise::new(|resolve, _| { js_set_timeout(move || resolve(()), ms as f64); }).await
}

async fn step(name: &str, ms: i64) -> String {
    sleep(ms).await;
    println!("done {name}");
    name.to_uppercase()
}

fn make_counter() -> impl FnMut() -> i64 {
    let count = Rc::new(RefCell::new(0));
    { let count = count.clone(); move || {
        (*count.borrow_mut()) += 1;
        (*count.borrow())
    } }
}

fn main() {
    let result = block_on(async {
        let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        js_set_timeout({ let log = log.clone(); move || { (*log.borrow_mut()).push("late".to_string()); } }, 200.0);
        js_set_timeout({ let log = log.clone(); move || {
            (*log.borrow_mut()).push("soon".to_string());
        } }, 50.0);
        let ticks = Rc::new(RefCell::new(0));
        let timer = js_timer_id();
        js_schedule(timer, { let ticks = ticks.clone(); let log = log.clone(); move || {
            (*ticks.borrow_mut()) += 1;
            (*log.borrow_mut()).push(format!("tick {}", (*ticks.borrow())));
            if (*ticks.borrow()) == 3 {
                js_clear_timer(timer);
                println!("ticks: {} {}", (*ticks.borrow()), (*log.borrow()).join(", "));
            }
        } }, 20.0, true);
        let first = step("a", 5).await;
        let second = step("b", 1).await;
        println!("{first} {second}");
        let value = JsPromise::new(|resolve, _| {
            js_set_timeout(move || resolve(42), 100.0);
        }).await;
        println!("value {value} {}", (*log.borrow()).len());
        let mut next = make_counter();
        next();
        next();
        println!("counter {}", next());
        let all = vec![step("c", 2).await, step("d", 3).await];
        println!("{}", all.join("+"));
    });
    js_run_timers();
    result
}

/// Runs a future to completion, running the timers it waits on.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
            return v;
        }
        if !js_run_next_timer() {
            panic!("a promise never settled");
        }
    }
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

/// A fresh timer id.
#[allow(dead_code)]
fn js_timer_id() -> i64 {
    JS_TIMER_ID.with(|n| {
        n.set(n.get() + 1);
        n.get()
    })
}

/// Schedules `callback` in `ms` milliseconds, again every `ms` when `repeat`.
#[allow(dead_code)]
fn js_schedule(id: i64, callback: impl FnMut() + 'static, ms: f64, repeat: bool) {
    let delay = std::time::Duration::from_secs_f64(ms.max(0.0) / 1000.0);
    JS_TIMERS.with(|timers| {
        timers.borrow_mut().push(JsTimer {
            id,
            due: std::time::Instant::now() + delay,
            every: if repeat { Some(delay) } else { None },
            callback: Box::new(callback),
        })
    });
}

/// `setTimeout(callback, ms)`: the timer id.
#[allow(dead_code)]
fn js_set_timeout(callback: impl FnMut() + 'static, ms: f64) -> i64 {
    let id = js_timer_id();
    js_schedule(id, callback, ms, false);
    id
}

/// `setInterval(callback, ms)`: the timer id.
#[allow(dead_code)]
fn js_set_interval(callback: impl FnMut() + 'static, ms: f64) -> i64 {
    let id = js_timer_id();
    js_schedule(id, callback, ms, true);
    id
}

/// `clearTimeout(id)` / `clearInterval(id)`.
#[allow(dead_code)]
fn js_clear_timer(id: i64) {
    JS_TIMERS.with(|timers| timers.borrow_mut().retain(|t| t.id != id));
    JS_CLEARED.with(|c| c.set(id));
}

/// Runs the next timer due, waiting for it; `false` when none is left.
#[allow(dead_code)]
fn js_run_next_timer() -> bool {
    let next = JS_TIMERS.with(|timers| {
        let mut timers = timers.borrow_mut();
        let i = (0..timers.len()).min_by_key(|&i| timers[i].due)?;
        Some(timers.remove(i))
    });
    let Some(mut timer) = next else {
        return false;
    };
    let now = std::time::Instant::now();
    if timer.due > now {
        std::thread::sleep(timer.due - now);
    }
    JS_CLEARED.with(|c| c.set(0));
    (timer.callback)();
    if let Some(every) = timer.every
        && JS_CLEARED.with(|c| c.get()) != timer.id
    {
        timer.due += every;
        JS_TIMERS.with(|timers| timers.borrow_mut().push(timer));
    }
    true
}

/// Runs every timer, like Node does once the script has run.
#[allow(dead_code)]
fn js_run_timers() {
    while js_run_next_timer() {}
}


/// A promise: settled by its `resolve` / `reject` callbacks, awaited by the event loop.
struct JsPromise<T> {
    state: std::rc::Rc<std::cell::RefCell<Option<Result<T, String>>>>,
}

impl<T: 'static> JsPromise<T> {
    #[allow(dead_code)]
    fn new(executor: impl FnOnce(Box<dyn Fn(T)>, Box<dyn Fn(String)>)) -> Self {
        let state = std::rc::Rc::new(std::cell::RefCell::new(None));
        let (resolve_state, reject_state) = (state.clone(), state.clone());
        executor(
            Box::new(move |value| {
                let mut slot = resolve_state.borrow_mut();
                if slot.is_none() {
                    *slot = Some(Ok(value));
                }
            }),
            Box::new(move |error| {
                let mut slot = reject_state.borrow_mut();
                if slot.is_none() {
                    *slot = Some(Err(error));
                }
            }),
        );
        JsPromise { state }
    }
}

impl<T> std::future::Future for JsPromise<T> {
    type Output = T;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<T> {
        match self.state.borrow_mut().take() {
            Some(Ok(value)) => std::task::Poll::Ready(value),
            Some(Err(error)) => panic!("{error}"),
            None => std::task::Poll::Pending,
        }
    }
}

