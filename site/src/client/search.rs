//! The documentation's search: the dialog `#search`, opened by the `/` key, Ctrl K (⌘ K
//! on a Mac) or anything marked `data-search-open` (a link that, without the client,
//! leads to the documentation's home).
//!
//! Every result is already in the page, written by the server (`li[data-title]
//! [data-words]`). The client shows those matching what is typed, best first (CSS
//! `order`), at most [`MAX`], and moves through them with the arrow keys; Enter opens
//! the one chosen. Before anything is typed, the pages are listed.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::JsCast;
use web_sys::{HtmlDialogElement, HtmlElement, HtmlInputElement};

use super::{all, closest, document, first, on};
use crate::search::{norm, score};

/// The most results shown at once.
const MAX: usize = 24;

struct Search {
    dialog: HtmlDialogElement,
    input: HtmlInputElement,
    /// Each result, with its title and words as the search compares them.
    items: Vec<(HtmlElement, String, String, bool)>,
    empty: Option<HtmlElement>,
    /// The results shown, in order (indices into `items`), and the one chosen.
    shown: RefCell<Vec<usize>>,
    active: Cell<Option<usize>>,
}

impl Search {
    fn open(&self) {
        if !self.dialog.open() {
            let _ = self.dialog.show_modal();
        }
        self.input.select();
        self.update();
    }

    /// Shows what matches the field, best first.
    fn update(&self) {
        let query = norm(&self.input.value());
        let mut found: Vec<(u32, usize)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, (_, title, words, page))| {
                let s = if query.is_empty() {
                    u32::from(*page)
                } else {
                    score(&query, title, words) * 2 + u32::from(*page)
                };
                (s > 0).then_some((s, i))
            })
            .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        found.truncate(MAX);
        for (item, ..) in &self.items {
            item.set_hidden(true);
        }
        for (rank, (_, i)) in found.iter().enumerate() {
            let item = &self.items[*i].0;
            item.set_hidden(false);
            let style = item.style();
            let _ = style.set_property("order", &rank.to_string());
            let _ = style.set_property("--r", &rank.min(12).to_string());
        }
        if let Some(empty) = &self.empty {
            empty.set_hidden(!found.is_empty());
        }
        *self.shown.borrow_mut() = found.iter().map(|(_, i)| *i).collect();
        self.choose(if found.is_empty() { None } else { Some(0) });
    }

    /// Marks the result at `rank` among those shown.
    fn choose(&self, rank: Option<usize>) {
        let shown = self.shown.borrow();
        let chosen = rank.and_then(|r| shown.get(r)).copied();
        for (i, (item, ..)) in self.items.iter().enumerate() {
            let on = Some(i) == chosen;
            let _ = item.class_list().toggle_with_force("on", on);
            if on {
                let options = web_sys::ScrollIntoViewOptions::new();
                options.set_block(web_sys::ScrollLogicalPosition::Nearest);
                item.scroll_into_view_with_scroll_into_view_options(&options);
            }
        }
        self.active.set(rank.filter(|r| *r < shown.len()));
    }

    fn step(&self, by: isize) {
        let len = self.shown.borrow().len() as isize;
        if len == 0 {
            return;
        }
        let at = self.active.get().map_or(-1, |r| r as isize);
        self.choose(Some((at + by).rem_euclid(len) as usize));
    }

    /// Opens the result chosen.
    fn follow(&self) {
        let Some(rank) = self.active.get() else {
            return;
        };
        let Some(i) = self.shown.borrow().get(rank).copied() else {
            return;
        };
        if let Some(link) = self.items[i].0.query_selector("a").ok().flatten() {
            self.dialog.close();
            link.unchecked_into::<HtmlElement>().click();
        }
    }
}

pub fn init() {
    let Some(dialog) = first("#search").and_then(|d| d.dyn_into::<HtmlDialogElement>().ok()) else {
        return;
    };
    let Some(input) = dialog
        .query_selector("input")
        .ok()
        .flatten()
        .and_then(|i| i.dyn_into::<HtmlInputElement>().ok())
    else {
        return;
    };
    let items = all("#search-results > li")
        .into_iter()
        .map(|item| {
            let title = item.get_attribute("data-title").unwrap_or_default();
            let words = item.get_attribute("data-words").unwrap_or_default();
            let page = item.class_list().contains("page");
            (item, title, words, page)
        })
        .collect();
    let search = Rc::new(Search {
        dialog: dialog.clone(),
        input: input.clone(),
        items,
        empty: first(".search-empty"),
        shown: RefCell::new(Vec::new()),
        active: Cell::new(None),
    });

    let doc: web_sys::EventTarget = document().into();
    let s = search.clone();
    on(&doc, "click", move |event: web_sys::MouseEvent| {
        if closest(&event, "[data-search-open]").is_some() {
            event.prevent_default();
            s.open();
        }
    });
    let s = search.clone();
    on(&doc, "keydown", move |event: web_sys::KeyboardEvent| {
        let typing = closest(&event, "input, textarea, select, [contenteditable]").is_some();
        let shortcut =
            (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("k");
        if shortcut || (event.key() == "/" && !typing && !event.ctrl_key() && !event.meta_key()) {
            event.prevent_default();
            s.open();
        }
    });
    let s = search.clone();
    on(&input, "input", move |_: web_sys::Event| s.update());
    let s = search.clone();
    on(&input, "keydown", move |event: web_sys::KeyboardEvent| {
        match event.key().as_str() {
            "ArrowDown" => s.step(1),
            "ArrowUp" => s.step(-1),
            "Enter" => s.follow(),
            _ => return,
        }
        event.prevent_default();
    });
    // A click on a result (it leads there), on the close button, or beside the box.
    let s = search.clone();
    on(&dialog, "click", move |event: web_sys::MouseEvent| {
        let outside = event
            .target()
            .is_some_and(|t| t == s.dialog.clone().unchecked_into::<web_sys::EventTarget>());
        if outside || closest(&event, ".search-close, #search-results a").is_some() {
            s.dialog.close();
        }
    });
}
