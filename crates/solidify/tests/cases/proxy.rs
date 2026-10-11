const PREFIX: &str = "read:";

fn main() -> Result<(), String> {
    // Proxies: reads and writes through traps, the target shared with the proxy.
    let mut user = { let mut o = JsValue::Object(None, Vec::new()); js_set(&mut o, "name", JsValue::Str("Ada".to_string())); js_set(&mut o, "age", JsValue::Int(36)); o };
    let mut validated = JsValue::Proxy(std::rc::Rc::new(JsProxy { target: { let o = &mut user; js_share(o) }, get: None, set: Some(Box::new({ let f = move |mut target: JsValue, key: String, value: JsValue| {
        if &key == "age" && js_typeof(&value) != "number" {
            panic!("{}", "age must be a number".to_string());
        }
        { let key = key; let v = value; { let o = &mut target; js_set(o, &key, v) } };
        true
    }; move |t: JsValue, k: String, v: JsValue, r: JsValue| { let _ = r; f(t, k, v) } })), has: None, delete: None }));
    { let key = "age"; let v = JsValue::Int(37); { let o = &mut validated; js_set(o, &key, v) } };
    println!("{} {} {}", js_show(&js_get(&validated, "age")), js_show(&js_get(&user, "age")), js_show(&user));
    let logged = JsValue::Proxy(std::rc::Rc::new(JsProxy { target: JsValue::Ref(std::rc::Rc::new(std::cell::RefCell::new({ let mut o = JsValue::Object(None, Vec::new()); js_set(&mut o, "x", JsValue::Int(1)); js_set(&mut o, "y", JsValue::Int(2)); o }))), get: Some(Box::new({ let f = move |t: JsValue, k: String| {
        println!("{PREFIX} {k}");
        js_get(&t, &k)
    }; move |t: JsValue, k: String, r: JsValue| { let _ = r; f(t, k) } })), set: None, has: None, delete: None }));
    println!("{}", js_show(&js_op_add(&js_get(&logged, "x"), &js_get(&logged, "y"))));
    let defaults = JsValue::Proxy(std::rc::Rc::new(JsProxy { target: JsValue::Ref(std::rc::Rc::new(std::cell::RefCell::new(JsValue::Object(None, Vec::new())))), get: Some(Box::new({ let f = move |t: JsValue, k: String| { if js_has(&t, &js_key_text(&JsValue::Str(k.clone()))) { js_get(&t, &k) } else { JsValue::Int(0) } }; move |t: JsValue, k: String, r: JsValue| { let _ = r; f(t, k) } })), set: None, has: None, delete: None }));
    println!("{} {}", js_show(&js_get(&defaults, "anything")), js_show(&js_get(&defaults, "other")));
    let mut target = js_json_parse_value("{\"name\":\"ada\"}")?;
    let mut shout = JsValue::Proxy(std::rc::Rc::new(JsProxy { target: { let o = &mut target; js_share(o) }, get: Some(Box::new({ let f = move |t: JsValue, key: String| {
        if js_has(&t, &js_key_text(&JsValue::Str(key.clone()))) { js_get(&t, &key) } else { JsValue::Str(format!("no {key}")) }
    }; move |t: JsValue, k: String, r: JsValue| { let _ = r; f(t, k) } })), set: Some(Box::new({ let f = move |mut t: JsValue, key: String, value: JsValue| {
        { let key = key; let v = if js_typeof(&value) == "string" { js_call_method(&value, "toUpperCase", vec![]) } else { value }; { let o = &mut t; js_set(o, &key, v) } };
        true
    }; move |t: JsValue, k: String, v: JsValue, r: JsValue| { let _ = r; f(t, k, v) } })), has: Some(Box::new({ let f = move |t: JsValue, key: String| key.starts_with("n"); move |t: JsValue, k: String| {  f(t, k) } })), delete: None }));
    { let key = "city"; let v = JsValue::Str("paris".to_string()); { let o = &mut shout; js_set(o, &key, v) } };
    println!("{} {} {} {}", js_show(&js_get(&shout, "name")), js_show(&js_get(&shout, "age")), js_show(&js_get(&shout, "city")), js_show(&js_get(&target, "city")));
    println!("{} {} {}", js_has(&shout, &js_key_text(&JsValue::Str("name".to_string()))), js_has(&shout, &js_key_text(&JsValue::Str("city".to_string()))), js_inspect(&js_keys(&target).js_value()));
    let mut record = js_json_parse_value("{\"a\":1,\"b\":2}")?;
    let _ = { let key = "a".to_string(); { let o = &mut record; js_delete(o, &key) } };
    println!("{}", js_show(&record));
    let mut scores = JsMap::from([("x".to_string(), 1), ("y".to_string(), 2)]);
    let _ = { scores.remove(&"x".to_string()); true };
    println!("{}", js_inspect(&JsValue::Object(None, scores.iter().map(|(k, v)| (k.to_string(), v.js_value())).collect())));
    Ok(())
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


/// The exact decimal digits of a finite double (no sign) and the position of the point:
/// `digits[..point]` is the integer part (`point` may be negative or past the end).
#[allow(dead_code)]
fn js_exact_digits(x: f64) -> (Vec<u8>, i64) {
    let text = format!("{:.1100}", x.abs());
    let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|b| b - b'0').collect();
    let mut point = int.len() as i64;
    while digits.len() > 1 && digits[0] == 0 {
        digits.remove(0);
        point -= 1;
    }
    while digits.len() > 1 && digits[digits.len() - 1] == 0 {
        digits.pop();
    }
    (digits, point)
}

/// Rounds `digits` to `keep` digits, a tie going up as JavaScript does; returns the digits
/// and whether a carry added one in front.
#[allow(dead_code)]
fn js_round_digits(digits: &[u8], keep: usize) -> (Vec<u8>, bool) {
    let mut out: Vec<u8> = digits.iter().take(keep).copied().collect();
    while out.len() < keep {
        out.push(0);
    }
    if digits.get(keep).is_some_and(|d| *d >= 5) {
        let mut i = out.len();
        loop {
            if i == 0 {
                out.insert(0, 1);
                return (out, true);
            }
            i -= 1;
            if out[i] == 9 {
                out[i] = 0;
            } else {
                out[i] += 1;
                break;
            }
        }
    }
    (out, false)
}

/// `x.toFixed(digits)`, as JavaScript writes it.
#[allow(dead_code)]
fn js_to_fixed(x: f64, digits: i64) -> String {
    if !x.is_finite() || x.abs() >= 1e21 {
        return js_number_text(x);
    }
    let f = digits.clamp(0, 100) as usize;
    let (all, point) = js_exact_digits(x);
    // The digits from the units on: zeros in front when the number is below one.
    let mut padded: Vec<u8> = Vec::new();
    if point <= 0 {
        padded.push(0);
        padded.extend(std::iter::repeat_n(0, (-point) as usize));
        padded.extend(&all);
    } else {
        padded.extend(&all);
    }
    let int_len = if point <= 0 { 1 } else { point as usize };
    let (rounded, carry) = js_round_digits(&padded, int_len + f);
    let int_len = int_len + carry as usize;
    let int: String = rounded[..int_len].iter().map(|d| (d + b'0') as char).collect();
    let frac: String = rounded[int_len..].iter().map(|d| (d + b'0') as char).collect();
    let body = if f == 0 { int } else { format!("{int}.{frac}") };
    if x < 0.0 { format!("-{body}") } else { body }
}

/// `x.toPrecision(p)`, as JavaScript writes it.
#[allow(dead_code)]
fn js_to_precision(x: f64, p: i64) -> String {
    if !x.is_finite() {
        return js_number_text(x);
    }
    let p = p.clamp(1, 100) as usize;
    let sign = if x < 0.0 { "-" } else { "" };
    if x == 0.0 {
        let zeros = "0".repeat(p - 1);
        return if p == 1 { "0".into() } else { format!("0.{zeros}") };
    }
    let (all, point) = js_exact_digits(x);
    let (rounded, carry) = js_round_digits(&all, p);
    let digits: String = rounded[..p].iter().map(|d| (d + b'0') as char).collect();
    let e = point - 1 + carry as i64;
    if e < -6 || e >= p as i64 {
        let (first, rest) = digits.split_at(1);
        let mantissa = if rest.is_empty() {
            first.to_string()
        } else {
            format!("{first}.{rest}")
        };
        let es = if e < 0 { "-" } else { "+" };
        return format!("{sign}{mantissa}e{es}{}", e.abs());
    }
    if e >= 0 {
        let (int, frac) = digits.split_at(e as usize + 1);
        if frac.is_empty() {
            format!("{sign}{int}")
        } else {
            format!("{sign}{int}.{frac}")
        }
    } else {
        format!("{sign}0.{}{digits}", "0".repeat((-e - 1) as usize))
    }
}


/// A double as JavaScript's bitwise operators see it: a 32-bit integer.
fn js_to_int32(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    (x.trunc().rem_euclid(4294967296.0) as u32) as i32
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
impl<T: JsFromJson> JsFromJson for std::rc::Rc<std::cell::RefCell<T>> {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        T::from_json(v).map(|x| std::rc::Rc::new(std::cell::RefCell::new(x)))
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


/// A value known only at run time (a `JSON.parse` result without a type, what a reviver,
/// `eval` or a proxy handler handles): JavaScript's own rules, on `JsValue`.
impl JsInspect for JsValue {
    fn js_value(&self) -> JsValue {
        self.clone()
    }
}

#[allow(dead_code)]
impl JsValue {
    /// `if (v)`.
    fn truthy(&self) -> bool {
        match self {
            JsValue::Int(n) => *n != 0,
            JsValue::Float(x) => *x != 0.0 && !x.is_nan(),
            JsValue::Str(s) => !s.is_empty(),
            JsValue::Bool(b) => *b,
            JsValue::Null | JsValue::Undefined => false,
            _ => true,
        }
    }

    /// `Number(v)`.
    fn to_number(&self) -> f64 {
        if let Some(inner) = self.shown() {
            return inner.to_number();
        }
        match self {
            JsValue::Int(n) => *n as f64,
            JsValue::Float(x) => *x,
            JsValue::Bool(b) => *b as i64 as f64,
            JsValue::Null => 0.0,
            JsValue::Str(s) => {
                let t = s.trim();
                if t.is_empty() {
                    0.0
                } else if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                    i64::from_str_radix(h, 16).map_or(f64::NAN, |n| n as f64)
                } else {
                    match t {
                        "Infinity" | "+Infinity" => f64::INFINITY,
                        "-Infinity" => f64::NEG_INFINITY,
                        _ if t.chars().all(|c| c.is_ascii_digit() || "+-.eE".contains(c)) => {
                            t.parse().unwrap_or(f64::NAN)
                        }
                        _ => f64::NAN,
                    }
                }
            }
            JsValue::Array(items) if items.is_empty() => 0.0,
            JsValue::Array(items) if items.len() == 1 => items[0].to_number(),
            JsValue::Date(iso, _) => js_date_ms(iso),
            _ => f64::NAN,
        }
    }

    /// A number, as an integer when it is one.
    fn number(x: f64) -> JsValue {
        if x.fract() == 0.0 && x.abs() < 9007199254740992.0 && !(x == 0.0 && x.is_sign_negative())
        {
            JsValue::Int(x as i64)
        } else {
            JsValue::Float(x)
        }
    }

    fn is_nullish(&self) -> bool {
        matches!(self, JsValue::Null | JsValue::Undefined)
    }
}

/// The milliseconds of an ISO date text.
#[allow(dead_code)]
fn js_date_ms(iso: &str) -> f64 {
    let digits: Vec<i64> = iso
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect();
    if digits.len() < 3 {
        return f64::NAN;
    }
    let (y, m, d) = (digits[0], digits[1], digits[2]);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    let part = |i: usize| digits.get(i).copied().unwrap_or(0);
    (days * 86_400_000 + part(3) * 3_600_000 + part(4) * 60_000 + part(5) * 1000 + part(6))
        as f64
}

/// `typeof v`.
#[allow(dead_code)]
fn js_typeof(v: &JsValue) -> &'static str {
    match v {
        JsValue::Int(_) | JsValue::Float(_) => "number",
        JsValue::Str(_) => "string",
        JsValue::Bool(_) => "boolean",
        JsValue::Undefined => "undefined",
        JsValue::Func(_) => "function",
        _ => "object",
    }
}

/// A value as `+` sees it: a string for objects and arrays.
#[allow(dead_code)]
fn js_primitive(v: &JsValue) -> JsValue {
    match v {
        JsValue::Array(_) | JsValue::Object(..) | JsValue::Map(_) | JsValue::Set(_)
        | JsValue::Func(_) | JsValue::Raw(_) | JsValue::Date(..) | JsValue::Ref(_)
        | JsValue::Proxy(_) => JsValue::Str(js_to_string(v)),
        other => other.clone(),
    }
}

/// `a + b`.
#[allow(dead_code)]
fn js_op_add(a: &JsValue, b: &JsValue) -> JsValue {
    let (a, b) = (js_primitive(a), js_primitive(b));
    match (&a, &b) {
        (JsValue::Str(_), _) | (_, JsValue::Str(_)) => {
            JsValue::Str(format!("{}{}", js_to_string(&a), js_to_string(&b)))
        }
        (JsValue::Int(x), JsValue::Int(y)) => match x.checked_add(*y) {
            Some(n) if n.abs() < 9007199254740992 => JsValue::Int(n),
            _ => JsValue::Float(*x as f64 + *y as f64),
        },
        _ => JsValue::number(a.to_number() + b.to_number()),
    }
}

/// `a - b`, `a * b`, `a / b`, `a % b`, `a ** b`.
#[allow(dead_code)]
fn js_op_num(op: &str, a: &JsValue, b: &JsValue) -> JsValue {
    let (x, y) = (a.to_number(), b.to_number());
    let r = match op {
        "-" => x - y,
        "*" => x * y,
        "/" => x / y,
        "%" => x % y,
        _ => x.powf(y),
    };
    if op == "/" && !(r.fract() == 0.0 && r.is_finite()) {
        return JsValue::Float(r);
    }
    JsValue::number(r)
}

/// `a < b`, `a <= b`, `a > b`, `a >= b`.
#[allow(dead_code)]
fn js_op_cmp(op: &str, a: &JsValue, b: &JsValue) -> bool {
    let (a, b) = (js_primitive(a), js_primitive(b));
    if let (JsValue::Str(x), JsValue::Str(y)) = (&a, &b) {
        return match op {
            "<" => x < y,
            "<=" => x <= y,
            ">" => x > y,
            _ => x >= y,
        };
    }
    let (x, y) = (a.to_number(), b.to_number());
    match op {
        "<" => x < y,
        "<=" => x <= y,
        ">" => x > y,
        _ => x >= y,
    }
}

/// `a === b`.
#[allow(dead_code)]
fn js_strict_eq(a: &JsValue, b: &JsValue) -> bool {
    match (a, b) {
        (JsValue::Ref(x), JsValue::Ref(y)) => std::rc::Rc::ptr_eq(x, y),
        (JsValue::Proxy(x), JsValue::Proxy(y)) => std::rc::Rc::ptr_eq(x, y),
        (JsValue::Ref(_) | JsValue::Proxy(_), _) | (_, JsValue::Ref(_) | JsValue::Proxy(_)) => false,
        (JsValue::Int(_) | JsValue::Float(_), JsValue::Int(_) | JsValue::Float(_)) => {
            a.to_number() == b.to_number()
        }
        _ => a == b,
    }
}

/// `a == b`: null and undefined are equal, numbers and strings compare as numbers.
#[allow(dead_code)]
fn js_loose_eq(a: &JsValue, b: &JsValue) -> bool {
    match (a, b) {
        (x, y) if x.is_nullish() || y.is_nullish() => x.is_nullish() && y.is_nullish(),
        (JsValue::Str(_), JsValue::Int(_) | JsValue::Float(_))
        | (JsValue::Int(_) | JsValue::Float(_), JsValue::Str(_))
        | (JsValue::Bool(_), _)
        | (_, JsValue::Bool(_)) => a.to_number() == b.to_number(),
        _ => js_strict_eq(a, b),
    }
}

/// `v.key`, `v[key]`.
#[allow(dead_code)]
fn js_get(v: &JsValue, key: &str) -> JsValue {
    let index = key.parse::<usize>().ok();
    match v {
        JsValue::Ref(cell) => js_get(&cell.borrow(), key),
        JsValue::Proxy(p) => match &p.get {
            Some(trap) => trap(p.target.clone(), key.to_string(), v.clone()),
            None => js_get(&p.target, key),
        },
        JsValue::Object(_, props) => props
            .iter()
            .find(|(k, _)| k == key)
            .map_or(JsValue::Undefined, |(_, x)| x.clone()),
        JsValue::Array(items) => match (key, index) {
            ("length", _) => JsValue::Int(items.len() as i64),
            (_, Some(i)) => items.get(i).cloned().unwrap_or(JsValue::Undefined),
            _ => JsValue::Undefined,
        },
        JsValue::Str(s) => match (key, index) {
            ("length", _) => JsValue::Int(s.chars().count() as i64),
            (_, Some(i)) => s
                .chars()
                .nth(i)
                .map_or(JsValue::Undefined, |c| JsValue::Str(c.to_string())),
            _ => JsValue::Undefined,
        },
        JsValue::Map(entries) if key == "size" => JsValue::Int(entries.len() as i64),
        JsValue::Set(items) if key == "size" => JsValue::Int(items.len() as i64),
        JsValue::Null | JsValue::Undefined => {
            panic!("TypeError: Cannot read properties of {} (reading '{key}')", js_to_string(v))
        }
        _ => JsValue::Undefined,
    }
}

/// `v[k]` for any key.
#[allow(dead_code)]
fn js_index(v: &JsValue, k: &JsValue) -> JsValue {
    js_get(v, &js_key_text(k))
}

/// A key as a property name: `1` and `"1"` are the same.
#[allow(dead_code)]
fn js_key_text(k: &JsValue) -> String {
    js_to_string(k)
}

/// `v.key = value`.
#[allow(dead_code)]
fn js_set(v: &mut JsValue, key: &str, value: JsValue) {
    match v {
        JsValue::Ref(cell) => js_set(&mut cell.borrow_mut(), key, value),
        JsValue::Proxy(p) => match &p.set {
            Some(trap) => {
                let receiver = JsValue::Proxy(p.clone());
                if !trap(p.target.clone(), key.to_string(), value, receiver) {
                    panic!("TypeError: 'set' on proxy: trap returned falsish for property '{key}'");
                }
            }
            None => {
                let mut target = p.target.clone();
                js_set(&mut target, key, value);
            }
        },
        JsValue::Object(_, props) => match props.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => props.push((key.to_string(), value)),
        },
        JsValue::Array(items) => {
            if let Ok(i) = key.parse::<usize>() {
                if i >= items.len() {
                    items.resize(i + 1, JsValue::Undefined);
                }
                items[i] = value;
            } else if key == "length" {
                items.resize(value.to_number() as usize, JsValue::Undefined);
            }
        }
        _ => {}
    }
}

/// `key in v`.
#[allow(dead_code)]
fn js_has(v: &JsValue, key: &str) -> bool {
    match v {
        JsValue::Ref(cell) => js_has(&cell.borrow(), key),
        JsValue::Proxy(p) => match &p.has {
            Some(trap) => trap(p.target.clone(), key.to_string()),
            None => js_has(&p.target, key),
        },
        JsValue::Object(_, props) => props.iter().any(|(k, _)| k == key),
        JsValue::Array(items) => {
            key == "length" || key.parse::<usize>().is_ok_and(|i| i < items.len())
        }
        _ => false,
    }
}

/// `delete v[key]`.
#[allow(dead_code)]
fn js_delete(v: &mut JsValue, key: &str) -> bool {
    match v {
        JsValue::Ref(cell) => js_delete(&mut cell.borrow_mut(), key),
        JsValue::Proxy(p) => match &p.delete {
            Some(trap) => trap(p.target.clone(), key.to_string()),
            None => js_delete(&mut p.target.clone(), key),
        },
        JsValue::Object(_, props) => {
            props.retain(|(k, _)| k != key);
            true
        }
        _ => true,
    }
}

/// `v.name(args)`: the methods of strings, arrays and numbers.
#[allow(dead_code)]
fn js_call_method(v: &JsValue, name: &str, args: Vec<JsValue>) -> JsValue {
    if let JsValue::Ref(cell) = v {
        return js_call_method(&cell.borrow(), name, args);
    }
    let arg = |i: usize| args.get(i).cloned().unwrap_or(JsValue::Undefined);
    let int_arg = |i: usize, default: i64| {
        args.get(i)
            .filter(|a| !matches!(a, JsValue::Undefined))
            .map_or(default, |a| a.to_number() as i64)
    };
    match (v, name) {
        (_, "toString") if args.is_empty() => JsValue::Str(js_to_string(v)),
        (JsValue::Str(s), _) => {
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as i64;
            let at = |i: i64| -> usize {
                (if i < 0 { (len + i).max(0) } else { i.min(len) }) as usize
            };
            match name {
                "toUpperCase" => JsValue::Str(s.to_uppercase()),
                "toLowerCase" => JsValue::Str(s.to_lowercase()),
                "trim" => JsValue::Str(s.trim().to_string()),
                "trimStart" => JsValue::Str(s.trim_start().to_string()),
                "trimEnd" => JsValue::Str(s.trim_end().to_string()),
                "includes" => JsValue::Bool(s.contains(&js_to_string(&arg(0)))),
                "startsWith" => JsValue::Bool(s.starts_with(&js_to_string(&arg(0)))),
                "endsWith" => JsValue::Bool(s.ends_with(&js_to_string(&arg(0)))),
                "indexOf" => JsValue::Int(
                    s.find(&js_to_string(&arg(0)))
                        .map_or(-1, |b| s[..b].chars().count() as i64),
                ),
                "charAt" => JsValue::Str(
                    chars
                        .get(int_arg(0, 0) as usize)
                        .map_or(String::new(), |c| c.to_string()),
                ),
                "slice" | "substring" => {
                    let (a, b) = (at(int_arg(0, 0)), at(int_arg(1, len)));
                    JsValue::Str(chars[a.min(b)..b.max(a)].iter().collect())
                }
                "repeat" => JsValue::Str(s.repeat(int_arg(0, 0).max(0) as usize)),
                "split" => {
                    let sep = js_to_string(&arg(0));
                    let parts: Vec<JsValue> = if sep.is_empty() {
                        chars.iter().map(|c| JsValue::Str(c.to_string())).collect()
                    } else {
                        s.split(sep.as_str()).map(|p| JsValue::Str(p.to_string())).collect()
                    };
                    JsValue::Array(parts)
                }
                "padStart" | "padEnd" => {
                    let width = int_arg(0, 0).max(0) as usize;
                    let fill = match arg(1) {
                        JsValue::Undefined => " ".to_string(),
                        f => js_to_string(&f),
                    };
                    if chars.len() >= width || fill.is_empty() {
                        return JsValue::Str(s.clone());
                    }
                    let pad: String = fill.chars().cycle().take(width - chars.len()).collect();
                    JsValue::Str(if name == "padStart" {
                        format!("{pad}{s}")
                    } else {
                        format!("{s}{pad}")
                    })
                }
                _ => panic!("TypeError: \"{s}\".{name} is not a function"),
            }
        }
        (JsValue::Array(items), _) => match name {
            "join" => {
                let sep = match arg(0) {
                    JsValue::Undefined => ",".to_string(),
                    s => js_to_string(&s),
                };
                JsValue::Str(
                    items
                        .iter()
                        .map(|i| if i.is_nullish() { String::new() } else { js_to_string(i) })
                        .collect::<Vec<_>>()
                        .join(&sep),
                )
            }
            "includes" => JsValue::Bool(items.iter().any(|i| js_strict_eq(i, &arg(0)))),
            "indexOf" => JsValue::Int(
                items
                    .iter()
                    .position(|i| js_strict_eq(i, &arg(0)))
                    .map_or(-1, |p| p as i64),
            ),
            "slice" => {
                let len = items.len() as i64;
                let at = |i: i64| (if i < 0 { (len + i).max(0) } else { i.min(len) }) as usize;
                let (a, b) = (at(int_arg(0, 0)), at(int_arg(1, len)));
                JsValue::Array(if a < b { items[a..b].to_vec() } else { Vec::new() })
            }
            "at" => {
                let i = int_arg(0, 0);
                let i = if i < 0 { items.len() as i64 + i } else { i };
                items.get(i as usize).cloned().unwrap_or(JsValue::Undefined)
            }
            _ => panic!("TypeError: an array's {name} is not supported here"),
        },
        (JsValue::Int(_) | JsValue::Float(_), "toFixed") => {
            JsValue::Str(js_to_fixed(v.to_number(), int_arg(0, 0)))
        }
        _ => panic!("TypeError: {}.{name} is not a function", js_to_string(v)),
    }
}

/// A JSON document as a value.
#[allow(dead_code)]
fn js_from_json_value(v: &JsonValue) -> JsValue {
    match v {
        JsonValue::Null => JsValue::Null,
        JsonValue::Bool(b) => JsValue::Bool(*b),
        JsonValue::Num(x) => JsValue::number(*x),
        JsonValue::Str(s) => JsValue::Str(s.clone()),
        JsonValue::Array(items) => JsValue::Array(items.iter().map(js_from_json_value).collect()),
        JsonValue::Object(props) => JsValue::Object(
            None,
            props
                .iter()
                .map(|(k, x)| (k.clone(), js_from_json_value(x)))
                .collect(),
        ),
    }
}

/// A value as the JSON a typed reader takes.
#[allow(dead_code)]
fn js_to_json_value(v: &JsValue) -> JsonValue {
    if let Some(inner) = v.shown() {
        return js_to_json_value(&inner);
    }
    match v {
        JsValue::Null | JsValue::Undefined | JsValue::Func(_) => JsonValue::Null,
        JsValue::Bool(b) => JsonValue::Bool(*b),
        JsValue::Int(_) | JsValue::Float(_) => JsonValue::Num(v.to_number()),
        JsValue::Str(s) | JsValue::Raw(s) => JsonValue::Str(s.clone()),
        JsValue::Date(iso, _) => JsonValue::Str(iso.clone()),
        JsValue::Array(items) | JsValue::Set(items) => {
            JsonValue::Array(items.iter().map(js_to_json_value).collect())
        }
        JsValue::Object(_, props) => JsonValue::Object(
            props
                .iter()
                .filter(|(_, x)| !matches!(x, JsValue::Undefined))
                .map(|(k, x)| (k.clone(), js_to_json_value(x)))
                .collect(),
        ),
        JsValue::Map(entries) => JsonValue::Object(
            entries
                .iter()
                .map(|(k, x)| (js_to_string(k), js_to_json_value(x)))
                .collect(),
        ),
        JsValue::Ref(_) | JsValue::Proxy(_) => unreachable!("shown above"),
    }
}

/// A value known at run time where a type is expected: read as that type.
#[allow(dead_code)]
fn js_into<T: JsFromJson>(v: &JsValue) -> T {
    T::from_json(&js_to_json_value(v)).unwrap_or_else(|e| panic!("TypeError: {e}"))
}

/// `JSON.parse(text)` without a type: a value known at run time.
#[allow(dead_code)]
fn js_json_parse_value(text: &str) -> Result<JsValue, String> {
    js_json_parse(text).map(|v| js_from_json_value(&v))
}

/// `JSON.parse(text, reviver)`: each value, innermost first, through the reviver (a
/// property whose reviver gives `undefined` is dropped).
#[allow(dead_code)]
fn js_json_parse_with(text: &str, reviver: &dyn Fn(&str, JsValue) -> JsValue) -> Result<JsValue, String> {
    fn walk(key: &str, v: JsValue, reviver: &dyn Fn(&str, JsValue) -> JsValue) -> JsValue {
        let v = match v {
            JsValue::Array(items) => JsValue::Array(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| walk(&i.to_string(), x, reviver))
                    .collect(),
            ),
            JsValue::Object(c, props) => JsValue::Object(
                c,
                props
                    .into_iter()
                    .map(|(k, x)| {
                        let x = walk(&k, x, reviver);
                        (k, x)
                    })
                    .filter(|(_, x)| !matches!(x, JsValue::Undefined))
                    .collect(),
            ),
            other => other,
        };
        reviver(key, v)
    }
    let v = js_json_parse_value(text)?;
    Ok(walk("", v, reviver))
}

/// `JSON.stringify(value, replacer, indent)`: each value, outermost first, through the
/// replacer (a property it turns into `undefined` is left out).
#[allow(dead_code)]
fn js_json_stringify_with(
    v: &JsValue,
    replacer: &dyn Fn(&str, JsValue) -> JsValue,
    indent: &str,
) -> String {
    fn walk(key: &str, v: JsValue, replacer: &dyn Fn(&str, JsValue) -> JsValue) -> JsValue {
        match replacer(key, v) {
            JsValue::Array(items) => JsValue::Array(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| match walk(&i.to_string(), x, replacer) {
                        JsValue::Undefined => JsValue::Null,
                        y => y,
                    })
                    .collect(),
            ),
            JsValue::Object(c, props) => JsValue::Object(
                c,
                props
                    .into_iter()
                    .map(|(k, x)| {
                        let x = walk(&k, x, replacer);
                        (k, x)
                    })
                    .collect(),
            ),
            other => other,
        }
    }
    js_json_stringify(&walk("", v.clone(), replacer), indent)
}

/// `JSON.stringify(value, ["a", "b"], indent)`: only these properties, at every level.
#[allow(dead_code)]
fn js_json_stringify_keys(v: &JsValue, keys: &[&str], indent: &str) -> String {
    fn keep(v: &JsValue, keys: &[&str]) -> JsValue {
        match v {
            JsValue::Array(items) => JsValue::Array(items.iter().map(|x| keep(x, keys)).collect()),
            JsValue::Object(c, props) => JsValue::Object(
                *c,
                keys.iter()
                    .filter_map(|k| {
                        props
                            .iter()
                            .find(|(p, _)| p == k)
                            .map(|(p, x)| (p.clone(), keep(x, keys)))
                    })
                    .collect(),
            ),
            other => other.clone(),
        }
    }
    js_json_stringify(&keep(v, keys), indent)
}

impl std::fmt::Display for JsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&js_to_string(self))
    }
}

/// `for (const x of v)`: the items of an array or a set, the characters of a text.
#[allow(dead_code)]
fn js_items(v: &JsValue) -> Vec<JsValue> {
    if let Some(inner) = v.shown() {
        return js_items(&inner);
    }
    match v {
        JsValue::Array(items) | JsValue::Set(items) => items.clone(),
        JsValue::Str(s) => s.chars().map(|c| JsValue::Str(c.to_string())).collect(),
        JsValue::Map(entries) => entries
            .iter()
            .map(|(k, x)| JsValue::Array(vec![k.clone(), x.clone()]))
            .collect(),
        _ => panic!("TypeError: {} is not iterable", js_to_string(v)),
    }
}

/// `for (const k in v)`, `Object.keys(v)`.
#[allow(dead_code)]
fn js_keys(v: &JsValue) -> Vec<String> {
    if let Some(inner) = v.shown() {
        return js_keys(&inner);
    }
    match v {
        JsValue::Object(_, props) => props.iter().map(|(k, _)| k.clone()).collect(),
        JsValue::Array(items) => (0..items.len()).map(|i| i.to_string()).collect(),
        JsValue::Str(s) => (0..s.chars().count()).map(|i| i.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// `v.name(args)` for the methods that change an array in place.
#[allow(dead_code)]
fn js_call_method_mut(v: &mut JsValue, name: &str, args: Vec<JsValue>) -> JsValue {
    if let JsValue::Ref(cell) = v {
        let cell = cell.clone();
        return js_call_method_mut(&mut cell.borrow_mut(), name, args);
    }
    let JsValue::Array(items) = v else {
        return js_call_method(v, name, args);
    };
    match name {
        "push" => {
            items.extend(args);
            JsValue::Int(items.len() as i64)
        }
        "unshift" => {
            items.splice(0..0, args);
            JsValue::Int(items.len() as i64)
        }
        "pop" => items.pop().unwrap_or(JsValue::Undefined),
        "shift" => {
            if items.is_empty() {
                JsValue::Undefined
            } else {
                items.remove(0)
            }
        }
        "reverse" => {
            items.reverse();
            v.clone()
        }
        // Without a comparison: as texts.
        "sort" => {
            items.sort_by_key(|x| js_to_string(x));
            v.clone()
        }
        "fill" => {
            let x = args.first().cloned().unwrap_or(JsValue::Undefined);
            for slot in items.iter_mut() {
                *slot = x.clone();
            }
            v.clone()
        }
        _ => {
            let len = items.len() as i64;
            let at = |i: i64| (if i < 0 { (len + i).max(0) } else { i.min(len) }) as usize;
            let start = at(args.first().map_or(0, |a| a.to_number() as i64));
            let count = args
                .get(1)
                .map_or(items.len() - start, |c| (c.to_number().max(0.0) as usize).min(items.len() - start));
            let removed: Vec<JsValue> = items
                .splice(start..start + count, args.into_iter().skip(2))
                .collect();
            JsValue::Array(removed)
        }
    }
}

/// `v.key` as a place to write: a missing property is added, an array grows.
#[allow(dead_code)]
fn js_get_mut<'v>(v: &'v mut JsValue, key: &str) -> &'v mut JsValue {
    match v {
        JsValue::Object(_, props) => {
            let i = match props.iter().position(|(k, _)| k == key) {
                Some(i) => i,
                None => {
                    props.push((key.to_string(), JsValue::Undefined));
                    props.len() - 1
                }
            };
            &mut props[i].1
        }
        JsValue::Array(items) => {
            let Ok(i) = key.parse::<usize>() else {
                panic!("TypeError: cannot set the property '{key}' of an array");
            };
            if i >= items.len() {
                items.resize(i + 1, JsValue::Undefined);
            }
            &mut items[i]
        }
        _ => panic!("TypeError: Cannot set properties of {} (setting '{key}')", js_to_string(v)),
    }
}

/// `place op= value`.
#[allow(dead_code)]
fn js_op_assign(place: &mut JsValue, op: &str, value: JsValue) -> JsValue {
    let new = match op {
        "+" => js_op_add(place, &value),
        "??" if place.is_nullish() => value,
        "||" if !place.truthy() => value,
        "&&" if place.truthy() => value,
        "??" | "||" | "&&" => place.clone(),
        "&" | "|" | "^" | "<<" | ">>" | ">>>" => {
            let (a, b) = (js_to_int32(place.to_number()), js_to_int32(value.to_number()));
            JsValue::Int(match op {
                "&" => (a & b) as i64,
                "|" => (a | b) as i64,
                "^" => (a ^ b) as i64,
                "<<" => a.wrapping_shl(b as u32) as i64,
                ">>" => a.wrapping_shr(b as u32) as i64,
                _ => (a as u32).wrapping_shr(b as u32) as i64,
            })
        }
        _ => js_op_num(op, place, &value),
    };
    *place = new.clone();
    new
}

/// `Object.values(v)`.
#[allow(dead_code)]
fn js_values(v: &JsValue) -> Vec<JsValue> {
    if let JsValue::Proxy(_) = v {
        return js_keys(v).iter().map(|k| js_get(v, k)).collect();
    }
    js_keys(v).iter().map(|k| js_get(v, k)).collect()
}

/// `Object.entries(v)`.
#[allow(dead_code)]
fn js_entries(v: &JsValue) -> Vec<(String, JsValue)> {
    js_keys(v)
        .into_iter()
        .map(|k| {
            let x = js_get(v, &k);
            (k, x)
        })
        .collect()
}

/// Changes what a path inside `v` names (`data.items[0]`), through shared objects and
/// proxies.
#[allow(dead_code)]
fn js_with_mut<R>(v: &mut JsValue, path: &[String], f: impl FnOnce(&mut JsValue) -> R) -> R {
    match v {
        JsValue::Ref(cell) => {
            let cell = cell.clone();
            let mut inner = cell.borrow_mut();
            js_with_mut(&mut inner, path, f)
        }
        // Through a proxy: what its `get` gives, changed (shared when it is an object).
        JsValue::Proxy(_) if !path.is_empty() => {
            let mut child = js_get(v, &path[0]);
            js_with_mut(&mut child, &path[1..], f)
        }
        _ => match path.split_first() {
            None => f(v),
            Some((k, rest)) => js_with_mut(js_get_mut(v, k), rest, f),
        },
    }
}

/// `v.key op= value`: the new value.
#[allow(dead_code)]
fn js_op_assign_prop(v: &mut JsValue, key: &str, op: &str, value: JsValue) -> JsValue {
    let mut current = js_get(v, key);
    let new = js_op_assign(&mut current, op, value);
    js_set(v, key, new.clone());
    new
}

/// `list.sort(compare)` in place.
#[allow(dead_code)]
fn js_sort_by(v: &mut JsValue, compare: &dyn Fn(JsValue, JsValue) -> f64) -> JsValue {
    if let JsValue::Ref(cell) = v {
        let cell = cell.clone();
        js_sort_by(&mut cell.borrow_mut(), compare);
        return v.clone();
    }
    if let JsValue::Array(items) = v {
        items.sort_by(|a, b| {
            compare(a.clone(), b.clone())
                .partial_cmp(&0.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    v.clone()
}

/// The object a proxy stands for, shared with it: a value held in a variable becomes a
/// shared object, changed for both.
#[allow(dead_code)]
fn js_share(v: &mut JsValue) -> JsValue {
    if matches!(v, JsValue::Ref(_) | JsValue::Proxy(_)) {
        return v.clone();
    }
    let shared = JsValue::Ref(std::rc::Rc::new(std::cell::RefCell::new(std::mem::replace(
        v,
        JsValue::Undefined,
    ))));
    *v = shared.clone();
    shared
}


impl<K: JsInspect + std::hash::Hash + Eq + Clone, V: JsInspect> JsInspect for JsMap<K, V> {
    fn js_value(&self) -> JsValue {
        JsValue::Map(self.iter().map(|(k, v)| (k.js_value(), v.js_value())).collect())
    }
}

impl<V: JsFromJson> JsFromJson for JsMap<String, V> {
    fn from_json(v: &JsonValue) -> Result<Self, String> {
        let mut out = JsMap::new();
        for (k, x) in v.object("an object")? {
            out.insert(k.clone(), V::from_json(x).map_err(|e| format!("{e} (in \"{k}\")"))?);
        }
        Ok(out)
    }
}
