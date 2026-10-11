#[derive(Debug, Clone, PartialEq)]
struct Account {
    pub owner: String,
    pub balance: i64,
}

impl Account {
    pub fn new(owner: String) -> Self {
        Self { owner, balance: 0 }
    }
}

impl Shared<Account> {
    pub fn deposit(&self, n: i64) -> Promise<i64> {
        let this = self.clone();
        Promise::spawn(async move {
            sleep(2).await;
            this.borrow_mut().balance += n;
            this.borrow().balance
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PromiseSettledResult<T> {
    pub status: String,
    pub value: Option<T>,
    pub reason: Option<String>,
}

fn sleep(ms: i64) -> Promise<()> {
    Promise::new(move |resolve, _| { js_set_timeout(move || resolve(()), ms as f64); })
}

fn risky(n: i64) -> Promise<i64> {
    Promise::spawn_result(async move {
        sleep(1).settled().await?;
        if n > 2 {
            return Err(format!("too big: {n}"));
        }
        Ok(n * 10)
    })
}

fn main_() -> Promise<()> {
    Promise::spawn_result(async move {
        let mut acc = Shared::new(Account::new("ann".to_string()));
        acc.deposit(5).settled().await?;
        let total = acc.deposit(7).settled().await?;
        println!("{} {total} {}", acc.borrow().owner.clone(), acc.borrow().balance);
        let result_: Result<(), String> = async {
            risky(5).settled().await?;
            Ok(())
        }.await;
        if let Err(e) = result_ {
            println!("caught: {e}");
        }
        let p: Promise<i64> = risky(1);
        println!("started");
        println!("{} {}", p.settled().await?, p.settled().await?);
        let doubled = Promise::all(vec![1, 2, 3].into_iter().map(move |x| { Promise::spawn(async move {
            sleep(3 - x).await;
            println!("done {x}");
            x * 2
        }) }).collect::<Vec<_>>()).settled().await?;
        println!("{}", js_inspect(&doubled.js_value()));
        let results = Promise::all_settled(vec![risky(1), risky(9)], |r| match r { Ok(v) => PromiseSettledResult { status: "fulfilled".to_string(), value: Some(v), reason: None }, Err(e) => PromiseSettledResult { status: "rejected".to_string(), value: None, reason: Some(e) } }).settled().await?;
        for r in &results {
            println!("{} {} {}", r.status, r.value.as_ref().map_or("undefined".to_string(), |v| v.to_string()), r.reason.as_ref().map(|v| v.clone()).as_ref().map_or("undefined".to_string(), |v| v.to_string()));
        }
        risky(1).then_promise(move |v| risky(v)).catch_none(move |e| println!("chain: {e}")).finally(move || println!("finally"));
        Ok(())
    })
}

fn main() {
    Promise::resolved(1).then(move |v| println!("micro {v}"));
    println!("sync first");
    main_();
    println!("after main call");
    js_run_timers();
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


/// A value as JavaScript sees it, for `console.log`, `String(…)` and `JSON.stringify`.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
enum JsValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Undefined,
    Array(Vec<JsValue>),
    /// A plain object, or an instance of the named class.
    Object(Option<&'static str>, Vec<(String, JsValue)>),
    Map(Vec<(JsValue, JsValue)>),
    Set(Vec<JsValue>),
    /// A function, by name.
    Func(&'static str),
    /// Text shown as it is (a regular expression).
    Raw(String),
    /// A date: its ISO text, and its `toString()` text.
    Date(String, String),
    /// An object held in several places: changed through one, changed for all.
    Ref(std::rc::Rc<std::cell::RefCell<JsValue>>),
    /// `new Proxy(target, handler)`.
    Proxy(std::rc::Rc<JsProxy>),
}

/// The traps of a proxy, and the object it stands for.
#[allow(dead_code, clippy::type_complexity)]
struct JsProxy {
    target: JsValue,
    get: Option<Box<dyn Fn(JsValue, String, JsValue) -> JsValue>>,
    set: Option<Box<dyn Fn(JsValue, String, JsValue, JsValue) -> bool>>,
    has: Option<Box<dyn Fn(JsValue, String) -> bool>>,
    delete: Option<Box<dyn Fn(JsValue, String) -> bool>>,
}

impl std::fmt::Debug for JsProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Proxy({:?})", self.target)
    }
}

impl PartialEq for JsProxy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[allow(dead_code)]
impl JsValue {
    /// What a shared object or a proxy holds, to be shown.
    fn shown(&self) -> Option<JsValue> {
        match self {
            JsValue::Ref(cell) => Some(cell.borrow().clone()),
            JsValue::Proxy(p) => p.target.shown().or_else(|| Some(p.target.clone())),
            _ => None,
        }
    }
}

/// Implemented by every type that `console.log` can show.
#[allow(dead_code)]
trait JsInspect {
    fn js_value(&self) -> JsValue;
}

impl JsInspect for i64 {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self)
    }
}
impl JsInspect for f64 {
    fn js_value(&self) -> JsValue {
        JsValue::Float(*self)
    }
}
impl JsInspect for usize {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self as i64)
    }
}
impl JsInspect for i32 {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self as i64)
    }
}
impl JsInspect for u32 {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self as i64)
    }
}
impl JsInspect for u64 {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self as i64)
    }
}
impl JsInspect for f32 {
    fn js_value(&self) -> JsValue {
        JsValue::Float(*self as f64)
    }
}
impl JsInspect for bool {
    fn js_value(&self) -> JsValue {
        JsValue::Bool(*self)
    }
}
impl JsInspect for str {
    fn js_value(&self) -> JsValue {
        JsValue::Str(self.to_string())
    }
}
impl JsInspect for String {
    fn js_value(&self) -> JsValue {
        JsValue::Str(self.clone())
    }
}
impl JsInspect for char {
    fn js_value(&self) -> JsValue {
        JsValue::Str(self.to_string())
    }
}
impl JsInspect for () {
    fn js_value(&self) -> JsValue {
        JsValue::Undefined
    }
}
impl<T: JsInspect + ?Sized> JsInspect for &T {
    fn js_value(&self) -> JsValue {
        (**self).js_value()
    }
}
impl<T: JsInspect + ?Sized> JsInspect for Box<T> {
    fn js_value(&self) -> JsValue {
        (**self).js_value()
    }
}
impl<T: JsInspect> JsInspect for Option<T> {
    fn js_value(&self) -> JsValue {
        match self {
            Some(v) => v.js_value(),
            None => JsValue::Null,
        }
    }
}
impl<T: JsInspect> JsInspect for std::rc::Rc<std::cell::RefCell<T>> {
    fn js_value(&self) -> JsValue {
        self.borrow().js_value()
    }
}
impl<T: JsInspect> JsInspect for Vec<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Array(self.iter().map(JsInspect::js_value).collect())
    }
}
impl<T: JsInspect> JsInspect for [T] {
    fn js_value(&self) -> JsValue {
        JsValue::Array(self.iter().map(JsInspect::js_value).collect())
    }
}
impl<A: JsInspect, B: JsInspect> JsInspect for (A, B) {
    fn js_value(&self) -> JsValue {
        JsValue::Array(vec![self.0.js_value(), self.1.js_value()])
    }
}
impl<A: JsInspect, B: JsInspect, C: JsInspect> JsInspect for (A, B, C) {
    fn js_value(&self) -> JsValue {
        JsValue::Array(vec![self.0.js_value(), self.1.js_value(), self.2.js_value()])
    }
}
impl<K: JsInspect, V: JsInspect> JsInspect for std::collections::HashMap<K, V> {
    fn js_value(&self) -> JsValue {
        JsValue::Map(self.iter().map(|(k, v)| (k.js_value(), v.js_value())).collect())
    }
}
impl<T: JsInspect> JsInspect for std::collections::HashSet<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Set(self.iter().map(JsInspect::js_value).collect())
    }
}

/// `String(value)`: what a value becomes in a string.
#[allow(dead_code)]
fn js_to_string(v: &JsValue) -> String {
    if let Some(inner) = v.shown() {
        return js_to_string(&inner);
    }
    match v {
        JsValue::Int(n) => n.to_string(),
        JsValue::Float(x) => js_number_text(*x),
        JsValue::Str(s) => s.clone(),
        JsValue::Bool(b) => b.to_string(),
        JsValue::Null => "null".into(),
        JsValue::Undefined => "undefined".into(),
        JsValue::Array(items) => items
            .iter()
            .map(|i| match i {
                JsValue::Null | JsValue::Undefined => String::new(),
                other => js_to_string(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        JsValue::Object(..) => "[object Object]".into(),
        JsValue::Map(_) => "[object Map]".into(),
        JsValue::Set(_) => "[object Set]".into(),
        JsValue::Func(name) => format!("function {name}() {{ [native code] }}"),
        JsValue::Raw(text) => text.clone(),
        JsValue::Date(_, text) => text.clone(),
        JsValue::Ref(_) | JsValue::Proxy(_) => unreachable!("shown above"),
    }
}

/// One argument of `console.log`: a string as it is, anything else inspected.
#[allow(dead_code)]
fn js_show(v: &JsValue) -> String {
    match v {
        JsValue::Str(s) => s.clone(),
        other => js_inspect(other),
    }
}

/// A string literal as Node's `util.inspect` quotes it.
#[allow(dead_code)]
fn js_quote(s: &str) -> String {
    let quote = if !s.contains('\'') {
        '\''
    } else if !s.contains('"') {
        '"'
    } else if !s.contains('`') && !s.contains("${") {
        '`'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c == '\u{7f}' => out.push_str(&format!("\\x{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

#[allow(dead_code)]
fn js_key(key: &str) -> String {
    let plain = key
        .chars()
        .enumerate()
        .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()));
    if plain && !key.is_empty() { key.to_string() } else { js_quote(key) }
}

#[allow(dead_code)]
struct JsInspectCtx {
    indentation: usize,
    current_depth: usize,
}

/// `util.inspect(value)`: Node's layout, line breaks and grouping included.
#[allow(dead_code)]
fn js_inspect(v: &JsValue) -> String {
    let mut ctx = JsInspectCtx { indentation: 0, current_depth: 0 };
    js_format(v, 0, &mut ctx)
}

#[allow(dead_code)]
fn js_format(v: &JsValue, recurse_times: usize, ctx: &mut JsInspectCtx) -> String {
    if let Some(inner) = v.shown() {
        return js_format(&inner, recurse_times, ctx);
    }
    let (open, close) = match v {
        JsValue::Int(n) => return n.to_string(),
        JsValue::Float(x) => {
            return if *x == 0.0 && x.is_sign_negative() { "-0".into() } else { js_number_text(*x) };
        }
        JsValue::Str(s) => return js_quote(s),
        JsValue::Bool(b) => return b.to_string(),
        JsValue::Null => return "null".into(),
        JsValue::Undefined => return "undefined".into(),
        JsValue::Func(name) => {
            return if name.is_empty() {
                "[Function (anonymous)]".into()
            } else {
                format!("[Function: {name}]")
            };
        }
        JsValue::Raw(text) => return text.clone(),
        JsValue::Date(iso, _) => return iso.clone(),
        JsValue::Ref(_) | JsValue::Proxy(_) => unreachable!("shown above"),
        JsValue::Array(items) => {
            if items.is_empty() {
                return "[]".into();
            }
            if recurse_times > 2 {
                return "[Array]".into();
            }
            ("[".to_string(), "]")
        }
        JsValue::Object(name, props) => {
            if props.is_empty() {
                return match name {
                    Some(n) => format!("{n} {{}}"),
                    None => "{}".into(),
                };
            }
            if recurse_times > 2 {
                return match name {
                    Some(n) => format!("[{n}]"),
                    None => "[Object]".into(),
                };
            }
            (name.map_or("{".to_string(), |n| format!("{n} {{")), "}")
        }
        JsValue::Map(entries) => {
            if entries.is_empty() {
                return "Map(0) {}".into();
            }
            if recurse_times > 2 {
                return "[Map]".into();
            }
            (format!("Map({}) {{", entries.len()), "}")
        }
        JsValue::Set(items) => {
            if items.is_empty() {
                return "Set(0) {}".into();
            }
            if recurse_times > 2 {
                return "[Set]".into();
            }
            (format!("Set({}) {{", items.len()), "}")
        }
    };
    let recurse_times = recurse_times + 1;
    ctx.current_depth = recurse_times;
    let mut output = Vec::new();
    ctx.indentation += 2;
    match v {
        JsValue::Array(items) => {
            for item in items.iter().take(100) {
                output.push(js_format(item, recurse_times, ctx));
            }
            if items.len() > 100 {
                let rest = items.len() - 100;
                output.push(format!("... {rest} more item{}", if rest > 1 { "s" } else { "" }));
            }
        }
        JsValue::Object(_, props) => {
            for (k, val) in props {
                let text = js_format(val, recurse_times, ctx);
                output.push(format!("{}: {text}", js_key(k)));
            }
        }
        JsValue::Map(entries) => {
            for (k, val) in entries {
                let key = js_format(k, recurse_times, ctx);
                let text = js_format(val, recurse_times, ctx);
                output.push(format!("{key} => {text}"));
            }
        }
        JsValue::Set(items) => {
            for item in items {
                output.push(js_format(item, recurse_times, ctx));
            }
        }
        _ => {}
    }
    ctx.indentation -= 2;
    let entries = output.len();
    if let JsValue::Array(items) = v
        && entries > 6
    {
        output = js_group_array(ctx, output, items);
    }
    if ctx.current_depth - recurse_times < 3 && entries == output.len() {
        // Node counts a `<ref *1>` marker apart from the braces, without the space.
        let marker = usize::from(open.starts_with("<ref *"));
        let start = output.len() + ctx.indentation + open.chars().count() - marker + 10;
        let mut total = output.len() + start;
        let fits = total + output.len() <= 80 && output.iter().all(|s| {
            total += s.chars().count();
            total <= 80
        });
        if fits {
            let joined = output.join(", ");
            if !joined.contains('\n') {
                return format!("{open} {joined} {close}");
            }
        }
    }
    let indentation = format!("\n{}", " ".repeat(ctx.indentation));
    format!(
        "{open}{indentation}  {}{indentation}{close}",
        output.join(&format!(",{indentation}  "))
    )
}

/// Node's column layout for arrays of more than six short entries.
#[allow(dead_code)]
fn js_group_array(ctx: &JsInspectCtx, output: Vec<String>, items: &[JsValue]) -> Vec<String> {
    let mut total_length = 0;
    let mut max_length = 0;
    let output_length = if items.len() > 100 { output.len() - 1 } else { output.len() };
    let separator_space = 2;
    let data_len: Vec<usize> = output.iter().map(|s| s.chars().count()).collect();
    for &len in data_len.iter().take(output_length) {
        total_length += len + separator_space;
        max_length = max_length.max(len);
    }
    let actual_max = max_length + separator_space;
    if actual_max * 3 + ctx.indentation < 80
        && (total_length as f64 / actual_max as f64 > 5.0 || max_length <= 6)
    {
        let approx_char_heights = 2.5;
        let average_bias = (actual_max as f64 - total_length as f64 / output.len() as f64).sqrt();
        let biased_max = (actual_max as f64 - 3.0 - average_bias).max(1.0);
        let columns = ((approx_char_heights * biased_max * output_length as f64).sqrt() / biased_max)
            .round()
            .min(((80 - ctx.indentation) / actual_max) as f64)
            .min(12.0)
            .min(15.0) as usize;
        if columns <= 1 {
            return output;
        }
        let mut max_line_length = Vec::new();
        for i in 0..columns {
            let mut line_length = 0;
            let mut j = i;
            while j < output_length {
                if data_len[j] > line_length {
                    line_length = data_len[j];
                }
                j += columns;
            }
            max_line_length.push(line_length + separator_space);
        }
        let pad_start = items.iter().all(|v| matches!(v, JsValue::Int(_) | JsValue::Float(_)));
        let mut tmp = Vec::new();
        let mut i = 0;
        while i < output_length {
            let max = (i + columns).min(output_length);
            let mut line = String::new();
            let mut j = i;
            while j < max - 1 {
                let cell = format!("{}, ", output[j]);
                let width = max_line_length[j - i];
                let padding = width.saturating_sub(cell.chars().count());
                if pad_start {
                    line.push_str(&" ".repeat(padding));
                    line.push_str(&cell);
                } else {
                    line.push_str(&cell);
                    line.push_str(&" ".repeat(padding));
                }
                j += 1;
            }
            if pad_start {
                let width = max_line_length[j - i] - separator_space;
                let padding = width.saturating_sub(data_len[j]);
                line.push_str(&" ".repeat(padding));
            }
            line.push_str(&output[j]);
            tmp.push(line);
            i += columns;
        }
        if items.len() > 100 {
            tmp.push(output[output_length].clone());
        }
        return tmp;
    }
    output
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


/// An object held in several places, like every JavaScript object: changed through one,
/// changed for all; equal only to itself.
struct Shared<T: ?Sized>(std::rc::Rc<std::cell::RefCell<T>>);

#[allow(dead_code)]
impl<T> Shared<T> {
    fn new(value: T) -> Self {
        Shared(std::rc::Rc::new(std::cell::RefCell::new(value)))
    }
}

#[allow(dead_code)]
impl<T: ?Sized> Shared<T> {
    fn borrow(&self) -> std::cell::Ref<'_, T> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> std::cell::RefMut<'_, T> {
        self.0.borrow_mut()
    }

    /// Reads the object for the time of `f` only.
    fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.0.borrow())
    }

    /// Changes the object for the time of `f` only.
    fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.0.borrow_mut())
    }
}

// The program runs one piece at a time (a generator's thread waits while the caller
// runs, and the other way round): its objects are never used by two threads at once.
unsafe impl<T: ?Sized> Send for Shared<T> {}
unsafe impl<T: ?Sized> Sync for Shared<T> {}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

impl<T: ?Sized> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> Eq for Shared<T> {}

impl<T: ?Sized> std::hash::Hash for Shared<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (std::rc::Rc::as_ptr(&self.0) as *const () as usize).hash(state);
    }
}

impl<T: std::fmt::Debug + ?Sized> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.try_borrow() {
            Ok(v) => v.fmt(f),
            Err(_) => f.write_str("[Circular]"),
        }
    }
}

impl<T: std::fmt::Display + ?Sized> std::fmt::Display for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.borrow().fmt(f)
    }
}

thread_local! {
    /// The objects being shown, and whether one of them was met again inside itself.
    static JS_SHOWING: std::cell::RefCell<Vec<(usize, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
}

impl<T: JsInspect + ?Sized> JsInspect for Shared<T> {
    fn js_value(&self) -> JsValue {
        let ptr = std::rc::Rc::as_ptr(&self.0) as *const () as usize;
        let cycle = JS_SHOWING.with(|s| {
            let mut s = s.borrow_mut();
            match s.iter_mut().find(|(p, _)| *p == ptr) {
                Some(entry) => {
                    entry.1 = true;
                    true
                }
                None => {
                    s.push((ptr, false));
                    false
                }
            }
        });
        if cycle {
            return JsValue::Raw("[Circular *1]".into());
        }
        let v = self.0.borrow().js_value();
        let referenced = JS_SHOWING.with(|s| s.borrow_mut().pop().is_some_and(|(_, r)| r));
        match v {
            // Node marks the object a cycle comes back to.
            JsValue::Object(name, props) if referenced => {
                let marked: &'static str = Box::leak(
                    match name {
                        Some(n) => format!("<ref *1> {n}"),
                        None => "<ref *1>".to_string(),
                    }
                    .into_boxed_str(),
                );
                JsValue::Object(Some(marked), props)
            }
            other => other,
        }
    }
}

