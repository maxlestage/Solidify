//! The home page, in each language, written with active.

use active::prelude::*;

use crate::highlight::{Language, highlight};
use crate::i18n::t;
use crate::islands::playground;
use crate::routes::{Lang, base, link};

pub const GITHUB: &str = "https://github.com/maxlestage/Solidify";
pub const LICENSE: &str = "https://github.com/maxlestage/Solidify/blob/master/LICENSE";
pub const AUTHOR: &str = "Maxime Nathan Lestage";
pub const AUTHOR_URL: &str = "https://github.com/maxlestage";
pub const FAMILY: [(&str, &str); 6] = [
    ("Active", "https://maxlestage.github.io/Active/"),
    ("Vitesse", "https://maxlestage.github.io/Vitesse/"),
    ("Securite", "https://maxlestage.github.io/Securite/"),
    ("Memoire", "https://maxlestage.github.io/Memoire/"),
    ("Paquets", "https://maxlestage.github.io/Paquets/"),
    ("FerroSwift", "https://maxlestage.github.io/FerroSwift/"),
];

const INSTALL_CLI: &str = "npx solidify-rs main.ts -o app.rs     # follows the imports of main.ts, one Rust file out\n\
npx solidify-rs < app.ts > app.rs     # or through the standard input\n\
rustc --edition 2024 app.rs && ./app";

const INSTALL_CHECK: &str =
    "npx solidify-rs --check app.ts        # errors with line and column, nothing written";

const INSTALL_LIB: &str = "let options = solidify::Options::default();\n\
let rust: String = solidify::transpile(typescript, &options)?;\n\
// A program of several files, from its entry file:\n\
let rust: String = solidify::transpile_file(Path::new(\"main.ts\"), &options)?;";

const CARGO_LIB: &str =
    "[dependencies]\nsolidify = { git = \"https://github.com/maxlestage/Solidify\" }";

/// The whole page for `lang`. `origin` is the public address (with the base path) for
/// the canonical and alternate URLs.
pub fn home(lang: Lang, origin: &str) -> String {
    let d = t(lang);
    let mut seo = Seo::new()
        .site_name("Solidify")
        .title(d.title)
        .description(d.description)
        .locale(d.locale)
        .base_url(origin)
        .canonical(lang.path())
        .json_ld(json_ld(lang));
    for other in Lang::ALL {
        seo = seo.alternate(other.code(), other.path());
    }
    seo = seo.alternate("x-default", Lang::En.path());
    let body = fragment([
        skip_link(lang),
        header(lang),
        main_content(lang),
        footer(lang),
    ]);
    Document::new(body)
        .lang(lang.code())
        .seo(seo)
        .stylesheet(link("/static/app.css"))
        .head(head_extra())
        .html_attr("data-base", base())
        .client(
            link("/pkg/solidify_site.js"),
            link("/pkg/solidify_site_bg.wasm"),
        )
        .render()
}

fn head_extra() -> String {
    format!(
        "<link rel=\"icon\" href=\"{}\" type=\"image/svg+xml\"/>\
         <meta name=\"theme-color\" content=\"#0b0b0c\"/>\
         <link rel=\"manifest\" href=\"{}\"/>\
         <link rel=\"apple-touch-icon\" href=\"{}\"/>\
         <meta name=\"apple-mobile-web-app-capable\" content=\"yes\"/>\
         <meta name=\"apple-mobile-web-app-title\" content=\"Solidify\"/>\
         <link rel=\"preload\" href=\"{}\" as=\"font\" type=\"font/woff2\" crossorigin=\"\"/>\
         <link rel=\"preload\" href=\"{}\" as=\"font\" type=\"font/woff2\" crossorigin=\"\"/>",
        link("/static/favicon.svg"),
        link("/manifest.webmanifest"),
        link("/static/apple-touch-icon.png"),
        link("/static/fonts/geist-latin-wght-normal.woff2"),
        link("/static/fonts/geist-mono-latin-wght-normal.woff2"),
    ) + &crate::pwa::register_script(&link("/sw.js"))
}

fn json_ld(lang: Lang) -> String {
    let d = t(lang);
    format!(
        r#"{{"@context":"https://schema.org","@type":"SoftwareApplication","name":"Solidify","applicationCategory":"DeveloperApplication","operatingSystem":"Linux, macOS, Windows","description":"{}","author":{{"@type":"Person","name":"{AUTHOR}","url":"{AUTHOR_URL}"}},"codeRepository":"{GITHUB}"}}"#,
        d.description.replace('"', "\\\"")
    )
}

fn skip_link(lang: Lang) -> Element {
    a().class("skip").href("#main").text(t(lang).nav.skip)
}

/// The logo: a dashed square turning into a solid one.
fn logo() -> Element {
    svg()
        .class("logo")
        .attr("viewBox", "0 0 32 32")
        .attr("aria-hidden", "true")
        .attr("width", "28")
        .attr("height", "28")
        .child(
            rect()
                .attr("x", "3")
                .attr("y", "9")
                .attr("width", "14")
                .attr("height", "14")
                .attr("rx", "3")
                .attr("fill", "none")
                .attr("stroke", "currentColor")
                .attr("stroke-width", "2")
                .attr("stroke-dasharray", "3 3"),
        )
        .child(
            rect()
                .attr("x", "15")
                .attr("y", "9")
                .attr("width", "14")
                .attr("height", "14")
                .attr("rx", "3")
                .attr("fill", "#ffd23f"),
        )
}

fn header(lang: Lang) -> Element {
    let d = t(lang);
    let nav_link = |href: &'static str, label: &'static str| a().href(href).text(label);
    el("header")
        .class("top")
        .child(
            a().class("brand")
                .href(link(lang.path()))
                .child(logo())
                .child(span().text("Solidify")),
        )
        .child(
            nav()
                .attr("aria-label", "Menu")
                .child(nav_link("#playground", d.nav.playground))
                .child(nav_link("#how", d.nav.how))
                .child(nav_link("#supported", d.nav.supported))
                .child(nav_link("#install", d.nav.install))
                .child(a().href(GITHUB).attr("rel", "external").text(d.nav.github))
                .child(
                    button()
                        .id("install-app")
                        .class("install-app")
                        .attr("type", "button")
                        .attr("hidden", "")
                        .text(d.nav.install_app),
                )
                .child(lang_switch(lang)),
        )
}

/// The other languages, by their code (`FR`, `ES`), named in full for screen readers.
fn lang_switch(lang: Lang) -> Element {
    let mut list = span().class("langs");
    for other in Lang::ALL {
        if other == lang {
            continue;
        }
        list = list.child(
            a().class("lang")
                .href(link(other.path()))
                .attr("hreflang", other.code())
                .attr("lang", other.code())
                .attr("title", t(other).lang_name)
                .attr("aria-label", t(other).lang_name)
                .text(other.code().to_uppercase()),
        );
    }
    list
}

fn main_content(lang: Lang) -> Element {
    let d = t(lang);
    main()
        .id("main")
        .child(hero(lang))
        .child(
            section()
                .id("playground")
                .class("block")
                .child(h2().text(d.play.title))
                .child(p().class("lead").text(d.play.lead))
                .child(island("playground", lang.code(), playground)),
        )
        .child(how(lang))
        .child(supported(lang))
        .child(install(lang))
}

fn hero(lang: Lang) -> Element {
    let d = t(lang);
    section()
        .class("hero")
        .child(p().class("eyebrow").text(d.hero.eyebrow))
        .child(emphasized(h1(), d.hero.title))
        .child(p().class("lead").text(d.hero.lead))
        .child(
            div()
                .class("cta")
                .child(
                    a().class("button primary")
                        .href("#playground")
                        .text(d.hero.cta),
                )
                .child(a().class("button").href("#install").text(d.hero.cta2)),
        )
        .child(
            ul().class("facts")
                .children(d.hero.facts.iter().map(|(value, label)| {
                    li().child(strong().text(*value)).child(span().text(*label))
                })),
        )
}

/// A title where `*words*` are set in the accent serif.
fn emphasized(root: Element, text: &str) -> Element {
    let mut root = root;
    for (i, part) in text.split('*').enumerate() {
        if part.is_empty() {
            continue;
        }
        root = if i % 2 == 1 {
            root.child(em().text(part.to_string()))
        } else {
            root.text(part.to_string())
        };
    }
    root
}

/// Text where `` `code` `` becomes `<code>`.
fn rich(text: &str) -> Vec<Node> {
    text.split('`')
        .enumerate()
        .filter(|(_, part)| !part.is_empty())
        .map(|(i, part)| {
            if i % 2 == 1 {
                code().text(part.to_string()).into()
            } else {
                Node::from(part.to_string())
            }
        })
        .collect()
}

fn how(lang: Lang) -> Element {
    let d = t(lang);
    section()
        .id("how")
        .class("block")
        .child(h2().text(d.how.title))
        .child(p().class("lead").text(d.how.lead))
        .child(
            ol().class("steps")
                .children(d.how.steps.iter().enumerate().map(|(i, (title, text))| {
                    li().child(span().class("num").text((i + 1).to_string()))
                        .child(h3().text(*title))
                        .child(p().children(rich(text)))
                })),
        )
        .child(h3().class("sub").text(d.how.rules_title))
        .child(
            dl().class("rules")
                .children(d.how.rules.iter().flat_map(|(title, text)| {
                    [
                        Node::from(dt().text(*title)),
                        Node::from(dd().children(rich(text))),
                    ]
                })),
        )
}

fn supported(lang: Lang) -> Element {
    let d = t(lang);
    section()
        .id("supported")
        .class("block")
        .child(h2().text(d.supported.title))
        .child(p().class("lead").text(d.supported.lead))
        .child(
            table()
                .class("mapping")
                .child(
                    thead().child(
                        tr().child(th().text(d.supported.head.0))
                            .child(th().text(d.supported.head.1)),
                    ),
                )
                .child(tbody().children(d.supported.rows.iter().map(|(ts, rs)| {
                    tr().child(td().child(code().text(*ts)))
                        .child(td().text(*rs))
                }))),
        )
        .child(h3().class("sub").text(d.supported.limits_title))
        .child(p().children(rich(d.supported.limits)))
}

fn install(lang: Lang) -> Element {
    let d = t(lang);
    let block = |title: &'static str, code_text: &str, language: Option<Language>| {
        let html = match language {
            Some(l) => highlight(code_text, l),
            None => shell_html(code_text),
        };
        div()
            .class("snippet")
            .child(h3().text(title))
            .child(pre().child(code().html(html)))
    };
    section()
        .id("install")
        .class("block")
        .child(h2().text(d.install.title))
        .child(p().class("lead").text(d.install.lead))
        .child(block(d.install.cli, INSTALL_CLI, None))
        .child(block(d.install.check, INSTALL_CHECK, None))
        .child(
            div()
                .class("snippet")
                .child(h3().text(d.install.library))
                .child(pre().child(code().html(shell_html(CARGO_LIB))))
                .child(pre().child(code().html(highlight(INSTALL_LIB, Language::Rust)))),
        )
}

/// Shell lines: comments after `#` are dimmed.
fn shell_html(text: &str) -> String {
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match line.split_once('#') {
            Some((cmd, comment)) if !line.trim_start().starts_with('#') => {
                crate::highlight::escape_into(&mut out, cmd);
                out.push_str("<span class=\"c\">#");
                crate::highlight::escape_into(&mut out, comment);
                out.push_str("</span>");
            }
            _ => crate::highlight::escape_into(&mut out, line),
        }
    }
    out
}

fn footer(lang: Lang) -> Element {
    let d = t(lang);
    el("footer")
        .class("bottom")
        .child(
            p().child(span().text(d.footer.made))
                .text(" ")
                .child(a().href(AUTHOR_URL).attr("rel", "external").text(AUTHOR))
                .text(", ")
                .text(d.footer.family)
                .text(" ")
                .children(FAMILY.iter().enumerate().flat_map(|(i, (name, url))| {
                    let sep: Node = if i + 1 == FAMILY.len() {
                        ".".into()
                    } else {
                        ", ".into()
                    };
                    [
                        a().href(*url).attr("rel", "external").text(*name).into(),
                        sep,
                    ]
                })),
        )
        .child(
            p().class("fine")
                .child(
                    a().href(LICENSE)
                        .attr("rel", "external")
                        .text(d.footer.license),
                )
                .text(" · ")
                .child(
                    a().href(GITHUB)
                        .attr("rel", "external")
                        .text(d.footer.source),
                )
                .text(" · ")
                .text(format!("Solidify {}", env!("CARGO_PKG_VERSION"))),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_render_with_their_seo() {
        for lang in Lang::ALL {
            let html = home(lang, "https://example.com");
            assert!(html.starts_with("<!DOCTYPE html>"));
            assert!(html.contains("<title>"));
            assert!(html.contains("hreflang=\"fr\""));
            assert!(html.contains("data-island=\"playground\""));
            assert!(html.contains("fib</span>(n: <span class=\"t\">i64</span>)"));
        }
        let fr = home(Lang::Fr, "https://example.com");
        assert!(fr.contains("<html lang=\"fr\""));
        assert!(fr.contains("rel=\"canonical\" href=\"https://example.com/fr/\""));
    }
}
