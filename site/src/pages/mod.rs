//! Every page of the site, written with active: the home page (`home.rs`), the
//! documentation (`docs.rs`), and what they share here: the head, the header and its
//! menu, the search, the footer.

mod docs;
mod home;

use active::prelude::*;

use crate::catalog::{CATEGORIES, PAGES, pages_of};
use crate::i18n::t;
use crate::routes::{Lang, Page, Route, base, doc_href, link};
use crate::search::norm;

pub const GITHUB: &str = "https://github.com/maxlestage/Solidify";
pub const LICENSE: &str = "https://github.com/maxlestage/Solidify/blob/master/LICENSE";
pub const NPM: &str = "https://www.npmjs.com/package/solidify-rs";
const ISSUES: &str = "https://github.com/maxlestage/Solidify/issues";
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

/// The page at `route`, as HTML. `origin` is the public address (with the base path) for
/// the canonical and alternate URLs.
pub fn render(route: Route, origin: &str) -> String {
    match route.page {
        Page::Home => {
            let d = t(route.lang);
            let head = Head {
                title: d.title.to_string(),
                description: d.description.to_string(),
                json_ld: app_json_ld(route.lang),
            };
            document(route, origin, head, home::content(route.lang))
        }
        Page::Docs => docs::hub(route, origin),
        Page::Doc(slug) => docs::page(route, origin, slug),
    }
}

/// The home page in `lang`.
pub fn home(lang: Lang, origin: &str) -> String {
    render(Route::new(lang, Page::Home), origin)
}

/// What a page puts in its head besides the common tags.
struct Head {
    title: String,
    description: String,
    json_ld: String,
}

/// A whole page around its `<main>`: the head and its SEO tags (alternates lead to the
/// same page in each language), the header and its menu, the search, the footer, and the
/// decorations the client drives. Only the home page has the preloader.
fn document(route: Route, origin: &str, head: Head, main: Element) -> String {
    let lang = route.lang;
    let d = t(lang);
    let mut seo = Seo::new()
        .site_name("Solidify")
        .title(head.title)
        .description(head.description)
        .locale(d.locale)
        .base_url(origin)
        .canonical(route.path())
        .json_ld(head.json_ld);
    for other in Lang::ALL {
        seo = seo.alternate(other.code(), route.with_lang(other).path());
    }
    seo = seo.alternate("x-default", route.with_lang(Lang::En).path());
    let home = route.page == Page::Home;
    let mut body: Vec<Node> = vec![skip_link(lang).into()];
    if home {
        body.push(home::loader(lang).into());
    }
    body.extend([
        // Decorations driven by the WebAssembly client (client/motion.rs): reading
        // progress and its spark, the light under the pointer, the cursor.
        Node::from(div().class("progress").attr("aria-hidden", "true")),
        div()
            .class("progress-head")
            .attr("aria-hidden", "true")
            .into(),
        div().class("glow").attr("aria-hidden", "true").into(),
        cursor().into(),
        header(route).into(),
        main.into(),
        search(lang).into(),
        footer(route).into(),
        to_top(lang).into(),
        div().class("noise").attr("aria-hidden", "true").into(),
    ]);
    let mut document = Document::new(fragment(body))
        .lang(lang.code())
        .seo(seo)
        .stylesheet(link("/static/app.css"))
        .head(head_extra())
        .html_attr("data-base", base());
    if home {
        // The hero waits for the preloader (`--hero-delay`).
        document = document.html_attr("data-intro", "");
    }
    document
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

/// `text` inside a JSON string.
fn json(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn app_json_ld(lang: Lang) -> String {
    let d = t(lang);
    format!(
        r#"{{"@context":"https://schema.org","@type":"SoftwareApplication","name":"Solidify","applicationCategory":"DeveloperApplication","operatingSystem":"Linux, macOS, Windows","description":"{}","author":{{"@type":"Person","name":"{AUTHOR}","url":"{AUTHOR_URL}"}},"codeRepository":"{GITHUB}"}}"#,
        json(d.description)
    )
}

fn skip_link(lang: Lang) -> Element {
    a().class("skip").href("#main").text(t(lang).nav.skip)
}

/// A lightning bolt (24 × 24).
fn bolt() -> Element {
    svg()
        .class("bolt")
        .attr("viewBox", "0 0 24 24")
        .attr("aria-hidden", "true")
        .child(
            el("path")
                .attr("d", "M13.5 2 4.5 13.6h6.3L9.6 22l9.9-12.4h-6.5L13.5 2z")
                .attr("pathLength", "100"),
        )
}

/// A line icon (24 × 24, drawn with the current colour).
fn icon(name: &str) -> Element {
    let paths: &[&str] = match name {
        "bolt" => &["M13 2 4 14h7l-1 8 9-12h-7z"],
        "flow" => &["M4 7h11", "m11 3 4 4-4 4", "M20 17H9", "m13 13-4 4 4 4"],
        "book" => &[
            "M2 4h7a3 3 0 0 1 3 3v13a2 2 0 0 0-2-2H2z",
            "M22 4h-7a3 3 0 0 0-3 3v13a2 2 0 0 1 2-2h8z",
        ],
        "search" => &["M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z", "m20 20-4-4"],
        "menu" => &["M4 7h16", "M4 12h16", "M4 17h10"],
        "close" => &["M6 6l12 12", "M18 6 6 18"],
        "github" => &[
            "M9 19c-4.3 1.4-4.3-2.5-6-3m12 5v-3.5c0-1 .1-1.4-.5-2 2.8-.3 5.5-1.4 5.5-6a4.6 4.6 0 0 0-1.3-3.2 4.2 4.2 0 0 0-.1-3.2s-1.1-.3-3.5 1.3a12.3 12.3 0 0 0-6.2 0C6.5 2.8 5.4 3.1 5.4 3.1a4.2 4.2 0 0 0-.1 3.2A4.6 4.6 0 0 0 4 9.5c0 4.6 2.7 5.7 5.5 6-.6.6-.6 1.2-.5 2V21",
        ],
        "edit" => &["M12 20h9", "M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z"],
        "terminal" => &["m4 17 6-6-6-6", "M12 19h8"],
        "play" => &["M7 4.5v15l12-7.5z"],
        "table" => &["M3 5h18v14H3z", "M3 10h18", "M10 10v9"],
        "crate" => &["M21 8 12 3 3 8v8l9 5 9-5z", "m3 8 9 5 9-5", "M12 13v8"],
        "up" => &["M12 19V5", "m5 12 7-7 7 7"],
        "hash" => &["M4 9h16", "M4 15h16", "M10 3 8 21", "m16 3-2 18"],
        "page" => &["M14 3H6v18h12V7z", "M14 3v4h4"],
        "home" => &["m3 11 9-8 9 8", "M5 10v10h14V10"],
        _ => &[],
    };
    svg()
        .class("icon")
        .attr("viewBox", "0 0 24 24")
        .attr("aria-hidden", "true")
        .attr("fill", "none")
        .attr("stroke", "currentColor")
        .attr("stroke-width", "1.7")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
        .children(paths.iter().map(|d| el("path").attr("d", *d)))
}

/// The cursor on a fine pointer: a spark, and a ring that catches up with it, grows over
/// links and shows their `data-cursor` label (moved by the client).
fn cursor() -> Element {
    div()
        .class("cursor")
        .attr("aria-hidden", "true")
        .child(div().class("cursor-dot"))
        .child(div().class("cursor-ring").child(span()))
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

/// A title where `*words*` are set in the accent serif. Each word sits in a mask it
/// rises out of (`--i` orders them).
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
                // A trailing period stays upright. The word is repeated for the
                // interference that crosses it now and then (`data-text`).
                let (w, dot) = match word.strip_suffix('.') {
                    Some(w) => (w, "."),
                    None => (word, ""),
                };
                span()
                    .class("w-in")
                    .child(em().attr("data-text", w.to_string()).text(w.to_string()))
                    .text(dot)
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

/// The sections of the home page, in order: anchor and label.
fn sections(lang: Lang) -> [(&'static str, &'static str); 4] {
    let d = t(lang);
    [
        ("playground", d.nav.playground),
        ("how", d.nav.how),
        ("supported", d.nav.supported),
        ("install", d.nav.install),
    ]
}

/// A section of the home page: its anchor there, its address from another page.
fn section_href(route: Route, id: &str) -> String {
    if route.page == Page::Home {
        format!("#{id}")
    } else {
        format!("{}#{id}", Route::new(route.lang, Page::Home).href())
    }
}

fn external(link: Element) -> Element {
    link.attr("rel", "external noopener")
        .attr("target", "_blank")
}

/// The bar at the top: the logo; the menu (the sections of the home page, the
/// documentation and its mega menu, GitHub; on small screens a whole screen that also
/// lists every page, the resources and the languages); the search, the install button,
/// the language picker and the menu button.
fn header(route: Route) -> Element {
    let lang = route.lang;
    let d = t(lang);
    let home = route.page == Page::Home;
    let links = sections(lang).map(|(id, label)| {
        let link = a().href(section_href(route, id)).child(span().text(label));
        if home {
            link.attr("data-spy", id)
        } else {
            link
        }
    });
    el("header")
        .class("top")
        .child(
            a().class("brand")
                .href(Route::new(lang, Page::Home).href())
                .child(logo())
                .child(span().text("Solidify")),
        )
        .child(
            nav()
                .id("menu")
                .class("menu")
                .attr("aria-label", "Menu")
                .child(
                    div()
                        .class("menu-links")
                        .children(links)
                        .child(docs_menu(route))
                        .child(external(
                            a().class("menu-github")
                                .href(GITHUB)
                                .child(span().text(d.nav.github)),
                        )),
                )
                .child(menu_more(route)),
        )
        .child(
            div()
                .class("tools")
                .child(
                    a().class("search-open")
                        .href(Route::new(lang, Page::Docs).href())
                        .attr("data-search-open", "")
                        .attr("aria-label", d.docs.search_label)
                        .child(icon("search"))
                        .child(span().class("search-open-text").text(d.docs.search))
                        .child(el("kbd").text("/")),
                )
                .child(
                    button()
                        .id("install-app")
                        .class("install-app")
                        .attr("type", "button")
                        .attr("hidden", "")
                        .text(d.nav.install_app),
                )
                .child(lang_picker(route))
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

/// "Docs" in the menu, and the panel it opens when pointed at or focused (CSS only): the
/// pages by category, and a card that leads to the playground.
fn docs_menu(route: Route) -> Element {
    let lang = route.lang;
    let d = &t(lang).docs;
    let mut link = a()
        .class("docs-link")
        .href(Route::new(lang, Page::Docs).href())
        .child(span().text(d.nav))
        .child(span().class("chev").attr("aria-hidden", "true"));
    if route.page != Page::Home {
        link = link.class("on");
    }
    let current = match route.page {
        Page::Doc(slug) => slug,
        _ => "",
    };
    let columns = CATEGORIES.iter().enumerate().map(|(c, category)| {
        div()
            .class("mega-col")
            .attr("style", format!("--i:{c}"))
            .child(
                p().class("mega-title")
                    .child(icon(category.icon))
                    .text(lang.pick(category.title)),
            )
            .child(ul().children(pages_of(c).map(|page| {
                let mut item = a()
                    .href(doc_href(lang, page.slug, None))
                    .child(span().class("mega-page").text(lang.pick(page.title)))
                    .child(span().class("mega-sum").text(lang.pick(page.summary)));
                if page.slug == current {
                    item = item.attr("aria-current", "page");
                }
                li().child(item)
            })))
    });
    div().class("menu-docs").child(link).child(
        div().class("mega").child(
            div().class("mega-inner").children(columns).child(
                a().class("mega-card")
                    .href(section_href(route, "playground"))
                    .attr("style", format!("--i:{}", CATEGORIES.len()))
                    .child(span().class("mega-bolt").child(bolt()))
                    .child(strong().text(d.menu_try))
                    .child(span().text(d.menu_try_text))
                    .child(arrow()),
            ),
        ),
    )
}

/// What the menu adds on small screens, under the sections: every page of the
/// documentation, the resources, the languages.
fn menu_more(route: Route) -> Element {
    let lang = route.lang;
    let d = t(lang);
    let pages = CATEGORIES.iter().enumerate().map(|(c, category)| {
        div()
            .class("menu-cat")
            .child(
                p().child(icon(category.icon))
                    .text(lang.pick(category.title)),
            )
            .child(ul().children(pages_of(c).map(|page| {
                li().child(
                    a().href(doc_href(lang, page.slug, None))
                        .text(lang.pick(page.title)),
                )
            })))
    });
    let resources = [
        (d.docs.title, Route::new(lang, Page::Docs).href(), false),
        (d.footer.source, GITHUB.to_string(), true),
        (d.footer.npm, NPM.to_string(), true),
        (d.footer.issues, ISSUES.to_string(), true),
        (d.footer.license_link, LICENSE.to_string(), true),
    ];
    div()
        .class("menu-more")
        .child(
            div()
                .class("menu-group")
                .child(p().class("menu-title").text(d.docs.title))
                .child(div().class("menu-pages").children(pages)),
        )
        .child(
            div()
                .class("menu-group")
                .child(p().class("menu-title").text(d.docs.menu_resources))
                .child(ul().class("menu-res").children(resources.into_iter().map(
                    |(label, href, out)| {
                        let link = a().href(href).text(label);
                        li().child(if out { external(link) } else { link })
                    },
                ))),
        )
        .child(
            div()
                .class("menu-group")
                .child(p().class("menu-title").text(d.nav.language))
                .child(
                    div()
                        .class("menu-langs")
                        .children(language_links(route, false)),
                ),
        )
}

/// The same page in each language, then "Automatic" (the English page, the browser
/// choosing): links the client remembers (`data-lang`).
fn language_links(route: Route, codes: bool) -> Vec<Element> {
    let lang = route.lang;
    let mut links: Vec<Element> = Lang::ALL
        .iter()
        .map(|other| {
            let mut item = a()
                .href(route.with_lang(*other).href())
                .attr("hreflang", other.code())
                .attr("lang", other.code())
                .attr("data-lang", other.code());
            if codes {
                item = item.child(span().class("code").text(other.code().to_uppercase()));
            }
            item = item.child(span().text(t(*other).lang_name));
            if *other == lang {
                item = item.attr("aria-current", "true");
            }
            item
        })
        .collect();
    let mut auto = a()
        .href(route.with_lang(Lang::En).href())
        .attr("data-lang", "auto");
    if codes {
        auto = auto.child(span().class("code").text("A"));
    }
    links.push(auto.child(span().text(t(lang).nav.automatic)));
    links
}

/// The language picker: works without WebAssembly (`<details>`); the client remembers the
/// choice, and "Automatic" hands it back to the browser.
fn lang_picker(route: Route) -> Element {
    let lang = route.lang;
    let d = t(lang);
    let links = language_links(route, true);
    let last = links.len() - 1;
    let list =
        ul().class("lang-list")
            .attr("role", "list")
            .children(links.into_iter().enumerate().map(|(i, link)| {
                let item = li().child(link);
                if i == last { item.class("auto") } else { item }
            }));
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

/// One result of the search, with what it is compared on.
struct Entry {
    href: String,
    title: String,
    context: String,
    words: String,
    /// A page (listed before anything is typed), not a section of one.
    page: bool,
}

/// What the search finds in `lang`: the sections of the home page, every page of the
/// documentation, then every section of each, in reading order.
fn search_entries(lang: Lang) -> Vec<Entry> {
    let d = t(lang);
    let mut entries: Vec<Entry> = sections(lang)
        .into_iter()
        .map(|(id, label)| Entry {
            href: section_href(Route::new(lang, Page::Docs), id),
            title: label.to_string(),
            context: d.docs.home.to_string(),
            words: norm(&format!("{label} Solidify")),
            page: false,
        })
        .collect();
    for page in &PAGES {
        let doc = crate::docs::page(lang, page.slug);
        let category = lang.pick(CATEGORIES[page.category].title);
        let href = doc_href(lang, page.slug, None);
        entries.push(Entry {
            href: href.clone(),
            title: doc.title.clone(),
            context: category.to_string(),
            words: norm(&format!(
                "{} {} {} {} {category}",
                doc.title,
                lang.pick(page.title),
                lang.pick(page.summary),
                doc.description
            )),
            page: true,
        });
        for heading in &doc.toc {
            entries.push(Entry {
                href: format!("{href}#{}", heading.id),
                title: heading.text.clone(),
                context: doc.title.clone(),
                words: norm(&format!("{} {}", heading.text, doc.title)),
                page: false,
            });
        }
    }
    entries
}

/// The search: a dialog the client opens (the `/` key, Ctrl K, the button in the header)
/// and fills as the visitor types, from the entries written here. Without the client, the
/// button leads to the documentation's home.
fn search(lang: Lang) -> Element {
    let d = &t(lang).docs;
    let results = search_entries(lang).into_iter().map(|entry| {
        let mut item = li()
            .class(if entry.page { "page" } else { "section" })
            .attr("data-title", norm(&entry.title))
            .attr("data-words", entry.words)
            .child(
                a().href(entry.href)
                    .child(icon(if entry.page { "page" } else { "hash" }))
                    .child(span().class("sr-title").text(entry.title))
                    .child(span().class("sr-context").text(entry.context)),
            );
        if !entry.page {
            item = item.attr("hidden", "");
        }
        item
    });
    el("dialog")
        .class("search")
        .id("search")
        .attr("aria-label", d.search_label)
        .child(
            div()
                .class("search-box")
                .child(
                    div()
                        .class("search-field")
                        .child(icon("search"))
                        .child(
                            input()
                                .attr("type", "search")
                                .attr("placeholder", d.search_placeholder)
                                .attr("aria-label", d.search_label)
                                .attr("aria-controls", "search-results")
                                .attr("autocomplete", "off")
                                .attr("spellcheck", "false"),
                        )
                        .child(
                            button()
                                .class("search-close")
                                .attr("type", "button")
                                .attr("aria-label", d.close)
                                .child(el("kbd").text("Esc")),
                        ),
                )
                .child(
                    ul().id("search-results")
                        .class("search-results")
                        .attr("role", "list")
                        .children(results),
                )
                .child(
                    p().class("search-empty")
                        .attr("hidden", "")
                        .text(d.search_empty),
                )
                .child(p().class("search-keys").text(d.search_keys)),
        )
}

/// Back to the top, once the page has scrolled: a ring that fills with the reading
/// (`--p`, set by the client).
fn to_top(lang: Lang) -> Element {
    a().class("to-top-ring")
        .href("#main")
        .attr("aria-label", t(lang).footer.top)
        .child(
            svg()
                .attr("viewBox", "0 0 48 48")
                .attr("aria-hidden", "true")
                .child(
                    el("circle")
                        .class("ring-track")
                        .attr("cx", "24")
                        .attr("cy", "24")
                        .attr("r", "21"),
                )
                .child(
                    el("circle")
                        .class("ring-fill")
                        .attr("cx", "24")
                        .attr("cy", "24")
                        .attr("r", "21")
                        .attr("pathLength", "100"),
                ),
        )
        .child(icon("up"))
}

/// The footer: the brand and the command to run it; the product, documentation,
/// resources, languages and family columns; a large wordmark; the legal line.
fn footer(route: Route) -> Element {
    let lang = route.lang;
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
    let out = |href: &'static str, label: &'static str| {
        external(a().href(href).child(span().text(label)))
    };
    let product: Vec<Element> = sections(lang)
        .into_iter()
        .map(|(id, label)| a().href(section_href(route, id)).child(span().text(label)))
        .collect();
    let docs: Vec<Element> = std::iter::once(
        a().href(Route::new(lang, Page::Docs).href())
            .child(span().text(d.docs.all_pages)),
    )
    .chain(
        [
            "introduction",
            "quick-start",
            "cli",
            "library",
            "mapping",
            "faq",
        ]
        .into_iter()
        .filter_map(crate::catalog::find)
        .map(|page| {
            a().href(doc_href(lang, page.slug, None))
                .child(span().text(lang.pick(page.title)))
        }),
    )
    .collect();
    let resources = vec![
        out(GITHUB, f.source),
        out(NPM, f.npm),
        out(ISSUES, f.issues),
        out(LICENSE, f.license_link),
    ];
    let languages = language_links(route, false);
    let family: Vec<Element> = FAMILY.iter().map(|(name, url)| out(url, name)).collect();
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
                                .href(Route::new(lang, Page::Home).href())
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
                .child(column(d.docs.title, docs, 2))
                .child(column(f.resources, resources, 3))
                .child(column(d.nav.language, languages, 4))
                .child(column(f.projects, family, 5)),
        )
        .child(
            div()
                .class("wordmark-wrap")
                .attr("aria-hidden", "true")
                .child(
                    p().class("wordmark").attr("data-reveal", "neon").children(
                        "Solidify"
                            .chars()
                            .enumerate()
                            .map(|(n, c)| span().style(format!("--i:{n}")).text(c.to_string())),
                    ),
                ),
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
            assert!(html.contains("class=\"loader\""));
        }
        let fr = home(Lang::Fr, "https://example.com");
        assert!(fr.contains("<html lang=\"fr\""));
        assert!(fr.contains("rel=\"canonical\" href=\"https://example.com/fr/\""));
    }

    #[test]
    fn every_page_renders_with_its_menu_and_search() {
        for route in Route::all() {
            let html = render(route, "https://example.com");
            let path = route.path();
            assert!(html.contains(&format!(
                "rel=\"canonical\" href=\"https://example.com{path}\""
            )));
            // The menu leads to every page of the documentation, and so does the search.
            for page in &PAGES {
                let href = doc_href(route.lang, page.slug, None);
                assert!(
                    html.contains(&format!("href=\"{href}\"")),
                    "{path} → {href}"
                );
            }
            assert!(html.contains("id=\"search\""));
            // The other languages of the same page.
            let es = route.with_lang(Lang::Es).path();
            assert!(html.contains(&format!("hreflang=\"es\" href=\"https://example.com{es}\"")));
            // Only the home page has the preloader.
            assert_eq!(
                html.contains("class=\"loader\""),
                route.page == Page::Home,
                "{path}"
            );
        }
        let cli = render(
            Route::new(Lang::Fr, Page::Doc("cli")),
            "https://example.com",
        );
        assert!(cli.contains("<html lang=\"fr\""));
        assert!(cli.contains("href=\"/fr/#playground\""));
        assert!(cli.contains("aria-current=\"page\""));
    }
}
