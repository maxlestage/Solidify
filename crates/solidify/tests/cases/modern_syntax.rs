use std::sync::LazyLock;

const WHO: &str = "Ann";
const K: &str = "id-7";
static COUNTER_COUNT: LazyLock<JsStatic<i64>> = LazyLock::new(|| JsStatic::new(0));
static COUNTER_NAMES: LazyLock<JsStatic<Vec<String>>> = LazyLock::new(|| JsStatic::new(Vec::new()));
const PRICES: &str = "USD 10, EUR 20, USD 35";

#[derive(Debug, Clone, PartialEq)]
enum StringOrNumber {
    String(String),
    Number(i64),
}

#[derive(Debug, Clone, PartialEq)]
struct Point {
    pub x: i64,
    pub y: i64,
}

impl Point {
    pub fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    pub fn norm(&self) -> i64 {
        self.x.abs() + self.y.abs()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Config {
}

impl Config {
    pub fn new() -> Self {
        Self {  }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Counter {
    pub id: i64,
}

impl Counter {
    pub const LIMIT: i64 = 10;

    pub fn new(name: String) -> Self {
        let mut this = Self { id: 0 };
        (*COUNTER_COUNT.write()) += 1;
        this.id = (*COUNTER_COUNT.read());
        (*COUNTER_NAMES.write()).push(name);
        this
    }

    pub fn reset() {
        let value_ = 0;
        (*COUNTER_COUNT.write()) = value_;
    }

    pub fn total() -> i64 {
        (*COUNTER_COUNT.read()) * Counter::LIMIT
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Special {
    pub id: i64,
}

impl Special {
    pub fn new(name: String) -> Self {
        let base = Counter::new(name);
        Self { id: base.id }
    }
}

fn html(strings: &[String], values: &[StringOrNumber]) -> String {
    let mut out = "".to_string();
    strings.iter().enumerate().for_each(|(i, s)| {
        out.push_str(s.as_str());
        if i < values.len() {
            out.push_str(&format!("[{}]", js_to_string(&values[i].js_value())));
        }
    });
    out.clone()
}

fn main_() -> Promise<()> {
    Promise::spawn(async move {
        let jobs = vec![Promise::resolved(1), Promise::resolved(2)];
        for pending104 in &jobs {
            let v = pending104.await;
            println!("got {v}");
        }
        for pending116 in vec![3, 4] {
            let n = { let v_ = pending116; Promise::resolved(()).await; v_ };
            println!("plain {n}");
        }
    })
}

fn main() {
    println!("{}", html(&["Hello ".to_string(), ", you are ".to_string(), "!".to_string()], &[StringOrNumber::String(WHO.to_string()), StringOrNumber::Number(42)]));
    println!("{}", Point::new(3, -4).norm());
    let mut config_values: Vec<String> = Vec::new();
    {
        config_values.push("a".to_string());
        config_values.push("b".to_string());
    }
    println!("{}", js_inspect(&config_values.js_value()));
    println!("{K}");
    main_();
    Counter::new("a".to_string());
    Special::new("b".to_string());
    println!("{} {} {} {}", (*COUNTER_COUNT.read()), js_inspect(&(*COUNTER_NAMES.read()).js_value()), (*COUNTER_COUNT.read()), Counter::total());
    Counter::reset();
    println!("{} {}", (*COUNTER_COUNT.read()), Counter::LIMIT);
    let xs = vec!["a".to_string(), "b".to_string()];
    let mut out = "".to_string();
    xs.iter().enumerate().for_each(|(i, s)| {
        out.push_str(s.as_str());
        if i < 1 {
            out.push_str("-");
        }
    });
    println!("{out}");
    println!("{}", JsRegex::new("(?<=USD )\\d+", "g").match_all(PRICES).as_ref().map_or("null".to_string(), |v| js_inspect(&v.js_value())));
    println!("{}", JsRegex::new("(?<![ab])\\d", "g").replace("a1b2c3", "#"));
    println!("{}", JsRegex::new("(?<=\\$)\\d+(\\.\\d+)?", "").exec("cost: $42.50").as_ref().map(|v| v.get(0)).as_ref().map_or("undefined".to_string(), |v| v.to_string()));
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


/// A regular expression (`/pattern/flags`): parsed once, matched by backtracking on the
/// characters of the text, with JavaScript's syntax and semantics.
#[derive(Debug)]
#[allow(dead_code)]
pub struct JsRegex {
    root: Vec<Vec<RxNode>>,
    groups: usize,
    names: Vec<(String, usize)>,
    ignore_case: bool,
    multiline: bool,
    dot_all: bool,
    global: bool,
    sticky: bool,
    source: String,
    flags: String,
    /// `re.lastIndex`: where the next `exec` or `test` starts, with the `g` or `y` flag.
    last_index: std::sync::atomic::AtomicUsize,
}

impl Clone for JsRegex {
    fn clone(&self) -> JsRegex {
        JsRegex {
            root: self.root.clone(),
            groups: self.groups,
            names: self.names.clone(),
            ignore_case: self.ignore_case,
            multiline: self.multiline,
            dot_all: self.dot_all,
            global: self.global,
            sticky: self.sticky,
            source: self.source.clone(),
            flags: self.flags.clone(),
            last_index: std::sync::atomic::AtomicUsize::new(self.last_index() as usize),
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum RxNode {
    Char(char),
    Any,
    /// Ranges of characters, and whether the class is negated.
    Class(Vec<(char, char)>, bool),
    Start,
    End,
    /// `\b` (true) or `\B` (false).
    WordBoundary(bool),
    /// Alternatives, and the capture index.
    Group(Vec<Vec<RxNode>>, Option<usize>),
    /// Lookahead: alternatives, and whether it is negative.
    Look(Vec<Vec<RxNode>>, bool),
    /// Lookbehind: alternatives ending where the match is, and whether it is negative.
    Behind(Vec<Vec<RxNode>>, bool),
    Repeat(Box<RxNode>, usize, Option<usize>, bool),
    BackRef(usize),
    /// `\p{…}` (negated: `\P{…}`): a Unicode property, by name.
    Prop(String, bool),
}

/// A match: the character index, the text, the full match and the groups (`None` for a
/// group that took no part).
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct JsMatch {
    pub index: i64,
    pub input: String,
    pub groups: Vec<Option<String>>,
}

impl JsMatch {
    /// The character position just after the match.
    pub fn end(&self) -> usize {
        self.index as usize + self.groups[0].as_ref().map_or(0, |g| g.chars().count())
    }
}

#[allow(dead_code)]
impl JsMatch {
    /// `m[i]`: a group, empty when it took no part.
    pub fn get(&self, i: i64) -> String {
        self.groups
            .get(i as usize)
            .cloned()
            .flatten()
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

type RxCaps = Vec<Option<(usize, usize)>>;

#[allow(dead_code)]
impl JsRegex {
    /// Parses `source` with `flags`; a bad pattern is a panic (the translator checks it).
    pub fn new(source: &str, flags: &str) -> JsRegex {
        match JsRegex::parse(source, flags) {
            Ok(re) => re,
            Err(e) => panic!("invalid regular expression /{source}/{flags}: {e}"),
        }
    }

    pub fn parse(source: &str, flags: &str) -> Result<JsRegex, String> {
        let chars: Vec<char> = source.chars().collect();
        let mut p = RxParser {
            chars: &chars,
            pos: 0,
            groups: 0,
            names: Vec::new(),
        };
        let root = p.alternatives()?;
        if p.pos < chars.len() {
            return Err(format!("unexpected `{}`", chars[p.pos]));
        }
        for f in flags.chars() {
            if !"gimsuyd".contains(f) {
                return Err(format!("unknown flag `{f}`"));
            }
        }
        Ok(JsRegex {
            root,
            groups: p.groups,
            names: p.names,
            ignore_case: flags.contains('i'),
            multiline: flags.contains('m'),
            dot_all: flags.contains('s'),
            global: flags.contains('g'),
            sticky: flags.contains('y'),
            source: source.to_string(),
            flags: flags.to_string(),
            last_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    fn same(&self, a: char, b: char) -> bool {
        a == b || (self.ignore_case && a.to_lowercase().eq(b.to_lowercase()))
    }

    fn in_class(&self, c: char, ranges: &[(char, char)], negated: bool) -> bool {
        let mut hit = ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi);
        if !hit && self.ignore_case {
            let lower = c.to_lowercase().next().unwrap_or(c);
            let upper = c.to_uppercase().next().unwrap_or(c);
            hit = ranges
                .iter()
                .any(|&(lo, hi)| (lo <= lower && lower <= hi) || (lo <= upper && upper <= hi));
        }
        hit != negated
    }

    fn is_word(c: char) -> bool {
        c == '_' || c.is_ascii_alphanumeric()
    }

    /// The Unicode properties understood: letters, numbers, cases, spaces, punctuation.
    fn has_property(c: char, name: &str) -> bool {
        match name {
            "L" | "Letter" | "Alphabetic" | "Alpha" => c.is_alphabetic(),
            "Lu" | "Uppercase_Letter" | "Uppercase" | "Upper" => c.is_uppercase(),
            "Ll" | "Lowercase_Letter" | "Lowercase" | "Lower" => c.is_lowercase(),
            "N" | "Number" | "Nd" | "Decimal_Number" | "digit" => c.is_numeric(),
            "White_Space" | "space" | "Zs" => c.is_whitespace(),
            "P" | "Punctuation" | "punct" => {
                c.is_ascii_punctuation()
                    || (!c.is_alphanumeric()
                        && !c.is_whitespace()
                        && !c.is_control()
                        && (c as u32) > 127
                        && !c.is_ascii())
            }
            "S" | "Symbol" => "$+<=>^`|~".contains(c),
            "Cc" | "Control" => c.is_control(),
            _ => false,
        }
    }

    /// Matches `seq` at `pos`, then calls `k` with where it ended.
    fn seq(
        &self,
        text: &[char],
        seq: &[RxNode],
        pos: usize,
        caps: &mut RxCaps,
        k: &mut dyn FnMut(usize, &mut RxCaps) -> bool,
    ) -> bool {
        let Some((node, rest)) = seq.split_first() else {
            return k(pos, caps);
        };
        match node {
            RxNode::Char(c) => {
                pos < text.len()
                    && self.same(text[pos], *c)
                    && self.seq(text, rest, pos + 1, caps, k)
            }
            RxNode::Any => {
                pos < text.len()
                    && (self.dot_all || text[pos] != '\n')
                    && self.seq(text, rest, pos + 1, caps, k)
            }
            RxNode::Class(ranges, negated) => {
                pos < text.len()
                    && self.in_class(text[pos], ranges, *negated)
                    && self.seq(text, rest, pos + 1, caps, k)
            }
            RxNode::Prop(name, negated) => {
                pos < text.len()
                    && (Self::has_property(text[pos], name) != *negated)
                    && self.seq(text, rest, pos + 1, caps, k)
            }
            RxNode::Start => {
                (pos == 0 || (self.multiline && text[pos - 1] == '\n'))
                    && self.seq(text, rest, pos, caps, k)
            }
            RxNode::End => {
                (pos == text.len() || (self.multiline && text[pos] == '\n'))
                    && self.seq(text, rest, pos, caps, k)
            }
            RxNode::WordBoundary(expected) => {
                let before = pos > 0 && Self::is_word(text[pos - 1]);
                let after = pos < text.len() && Self::is_word(text[pos]);
                ((before != after) == *expected) && self.seq(text, rest, pos, caps, k)
            }
            RxNode::Group(alts, cap) => {
                for alt in alts {
                    let saved = caps.clone();
                    let matched = self.seq(text, alt, pos, caps, &mut |end, caps| {
                        if let Some(i) = cap {
                            caps[*i] = Some((pos, end));
                        }
                        self.seq(text, rest, end, caps, k)
                    });
                    if matched {
                        return true;
                    }
                    *caps = saved;
                }
                false
            }
            RxNode::Look(alts, negative) => {
                let saved = caps.clone();
                let found = alts
                    .iter()
                    .any(|alt| self.seq(text, alt, pos, caps, &mut |_, _| true));
                if found == *negative {
                    *caps = saved;
                    return false;
                }
                if *negative {
                    *caps = saved;
                }
                self.seq(text, rest, pos, caps, k)
            }
            RxNode::Behind(alts, negative) => {
                let saved = caps.clone();
                // Some start before `pos` from which an alternative ends exactly at `pos`.
                let found = (0..=pos).rev().any(|start| {
                    alts.iter()
                        .any(|alt| self.seq(text, alt, start, caps, &mut |end, _| end == pos))
                });
                if found == *negative {
                    *caps = saved;
                    return false;
                }
                if *negative {
                    *caps = saved;
                }
                self.seq(text, rest, pos, caps, k)
            }
            RxNode::Repeat(inner, min, max, greedy) => {
                self.repeat(text, inner, *min, *max, *greedy, 0, pos, caps, rest, k)
            }
            RxNode::BackRef(i) => {
                let Some(Some((a, b))) = caps.get(*i).copied() else {
                    return self.seq(text, rest, pos, caps, k);
                };
                let len = b - a;
                if pos + len > text.len() {
                    return false;
                }
                for j in 0..len {
                    if !self.same(text[pos + j], text[a + j]) {
                        return false;
                    }
                }
                self.seq(text, rest, pos + len, caps, k)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat(
        &self,
        text: &[char],
        inner: &RxNode,
        min: usize,
        max: Option<usize>,
        greedy: bool,
        count: usize,
        pos: usize,
        caps: &mut RxCaps,
        rest: &[RxNode],
        k: &mut dyn FnMut(usize, &mut RxCaps) -> bool,
    ) -> bool {
        let can_more = max.is_none_or(|m| count < m);
        let more = |this: &Self,
                    caps: &mut RxCaps,
                    k: &mut dyn FnMut(usize, &mut RxCaps) -> bool|
         -> bool {
            can_more
                && this.seq(
                    text,
                    std::slice::from_ref(inner),
                    pos,
                    caps,
                    &mut |end, caps| {
                        // An empty iteration would loop forever.
                        if end == pos && count >= min {
                            return false;
                        }
                        this.repeat(text, inner, min, max, greedy, count + 1, end, caps, rest, k)
                    },
                )
        };
        if greedy {
            if more(self, caps, k) {
                return true;
            }
            count >= min && self.seq(text, rest, pos, caps, k)
        } else {
            if count >= min && self.seq(text, rest, pos, caps, k) {
                return true;
            }
            more(self, caps, k)
        }
    }

    /// The first match at or after character `from`.
    fn find_from(&self, text: &[char], input: &str, from: usize) -> Option<JsMatch> {
        (from..=text.len()).find_map(|start| self.try_at(text, input, start))
    }

    /// The match starting exactly at `start`.
    fn try_at(&self, text: &[char], input: &str, start: usize) -> Option<JsMatch> {
        let mut caps: RxCaps = vec![None; self.groups + 1];
        let mut found: Option<usize> = None;
        let matched = self.seq(
            text,
            &[RxNode::Group(self.root.clone(), Some(0))],
            start,
            &mut caps,
            &mut |end, _| {
                found = Some(end);
                true
            },
        );
        if !matched {
            return None;
        }
        let groups = caps
            .iter()
            .map(|c| c.map(|(a, b)| text[a..b].iter().collect::<String>()))
            .collect();
        let _ = found;
        Some(JsMatch {
            index: start as i64,
            input: input.to_string(),
            groups,
        })
    }

    fn chars(input: &str) -> Vec<char> {
        input.chars().collect()
    }

    /// `re.test(s)`: like `exec`, `lastIndex` included.
    pub fn test(&self, input: &str) -> bool {
        self.exec(input).is_some()
    }

    /// `re.exec(s)`: the first match, or with the `g` or `y` flag the next one from
    /// `lastIndex`, which then moves past it (back to 0 when nothing matches).
    pub fn exec(&self, input: &str) -> Option<JsMatch> {
        use std::sync::atomic::Ordering;
        let text = Self::chars(input);
        if !(self.global || self.sticky) {
            return self.find_from(&text, input, 0);
        }
        let from = self.last_index.load(Ordering::Relaxed);
        let found = if from > text.len() {
            None
        } else if self.sticky {
            self.try_at(&text, input, from)
        } else {
            self.find_from(&text, input, from)
        };
        let next = found.as_ref().map_or(0, JsMatch::end);
        self.last_index.store(next, Ordering::Relaxed);
        found
    }

    /// `re.lastIndex`.
    pub fn last_index(&self) -> i64 {
        self.last_index.load(std::sync::atomic::Ordering::Relaxed) as i64
    }

    /// `re.lastIndex = i`.
    pub fn set_last_index(&self, i: i64) {
        self.last_index
            .store(i.max(0) as usize, std::sync::atomic::Ordering::Relaxed);
    }

    /// Every match, as `s.matchAll(re)` gives them.
    pub fn find_all(&self, input: &str) -> Vec<JsMatch> {
        let text = Self::chars(input);
        let mut out = Vec::new();
        let mut from = 0;
        while from <= text.len() {
            let Some(m) = self.find_from(&text, input, from) else {
                break;
            };
            let len = m.groups[0].as_ref().map_or(0, |g| g.chars().count());
            from = m.index as usize + len.max(1);
            out.push(m);
        }
        out
    }

    /// `s.match(re)` with the `g` flag: the full matches.
    pub fn match_all(&self, input: &str) -> Option<Vec<String>> {
        let all: Vec<String> = self.find_all(input).into_iter().map(|m| m.get(0)).collect();
        if all.is_empty() { None } else { Some(all) }
    }

    /// `s.search(re)`: the index of the first match, or -1.
    pub fn search(&self, input: &str) -> i64 {
        self.exec(input).map_or(-1, |m| m.index)
    }

    /// `s.replace(re, text)`: `$1`, `$&`, `$$` and `$<name>` are expanded; every match when
    /// the flag `g` is set.
    pub fn replace(&self, input: &str, replacement: &str) -> String {
        self.replace_with(input, |m| self.expand(replacement, m))
    }

    /// `s.replace(re, (match, ...groups) => text)`.
    pub fn replace_with(&self, input: &str, mut f: impl FnMut(&JsMatch) -> String) -> String {
        let text = Self::chars(input);
        let matches = if self.global {
            self.find_all(input)
        } else {
            self.exec(input).into_iter().collect()
        };
        let mut out = String::new();
        let mut last = 0;
        for m in &matches {
            let start = m.index as usize;
            let end = start + m.groups[0].as_ref().map_or(0, |g| g.chars().count());
            out.extend(text[last..start].iter());
            out.push_str(&f(m));
            last = end;
        }
        out.extend(text[last..].iter());
        out
    }

    fn expand(&self, replacement: &str, m: &JsMatch) -> String {
        let mut out = String::new();
        let mut chars = replacement.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            match chars.peek().copied() {
                Some('$') => {
                    chars.next();
                    out.push('$');
                }
                Some('&') => {
                    chars.next();
                    out.push_str(&m.get(0));
                }
                Some(d) if d.is_ascii_digit() => {
                    chars.next();
                    let mut n = d.to_digit(10).unwrap_or(0) as usize;
                    if let Some(d2) = chars.peek().copied().filter(|c| c.is_ascii_digit())
                        && n * 10 + d2.to_digit(10).unwrap_or(0) as usize <= self.groups
                    {
                        chars.next();
                        n = n * 10 + d2.to_digit(10).unwrap_or(0) as usize;
                    }
                    if n >= 1 && n <= self.groups {
                        out.push_str(&m.get(n as i64));
                    } else {
                        out.push('$');
                        out.push_str(&n.to_string());
                    }
                }
                Some('<') => {
                    chars.next();
                    let name: String = chars.by_ref().take_while(|c| *c != '>').collect();
                    if let Some((_, i)) = self.names.iter().find(|(n, _)| *n == name) {
                        out.push_str(&m.get(*i as i64));
                    }
                }
                _ => out.push('$'),
            }
        }
        out
    }

    /// `s.split(re)`.
    pub fn split(&self, input: &str) -> Vec<String> {
        let text = Self::chars(input);
        if text.is_empty() {
            return if self.exec("").is_some() {
                Vec::new()
            } else {
                vec![String::new()]
            };
        }
        let mut out = Vec::new();
        let mut last = 0;
        let mut from = 0;
        while from < text.len() {
            let Some(m) = self.find_from(&text, input, from) else {
                break;
            };
            let start = m.index as usize;
            let len = m.groups[0].as_ref().map_or(0, |g| g.chars().count());
            if len == 0 {
                if start >= text.len() {
                    break;
                }
                if start == last {
                    from = start + 1;
                    if start == 0 {
                        continue;
                    }
                }
            }
            if start >= text.len() {
                break;
            }
            out.push(text[last..start].iter().collect());
            for g in m.groups.iter().skip(1) {
                out.push(g.clone().unwrap_or_default());
            }
            last = start + len;
            from = if len == 0 { start + 1 } else { last };
        }
        out.push(text[last..].iter().collect());
        out
    }

    /// `re.source`.
    pub fn source(&self) -> String {
        self.source.clone()
    }

    /// `re.flags`.
    pub fn flags(&self) -> String {
        self.flags.clone()
    }
}

impl std::fmt::Display for JsRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "/{}/{}", self.source, self.flags)
    }
}

struct RxParser<'a> {
    chars: &'a [char],
    pos: usize,
    groups: usize,
    names: Vec<(String, usize)>,
}

impl RxParser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn alternatives(&mut self) -> Result<Vec<Vec<RxNode>>, String> {
        let mut alts = vec![self.sequence()?];
        while self.eat('|') {
            alts.push(self.sequence()?);
        }
        Ok(alts)
    }

    fn sequence(&mut self) -> Result<Vec<RxNode>, String> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.atom()?;
            let atom = self.quantified(atom)?;
            out.push(atom);
        }
        Ok(out)
    }

    fn quantified(&mut self, atom: RxNode) -> Result<RxNode, String> {
        let (min, max) = match self.peek() {
            Some('*') => {
                self.pos += 1;
                (0, None)
            }
            Some('+') => {
                self.pos += 1;
                (1, None)
            }
            Some('?') => {
                self.pos += 1;
                (0, Some(1))
            }
            Some('{') => {
                let save = self.pos;
                self.pos += 1;
                let min = self.number();
                let (min, max) = match (min, self.peek()) {
                    (Some(n), Some('}')) => (n, Some(n)),
                    (Some(n), Some(',')) => {
                        self.pos += 1;
                        if self.peek() == Some('}') {
                            (n, None)
                        } else {
                            match self.number() {
                                Some(m) => (n, Some(m)),
                                None => {
                                    self.pos = save;
                                    return Ok(atom);
                                }
                            }
                        }
                    }
                    _ => {
                        self.pos = save;
                        return Ok(atom);
                    }
                };
                if !self.eat('}') {
                    self.pos = save;
                    return Ok(atom);
                }
                (min, max)
            }
            _ => return Ok(atom),
        };
        if matches!(
            atom,
            RxNode::Start
                | RxNode::End
                | RxNode::WordBoundary(_)
                | RxNode::Look(..)
                | RxNode::Behind(..)
        ) {
            return Err("nothing to repeat".into());
        }
        let greedy = !self.eat('?');
        Ok(RxNode::Repeat(Box::new(atom), min, max, greedy))
    }

    fn number(&mut self) -> Option<usize> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        if start == self.pos {
            return None;
        }
        self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse()
            .ok()
    }

    fn atom(&mut self) -> Result<RxNode, String> {
        let c = self.peek().ok_or("unexpected end")?;
        self.pos += 1;
        Ok(match c {
            '.' => RxNode::Any,
            '^' => RxNode::Start,
            '$' => RxNode::End,
            '(' => {
                let mut cap = None;
                let mut look = None;
                if self.eat('?') {
                    match self.peek() {
                        Some(':') => {
                            self.pos += 1;
                        }
                        Some('=') => {
                            self.pos += 1;
                            look = Some(false);
                        }
                        Some('!') => {
                            self.pos += 1;
                            look = Some(true);
                        }
                        Some('<') => {
                            self.pos += 1;
                            if self.peek() == Some('=') || self.peek() == Some('!') {
                                let negative = self.peek() == Some('!');
                                self.pos += 1;
                                let alts = self.alternatives()?;
                                if !self.eat(')') {
                                    return Err("missing `)`".into());
                                }
                                return Ok(RxNode::Behind(alts, negative));
                            }
                            let name: String = std::iter::from_fn(|| {
                                let c = self.peek()?;
                                if c == '>' {
                                    None
                                } else {
                                    self.pos += 1;
                                    Some(c)
                                }
                            })
                            .collect();
                            if !self.eat('>') {
                                return Err("unterminated group name".into());
                            }
                            self.groups += 1;
                            self.names.push((name, self.groups));
                            cap = Some(self.groups);
                        }
                        _ => return Err("unknown group kind".into()),
                    }
                } else {
                    self.groups += 1;
                    cap = Some(self.groups);
                }
                let alts = self.alternatives()?;
                if !self.eat(')') {
                    return Err("missing `)`".into());
                }
                match look {
                    Some(negative) => RxNode::Look(alts, negative),
                    None => RxNode::Group(alts, cap),
                }
            }
            ')' => return Err("unmatched `)`".into()),
            '[' => self.class()?,
            '\\' => self.escape(false)?,
            '*' | '+' | '?' => return Err("nothing to repeat".into()),
            c => RxNode::Char(c),
        })
    }

    fn class(&mut self) -> Result<RxNode, String> {
        let negated = self.eat('^');
        let mut ranges = Vec::new();
        let mut first = true;
        loop {
            let c = self.peek().ok_or("unterminated character class")?;
            if c == ']' && !first {
                self.pos += 1;
                break;
            }
            if c == ']' {
                self.pos += 1;
                break;
            }
            first = false;
            self.pos += 1;
            let lo = if c == '\\' {
                match self.escape(true)? {
                    RxNode::Char(c) => c,
                    RxNode::Class(mut r, false) => {
                        ranges.append(&mut r);
                        continue;
                    }
                    RxNode::Class(r, true) => {
                        // A negated class inside a class: its complement.
                        ranges.extend(complement(&r));
                        continue;
                    }
                    _ => return Err("unexpected escape in a class".into()),
                }
            } else {
                c
            };
            if self.peek() == Some('-') && self.chars.get(self.pos + 1).is_some_and(|c| *c != ']') {
                self.pos += 1;
                let hi_c = self.peek().ok_or("unterminated character class")?;
                self.pos += 1;
                let hi = if hi_c == '\\' {
                    match self.escape(true)? {
                        RxNode::Char(c) => c,
                        _ => return Err("bad range".into()),
                    }
                } else {
                    hi_c
                };
                if hi < lo {
                    return Err("range out of order".into());
                }
                ranges.push((lo, hi));
            } else {
                ranges.push((lo, lo));
            }
        }
        Ok(RxNode::Class(ranges, negated))
    }

    fn escape(&mut self, in_class: bool) -> Result<RxNode, String> {
        let c = self.peek().ok_or("trailing backslash")?;
        self.pos += 1;
        let digit = vec![('0', '9')];
        let word = vec![('0', '9'), ('A', 'Z'), ('_', '_'), ('a', 'z')];
        let space = vec![
            (' ', ' '),
            ('\t', '\t'),
            ('\n', '\n'),
            ('\r', '\r'),
            ('\u{b}', '\u{c}'),
            ('\u{a0}', '\u{a0}'),
            ('\u{feff}', '\u{feff}'),
            ('\u{2028}', '\u{2029}'),
        ];
        Ok(match c {
            'd' => RxNode::Class(digit, false),
            'D' => RxNode::Class(digit, true),
            'w' => RxNode::Class(word, false),
            'W' => RxNode::Class(word, true),
            's' => RxNode::Class(space, false),
            'S' => RxNode::Class(space, true),
            'b' if in_class => RxNode::Char('\u{8}'),
            'b' => RxNode::WordBoundary(true),
            'B' => RxNode::WordBoundary(false),
            'n' => RxNode::Char('\n'),
            't' => RxNode::Char('\t'),
            'r' => RxNode::Char('\r'),
            'f' => RxNode::Char('\u{c}'),
            'v' => RxNode::Char('\u{b}'),
            '0' => RxNode::Char('\0'),
            'p' | 'P' if !in_class => {
                if !self.eat('{') {
                    return Err("`\\p` needs `{…}`".into());
                }
                let name: String = std::iter::from_fn(|| {
                    let c = self.peek()?;
                    if c == '}' {
                        None
                    } else {
                        self.pos += 1;
                        Some(c)
                    }
                })
                .collect();
                if !self.eat('}') {
                    return Err("unterminated `\\p{…}`".into());
                }
                let name = name.rsplit('=').next().unwrap_or(&name).to_string();
                if !matches!(
                    name.as_str(),
                    "L" | "Letter"
                        | "Alphabetic"
                        | "Alpha"
                        | "Lu"
                        | "Uppercase_Letter"
                        | "Uppercase"
                        | "Upper"
                        | "Ll"
                        | "Lowercase_Letter"
                        | "Lowercase"
                        | "Lower"
                        | "N"
                        | "Number"
                        | "Nd"
                        | "Decimal_Number"
                        | "digit"
                        | "White_Space"
                        | "space"
                        | "Zs"
                        | "P"
                        | "Punctuation"
                        | "punct"
                        | "S"
                        | "Symbol"
                        | "Cc"
                        | "Control"
                ) {
                    return Err(format!("the Unicode property `{name}` is not supported"));
                }
                RxNode::Prop(name, c == 'P')
            }
            'x' => RxNode::Char(self.hex(2)?),
            'u' => {
                if self.eat('{') {
                    let start = self.pos;
                    while self.peek().is_some_and(|c| c != '}') {
                        self.pos += 1;
                    }
                    let hex: String = self.chars[start..self.pos].iter().collect();
                    self.eat('}');
                    let code = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u{…}")?;
                    RxNode::Char(char::from_u32(code).ok_or("bad code point")?)
                } else {
                    RxNode::Char(self.hex(4)?)
                }
            }
            '1'..='9' if !in_class => {
                let mut n = c.to_digit(10).unwrap_or(0) as usize;
                while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
                    self.pos += 1;
                    n = n * 10 + d as usize;
                }
                RxNode::BackRef(n)
            }
            'k' if !in_class => {
                if !self.eat('<') {
                    return Err("bad named reference".into());
                }
                let name: String = std::iter::from_fn(|| {
                    let c = self.peek()?;
                    if c == '>' {
                        None
                    } else {
                        self.pos += 1;
                        Some(c)
                    }
                })
                .collect();
                self.eat('>');
                let (_, i) = self
                    .names
                    .iter()
                    .find(|(n, _)| *n == name)
                    .ok_or("unknown group name")?;
                RxNode::BackRef(*i)
            }
            c => RxNode::Char(c),
        })
    }

    fn hex(&mut self, n: usize) -> Result<char, String> {
        let hex: String = self.chars[self.pos..(self.pos + n).min(self.chars.len())]
            .iter()
            .collect();
        if hex.len() != n {
            return Err("bad escape".into());
        }
        self.pos += n;
        let code = u32::from_str_radix(&hex, 16).map_err(|_| "bad escape")?;
        char::from_u32(code).ok_or_else(|| "bad escape".into())
    }
}

/// The ranges outside the given ones.
fn complement(ranges: &[(char, char)]) -> Vec<(char, char)> {
    let mut sorted: Vec<(char, char)> = ranges.to_vec();
    sorted.sort();
    let mut out = Vec::new();
    let mut next = '\0';
    for (lo, hi) in sorted {
        if lo > next {
            out.push((next, char::from_u32(lo as u32 - 1).unwrap_or(next)));
        }
        if let Some(c) = char::from_u32(hi as u32 + 1) {
            next = c;
        } else {
            return out;
        }
    }
    out.push((next, char::MAX));
    out
}


/// A top-level variable that functions change. The program runs one piece at a
/// time: read and written like a `RefCell`.
struct JsStatic<T>(std::cell::RefCell<T>);

unsafe impl<T> Sync for JsStatic<T> {}
unsafe impl<T> Send for JsStatic<T> {}

#[allow(dead_code)]
impl<T> JsStatic<T> {
    fn new(value: T) -> Self {
        JsStatic(std::cell::RefCell::new(value))
    }

    fn read(&self) -> std::cell::Ref<'_, T> {
        self.0.borrow()
    }

    fn write(&self) -> std::cell::RefMut<'_, T> {
        self.0.borrow_mut()
    }
}

impl JsInspect for JsRegex {
    fn js_value(&self) -> JsValue {
        JsValue::Raw(self.to_string())
    }
}

impl JsInspect for JsMatch {
    fn js_value(&self) -> JsValue {
        JsValue::Array(self.groups.iter().map(|g| g.as_ref().map_or(JsValue::Undefined, |s| JsValue::Str(s.clone()))).collect())
    }
}

impl JsInspect for StringOrNumber {
    fn js_value(&self) -> JsValue {
        match self {
            StringOrNumber::String(v) => v.js_value(),
            StringOrNumber::Number(v) => v.js_value(),
        }
    }
}
