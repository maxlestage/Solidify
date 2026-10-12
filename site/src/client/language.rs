//! The page in the visitor's language.
//!
//! - A language picked in the header or the footer (`a[data-lang]`) is kept in
//!   `localStorage` under [`STORE`], and opened from then on.
//! - Without one, the browser's languages (`navigator.languages`) decide, on the English
//!   pages only: a link straight to `/fr/…` or `/es/…` stays where it leads.
//! - Either way, the same page opens in that language (the language picker's link to it).
//! - "Automatic" (`data-lang="auto"`) forgets the choice.

use wasm_bindgen::JsCast;

use super::{closest, document, later, on, root, window};
use crate::routes::{Lang, link};

/// The `localStorage` key of the chosen language.
const STORE: &str = "solidify-lang";

fn storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}

/// The first of the browser's languages the site has.
fn browser_language() -> Option<Lang> {
    window()
        .navigator()
        .languages()
        .iter()
        .filter_map(|l| l.as_string())
        .find_map(|l| Lang::parse(&l.get(..2)?.to_ascii_lowercase()))
}

/// Opens the page in the visitor's language when this one is not it. Returns whether
/// the browser is leaving for it.
pub fn redirect() -> bool {
    let here = Lang::parse(&root().lang()).unwrap_or(Lang::En);
    let chosen = storage()
        .and_then(|s| s.get_item(STORE).ok().flatten())
        .and_then(|code| Lang::parse(&code));
    let wanted = match chosen {
        Some(lang) => Some(lang),
        None if here == Lang::En => browser_language(),
        None => None,
    };
    match wanted {
        Some(lang) if lang != here => {
            // The same page in that language: the picker links to it.
            let target = super::first(&format!(".lang-list a[data-lang=\"{}\"]", lang.code()))
                .and_then(|a| a.get_attribute("href"))
                .unwrap_or_else(|| link(lang.path()));
            let location = window().location();
            let hash = location.hash().unwrap_or_default();
            location.replace(&format!("{target}{hash}")).is_ok()
        }
        _ => false,
    }
}

/// Keeps the language picked, fades the page out on the way to it, and closes the picker
/// on a click elsewhere.
pub fn init() {
    let doc: web_sys::EventTarget = document().into();
    on(&doc, "click", |event: web_sys::MouseEvent| {
        let Some(a) = closest(&event, "a[data-lang]") else {
            return;
        };
        let code = a.get_attribute("data-lang").unwrap_or_default();
        if let Some(storage) = storage() {
            let _ = if code == "auto" {
                storage.remove_item(STORE)
            } else {
                storage.set_item(STORE, &code)
            };
        }
        if super::reduced_motion() || event.meta_key() || event.ctrl_key() || event.shift_key() {
            return;
        }
        event.prevent_default();
        let _ = root().class_list().add_1("leaving");
        let href = a.unchecked_into::<web_sys::HtmlAnchorElement>().href();
        later(260, move || {
            let hash = window().location().hash().unwrap_or_default();
            let _ = window().location().set_href(&format!("{href}{hash}"));
        });
    });
    // The picker (a <details>) closes on a click outside it.
    on(&doc, "click", |event: web_sys::MouseEvent| {
        if let Some(picker) = super::first(".lang-picker[open]")
            && closest(&event, ".lang-picker").is_none()
        {
            let _ = picker.remove_attribute("open");
        }
    });
}
