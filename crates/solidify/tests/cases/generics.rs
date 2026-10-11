#[derive(Debug, Clone, PartialEq)]
struct Stack<T> {
    items: Vec<T>,
}

impl<T: Clone> Stack<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, item: T) -> &mut Self {
        self.items.push(item);
        self
    }

    pub fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    pub fn peek(&self) -> Option<T> {
        if self.items.len() > 0 { Some(self.items[self.items.len() - 1].clone()) } else { None }
    }

    pub fn size(&self) -> i64 {
        self.items.len() as i64
    }
}

#[derive(Debug, Clone, PartialEq)]
struct NumberStack {
    items: Vec<i64>,
}

impl NumberStack {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn sum(&self) -> i64 {
        self.items.iter().fold(0, |a, &b| a + b)
    }

    pub fn push(&mut self, item: i64) -> &mut Self {
        self.items.push(item);
        self
    }

    pub fn pop(&mut self) -> Option<i64> {
        self.items.pop()
    }

    pub fn peek(&self) -> Option<i64> {
        if self.items.len() > 0 { Some(self.items[self.items.len() - 1]) } else { None }
    }

    pub fn size(&self) -> i64 {
        self.items.len() as i64
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Pair<A, B> {
    pub first: A,
    pub second: B,
}

impl<A: Clone, B: Clone> Pair<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }

    pub fn swap(&self) -> Pair<B, A> {
        Pair::new(self.second.clone(), self.first.clone())
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Box_<T> {
    pub value: T,
}

impl<T: Clone> Box_<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn map<U: Clone>(&self, f: impl Fn(T) -> U) -> Box_<U> {
        Box_::new(f(self.value.clone()))
    }
}

impl Box_<()> {

    pub fn of<U: Clone>(value: U) -> Box_<U> {
        Box_::new(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ListNode<T> {
    pub value: T,
    pub next: Option<Box<ListNode<T>>>,
}

impl<T: Clone> ListNode<T> {
    pub fn new(value: T) -> Self {
        Self { value, next: None }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LinkedList<T> {
    head: Option<Box<ListNode<T>>>,
    count: i64,
}

impl<T: Clone> LinkedList<T> {
    pub fn new() -> Self {
        Self { head: None, count: 0 }
    }

    pub fn add(&mut self, value: T) -> &mut Self {
        let mut node: ListNode<T> = ListNode::new(value);
        node.next = self.head.clone();
        self.head = Some(Box::new(node));
        self.count += 1;
        self
    }

    pub fn to_array(&self) -> Vec<T> {
        let mut out: Vec<T> = Vec::new();
        let mut current = &self.head;
        while current.is_some() {
            out.push(current.as_ref().unwrap().value.clone());
            current = &current.as_ref().unwrap().next;
        }
        out
    }

    pub fn length(&self) -> i64 {
        self.count
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Page<T> {
    pub items: Vec<T>,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct HasId {
    pub id: i64,
}

#[derive(Debug, Clone, PartialEq)]
struct User {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct Repo<T> {
    items: Vec<T>,
}

impl<T: Clone + HasIdLike> Repo<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: T) {
        self.items.push(item);
    }

    pub fn ids(&self) -> Vec<i64> {
        self.items.iter().map(|i| *i.id()).collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenameT {
    pub name: String,
}

fn find_by_id<T: Clone + HasIdLike>(items: &[T], id: i64) -> Option<T> {
    items.iter().find(|item| *item.id() == id).cloned()
}

fn rename<T: Clone + RenameTLike>(mut x: T, name: String) -> T {
    *x.name_mut() = name;
    x
}

fn show<T: Clone + JsInspect>(x: T) -> String {
    format!("<{}>", js_to_string(&x.js_value()))
}

fn identity<T: Clone>(x: T) -> T {
    x
}

fn main() -> Result<(), String> {
    let mut numbers: Stack<i64> = Stack::new();
    numbers.push(1).push(2).push(3);
    println!("{} {} {}", numbers.pop().as_ref().map_or("undefined".to_string(), |v| v.to_string()), numbers.peek().as_ref().map_or("undefined".to_string(), |v| v.to_string()), numbers.size());
    let mut words: Stack<String> = Stack::new();
    words.push("a".to_string());
    println!("{} {}", words.peek().as_ref().map_or("undefined".to_string(), |v| v.to_string()), js_inspect(&words.js_value()));
    let mut total = NumberStack::new();
    total.push(3).push(4);
    println!("{} {}", total.sum(), total.size());
    let pair: Pair<String, i64> = Pair::new("x".to_string(), 3);
    let swapped: Pair<i64, String> = pair.swap();
    println!("{} {} {}", swapped.first + 1, swapped.second, js_inspect(&swapped.js_value()));
    let boxed: Box_<String> = Box_::of(2).map(|n| n * 10).map(|n| format!("n={n}"));
    println!("{} {}", boxed.value, js_inspect(&boxed.js_value()));
    let mut list: LinkedList<String> = LinkedList::new();
    list.add("a".to_string()).add("b".to_string()).add("c".to_string());
    println!("{} {}", js_inspect(&list.to_array().js_value()), list.length());
    let page: Page<f64> = js_json_parse_as::<Page<f64>>("{\"items\":[1,2],\"total\":2}")?;
    println!("{} {} {}", page.items.len(), js_number_text(page.total), js_inspect(&page.js_value()));
    let empty: Page<String> = Page { items: Vec::new(), total: 0.0 };
    println!("{}", js_inspect(&empty.js_value()));
    let entry = ("one".to_string(), 1);
    println!("{} {}", entry.0, entry.1);
    let users = vec![User { id: 1, name: "ann".to_string() }, User { id: 2, name: "bob".to_string() }];
    println!("{} {}", find_by_id(&users, 2).as_ref().map(|v| v.name.clone()).as_ref().map_or("undefined".to_string(), |v| v.to_string()), find_by_id(&users, 5).as_ref().map_or("undefined".to_string(), |v| js_inspect(&v.js_value())));
    println!("{}", js_inspect(&(rename(RenameT { name: "a".to_string() }, "b".to_string())).js_value()));
    let mut repo: Repo<User> = Repo::new();
    repo.add(users[0].clone());
    repo.add(users[1].clone());
    println!("{}", js_inspect(&repo.ids().js_value()));
    println!("{} {} {}", show(1), show("s"), show(users[0].clone()));
    println!("{} {}", identity(5), identity("x"));
    Ok(())
}

/// What `T extends HasId` needs of `T`.
trait HasIdLike {
    fn id(&self) -> &i64;
}

impl HasIdLike for User {
    fn id(&self) -> &i64 {
        &self.id
    }
}

/// What `T extends RenameT` needs of `T`.
trait RenameTLike {
    fn name(&self) -> &String;
    fn name_mut(&mut self) -> &mut String;
}

impl RenameTLike for RenameT {
    fn name(&self) -> &String {
        &self.name
    }

    fn name_mut(&mut self) -> &mut String {
        &mut self.name
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


/// `JSON.stringify(value, null, indent)`: the JSON text of a value (`undefined` when the
/// value has none).
#[allow(dead_code)]
fn js_json_stringify(v: &JsValue, indent: &str) -> String {
    let indent: String = indent.chars().take(10).collect();
    let mut out = String::new();
    if js_json_write(v, &indent, 0, &mut out) {
        out
    } else {
        "undefined".to_string()
    }
}

/// Writes `v`; false when JSON has no text for it (`undefined`, a function).
#[allow(dead_code)]
fn js_json_write(v: &JsValue, indent: &str, depth: usize, out: &mut String) -> bool {
    let newline = |out: &mut String, depth: usize| {
        if !indent.is_empty() {
            out.push('\n');
            for _ in 0..depth {
                out.push_str(indent);
            }
        }
    };
    if let Some(inner) = v.shown() {
        return js_json_write(&inner, indent, depth, out);
    }
    match v {
        JsValue::Int(n) => out.push_str(&n.to_string()),
        JsValue::Float(x) => {
            if x.is_finite() {
                out.push_str(&js_number_text(*x));
            } else {
                out.push_str("null");
            }
        }
        JsValue::Str(s) => js_json_quote(s, out),
        JsValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsValue::Null => out.push_str("null"),
        JsValue::Undefined | JsValue::Func(_) => return false,
        JsValue::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return true;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, depth + 1);
                if !js_json_write(item, indent, depth + 1, out) {
                    out.push_str("null");
                }
            }
            newline(out, depth);
            out.push(']');
        }
        JsValue::Object(_, props) => {
            let mut written = 0;
            let mut text = String::new();
            for (key, value) in props {
                let mut item = String::new();
                if !js_json_write(value, indent, depth + 1, &mut item) {
                    continue;
                }
                if written > 0 {
                    text.push(',');
                }
                newline(&mut text, depth + 1);
                js_json_quote(key, &mut text);
                text.push(':');
                if !indent.is_empty() {
                    text.push(' ');
                }
                text.push_str(&item);
                written += 1;
            }
            if written == 0 {
                out.push_str("{}");
                return true;
            }
            out.push('{');
            out.push_str(&text);
            newline(out, depth);
            out.push('}');
        }
        JsValue::Map(_) | JsValue::Set(_) | JsValue::Raw(_) => out.push_str("{}"),
        JsValue::Date(iso, _) => js_json_quote(iso, out),
        JsValue::Ref(_) | JsValue::Proxy(_) => unreachable!("shown above"),
    }
    true
}

/// A string as JSON quotes it.
#[allow(dead_code)]
fn js_json_quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A JSON document, as `JSON.parse` reads it.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

#[allow(dead_code)]
impl JsonValue {
    fn kind(&self) -> &'static str {
        match self {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "a boolean",
            JsonValue::Num(_) => "a number",
            JsonValue::Str(_) => "a string",
            JsonValue::Array(_) => "an array",
            JsonValue::Object(_) => "an object",
        }
    }

    /// The entries of an object, or the error for a `what` that needs one.
    fn object(&self, what: &str) -> Result<&[(String, JsonValue)], String> {
        match self {
            JsonValue::Object(o) => Ok(o),
            other => Err(format!(
                "JSON: expected an object for {what}, got {}",
                other.kind()
            )),
        }
    }

    fn get<'a>(o: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
        o.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

/// Implemented by every type `JSON.parse` can build.
#[allow(dead_code)]
trait JsFromJson: Sized {
    fn from_json(v: &JsonValue) -> Result<Self, String>;

    /// The value of a field the object does not have (`undefined`).
    fn from_missing(key: &str) -> Result<Self, String> {
        Err(format!("JSON: missing field \"{key}\""))
    }
}

/// A field of a parsed object.
#[allow(dead_code)]
fn js_json_field<T: JsFromJson>(o: &[(String, JsonValue)], key: &str) -> Result<T, String> {
    match JsonValue::get(o, key) {
        Some(v) => T::from_json(v).map_err(|e| format!("{e} (in \"{key}\")")),
        None => T::from_missing(key),
    }
}

/// `JSON.parse(text)` as a `T`.
#[allow(dead_code)]
fn js_json_parse_as<T: JsFromJson>(text: &str) -> Result<T, String> {
    T::from_json(&js_json_parse(text)?)
}

impl JsFromJson for i64 {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Num(x) if x.fract() == 0.0 && x.abs() < 9.2e18 => Ok(*x as i64),
            JsonValue::Num(x) => Err(format!(
                "JSON: expected an integer, got {}",
                js_number_text(*x)
            )),
            other => Err(format!("JSON: expected a number, got {}", other.kind())),
        }
    }
}
impl JsFromJson for f64 {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Num(x) => Ok(*x),
            other => Err(format!("JSON: expected a number, got {}", other.kind())),
        }
    }
}
impl JsFromJson for String {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Str(s) => Ok(s.clone()),
            other => Err(format!("JSON: expected a string, got {}", other.kind())),
        }
    }
}
impl JsFromJson for bool {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Bool(b) => Ok(*b),
            other => Err(format!("JSON: expected a boolean, got {}", other.kind())),
        }
    }
}
impl JsFromJson for () {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Null => Ok(()),
            other => Err(format!("JSON: expected null, got {}", other.kind())),
        }
    }
}
impl<T: JsFromJson> JsFromJson for Option<T> {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Null => Ok(None),
            other => T::from_json(other).map(Some),
        }
    }
    fn from_missing(_key: &str) -> Result<Self, String> {
        Ok(None)
    }
}
impl<T: JsFromJson> JsFromJson for Box<T> {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        T::from_json(v).map(Box::new)
    }
}
impl<T: JsFromJson> JsFromJson for Vec<T> {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, x)| T::from_json(x).map_err(|e| format!("{e} (at index {i})")))
                .collect(),
            other => Err(format!("JSON: expected an array, got {}", other.kind())),
        }
    }
}
impl<V: JsFromJson> JsFromJson for std::collections::HashMap<String, V> {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        let mut out = std::collections::HashMap::new();
        for (k, x) in v.object("an object")? {
            out.insert(
                k.clone(),
                V::from_json(x).map_err(|e| format!("{e} (in \"{k}\")"))?,
            );
        }
        Ok(out)
    }
}
impl<A: JsFromJson, B: JsFromJson> JsFromJson for (A, B) {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Array(items) if items.len() == 2 => {
                Ok((A::from_json(&items[0])?, B::from_json(&items[1])?))
            }
            other => Err(format!("JSON: expected a pair, got {}", other.kind())),
        }
    }
}
impl<A: JsFromJson, B: JsFromJson, C: JsFromJson> JsFromJson for (A, B, C) {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        match v {
            JsonValue::Array(items) if items.len() == 3 => Ok((
                A::from_json(&items[0])?,
                B::from_json(&items[1])?,
                C::from_json(&items[2])?,
            )),
            other => Err(format!("JSON: expected 3 values, got {}", other.kind())),
        }
    }
}

/// `JSON.parse(text)`: the document, or the message of the `SyntaxError`.
#[allow(dead_code)]
fn js_json_parse(text: &str) -> Result<JsonValue, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut p = JsonParser {
        text: &chars,
        pos: 0,
    };
    p.skip_ws();
    let v = p.value()?;
    p.skip_ws();
    if p.pos < chars.len() {
        return Err(p.at("Unexpected non-whitespace character after JSON", p.pos));
    }
    Ok(v)
}

#[allow(dead_code)]
struct JsonParser<'a> {
    text: &'a [char],
    pos: usize,
}

#[allow(dead_code)]
impl JsonParser<'_> {
    fn peek(&self) -> Option<char> {
        self.text.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.pos += 1;
        }
    }

    /// A message located the way V8 does it.
    fn at(&self, msg: &str, pos: usize) -> String {
        let (mut line, mut col) = (1, 1);
        for &c in &self.text[..pos.min(self.text.len())] {
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        let sep = if msg.ends_with("JSON") { "" } else { " in JSON" };
        format!("{msg}{sep} at position {pos} (line {line} column {col})")
    }

    fn end(&self) -> String {
        "Unexpected end of JSON input".to_string()
    }

    /// The character at `pos` cannot start what was expected.
    fn unexpected(&self) -> String {
        let c = self.text[self.pos];
        let src: String = self.text.iter().collect();
        if matches!(
            src.as_str(),
            "undefined" | "NaN" | "Infinity" | "-Infinity" | "[object Object]"
        ) {
            return format!("\"{src}\" is not valid JSON");
        }
        if self.text.len() < 21 {
            return format!("Unexpected token '{c}', \"{src}\" is not valid JSON");
        }
        let start = self.pos.saturating_sub(10);
        let end = (self.pos + 10).min(self.text.len());
        let window: String = self.text[start..end].iter().collect();
        format!(
            "Unexpected token '{c}', {}\"{window}\"{} is not valid JSON",
            if start > 0 { "..." } else { "" },
            if end < self.text.len() { "..." } else { "" }
        )
    }

    fn value(&mut self) -> Result<JsonValue, String> {
        match self.peek() {
            None => Err(self.end()),
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(JsonValue::Str(self.string()?)),
            Some('-' | '0'..='9') => self.number(),
            Some('t') => self.word("true", JsonValue::Bool(true)),
            Some('f') => self.word("false", JsonValue::Bool(false)),
            Some('n') => self.word("null", JsonValue::Null),
            Some(_) => Err(self.unexpected()),
        }
    }

    fn word(&mut self, word: &str, value: JsonValue) -> Result<JsonValue, String> {
        for expected in word.chars() {
            match self.peek() {
                None => return Err(self.end()),
                Some(c) if c == expected => self.pos += 1,
                Some(_) => return Err(self.unexpected()),
            }
        }
        Ok(value)
    }

    fn object(&mut self) -> Result<JsonValue, String> {
        self.pos += 1;
        let mut entries: Vec<(String, JsonValue)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some('"') {
                let msg = if entries.is_empty() {
                    "Expected property name or '}'"
                } else {
                    "Expected double-quoted property name"
                };
                return Err(self.at(msg, self.pos));
            }
            let key = self.string()?;
            self.skip_ws();
            if self.peek() != Some(':') {
                return Err(self.at("Expected ':' after property name", self.pos));
            }
            self.pos += 1;
            self.skip_ws();
            let value = self.value()?;
            match entries.iter_mut().find(|(k, _)| *k == key) {
                Some(entry) => entry.1 = value,
                None => entries.push((key, value)),
            }
            self.skip_ws();
            match self.peek() {
                Some(',') => self.pos += 1,
                Some('}') => {
                    self.pos += 1;
                    return Ok(JsonValue::Object(entries));
                }
                _ => return Err(self.at("Expected ',' or '}' after property value", self.pos)),
            }
        }
    }

    fn array(&mut self) -> Result<JsonValue, String> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(',') => self.pos += 1,
                Some(']') => {
                    self.pos += 1;
                    return Ok(JsonValue::Array(items));
                }
                _ => return Err(self.at("Expected ',' or ']' after array element", self.pos)),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(self.at("Unterminated string", self.pos));
            };
            match c {
                '"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                '\\' => {
                    self.pos += 1;
                    let Some(e) = self.peek() else {
                        return Err(self.end());
                    };
                    self.pos += 1;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let mut code = self.hex4()?;
                            if (0xD800..0xDC00).contains(&code)
                                && self.peek() == Some('\\')
                                && self.text.get(self.pos + 1) == Some(&'u')
                            {
                                let save = self.pos;
                                self.pos += 2;
                                match self.hex4() {
                                    Ok(low) if (0xDC00..0xE000).contains(&low) => {
                                        code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                    }
                                    _ => self.pos = save,
                                }
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        _ => return Err(self.at("Bad escaped character", self.pos - 1)),
                    }
                }
                c if (c as u32) < 0x20 => {
                    return Err(self.at("Bad control character in string literal", self.pos));
                }
                c => {
                    out.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut code = 0;
        for _ in 0..4 {
            match self.peek().and_then(|c| c.to_digit(16)) {
                Some(d) => {
                    code = code * 16 + d;
                    self.pos += 1;
                }
                None => return Err(self.at("Bad Unicode escape", self.pos)),
            }
        }
        Ok(code)
    }

    fn number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        match self.peek() {
            Some('0') => {
                self.pos += 1;
                if matches!(self.peek(), Some('0'..='9')) {
                    return Err(self.at("Unexpected number", self.pos));
                }
            }
            Some('1'..='9') => {
                while matches!(self.peek(), Some('0'..='9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(self.at("No number after minus sign", self.pos)),
        }
        if self.peek() == Some('.') {
            self.pos += 1;
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.at("Unterminated fractional number", self.pos));
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some('+' | '-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.at("Exponent part is missing a number", self.pos));
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.pos += 1;
            }
        }
        let text: String = self.text[start..self.pos].iter().collect();
        Ok(JsonValue::Num(text.parse().unwrap_or(0.0)))
    }
}


impl<T: JsInspect> JsInspect for Box_<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Box"), vec![("value".to_string(), self.value.js_value())])
    }
}

impl JsInspect for HasId {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("id".to_string(), self.id.js_value())])
    }
}

impl<T: JsInspect> JsInspect for LinkedList<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("LinkedList"), vec![("head".to_string(), self.head.js_value()), ("count".to_string(), self.count.js_value())])
    }
}

impl<T: JsInspect> JsInspect for ListNode<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("ListNode"), vec![("value".to_string(), self.value.js_value()), ("next".to_string(), self.next.js_value())])
    }
}

impl JsInspect for NumberStack {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("NumberStack"), vec![("items".to_string(), self.items.js_value())])
    }
}

impl<T: JsInspect> JsInspect for Page<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("items".to_string(), self.items.js_value()), ("total".to_string(), self.total.js_value())])
    }
}

impl<A: JsInspect, B: JsInspect> JsInspect for Pair<A, B> {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Pair"), vec![("first".to_string(), self.first.js_value()), ("second".to_string(), self.second.js_value())])
    }
}

impl JsInspect for RenameT {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("name".to_string(), self.name.js_value())])
    }
}

impl<T: JsInspect> JsInspect for Repo<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Repo"), vec![("items".to_string(), self.items.js_value())])
    }
}

impl<T: JsInspect> JsInspect for Stack<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Stack"), vec![("items".to_string(), self.items.js_value())])
    }
}

impl JsInspect for User {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("id".to_string(), self.id.js_value()), ("name".to_string(), self.name.js_value())])
    }
}

impl<T: JsFromJson + JsInspect> JsFromJson for Page<T> {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        let o = v.object("Page")?;
        Ok(Page {
            items: js_json_field(o, "items")?,
            total: js_json_field(o, "total")?,
        })
    }
}
