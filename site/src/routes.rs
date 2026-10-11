//! URLs of the site: `/` (English) and `/fr/` (French), behind an optional base path
//! (`/Solidify` when the site is published on GitHub Pages).

use std::sync::OnceLock;

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

    /// The page for a request path, if it is a page.
    pub fn from_path(path: &str) -> Option<Lang> {
        let path = path.strip_prefix(base()).unwrap_or(path);
        match path.trim_end_matches('/') {
            "" | "/index.html" => Some(Lang::En),
            "/fr" | "/fr/index.html" => Some(Lang::Fr),
            "/es" | "/es/index.html" => Some(Lang::Es),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_are_found_by_path() {
        assert_eq!(Lang::from_path("/"), Some(Lang::En));
        assert_eq!(Lang::from_path("/fr/"), Some(Lang::Fr));
        assert_eq!(Lang::from_path("/fr"), Some(Lang::Fr));
        assert_eq!(Lang::from_path("/es/"), Some(Lang::Es));
        assert_eq!(Lang::from_path("/de/"), None);
        assert_eq!(link("/fr/"), "/fr/");
    }
}
