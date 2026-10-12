//! The pages of the documentation: the list of pages, a drawer on small screens
//! (`.side-open` on `<html>`), and "On this page", whose current section is marked
//! (`.on`) as the page is read, a current running down its wire to it (`--wire`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::{all, closest, document, first, frames, on, on_passive, root, window};

pub fn init() {
    sidebar();
    toc();
}

fn sidebar() {
    let toggles = all("[data-toggle-side]");
    if toggles.is_empty() {
        return;
    }
    let set = |open: bool| {
        let _ = root().class_list().toggle_with_force("side-open", open);
        for button in all(".side-toggle") {
            let _ = button.set_attribute("aria-expanded", if open { "true" } else { "false" });
        }
    };
    for toggle in toggles {
        on(&toggle, "click", move |_: web_sys::MouseEvent| {
            set(!root().class_list().contains("side-open"));
        });
    }
    let doc: web_sys::EventTarget = document().into();
    on(&doc, "click", move |event: web_sys::MouseEvent| {
        if closest(&event, ".doc-side a").is_some() {
            set(false);
        }
    });
    on(&doc, "keydown", move |event: web_sys::KeyboardEvent| {
        if event.key() == "Escape" {
            set(false);
        }
    });
}

/// The section being read: the last heading that has passed the top third of the screen
/// (or the first one), worked out on each frame that scrolls, so that a jump to a
/// heading marks it too.
fn toc() {
    let links = all(".doc-toc a[data-toc]");
    let Some(list) = first(".toc-list") else {
        return;
    };
    let headings = all(".doc-body h2[id], .doc-body h3[id]");
    if links.is_empty() || headings.is_empty() {
        return;
    }
    let current = Rc::new(RefCell::new(String::new()));
    let update = move || {
        let line = window()
            .inner_height()
            .ok()
            .and_then(|h| h.as_f64())
            .unwrap_or(800.0)
            * 0.3;
        let id = headings
            .iter()
            .take_while(|h| h.get_bounding_client_rect().top() < line)
            .last()
            .unwrap_or(&headings[0])
            .id();
        if *current.borrow() == id {
            return;
        }
        for link in &links {
            let on = link.get_attribute("data-toc").as_deref() == Some(id.as_str());
            let _ = link.class_list().toggle_with_force("on", on);
            if on {
                let y = link.offset_top() + link.offset_height() / 2;
                let _ = list.style().set_property("--wire", &format!("{y}px"));
            }
        }
        *current.borrow_mut() = id;
    };
    update();
    let update = Rc::new(update);
    let ticking = Rc::new(Cell::new(false));
    on_passive(&window(), "scroll", move |_: web_sys::Event| {
        if ticking.replace(true) {
            return;
        }
        let (update, ticking) = (update.clone(), ticking.clone());
        frames(move |_| {
            ticking.set(false);
            update();
            false
        });
    });
}
