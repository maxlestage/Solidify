//! A small Markdown renderer for the documentation, without dependency: the subset its
//! pages use.
//!
//! - Headings, with GitHub's anchors (`## Exit codes` gets `id="exit-codes"`), so links
//!   work the same on the site and on GitHub. The `#` title is the page's title, `##` and
//!   `###` make its table of contents.
//! - Paragraphs, lists (nested by indentation), tables, `---`, and quotes; a quote that
//!   starts with `[!NOTE]`, `[!TIP]` or `[!WARNING]` is a callout.
//! - Fenced code, highlighted (TypeScript, Rust, shell, TOML), with a copy button. A
//!   TypeScript block followed by a Rust block is one pair: "this becomes that". The
//!   words after the language are notes for the tests (`fragment`, `error`), not shown.
//! - Inline: `` `code` ``, `**strong**`, `*emphasis*`, `[links](…)` and `\` escapes. A
//!   link to `cli.md` or `cli.md#options` goes to that page of the documentation in the
//!   same language; another relative link goes to the file on GitHub.

use std::collections::HashMap;

use crate::catalog;
use crate::highlight::{self, Language, escape_into};
use crate::routes::{Lang, doc_href};

/// Where relative links that are not pages of the documentation lead.
pub const REPOSITORY: &str = "https://github.com/maxlestage/Solidify/blob/master";

#[derive(Clone, Debug, PartialEq)]
pub struct TocEntry {
    pub level: u8,
    pub id: String,
    pub text: String,
}

/// A rendered page.
pub struct Rendered {
    /// The text of its `#` heading.
    pub title: String,
    /// Its first paragraph, as plain text (the meta description).
    pub description: String,
    pub html: String,
    /// Its `##` and `###` headings.
    pub toc: Vec<TocEntry>,
}

/// The few words the rendered HTML needs, in the page's language.
pub struct Labels {
    pub copy: &'static str,
    pub copied: &'static str,
    pub note: &'static str,
    pub tip: &'static str,
    pub warning: &'static str,
    /// Between the two blocks of a pair ("becomes").
    pub becomes: &'static str,
}

/// Renders `source`, a page of the documentation in `lang`.
pub fn render(source: &str, lang: Lang, labels: &Labels) -> Rendered {
    let lines: Vec<&str> = source.lines().collect();
    let mut r = Renderer {
        lang,
        labels,
        slugs: Slugger::default(),
        out: String::new(),
        title: String::new(),
        description: String::new(),
        toc: Vec::new(),
    };
    r.blocks(&lines);
    Rendered {
        title: r.title,
        description: r.description,
        html: r.out,
        toc: r.toc,
    }
}

/// A fenced block of code: its language (the first word of the info string), the other
/// words, and its text.
#[derive(Clone, Debug, PartialEq)]
pub struct Fence {
    pub lang: String,
    pub notes: Vec<String>,
    pub code: String,
}

/// Every fenced block of `source`, in order, with whether a heading comes between it and
/// the previous one (the tests pair TypeScript with the Rust that follows it).
pub fn fences(source: &str) -> Vec<(Fence, bool)> {
    let mut out = Vec::new();
    let mut heading = false;
    let mut lines = source.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if let Some(info) = trimmed.strip_prefix("```") {
            let code: Vec<&str> = lines
                .by_ref()
                .take_while(|l| !l.trim_start().starts_with("```"))
                .collect();
            out.push((fence(info, code.join("\n")), heading));
            heading = false;
        } else if trimmed.starts_with('#') {
            heading = true;
        }
    }
    out
}

fn fence(info: &str, code: String) -> Fence {
    let mut words = info.split_whitespace();
    Fence {
        lang: words.next().unwrap_or("").to_ascii_lowercase(),
        notes: words.map(str::to_string).collect(),
        code,
    }
}

/// Heading ids as GitHub makes them: lowercase, punctuation removed, spaces as dashes,
/// `-1`, `-2`… after a repeated one.
#[derive(Default)]
pub struct Slugger {
    seen: HashMap<String, usize>,
}

impl Slugger {
    pub fn slug(&mut self, text: &str) -> String {
        let base: String = text
            .trim()
            .to_lowercase()
            .chars()
            .filter_map(|c| match c {
                ' ' => Some('-'),
                c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
                _ => None,
            })
            .collect();
        let count = self.seen.entry(base.clone()).or_insert(0);
        let slug = if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        };
        *count += 1;
        slug
    }
}

struct Renderer<'a> {
    lang: Lang,
    labels: &'a Labels,
    slugs: Slugger,
    out: String,
    title: String,
    description: String,
    toc: Vec<TocEntry>,
}

/// `(indent, ordered, where the item's text starts)` when `line` starts a list item.
fn list_marker(line: &str) -> Option<(usize, bool, usize)> {
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    if rest.starts_with("- ") || rest.starts_with("* ") {
        return Some((indent, false, indent + 2));
    }
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && rest[digits..].starts_with(". ") {
        return Some((indent, true, indent + digits + 2));
    }
    None
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if (1..=4).contains(&hashes) && line[hashes..].starts_with(' ') {
        Some((hashes as u8, line[hashes..].trim()))
    } else {
        None
    }
}

fn is_table_separator(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('|')
        && line.contains('-')
        && line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Whether `line` starts a block other than a paragraph.
fn starts_block(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```")
        || heading(t).is_some()
        || t.starts_with('>')
        || t.starts_with('|')
        || list_marker(line).is_some()
        || t == "---"
}

/// The cells of a table row; `\|` is a pipe inside a cell, in code too (as on GitHub).
fn cells(row: &str) -> Vec<String> {
    let row = row.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    let row = row.strip_suffix('|').unwrap_or(row);
    let mut cells = vec![String::new()];
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cells.last_mut().unwrap().push('|');
                chars.next();
            }
            '|' => cells.push(String::new()),
            c => cells.last_mut().unwrap().push(c),
        }
    }
    cells.into_iter().map(|c| c.trim().to_string()).collect()
}

/// The name shown above a block of code, and how to colour it.
fn code_language(lang: &str) -> (&'static str, &'static str) {
    match lang {
        "ts" | "typescript" => ("TypeScript", "ts"),
        "js" | "javascript" => ("JavaScript", "ts"),
        "rust" | "rs" => ("Rust", "rust"),
        "sh" | "shell" | "bash" | "console" => ("Shell", "sh"),
        "toml" => ("Cargo.toml", "toml"),
        _ => ("", "text"),
    }
}

impl Renderer<'_> {
    fn blocks(&mut self, lines: &[&str]) {
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            if trimmed.is_empty() {
                i += 1;
            } else if let Some(info) = trimmed.strip_prefix("```") {
                let (first, next) = self.read_fence(lines, i, info);
                i = next;
                // TypeScript followed by Rust: one pair.
                let mut j = i;
                while j < lines.len() && lines[j].trim().is_empty() {
                    j += 1;
                }
                let follows = lines
                    .get(j)
                    .and_then(|l| l.trim().strip_prefix("```"))
                    .map(|info| fence(info, String::new()).lang);
                if code_language(&first.lang).1 == "ts"
                    && follows
                        .as_deref()
                        .is_some_and(|l| code_language(l).1 == "rust")
                {
                    let info = lines[j].trim().trim_start_matches('`');
                    let (second, after) = self.read_fence(lines, j, info);
                    i = after;
                    self.out
                        .push_str("<div class=\"pair\" data-reveal=\"pair\">");
                    self.code(&first);
                    self.out
                        .push_str("<p class=\"pair-wire\" aria-hidden=\"true\"><span>");
                    escape_into(&mut self.out, self.labels.becomes);
                    self.out.push_str("</span></p>");
                    self.code(&second);
                    self.out.push_str("</div>");
                } else {
                    self.code(&first);
                }
            } else if let Some((level, text)) = heading(trimmed) {
                self.heading(level, text);
                i += 1;
            } else if trimmed == "---" {
                self.out.push_str("<hr/>");
                i += 1;
            } else if trimmed.starts_with('>') {
                let mut inner = Vec::new();
                while let Some(l) = lines.get(i).and_then(|l| l.trim_start().strip_prefix('>')) {
                    inner.push(l.strip_prefix(' ').unwrap_or(l));
                    i += 1;
                }
                self.quote(&inner);
            } else if trimmed.starts_with('|')
                && lines.get(i + 1).is_some_and(|l| is_table_separator(l))
            {
                let head = cells(trimmed);
                i += 2;
                let mut rows = Vec::new();
                while let Some(l) = lines.get(i).filter(|l| l.trim().starts_with('|')) {
                    rows.push(cells(l));
                    i += 1;
                }
                self.table(&head, &rows);
            } else if list_marker(line).is_some() {
                i = self.list(lines, i);
            } else {
                let mut text = vec![trimmed];
                i += 1;
                while let Some(l) = lines
                    .get(i)
                    .filter(|l| !l.trim().is_empty() && !starts_block(l))
                {
                    text.push(l.trim());
                    i += 1;
                }
                self.paragraph(&text.join(" "));
            }
        }
    }

    /// The fence opening at `lines[start]`, and the line after its end.
    fn read_fence(&self, lines: &[&str], start: usize, info: &str) -> (Fence, usize) {
        let mut i = start + 1;
        let mut code = Vec::new();
        while let Some(l) = lines.get(i).filter(|l| !l.trim_start().starts_with("```")) {
            code.push(*l);
            i += 1;
        }
        (fence(info, code.join("\n")), i + 1)
    }

    fn heading(&mut self, level: u8, text: &str) {
        let plain = plain(text);
        if level == 1 {
            if self.title.is_empty() {
                self.title = plain;
            }
            self.out.push_str("<h1 class=\"doc-title\">");
            self.out.push_str(&self.inline(text));
            self.out.push_str("</h1>");
            return;
        }
        let id = self.slugs.slug(&plain);
        if level <= 3 {
            self.toc.push(TocEntry {
                level,
                id: id.clone(),
                text: plain,
            });
        }
        // A `##` heading charges up as it comes into view (`data-reveal="line"`).
        let reveal = if level == 2 {
            " data-reveal=\"line\""
        } else {
            ""
        };
        self.out.push_str(&format!(
            "<h{level} id=\"{id}\"{reveal}>{}<a class=\"anchor\" href=\"#{id}\" aria-hidden=\"true\" tabindex=\"-1\">#</a></h{level}>",
            self.inline(text)
        ));
    }

    fn paragraph(&mut self, text: &str) {
        if self.description.is_empty() {
            self.description = plain(text);
        }
        self.out.push_str("<p>");
        self.out.push_str(&self.inline(text));
        self.out.push_str("</p>");
    }

    fn code(&mut self, fence: &Fence) {
        let (name, kind) = code_language(&fence.lang);
        let html = match kind {
            "ts" => highlight::highlight(&fence.code, Language::TypeScript),
            "rust" => highlight::highlight(&fence.code, Language::Rust),
            "sh" | "toml" => highlight::shell(&fence.code),
            _ => {
                let mut out = String::new();
                escape_into(&mut out, &fence.code);
                out
            }
        };
        let mut copy = String::new();
        escape_into(&mut copy, &fence.code);
        self.out.push_str(&format!(
            "<div class=\"code {kind}\"><div class=\"code-head\">"
        ));
        if !name.is_empty() {
            self.out
                .push_str(&format!("<span class=\"code-lang\">{name}</span>"));
        }
        self.out.push_str(&format!(
            "<button class=\"code-copy\" type=\"button\" data-copy=\"{copy}\" data-done=\"{}\">{}</button></div><pre><code>{html}</code></pre></div>",
            self.labels.copied, self.labels.copy
        ));
    }

    fn quote(&mut self, lines: &[&str]) {
        let kind = lines
            .first()
            .map(|l| l.trim())
            .and_then(|first| match first {
                "[!NOTE]" => Some(("note", self.labels.note)),
                "[!TIP]" => Some(("tip", self.labels.tip)),
                "[!WARNING]" | "[!IMPORTANT]" | "[!CAUTION]" => {
                    Some(("warning", self.labels.warning))
                }
                _ => None,
            });
        match kind {
            Some((class, label)) => {
                self.out.push_str(&format!(
                    "<div class=\"callout {class}\"><p class=\"callout-title\"><span class=\"callout-icon\" aria-hidden=\"true\"></span>{label}</p>"
                ));
                self.blocks(&lines[1..]);
                self.out.push_str("</div>");
            }
            None => {
                self.out.push_str("<blockquote>");
                self.blocks(lines);
                self.out.push_str("</blockquote>");
            }
        }
    }

    fn table(&mut self, head: &[String], rows: &[Vec<String>]) {
        self.out.push_str("<table class=\"doc-table\"><thead><tr>");
        for cell in head {
            self.out
                .push_str(&format!("<th>{}</th>", self.inline(cell)));
        }
        self.out.push_str("</tr></thead><tbody>");
        for row in rows {
            self.out.push_str("<tr>");
            for (n, cell) in row.iter().enumerate() {
                // Each cell carries its column's title, shown above it on small screens.
                let title = head.get(n).map(|h| plain(h)).unwrap_or_default();
                let mut attr = String::new();
                escape_into(&mut attr, &title);
                self.out.push_str(&format!(
                    "<td data-th=\"{attr}\">{}</td>",
                    self.inline(cell)
                ));
            }
            self.out.push_str("</tr>");
        }
        self.out.push_str("</tbody></table>");
    }

    /// The list starting at `lines[start]`; returns the line after it.
    fn list(&mut self, lines: &[&str], start: usize) -> usize {
        let (indent, ordered, _) = list_marker(lines[start]).expect("a list item");
        let tag = if ordered { "ol" } else { "ul" };
        self.out.push_str(&format!("<{tag}>"));
        let mut i = start;
        while let Some((at, o, from)) = lines.get(i).and_then(|l| list_marker(l)) {
            if at != indent || o != ordered {
                break;
            }
            let mut text = vec![lines[i][from..].trim()];
            i += 1;
            // Its other lines: indented, or simply following it.
            while let Some(l) = lines.get(i).filter(|l| !l.trim().is_empty()) {
                if list_marker(l).is_some() || (starts_block(l) && !l.starts_with(' ')) {
                    break;
                }
                text.push(l.trim());
                i += 1;
            }
            self.out.push_str("<li>");
            self.out.push_str(&self.inline(&text.join(" ")));
            if let Some((deeper, _, _)) = lines.get(i).and_then(|l| list_marker(l))
                && deeper > indent
            {
                i = self.list(lines, i);
            }
            self.out.push_str("</li>");
            // A blank line between two items of the same list.
            let mut j = i;
            while lines.get(j).is_some_and(|l| l.trim().is_empty()) {
                j += 1;
            }
            match lines.get(j).and_then(|l| list_marker(l)) {
                Some((at, o, _)) if at == indent && o == ordered => i = j,
                _ => break,
            }
        }
        self.out.push_str(&format!("</{tag}>"));
        i
    }

    /// Inline Markdown as HTML.
    fn inline(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        let find = |from: usize, pattern: &[char]| -> Option<usize> {
            (from..chars.len().saturating_sub(pattern.len() - 1))
                .find(|&k| chars[k..k + pattern.len()] == *pattern)
        };
        while i < chars.len() {
            let c = chars[i];
            match c {
                '\\' if chars
                    .get(i + 1)
                    .is_some_and(|n| "\\`*_[]()|#<>!".contains(*n)) =>
                {
                    escape_into(&mut out, &chars[i + 1].to_string());
                    i += 2;
                }
                '`' => {
                    let run = chars[i..].iter().take_while(|c| **c == '`').count();
                    let fence: Vec<char> = vec!['`'; run];
                    match find(i + run, &fence) {
                        Some(end) => {
                            let code: String = chars[i + run..end].iter().collect();
                            let code = if run > 1 { code.trim() } else { code.as_str() };
                            out.push_str("<code>");
                            escape_into(&mut out, code);
                            out.push_str("</code>");
                            i = end + run;
                        }
                        None => {
                            out.push_str(&"`".repeat(run));
                            i += run;
                        }
                    }
                }
                '*' if chars.get(i + 1) == Some(&'*') => match find(i + 2, &['*', '*']) {
                    Some(end) => {
                        let inner: String = chars[i + 2..end].iter().collect();
                        out.push_str(&format!("<strong>{}</strong>", self.inline(&inner)));
                        i = end + 2;
                    }
                    None => {
                        out.push_str("**");
                        i += 2;
                    }
                },
                '*' if chars.get(i + 1).is_some_and(|n| !n.is_whitespace()) => {
                    match find(i + 1, &['*']) {
                        Some(end) => {
                            let inner: String = chars[i + 1..end].iter().collect();
                            out.push_str(&format!("<em>{}</em>", self.inline(&inner)));
                            i = end + 1;
                        }
                        None => {
                            out.push('*');
                            i += 1;
                        }
                    }
                }
                '[' => match self.link_at(&chars, i) {
                    Some((html, next)) => {
                        out.push_str(&html);
                        i = next;
                    }
                    None => {
                        out.push('[');
                        i += 1;
                    }
                },
                c => {
                    escape_into(&mut out, &c.to_string());
                    i += 1;
                }
            }
        }
        out
    }

    /// The link `[text](destination)` starting at `chars[at]`, and where it ends.
    fn link_at(&self, chars: &[char], at: usize) -> Option<(String, usize)> {
        let mut depth = 0;
        let mut close = None;
        for (k, c) in chars.iter().enumerate().skip(at) {
            match c {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(k);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close?;
        if chars.get(close + 1) != Some(&'(') {
            return None;
        }
        let end = (close + 2..chars.len()).find(|&k| chars[k] == ')')?;
        let text: String = chars[at + 1..close].iter().collect();
        let dest: String = chars[close + 2..end].iter().collect();
        let (href, external) = rewrite_link(dest.trim(), self.lang);
        let mut attr = String::new();
        escape_into(&mut attr, &href);
        let extra = if external {
            " rel=\"external noopener\" target=\"_blank\""
        } else {
            ""
        };
        Some((
            format!("<a href=\"{attr}\"{extra}>{}</a>", self.inline(&text)),
            end + 1,
        ))
    }
}

/// Where a link of a page in `lang` leads, and whether it leaves the site.
pub fn rewrite_link(dest: &str, lang: Lang) -> (String, bool) {
    if dest.starts_with("http://") || dest.starts_with("https://") {
        return (dest.to_string(), true);
    }
    if dest.starts_with('#') || dest.starts_with("mailto:") {
        return (dest.to_string(), false);
    }
    let (path, anchor) = dest.split_once('#').unwrap_or((dest, ""));
    let name = path.trim_start_matches("./");
    if let Some(page) = name.strip_suffix(".md").and_then(catalog::find) {
        return (doc_href(lang, page.slug, Some(anchor)), false);
    }
    // Another file of the repository, relative to `docs/{lang}/`.
    let mut parts = vec!["docs", lang.code()];
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    let mut url = format!("{REPOSITORY}/{}", parts.join("/"));
    if !anchor.is_empty() {
        url = format!("{url}#{anchor}");
    }
    (url, true)
}

/// Inline Markdown as plain text: the marks removed, code kept as it is.
pub fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    let (mut code, mut target) = (false, false);
    while let Some(c) = chars.next() {
        if code {
            if c == '`' {
                code = false;
            } else {
                out.push(c);
            }
            continue;
        }
        if target {
            target = c != ')';
            continue;
        }
        match c {
            '\\' => out.extend(chars.next()),
            '`' => code = true,
            '*' | '[' => {}
            ']' if chars.peek() == Some(&'(') => {
                chars.next();
                target = true;
            }
            ']' => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABELS: Labels = Labels {
        copy: "Copy",
        copied: "Copied",
        note: "Note",
        tip: "Tip",
        warning: "Warning",
        becomes: "becomes",
    };

    #[test]
    fn renders_the_blocks_of_a_page() {
        let source = "# The CLI\n\nRuns **Solidify** on `main.ts`.\nSee [the library](library.md#errors).\n\n\
                      ## Exit codes\n\n- `0`: done\n- `1`: an error\n  in the program\n  - nested\n\n\
                      1. one\n2. two\n\n| Option | Meaning |\n| --- | --- |\n| `-o` | the output \\| file |\n\n\
                      > [!NOTE]\n> Read *this*.\n\n```sh\nnpx solidify-rs main.ts  # go\n```\n\n## Exit codes\n";
        let r = render(source, Lang::Fr, &LABELS);
        assert_eq!(r.title, "The CLI");
        assert_eq!(r.description, "Runs Solidify on main.ts. See the library.");
        assert!(r.html.contains("<strong>Solidify</strong>"));
        assert!(r.html.contains("<code>main.ts</code>"));
        assert!(r.html.contains("href=\"/fr/docs/library/#errors\""));
        assert!(
            r.html
                .contains("<h2 id=\"exit-codes\" data-reveal=\"line\">Exit codes")
        );
        assert!(
            r.html
                .contains("<h2 id=\"exit-codes-1\" data-reveal=\"line\">")
        );
        assert!(
            r.html.contains(
                "<li><code>1</code>: an error in the program<ul><li>nested</li></ul></li>"
            )
        );
        assert!(r.html.contains("<ol><li>one</li><li>two</li></ol>"));
        assert!(
            r.html
                .contains("<td data-th=\"Meaning\">the output | file</td>")
        );
        assert!(r.html.contains("class=\"callout note\""));
        assert!(r.html.contains("<em>this</em>"));
        assert!(r.html.contains("<span class=\"c\"># go</span>"));
        assert!(
            r.html
                .contains("data-copy=\"npx solidify-rs main.ts  # go\"")
        );
        assert_eq!(r.toc.len(), 2);
        assert_eq!(r.toc[0].id, "exit-codes");
    }

    #[test]
    fn pairs_typescript_with_the_rust_that_follows() {
        let source = "```ts\nlet x = 1;\n```\n\n```rust fragment\nlet x = 1;\n```\n";
        let r = render(source, Lang::En, &LABELS);
        assert!(r.html.starts_with("<div class=\"pair\""));
        assert_eq!(r.html.matches("class=\"code ").count(), 2);
        let found = fences(source);
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].0.lang, "rust");
        assert_eq!(found[1].0.notes, ["fragment"]);
    }

    #[test]
    fn links_lead_to_pages_or_to_github() {
        assert_eq!(
            rewrite_link("cli.md", Lang::En),
            ("/docs/cli/".into(), false)
        );
        assert_eq!(
            rewrite_link("./faq.md#why", Lang::Es),
            ("/es/docs/faq/#why".into(), false)
        );
        assert_eq!(
            rewrite_link("../../LICENSE", Lang::Fr),
            (format!("{REPOSITORY}/LICENSE"), true)
        );
        assert_eq!(rewrite_link("#top", Lang::En), ("#top".into(), false));
        assert_eq!(
            plain("Use `T[]` and **[docs](cli.md)**"),
            "Use T[] and docs"
        );
    }
}
