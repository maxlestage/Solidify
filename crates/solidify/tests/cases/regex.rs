use std::sync::LazyLock;

/// Regular expressions: a small engine written into the program, with JavaScript's syntax.
static EMAIL: LazyLock<JsRegex> = LazyLock::new(|| JsRegex::new("^([\\w.+-]+)@([\\w-]+)\\.([a-z]{2,})$", "i"));
const TEXT: &str = "Dates: 2024-01-15, 1999-12-31 and 2000-02-29.";

fn check(address: &str) -> String {
    let m = (*EMAIL).exec(address);
    let Some(m) = &m else {
        return format!("{address}: invalid");
    };
    format!("{address}: user={} host={} tld={}", m.get(1), m.get(2), m.get(3).to_lowercase())
}

fn main() {
    // Regular expressions: a small engine written into the program, with JavaScript's syntax.
    let date_re = JsRegex::new("(\\d{4})-(\\d{2})-(\\d{2})", "");
    println!("{} {} {}", check("Ada.Lovelace+math@Example.COM"), check("nope@"), (*EMAIL).test("x@y.io"));
    let found = JsRegex::new("\\d{4}-\\d{2}-\\d{2}", "g").match_all(TEXT);
    println!("{} {} {}", found.as_ref().map_or("null".to_string(), |v| js_inspect(&v.js_value())), if let Some(found) = &found { found.len() as i64 } else { 0 }, date_re.search(TEXT));
    for m in JsRegex::new("(\\d{4})-(\\d{2})", "g").find_all(TEXT) {
        println!("{} year {} month {} at {}", m.get(0), m.get(1), m.get(2), m.index);
    }
    let first = date_re.exec(TEXT);
    if let Some(first) = &first {
        println!("{} {} {} {}", first.get(0), first.index, first.len(), format!("{}/{}", first.get(1), first.get(2)));
    }
    println!("{} {}", date_re.replace(TEXT, "$3/$2/$1"), JsRegex::new("\\d+", "g").replace(TEXT, "#"));
    println!("{} {}", JsRegex::new("\\b\\w", "g").replace_with("hello world", |m_| (|c: String| c.to_uppercase())(m_.get(0))), JsRegex::new("[-_]", "g").replace("a-b_c", " "));
    println!("{}", JsRegex::new("(\\d+)-(\\d+)-(\\d+)", "").replace_with("2024-06-01", |m_| (|all: String, y: String, mo: String, d: String| format!("{d}.{mo}.{y} ({})", all.chars().count()))(m_.get(0), m_.get(1), m_.get(2), m_.get(3))));
    println!("{} {} {}", js_inspect(&(JsRegex::new("\\d+", "").split("a1b22c333")).js_value()), JsRegex::new("\\s*,\\s*", "").split("one, two,three ,four").join("|"), js_inspect(&(JsRegex::new(",", "").split("x")).js_value()));
    let words = JsRegex::new("\\s+", "").split("The quick brown fox");
    println!("{} {} {} {} {}", words.len(), words.iter().map(|w| w.chars().count() as i64).collect::<Vec<_>>().iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","), JsRegex::new("quick", "").source(), JsRegex::new("x", "gi").flags(), JsRegex::new("x", "g").flags().contains('g'));
    let re = JsRegex::new("c.t", "g");
    println!("{} {} {}", re.replace("cat cot cut", "-"), re.test("dog"), JsRegex::new("^a+$", "").test("aaa"));
    println!("{} {} {}", JsRegex::new("\\s", "").split("tabs\tand\nnewlines").len(), JsRegex::new("s+", "g").replace("mississippi", "S"), JsRegex::new("(?<word>\\w+)", "").exec("hi there").as_ref().map(|v| v.get(1)).as_ref().map_or("null".to_string(), |v| v.to_string()));
    println!("{} {}", JsRegex::new("42", "").replace_with("price: 42 EUR", |m_| (|n: String| (js_parse_int(&n).unwrap() * 2).to_string())(m_.get(0))), JsRegex::new("a", "g").replace_with("aaa", |m_| (|x: String| x.to_uppercase())(m_.get(0))));
    println!("{} {} {} {re}", JsRegex::new("^\\p{L}+$", "u").test("été"), "é".chars().count(), JsRegex::new("colou?r", "").test("the colour"));
}

/// `parseInt(s)`: the leading integer, or `None` for `NaN`.
fn js_parse_int(s: &str) -> Option<i64> {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .take_while(|&(i, c)| c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')))
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    s[..end].parse::<i64>().ok()
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


/// A regular expression (`/pattern/flags`): parsed once, matched by backtracking on the
/// characters of the text, with JavaScript's syntax and semantics.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct JsRegex {
    root: Vec<Vec<RxNode>>,
    groups: usize,
    names: Vec<(String, usize)>,
    ignore_case: bool,
    multiline: bool,
    dot_all: bool,
    global: bool,
    source: String,
    flags: String,
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
            source: source.to_string(),
            flags: flags.to_string(),
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
        for start in from..=text.len() {
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
            if matched {
                let groups = caps
                    .iter()
                    .map(|c| c.map(|(a, b)| text[a..b].iter().collect::<String>()))
                    .collect();
                let _ = found;
                return Some(JsMatch {
                    index: start as i64,
                    input: input.to_string(),
                    groups,
                });
            }
        }
        None
    }

    fn chars(input: &str) -> Vec<char> {
        input.chars().collect()
    }

    /// `re.test(s)`.
    pub fn test(&self, input: &str) -> bool {
        self.find_from(&Self::chars(input), input, 0).is_some()
    }

    /// `re.exec(s)` / `s.match(re)` without the `g` flag.
    pub fn exec(&self, input: &str) -> Option<JsMatch> {
        self.find_from(&Self::chars(input), input, 0)
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
            RxNode::Start | RxNode::End | RxNode::WordBoundary(_) | RxNode::Look(..)
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
                                return Err("lookbehind is not supported".into());
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
