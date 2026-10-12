//! URLs of the site, behind an optional base path (`/Solidify` when the site is published
//! on GitHub Pages). English is at the root, the other languages under their code:
//!
//! | Page | English | French |
//! | --- | --- | --- |
//! | Home | `/` | `/fr/` |
//! | Documentation | `/docs/` | `/fr/docs/` |
//! | A page of it | `/docs/cli/` | `/fr/docs/cli/` |

use std::sync::OnceLock;

use crate::catalog;

static BASE: OnceLock<String> = OnceLock::new();

/// Sets the base path once, before anything is rendered: `""` (the default) or `/Solidify`.
/// The server takes it from `BASE_PATH`, the browser from `<html data-base>`.
pub fn set_base(base: &str) {
    let base = base.trim().trim_end_matches('/');
    let base = match base {
        "" => String::new(),
        b if b.starts_with('/') => b.to_string(),
        b => format!("/{b}"),
    };
    let _ = BASE.set(base);
}

/// The base path, without trailing slash (empty by default).
pub fn base() -> &'static str {
    BASE.get().map_or("", String::as_str)
}

/// A link to `path` (which starts with `/`), behind the base path.
pub fn link(path: &str) -> String {
    format!("{}{path}", base())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Fr,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::En, Lang::Fr, Lang::Es];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
        }
    }

    pub fn parse(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code)
    }

    /// The path of the home page in this language, without the base path.
    pub fn path(self) -> &'static str {
        match self {
            Lang::En => "/",
            Lang::Fr => "/fr/",
            Lang::Es => "/es/",
        }
    }

    /// One of three texts, given in the order English, French, Spanish.
    pub fn pick<T: Copy>(self, texts: [T; 3]) -> T {
        texts[self as usize]
    }
}

/// A page of the site, in any language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Home,
    /// The documentation's home: its categories and pages, and the search.
    Docs,
    /// A page of the documentation, by its slug (see [`catalog::PAGES`]).
    Doc(&'static str),
}

/// A page in a language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Route {
    pub lang: Lang,
    pub page: Page,
}

impl Route {
    pub fn new(lang: Lang, page: Page) -> Route {
        Route { lang, page }
    }

    /// The route of a request path (the base path is removed first, `index.html` is the
    /// folder); `None` when it is not a page.
    pub fn parse(path: &str) -> Option<Route> {
        let path = path.strip_prefix(base()).unwrap_or(path);
        let path = path.strip_suffix("index.html").unwrap_or(path);
        let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let lang = match parts.first().and_then(|p| Lang::parse(p)) {
            // English lives at the root: `/en/` is not one of its addresses.
            Some(Lang::En) => return None,
            Some(lang) => {
                parts.remove(0);
                lang
            }
            None => Lang::En,
        };
        let page = match parts[..] {
            [] => Page::Home,
            ["docs"] => Page::Docs,
            ["docs", slug] => Page::Doc(catalog::find(slug)?.slug),
            _ => return None,
        };
        Some(Route { lang, page })
    }

    /// The path of the page, without the base path.
    pub fn path(&self) -> String {
        let home = self.lang.path();
        match self.page {
            Page::Home => home.to_string(),
            Page::Docs => format!("{home}docs/"),
            Page::Doc(slug) => format!("{home}docs/{slug}/"),
        }
    }

    /// A link to the page, behind the base path.
    pub fn href(&self) -> String {
        link(&self.path())
    }

    /// The same page in another language.
    pub fn with_lang(&self, lang: Lang) -> Route {
        Route { lang, ..*self }
    }

    /// Every page of the site, in every language: what the static export writes and the
    /// service worker keeps.
    pub fn all() -> Vec<Route> {
        let mut routes = Vec::new();
        for lang in Lang::ALL {
            routes.push(Route::new(lang, Page::Home));
            routes.push(Route::new(lang, Page::Docs));
            routes.extend(
                catalog::PAGES
                    .iter()
                    .map(|p| Route::new(lang, Page::Doc(p.slug))),
            );
        }
        routes
    }
}

/// A link to a page of the documentation in `lang`, with an optional anchor.
pub fn doc_href(lang: Lang, slug: &'static str, anchor: Option<&str>) -> String {
    let href = Route::new(lang, Page::Doc(slug)).href();
    match anchor {
        Some(anchor) if !anchor.is_empty() => format!("{href}#{anchor}"),
        _ => href,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_are_found_by_path() {
        let home = |lang| Some(Route::new(lang, Page::Home));
        assert_eq!(Route::parse("/"), home(Lang::En));
        assert_eq!(Route::parse("/index.html"), home(Lang::En));
        assert_eq!(Route::parse("/fr/"), home(Lang::Fr));
        assert_eq!(Route::parse("/fr"), home(Lang::Fr));
        assert_eq!(Route::parse("/es/index.html"), home(Lang::Es));
        assert_eq!(Route::parse("/de/"), None);
        assert_eq!(Route::parse("/en/"), None);
        assert_eq!(
            Route::parse("/docs/"),
            Some(Route::new(Lang::En, Page::Docs))
        );
        assert_eq!(
            Route::parse("/fr/docs/cli/"),
            Some(Route::new(Lang::Fr, Page::Doc("cli")))
        );
        assert_eq!(
            Route::parse("/es/docs/cli/index.html"),
            Some(Route::new(Lang::Es, Page::Doc("cli")))
        );
        assert_eq!(Route::parse("/docs/nope/"), None);
        assert_eq!(Route::parse("/static/app.css"), None);
        assert_eq!(link("/fr/"), "/fr/");
    }

    #[test]
    fn every_page_has_one_address() {
        let all = Route::all();
        assert_eq!(all.len(), 3 * (2 + catalog::PAGES.len()));
        for route in &all {
            assert_eq!(
                Route::parse(&route.path()),
                Some(*route),
                "{}",
                route.path()
            );
        }
        let mut paths: Vec<String> = all.iter().map(Route::path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), all.len());
        assert_eq!(
            Route::new(Lang::Fr, Page::Doc("cli"))
                .with_lang(Lang::En)
                .path(),
            "/docs/cli/"
        );
        assert_eq!(
            doc_href(Lang::Es, "cli", Some("options")),
            "/es/docs/cli/#options"
        );
    }
}
