use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
struct Range {
    from: i64,
    to: i64,
}

impl Range {
    pub fn new(from: i64, to: i64) -> Self {
        Self { from, to }
    }

    pub fn js_iterator(&self) -> JsGenerator<i64> {
        let this = self.clone();
        JsGenerator::new(move |y| {
            for i in this.from..=this.to {
                y.yield_(i);
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Countdown {
    n: i64,
}

impl Countdown {
    pub fn new(n: i64) -> Self {
        Self { n }
    }

    pub fn js_iterator(&self) -> Object2 {
        let k = Rc::new(RefCell::new(self.n));
        Object2 { next: Rc::new({ let k = k.clone(); move || { if (*k.borrow()) > 0 { Object { value: { let old = (*k.borrow()); (*k.borrow_mut()) -= 1; old }, done: false } } else { Object { value: 0, done: true } } } }) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub value: i64,
    pub done: bool,
}

#[derive(Clone)]
pub struct Object2 {
    pub next: Rc<dyn Fn() -> Object>,
}

impl std::fmt::Debug for Object2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Object2")
            .field("next", &"[Function]")
            .finish()
    }
}

fn main() {
    let r = Range::new(1, 4);
    for x in r.js_iterator() {
        println!("{x}");
    }
    println!("{} {}", js_inspect(&(Range::new(2, 3).js_iterator().collect::<Vec<_>>()).js_value()), js_inspect(&({ let it = Countdown::new(3).js_iterator(); std::iter::from_fn(move || { let r = (it.next)(); if r.done { None } else { Some(r.value) } }) }.collect::<Vec<_>>()).js_value()));
    let id = JsSymbol::new(Some("id"));
    let other = JsSymbol::new(Some("id"));
    println!("{} {} {} {} {}", id.to_string(), id.description().as_ref().map_or("null".to_string(), |v| v.to_string()), id == other, id == id, "symbol");
    let mut tags: HashMap<JsSymbol, String> = HashMap::new();
    tags.insert(id, "first".to_string());
    println!("{} {} {}", tags.get(&id).cloned().as_ref().map_or("undefined".to_string(), |v| v.to_string()), tags.get(&other).cloned().as_ref().map_or("undefined".to_string(), |v| v.to_string()), JsSymbol::for_key("app") == JsSymbol::for_key("app"));
    println!("{} {} {}", JsSymbol::for_key("app").key_for().as_ref().map_or("undefined".to_string(), |v| v.to_string()), id.key_for().as_ref().map_or("undefined".to_string(), |v| v.to_string()), JsSymbol::new(None).to_string());
    let xs = vec![10, 20, 30];
    let mut i = 0;
    { let a0 = xs[{ let old = i; i += 1; old } as usize]; let a1 = xs[{ let old = i; i += 1; old } as usize]; let a2 = i; let a3 = xs[{ i -= 1; i } as usize]; println!("{} {} {} {}", a0, a1, a2, a3) };
    let a: i64;
    let b: i64;
    a = { b = 5; b };
    println!("{}", a + b);
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


/// A generator (`function*`): its body runs on its own thread and hands each `yield` over
/// a channel, one value per `next()`, so the sequence is lazy and may be infinite.
struct JsGenerator<T> {
    ask: std::sync::mpsc::SyncSender<()>,
    values: std::sync::mpsc::Receiver<Option<T>>,
}

/// What a generator body yields through.
struct JsYield<T> {
    asks: std::sync::mpsc::Receiver<()>,
    values: std::sync::mpsc::SyncSender<Option<T>>,
}

impl<T: Send + 'static> JsGenerator<T> {
    fn new(body: impl FnOnce(&JsYield<T>) + Send + 'static) -> Self {
        let (ask, asks) = std::sync::mpsc::sync_channel(0);
        let (values, received) = std::sync::mpsc::sync_channel(0);
        std::thread::spawn(move || {
            let y = JsYield { asks, values };
            if y.asks.recv().is_err() {
                return;
            }
            body(&y);
            let _ = y.values.send(None);
        });
        JsGenerator { ask, values: received }
    }
}

impl<T> JsYield<T> {
    /// `yield value`: hands the value over and waits for the next `next()`.
    fn yield_(&self, value: T) {
        if self.values.send(Some(value)).is_err() || self.asks.recv().is_err() {
            // The generator was dropped: its body never resumes.
            loop {
                std::thread::park();
            }
        }
    }
}

impl<T> Iterator for JsGenerator<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.ask.send(()).ok()?;
        self.values.recv().ok().flatten()
    }
}

impl<T> std::fmt::Debug for JsGenerator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Object [Generator] {}")
    }
}


/// A `Symbol`: equal only to itself (or, made by `Symbol.for`, to the symbol of the same
/// key).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(dead_code)]
struct JsSymbol {
    id: u64,
    description: Option<&'static str>,
    registered: bool,
}

#[allow(dead_code)]
impl JsSymbol {
    /// `Symbol("id")`: a new symbol.
    fn new(description: Option<&str>) -> JsSymbol {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        JsSymbol {
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            description: description.map(|d| &*Box::leak(d.to_string().into_boxed_str())),
            registered: false,
        }
    }

    /// `Symbol.for(key)`: the symbol of this key, made once.
    fn for_key(key: &str) -> JsSymbol {
        static REGISTRY: std::sync::Mutex<Vec<JsSymbol>> = std::sync::Mutex::new(Vec::new());
        let mut registry = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(s) = registry.iter().find(|s| s.description == Some(key)) {
            return *s;
        }
        let mut s = JsSymbol::new(Some(key));
        s.registered = true;
        registry.push(s);
        s
    }

    /// `Symbol.keyFor(s)`.
    fn key_for(self) -> Option<String> {
        if self.registered {
            self.description.map(str::to_string)
        } else {
            None
        }
    }

    /// `s.description`.
    fn description(&self) -> Option<String> {
        self.description.map(str::to_string)
    }
}

/// `String(s)`: `Symbol(id)`.
impl std::fmt::Display for JsSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Symbol({})", self.description.unwrap_or(""))
    }
}

impl std::fmt::Debug for JsSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

