//! The browser bundle, Rust compiled to WebAssembly: the site has no JavaScript of its own
//! (only the loader `wasm-bindgen` generates). It opens the visitor's language, hydrates
//! the playground, and runs the motion and small conveniences of the page:
//!
//! - [`language`]: the page in the visitor's language, the picker's choice kept;
//! - [`motion`]: reveals, scroll state, pointer effects, copy buttons, the menu;
//! - [`app`]: the service worker (offline use) and the "Install the app" button.
//!
//! Everything here adds to a page that already works: links, the language picker (a
//! `<details>`) and the text are plain HTML, and the hero's entrance is CSS.

mod app;
mod language;
mod motion;

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    // Links of a site published under a sub-path (GitHub Pages: `/Solidify`).
    if let Some(base) = root().get_attribute("data-base") {
        crate::routes::set_base(&base);
    }
    // Another language first: nothing else is worth starting on this page.
    if language::redirect() {
        return;
    }
    // `app`: the client runs (the small-screen menu needs it); `motion`: what is below
    // may wait to be revealed.
    let _ = root().class_list().add_1("app");
    if !reduced_motion() {
        let _ = root().class_list().add_1("motion");
    }
    active::hydrate(&crate::islands::ISLANDS);
    language::init();
    motion::init();
    app::init();
}

/// Copies `text` to the clipboard.
pub fn copy(text: &str) {
    let _ = window().navigator().clipboard().write_text(text);
}

pub(crate) fn window() -> web_sys::Window {
    web_sys::window().expect("a window")
}

pub(crate) fn document() -> web_sys::Document {
    window().document().expect("a document")
}

/// `<html>`.
pub(crate) fn root() -> web_sys::HtmlElement {
    document()
        .document_element()
        .expect("<html>")
        .unchecked_into()
}

/// Whether `query` (a media query) matches.
pub(crate) fn media(query: &str) -> bool {
    window()
        .match_media(query)
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

/// Whether the system asks for less motion.
pub(crate) fn reduced_motion() -> bool {
    media("(prefers-reduced-motion: reduce)")
}

/// Calls `f` with every `event` of `target`, for the page's lifetime.
pub(crate) fn on<E: JsCast + 'static>(
    target: &web_sys::EventTarget,
    event: &str,
    f: impl FnMut(E) + 'static,
) {
    let mut f = f;
    let closure =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| f(e.unchecked_into()));
    let _ = target.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
    closure.forget();
}

/// Like [`on`], for scroll and pointer events the browser need not wait for.
pub(crate) fn on_passive<E: JsCast + 'static>(
    target: &web_sys::EventTarget,
    event: &str,
    f: impl FnMut(E) + 'static,
) {
    let mut f = f;
    let closure =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| f(e.unchecked_into()));
    let options = web_sys::AddEventListenerOptions::new();
    options.set_passive(true);
    let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
        event,
        closure.as_ref().unchecked_ref(),
        &options,
    );
    closure.forget();
}

/// Calls `f` once, after `ms` milliseconds.
pub(crate) fn later(ms: i32, f: impl FnOnce() + 'static) {
    let f = Closure::once_into_js(f);
    let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(f.unchecked_ref(), ms);
}

type FrameCallback = Closure<dyn FnMut(f64)>;

/// Calls `f(timestamp)` on the next animation frame, and on every one after while it
/// returns `true`.
pub(crate) fn frames(f: impl FnMut(f64) -> bool + 'static) {
    let f = Rc::new(RefCell::new(f));
    let slot: Rc<RefCell<Option<FrameCallback>>> = Rc::new(RefCell::new(None));
    let again = slot.clone();
    *slot.borrow_mut() = Some(Closure::new(move |now: f64| {
        if (f.borrow_mut())(now) {
            if let Some(cb) = again.borrow().as_ref() {
                let _ = window().request_animation_frame(cb.as_ref().unchecked_ref());
            }
        } else {
            again.borrow_mut().take();
        }
    }));
    if let Some(cb) = slot.borrow().as_ref() {
        let _ = window().request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

/// Every element matching `selector`.
pub(crate) fn all(selector: &str) -> Vec<web_sys::HtmlElement> {
    let Ok(list) = document().query_selector_all(selector) else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.item(i)?.dyn_into().ok())
        .collect()
}

/// The first element matching `selector`.
pub(crate) fn first(selector: &str) -> Option<web_sys::HtmlElement> {
    document()
        .query_selector(selector)
        .ok()
        .flatten()
        .and_then(|e| e.dyn_into().ok())
}

/// The element an event happened on, or its nearest ancestor matching `selector`.
pub(crate) fn closest(event: &web_sys::Event, selector: &str) -> Option<web_sys::HtmlElement> {
    event
        .target()?
        .dyn_into::<web_sys::Element>()
        .ok()?
        .closest(selector)
        .ok()
        .flatten()?
        .dyn_into()
        .ok()
}
