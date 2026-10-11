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
