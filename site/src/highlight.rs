//! A small syntax highlighter for TypeScript and Rust, used by the server for the static
//! examples and by the browser for the live output. Produces escaped HTML with
//! `<span class="…">` around comments (`c`), strings (`s`), numbers (`n`), keywords (`k`),
//! types (`t`), function names (`f`) and attributes/macros (`m`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    TypeScript,
    Rust,
}

const TS_KEYWORDS: &[&str] = &[
    "abstract",
    "as",
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "get",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "of",
    "private",
    "protected",
    "public",
    "readonly",
    "return",
    "set",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "yield",
];

const TS_TYPES: &[&str] = &[
    "number", "string", "boolean", "void", "any", "unknown", "never", "object",
];

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];

const RUST_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char", "str", "String", "Vec", "Option", "Result", "Box", "HashMap", "HashSet",
    "Some", "None", "Ok", "Err",
];

/// Highlights `code` as HTML.
pub fn highlight(code: &str, lang: Language) -> String {
    let (keywords, types) = match lang {
        Language::TypeScript => (TS_KEYWORDS, TS_TYPES),
        Language::Rust => (RUST_KEYWORDS, RUST_TYPES),
    };
    let bytes = code.as_bytes();
    let mut out = String::with_capacity(code.len() * 2);
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        // Comments.
        if c == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let end = code[i..].find('\n').map_or(code.len(), |n| i + n);
            span(&mut out, "c", &code[i..end]);
            i = end;
            continue;
        }
        if c == b'/' && bytes.get(i + 1) == Some(&b'*') {
            let end = code[i + 2..].find("*/").map_or(code.len(), |n| i + n + 4);
            span(&mut out, "c", &code[i..end.min(code.len())]);
            i = end.min(code.len());
            continue;
        }
        // Attributes and macros (Rust).
        if lang == Language::Rust && c == b'#' && bytes.get(i + 1) == Some(&b'[') {
            let end = code[i..].find(']').map_or(code.len(), |n| i + n + 1);
            span(&mut out, "m", &code[i..end]);
            i = end;
            continue;
        }
        // Strings.
        if c == b'"' || c == b'\'' || c == b'`' {
            let end = string_end(bytes, i, c);
            span(&mut out, "s", &code[i..end]);
            i = end;
            continue;
        }
        // Rust lifetimes and chars: 'a, 'x'.
        // Numbers.
        if c.is_ascii_digit() {
            let mut end = i + 1;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'.')
            {
                if bytes[end] == b'.' && bytes.get(end + 1) == Some(&b'.') {
                    break;
                }
                end += 1;
            }
            span(&mut out, "n", &code[i..end]);
            i = end;
            continue;
        }
        // Words.
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let mut end = i + 1;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'$')
            {
                end += 1;
            }
            let word = &code[i..end];
            let next = bytes.get(end).copied();
            let after_macro = lang == Language::Rust && next == Some(b'!');
            let class = if keywords.contains(&word) {
                Some("k")
            } else if types.contains(&word)
                || word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            {
                Some("t")
            } else if after_macro {
                Some("m")
            } else if next == Some(b'(')
                || (lang == Language::Rust
                    && bytes.get(end) == Some(&b':')
                    && bytes.get(end + 1) == Some(&b':'))
            {
                Some("f")
            } else {
                None
            };
            match class {
                Some(cl) => span(&mut out, cl, word),
                None => escape_into(&mut out, word),
            }
            i = end;
            continue;
        }
        escape_into(&mut out, &code[i..i + 1]);
        i += 1;
    }
    out
}

fn string_end(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            c if c == quote => return i + 1,
            b'\n' if quote != b'`' => return i,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn span(out: &mut String, class: &str, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(class);
    out.push_str("\">");
    escape_into(out, text);
    out.push_str("</span>");
}

/// Shell commands and TOML: comments (a whole line, or after ` #`) dimmed, strings in
/// double quotes marked.
pub fn shell(text: &str) -> String {
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let (code, comment) = if line.trim_start().starts_with('#') {
            ("", line)
        } else {
            match line.find(" #") {
                Some(at) => line.split_at(at + 1),
                None => (line, ""),
            }
        };
        // Odd parts are inside quotes; the last one may lack its closing quote.
        let parts: Vec<&str> = code.split('"').collect();
        for (n, part) in parts.iter().enumerate() {
            if n % 2 == 0 {
                escape_into(&mut out, part);
            } else if n + 1 < parts.len() {
                span(&mut out, "s", &format!("\"{part}\""));
            } else {
                span(&mut out, "s", &format!("\"{part}"));
            }
        }
        if !comment.is_empty() {
            span(&mut out, "c", comment);
        }
    }
    out
}

/// Escapes text for HTML.
pub fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_keywords_strings_and_comments() {
        let html = highlight("let x: number = \"a<b\"; // hi", Language::TypeScript);
        assert!(html.contains("<span class=\"k\">let</span>"));
        assert!(html.contains("<span class=\"t\">number</span>"));
        assert!(html.contains("<span class=\"s\">&quot;a&lt;b&quot;</span>"));
        assert!(html.contains("<span class=\"c\">// hi</span>"));
        let rust = highlight(
            "fn main() { println!(\"{}\", Vec::<i64>::new()); }",
            Language::Rust,
        );
        assert!(rust.contains("<span class=\"f\">main</span>"));
        assert!(rust.contains("<span class=\"m\">println</span>"));
        assert!(rust.contains("<span class=\"t\">Vec</span>"));
        assert_eq!(
            shell("# all\nnpx a \"b c\" # d\nx = \"y"),
            "<span class=\"c\"># all</span>\nnpx a <span class=\"s\">&quot;b c&quot;</span> <span class=\"c\"># d</span>\nx = <span class=\"s\">&quot;y</span>"
        );
    }
}
