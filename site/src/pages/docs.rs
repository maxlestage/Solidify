//! The documentation: its home (the search, where to start, every page by category) and
//! its pages (the list of pages beside, the page's sections on the other side, the pages
//! before and after it). The pages are Markdown, rendered by `crate::docs`.

use active::prelude::*;

use super::{Head, arrow, bolt, document, emphasized, icon, json};
use crate::catalog::{self, CATEGORIES, PAGES, pages_of};
use crate::i18n::t;
use crate::routes::{Page, Route, doc_href};

/// The documentation's home.
pub fn hub(route: Route, origin: &str) -> String {
    let lang = route.lang;
    let d = &t(lang).docs;
    // Where to start: (slug, icon).
    let start = [
        ("introduction", "bolt"),
        ("quick-start", "play"),
        ("mapping", "table"),
    ];
    let cards = start.iter().enumerate().filter_map(|(i, (slug, name))| {
        let page = catalog::find(slug)?;
        Some(
            a().class("start-card spot charged")
                .href(doc_href(lang, page.slug, None))
                .attr("data-reveal", "")
                .attr("data-tilt", "8")
                .attr("style", format!("--i:{i}"))
                .child(span().class("start-icon").child(icon(name)))
                .child(strong().text(lang.pick(page.title)))
                .child(span().class("start-text").text(lang.pick(page.summary)))
                .child(span().class("start-arrow").child(arrow())),
        )
    });
    let categories = CATEGORIES.iter().enumerate().map(|(c, category)| {
        let pages: Vec<_> = pages_of(c).collect();
        el("article")
            .class("doc-cat spot charged")
            .attr("data-reveal", "")
            .attr("style", format!("--i:{c}"))
            .child(
                div()
                    .class("doc-cat-head")
                    .child(span().class("doc-cat-icon").child(icon(category.icon)))
                    .child(h2().text(lang.pick(category.title)))
                    .child(
                        span()
                            .class("doc-cat-count")
                            .text(d.pages.replace("{}", &pages.len().to_string())),
                    ),
            )
            .child(ul().children(pages.iter().map(|page| {
                li().child(
                    a().href(doc_href(lang, page.slug, None))
                        .child(strong().text(lang.pick(page.title)))
                        .child(span().text(lang.pick(page.summary))),
                )
            })))
    });
    let main = main()
        .id("main")
        .class("docs-home")
        .child(
            section()
                .class("docs-hero")
                .child(core())
                .child(
                    p().class("eyebrow intro")
                        .attr("style", "--d:0")
                        .child(span().class("dot").attr("aria-hidden", "true"))
                        .text(format!("Solidify · v{}", env!("CARGO_PKG_VERSION"))),
                )
                .child(emphasized(h1(), &format!("*{}*", d.title)))
                .child(p().class("lead intro").attr("style", "--d:3").text(d.lead))
                .child(
                    a().class("search-wide intro")
                        .attr("style", "--d:5")
                        .href("#pages")
                        .attr("data-search-open", "")
                        .child(icon("search"))
                        .child(span().text(d.search_placeholder))
                        .child(el("kbd").text("/")),
                ),
        )
        .child(
            section()
                .class("docs-start")
                .child(
                    h2().class("docs-h2")
                        .attr("data-reveal", "")
                        .text(d.start_here),
                )
                .child(div().class("start-grid").children(cards)),
        )
        .child(
            section()
                .id("pages")
                .class("docs-cats")
                .child(
                    h2().class("docs-h2")
                        .attr("data-reveal", "")
                        .text(d.all_pages),
                )
                .child(div().class("cat-grid").children(categories)),
        );
    let head = Head {
        title: format!("{} · Solidify", d.title),
        description: d.lead.to_string(),
        json_ld: format!(
            r#"{{"@context":"https://schema.org","@type":"CollectionPage","name":"{}","description":"{}","inLanguage":"{}"}}"#,
            json(d.title),
            json(d.lead),
            lang.code()
        ),
    };
    document(route, origin, head, main)
}

/// The documentation's emblem, CSS only: a bolt at the heart of three rings of charge
/// turning at their own speeds, sparks orbiting it.
fn core() -> Element {
    div()
        .class("core")
        .attr("aria-hidden", "true")
        .child(span().class("core-glow"))
        .children((0..3).map(|n| span().class("core-ring").attr("style", format!("--i:{n}"))))
        .children((0..5).map(|n| span().class("core-spark").attr("style", format!("--i:{n}"))))
        .child(span().class("core-bolt").child(bolt()))
}

/// A page of the documentation.
pub fn page(route: Route, origin: &str, slug: &'static str) -> String {
    let lang = route.lang;
    let d = &t(lang).docs;
    let doc = crate::docs::page(lang, slug);
    let index = catalog::index(slug).expect("a page of the catalog");
    let page = &PAGES[index];
    let pager = |page: Option<&catalog::Page>, label: &str, class: &str| {
        page.map(|page| {
            a().class(format!("pager-link {class}"))
                .href(doc_href(lang, page.slug, None))
                .attr("rel", if class == "prev" { "prev" } else { "next" })
                .child(span().class("pager-label").text(label.to_string()))
                .child(span().class("pager-title").text(lang.pick(page.title)))
        })
    };
    let category = lang.pick(CATEGORIES[page.category].title);
    let article = el("article")
        .class("doc")
        .child(
            p().class("crumbs")
                .child(a().href(Route::new(lang, Page::Docs).href()).text(d.title))
                .child(
                    span()
                        .class("crumb-sep")
                        .attr("aria-hidden", "true")
                        .text("/"),
                )
                .child(span().text(category)),
        )
        .child(div().class("doc-body").html(doc.html.clone()))
        .child(
            div().class("doc-foot").child(super::external(
                a().class("doc-edit")
                    .href(crate::docs::source_url(lang, slug))
                    .child(icon("edit"))
                    .child(span().text(d.edit)),
            )),
        )
        .child(
            nav()
                .class("pager")
                .attr("aria-label", format!("{} / {}", d.prev, d.next))
                .children(pager(
                    index.checked_sub(1).map(|i| &PAGES[i]),
                    d.prev,
                    "prev",
                ))
                .children(pager(PAGES.get(index + 1), d.next, "next")),
        );
    let toc = nav()
        .class("doc-toc")
        .attr("aria-label", d.toc)
        .child(p().class("toc-title").text(d.toc))
        .child(
            div()
                .class("toc-list")
                .child(span().class("toc-wire").attr("aria-hidden", "true"))
                .child(ul().children(doc.toc.iter().map(|entry| {
                    li().class(if entry.level == 3 { "sub" } else { "main" })
                        .child(
                            a().href(format!("#{}", entry.id))
                                .attr("data-toc", entry.id.clone())
                                .text(entry.text.clone()),
                        )
                }))),
        );
    let main = main()
        .id("main")
        .class("docs")
        .child(
            div()
                .class("docs-bar")
                .child(
                    button()
                        .class("side-toggle")
                        .attr("type", "button")
                        .attr("data-toggle-side", "")
                        .attr("aria-controls", "doc-side")
                        .attr("aria-expanded", "false")
                        .child(icon("menu"))
                        .child(span().text(d.all_pages)),
                )
                .child(span().class("docs-bar-title").text(doc.title.clone())),
        )
        .child(sidebar(route, slug))
        .child(
            div()
                .class("side-scrim")
                .attr("data-toggle-side", "")
                .attr("aria-hidden", "true"),
        )
        .child(article)
        .child(el("aside").class("doc-aside").child(toc));
    let head = Head {
        title: format!("{} · {} · Solidify", doc.title, d.title),
        description: doc.description.clone(),
        json_ld: format!(
            r#"{{"@context":"https://schema.org","@type":"TechArticle","headline":"{}","description":"{}","inLanguage":"{}","author":{{"@type":"Person","name":"{}"}},"isPartOf":{{"@type":"CreativeWork","name":"{}"}}}}"#,
            json(&doc.title),
            json(&doc.description),
            lang.code(),
            super::AUTHOR,
            json(d.title)
        ),
    };
    document(route, origin, head, main)
}

/// The list of pages beside a page (a drawer on small screens), the current one marked.
fn sidebar(route: Route, current: &str) -> Element {
    let lang = route.lang;
    let d = &t(lang).docs;
    el("aside")
        .id("doc-side")
        .class("doc-side")
        .attr("aria-label", d.all_pages)
        .child(
            a().class("side-home")
                .href(Route::new(lang, Page::Docs).href())
                .child(icon("book"))
                .child(span().text(d.title)),
        )
        .child(
            nav()
                .class("side-nav")
                .children(CATEGORIES.iter().enumerate().map(|(c, category)| {
                    div()
                        .class("side-group")
                        .child(
                            p().class("side-cat")
                                .child(icon(category.icon))
                                .text(lang.pick(category.title)),
                        )
                        .child(ul().children(pages_of(c).map(|page| {
                            let link = a()
                                .href(doc_href(lang, page.slug, None))
                                .text(lang.pick(page.title));
                            li().child(if page.slug == current {
                                link.class("on").attr("aria-current", "page")
                            } else {
                                link
                            })
                        })))
                })),
        )
}
