//! The motion CSS cannot do alone, and the small conveniences of the page. The hero's
//! entrance, the counters, the marquee and every hover effect are CSS (static/app.css);
//! this adds what needs the scroll position, the pointer or the elements in view:
//!
//! 1. the header that tightens once the page scrolls, the reading progress bar, the
//!    hero's parallax and the footer's wordmark rising (`--py`, `--seen`);
//! 2. the menu link of the section being read (`.on`);
//! 3. reveals: elements marked `data-reveal` get `.in` when they scroll into view
//!    (`words`: a heading's words rise in turn, `rows`: a table's rows, `scale`: grows);
//! 4. on a fine pointer: the spotlight on cards (`.spot`, `--mx`, `--my`), the light
//!    under the pointer (`.glow`), buttons drawn towards it (`.magnetic`);
//! 5. the menu on small screens (`.menu-open`), Escape closing it and the picker;
//! 6. copy buttons (`data-copy`).
//!
//! When the system asks for less motion, nothing moves: reveals show at once.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    HtmlElement, IntersectionObserver, IntersectionObserverEntry, IntersectionObserverInit,
};

use super::{
    all, closest, document, first, frames, later, media, on, on_passive, reduced_motion, root,
    window,
};

pub fn init() {
    scroll_state();
    current_section();
    reveals();
    if media("(pointer: fine)") && !reduced_motion() {
        pointer();
        magnetic();
    }
    menu();
    copy_buttons();
    // Back from another page with the browser's Back button: shown again, not faded.
    on(&window(), "pageshow", |_: web_sys::Event| {
        let _ = root().class_list().remove_1("leaving");
    });
}

/// The window's height in pixels.
fn viewport_height() -> f64 {
    window()
        .inner_height()
        .ok()
        .and_then(|h| h.as_f64())
        .unwrap_or(800.0)
}

/// Header state, progress bar, parallax and wordmark, once per frame while scrolling.
fn scroll_state() {
    let header = first(".top");
    let progress = first(".progress");
    let hero = first(".hero");
    let wordmark = first(".wordmark");
    let still = reduced_motion();
    let update = move || {
        let y = window().scroll_y().unwrap_or(0.0);
        let height = viewport_height();
        if let Some(h) = &header {
            let _ = h.class_list().toggle_with_force("scrolled", y > 8.0);
        }
        if let Some(p) = &progress {
            let max = f64::from(root().scroll_height()) - height;
            let ratio = if max > 0.0 {
                (y / max).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let _ = p
                .style()
                .set_property("transform", &format!("scaleX({ratio:.4})"));
        }
        if still {
            return;
        }
        // The hero's light drifts slower than the page.
        if let Some(h) = &hero
            && y < height * 1.2
        {
            let _ = h
                .style()
                .set_property("--py", &format!("{:.1}px", y * 0.25));
        }
        // The wordmark rises as the end of the page comes.
        if let Some(w) = &wordmark {
            let top = w.get_bounding_client_rect().top();
            let seen = 1.0 - (top / height).clamp(0.0, 1.0);
            let _ = w.style().set_property("--seen", &format!("{seen:.3}"));
        }
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

/// Watches the elements matching `selector`, calling `f` for each change of visibility.
fn observe(
    selector: &str,
    options: &IntersectionObserverInit,
    f: impl Fn(&IntersectionObserverEntry, &IntersectionObserver) + 'static,
) {
    let callback = Closure::<dyn Fn(js_sys::Array, IntersectionObserver)>::new(
        move |entries: js_sys::Array, io: IntersectionObserver| {
            for entry in entries.iter() {
                if let Ok(entry) = entry.dyn_into::<IntersectionObserverEntry>() {
                    f(&entry, &io);
                }
            }
        },
    );
    if let Ok(io) =
        IntersectionObserver::new_with_options(callback.as_ref().unchecked_ref(), options)
    {
        for el in all(selector) {
            io.observe(&el);
        }
    }
    callback.forget();
}

/// The menu link of the section in the middle of the screen gets `.on`.
fn current_section() {
    let links = all("[data-spy]");
    if links.is_empty() {
        return;
    }
    let selector = links
        .iter()
        .filter_map(|l| l.get_attribute("data-spy"))
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(",");
    let options = IntersectionObserverInit::new();
    options.set_root_margin("-45% 0px -50% 0px");
    observe(&selector, &options, move |entry, _| {
        if !entry.is_intersecting() {
            return;
        }
        let id = entry.target().id();
        for link in &links {
            let on = link.get_attribute("data-spy").as_deref() == Some(id.as_str());
            let _ = link.class_list().toggle_with_force("on", on);
        }
    });
}

/// `data-reveal` elements get `.in` as they come into view.
fn reveals() {
    // A table's rows get their order, for an entrance in turn (capped: a long table stays
    // quick).
    for (i, row) in all("[data-reveal=\"rows\"] tbody tr").iter().enumerate() {
        let _ = row.style().set_property("--i", &i.min(16).to_string());
    }
    if reduced_motion() {
        for el in all("[data-reveal]") {
            let _ = el.class_list().add_1("in");
        }
        return;
    }
    let options = IntersectionObserverInit::new();
    options.set_root_margin("0px 0px -8% 0px");
    options.set_threshold(&JsValue::from_f64(0.08));
    observe("[data-reveal]", &options, |entry, io| {
        if entry.is_intersecting() {
            let target = entry.target();
            let _ = target.class_list().add_1("in");
            io.unobserve(&target);
        }
    });
}

/// The spotlight that follows the pointer on cards, and the light under it.
fn pointer() {
    let glow = first(".glow");
    on_passive(
        &window(),
        "pointermove",
        move |event: web_sys::PointerEvent| {
            let (x, y) = (f64::from(event.client_x()), f64::from(event.client_y()));
            if let Some(g) = &glow {
                let _ = g.style().set_property(
                    "transform",
                    &format!("translate({}px,{}px)", x - 300.0, y - 300.0),
                );
                let _ = g.class_list().add_1("on");
            }
            if let Some(card) = closest(&event, ".spot") {
                let r = card.get_bounding_client_rect();
                let style = card.style();
                let _ = style.set_property("--mx", &format!("{:.0}px", x - r.left()));
                let _ = style.set_property("--my", &format!("{:.0}px", y - r.top()));
            }
        },
    );
}

/// Buttons drawn a little towards the pointer.
fn magnetic() {
    for button in all(".magnetic") {
        let target = button.clone();
        on(
            &button,
            "pointermove",
            move |event: web_sys::PointerEvent| {
                let r = target.get_bounding_client_rect();
                let x = (f64::from(event.client_x()) - r.left() - r.width() / 2.0) * 0.25;
                let y = (f64::from(event.client_y()) - r.top() - r.height() / 2.0) * 0.35;
                let _ = target
                    .style()
                    .set_property("transform", &format!("translate({x:.1}px,{y:.1}px)"));
            },
        );
        let target = button.clone();
        on(&button, "pointerleave", move |_: web_sys::PointerEvent| {
            let _ = target.style().remove_property("transform");
        });
    }
}

/// The menu on small screens: the button opens and closes it, a link closes it, Escape
/// closes it and the language picker.
fn menu() {
    let Some(toggle) = first(".menu-toggle") else {
        return;
    };
    let set = Rc::new(move |toggle: &HtmlElement, open: bool| {
        let _ = root().class_list().toggle_with_force("menu-open", open);
        let _ = toggle.set_attribute("aria-expanded", if open { "true" } else { "false" });
        let label = toggle.get_attribute(if open { "data-close" } else { "data-open" });
        let _ = toggle.set_attribute("aria-label", &label.unwrap_or_default());
    });
    let (button, change) = (toggle.clone(), set.clone());
    on(&toggle, "click", move |_: web_sys::MouseEvent| {
        let open = !root().class_list().contains("menu-open");
        change(&button, open);
    });
    if let Some(menu) = first("#menu") {
        let (button, change) = (toggle.clone(), set.clone());
        on(&menu, "click", move |event: web_sys::MouseEvent| {
            if closest(&event, "a").is_some() {
                change(&button, false);
            }
        });
    }
    let doc: web_sys::EventTarget = document().into();
    on(&doc, "keydown", move |event: web_sys::KeyboardEvent| {
        if event.key() != "Escape" {
            return;
        }
        set(&toggle, false);
        if let Some(picker) = first(".lang-picker[open]") {
            let _ = picker.remove_attribute("open");
        }
    });
}

/// Buttons with `data-copy` put its text on the clipboard, and say so for a moment
/// (`data-done`).
fn copy_buttons() {
    let doc: web_sys::EventTarget = document().into();
    on(&doc, "click", |event: web_sys::MouseEvent| {
        let Some(button) = closest(&event, "[data-copy]") else {
            return;
        };
        let text = button.get_attribute("data-copy").unwrap_or_default();
        let promise = window().navigator().clipboard().write_text(&text);
        wasm_bindgen_futures::spawn_local(async move {
            if wasm_bindgen_futures::JsFuture::from(promise).await.is_err() {
                return;
            }
            let label = button.text_content().unwrap_or_default();
            let done = button
                .get_attribute("data-done")
                .unwrap_or_else(|| label.clone());
            button.set_text_content(Some(&done));
            let _ = button.class_list().add_1("done");
            later(1600, move || {
                button.set_text_content(Some(&label));
                let _ = button.class_list().remove_1("done");
            });
        });
    });
}
