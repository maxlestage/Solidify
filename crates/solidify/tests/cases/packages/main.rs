use std::collections::HashMap;

mod node_os {
    use super::*;

    /// Node's `os` module.
    pub const EOL: &str = "\n";

    pub fn platform() -> String {
        js_node_os_info("platform")
    }

    pub fn arch() -> String {
        js_node_os_info("arch")
    }

    pub fn r#type() -> String {
        js_node_os_info("type")
    }

    pub fn homedir() -> String {
        js_node_os_info("homedir")
    }

    pub fn tmpdir() -> String {
        js_node_os_info("tmpdir")
    }

    pub fn hostname() -> String {
        js_node_os_info("hostname")
    }

    pub fn available_parallelism() -> i64 {
        js_node_os_cpus()
    }
}

pub use node_os::*;

mod greet {
    use super::*;

    pub(crate) fn greet<T: Clone + std::fmt::Display>(name: T) -> String {
        format!("hello {name}")
    }

    pub(crate) fn shout(s: &str) -> String {
        s.to_uppercase()
    }
}

pub use greet::*;

mod counter {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub(crate) struct Counter {
        pub n: i64,
    }

    impl Counter {
        pub fn new() -> Self {
            Self { n: 0 }
        }

        pub fn inc(&mut self) -> i64 {
            self.n += 1;
            self.n
        }
    }
}

pub use counter::*;

mod node_path {
    use super::*;

    /// Node's `path` module (POSIX), written in the subset Solidify translates.
    pub const SEP: &str = "/";
    pub const DELIMITER: &str = ":";

    #[derive(Debug, Clone, PartialEq)]
    pub struct ParsedPath {
        pub root: String,
        pub dir: String,
        pub base: String,
        pub ext: String,
        pub name: String,
    }

    pub fn is_absolute(p: &str) -> bool {
        p.starts_with("/")
    }

    pub fn normalize(p: &str) -> String {
        if p == "" {
            return ".".to_string();
        }
        let absolute = p.starts_with("/");
        let trailing = p.ends_with("/");
        let mut out: Vec<String> = Vec::new();
        for part in p.split("/").map(String::from).collect::<Vec<String>>() {
            if &part == "" || &part == "." {
                continue;
            }
            if &part == ".." {
                if out.len() > 0 && &out[out.len() - 1] != ".." {
                    out.pop();
                } else if !absolute {
                    out.push("..".to_string());
                }
                continue;
            }
            out.push(part);
        }
        let mut result = out.join("/");
        if absolute {
            result = format!("/{result}");
        }
        if &result == "" {
            return if absolute { "/".to_string() } else { ".".to_string() };
        }
        if trailing && &result != "/" {
            result.push_str("/");
        }
        result
    }

    pub fn join(parts: &[String]) -> String {
        let kept = parts.iter().filter(|p| p.as_str() != "").cloned().collect::<Vec<_>>();
        if (kept.len() as i64) == 0 {
            return ".".to_string();
        }
        normalize(&kept.join("/"))
    }

    pub fn resolve(parts: &[String]) -> String {
        let mut result = "".to_string();
        for i in (0..parts.len()).rev() {
            let p = parts[i].clone();
            if &p == "" {
                continue;
            }
            result = if &result == "" { p.clone() } else { format!("{p}/{result}") };
            if p.starts_with("/") {
                break;
            }
        }
        if !(result.starts_with("/")) {
            result = if &result == "" { js_node_os_info("cwd") } else { format!("{}/{result}", js_node_os_info("cwd")) };
        }
        let n = normalize(&result);
        if n.chars().count() > 1 && n.ends_with("/") { js_slice(&n, 0, Some(-1)) } else { n }
    }

    pub fn dirname(p: &str) -> String {
        if p == "" {
            return ".".to_string();
        }
        let mut end = p.chars().count() as i64;
        while end > 1 && &js_char_at(p, end - 1) == "/" {
            end -= 1;
        }
        let i = js_last_index_of_from(p, "/", end - 1);
        if i < 0 {
            return ".".to_string();
        }
        if i == 0 {
            return "/".to_string();
        }
        js_slice(p, 0, Some(i))
    }

    pub fn basename(p: &str, ext: Option<&str>) -> String {
        let mut end = p.chars().count() as i64;
        while end > 1 && &js_char_at(p, end - 1) == "/" {
            end -= 1;
        }
        let trimmed = js_slice(p, 0, Some(end));
        let i = js_last_index_of(&trimmed, "/");
        let base = if i >= 0 { js_slice(&trimmed, i + 1, None) } else { trimmed };
        if let Some(ext) = ext && ext != &base && base.ends_with(ext) {
            return js_slice(&base, 0, Some((base.chars().count() - ext.chars().count()) as i64));
        }
        base
    }

    pub fn extname(p: &str) -> String {
        let base = basename(p, None);
        let i = js_last_index_of(&base, ".");
        if i <= 0 {
            return "".to_string();
        }
        js_slice(&base, i, None)
    }

    pub fn relative(from: String, to: String) -> String {
        let a = resolve(&[from]).split("/").map(String::from).collect::<Vec<String>>().into_iter().filter(|x| x.as_str() != "").collect::<Vec<_>>();
        let b = resolve(&[to]).split("/").map(String::from).collect::<Vec<String>>().into_iter().filter(|x| x.as_str() != "").collect::<Vec<_>>();
        let mut common = 0;
        while common < a.len() as i64 && common < b.len() as i64 && &a[common as usize] == &b[common as usize] {
            common += 1;
        }
        let mut up: Vec<String> = Vec::new();
        for _ in common..a.len() as i64 {
            up.push("..".to_string());
        }
        [&(&up)[..], &(&js_slice_vec(&b, common, None))[..]].concat().join("/")
    }

    pub fn parse(p: &str) -> ParsedPath {
        let base = basename(p, None);
        let ext = extname(p);
        let dir = if p.contains("/") { dirname(p) } else { "".to_string() };
        ParsedPath { root: if p.starts_with("/") { "/".to_string() } else { "".to_string() }, dir, base: base.clone(), ext: ext.clone(), name: if &ext == "" { base } else { js_slice(&base, 0, Some((base.chars().count() - ext.chars().count()) as i64)) } }
    }

    pub fn format(p: &ParsedPath) -> String {
        let base = if &p.base != "" { p.base.clone() } else { format!("{}{}", p.name, p.ext) };
        if &p.dir == "" {
            return format!("{}{base}", p.root);
        }
        if &p.dir == &p.root { format!("{}{base}", p.dir) } else { format!("{}/{base}", p.dir) }
    }
}

pub use node_path::*;

mod promises {
    use super::*;

    /// Node's `fs/promises` module.
    pub fn read_file(path: String, encoding: Option<String>) -> Promise<String> {
        Promise::spawn_result(async move {
            if !(js_node_fs_exists(&path)) {
                return Err(format!("ENOENT: no such file or directory, open '{path}'"));
            }
            Ok(js_node_fs_read(&path))
        })
    }

    pub fn write_file(path: String, data: String) -> Promise<()> {
        Promise::spawn(async move {
            js_node_fs_write(&path, &data);
        })
    }

    pub fn append_file(path: String, data: String) -> Promise<()> {
        Promise::spawn(async move {
            js_node_fs_append(&path, &data);
        })
    }

    pub fn readdir(path: String) -> Promise<Vec<String>> {
        Promise::spawn_result(async move {
            if !(js_node_fs_is_dir(&path)) {
                return Err(format!("ENOENT: no such file or directory, scandir '{path}'"));
            }
            Ok(js_node_fs_readdir(&path))
        })
    }

    pub fn mkdir(path: String) -> Promise<()> {
        Promise::spawn(async move {
            js_node_fs_mkdir(&path, true);
        })
    }

    pub fn rm(path: String) -> Promise<()> {
        Promise::spawn(async move {
            if js_node_fs_exists(&path) {
                js_node_fs_remove(&path, true);
            }
        })
    }
}

pub use promises::*;

mod index {
    use super::*;

    pub const PHI: f64 = 1.618;

    pub fn clamp(x: i64, lo: i64, hi: i64) -> i64 {
        x.max(lo).min(hi)
    }
}

pub use index::*;

mod node_process {
    use super::*;

    pub fn cwd() -> String {
        js_node_os_info("cwd")
    }

    pub fn exit(code: Option<i64>) {
        std::process::exit(code.unwrap_or(0) as i32);
    }
}

pub use node_process::*;

fn main_() -> Promise<()> {
    Promise::spawn_result(async move {
        let file = join(&[tmpdir(), "solidify-packages.txt".to_string()]);
        write_file(file.clone(), "data".to_string()).settled().await?;
        println!("{}", read_file(file.clone(), Some("utf8".to_string())).settled().await?);
        rm(file).settled().await?;
        Ok(())
    })
}
fn main() {
    // Node's `os` module.
    // Node's `path` module (POSIX), written in the subset Solidify translates.
    // Node's `process` module: the global `process`, its parts as names.
    let argv = std::env::args().collect::<Vec<String>>();
    let env = { let mut m = HashMap::new(); for (k, v) in std::env::vars() { m.insert(k, v); } m };
    let platform_node_process = js_node_os_info("platform");
    println!("{} {}", greet("ada"), shout("hi"));
    let mut c = Counter::new();
    c.inc();
    println!("{} {} {} {}", c.inc(), join(&["x".to_string(), "y".to_string()]), clamp(12, 0, 10), js_number_text(PHI));
    println!("{} {}", argv.len() > 0, &platform_node_process == &js_node_os_info("platform"));
    main_();
    js_run_timers();
}

/// `s.slice(start, end)`: character positions, negative ones counted from the end.
fn js_slice(s: &str, start: i64, end: Option<i64>) -> String {
    let len = s.chars().count() as i64;
    let clamp = |i: i64| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (a, b) = (clamp(start), clamp(end.unwrap_or(len)));
    if b <= a {
        return String::new();
    }
    s.chars().skip(a as usize).take((b - a) as usize).collect()
}

/// `s.charAt(i)` / `s[i]`: one character, or an empty string.
fn js_char_at(s: &str, i: i64) -> String {
    if i < 0 {
        return String::new();
    }
    s.chars().nth(i as usize).map(String::from).unwrap_or_default()
}

/// `s.lastIndexOf(needle)`: the last character position, or -1.
fn js_last_index_of(s: &str, needle: &str) -> i64 {
    match s.rfind(needle) {
        Some(byte) => s[..byte].chars().count() as i64,
        None => -1,
    }
}

/// `s.lastIndexOf(needle, from)`: the last match starting at `from` or before.
#[allow(dead_code)]
fn js_last_index_of_from(s: &str, needle: &str, from: i64) -> i64 {
    let chars: Vec<char> = s.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    if n.len() > chars.len() {
        return -1;
    }
    let last = (chars.len() - n.len()) as i64;
    let mut i = from.clamp(0, last.max(0)).min(last);
    while i >= 0 {
        if chars[i as usize..].starts_with(&n) {
            return i;
        }
        i -= 1;
    }
    -1
}

/// `list.slice(start, end)`: negative positions are counted from the end.
fn js_slice_vec<T: Clone>(v: &[T], start: i64, end: Option<i64>) -> Vec<T> {
    let len = v.len() as i64;
    let clamp = |i: i64| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (a, b) = (clamp(start), clamp(end.unwrap_or(len)));
    if b <= a {
        return Vec::new();
    }
    v[a as usize..b as usize].to_vec()
}

/// A number the way JavaScript prints it.
#[allow(dead_code)]
fn js_number_text(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.fract() == 0.0 && x.abs() < 1e15 {
        return format!("{}", x as i64);
    }
    // The shortest digits that read back as `x`, then JavaScript's placement of the point.
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let n: i64 = exp.parse::<i64>().unwrap_or(0) + 1;
    let k = digits.len() as i64;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let sign = if e < 0 { "-" } else { "+" };
        if k == 1 {
            format!("{digits}e{sign}{}", e.abs())
        } else {
            format!("{}.{}e{sign}{}", &digits[..1], &digits[1..], e.abs())
        }
    };
    if x < 0.0 { format!("-{body}") } else { body }
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


/// The native side of Node's `fs` and `os` modules.
#[allow(dead_code)]
fn js_fs_fail(op: &str, path: &str, e: std::io::Error) -> ! {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT: no such file or directory",
        std::io::ErrorKind::PermissionDenied => "EACCES: permission denied",
        std::io::ErrorKind::AlreadyExists => "EEXIST: file already exists",
        _ => "EIO: i/o error",
    };
    panic!("Error: {code}, {op} '{path}'")
}

#[allow(dead_code)]
fn js_node_fs_read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_write(path: &str, data: &str) {
    std::fs::write(path, data).unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_append(path: &str, data: &str) {
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| f.write_all(data.as_bytes()))
        .unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

#[allow(dead_code)]
fn js_node_fs_is_dir(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

#[allow(dead_code)]
fn js_node_fs_size(path: &str) -> i64 {
    std::fs::metadata(path).map_or(0, |m| m.len() as i64)
}

/// The names in a directory, sorted like Node gives them.
#[allow(dead_code)]
fn js_node_fs_readdir(path: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(path)
        .unwrap_or_else(|e| js_fs_fail("scandir", path, e))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[allow(dead_code)]
fn js_node_fs_mkdir(path: &str, recursive: bool) {
    let r = if recursive {
        std::fs::create_dir_all(path)
    } else {
        std::fs::create_dir(path)
    };
    r.unwrap_or_else(|e| js_fs_fail("mkdir", path, e))
}

#[allow(dead_code)]
fn js_node_fs_remove(path: &str, recursive: bool) {
    let p = std::path::Path::new(path);
    let r = if p.is_dir() {
        if recursive {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_dir(p)
        }
    } else {
        std::fs::remove_file(p)
    };
    r.unwrap_or_else(|e| js_fs_fail("rm", path, e))
}

#[allow(dead_code)]
fn js_node_fs_rename(from: &str, to: &str) {
    std::fs::rename(from, to).unwrap_or_else(|e| js_fs_fail("rename", from, e))
}

#[allow(dead_code)]
fn js_node_fs_copy(from: &str, to: &str) {
    std::fs::copy(from, to)
        .map(|_| ())
        .unwrap_or_else(|e| js_fs_fail("copyfile", from, e))
}

/// `os.platform()`, `os.homedir()`, `process.cwd()`…
#[allow(dead_code)]
fn js_node_os_info(key: &str) -> String {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    match key {
        "platform" => match std::env::consts::OS {
            "macos" => "darwin".to_string(),
            "windows" => "win32".to_string(),
            other => other.to_string(),
        },
        "arch" => match std::env::consts::ARCH {
            "x86_64" => "x64".to_string(),
            "aarch64" => "arm64".to_string(),
            "x86" => "ia32".to_string(),
            other => other.to_string(),
        },
        "type" => match std::env::consts::OS {
            "linux" => "Linux".to_string(),
            "macos" => "Darwin".to_string(),
            "windows" => "Windows_NT".to_string(),
            other => other.to_string(),
        },
        "homedir" => env("HOME"),
        "tmpdir" => std::env::temp_dir()
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string(),
        "hostname" => std::fs::read_to_string("/etc/hostname")
            .map(|h| h.trim().to_string())
            .unwrap_or_else(|_| env("HOSTNAME")),
        "cwd" => std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

#[allow(dead_code)]
fn js_node_os_cpus() -> i64 {
    std::thread::available_parallelism().map_or(1, |n| n.get() as i64)
}

