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
