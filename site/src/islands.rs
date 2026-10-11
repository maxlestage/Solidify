//! The interactive part of the site: the playground, rendered on the server with its first
//! example and, compiled to WebAssembly, hydrated in the browser where it runs Solidify on
//! every keystroke.

use active::prelude::*;

use crate::examples::EXAMPLES;
use crate::highlight::{Language, escape_into, highlight};
use crate::i18n::t;
use crate::routes::Lang;

/// Every island, registered under the name used by [`island`].
pub const ISLANDS: [(&str, active::IslandFn); 1] = [("playground", playground)];

/// Runs Solidify on a source; the error is formatted with its line and column.
pub fn translate(src: &str) -> Result<String, String> {
    let options = solidify::Options {
        file_name: None,
        no_header: true,
    };
    solidify::transpile(src, &options).map_err(|e| e.pretty(src))
}

/// The HTML shown in the output pane.
fn output_html(result: &Result<String, String>) -> String {
    match result {
        Ok(rust) => highlight(rust, Language::Rust),
        Err(message) => {
            let mut out = String::new();
            escape_into(&mut out, message);
            out
        }
    }
}

/// Milliseconds since the page loaded, in the browser; `None` on the server.
fn now() -> Option<f64> {
    #[cfg(target_arch = "wasm32")]
    return web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now());
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// The live playground. Its props are the language code.
pub fn playground(props: &str) -> Node {
    let lang = Lang::parse(props).unwrap_or(Lang::En);
    let d = &t(lang).play;
    let first = &EXAMPLES[0];
    let source = use_state(first.source.to_string());
    let result = use_state(translate(first.source));
    let current = use_state(0usize);
    // Where the output was computed: the server first, then Rust/wasm in the browser.
    let took = use_state(None::<f64>);
    let copied = use_state(false);

    let run = move |src: String| {
        let start = now();
        let r = translate(&src);
        if let (Some(start), Some(end)) = (start, now()) {
            took.set(Some(end - start));
        }
        result.set(r);
        source.set(src);
    };

    div()
        .class("play")
        .child(
            div()
                .class("play-head")
                .child(
                    div()
                        .class("examples")
                        .attr("role", "group")
                        .attr("aria-label", d.examples)
                        .children(EXAMPLES.iter().enumerate().map(|(i, ex)| {
                            button()
                                .attr("type", "button")
                                .class_if("on", move || current.get() == i)
                                .on_click(move |_| {
                                    current.set(i);
                                    copied.set(false);
                                    run(ex.source.to_string());
                                })
                                .text(ex.title(lang))
                        })),
                )
                .child(
                    p().class("took")
                        .attr("aria-live", "polite")
                        .child(span().class("tag").text("Rust → wasm"))
                        .text(" ")
                        .text_dyn(move || match took.get() {
                            None => d.server.to_string(),
                            Some(ms) if ms < 0.05 => format!("{} · <0.05 ms", d.browser),
                            Some(ms) => format!("{} · {ms:.2} ms", d.browser),
                        }),
                ),
        )
        .child(
            div()
                .class("panes")
                .child(
                    div()
                        .class("pane")
                        .child(
                            div()
                                .class("pane-head")
                                .child(span().class("lang-tag ts").text(d.ts))
                                .child(span().class("count").text_dyn(move || {
                                    format!("{} {}", source.with(|s| s.lines().count()), d.lines)
                                })),
                        )
                        .child(
                            textarea()
                                .attr("spellcheck", "false")
                                .attr("autocapitalize", "off")
                                .attr("autocomplete", "off")
                                .attr("aria-label", d.ts)
                                .attr_dyn("value", move || source.get())
                                .on_input(move |e| {
                                    copied.set(false);
                                    run(e.value());
                                })
                                .text(first.source),
                        ),
                )
                .child(
                    div()
                        .class("pane")
                        .class_if("failed", move || result.with(|r| r.is_err()))
                        .child(
                            div()
                                .class("pane-head")
                                .child(span().class("lang-tag rs").text(d.rust))
                                .child(span().class("count").text_dyn(move || {
                                    result.with(|r| match r {
                                        Ok(rust) => format!("{} {}", rust.lines().count(), d.lines),
                                        Err(_) => d.error.to_string(),
                                    })
                                }))
                                .child(
                                    button()
                                        .class("copy")
                                        .attr("type", "button")
                                        .on_click(move |_| {
                                            let text =
                                                result.with(|r| r.clone().unwrap_or_default());
                                            #[cfg(target_arch = "wasm32")]
                                            crate::client::copy(&text);
                                            let _ = text;
                                            copied.set(true);
                                        })
                                        .text_dyn(move || {
                                            if copied.get() {
                                                d.copied.to_string()
                                            } else {
                                                d.copy.to_string()
                                            }
                                        }),
                                ),
                        )
                        .child(
                            pre()
                                .attr("aria-live", "polite")
                                .child(code().html_dyn(move || result.with(output_html))),
                        ),
                ),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_playground_renders_its_first_example_on_the_server() {
        let html = active::island("playground", "en", playground).render();
        assert!(html.contains("fib</span>(n: <span class=\"t\">i64</span>)"));
        assert!(html.contains("translated on the server"));
        for ex in EXAMPLES {
            assert!(
                translate(ex.source).is_ok(),
                "example {} must translate",
                ex.id
            );
        }
    }
}
