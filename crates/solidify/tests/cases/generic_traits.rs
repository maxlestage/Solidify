#[derive(Debug, Clone, PartialEq)]
struct Named<T> {
    pub name: String,
    pub value: T,
}

#[derive(Debug, Clone, PartialEq)]
struct Tagged {
    pub name: String,
    pub value: i64,
    pub tag: String,
}

#[derive(Debug, Clone, PartialEq)]
struct Cell {
    v: i64,
}

impl Cell {
    pub fn new(v: i64) -> Self {
        Self { v }
    }

    pub fn get(&self) -> i64 {
        self.v
    }

    pub fn size(&self) -> i64 {
        1
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Many {
    items: Vec<String>,
}

impl Many {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }

    pub fn get(&self) -> String {
        self.items.join("+")
    }

    pub fn size(&self) -> i64 {
        self.items.len() as i64
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Logger {
    pub lines: Vec<String>,
}

impl Logger {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn notify(&mut self, value: &str) {
        self.lines.push(format!("log: {value}"));
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Counter {
    pub count: i64,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn notify(&mut self, value: &str) {
        self.count += value.chars().count() as i64;
    }
}

#[derive(Debug, Clone)]
struct Emitter<T> {
    listeners: Vec<Box<dyn ListenerLike<T>>>,
}

impl<T: Clone> Emitter<T> {
    pub fn new() -> Self {
        Self { listeners: Vec::new() }
    }

    pub fn on(&mut self, l: Box<dyn ListenerLike<T>>) {
        self.listeners.push(l);
    }

    pub fn emit(&mut self, value: T) {
        for l in &mut self.listeners {
            l.notify(value.clone());
        }
    }

    pub fn size(&self) -> i64 {
        self.listeners.len() as i64
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Recorder<T> {
    pub seen: Vec<T>,
}

impl<T: Clone> Recorder<T> {
    pub fn new() -> Self {
        Self { seen: Vec::new() }
    }

    pub fn notify(&mut self, value: T) {
        self.seen.push(value);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Shape<T> {
    pub meta: T,
}

impl<T: Clone> Shape<T> {
    pub fn new(meta: T) -> Self {
        Self { meta }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Square {
    pub meta: String,
    side: f64,
}

impl Square {
    pub fn new(side: f64) -> Self {
        let base = Shape::new("square".to_string());
        Self { meta: base.meta, side }
    }

    pub fn area(&self) -> f64 {
        self.side * self.side
    }

    pub fn describe(&self) -> String {
        format!("{} {}", format!("{:.1}", self.area()), self.meta)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Circle {
    pub meta: String,
    r: f64,
}

impl Circle {
    pub fn new(r: f64) -> Self {
        let base = Shape::new("circle".to_string());
        Self { meta: base.meta, r }
    }

    pub fn area(&self) -> f64 {
        std::f64::consts::PI * self.r * self.r
    }

    pub fn describe(&self) -> String {
        format!("{} {}", format!("{:.1}", self.area()), self.meta)
    }
}

fn describe<T: Clone + std::fmt::Display>(c: &dyn ContainerLike<T>) -> String {
    format!("{} ({})", c.get(), c.size())
}

fn main() {
    let t = Tagged { name: "n".to_string(), value: 3, tag: "x".to_string() };
    println!("{} {}", t.value + 1, js_inspect(&t.js_value()));
    println!("{} {}", describe(&Cell::new(4)), describe(&Many::new(vec!["a".to_string(), "b".to_string()])));
    let cells = vec![Box::new(Cell::new(1)) as Box<dyn ContainerLike<_>>, Box::new(Cell::new(2)) as Box<dyn ContainerLike<_>>];
    println!("{}", js_inspect(&(cells.iter().map(|c| c.get()).collect::<Vec<_>>()).js_value()));
    let mut emitter: Emitter<String> = Emitter::new();
    emitter.on(Box::new(Logger::new()) as Box<dyn ListenerLike<_>>);
    emitter.on(Box::new(Counter::new()) as Box<dyn ListenerLike<_>>);
    emitter.emit("hello".to_string());
    emitter.emit("hi".to_string());
    println!("{}", emitter.size());
    let mut recorder: Recorder<i64> = Recorder::new();
    recorder.notify(3);
    recorder.notify(4);
    println!("{}", js_inspect(&recorder.seen.js_value()));
    let shapes = vec![Box::new(Square::new(2.0)) as Box<dyn ShapeLike<_>>, Box::new(Circle::new(1.0)) as Box<dyn ShapeLike<_>>];
    for s in &shapes {
        println!("{}", s.describe());
    }
}

/// `Container` and the classes implementing it.
trait ContainerLike<T>: std::fmt::Debug + JsInspect {
    fn get(&self) -> T;
    fn size(&self) -> i64;
    fn clone_container(&self) -> Box<dyn ContainerLike<T>>;
}

impl<T> Clone for Box<dyn ContainerLike<T>> {
    fn clone(&self) -> Self {
        self.clone_container()
    }
}

impl ContainerLike<i64> for Cell {
    fn get(&self) -> i64 {
        Cell::get(self)
    }

    fn size(&self) -> i64 {
        Cell::size(self)
    }

    fn clone_container(&self) -> Box<dyn ContainerLike<i64>> {
        Box::new(self.clone())
    }
}

impl ContainerLike<String> for Many {
    fn get(&self) -> String {
        Many::get(self)
    }

    fn size(&self) -> i64 {
        Many::size(self)
    }

    fn clone_container(&self) -> Box<dyn ContainerLike<String>> {
        Box::new(self.clone())
    }
}

/// `Listener` and the classes implementing it.
trait ListenerLike<T>: std::fmt::Debug + JsInspect {
    fn notify(&mut self, value: T);
    fn clone_listener(&self) -> Box<dyn ListenerLike<T>>;
}

impl<T> Clone for Box<dyn ListenerLike<T>> {
    fn clone(&self) -> Self {
        self.clone_listener()
    }
}

impl ListenerLike<String> for Logger {
    fn notify(&mut self, value: String) {
        Logger::notify(self, &value)
    }

    fn clone_listener(&self) -> Box<dyn ListenerLike<String>> {
        Box::new(self.clone())
    }
}

impl ListenerLike<String> for Counter {
    fn notify(&mut self, value: String) {
        Counter::notify(self, &value)
    }

    fn clone_listener(&self) -> Box<dyn ListenerLike<String>> {
        Box::new(self.clone())
    }
}

impl<T: 'static + Clone + JsInspect + std::fmt::Debug> ListenerLike<T> for Recorder<T> {
    fn notify(&mut self, value: T) {
        Recorder::notify(self, value)
    }

    fn clone_listener(&self) -> Box<dyn ListenerLike<T>> {
        Box::new(self.clone())
    }
}

/// `Shape` and its subclasses.
trait ShapeLike<T>: std::fmt::Debug + JsInspect {
    fn meta(&self) -> &T;
    fn area(&self) -> f64;
    fn describe(&self) -> String;
    fn clone_shape(&self) -> Box<dyn ShapeLike<T>>;
}

impl<T> Clone for Box<dyn ShapeLike<T>> {
    fn clone(&self) -> Self {
        self.clone_shape()
    }
}

impl ShapeLike<String> for Square {
    fn meta(&self) -> &String {
        &self.meta
    }

    fn area(&self) -> f64 {
        Square::area(self)
    }

    fn describe(&self) -> String {
        Square::describe(self)
    }

    fn clone_shape(&self) -> Box<dyn ShapeLike<String>> {
        Box::new(self.clone())
    }
}

impl ShapeLike<String> for Circle {
    fn meta(&self) -> &String {
        &self.meta
    }

    fn area(&self) -> f64 {
        Circle::area(self)
    }

    fn describe(&self) -> String {
        Circle::describe(self)
    }

    fn clone_shape(&self) -> Box<dyn ShapeLike<String>> {
        Box::new(self.clone())
    }
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

/// `String(value)`: what a value becomes in a string.
#[allow(dead_code)]
fn js_to_string(v: &JsValue) -> String {
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
        let start = output.len() + ctx.indentation + open.chars().count() + 10;
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


impl JsInspect for Cell {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Cell"), vec![("v".to_string(), self.v.js_value())])
    }
}

impl JsInspect for Circle {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Circle"), vec![("meta".to_string(), self.meta.js_value()), ("r".to_string(), self.r.js_value())])
    }
}

impl JsInspect for Counter {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Counter"), vec![("count".to_string(), self.count.js_value())])
    }
}

impl JsInspect for Logger {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Logger"), vec![("lines".to_string(), self.lines.js_value())])
    }
}

impl JsInspect for Many {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Many"), vec![("items".to_string(), self.items.js_value())])
    }
}

impl<T: JsInspect> JsInspect for Recorder<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Recorder"), vec![("seen".to_string(), self.seen.js_value())])
    }
}

impl JsInspect for Square {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Square"), vec![("meta".to_string(), self.meta.js_value()), ("side".to_string(), self.side.js_value())])
    }
}

impl JsInspect for Tagged {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("name".to_string(), self.name.js_value()), ("value".to_string(), self.value.js_value()), ("tag".to_string(), self.tag.js_value())])
    }
}
