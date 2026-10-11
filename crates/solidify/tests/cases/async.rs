fn fetch_value(x: i64) -> Promise<i64> {
    Promise::spawn(async move {
        x * 2
    })
}

fn total(xs: Vec<i64>) -> Promise<i64> {
    Promise::spawn(async move {
        let mut t = 0;
        for &x in &xs {
            t += fetch_value(x).await;
        }
        t
    })
}

fn main() {
    let result = block_on(async {
        let result = total(vec![1, 2, 3]).await;
        println!("total: {result}");
    });
    js_run_timers();
    result
}

/// Runs the top level, with the tasks and timers it waits for.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let mut future = std::pin::pin!(future);
    loop {
        if let std::task::Poll::Ready(v) = future.as_mut().poll(&mut cx) {
            return v;
        }
        let before = JS_TICK.with(|t| t.get());
        js_run_tasks();
        if JS_TICK.with(|t| t.get()) != before {
            continue;
        }
        js_report_unhandled();
        if !js_run_next_timer() {
            // Node leaves a top-level await nothing will settle.
            std::process::exit(13);
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


/// A promise: a task started at once, whose value every `await` gets.
struct Promise<T>(std::rc::Rc<JsPromiseState<T>>);

/// What a promise holds: its result once settled, and whether something waits for it.
struct JsPromiseState<T> {
    result: std::cell::RefCell<Option<Result<T, String>>>,
    handled: std::cell::Cell<bool>,
}

impl<T> Clone for Promise<T> {
    fn clone(&self) -> Self {
        Promise(self.0.clone())
    }
}

impl<T> std::fmt::Debug for Promise<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = match &*self.0.result.borrow() {
            None => "<pending>",
            Some(Ok(_)) => "<fulfilled>",
            Some(Err(_)) => "<rejected>",
        };
        write!(f, "Promise {{ {state} }}")
    }
}

impl<T> PartialEq for Promise<T> {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

#[allow(dead_code)]
impl<T: Clone + 'static> Promise<T> {
    fn id(&self) -> usize {
        std::rc::Rc::as_ptr(&self.0) as *const () as usize
    }

    fn pending() -> Self {
        Promise(std::rc::Rc::new(JsPromiseState {
            result: std::cell::RefCell::new(None),
            handled: std::cell::Cell::new(false),
        }))
    }

    /// Something waits for the promise: its rejection is handled.
    fn handle(&self) {
        self.0.handled.set(true);
        let id = self.id();
        JS_UNHANDLED.with(|u| u.borrow_mut().retain(|(p, _)| *p != id));
    }

    /// Settles the promise once; later calls do nothing.
    fn settle(&self, result: Result<T, String>) {
        let mut slot = self.0.result.borrow_mut();
        if slot.is_some() {
            return;
        }
        if let Err(message) = &result
            && !self.0.handled.get()
        {
            JS_UNHANDLED.with(|u| u.borrow_mut().push((self.id(), message.clone())));
        }
        *slot = Some(result);
        JS_TICK.with(|t| t.set(t.get() + 1));
    }

    /// `async function`: runs the body now, until it waits.
    fn spawn(body: impl std::future::Future<Output = T> + 'static) -> Self {
        Self::spawn_result(async move { Ok(body.await) })
    }

    /// An async body that may throw: a throw rejects the promise.
    fn spawn_result(body: impl std::future::Future<Output = Result<T, String>> + 'static) -> Self {
        let promise = Self::pending();
        let settle = promise.clone();
        js_start_task(Box::pin(async move { settle.settle(body.await) }));
        promise
    }

    /// `new Promise((resolve, reject) => …)`.
    fn new(executor: impl FnOnce(Box<dyn Fn(T)>, Box<dyn Fn(String)>)) -> Self {
        let promise = Self::pending();
        let (resolve, reject) = (promise.clone(), promise.clone());
        executor(
            Box::new(move |value| resolve.settle(Ok(value))),
            Box::new(move |error| reject.settle(Err(error))),
        );
        promise
    }

    /// `Promise.resolve(value)`.
    fn resolved(value: T) -> Self {
        let promise = Self::pending();
        promise.settle(Ok(value));
        promise
    }

    /// `Promise.reject(error)`.
    fn rejected(error: String) -> Self {
        let promise = Self::pending();
        promise.settle(Err(error));
        promise
    }

    /// Waits for the promise: its value, or the error it was rejected with. Like in
    /// JavaScript, what follows runs later even when it is settled already (a microtask).
    fn settled(&self) -> impl std::future::Future<Output = Result<T, String>> + 'static {
        self.handle();
        let me = self.clone();
        let mut queued = false;
        std::future::poll_fn(move |_| match &*me.0.result.borrow() {
            _ if !queued => {
                queued = true;
                JS_TICK.with(|t| t.set(t.get() + 1));
                std::task::Poll::Pending
            }
            Some(result) => std::task::Poll::Ready(result.clone()),
            None => std::task::Poll::Pending,
        })
    }

    /// `p.then(f)`.
    fn then<U: Clone + 'static>(&self, f: impl FnOnce(T) -> U + 'static) -> Promise<U> {
        let done = self.settled();
        Promise::spawn_result(async move { Ok(f(done.await?)) })
    }

    /// `p.then(f)` where `f` gives a promise: its value.
    fn then_promise<U: Clone + 'static>(
        &self,
        f: impl FnOnce(T) -> Promise<U> + 'static,
    ) -> Promise<U> {
        let done = self.settled();
        Promise::spawn_result(async move { f(done.await?).settled().await })
    }

    /// `p.then(f, g)`.
    fn then_else<U: Clone + 'static>(
        &self,
        f: impl FnOnce(T) -> U + 'static,
        g: impl FnOnce(String) -> U + 'static,
    ) -> Promise<U> {
        let done = self.settled();
        Promise::spawn(async move {
            match done.await {
                Ok(v) => f(v),
                Err(e) => g(e),
            }
        })
    }

    /// `p.catch(g)` where `g` gives a value in place of the error.
    fn catch(&self, g: impl FnOnce(String) -> T + 'static) -> Promise<T> {
        let done = self.settled();
        Promise::spawn(async move { done.await.unwrap_or_else(g) })
    }

    /// `p.catch(g)` where `g` gives a promise in place of the error.
    fn catch_promise(&self, g: impl FnOnce(String) -> Promise<T> + 'static) -> Promise<T> {
        let done = self.settled();
        Promise::spawn_result(async move {
            match done.await {
                Ok(v) => Ok(v),
                Err(e) => g(e).settled().await,
            }
        })
    }

    /// `p.catch(g)` where `g` gives nothing: no value when it failed.
    fn catch_none(&self, g: impl FnOnce(String) + 'static) -> Promise<Option<T>> {
        let done = self.settled();
        Promise::spawn(async move {
            match done.await {
                Ok(v) => Some(v),
                Err(e) => {
                    g(e);
                    None
                }
            }
        })
    }

    /// `p.finally(h)`.
    fn finally(&self, h: impl FnOnce() + 'static) -> Promise<T> {
        let done = self.settled();
        Promise::spawn_result(async move {
            let result = done.await;
            h();
            result
        })
    }

    /// `Promise.all(promises)`: every value, or the first error.
    fn all(promises: Vec<Promise<T>>) -> Promise<Vec<T>> {
        promises.iter().for_each(Promise::handle);
        Promise::spawn_result(async move {
            let mut values = Vec::with_capacity(promises.len());
            for p in &promises {
                values.push(p.settled().await?);
            }
            Ok(values)
        })
    }

    /// `Promise.allSettled(promises)`: how each one ended.
    fn all_settled<R: Clone + 'static>(
        promises: Vec<Promise<T>>,
        each: impl Fn(Result<T, String>) -> R + 'static,
    ) -> Promise<Vec<R>> {
        promises.iter().for_each(Promise::handle);
        Promise::spawn(async move {
            let mut out = Vec::with_capacity(promises.len());
            for p in &promises {
                out.push(each(p.settled().await));
            }
            out
        })
    }

    /// `Promise.race(promises)`: the first to settle.
    fn race(promises: Vec<Promise<T>>) -> Promise<T> {
        promises.iter().for_each(Promise::handle);
        Promise::spawn_result(std::future::poll_fn(move |_| {
            for p in &promises {
                if let Some(result) = &*p.0.result.borrow() {
                    return std::task::Poll::Ready(result.clone());
                }
            }
            std::task::Poll::Pending
        }))
    }

    /// `Promise.any(promises)`: the first value, or an error when all fail.
    fn any(promises: Vec<Promise<T>>) -> Promise<T> {
        promises.iter().for_each(Promise::handle);
        Promise::spawn_result(std::future::poll_fn(move |_| {
            let mut failed = 0;
            for p in &promises {
                match &*p.0.result.borrow() {
                    Some(Ok(v)) => return std::task::Poll::Ready(Ok(v.clone())),
                    Some(Err(_)) => failed += 1,
                    None => {}
                }
            }
            if failed == promises.len() {
                return std::task::Poll::Ready(Err("All promises were rejected".to_string()));
            }
            std::task::Poll::Pending
        }))
    }
}

/// `await p`: the value; a rejection nothing catches ends the program.
impl<T: Clone + 'static> std::future::IntoFuture for Promise<T> {
    type Output = T;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = T>>>;
    fn into_future(self) -> Self::IntoFuture {
        let done = self.settled();
        Box::pin(async move {
            match done.await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Uncaught {e}");
                    std::process::exit(1)
                }
            }
        })
    }
}

impl<T: Clone + 'static> std::future::IntoFuture for &Promise<T> {
    type Output = T;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = T>>>;
    fn into_future(self) -> Self::IntoFuture {
        self.clone().into_future()
    }
}

