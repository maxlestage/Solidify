#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Status {
    Active,
    Idle,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Status::Active => "active",
            Status::Idle => "idle",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Point {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, PartialEq)]
struct Dog {
    pub name: String,
    pub age: i64,
    pub nickname: Option<String>,
}

impl Dog {
    pub fn new(name: String, age: i64, nickname: Option<String>) -> Self {
        Self { name, age, nickname }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct User {
    pub id: i64,
    pub tags: Vec<String>,
    pub status: Status,
    pub home: Option<Point>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NestedDeepDeeper {
    pub deepest: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NestedDeep {
    pub deeper: NestedDeepDeeper,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Nested {
    pub deep: NestedDeep,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub nested: Nested,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object2 {
    pub matrix: Vec<Vec<i64>>,
}

fn main() {
    let points = vec![Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
    println!("{} {} {} {} []", js_inspect(&points.js_value()), js_inspect(&points[0].js_value()), js_inspect(&(vec![1, 2, 3]).js_value()), js_inspect(&(vec!["a".to_string(), "it's".to_string()]).js_value()));
    println!("{} {}", js_inspect(&(Dog::new("Rex".to_string(), 3, None)).js_value()), js_inspect(&(vec![Dog::new("a".to_string(), 1, None), Dog::new("b".to_string(), 2, Some("bee".to_string()))]).js_value()));
    let user = User { id: 7, tags: vec!["x".to_string(), "y".to_string()], status: Status::Active, home: None };
    println!("{} {}", js_inspect(&user.js_value()), js_inspect(&(Object { nested: Nested { deep: NestedDeep { deeper: NestedDeepDeeper { deepest: 1 } } } }).js_value()));
    let mut scores: JsMap<String, i64> = JsMap::new();
    scores.insert("zoe".to_string(), 10);
    scores.insert("adam".to_string(), 7);
    scores.insert("maya".to_string(), 9);
    scores.remove("adam");
    scores.insert("adam".to_string(), 8);
    let seen = vec![3, 1, 3, 2].into_iter().collect::<JsSet<_>>();
    println!("{} {} Map(0) {{}} Set(0) {{}}", js_inspect(&scores.js_value()), js_inspect(&seen.js_value()));
    println!("Scores: {} {} {}", js_inspect(&scores.keys().cloned().collect::<Vec<_>>().js_value()), js_inspect(&scores.values().cloned().collect::<Vec<_>>().js_value()), js_inspect(&seen.iter().cloned().collect::<Vec<_>>().js_value()));
    for (name, &score) in &scores {
        println!("{name} => {score}");
    }
    let numbers = (0..30 as i64).map(|i| (|_, i| i * 100)((), i)).collect::<Vec<_>>();
    println!("{}", js_inspect(&numbers.js_value()));
    let words = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string(), "delta".to_string(), "epsilon".to_string(), "zeta".to_string(), "eta".to_string(), "theta".to_string(), "iota".to_string(), "kappa".to_string(), "lambda".to_string()];
    println!("{}", js_inspect(&words.js_value()));
    let matrix = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
    println!("{} {}", js_inspect(&(Object2 { matrix: matrix.clone() }).js_value()), js_inspect(&vec![vec![vec![vec!["deep".to_string()]]]].js_value()));
    println!("{} {} {} {} {}", js_to_string(&(vec![1, 2, 3]).js_value()), js_to_string(&points[1].js_value()), js_to_string(&matrix.js_value()), js_inspect(&(vec![1.5, 2.25, -0.5]).js_value()), js_inspect(&(vec![true, false]).js_value()));
    let record = JsMap::from([("b".to_string(), vec![2]), ("a".to_string(), vec![1, 1])]);
    println!("{} {} {}", js_inspect(&JsValue::Object(None, record.iter().map(|(k, v)| (k.to_string(), v.js_value())).collect())), js_inspect(&record.keys().cloned().collect::<Vec<_>>().js_value()), js_inspect(&(record.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>()).js_value()));
    let found = points.iter().find(|p| p.x > 2).cloned();
    println!("{} {} {} {}", found.as_ref().map_or("undefined".to_string(), |v| js_inspect(&v.js_value())), points.iter().find(|p| p.x > 10).cloned().as_ref().map_or("undefined".to_string(), |v| js_inspect(&v.js_value())), scores.get("zoe").copied().as_ref().map_or("undefined".to_string(), |v| v.to_string()), scores.get("nobody").copied().as_ref().map_or("undefined".to_string(), |v| v.to_string()));
    let mut line = "items: ".to_string();
    line.push_str(&format!("{}", js_to_string(&(points.iter().map(|p| p.x).collect::<Vec<_>>()).js_value())));
    println!("{line} {} {} {}", vec![1, 2].iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "), vec![vec![1, 2], vec![3]].iter().map(|x| js_to_string(&x.js_value())).collect::<Vec<_>>().join(";"), vec![1, 2, 3].iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
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


/// A `Map`: a hash map that keeps its keys in insertion order, like JavaScript.
#[derive(Clone, Debug)]
struct JsMap<K, V> {
    entries: Vec<(K, V)>,
    index: std::collections::HashMap<K, usize>,
}

#[allow(dead_code)]
impl<K: std::hash::Hash + Eq + Clone, V> JsMap<K, V> {
    fn new() -> Self {
        JsMap { entries: Vec::new(), index: std::collections::HashMap::new() }
    }
    fn len(&self) -> usize {
        self.entries.len()
    }
    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    fn position<Q: ?Sized + std::hash::Hash + Eq>(&self, key: &Q) -> Option<usize>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.index.get(key).copied()
    }
    fn get<Q: ?Sized + std::hash::Hash + Eq>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.position(key).map(|i| &self.entries[i].1)
    }
    fn get_mut<Q: ?Sized + std::hash::Hash + Eq>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.position(key).map(|i| &mut self.entries[i].1)
    }
    fn contains_key<Q: ?Sized + std::hash::Hash + Eq>(&self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
    {
        self.index.contains_key(key)
    }
    /// `map.set(key, value)`: an existing key keeps its position.
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.index.get(&key) {
            Some(&i) => Some(std::mem::replace(&mut self.entries[i].1, value)),
            None => {
                self.index.insert(key.clone(), self.entries.len());
                self.entries.push((key, value));
                None
            }
        }
    }
    fn remove<Q: ?Sized + std::hash::Hash + Eq>(&mut self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
    {
        let i = self.index.remove(key)?;
        let (_, value) = self.entries.remove(i);
        for (_, pos) in self.index.iter_mut() {
            if *pos > i {
                *pos -= 1;
            }
        }
        Some(value)
    }
    fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }
    fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }
    fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }
    fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|(_, v)| v)
    }
    fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
    fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.entries.iter_mut().map(|(k, v)| (&*k, v))
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> Default for JsMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: PartialEq> PartialEq for JsMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> FromIterator<(K, V)> for JsMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = JsMap::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl<K: std::hash::Hash + Eq + Clone, V, const N: usize> From<[(K, V); N]> for JsMap<K, V> {
    fn from(entries: [(K, V); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> Extend<(K, V)> for JsMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> IntoIterator for JsMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a, K: std::hash::Hash + Eq + Clone, V> IntoIterator for &'a JsMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Map<std::slice::Iter<'a, (K, V)>, fn(&'a (K, V)) -> (&'a K, &'a V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

impl<K: std::hash::Hash + Eq + Clone + std::borrow::Borrow<Q>, V, Q: ?Sized + std::hash::Hash + Eq>
    std::ops::Index<&Q> for JsMap<K, V>
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}


/// A `Set`: a hash set that keeps its values in insertion order, like JavaScript.
#[derive(Clone, Debug)]
struct JsSet<T> {
    items: Vec<T>,
    index: std::collections::HashMap<T, usize>,
}

#[allow(dead_code)]
impl<T: std::hash::Hash + Eq + Clone> JsSet<T> {
    fn new() -> Self {
        JsSet { items: Vec::new(), index: std::collections::HashMap::new() }
    }
    fn len(&self) -> usize {
        self.items.len()
    }
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    fn contains<Q: ?Sized + std::hash::Hash + Eq>(&self, value: &Q) -> bool
    where
        T: std::borrow::Borrow<Q>,
    {
        self.index.contains_key(value)
    }
    /// `set.add(value)`: true when the value was not there.
    fn insert(&mut self, value: T) -> bool {
        if self.index.contains_key(&value) {
            return false;
        }
        self.index.insert(value.clone(), self.items.len());
        self.items.push(value);
        true
    }
    fn remove<Q: ?Sized + std::hash::Hash + Eq>(&mut self, value: &Q) -> bool
    where
        T: std::borrow::Borrow<Q>,
    {
        let Some(i) = self.index.remove(value) else {
            return false;
        };
        self.items.remove(i);
        for (_, pos) in self.index.iter_mut() {
            if *pos > i {
                *pos -= 1;
            }
        }
        true
    }
    fn clear(&mut self) {
        self.items.clear();
        self.index.clear();
    }
    fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }
}

impl<T: std::hash::Hash + Eq + Clone> Default for JsSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::hash::Hash + Eq + Clone> PartialEq for JsSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl<T: std::hash::Hash + Eq + Clone> FromIterator<T> for JsSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = JsSet::new();
        for v in iter {
            set.insert(v);
        }
        set
    }
}

impl<T: std::hash::Hash + Eq + Clone, const N: usize> From<[T; N]> for JsSet<T> {
    fn from(items: [T; N]) -> Self {
        items.into_iter().collect()
    }
}

impl<T: std::hash::Hash + Eq + Clone> Extend<T> for JsSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for v in iter {
            self.insert(v);
        }
    }
}

impl<T: std::hash::Hash + Eq + Clone> IntoIterator for JsSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T: std::hash::Hash + Eq + Clone> IntoIterator for &'a JsSet<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}


impl<K: JsInspect + std::hash::Hash + Eq + Clone, V: JsInspect> JsInspect for JsMap<K, V> {
    fn js_value(&self) -> JsValue {
        JsValue::Map(self.iter().map(|(k, v)| (k.js_value(), v.js_value())).collect())
    }
}

impl<T: JsInspect + std::hash::Hash + Eq + Clone> JsInspect for JsSet<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Set(self.iter().map(JsInspect::js_value).collect())
    }
}

impl JsInspect for Dog {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Dog"), vec![("name".to_string(), self.name.js_value()), ("age".to_string(), self.age.js_value()), ("nickname".to_string(), self.nickname.as_ref().map_or(JsValue::Undefined, |v| v.js_value()))])
    }
}

impl JsInspect for Nested {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("deep".to_string(), self.deep.js_value())])
    }
}

impl JsInspect for NestedDeep {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("deeper".to_string(), self.deeper.js_value())])
    }
}

impl JsInspect for NestedDeepDeeper {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("deepest".to_string(), self.deepest.js_value())])
    }
}

impl JsInspect for Object {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("nested".to_string(), self.nested.js_value())])
    }
}

impl JsInspect for Object2 {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("matrix".to_string(), self.matrix.js_value())])
    }
}

impl JsInspect for Point {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("x".to_string(), self.x.js_value()), ("y".to_string(), self.y.js_value())])
    }
}

impl JsInspect for Status {
    fn js_value(&self) -> JsValue {
        JsValue::Str(self.to_string())
    }
}

impl JsInspect for User {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("id".to_string(), self.id.js_value()), ("tags".to_string(), self.tags.js_value()), ("status".to_string(), self.status.js_value()), ("home".to_string(), self.home.js_value())])
    }
}
