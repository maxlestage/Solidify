//! The motion CSS cannot do alone, and the small conveniences of the page. The preloader,
//! the hero's entrance and scene, the counters, the marquee and every hover effect are
//! CSS (static/app.css); this adds what needs the scroll position, the pointer, the
//! elements in view or the visit:
//!
//! 1. the preloader, played once per visit (`.intro-seen` skips it, a click too);
//! 2. the header that tightens once the page scrolls, the reading progress bar and its
//!    spark, the ring of the button back to the top (`--p`, `.show`), the hero's parallax (`--py`, `data-speed`), the current along the steps
//!    (`--p`, `.lit`) and the footer's wordmark rising (`--seen`);
//! 3. the menu link of the section being read (`.on`);
//! 4. reveals: elements marked `data-reveal` get `.in` when they scroll into view
//!    (`words`: a heading's words rise in turn, `rows`: a table's rows, `scale`: grows,
//!    `neon`: letters light up);
//! 5. on a fine pointer: the cursor (a spark and its ring, `data-cursor` labels), the
//!    spotlight on cards (`.spot`, `--mx`, `--my`), cards tilting in 3D (`data-tilt`),
//!    the hero's scene leaning towards the pointer (`--hx`, `--hy`), the light under the
//!    pointer (`.glow`), buttons drawn towards it (`.magnetic`);
//! 6. sparks where the page is clicked or tapped, a discharge across the playground at
//!    every translation (`.surge`);
//! 7. the menu on small screens (`.menu-open`), Escape closing it and the picker;
//! 8. copy buttons (`data-copy`).
//!
//! When the system asks for less motion, nothing moves: no preloader, reveals show at
//! once, the steps are lit.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, IntersectionObserverInit};

use super::{
    all, closest, document, first, frames, later, media, observe, on, on_passive, reduced_motion,
    root, window,
};

pub fn init() {
    intro();
    scroll_state();
    current_section();
    reveals();
    if media("(pointer: fine)") && !reduced_motion() {
        let _ = root().class_list().add_1("has-cursor");
        cursor();
        pointer();
        magnetic();
        tilt();
        hero_mouse();
    }
    if !reduced_motion() {
        sparks();
        surge();
    }
    menu();
    copy_buttons();
    // Back from another page with the browser's Back button: shown again, not faded.
    on(&window(), "pageshow", |_: web_sys::Event| {
        let _ = root().class_list().remove_1("leaving");
    });
}

/// The preloader plays once per visit: on the next pages it is not shown (`intro-seen`).
/// A click lifts it at once (`intro-skip`).
fn intro() {
    let Some(loader) = first(".loader") else {
        return;
    };
    const SEEN: &str = "solidify-intro";
    let storage = window().session_storage().ok().flatten();
    let seen = storage
        .as_ref()
        .and_then(|s| s.get_item(SEEN).ok().flatten())
        .is_some();
    if seen || reduced_motion() {
        let _ = root().class_list().add_1("intro-seen");
        return;
    }
    if let Some(storage) = storage {
        let _ = storage.set_item(SEEN, "1");
    }
    on(&loader, "click", |_: web_sys::MouseEvent| {
        let _ = root().class_list().add_1("intro-skip");
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

/// Header state, progress bar, parallax, the current along the steps and the wordmark,
/// once per frame while scrolling.
fn scroll_state() {
    let header = first(".top");
    let progress = first(".progress");
    let spark = first(".progress-head");
    let to_top = first(".to-top-ring");
    let hero = first(".hero");
    let steps = first(".steps");
    let step_items = all(".steps > li");
    let wordmark = first(".wordmark");
    let still = reduced_motion();
    let parallax: Vec<(HtmlElement, f64)> = if still {
        Vec::new()
    } else {
        all("[data-speed]")
            .into_iter()
            .map(|el| {
                let speed = el
                    .get_attribute("data-speed")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                (el, speed)
            })
            .collect()
    };
    let update = move || {
        let y = window().scroll_y().unwrap_or(0.0);
        let height = viewport_height();
        if let Some(h) = &header {
            let _ = h.class_list().toggle_with_force("scrolled", y > 8.0);
        }
        let max = f64::from(root().scroll_height()) - height;
        let ratio = if max > 0.0 {
            (y / max).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // Back to the top: shown once the page has scrolled, its ring filled as far.
        if let Some(button) = &to_top {
            let _ = button
                .class_list()
                .toggle_with_force("show", y > height * 0.8);
            let _ = button.style().set_property("--p", &format!("{ratio:.4}"));
        }
        if let Some(p) = &progress {
            let _ = p
                .style()
                .set_property("transform", &format!("scaleX({ratio:.4})"));
            if let Some(s) = &spark {
                let width = f64::from(root().client_width());
                let style = s.style();
                let _ =
                    style.set_property("transform", &format!("translateX({:.1}px)", ratio * width));
                let _ = style.set_property("opacity", if ratio > 0.002 { "1" } else { "0" });
            }
        }
        // The current reaches the steps as they come up the screen; all lit when nothing
        // may move.
        if let Some(s) = &steps {
            let r = s.get_bounding_client_rect();
            let p = if still {
                1.0
            } else {
                ((height * 0.8 - r.top()) / r.height().max(height * 0.45)).clamp(0.0, 1.0)
            };
            let _ = s.style().set_property("--p", &format!("{p:.4}"));
            let n = step_items.len().max(1) as f64;
            for (i, item) in step_items.iter().enumerate() {
                let _ = item
                    .class_list()
                    .toggle_with_force("lit", p >= i as f64 / n + 0.04);
            }
        }
        if still {
            return;
        }
        for (el, speed) in &parallax {
            let _ = el
                .style()
                .set_property("transform", &format!("translate3d(0,{:.1}px,0)", y * speed));
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

/// A spark follows the pointer, a ring catches up with it; the ring grows over links
/// and buttons and shows the `data-cursor` label of what it points at.
fn cursor() {
    let (Some(dot), Some(ring)) = (first(".cursor-dot"), first(".cursor-ring")) else {
        return;
    };
    // Hidden until the pointer first moves.
    let _ = root().class_list().add_1("cursor-out");
    let target = Rc::new(Cell::new((-100.0f64, -100.0f64)));
    {
        let (target, dot, ring) = (target.clone(), dot.clone(), ring.clone());
        on_passive(
            &window(),
            "pointermove",
            move |event: web_sys::PointerEvent| {
                let (x, y) = (f64::from(event.client_x()), f64::from(event.client_y()));
                target.set((x, y));
                if root().class_list().contains("cursor-out") {
                    let _ = root().class_list().remove_1("cursor-out");
                }
                let _ = dot
                    .style()
                    .set_property("transform", &format!("translate3d({x}px,{y}px,0)"));
                let hovered = closest(&event, "a, button, summary, [data-cursor]");
                let label = hovered
                    .as_ref()
                    .and_then(|el| el.get_attribute("data-cursor"))
                    .unwrap_or_default();
                let typing = closest(&event, "textarea, input").is_some();
                let list = ring.class_list();
                let _ = list.toggle_with_force("hover", hovered.is_some());
                let _ = list.toggle_with_force("label", !label.is_empty());
                let _ = root().class_list().toggle_with_force("cursor-text", typing);
                if let Some(span) = ring.query_selector("span").ok().flatten()
                    && span.text_content().unwrap_or_default() != label
                {
                    span.set_text_content(Some(&label));
                }
            },
        );
    }
    for (event, down) in [("pointerdown", true), ("pointerup", false)] {
        let ring = ring.clone();
        on(&window(), event, move |_: web_sys::PointerEvent| {
            let _ = ring.class_list().toggle_with_force("down", down);
        });
    }
    // Off the page: hidden until it comes back.
    let doc: web_sys::EventTarget = document().into();
    on(&doc, "pointerleave", |_: web_sys::PointerEvent| {
        let _ = root().class_list().add_1("cursor-out");
    });
    on(&doc, "pointerenter", |_: web_sys::PointerEvent| {
        let _ = root().class_list().remove_1("cursor-out");
    });
    let mut pos = (-100.0f64, -100.0f64);
    frames(move |_| {
        let (tx, ty) = target.get();
        if (tx - pos.0).abs() > 0.1 || (ty - pos.1).abs() > 0.1 {
            pos.0 += (tx - pos.0) * 0.18;
            pos.1 += (ty - pos.1) * 0.18;
            let _ = ring.style().set_property(
                "transform",
                &format!("translate3d({:.1}px,{:.1}px,0)", pos.0, pos.1),
            );
        }
        true
    });
}

/// Cards lean in 3D towards the pointer (`data-tilt`: the most they turn, in degrees).
fn tilt() {
    for card in all("[data-tilt]") {
        let max: f64 = card
            .get_attribute("data-tilt")
            .and_then(|v| v.parse().ok())
            .unwrap_or(8.0);
        let target = card.clone();
        on(&card, "pointermove", move |event: web_sys::PointerEvent| {
            let r = target.get_bounding_client_rect();
            let x = (f64::from(event.client_x()) - r.left()) / r.width().max(1.0);
            let y = (f64::from(event.client_y()) - r.top()) / r.height().max(1.0);
            let style = target.style();
            let _ = style.set_property("--ry", &format!("{:.2}deg", (x - 0.5) * max));
            let _ = style.set_property("--rx", &format!("{:.2}deg", (0.5 - y) * max));
        });
        let target = card.clone();
        on(&card, "pointerleave", move |_: web_sys::PointerEvent| {
            let style = target.style();
            let _ = style.set_property("--rx", "0deg");
            let _ = style.set_property("--ry", "0deg");
        });
    }
}

/// The hero's scene leans towards the pointer, with some inertia (`--hx`, `--hy`, from
/// -0.5 to 0.5).
fn hero_mouse() {
    let Some(hero) = first(".hero") else {
        return;
    };
    let mouse = Rc::new(Cell::new((0.0f64, 0.0f64)));
    let eased = Rc::new(Cell::new((0.0f64, 0.0f64)));
    let running = Rc::new(Cell::new(false));
    on_passive(
        &window(),
        "pointermove",
        move |event: web_sys::PointerEvent| {
            let width = window()
                .inner_width()
                .ok()
                .and_then(|w| w.as_f64())
                .unwrap_or(1.0);
            mouse.set((
                f64::from(event.client_x()) / width.max(1.0) - 0.5,
                f64::from(event.client_y()) / viewport_height().max(1.0) - 0.5,
            ));
            if running.replace(true) {
                return;
            }
            let (hero, mouse, eased, running) =
                (hero.clone(), mouse.clone(), eased.clone(), running.clone());
            frames(move |_| {
                let (mx, my) = mouse.get();
                let (ex, ey) = eased.get();
                let next = (ex + (mx - ex) * 0.06, ey + (my - ey) * 0.06);
                eased.set(next);
                let style = hero.style();
                let _ = style.set_property("--hx", &format!("{:.4}", next.0));
                let _ = style.set_property("--hy", &format!("{:.4}", next.1));
                let moving = (mx - next.0).abs() > 0.001 || (my - next.1).abs() > 0.001;
                running.set(moving);
                moving
            });
        },
    );
}

/// Sparks fly where the page is clicked or tapped: eight rays of random length, gone
/// in under a second.
fn sparks() {
    let Some(body) = document().body() else {
        return;
    };
    on_passive(
        &window(),
        "pointerdown",
        move |event: web_sys::PointerEvent| {
            let Ok(zap) = document().create_element("div") else {
                return;
            };
            let _ = zap.set_attribute("class", "sparkburst");
            let _ = zap.set_attribute("aria-hidden", "true");
            let _ = zap.set_attribute(
                "style",
                &format!("left:{}px;top:{}px", event.client_x(), event.client_y()),
            );
            for n in 0..8 {
                let Ok(ray) = document().create_element("i") else {
                    continue;
                };
                let angle = f64::from(n) * 45.0 + (js_sys::Math::random() - 0.5) * 30.0;
                let length = 22.0 + js_sys::Math::random() * 26.0;
                let _ = ray.set_attribute("style", &format!("--a:{angle:.0}deg;--l:{length:.0}px"));
                let _ = zap.append_child(&ray);
            }
            let _ = body.append_child(&zap);
            later(700, move || zap.remove());
        },
    );
}

/// Every translation in the playground (a keystroke, an example picked) sends a
/// discharge from the TypeScript to the Rust (`.surge`, restarted each time).
fn surge() {
    let fire = || {
        let Some(play) = first(".play") else {
            return;
        };
        let list = play.class_list();
        let _ = list.remove_1("surge");
        // Read the layout, so that the animation starts again.
        let _ = play.offset_width();
        let _ = list.add_1("surge");
    };
    let doc: web_sys::EventTarget = document().into();
    on(&doc, "input", move |event: web_sys::Event| {
        if closest(&event, ".play textarea").is_some() {
            fire();
        }
    });
    on(&doc, "click", move |event: web_sys::MouseEvent| {
        if closest(&event, ".play .examples button").is_some() {
            fire();
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
