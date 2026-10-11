//! Small helpers written into the generated file when the program needs them: JavaScript
//! string and array operations with no direct equivalent in Rust's standard library.

/// A helper, by name; the text is appended at the end of the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Helper {
    StrSlice,
    StrSubstring,
    StrCharAt,
    StrIndexOf,
    StrLastIndexOf,
    StrCharCodeAt,
    StrPadStart,
    StrPadEnd,
    StrSplit,
    VecSlice,
    VecSplice,
    VecIndexOf,
    VecLastIndexOf,
    VecAt,
    VecShift,
    Compare,
    Ordering,
    ParseInt,
    ParseIntRadix,
    ParseFloat,
    /// A double as JavaScript prints it (`1e+21`, `Infinity`).
    NumText,
    /// `toFixed` and `toPrecision`, rounded as JavaScript does.
    NumFormat,
    /// JavaScript's `ToInt32`, for bitwise operators on doubles.
    ToInt32,
    ToNumber,
    Random,
    NumToString,
    BlockOn,
    /// `JsValue` and `JsInspect`: `console.log` and `String(…)` of objects, arrays, maps and sets.
    Inspect,
    /// A map in insertion order.
    JsMap,
    /// A set in insertion order.
    JsSet,
    /// `JsGenerator`: a `function*` running on its own thread.
    Generator,
    /// `setTimeout` / `setInterval` and the loop running them.
    Timers,
    /// `new Promise`.
    Promise,
    /// `JsRegex`: regular expressions.
    Regex,
    /// `JSON.stringify` and `JSON.parse` (`JsonValue`, `JsFromJson`).
    Json,
    /// `JsDate`: `new Date(…)` and its methods, in UTC.
    Date,
    /// `JsSymbol`: `Symbol(…)`, `Symbol.for(…)`.
    Symbol,
    /// Values known only at run time: JavaScript's operators on `JsValue`.
    Dynamic,
    /// `eval` of code known only at run time.
    Eval,
    /// `Shared<T>`: objects held in several places.
    Shared,
    /// The native side of the Node modules (`fs`, `os`).
    Node,
    /// `JsBigInt`: `bigint`.
    BigInt,
    /// Errors of the program's own classes.
    Error,
}

/// The regular expression engine, compiled into Solidify too: patterns are checked when
/// translating.
#[path = "runtime/regex.rs"]
pub mod regex_rt;

impl Helper {
    pub fn source(self) -> &'static str {
        match self {
            Helper::StrSlice => {
                r#"/// `s.slice(start, end)`: character positions, negative ones counted from the end.
fn js_slice(s: &str, start: i64, end: Option<i64>) -> String {
    let len = s.chars().count() as i64;
    let clamp = |i: i64| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (a, b) = (clamp(start), clamp(end.unwrap_or(len)));
    if b <= a {
        return String::new();
    }
    s.chars().skip(a as usize).take((b - a) as usize).collect()
}"#
            }
            Helper::StrSubstring => {
                r#"/// `s.substring(start, end)`: character positions, swapped when out of order.
fn js_substring(s: &str, start: i64, end: Option<i64>) -> String {
    let len = s.chars().count() as i64;
    let clamp = |i: i64| i.clamp(0, len);
    let (mut a, mut b) = (clamp(start), clamp(end.unwrap_or(len)));
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    s.chars().skip(a as usize).take((b - a) as usize).collect()
}"#
            }
            Helper::StrCharAt => {
                r#"/// `s.charAt(i)` / `s[i]`: one character, or an empty string.
fn js_char_at(s: &str, i: i64) -> String {
    if i < 0 {
        return String::new();
    }
    s.chars().nth(i as usize).map(String::from).unwrap_or_default()
}"#
            }
            Helper::StrIndexOf => {
                r#"/// `s.indexOf(needle)`: the character position, or -1.
fn js_index_of(s: &str, needle: &str) -> i64 {
    match s.find(needle) {
        Some(byte) => s[..byte].chars().count() as i64,
        None => -1,
    }
}

/// `s.indexOf(needle, from)`.
#[allow(dead_code)]
fn js_index_of_from(s: &str, needle: &str, from: i64) -> i64 {
    let from = from.max(0) as usize;
    let Some((start, _)) = s.char_indices().nth(from).or((from == s.chars().count()).then_some((s.len(), ' '))) else {
        return if needle.is_empty() { s.chars().count() as i64 } else { -1 };
    };
    match s[start..].find(needle) {
        Some(byte) => s[..start + byte].chars().count() as i64,
        None => -1,
    }
}"#
            }
            Helper::StrLastIndexOf => {
                r#"/// `s.lastIndexOf(needle)`: the last character position, or -1.
fn js_last_index_of(s: &str, needle: &str) -> i64 {
    match s.rfind(needle) {
        Some(byte) => s[..byte].chars().count() as i64,
        None => -1,
    }
}

/// `s.lastIndexOf(needle, from)`: the last match starting at `from` or before.
#[allow(dead_code)]
fn js_last_index_of_from(s: &str, needle: &str, from: i64) -> i64 {
    let chars: Vec<char> = s.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    if n.len() > chars.len() {
        return -1;
    }
    let last = (chars.len() - n.len()) as i64;
    let mut i = from.clamp(0, last.max(0)).min(last);
    while i >= 0 {
        if chars[i as usize..].starts_with(&n) {
            return i;
        }
        i -= 1;
    }
    -1
}"#
            }
            Helper::StrCharCodeAt => {
                r#"/// `s.charCodeAt(i)`: the code of a character, or -1 (NaN in JavaScript).
fn js_char_code_at(s: &str, i: i64) -> i64 {
    if i < 0 {
        return -1;
    }
    s.chars().nth(i as usize).map_or(-1, |c| c as i64)
}"#
            }
            Helper::StrPadStart => {
                r#"/// `s.padStart(width, pad)`.
fn js_pad_start(s: &str, width: i64, pad: &str) -> String {
    let len = s.chars().count() as i64;
    if width <= len || pad.is_empty() {
        return s.to_string();
    }
    let fill: String = pad.chars().cycle().take((width - len) as usize).collect();
    format!("{fill}{s}")
}"#
            }
            Helper::StrPadEnd => {
                r#"/// `s.padEnd(width, pad)`.
fn js_pad_end(s: &str, width: i64, pad: &str) -> String {
    let len = s.chars().count() as i64;
    if width <= len || pad.is_empty() {
        return s.to_string();
    }
    let fill: String = pad.chars().cycle().take((width - len) as usize).collect();
    format!("{s}{fill}")
}"#
            }
            Helper::StrSplit => {
                r#"/// `s.split(sep)`: an empty separator splits into characters.
fn js_split(s: &str, sep: &str) -> Vec<String> {
    if sep.is_empty() {
        s.chars().map(String::from).collect()
    } else {
        s.split(sep).map(String::from).collect()
    }
}"#
            }
            Helper::VecSlice => {
                r#"/// `list.slice(start, end)`: negative positions are counted from the end.
fn js_slice_vec<T: Clone>(v: &[T], start: i64, end: Option<i64>) -> Vec<T> {
    let len = v.len() as i64;
    let clamp = |i: i64| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (a, b) = (clamp(start), clamp(end.unwrap_or(len)));
    if b <= a {
        return Vec::new();
    }
    v[a as usize..b as usize].to_vec()
}"#
            }
            Helper::VecSplice => {
                r#"/// `list.splice(start, count, ...items)`: removes and inserts in place, returns the removed.
fn js_splice<T>(v: &mut Vec<T>, start: i64, count: Option<i64>, items: Vec<T>) -> Vec<T> {
    let len = v.len() as i64;
    let start = if start < 0 { (len + start).max(0) } else { start.min(len) } as usize;
    let count = count.map_or(v.len() - start, |c| c.clamp(0, len - start as i64) as usize);
    v.splice(start..start + count, items).collect()
}"#
            }
            Helper::VecIndexOf => {
                r#"/// `list.indexOf(item)`: the position, or -1.
fn js_index_of_item<T: PartialEq>(v: &[T], item: &T) -> i64 {
    v.iter().position(|x| x == item).map_or(-1, |i| i as i64)
}"#
            }
            Helper::VecLastIndexOf => {
                r#"/// `list.lastIndexOf(item)`: the last position, or -1.
fn js_last_index_of_item<T: PartialEq>(v: &[T], item: &T) -> i64 {
    v.iter().rposition(|x| x == item).map_or(-1, |i| i as i64)
}"#
            }
            Helper::VecAt => {
                r#"/// `list.at(i)`: negative positions are counted from the end.
fn js_at<T: Clone>(v: &[T], i: i64) -> Option<T> {
    let i = if i < 0 { v.len() as i64 + i } else { i };
    if i < 0 { None } else { v.get(i as usize).cloned() }
}"#
            }
            Helper::VecShift => {
                r#"/// `list.shift()`: removes and returns the first element.
fn js_shift<T>(v: &mut Vec<T>) -> Option<T> {
    if v.is_empty() { None } else { Some(v.remove(0)) }
}"#
            }
            Helper::Compare => {
                r#"/// `a.localeCompare(b)`: -1, 0 or 1.
fn js_compare(a: &str, b: &str) -> i64 {
    match a.cmp(b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}"#
            }
            Helper::Ordering => {
                r#"/// The ordering a JavaScript comparator's result stands for.
fn js_ordering(n: f64) -> std::cmp::Ordering {
    n.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
}"#
            }
            Helper::ParseInt => {
                r#"/// `parseInt(s)`: the leading integer, or `None` for `NaN`.
fn js_parse_int(s: &str) -> Option<i64> {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .take_while(|&(i, c)| c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')))
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    s[..end].parse::<i64>().ok()
}"#
            }
            Helper::ParseIntRadix => {
                r#"/// `parseInt(s, radix)`: the leading digits in the given base, or `None` for `NaN`.
fn js_parse_int_radix(s: &str, radix: i64) -> Option<i64> {
    if !(2..=36).contains(&radix) {
        return None;
    }
    let s = s.trim_start();
    let (negative, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let digits = if radix == 16 {
        digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
            .unwrap_or(digits)
    } else {
        digits
    };
    let end = digits
        .char_indices()
        .take_while(|&(_, c)| c.is_digit(radix as u32))
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    let n = i64::from_str_radix(&digits[..end], radix as u32).ok()?;
    Some(if negative { -n } else { n })
}"#
            }
            Helper::NumText => include_str!("runtime/number.rs"),
            Helper::NumFormat => include_str!("runtime/number_format.rs"),
            Helper::ToInt32 => {
                r#"/// A double as JavaScript's bitwise operators see it: a 32-bit integer.
fn js_to_int32(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    (x.trunc().rem_euclid(4294967296.0) as u32) as i32
}"#
            }
            Helper::ParseFloat => {
                r#"/// `parseFloat(s)`: the leading number, or NaN.
fn js_parse_float(s: &str) -> f64 {
    let s = s.trim_start();
    let mut end = 0;
    let (mut dot, mut exp, mut digits) = (false, false, false);
    for (i, c) in s.char_indices() {
        let ok = match c {
            '0'..='9' => {
                digits = true;
                true
            }
            '-' | '+' => i == 0 || matches!(s[..i].chars().last(), Some('e' | 'E')),
            '.' => !dot && !exp && std::mem::replace(&mut dot, true) == false,
            'e' | 'E' => digits && !exp && std::mem::replace(&mut exp, true) == false,
            _ => false,
        };
        if !ok {
            break;
        }
        end = i + c.len_utf8();
    }
    let mut text = &s[..end];
    while !text.is_empty() && text.parse::<f64>().is_err() {
        text = &text[..text.len() - 1];
    }
    text.parse::<f64>().unwrap_or(f64::NAN)
}"#
            }
            Helper::ToNumber => {
                r#"/// `Number(s)`: the whole string as a number, 0 when blank, NaN otherwise.
fn js_number(s: &str) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).map_or(f64::NAN, |n| n as f64);
    }
    match s {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => s.parse::<f64>().unwrap_or(f64::NAN),
    }
}"#
            }
            Helper::Random => {
                r#"/// `Math.random()`: a pseudo-random number in [0, 1), seeded from the clock.
fn js_random() -> f64 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = Cell::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64 | 1),
        );
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}"#
            }
            Helper::NumToString => {
                r#"/// `n.toString(radix)`.
fn js_num_to_string(mut n: i64, radix: i64) -> String {
    if !(2..=36).contains(&radix) {
        return n.to_string();
    }
    if n == 0 {
        return "0".into();
    }
    let negative = n < 0;
    n = n.abs();
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(std::char::from_digit((n % radix) as u32, radix as u32).unwrap_or('?'));
        n /= radix;
    }
    if negative {
        digits.push('-');
    }
    digits.iter().rev().collect()
}"#
            }
            Helper::Inspect => include_str!("runtime/inspect.rs"),
            Helper::JsMap => include_str!("runtime/map.rs"),
            Helper::JsSet => include_str!("runtime/set.rs"),
            Helper::Generator => include_str!("runtime/generator.rs"),
            Helper::Timers => include_str!("runtime/timers.rs"),
            Helper::Promise => include_str!("runtime/promise.rs"),
            Helper::Regex => include_str!("runtime/regex.rs"),
            Helper::Json => include_str!("runtime/json.rs"),
            Helper::Date => include_str!("runtime/date.rs"),
            Helper::Symbol => include_str!("runtime/symbol.rs"),
            Helper::Dynamic => include_str!("runtime/dynamic.rs"),
            Helper::Eval => include_str!("runtime/eval.rs"),
            Helper::Shared => include_str!("runtime/shared.rs"),
            Helper::Node => include_str!("runtime/node.rs"),
            Helper::BigInt => include_str!("runtime/bigint.rs"),
            Helper::Error => include_str!("runtime/error.rs"),
            Helper::BlockOn => {
                r#"/// Runs the top level, with the tasks and timers it waits for.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let mut future = std::pin::pin!(future);
    loop {
        if let std::task::Poll::Ready(v) = future.as_mut().poll(&mut cx) {
            return v;
        }
        let before = JS_TICK.with(|t| t.get());
        js_run_tasks();
        if JS_TICK.with(|t| t.get()) != before {
            continue;
        }
        js_report_unhandled();
        if !js_run_next_timer() {
            // Node leaves a top-level await nothing will settle.
            std::process::exit(13);
        }
    }
}"#
            }
        }
    }
}
