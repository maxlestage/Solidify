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
        | JsValue::Func(_) | JsValue::Raw(_) | JsValue::Date(..) => JsValue::Str(js_to_string(v)),
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
    if let JsValue::Object(_, props) = v {
        props.retain(|(k, _)| k != key);
    }
    true
}

/// `v.name(args)`: the methods of strings, arrays and numbers.
#[allow(dead_code)]
fn js_call_method(v: &JsValue, name: &str, args: Vec<JsValue>) -> JsValue {
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
