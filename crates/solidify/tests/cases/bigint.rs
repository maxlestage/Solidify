use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub id: JsBigInt,
}

/// BigInt: integers of any size.
fn factorial(n: &JsBigInt) -> JsBigInt {
    let mut r = JsBigInt::from_i64(1);
    let mut i = JsBigInt::from_i64(2);
    while &i <= n {
        r = r.times(&i);
        i = i.plus(&JsBigInt::from_i64(1));
    }
    r
}

fn main() {
    println!("{}n", factorial(&JsBigInt::from_i64(30)));
    println!("25! = {}", factorial(&JsBigInt::from_i64(25)));
    let big = JsBigInt::from_i64(2).pow(&JsBigInt::from_i64(128)).minus(&JsBigInt::from_i64(1));
    println!("{}n {} {} {}n", big, "bigint", big.to_string_radix(16 as u32), big.rem(&JsBigInt::from_i64(1000)));
    let a = JsBigInt::parse("123456789012345678901234567890");
    let b = JsBigInt::from_i64(42);
    println!("{}n {}n {}n {} {} {} {}", a.div(&b), a.rem(&b), b.negated(), &b.times(&b) > &JsBigInt::from_i64(1000), b.to_f64() < 100.0, &b == &JsBigInt::from_i64(42), &a == &a);
    println!("{} {} {} {} {}n", js_inspect(&(vec![JsBigInt::from_i64(1), JsBigInt::from_i64(2), JsBigInt::from_i64(3)]).js_value()), js_inspect(&(Object { id: JsBigInt::from_i64(7) }).js_value()), js_number_text(b.to_f64() + 0.5), format!("{b}"), JsBigInt::from_i64(9007199254740991_i64).plus(&JsBigInt::from_i64(2)));
    let mut x = JsBigInt::from_i64(10);
    x = x.plus(&JsBigInt::from_i64(5));
    x = x.minus(&JsBigInt::from_i64(1));
    println!("{}n {}n {}n {}n {}n", x, x.bitwise(&JsBigInt::parse("0xff"), '&'), x.shift_left((&JsBigInt::from_i64(3)).to_i64()), x.negated().shift_left(-(&JsBigInt::from_i64(2)).to_i64()), x.bitwise(&JsBigInt::from_i64(7), '^'));
    let mut ids: HashMap<JsBigInt, String> = HashMap::new();
    ids.insert(JsBigInt::from_i64(1), "one".to_string());
    println!("{} {}", ids.get(&JsBigInt::from_i64(1)).cloned().as_ref().map_or("undefined".to_string(), |v| v.to_string()), ids.contains_key(&JsBigInt::from_i64(2)));
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


/// `bigint`: an integer of any size (sign and magnitude, 32-bit limbs, lowest first).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
struct JsBigInt {
    neg: bool,
    mag: Vec<u32>,
}

#[allow(dead_code)]
impl JsBigInt {
    fn normalized(mut self) -> JsBigInt {
        while self.mag.last() == Some(&0) {
            self.mag.pop();
        }
        if self.mag.is_empty() {
            self.neg = false;
        }
        self
    }

    fn from_i64(v: i64) -> JsBigInt {
        let m = v.unsigned_abs();
        JsBigInt { neg: v < 0, mag: vec![m as u32, (m >> 32) as u32] }.normalized()
    }

    /// `BigInt(x)` of a number: it must be an integer.
    fn from_f64(x: f64) -> JsBigInt {
        if x.fract() != 0.0 || !x.is_finite() {
            panic!("RangeError: The number {} cannot be converted to a BigInt because it is not an integer", js_number_text(x));
        }
        let neg = x < 0.0;
        let mut a = x.abs();
        let mut mag = Vec::new();
        while a >= 1.0 {
            let limb = a % 4294967296.0;
            mag.push(limb as u32);
            a = (a - limb) / 4294967296.0;
        }
        JsBigInt { neg, mag }.normalized()
    }

    /// `BigInt("123")`, also `0x…`, `0o…`, `0b…`.
    fn parse(text: &str) -> JsBigInt {
        let t = text.trim();
        let (neg, t) = match t.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, t.strip_prefix('+').unwrap_or(t)),
        };
        let (radix, digits) = match t.get(..2).map(|p| p.to_ascii_lowercase()) {
            Some(p) if p == "0x" => (16, &t[2..]),
            Some(p) if p == "0o" => (8, &t[2..]),
            Some(p) if p == "0b" => (2, &t[2..]),
            _ => (10, t),
        };
        let mut out = JsBigInt::default();
        let r = JsBigInt::from_i64(radix as i64);
        for c in digits.chars().filter(|c| *c != '_') {
            let Some(d) = c.to_digit(radix) else {
                panic!("SyntaxError: Cannot convert {text} to a BigInt");
            };
            out = out.times(&r).plus(&JsBigInt::from_i64(d as i64));
        }
        if t.is_empty() {
            return JsBigInt::default();
        }
        JsBigInt { neg, mag: out.mag }.normalized()
    }

    fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }

    fn cmp_mag(a: &[u32], b: &[u32]) -> std::cmp::Ordering {
        a.len().cmp(&b.len()).then_with(|| {
            for i in (0..a.len()).rev() {
                match a[i].cmp(&b[i]) {
                    std::cmp::Ordering::Equal => continue,
                    o => return o,
                }
            }
            std::cmp::Ordering::Equal
        })
    }

    fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
        let mut carry = 0u64;
        for i in 0..a.len().max(b.len()) {
            let s = *a.get(i).unwrap_or(&0) as u64 + *b.get(i).unwrap_or(&0) as u64 + carry;
            out.push(s as u32);
            carry = s >> 32;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        out
    }

    /// `a - b` for `|a| >= |b|`.
    fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut out = Vec::with_capacity(a.len());
        let mut borrow = 0i64;
        for i in 0..a.len() {
            let mut d = a[i] as i64 - *b.get(i).unwrap_or(&0) as i64 - borrow;
            borrow = 0;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            }
            out.push(d as u32);
        }
        out
    }

    fn plus(&self, other: &JsBigInt) -> JsBigInt {
        if self.neg == other.neg {
            return JsBigInt { neg: self.neg, mag: Self::add_mag(&self.mag, &other.mag) }.normalized();
        }
        match Self::cmp_mag(&self.mag, &other.mag) {
            std::cmp::Ordering::Less => {
                JsBigInt { neg: other.neg, mag: Self::sub_mag(&other.mag, &self.mag) }.normalized()
            }
            _ => JsBigInt { neg: self.neg, mag: Self::sub_mag(&self.mag, &other.mag) }.normalized(),
        }
    }

    fn negated(&self) -> JsBigInt {
        JsBigInt { neg: !self.neg, mag: self.mag.clone() }.normalized()
    }

    fn minus(&self, other: &JsBigInt) -> JsBigInt {
        self.plus(&other.negated())
    }

    fn times(&self, other: &JsBigInt) -> JsBigInt {
        let mut out = vec![0u32; self.mag.len() + other.mag.len()];
        for (i, &a) in self.mag.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in other.mag.iter().enumerate() {
                let t = out[i + j] as u64 + a as u64 * b as u64 + carry;
                out[i + j] = t as u32;
                carry = t >> 32;
            }
            let mut k = i + other.mag.len();
            while carry > 0 {
                let t = out[k] as u64 + carry;
                out[k] = t as u32;
                carry = t >> 32;
                k += 1;
            }
        }
        JsBigInt { neg: self.neg != other.neg, mag: out }.normalized()
    }

    /// Quotient and remainder of magnitudes, bit by bit.
    fn divrem_mag(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
        if Self::cmp_mag(a, b) == std::cmp::Ordering::Less {
            return (Vec::new(), a.to_vec());
        }
        // A one-limb divisor: limb by limb.
        if b.len() == 1 {
            let d = b[0] as u64;
            let mut q = vec![0u32; a.len()];
            let mut r = 0u64;
            for i in (0..a.len()).rev() {
                let cur = (r << 32) | a[i] as u64;
                q[i] = (cur / d) as u32;
                r = cur % d;
            }
            return (q, vec![r as u32]);
        }
        let bits = a.len() * 32;
        let mut q = vec![0u32; a.len()];
        let mut r: Vec<u32> = Vec::new();
        for i in (0..bits).rev() {
            // r = r * 2 + bit i of a
            let mut carry = (a[i / 32] >> (i % 32)) & 1;
            for limb in r.iter_mut() {
                let next = *limb >> 31;
                *limb = (*limb << 1) | carry;
                carry = next;
            }
            if carry > 0 {
                r.push(carry);
            }
            if Self::cmp_mag(&r, b) != std::cmp::Ordering::Less {
                r = Self::sub_mag(&r, b);
                while r.last() == Some(&0) {
                    r.pop();
                }
                q[i / 32] |= 1 << (i % 32);
            }
        }
        (q, r)
    }

    fn div(&self, other: &JsBigInt) -> JsBigInt {
        if other.is_zero() {
            panic!("RangeError: Division by zero");
        }
        let (q, _) = Self::divrem_mag(&self.mag, &other.mag);
        JsBigInt { neg: self.neg != other.neg, mag: q }.normalized()
    }

    fn rem(&self, other: &JsBigInt) -> JsBigInt {
        if other.is_zero() {
            panic!("RangeError: Division by zero");
        }
        let (_, r) = Self::divrem_mag(&self.mag, &other.mag);
        JsBigInt { neg: self.neg, mag: r }.normalized()
    }

    fn pow(&self, exp: &JsBigInt) -> JsBigInt {
        if exp.neg {
            panic!("RangeError: Exponent must be non-negative");
        }
        let mut result = JsBigInt::from_i64(1);
        let mut base = self.clone();
        let mut e = exp.clone();
        let two = JsBigInt::from_i64(2);
        while !e.is_zero() {
            if e.mag[0] & 1 == 1 {
                result = result.times(&base);
            }
            e = e.div(&two);
            if !e.is_zero() {
                base = base.times(&base);
            }
        }
        result
    }

    /// Two's complement limbs, `n` long.
    fn twos(&self, n: usize) -> Vec<u32> {
        let mut m = self.mag.clone();
        m.resize(n, 0);
        if self.neg {
            for limb in m.iter_mut() {
                *limb = !*limb;
            }
            let mut carry = 1u64;
            for limb in m.iter_mut() {
                let t = *limb as u64 + carry;
                *limb = t as u32;
                carry = t >> 32;
            }
        }
        m
    }

    fn from_twos(mut m: Vec<u32>) -> JsBigInt {
        let neg = m.last().is_some_and(|l| l >> 31 == 1);
        if neg {
            for limb in m.iter_mut() {
                *limb = !*limb;
            }
            let mut carry = 1u64;
            for limb in m.iter_mut() {
                let t = *limb as u64 + carry;
                *limb = t as u32;
                carry = t >> 32;
            }
        }
        JsBigInt { neg, mag: m }.normalized()
    }

    /// `a & b`, `a | b`, `a ^ b`.
    fn bitwise(&self, other: &JsBigInt, op: char) -> JsBigInt {
        let n = self.mag.len().max(other.mag.len()) + 1;
        let (a, b) = (self.twos(n), other.twos(n));
        let m = a
            .iter()
            .zip(&b)
            .map(|(x, y)| match op {
                '&' => x & y,
                '|' => x | y,
                _ => x ^ y,
            })
            .collect();
        Self::from_twos(m)
    }

    /// `a << n` (`a >> n` for a negative `n`): floor division by a power of two.
    fn shift_left(&self, n: i64) -> JsBigInt {
        if n >= 0 {
            self.times(&JsBigInt::from_i64(2).pow(&JsBigInt::from_i64(n)))
        } else {
            let d = JsBigInt::from_i64(2).pow(&JsBigInt::from_i64(-n));
            let q = self.div(&d);
            // Toward minus infinity, like `>>`.
            if self.neg && !self.rem(&d).is_zero() {
                q.minus(&JsBigInt::from_i64(1))
            } else {
                q
            }
        }
    }

    fn to_f64(&self) -> f64 {
        let v = self.mag.iter().rev().fold(0.0, |acc, &l| acc * 4294967296.0 + l as f64);
        if self.neg { -v } else { v }
    }

    fn to_i64(&self) -> i64 {
        self.to_f64() as i64
    }

    fn to_string_radix(&self, radix: u32) -> String {
        if self.is_zero() {
            return "0".into();
        }
        let mut digits = Vec::new();
        let mut m = self.mag.clone();
        let r = vec![radix];
        while !m.is_empty() {
            let (q, rem) = Self::divrem_mag(&m, &r);
            digits.push(std::char::from_digit(*rem.first().unwrap_or(&0), radix).unwrap_or('0'));
            m = q;
            while m.last() == Some(&0) {
                m.pop();
            }
        }
        if self.neg {
            digits.push('-');
        }
        digits.iter().rev().collect()
    }
}

impl std::fmt::Display for JsBigInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string_radix(10))
    }
}

impl PartialOrd for JsBigInt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JsBigInt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.neg, other.neg) {
            (false, true) => std::cmp::Ordering::Greater,
            (true, false) => std::cmp::Ordering::Less,
            (false, false) => Self::cmp_mag(&self.mag, &other.mag),
            (true, true) => Self::cmp_mag(&other.mag, &self.mag),
        }
    }
}

impl JsInspect for JsBigInt {
    fn js_value(&self) -> JsValue {
        JsValue::Raw(format!("{self}n"))
    }
}


impl JsInspect for Object {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("id".to_string(), self.id.js_value())])
    }
}
