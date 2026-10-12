//! The documentation: the Markdown pages `docs/{lang}/{slug}.md` at the root of the
//! repository, built into the server (nothing to copy next to it) and rendered once.
//!
//! The tests hold the pages to what Solidify does: every TypeScript example must
//! translate, a TypeScript example followed by a Rust block must translate to exactly that
//! Rust (unless the Rust block is marked `fragment`), and an example marked `error` must
//! fail, with the message of the `text` block that follows it.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::catalog::{self, PAGES};
use crate::i18n::t;
use crate::markdown::{Labels, Rendered, render};
use crate::routes::Lang;

macro_rules! sources {
    ($($slug:literal),* $(,)?) => {
        [$((
            $slug,
            [
                include_str!(concat!("../../docs/en/", $slug, ".md")),
                include_str!(concat!("../../docs/fr/", $slug, ".md")),
                include_str!(concat!("../../docs/es/", $slug, ".md")),
            ],
        )),*]
    };
}

/// Every page, in the order of the catalog, in English, French and Spanish.
pub const SOURCES: [(&str, [&str; 3]); PAGES.len()] = sources![
    "introduction",
    "installation",
    "quick-start",
    "how-it-works",
    "types",
    "ownership",
    "classes",
    "errors",
    "async",
    "modules",
    "runtime",
    "cli",
    "library",
    "mapping",
    "limits",
    "faq",
];

/// Where a page's source is on GitHub, for its "Edit this page" link.
pub fn source_url(lang: Lang, slug: &str) -> String {
    format!(
        "https://github.com/maxlestage/Solidify/blob/master/docs/{}/{slug}.md",
        lang.code()
    )
}

static RENDERED: LazyLock<HashMap<(Lang, &'static str), Rendered>> = LazyLock::new(|| {
    let mut pages = HashMap::new();
    for (slug, sources) in SOURCES {
        for lang in Lang::ALL {
            let d = &t(lang).docs;
            let labels = Labels {
                copy: t(lang).play.copy,
                copied: t(lang).play.copied,
                note: d.note,
                tip: d.tip,
                warning: d.warning,
                becomes: d.becomes,
            };
            pages.insert((lang, slug), render(lang.pick(sources), lang, &labels));
        }
    }
    pages
});

/// The page `slug` (one of the catalog's) in `lang`, rendered.
pub fn page(lang: Lang, slug: &str) -> &'static Rendered {
    let page = catalog::find(slug).expect("a page of the catalog");
    &RENDERED[&(lang, page.slug)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::islands::translate;
    use crate::markdown::fences;

    #[test]
    fn every_page_is_written_in_every_language() {
        for ((slug, sources), page) in SOURCES.iter().zip(&PAGES) {
            assert_eq!(*slug, page.slug, "SOURCES follows the catalog");
            for lang in Lang::ALL {
                let source = lang.pick(*sources);
                assert!(
                    source.starts_with("# "),
                    "docs/{}/{slug}.md starts with its title",
                    lang.code()
                );
                let rendered = super::page(lang, slug);
                assert!(!rendered.description.is_empty());
                assert!(
                    !rendered.toc.is_empty(),
                    "docs/{}/{slug}.md has sections",
                    lang.code()
                );
            }
        }
    }

    /// Each piece of a `fragment` (pieces are separated by `// …` lines) is in `rust`, in
    /// that order.
    fn pieces_in_order(fragment: &str, rust: &str) -> Result<(), String> {
        let mut from = 0;
        for piece in fragment
            .split("// …")
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            match rust[from..].find(piece) {
                Some(at) => from += at + piece.len(),
                None => return Err(piece.to_string()),
            }
        }
        Ok(())
    }

    /// The examples are what Solidify does, in every language.
    #[test]
    fn examples_are_what_solidify_writes() {
        let is_ts = |b: &crate::markdown::Fence| {
            matches!(b.lang.as_str(), "ts" | "typescript" | "js" | "javascript")
        };
        let marked = |b: &crate::markdown::Fence, note: &str| b.notes.iter().any(|w| w == note);
        for (slug, sources) in SOURCES {
            for lang in Lang::ALL {
                let file = format!("docs/{}/{slug}.md", lang.code());
                let blocks = fences(lang.pick(sources));
                for (n, (block, _)) in blocks.iter().enumerate() {
                    if !is_ts(block) || marked(block, "fragment") {
                        continue;
                    }
                    let next = blocks
                        .get(n + 1)
                        .filter(|(_, heading)| !heading)
                        .map(|(b, _)| b);
                    let result = translate(&block.code);
                    if marked(block, "error") {
                        let message = result.expect_err(&format!("{file}: example {n} must fail"));
                        // The message, its first line (the location names the file).
                        if let Some(text) = next.filter(|b| b.lang == "text") {
                            assert_eq!(
                                message.lines().next(),
                                text.code.lines().next(),
                                "{file}: example {n}"
                            );
                        }
                        continue;
                    }
                    let rust = result.unwrap_or_else(|e| panic!("{file}: example {n}:\n{e}"));
                    let Some(expected) = next.filter(|b| b.lang == "rust") else {
                        continue;
                    };
                    if marked(expected, "fragment") {
                        if let Err(piece) = pieces_in_order(&expected.code, &rust) {
                            panic!(
                                "{file}: example {n}: not in the translation, or not in order:\n{piece}\n\n{rust}"
                            );
                        }
                    } else {
                        assert_eq!(
                            rust.trim(),
                            expected.code.trim(),
                            "{file}: example {n} translates differently"
                        );
                    }
                }
            }
        }
    }

    /// Every link from a page to a section (`errors.md#…`, `#…`) leads to a heading of that
    /// page in the same language: anchors change when headings are translated.
    #[test]
    fn links_to_sections_lead_to_headings() {
        for (slug, _) in SOURCES {
            for lang in Lang::ALL {
                let html = &page(lang, slug).html;
                let docs = crate::routes::Route::new(lang, crate::routes::Page::Docs).href();
                for link in html.split("href=\"").skip(1) {
                    let href = &link[..link.find('"').unwrap_or(0)];
                    let (target, anchor) = match href.split_once('#') {
                        Some(("", anchor)) => (slug, anchor),
                        Some((path, anchor)) => match path.strip_prefix(docs.as_str()) {
                            Some(target) => (target.trim_end_matches('/'), anchor),
                            None => continue,
                        },
                        None => continue,
                    };
                    let target_html = &page(lang, target).html;
                    assert!(
                        target_html.contains(&format!("id=\"{anchor}\"")),
                        "docs/{}/{slug}.md: {href} leads to no heading",
                        lang.code()
                    );
                }
            }
        }
    }

    #[test]
    fn fragments_are_checked_piece_by_piece() {
        let rust = "fn a() {}\n\nfn b() {}\n\nfn main() {}\n";
        assert!(pieces_in_order("fn a() {}\n// …\nfn main() {}", rust).is_ok());
        assert!(pieces_in_order("fn main() {}\n// …\nfn a() {}", rust).is_err());
        assert!(pieces_in_order("// …\nfn b() {}\n// …", rust).is_ok());
    }
}
