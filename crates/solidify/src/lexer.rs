//! Tokens of JavaScript and TypeScript: identifiers (keywords included), numbers, strings,
//! template literals (with their nested expressions already tokenized), regular expressions
//! and punctuators. Comments are kept apart, so that the generated Rust can carry them.

use crate::error::{Error, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// An identifier or a keyword: the parser decides, by context.
    Ident(String),
    /// `#private` names of classes.
    Private(String),
    Num {
        value: f64,
        raw: String,
        /// Written without a fractional part or an exponent (`3`, `0xff`): an integer
        /// literal, which may still become a float by inference.
        int: bool,
    },
    /// `123n`: the digits as written, with their prefix (`0x…`).
    BigInt(String),
    Str(String),
    /// `` `a${b}c` ``: the cooked text parts (one more than the expressions) and the tokens
    /// of each `${…}`.
    Template {
        quasis: Vec<String>,
        exprs: Vec<Vec<Token>>,
    },
    Regex {
        pattern: String,
        flags: String,
    },
    Punct(&'static str),
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
    /// A line break separates this token from the previous one (automatic semicolons).
    pub nl_before: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comment {
    /// The text without `//` or `/* */`, one line per element.
    pub lines: Vec<String>,
    pub span: Span,
}

/// Longest first, so that `>>>=` wins over `>`.
const PUNCTS: &[&str] = &[
    ">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=", "=>", "==", "!=",
    "<=", ">=", "&&", "||", "??", "?.", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "<<", ">>", "**", "{", "}", "(", ")", "[", "]", ";", ",", "<", ">", "+", "-", "*", "/", "%",
    "&", "|", "^", "!", "~", "?", ":", "=", ".",
];

/// Keywords after which a `/` starts a regular expression rather than a division.
const REGEX_AFTER: &[&str] = &[
    "return",
    "typeof",
    "case",
    "do",
    "else",
    "in",
    "instanceof",
    "new",
    "throw",
    "void",
    "delete",
    "yield",
    "await",
    "of",
    "extends",
];

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    pub comments: Vec<Comment>,
}

pub fn lex(src: &str) -> Result<(Vec<Token>, Vec<Comment>), Error> {
    let mut lexer = Lexer {
        src,
        bytes: src.as_bytes(),
        pos: 0,
        comments: Vec::new(),
    };
    let tokens = lexer.run(None)?;
    Ok((tokens, lexer.comments))
}

impl<'a> Lexer<'a> {
    /// Tokenizes until the end of the input, or, inside `${…}`, until the closing brace.
    fn run(&mut self, until_brace: Option<Span>) -> Result<Vec<Token>, Error> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut depth = 0usize;
        loop {
            let nl_before = self.skip_trivia()?;
            let start = self.pos;
            let Some(&c) = self.bytes.get(self.pos) else {
                if let Some(open) = until_brace {
                    return Err(Error::new("unterminated `${` in template literal", open));
                }
                tokens.push(Token {
                    tok: Tok::Eof,
                    span: Span::new(start, start),
                    nl_before,
                });
                return Ok(tokens);
            };
            let tok = if c == b'}' && until_brace.is_some() && depth == 0 {
                self.pos += 1;
                return Ok(tokens);
            } else if c.is_ascii_digit() || (c == b'.' && self.peek_digit(1)) {
                self.number()?
            } else if c == b'"' || c == b'\'' {
                self.string(c)?
            } else if c == b'`' {
                self.template()?
            } else if c == b'#' {
                self.pos += 1;
                let name = self.ident_text();
                if name.is_empty() {
                    return Err(Error::new("unexpected `#`", Span::new(start, start + 1)));
                }
                Tok::Private(name)
            } else if is_ident_start(c) || c == b'\\' || c >= 0x80 {
                let text = self.ident_text();
                if text.is_empty() {
                    return Err(Error::new(
                        format!("unexpected character {:?}", self.char_at(start)),
                        Span::new(start, start + self.char_at(start).len_utf8()),
                    ));
                }
                Tok::Ident(text)
            } else if c == b'/' && regex_allowed(tokens.last()) {
                self.regex()?
            } else {
                let Some(p) = PUNCTS.iter().find(|p| self.src[self.pos..].starts_with(*p)) else {
                    return Err(Error::new(
                        format!("unexpected character {:?}", self.char_at(start)),
                        Span::new(start, start + self.char_at(start).len_utf8()),
                    ));
                };
                self.pos += p.len();
                match *p {
                    "{" => depth += 1,
                    "}" => depth = depth.saturating_sub(1),
                    _ => {}
                }
                Tok::Punct(p)
            };
            tokens.push(Token {
                tok,
                span: Span::new(start, self.pos),
                nl_before,
            });
        }
    }

    fn char_at(&self, pos: usize) -> char {
        self.src[pos..].chars().next().unwrap_or('\0')
    }

    fn peek_digit(&self, offset: usize) -> bool {
        self.bytes
            .get(self.pos + offset)
            .is_some_and(u8::is_ascii_digit)
    }

    /// Skips spaces and comments; returns whether a line break was crossed.
    fn skip_trivia(&mut self) -> Result<bool, Error> {
        let mut newline = false;
        loop {
            let Some(&c) = self.bytes.get(self.pos) else {
                return Ok(newline);
            };
            match c {
                b'\n' => {
                    newline = true;
                    self.pos += 1;
                }
                b' ' | b'\t' | b'\r' | 0x0b | 0x0c => self.pos += 1,
                b'/' if self.bytes.get(self.pos + 1) == Some(&b'/') => {
                    let start = self.pos;
                    while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                    let text = self.src[start + 2..self.pos].trim().to_string();
                    self.comments.push(Comment {
                        lines: vec![text],
                        span: Span::new(start, self.pos),
                    });
                }
                b'/' if self.bytes.get(self.pos + 1) == Some(&b'*') => {
                    let start = self.pos;
                    let Some(len) = self.src[start + 2..].find("*/") else {
                        return Err(Error::new(
                            "unterminated comment",
                            Span::new(start, start + 2),
                        ));
                    };
                    let body = &self.src[start + 2..start + 2 + len];
                    if body.contains('\n') {
                        newline = true;
                    }
                    self.pos = start + 2 + len + 2;
                    let lines = body
                        .lines()
                        .map(|l| l.trim().trim_start_matches('*').trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect();
                    self.comments.push(Comment {
                        lines,
                        span: Span::new(start, self.pos),
                    });
                }
                0xc2 if self.bytes.get(self.pos + 1) == Some(&0xa0) => self.pos += 2, // nbsp
                0xe2 if self.src[self.pos..].starts_with('\u{2028}')
                    || self.src[self.pos..].starts_with('\u{2029}') =>
                {
                    newline = true;
                    self.pos += 3;
                }
                0xef if self.src[self.pos..].starts_with('\u{feff}') => self.pos += 3,
                _ => return Ok(newline),
            }
        }
    }

    fn ident_text(&mut self) -> String {
        let start = self.pos;
        let mut out = String::new();
        for c in self.src[start..].chars() {
            let ok = if out.is_empty() {
                c == '$' || c == '_' || c.is_alphabetic()
            } else {
                c == '$' || c == '_' || c.is_alphanumeric()
            };
            if !ok {
                break;
            }
            out.push(c);
            self.pos += c.len_utf8();
        }
        out
    }

    fn number(&mut self) -> Result<Tok, Error> {
        let start = self.pos;
        let rest = &self.src[start..];
        let lower = rest.get(..2).map(|p| p.to_ascii_lowercase());
        let (radix, skip): (u32, usize) = match lower.as_deref() {
            Some("0x") => (16, 2),
            Some("0o") => (8, 2),
            Some("0b") => (2, 2),
            _ => (10, 0),
        };
        self.pos += skip;
        let mut digits = String::new();
        let mut int = true;
        if radix != 10 {
            while let Some(&c) = self.bytes.get(self.pos) {
                if c == b'_' {
                    self.pos += 1;
                } else if (c as char).is_digit(radix) {
                    digits.push(c as char);
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if digits.is_empty() {
                return Err(Error::new("malformed number", Span::new(start, self.pos)));
            }
        } else {
            self.digits(&mut digits);
            if self.bytes.get(self.pos) == Some(&b'.') {
                int = false;
                digits.push('.');
                self.pos += 1;
                self.digits(&mut digits);
            }
            if matches!(self.bytes.get(self.pos), Some(b'e' | b'E')) {
                let save = self.pos;
                let mut exp = String::from("e");
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(b'+' | b'-')) {
                    exp.push(self.bytes[self.pos] as char);
                    self.pos += 1;
                }
                let before = exp.len();
                self.digits(&mut exp);
                if exp.len() == before {
                    self.pos = save;
                } else {
                    int = false;
                    digits.push_str(&exp);
                }
            }
        }
        if self.bytes.get(self.pos) == Some(&b'n') {
            if !int {
                return Err(Error::new(
                    "a BigInt literal is an integer",
                    Span::new(start, self.pos + 1),
                ));
            }
            let text = self.src[start..self.pos].replace('_', "");
            self.pos += 1;
            return Ok(Tok::BigInt(text));
        }
        if self
            .bytes
            .get(self.pos)
            .is_some_and(|&c| is_ident_start(c) || c.is_ascii_digit())
        {
            return Err(Error::new(
                "identifier directly after a number",
                Span::new(start, self.pos + 1),
            ));
        }
        let value = if radix == 10 {
            let text = if digits.starts_with('.') {
                format!("0{digits}")
            } else {
                digits.clone()
            };
            text.parse::<f64>()
                .map_err(|_| Error::new("malformed number", Span::new(start, self.pos)))?
        } else {
            u64::from_str_radix(&digits, radix)
                .map_err(|_| Error::new("number too large", Span::new(start, self.pos)))?
                as f64
        };
        Ok(Tok::Num {
            value,
            raw: self.src[start..self.pos].to_string(),
            int,
        })
    }

    fn digits(&mut self, out: &mut String) {
        while let Some(&c) = self.bytes.get(self.pos) {
            if c == b'_' {
                self.pos += 1;
            } else if c.is_ascii_digit() {
                out.push(c as char);
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn string(&mut self, quote: u8) -> Result<Tok, Error> {
        let start = self.pos;
        self.pos += 1;
        let mut out = String::new();
        loop {
            let Some(&c) = self.bytes.get(self.pos) else {
                return Err(Error::new(
                    "unterminated string",
                    Span::new(start, self.pos),
                ));
            };
            match c {
                b'\n' => {
                    return Err(Error::new(
                        "unterminated string",
                        Span::new(start, self.pos),
                    ));
                }
                b'\\' => self.escape(&mut out, false)?,
                c if c == quote => {
                    self.pos += 1;
                    return Ok(Tok::Str(out));
                }
                _ => {
                    let ch = self.char_at(self.pos);
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    /// Reads an escape sequence at `\`; in templates, `\${` keeps the dollar.
    fn escape(&mut self, out: &mut String, template: bool) -> Result<(), Error> {
        let start = self.pos;
        self.pos += 1;
        let Some(&c) = self.bytes.get(self.pos) else {
            return Err(Error::new(
                "unterminated escape",
                Span::new(start, self.pos),
            ));
        };
        self.pos += 1;
        match c {
            b'n' => out.push('\n'),
            b't' => out.push('\t'),
            b'r' => out.push('\r'),
            b'b' => out.push('\u{8}'),
            b'f' => out.push('\u{c}'),
            b'v' => out.push('\u{b}'),
            b'0' if !self.peek_digit(0) => out.push('\0'),
            b'\n' => {}
            b'\r' => {
                if self.bytes.get(self.pos) == Some(&b'\n') {
                    self.pos += 1;
                }
            }
            b'x' => {
                let hex = self.src.get(self.pos..self.pos + 2).unwrap_or("");
                let code = u32::from_str_radix(hex, 16)
                    .map_err(|_| Error::new("bad \\x escape", Span::new(start, self.pos)))?;
                self.pos += 2;
                out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
            }
            b'u' => {
                let code = if self.bytes.get(self.pos) == Some(&b'{') {
                    let end = self.src[self.pos..]
                        .find('}')
                        .ok_or_else(|| Error::new("bad \\u escape", Span::new(start, self.pos)))?;
                    let hex = &self.src[self.pos + 1..self.pos + end];
                    self.pos += end + 1;
                    u32::from_str_radix(hex, 16)
                        .map_err(|_| Error::new("bad \\u escape", Span::new(start, self.pos)))?
                } else {
                    let hex = self.src.get(self.pos..self.pos + 4).unwrap_or("");
                    let code = u32::from_str_radix(hex, 16)
                        .map_err(|_| Error::new("bad \\u escape", Span::new(start, self.pos)))?;
                    self.pos += 4;
                    // A surrogate pair written as two escapes.
                    if (0xd800..0xdc00).contains(&code)
                        && self.src[self.pos..].starts_with("\\u")
                        && let Some(hex) = self.src.get(self.pos + 2..self.pos + 6)
                        && let Ok(low) = u32::from_str_radix(hex, 16)
                        && (0xdc00..0xe000).contains(&low)
                    {
                        self.pos += 6;
                        0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00)
                    } else {
                        code
                    }
                };
                out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
            }
            _ => {
                let ch = self.char_at(self.pos - 1);
                if template && ch == '$' {
                    out.push('$');
                } else {
                    out.push(ch);
                }
                self.pos += ch.len_utf8() - 1;
            }
        }
        Ok(())
    }

    fn template(&mut self) -> Result<Tok, Error> {
        let start = self.pos;
        self.pos += 1;
        let mut quasis = Vec::new();
        let mut exprs = Vec::new();
        let mut current = String::new();
        loop {
            let Some(&c) = self.bytes.get(self.pos) else {
                return Err(Error::new(
                    "unterminated template literal",
                    Span::new(start, self.pos),
                ));
            };
            match c {
                b'`' => {
                    self.pos += 1;
                    quasis.push(std::mem::take(&mut current));
                    return Ok(Tok::Template { quasis, exprs });
                }
                b'\\' => self.escape(&mut current, true)?,
                b'$' if self.bytes.get(self.pos + 1) == Some(&b'{') => {
                    let open = Span::new(self.pos, self.pos + 2);
                    self.pos += 2;
                    quasis.push(std::mem::take(&mut current));
                    let tokens = self.run(Some(open))?;
                    if tokens.is_empty() {
                        return Err(Error::new("empty `${}` in template literal", open));
                    }
                    exprs.push(tokens);
                }
                b'\r' => {
                    self.pos += 1;
                    if self.bytes.get(self.pos) != Some(&b'\n') {
                        current.push('\n');
                    }
                }
                _ => {
                    let ch = self.char_at(self.pos);
                    current.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn regex(&mut self) -> Result<Tok, Error> {
        let start = self.pos;
        self.pos += 1;
        let mut in_class = false;
        loop {
            let Some(&c) = self.bytes.get(self.pos) else {
                return Err(Error::new(
                    "unterminated regular expression",
                    Span::new(start, self.pos),
                ));
            };
            match c {
                b'\n' => {
                    return Err(Error::new(
                        "unterminated regular expression",
                        Span::new(start, self.pos),
                    ));
                }
                b'\\' => self.pos += 2,
                b'[' => {
                    in_class = true;
                    self.pos += 1;
                }
                b']' => {
                    in_class = false;
                    self.pos += 1;
                }
                b'/' if !in_class => break,
                _ => self.pos += self.char_at(self.pos).len_utf8(),
            }
        }
        let pattern = self.src[start + 1..self.pos].to_string();
        self.pos += 1;
        let flags = self.ident_text();
        Ok(Tok::Regex { pattern, flags })
    }
}

fn is_ident_start(c: u8) -> bool {
    c == b'$' || c == b'_' || c.is_ascii_alphabetic()
}

fn regex_allowed(prev: Option<&Token>) -> bool {
    match prev.map(|t| &t.tok) {
        None => true,
        Some(Tok::Ident(word)) => REGEX_AFTER.contains(&word.as_str()),
        Some(Tok::Punct(p)) => !matches!(*p, ")" | "]" | "}"),
        Some(
            Tok::Num { .. }
            | Tok::BigInt(_)
            | Tok::Str(_)
            | Tok::Template { .. }
            | Tok::Regex { .. },
        ) => false,
        Some(Tok::Private(_) | Tok::Eof) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        lex(src).unwrap().0.into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn numbers_strings_and_punctuation() {
        let t = toks("let x = 0xff + 1_000.5 - .5e2; 'a\\n' \"b\"");
        assert_eq!(t[0], Tok::Ident("let".into()));
        assert_eq!(t[2], Tok::Punct("="));
        assert!(matches!(&t[3], Tok::Num { value, int: true, .. } if *value == 255.0));
        assert!(matches!(&t[5], Tok::Num { value, int: false, .. } if *value == 1000.5));
        assert!(matches!(&t[7], Tok::Num { value, int: false, .. } if *value == 50.0));
        assert_eq!(t[9], Tok::Str("a\n".into()));
        assert_eq!(t[10], Tok::Str("b".into()));
    }

    #[test]
    fn templates_nest() {
        let t = toks("`a${`b${c}`}d`");
        let Tok::Template { quasis, exprs } = &t[0] else {
            panic!("{t:?}");
        };
        assert_eq!(quasis, &["a", "d"]);
        assert!(matches!(exprs[0][0].tok, Tok::Template { .. }));
    }

    #[test]
    fn regex_or_division() {
        assert!(matches!(toks("a / b / c")[1], Tok::Punct("/")));
        assert!(matches!(toks("x = /a\\/b/gi")[2], Tok::Regex { .. }));
        assert!(matches!(toks("return /re/")[1], Tok::Regex { .. }));
    }

    #[test]
    fn comments_and_newlines() {
        let (tokens, comments) = lex("a // one\n/* two\n * lines */ b").unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].lines, vec!["one"]);
        assert_eq!(comments[1].lines, vec!["two", "lines"]);
        assert!(tokens[1].nl_before);
    }
}
