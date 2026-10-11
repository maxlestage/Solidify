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
        // Decorations driven by the WebAssembly client (client.rs): reading progress and
        // the light under the pointer.
        div().class("progress").attr("aria-hidden", "true"),
        div().class("glow").attr("aria-hidden", "true"),
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
         <meta name=\"theme-color\" content=\"{}\"/>\
         <link rel=\"manifest\" href=\"{}\"/>\
         <link rel=\"apple-touch-icon\" href=\"{}\"/>\
         <meta name=\"apple-mobile-web-app-capable\" content=\"yes\"/>\
         <meta name=\"apple-mobile-web-app-title\" content=\"Solidify\"/>\
         <link rel=\"preload\" href=\"{}\" as=\"font\" type=\"font/woff2\" crossorigin=\"\"/>\
         <link rel=\"preload\" href=\"{}\" as=\"font\" type=\"font/woff2\" crossorigin=\"\"/>",
        link("/static/favicon.svg"),
        crate::pwa::THEME,
        link("/manifest.webmanifest"),
        link("/static/apple-touch-icon.png"),
        link("/static/fonts/geist-latin-wght-normal.woff2"),
        link("/static/fonts/geist-mono-latin-wght-normal.woff2"),
    )
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

/// The sections, in order: anchor and menu label.
fn sections(lang: Lang) -> [(&'static str, &'static str); 4] {
    let d = t(lang);
    [
        ("playground", d.nav.playground),
        ("how", d.nav.how),
        ("supported", d.nav.supported),
        ("install", d.nav.install),
    ]
}

/// The bar at the top: the logo, the sections (a menu on small screens), the install
/// button, the language picker.
fn header(lang: Lang) -> Element {
    let d = t(lang);
    let links = sections(lang).map(|(id, label)| {
        a().href(format!("#{id}"))
            .attr("data-spy", id)
            .child(span().text(label))
    });
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
                .id("menu")
                .class("menu")
                .attr("aria-label", "Menu")
                .children(links)
                .child(
                    a().href(GITHUB)
                        .attr("rel", "external")
                        .child(span().text(d.nav.github)),
                ),
        )
        .child(
            div()
                .class("tools")
                .child(
                    button()
                        .id("install-app")
                        .class("install-app")
                        .attr("type", "button")
                        .attr("hidden", "")
                        .text(d.nav.install_app),
                )
                .child(lang_picker(lang))
                .child(
                    button()
                        .class("menu-toggle")
                        .attr("type", "button")
                        .attr("aria-controls", "menu")
                        .attr("aria-expanded", "false")
                        .attr("data-open", d.nav.menu)
                        .attr("data-close", d.nav.close)
                        .attr("aria-label", d.nav.menu)
                        .child(span().class("bars").attr("aria-hidden", "true")),
                ),
        )
}

/// The language picker: works without WebAssembly (`<details>`); client.rs remembers the
/// choice, and "Automatic" hands it back to the browser.
fn lang_picker(lang: Lang) -> Element {
    let d = t(lang);
    let mut list = ul().class("lang-list").attr("role", "list");
    for other in Lang::ALL {
        let mut item = a()
            .href(link(other.path()))
            .attr("hreflang", other.code())
            .attr("lang", other.code())
            .attr("data-lang", other.code())
            .child(span().class("code").text(other.code().to_uppercase()))
            .child(span().text(t(other).lang_name));
        if other == lang {
            item = item.attr("aria-current", "true");
        }
        list = list.child(li().child(item));
    }
    list = list.child(
        li().class("auto").child(
            a().href(link(Lang::En.path()))
                .attr("data-lang", "auto")
                .child(span().class("code").text("A"))
                .child(span().text(d.nav.automatic)),
        ),
    );
    el("details")
        .class("lang-picker")
        .child(
            el("summary")
                .attr(
                    "aria-label",
                    format!("{} : {}", d.nav.language, d.lang_name),
                )
                .child(globe())
                .child(span().text(lang.code().to_uppercase()))
                .child(span().class("chev").attr("aria-hidden", "true")),
        )
        .child(list)
}

fn globe() -> Element {
    svg()
        .attr("viewBox", "0 0 24 24")
        .attr("width", "16")
        .attr("height", "16")
        .attr("aria-hidden", "true")
        .attr("fill", "none")
        .attr("stroke", "currentColor")
        .attr("stroke-width", "1.6")
        .child(el("circle").attr("cx", "12").attr("cy", "12").attr("r", "9"))
        .child(el("path").attr("d", "M3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9M12 3c-2.5 2.6-3.8 5.6-3.8 9s1.3 6.4 3.8 9"))
}

fn main_content(lang: Lang) -> Element {
    let d = t(lang);
    main()
        .id("main")
        .child(hero(lang))
        .child(marquee())
        .child(
            section()
                .id("playground")
                .class("block")
                .child(kicker(1, d.nav.playground))
                .child(split(h2(), d.play.title))
                .child(p().class("lead").attr("data-reveal", "").text(d.play.lead))
                .child(
                    div()
                        .class("play-wrap")
                        .attr("data-reveal", "scale")
                        .child(island("playground", lang.code(), playground)),
                ),
        )
        .child(how(lang))
        .child(supported(lang))
        .child(install(lang))
}

fn hero(lang: Lang) -> Element {
    let d = t(lang);
    section()
        .class("hero")
        .child(aurora())
        .child(
            p().class("eyebrow intro")
                .attr("style", "--d:0")
                .child(span().class("dot").attr("aria-hidden", "true"))
                .text(d.hero.eyebrow),
        )
        .child(emphasized(h1(), d.hero.title))
        .child(
            p().class("lead intro")
                .attr("style", "--d:6")
                .text(d.hero.lead),
        )
        .child(
            div()
                .class("cta intro")
                .attr("style", "--d:8")
                .child(
                    a().class("button primary magnetic")
                        .href("#playground")
                        .child(span().text(d.hero.cta))
                        .child(arrow()),
                )
                .child(
                    a().class("button magnetic")
                        .href("#install")
                        .child(span().text(d.hero.cta2))
                        .child(arrow()),
                ),
        )
        .child(
            ul().class("facts intro")
                .attr("style", "--d:10")
                .children(d.hero.facts.iter().map(|(value, label)| {
                    // A whole number counts up as the page opens (CSS only, see
                    // `.facts strong[data-count]`); the figure stays in the text.
                    let figure = if value.parse::<u32>().is_ok() {
                        strong()
                            .attr("data-count", *value)
                            .attr("style", format!("--to:{value}"))
                            .child(span().class("value").text(*value))
                    } else {
                        strong().text(*value)
                    };
                    li().child(figure).child(span().text(*label))
                })),
        )
        .child(
            a().class("scroll-cue")
                .href("#playground")
                .attr("aria-label", d.nav.playground)
                .child(span().attr("aria-hidden", "true")),
        )
}

/// A title where `*words*` are set in the accent serif. Each word sits in a mask it
/// rises out of when the page opens (`--i` orders them).
fn emphasized(root: Element, text: &str) -> Element {
    let mut root = root.class("split");
    let mut n = 0;
    for (i, part) in text.split('*').enumerate() {
        for (j, word) in part.split(' ').enumerate() {
            if j > 0 {
                root = root.text(" ");
            }
            if word.is_empty() {
                continue;
            }
            let inner = if i % 2 == 1 {
                // A trailing period stays upright.
                match word.strip_suffix('.') {
                    Some(w) => span()
                        .class("w-in")
                        .child(em().text(w.to_string()))
                        .text("."),
                    None => span().class("w-in").child(em().text(word.to_string())),
                }
            } else {
                span().class("w-in").text(word.to_string())
            };
            root = root.child(
                span()
                    .class("w")
                    .attr("style", format!("--i:{n}"))
                    .child(inner),
            );
            n += 1;
        }
    }
    root
}

/// A heading whose words rise in turn when it scrolls into view.
fn split(root: Element, text: &str) -> Element {
    emphasized(root.attr("data-reveal", "words"), text)
}

/// `01 — Playground` above a section's title.
fn kicker(n: usize, label: &str) -> Element {
    p().class("kicker")
        .attr("data-reveal", "")
        .child(span().class("kicker-n").text(format!("{n:02}")))
        .child(span().class("kicker-line").attr("aria-hidden", "true"))
        .text(label.to_string())
}

/// The light behind the hero: blurred shapes drifting slowly, over a fading grid.
fn aurora() -> Element {
    div()
        .class("aurora")
        .attr("aria-hidden", "true")
        .child(span().class("blob b1"))
        .child(span().class("blob b2"))
        .child(span().class("blob b3"))
        .child(span().class("grid"))
}

fn arrow() -> Element {
    svg()
        .class("arrow")
        .attr("viewBox", "0 0 16 16")
        .attr("width", "14")
        .attr("height", "14")
        .attr("aria-hidden", "true")
        .attr("fill", "none")
        .attr("stroke", "currentColor")
        .attr("stroke-width", "1.8")
        .child(el("path").attr("d", "M3 8h10M9 4l4 4-4 4"))
}

/// What becomes what, on two bands sliding in opposite directions. The same in every
/// language: code reads the same everywhere.
const PAIRS: [(&str, &str); 16] = [
    ("T | null", "Option<T>"),
    ("throw", "Result<T, E>"),
    ("interface", "struct"),
    ("A | B", "enum"),
    ("async / await", "Promise<T>"),
    ("xs.map(f)", "iter().map(f)"),
    ("Map<K, V>", "HashMap<K, V>"),
    ("extends", "trait"),
    ("bigint", "JsBigInt"),
    ("switch", "match"),
    ("a?.b", "map(|a| a.b)"),
    ("function*", "JsGenerator<T>"),
    ("for … of", "for x in &v"),
    ("number", "i64 · f64"),
    ("JSON.parse", "JsFromJson"),
    ("import", "mod"),
];

fn marquee() -> Element {
    let band = |pairs: &[(&'static str, &'static str)], reverse: bool| {
        let chips = |hidden: bool| {
            let mut group = div().class("marquee-group");
            if hidden {
                group = group.attr("aria-hidden", "true");
            }
            group.children(pairs.iter().map(|(ts, rs)| {
                span()
                    .class("chip")
                    .child(code().class("ts").text(*ts))
                    .child(span().class("to").attr("aria-hidden", "true").text("→"))
                    .child(code().class("rs").text(*rs))
            }))
        };
        div()
            .class(if reverse {
                "marquee-row reverse"
            } else {
                "marquee-row"
            })
            .child(chips(false))
            .child(chips(true))
    };
    div()
        .class("marquee")
        .attr("data-reveal", "")
        .child(band(&PAIRS[..8], false))
        .child(band(&PAIRS[8..], true))
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
        .child(kicker(2, d.nav.how))
        .child(split(h2(), d.how.title))
        .child(p().class("lead").attr("data-reveal", "").text(d.how.lead))
        .child(
            ol().class("steps")
                .children(d.how.steps.iter().enumerate().map(|(i, (title, text))| {
                    li().class("spot")
                        .attr("data-reveal", "")
                        .attr("style", format!("--i:{i}"))
                        .child(span().class("num").text((i + 1).to_string()))
                        .child(h3().text(*title))
                        .child(p().children(rich(text)))
                })),
        )
        .child(
            h3().class("sub")
                .attr("data-reveal", "")
                .text(d.how.rules_title),
        )
        .child(
            div()
                .class("rules")
                .children(d.how.rules.iter().enumerate().map(|(i, (title, text))| {
                    div()
                        .class("rule spot")
                        .attr("data-reveal", "")
                        .attr("style", format!("--i:{i}"))
                        .child(h4().text(*title))
                        .child(p().children(rich(text)))
                })),
        )
}

fn supported(lang: Lang) -> Element {
    let d = t(lang);
    section()
        .id("supported")
        .class("block")
        .child(kicker(3, d.nav.supported))
        .child(split(h2(), d.supported.title))
        .child(
            p().class("lead")
                .attr("data-reveal", "")
                .text(d.supported.lead),
        )
        .child(
            table()
                .class("mapping")
                .attr("data-reveal", "rows")
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
        .child(
            div()
                .class("limits spot")
                .attr("data-reveal", "")
                .child(h3().text(d.supported.limits_title))
                .child(p().children(rich(d.supported.limits))),
        )
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
            .attr("data-reveal", "")
            .child(h3().text(title))
            .child(pre().class("spot").child(code().html(html)))
    };
    section()
        .id("install")
        .class("block")
        .child(kicker(4, d.nav.install))
        .child(split(h2(), d.install.title))
        .child(
            p().class("lead")
                .attr("data-reveal", "")
                .text(d.install.lead),
        )
        .child(block(d.install.cli, INSTALL_CLI, None))
        .child(block(d.install.check, INSTALL_CHECK, None))
        .child(
            div()
                .class("snippet")
                .attr("data-reveal", "")
                .child(h3().text(d.install.library))
                .child(
                    pre()
                        .class("spot")
                        .child(code().html(shell_html(CARGO_LIB))),
                )
                .child(
                    pre()
                        .class("spot")
                        .child(code().html(highlight(INSTALL_LIB, Language::Rust))),
                ),
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

pub const NPM: &str = "https://www.npmjs.com/package/solidify-rs";
const ISSUES: &str = "https://github.com/maxlestage/Solidify/issues";
const DOCS: [&str; 3] = [
    "https://github.com/maxlestage/Solidify#readme",
    "https://github.com/maxlestage/Solidify/blob/master/README.fr.md",
    "https://github.com/maxlestage/Solidify/blob/master/README.es.md",
];

/// The footer: the brand and the command to run it, then the product, resources,
/// languages and family columns, a large wordmark, and the legal line.
fn footer(lang: Lang) -> Element {
    let d = t(lang);
    let f = &d.footer;
    let column = |title: &'static str, links: Vec<Element>, i: usize| {
        div()
            .class("foot-col")
            .attr("data-reveal", "")
            .attr("style", format!("--i:{i}"))
            .child(h2().class("foot-title").text(title))
            .child(
                ul().attr("role", "list")
                    .children(links.into_iter().map(|l| li().child(l))),
            )
    };
    let external = |href: &'static str, label: &'static str| {
        a().href(href)
            .attr("rel", "external")
            .child(span().text(label))
    };
    let docs = DOCS[match lang {
        Lang::En => 0,
        Lang::Fr => 1,
        Lang::Es => 2,
    }];
    let product: Vec<Element> = sections(lang)
        .into_iter()
        .map(|(id, label)| a().href(format!("#{id}")).child(span().text(label)))
        .collect();
    let resources = vec![
        external(docs, f.docs),
        external(GITHUB, f.source),
        external(NPM, f.npm),
        external(ISSUES, f.issues),
        external(LICENSE, f.license_link),
    ];
    let languages: Vec<Element> = Lang::ALL
        .iter()
        .map(|other| {
            let mut l = a()
                .href(link(other.path()))
                .attr("hreflang", other.code())
                .attr("lang", other.code())
                .attr("data-lang", other.code())
                .child(span().text(t(*other).lang_name));
            if *other == lang {
                l = l.attr("aria-current", "true");
            }
            l
        })
        .chain(std::iter::once(
            a().href(link(Lang::En.path()))
                .attr("data-lang", "auto")
                .child(span().text(d.nav.automatic)),
        ))
        .collect();
    let family: Vec<Element> = FAMILY
        .iter()
        .map(|(name, url)| external(url, name))
        .collect();
    el("footer")
        .class("bottom")
        .child(
            div()
                .class("foot-grid")
                .child(
                    div()
                        .class("foot-brand")
                        .attr("data-reveal", "")
                        .child(
                            a().class("brand")
                                .href(link(lang.path()))
                                .child(logo())
                                .child(span().text("Solidify")),
                        )
                        .child(p().class("foot-tagline").text(f.tagline))
                        .child(
                            div()
                                .class("npx")
                                .child(code().text("npx solidify-rs main.ts"))
                                .child(
                                    button()
                                        .class("npx-copy")
                                        .attr("type", "button")
                                        .attr("data-copy", "npx solidify-rs main.ts")
                                        .attr("data-done", f.copied)
                                        .text(f.copy),
                                ),
                        )
                        .child(
                            p().class("status")
                                .child(span().class("pulse").attr("aria-hidden", "true"))
                                .text(f.status),
                        ),
                )
                .child(column(f.product, product, 1))
                .child(column(f.resources, resources, 2))
                .child(column(d.nav.language, languages, 3))
                .child(column(f.projects, family, 4)),
        )
        .child(
            div()
                .class("wordmark-wrap")
                .attr("aria-hidden", "true")
                .child(p().class("wordmark").text("Solidify")),
        )
        .child(
            div()
                .class("foot-legal")
                .child(
                    p().text(format!("© 2026 {AUTHOR}."))
                        .text(" ")
                        .child(span().class("fine").text(f.license)),
                )
                .child(
                    p().class("foot-meta")
                        .child(span().text(f.made))
                        .text(" ")
                        .child(a().href(AUTHOR_URL).attr("rel", "external").text(AUTHOR))
                        .text(" · ")
                        .child(span().text(format!("v{}", env!("CARGO_PKG_VERSION"))))
                        .text(" · ")
                        .child(a().class("to-top").href("#main").text(f.top)),
                ),
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
