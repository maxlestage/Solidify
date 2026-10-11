//! The browser bundle: hydrates the playground.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    // Links of a site published under a sub-path (GitHub Pages: `/Solidify`).
    if let Some(base) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .and_then(|html| html.get_attribute("data-base"))
    {
        crate::routes::set_base(&base);
    }
    active::hydrate(&crate::islands::ISLANDS);
}

/// Copies `text` to the clipboard.
pub fn copy(text: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.navigator().clipboard().write_text(text);
    }
}
