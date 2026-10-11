//! The installable app: registers the service worker (Rust too: site/worker), and shows
//! "Install the app" when the browser offers to install the site.

use std::cell::RefCell;

use wasm_bindgen::JsValue;

use super::{document, first, on, window};
use crate::routes::link;

thread_local! {
    /// The browser's offer to install the app (`beforeinstallprompt`), kept for the
    /// button.
    static OFFER: RefCell<Option<web_sys::Event>> = const { RefCell::new(None) };
}

pub fn init() {
    // Once the page has loaded, so that the worker's downloads come after the page's
    // (the WebAssembly often starts after that already).
    let register = || {
        let _ = window()
            .navigator()
            .service_worker()
            .register(&link("/sw.js"));
    };
    if document().ready_state() == "complete" {
        register();
    } else {
        on(&window(), "load", move |_: web_sys::Event| register());
    }
    on(&window(), "beforeinstallprompt", |event: web_sys::Event| {
        event.prevent_default();
        OFFER.with(|offer| *offer.borrow_mut() = Some(event));
        if let Some(button) = first("#install-app") {
            button.set_hidden(false);
        }
    });
    on(&window(), "appinstalled", |_: web_sys::Event| {
        if let Some(button) = first("#install-app") {
            button.set_hidden(true);
        }
    });
    let doc: web_sys::EventTarget = document().into();
    on(&doc, "click", |event: web_sys::MouseEvent| {
        if super::closest(&event, "#install-app").is_none() {
            return;
        }
        let Some(offer) = OFFER.with(|offer| offer.borrow_mut().take()) else {
            return;
        };
        // `BeforeInstallPromptEvent.prompt()`, which web-sys does not describe.
        if let Ok(prompt) = js_sys::Reflect::get(&offer, &JsValue::from_str("prompt"))
            && let Some(prompt) = wasm_bindgen::JsCast::dyn_ref::<js_sys::Function>(&prompt)
        {
            let _ = prompt.call0(&offer);
        }
        if let Some(button) = first("#install-app") {
            button.set_hidden(true);
        }
    });
}
