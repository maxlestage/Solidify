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
    // Like Node: a delay under 1 ms (or not a number) is 1 ms.
    let ms = if ms >= 1.0 { ms } else { 1.0 };
    let delay = std::time::Duration::from_secs_f64(ms / 1000.0);
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

/// Runs the tasks and every timer, like Node does once the script has run.
#[allow(dead_code)]
fn js_run_timers() {
    loop {
        js_run_tasks();
        js_report_unhandled();
        if !js_run_next_timer() {
            break;
        }
    }
}

type JsTask = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

thread_local! {
    /// The async functions waiting for a promise.
    static JS_TASKS: std::cell::RefCell<Vec<Option<JsTask>>> = const { std::cell::RefCell::new(Vec::new()) };
    /// Counts settled promises: the tasks move on when it changes.
    static JS_TICK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Rejected promises nothing waited for: (promise, message).
    static JS_UNHANDLED: std::cell::RefCell<Vec<(usize, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Runs a task until it waits; kept for later when it does.
#[allow(dead_code)]
fn js_start_task(mut task: JsTask) {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    if task.as_mut().poll(&mut cx).is_pending() {
        JS_TASKS.with(|t| t.borrow_mut().push(Some(task)));
    }
}

/// Runs the waiting tasks until none can move on.
#[allow(dead_code)]
fn js_run_tasks() {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        let before = JS_TICK.with(|t| t.get());
        let mut i = 0;
        while let Some(slot) = JS_TASKS.with(|t| t.borrow_mut().get_mut(i).map(Option::take)) {
            if let Some(mut task) = slot
                && task.as_mut().poll(&mut cx).is_pending()
            {
                JS_TASKS.with(|t| t.borrow_mut()[i] = Some(task));
            }
            i += 1;
        }
        JS_TASKS.with(|t| t.borrow_mut().retain(Option::is_some));
        if JS_TICK.with(|t| t.get()) == before {
            break;
        }
    }
}

/// A rejection nothing handled ends the program, like in Node.
#[allow(dead_code)]
fn js_report_unhandled() {
    if let Some((_, message)) = JS_UNHANDLED.with(|u| u.borrow().first().cloned()) {
        eprintln!("Uncaught {message}");
        std::process::exit(1);
    }
}
