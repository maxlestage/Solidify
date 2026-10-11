#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub when: JsDate,
}

fn main() {
    let epoch = JsDate::from_ms(0);
    let launch = JsDate::parse("2024-06-01T12:30:00.250Z");
    let day = JsDate::from_parts(2024, 5, 1, 0, 0, 0, 0);
    println!("{} {} {} {} {} {} {} {} {}", epoch.get_full_year(), epoch.to_iso_string(), launch.get_month(), launch.get_date(), launch.get_day(), launch.get_hours(), launch.get_minutes(), launch.get_seconds(), launch.get_milliseconds());
    println!("{} {} {} {}", day.to_iso_string(), day.get_time(), JsDate::from_ms(1700000000000_i64).to_iso_string(), JsDate::parse("2000-02-29").get_day());
    println!("{} {} {}", launch > day, launch.get_time() - day.get_time(), js_number_text((launch.get_time() - day.get_time()) as f64 / 60000.0));
    println!("{} {} {} {}", js_inspect(&launch.js_value()), js_to_string(&epoch.js_value()), js_to_string(&day.js_value()), js_json_stringify(&(Object { when: launch }).js_value(), ""));
    println!("{} {} {}", epoch.to_utc_string(), day.to_date_string(), JsDate::from_parts(2024, 13, 1, 0, 0, 0, 0).to_iso_string());
    let later = JsDate::from_ms(launch.get_time() + 86400000);
    println!("{} {} {}", later.to_iso_string(), later.value_of() > launch.value_of(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64) > 0);
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


/// A `Date`: milliseconds since 1970-01-01T00:00:00Z, read and shown in UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub struct JsDate {
    ms: i64,
}

#[allow(dead_code)]
impl JsDate {
    /// `new Date()`.
    pub fn now() -> JsDate {
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        JsDate { ms }
    }

    /// `new Date(ms)`.
    pub fn from_ms(ms: i64) -> JsDate {
        JsDate { ms }
    }

    /// `new Date(year, month, day, hours, minutes, seconds, ms)`: the month counts from
    /// 0 and overflows into the year, as in JavaScript.
    pub fn from_parts(year: i64, month: i64, day: i64, h: i64, mi: i64, s: i64, ms: i64) -> JsDate {
        let y = year + month.div_euclid(12);
        let m = month.rem_euclid(12) + 1;
        let days = js_days_from_civil(y, m, 1) + (day - 1);
        JsDate {
            ms: days * 86_400_000 + h * 3_600_000 + mi * 60_000 + s * 1000 + ms,
        }
    }

    /// `new Date("2024-06-01")`, `new Date("2024-06-01T12:30:00Z")`,
    /// `new Date("2024-06-01T12:30:00.250+02:00")`.
    pub fn parse(text: &str) -> JsDate {
        match js_parse_date(text.trim()) {
            Some(ms) => JsDate { ms },
            None => panic!("Invalid Date: {text:?}"),
        }
    }

    /// `d.getTime()`, `d.valueOf()`.
    pub fn get_time(&self) -> i64 {
        self.ms
    }

    pub fn value_of(&self) -> i64 {
        self.ms
    }

    fn days(&self) -> i64 {
        self.ms.div_euclid(86_400_000)
    }

    fn ms_of_day(&self) -> i64 {
        self.ms.rem_euclid(86_400_000)
    }

    pub fn get_full_year(&self) -> i64 {
        js_civil_from_days(self.days()).0
    }

    /// `d.getMonth()`: 0 to 11.
    pub fn get_month(&self) -> i64 {
        js_civil_from_days(self.days()).1 - 1
    }

    /// `d.getDate()`: the day of the month, 1 to 31.
    pub fn get_date(&self) -> i64 {
        js_civil_from_days(self.days()).2
    }

    /// `d.getDay()`: the day of the week, 0 for Sunday.
    pub fn get_day(&self) -> i64 {
        (self.days() + 4).rem_euclid(7)
    }

    pub fn get_hours(&self) -> i64 {
        self.ms_of_day() / 3_600_000
    }

    pub fn get_minutes(&self) -> i64 {
        self.ms_of_day() / 60_000 % 60
    }

    pub fn get_seconds(&self) -> i64 {
        self.ms_of_day() / 1000 % 60
    }

    pub fn get_milliseconds(&self) -> i64 {
        self.ms_of_day() % 1000
    }

    pub fn get_utc_full_year(&self) -> i64 {
        self.get_full_year()
    }

    pub fn get_utc_month(&self) -> i64 {
        self.get_month()
    }

    pub fn get_utc_date(&self) -> i64 {
        self.get_date()
    }

    pub fn get_utc_day(&self) -> i64 {
        self.get_day()
    }

    pub fn get_utc_hours(&self) -> i64 {
        self.get_hours()
    }

    pub fn get_utc_minutes(&self) -> i64 {
        self.get_minutes()
    }

    pub fn get_utc_seconds(&self) -> i64 {
        self.get_seconds()
    }

    pub fn get_utc_milliseconds(&self) -> i64 {
        self.get_milliseconds()
    }

    /// `d.getTimezoneOffset()`: minutes, 0 in UTC.
    pub fn get_timezone_offset(&self) -> i64 {
        0
    }

    /// `d.toISOString()`: `2024-06-01T12:30:00.250Z`.
    pub fn to_iso_string(&self) -> String {
        let (y, m, d) = js_civil_from_days(self.days());
        let year = if (0..=9999).contains(&y) {
            format!("{y:04}")
        } else if y < 0 {
            format!("-{:06}", -y)
        } else {
            format!("+{y:06}")
        };
        format!(
            "{year}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
            self.get_hours(),
            self.get_minutes(),
            self.get_seconds(),
            self.get_milliseconds()
        )
    }

    pub fn to_json(&self) -> String {
        self.to_iso_string()
    }

    /// `d.toDateString()`: `Sat Jun 01 2024`.
    pub fn to_date_string(&self) -> String {
        let (y, m, d) = js_civil_from_days(self.days());
        format!(
            "{} {} {d:02} {y:04}",
            JS_WEEKDAYS[self.get_day() as usize],
            JS_MONTHS[(m - 1) as usize]
        )
    }

    /// `d.toTimeString()`: `12:30:00 GMT+0000 (Coordinated Universal Time)`.
    pub fn to_time_string(&self) -> String {
        format!(
            "{:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
            self.get_hours(),
            self.get_minutes(),
            self.get_seconds()
        )
    }

    /// `d.toUTCString()`: `Sat, 01 Jun 2024 12:30:00 GMT`.
    pub fn to_utc_string(&self) -> String {
        let (y, m, d) = js_civil_from_days(self.days());
        format!(
            "{}, {d:02} {} {y:04} {:02}:{:02}:{:02} GMT",
            JS_WEEKDAYS[self.get_day() as usize],
            JS_MONTHS[(m - 1) as usize],
            self.get_hours(),
            self.get_minutes(),
            self.get_seconds()
        )
    }
}

/// `String(d)`: `Sat Jun 01 2024 12:30:00 GMT+0000 (Coordinated Universal Time)`.
impl std::fmt::Display for JsDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.to_date_string(), self.to_time_string())
    }
}

#[allow(dead_code)]
const JS_WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
#[allow(dead_code)]
const JS_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Days since 1970-01-01 of a civil date (proleptic Gregorian calendar).
#[allow(dead_code)]
fn js_days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date (year, month 1-12, day) of a number of days since 1970-01-01.
#[allow(dead_code)]
fn js_civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYY-MM-DD`, with an optional `THH:MM[:SS[.mmm]]` and a `Z` or `±HH:MM` offset (a
/// date alone is UTC, as in JavaScript).
#[allow(dead_code)]
fn js_parse_date(text: &str) -> Option<i64> {
    let (date, rest) = match text.find('T') {
        Some(i) => (&text[..i], &text[i + 1..]),
        None => (text, ""),
    };
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next().map_or(Some(1), |p| p.parse().ok())?;
    let d: i64 = parts.next().map_or(Some(1), |p| p.parse().ok())?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut ms = js_days_from_civil(y, m, d) * 86_400_000;
    if rest.is_empty() {
        return Some(ms);
    }
    let (time, offset) = match rest.find(['Z', '+', '-']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut fields = time.split(':');
    let h: i64 = fields.next()?.parse().ok()?;
    let mi: i64 = fields.next()?.parse().ok()?;
    let s_text = fields.next().unwrap_or("0");
    let (s, frac) = match s_text.find('.') {
        Some(i) => (&s_text[..i], &s_text[i + 1..]),
        None => (s_text, ""),
    };
    let s: i64 = s.parse().ok()?;
    let millis: i64 = if frac.is_empty() {
        0
    } else {
        let digits: String = frac.chars().take(3).collect();
        let n: i64 = digits.parse().ok()?;
        n * 10i64.pow(3 - digits.len() as u32)
    };
    if h > 24 || mi > 59 || s > 59 {
        return None;
    }
    ms += h * 3_600_000 + mi * 60_000 + s * 1000 + millis;
    match offset {
        "" | "Z" => Some(ms),
        off => {
            let sign = if off.starts_with('-') { 1 } else { -1 };
            let (oh, om) = off[1..].split_once(':')?;
            let (oh, om): (i64, i64) = (oh.parse().ok()?, om.parse().ok()?);
            Some(ms + sign * (oh * 3_600_000 + om * 60_000))
        }
    }
}


impl JsInspect for JsDate {
    fn js_value(&self) -> JsValue {
        JsValue::Date(self.to_iso_string(), self.to_string())
    }
}

impl JsInspect for Object {
    fn js_value(&self) -> JsValue {
        JsValue::Object(None, vec![("when".to_string(), self.when.js_value())])
    }
}
