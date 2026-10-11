#[derive(Debug, Clone, PartialEq)]
struct Node {
    pub value: i64,
    pub next: Option<Shared<Node>>,
}

impl Node {
    pub fn new(value: i64) -> Self {
        Self { value, next: None }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct List {
    pub head: Option<Shared<Node>>,
    pub size: i64,
}

impl List {
    pub fn new() -> Self {
        Self { head: None, size: 0 }
    }

    pub fn push(&mut self, value: i64) {
        let node = Shared::new(Node::new(value));
        if let Some(head) = &mut self.head {
            let mut current = head.clone();
            while current.borrow().next.clone().is_some() {
                let value_ = current.borrow().next.clone().as_mut().unwrap().clone();
                current = value_;
            }
            current.borrow_mut().next = Some(node);
        } else {
            self.head = Some(node);
        }
        self.size += 1;
    }

    pub fn sum(&self) -> i64 {
        let mut total = 0;
        let mut current = self.head.clone();
        while current.is_some() {
            total += current.as_ref().unwrap().borrow().value;
            let value_ = current.as_ref().unwrap().borrow().next.clone();
            current = value_;
        }
        total
    }

    pub fn first(&self) -> Option<i64> {
        let Some(head) = &self.head else {
            return None;
        };
        Some(head.borrow().value)
    }
}

fn main() {
    let mut list = List::new();
    list.push(3);
    list.push(4);
    list.push(5);
    println!("{} {} {}", list.size, list.sum(), list.first().as_ref().map_or("null".to_string(), |v| v.to_string()));
    let empty = List::new();
    let f = empty.first();
    println!("{}", if let Some(f) = f { format!("first {f}") } else { "empty".to_string() });
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

