/// `eval(code)` for code known only at run time: JavaScript expressions and statements
/// (`let`, `const`, `var`, assignments, `if`, blocks) evaluated on `JsValue`s, with the
/// variables of the program in scope given by name.
#[allow(dead_code)]
fn js_eval(code: &str, scope: Vec<(&str, JsValue)>) -> JsValue {
    let tokens = js_eval_lex(code).unwrap_or_else(|e| panic!("SyntaxError: {e}"));
    let mut ev = JsEval {
        tokens,
        pos: 0,
        vars: vec![scope.into_iter().map(|(k, v)| (k.to_string(), v)).collect()],
        returned: None,
    };
    let mut last = JsValue::Undefined;
    while !ev.at_end() && ev.returned.is_none() {
        last = ev.statement().unwrap_or_else(|e| panic!("{e}"));
    }
    ev.returned.unwrap_or(last)
}

#[derive(Clone, Debug, PartialEq)]
enum JsEvalTok {
    Num(f64),
    Str(String),
    Ident(String),
    Punct(&'static str),
}

fn js_eval_lex(code: &str) -> Result<Vec<JsEvalTok>, String> {
    const PUNCTS: [&str; 45] = [
        "===", "!==", "**=", "...", "&&", "||", "??", "==", "!=", "<=", ">=", "+=", "-=", "*=",
        "/=", "%=", "**", "=>", "?.", "++", "--", "<<", ">>", "+", "-", "*", "/", "%", "<",
        ">", "=", "!", "?", ":", "(", ")", "[", "]", "{", "}", ",", ".", ";", "&", "|",
    ];
    let chars: Vec<char> = code.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let start = i;
            if c == '0' && matches!(chars.get(i + 1), Some('x' | 'X')) {
                i += 2;
                while i < chars.len() && chars[i].is_ascii_hexdigit() {
                    i += 1;
                }
                let text: String = chars[start + 2..i].iter().collect();
                out.push(JsEvalTok::Num(i64::from_str_radix(&text, 16).map_err(|e| e.to_string())? as f64));
                continue;
            }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            if i < chars.len() && matches!(chars[i], 'e' | 'E') {
                i += 1;
                if i < chars.len() && matches!(chars[i], '+' | '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text: String = chars[start..i].iter().filter(|c| **c != '_').collect();
            out.push(JsEvalTok::Num(text.parse().map_err(|_| format!("Invalid number {text}"))?));
        } else if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    s.push(match chars[i] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '0' => '\0',
                        other => other,
                    });
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            if i >= chars.len() {
                return Err("Invalid or unexpected token".into());
            }
            i += 1;
            if quote == '`' {
                // A template: `a${x}b` as ("a" + (x) + "b").
                out.push(JsEvalTok::Punct("("));
                out.push(JsEvalTok::Str(String::new()));
                let mut rest = s.as_str();
                while let Some(start) = rest.find("${") {
                    let end = rest[start..].find('}').map(|e| start + e).ok_or("Unterminated template")?;
                    out.push(JsEvalTok::Punct("+"));
                    out.push(JsEvalTok::Str(rest[..start].to_string()));
                    out.push(JsEvalTok::Punct("+"));
                    out.push(JsEvalTok::Punct("("));
                    out.extend(js_eval_lex(&rest[start + 2..end])?);
                    out.push(JsEvalTok::Punct(")"));
                    rest = &rest[end + 1..];
                }
                out.push(JsEvalTok::Punct("+"));
                out.push(JsEvalTok::Str(rest.to_string()));
                out.push(JsEvalTok::Punct(")"));
            } else {
                out.push(JsEvalTok::Str(s));
            }
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$') {
                i += 1;
            }
            out.push(JsEvalTok::Ident(chars[start..i].iter().collect()));
        } else {
            let rest: String = chars[i..chars.len().min(i + 3)].iter().collect();
            let Some(p) = PUNCTS.iter().find(|p| rest.starts_with(**p)) else {
                return Err(format!("Invalid or unexpected token '{c}'"));
            };
            i += p.chars().count();
            out.push(JsEvalTok::Punct(p));
        }
    }
    Ok(out)
}

struct JsEval {
    tokens: Vec<JsEvalTok>,
    pos: usize,
    /// The variables, innermost block last.
    vars: Vec<Vec<(String, JsValue)>>,
    /// The value of a `return` run.
    returned: Option<JsValue>,
}

type JsEvalResult = Result<JsValue, String>;

#[allow(dead_code)]
impl JsEval {
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&JsEvalTok> {
        self.tokens.get(self.pos)
    }

    fn is(&self, p: &str) -> bool {
        matches!(self.peek(), Some(JsEvalTok::Punct(q)) if *q == p)
    }

    fn is_word(&self, w: &str) -> bool {
        matches!(self.peek(), Some(JsEvalTok::Ident(q)) if q == w)
    }

    fn eat(&mut self, p: &str) -> bool {
        if self.is(p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, p: &str) -> Result<(), String> {
        if self.eat(p) {
            Ok(())
        } else {
            Err(format!("SyntaxError: expected '{p}'"))
        }
    }

    fn ident(&mut self) -> Result<String, String> {
        match self.peek().cloned() {
            Some(JsEvalTok::Ident(n)) => {
                self.pos += 1;
                Ok(n)
            }
            _ => Err("SyntaxError: expected a name".into()),
        }
    }

    fn lookup(&self, name: &str) -> Option<&JsValue> {
        self.vars
            .iter()
            .rev()
            .find_map(|s| s.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v))
    }

    fn assign(&mut self, name: &str, v: JsValue) -> Result<(), String> {
        for s in self.vars.iter_mut().rev() {
            if let Some(slot) = s.iter_mut().rev().find(|(k, _)| k == name) {
                slot.1 = v;
                return Ok(());
            }
        }
        Err(format!("ReferenceError: {name} is not defined"))
    }

    fn statement(&mut self) -> JsEvalResult {
        if self.eat(";") {
            return Ok(JsValue::Undefined);
        }
        if self.is_word("return") {
            self.pos += 1;
            let v = if self.at_end() || self.is(";") || self.is("}") {
                JsValue::Undefined
            } else {
                self.expr()?
            };
            self.eat(";");
            self.returned = Some(v.clone());
            return Ok(v);
        }
        if self.is_word("let") || self.is_word("const") || self.is_word("var") {
            self.pos += 1;
            loop {
                let name = self.ident()?;
                let v = if self.eat("=") { self.expr()? } else { JsValue::Undefined };
                self.vars.last_mut().expect("scope").push((name, v));
                if !self.eat(",") {
                    break;
                }
            }
            self.eat(";");
            return Ok(JsValue::Undefined);
        }
        if self.is_word("if") {
            self.pos += 1;
            self.expect("(")?;
            let test = self.expr()?.truthy();
            self.expect(")")?;
            let start = self.pos;
            let then = if test { Some(self.statement()?) } else { self.skip_statement()?; None };
            let _ = start;
            if self.is_word("else") {
                self.pos += 1;
                if test {
                    self.skip_statement()?;
                } else {
                    return self.statement();
                }
            }
            return Ok(then.unwrap_or(JsValue::Undefined));
        }
        if self.eat("{") {
            self.vars.push(Vec::new());
            let mut last = JsValue::Undefined;
            while !self.eat("}") {
                if self.at_end() {
                    return Err("SyntaxError: missing '}'".into());
                }
                if self.returned.is_some() {
                    self.skip_statement()?;
                    continue;
                }
                last = self.statement()?;
            }
            self.vars.pop();
            return Ok(last);
        }
        let v = self.expr()?;
        self.eat(";");
        Ok(v)
    }

    /// Skips a statement not run (the other branch of an `if`).
    fn skip_statement(&mut self) -> Result<(), String> {
        let mut depth = 0i32;
        loop {
            match self.peek() {
                None => return Ok(()),
                Some(JsEvalTok::Punct("{" | "(" | "[")) => depth += 1,
                Some(JsEvalTok::Punct("}" | ")" | "]")) => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos += 1;
                        if !self.is_word("else") {
                            self.eat(";");
                        }
                        return Ok(());
                    }
                }
                Some(JsEvalTok::Punct(";")) if depth == 0 => {
                    self.pos += 1;
                    return Ok(());
                }
                _ => {}
            }
            self.pos += 1;
        }
    }

    fn expr(&mut self) -> JsEvalResult {
        // An assignment to a name.
        if let (Some(JsEvalTok::Ident(name)), Some(JsEvalTok::Punct(op))) =
            (self.tokens.get(self.pos).cloned(), self.tokens.get(self.pos + 1).cloned())
            && matches!(op, "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "**=")
        {
            self.pos += 2;
            let value = self.expr()?;
            let current = self.lookup(&name).cloned();
            let new = match op {
                "=" => value,
                "+=" => js_op_add(&current.unwrap_or(JsValue::Undefined), &value),
                _ => js_op_num(&op[..op.len() - 1], &current.unwrap_or(JsValue::Undefined), &value),
            };
            self.assign(&name, new.clone())?;
            return Ok(new);
        }
        self.conditional()
    }

    fn conditional(&mut self) -> JsEvalResult {
        let test = self.binary(0)?;
        if !self.eat("?") {
            return Ok(test);
        }
        let a = self.expr()?;
        self.expect(":")?;
        let b = self.expr()?;
        Ok(if test.truthy() { a } else { b })
    }

    fn binary(&mut self, min: u8) -> JsEvalResult {
        let mut left = self.unary()?;
        loop {
            let Some(JsEvalTok::Punct(op)) = self.peek().cloned() else {
                if self.is_word("in") && min <= 7 {
                    self.pos += 1;
                    let right = self.binary(8)?;
                    left = JsValue::Bool(js_has(&right, &js_key_text(&left)));
                    continue;
                }
                break;
            };
            let prec = match op {
                "??" => 1,
                "||" => 2,
                "&&" => 3,
                "|" => 4,
                "&" => 5,
                "==" | "!=" | "===" | "!==" => 6,
                "<" | ">" | "<=" | ">=" => 7,
                "<<" | ">>" => 8,
                "+" | "-" => 9,
                "*" | "/" | "%" => 10,
                "**" => 11,
                _ => break,
            };
            if prec < min {
                break;
            }
            self.pos += 1;
            // `**` groups to the right.
            let next = if op == "**" { prec } else { prec + 1 };
            // `a && b`, `a || b`, `a ?? b`: the right side only when needed.
            if matches!(op, "&&" | "||" | "??") {
                let take_right = match op {
                    "&&" => left.truthy(),
                    "||" => !left.truthy(),
                    _ => left.is_nullish(),
                };
                let right = self.binary(next)?;
                if take_right {
                    left = right;
                }
                continue;
            }
            let right = self.binary(next)?;
            left = match op {
                "+" => js_op_add(&left, &right),
                "-" | "*" | "/" | "%" | "**" => js_op_num(op, &left, &right),
                "<" | ">" | "<=" | ">=" => JsValue::Bool(js_op_cmp(op, &left, &right)),
                "===" => JsValue::Bool(js_strict_eq(&left, &right)),
                "!==" => JsValue::Bool(!js_strict_eq(&left, &right)),
                "==" => JsValue::Bool(js_loose_eq(&left, &right)),
                "!=" => JsValue::Bool(!js_loose_eq(&left, &right)),
                _ => {
                    let (a, b) = (js_to_int32(left.to_number()), js_to_int32(right.to_number()));
                    JsValue::Int(match op {
                        "&" => (a & b) as i64,
                        "|" => (a | b) as i64,
                        "<<" => a.wrapping_shl(b as u32) as i64,
                        _ => a.wrapping_shr(b as u32) as i64,
                    })
                }
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> JsEvalResult {
        if self.eat("!") {
            return Ok(JsValue::Bool(!self.unary()?.truthy()));
        }
        if self.eat("-") {
            return Ok(JsValue::number(-self.unary()?.to_number()));
        }
        if self.eat("+") {
            return Ok(JsValue::number(self.unary()?.to_number()));
        }
        if self.is_word("typeof") {
            self.pos += 1;
            // `typeof x` of an unknown name is "undefined".
            if let Some(JsEvalTok::Ident(n)) = self.peek().cloned()
                && self.lookup(&n).is_none()
                && js_eval_global(&n).is_none()
            {
                self.pos += 1;
                return Ok(JsValue::Str("undefined".into()));
            }
            return Ok(JsValue::Str(js_typeof(&self.unary()?).into()));
        }
        self.postfix()
    }

    fn args(&mut self) -> Result<Vec<JsValue>, String> {
        let mut args = Vec::new();
        while !self.eat(")") {
            if self.eat("...") {
                args.extend(js_items(&self.expr()?));
            } else {
                args.push(self.expr()?);
            }
            if !self.eat(",") {
                self.expect(")")?;
                break;
            }
        }
        Ok(args)
    }

    fn postfix(&mut self) -> JsEvalResult {
        // `Math.max(…)`, `JSON.stringify(…)`: functions of the global objects.
        if let (Some(JsEvalTok::Ident(ns)), Some(JsEvalTok::Punct(".")), Some(JsEvalTok::Ident(f)), Some(JsEvalTok::Punct("("))) = (
            self.tokens.get(self.pos).cloned(),
            self.tokens.get(self.pos + 1).cloned(),
            self.tokens.get(self.pos + 2).cloned(),
            self.tokens.get(self.pos + 3).cloned(),
        ) && self.lookup(&ns).is_none()
            && matches!(ns.as_str(), "Math" | "JSON" | "Number" | "Object" | "Array" | "String")
        {
            self.pos += 4;
            let args = self.args()?;
            let mut v = js_eval_call(&format!("{ns}.{f}"), args)?;
            return self.member_chain(&mut v);
        }
        let mut v = self.primary()?;
        self.member_chain(&mut v)
    }

    fn member_chain(&mut self, v: &mut JsValue) -> JsEvalResult {
        loop {
            let optional = self.eat("?.");
            if optional || self.eat(".") {
                if optional && self.is("[") {
                    continue;
                }
                let name = self.ident()?;
                if optional && v.is_nullish() {
                    // Skip the rest of the chain.
                    while self.eat(".") || self.eat("?.") {
                        self.ident()?;
                        if self.eat("(") {
                            self.args()?;
                        }
                    }
                    return Ok(JsValue::Undefined);
                }
                if self.eat("(") {
                    let args = self.args()?;
                    *v = js_call_method(v, &name, args);
                } else {
                    *v = js_get(v, &name);
                }
            } else if self.eat("[") {
                let k = self.expr()?;
                self.expect("]")?;
                *v = js_index(v, &k);
            } else {
                return Ok(v.clone());
            }
        }
    }

    fn primary(&mut self) -> JsEvalResult {
        let Some(t) = self.peek().cloned() else {
            return Err("SyntaxError: Unexpected end of input".into());
        };
        self.pos += 1;
        match t {
            JsEvalTok::Num(x) => Ok(JsValue::number(x)),
            JsEvalTok::Str(s) => Ok(JsValue::Str(s)),
            JsEvalTok::Punct("(") => {
                let v = self.expr()?;
                // `(a, b)`: the last.
                let mut v = v;
                while self.eat(",") {
                    v = self.expr()?;
                }
                self.expect(")")?;
                Ok(v)
            }
            JsEvalTok::Punct("[") => {
                let mut items = Vec::new();
                while !self.eat("]") {
                    if self.eat("...") {
                        items.extend(js_items(&self.expr()?));
                    } else {
                        items.push(self.expr()?);
                    }
                    if !self.eat(",") {
                        self.expect("]")?;
                        break;
                    }
                }
                Ok(JsValue::Array(items))
            }
            JsEvalTok::Punct("{") => {
                let mut o = JsValue::Object(None, Vec::new());
                while !self.eat("}") {
                    if self.eat("...") {
                        let src = self.expr()?;
                        for (k, x) in js_entries(&src) {
                            js_set(&mut o, &k, x);
                        }
                    } else {
                        let key = match self.peek().cloned() {
                            Some(JsEvalTok::Ident(k)) | Some(JsEvalTok::Str(k)) => {
                                self.pos += 1;
                                k
                            }
                            Some(JsEvalTok::Num(n)) => {
                                self.pos += 1;
                                js_to_string(&JsValue::number(n))
                            }
                            Some(JsEvalTok::Punct("[")) => {
                                self.pos += 1;
                                let k = self.expr()?;
                                self.expect("]")?;
                                js_key_text(&k)
                            }
                            _ => return Err("SyntaxError: Unexpected token in object".into()),
                        };
                        let x = if self.eat(":") {
                            self.expr()?
                        } else {
                            self.lookup(&key).cloned().unwrap_or(JsValue::Undefined)
                        };
                        js_set(&mut o, &key, x);
                    }
                    if !self.eat(",") {
                        self.expect("}")?;
                        break;
                    }
                }
                Ok(o)
            }
            JsEvalTok::Ident(name) => {
                if let Some(v) = self.lookup(&name) {
                    let v = v.clone();
                    if self.eat("(") {
                        let args = self.args()?;
                        return Err(format!(
                            "TypeError: {name} is not a function ({} arguments)",
                            args.len()
                        ));
                    }
                    return Ok(v);
                }
                if self.eat("(") {
                    let args = self.args()?;
                    return js_eval_call(&name, args);
                }
                js_eval_global(&name).ok_or_else(|| format!("ReferenceError: {name} is not defined"))
            }
            JsEvalTok::Punct(p) => Err(format!("SyntaxError: Unexpected token '{p}'")),
        }
    }
}

/// The global values eval reads.
fn js_eval_global(name: &str) -> Option<JsValue> {
    Some(match name {
        "true" => JsValue::Bool(true),
        "false" => JsValue::Bool(false),
        "null" => JsValue::Null,
        "undefined" => JsValue::Undefined,
        "NaN" => JsValue::Float(f64::NAN),
        "Infinity" => JsValue::Float(f64::INFINITY),
        _ => return None,
    })
}

/// The global functions eval calls.
fn js_eval_call(name: &str, args: Vec<JsValue>) -> JsEvalResult {
    let num = |i: usize| args.get(i).map_or(f64::NAN, |a| a.to_number());
    let n = |x: f64| JsValue::number(x);
    Ok(match name {
        "Math.abs" => n(num(0).abs()),
        "Math.floor" => n(num(0).floor()),
        "Math.ceil" => n(num(0).ceil()),
        "Math.round" => n((num(0) + 0.5).floor()),
        "Math.trunc" => n(num(0).trunc()),
        "Math.sign" => n(if num(0) == 0.0 { 0.0 } else { num(0).signum() }),
        "Math.sqrt" => n(num(0).sqrt()),
        "Math.cbrt" => n(num(0).cbrt()),
        "Math.pow" => n(num(0).powf(num(1))),
        "Math.exp" => n(num(0).exp()),
        "Math.log" => n(num(0).ln()),
        "Math.log2" => n(num(0).log2()),
        "Math.log10" => n(num(0).log10()),
        "Math.sin" => n(num(0).sin()),
        "Math.cos" => n(num(0).cos()),
        "Math.tan" => n(num(0).tan()),
        "Math.atan2" => n(num(0).atan2(num(1))),
        "Math.hypot" => n(args.iter().map(|a| a.to_number().powi(2)).sum::<f64>().sqrt()),
        "Math.max" => n(args.iter().map(|a| a.to_number()).fold(f64::NEG_INFINITY, |a, b| if a.is_nan() || b.is_nan() { f64::NAN } else { a.max(b) })),
        "Math.min" => n(args.iter().map(|a| a.to_number()).fold(f64::INFINITY, |a, b| if a.is_nan() || b.is_nan() { f64::NAN } else { a.min(b) })),
        "Number" | "Number.parseFloat" | "parseFloat" if name != "parseFloat" => n(num(0)),
        "parseFloat" | "Number.parseFloat" => {
            let s = args.first().map(js_to_string).unwrap_or_default();
            let s = s.trim_start();
            let end = s
                .char_indices()
                .take_while(|(i, c)| c.is_ascii_digit() || *c == '.' || (*i == 0 && (*c == '-' || *c == '+')))
                .count();
            n(s[..end].parse().unwrap_or(f64::NAN))
        }
        "parseInt" | "Number.parseInt" => {
            let s = args.first().map(js_to_string).unwrap_or_default();
            let s = s.trim_start();
            let end = s
                .char_indices()
                .take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+')))
                .count();
            n(s[..end].parse::<i64>().map_or(f64::NAN, |v| v as f64))
        }
        "String" => JsValue::Str(args.first().map(js_to_string).unwrap_or_default()),
        "Boolean" => JsValue::Bool(args.first().is_some_and(|a| a.truthy())),
        "isNaN" => JsValue::Bool(num(0).is_nan()),
        "Number.isInteger" => JsValue::Bool(num(0).fract() == 0.0 && num(0).is_finite()),
        "Array.isArray" => JsValue::Bool(matches!(args.first(), Some(JsValue::Array(_)))),
        "Object.keys" => JsValue::Array(args.first().map(js_keys).unwrap_or_default().into_iter().map(JsValue::Str).collect()),
        "Object.values" => JsValue::Array(args.first().map(js_values).unwrap_or_default()),
        "JSON.stringify" => JsValue::Str(js_json_stringify(&args.first().cloned().unwrap_or(JsValue::Undefined), "")),
        "JSON.parse" => js_json_parse_value(&args.first().map(js_to_string).unwrap_or_default())
            .map_err(|e| format!("SyntaxError: {e}"))?,
        _ => return Err(format!("TypeError: {name} is not a function")),
    })
}
