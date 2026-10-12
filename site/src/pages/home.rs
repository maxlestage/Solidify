//! The home page: the preloader, the hero and its scene, the marquee, the playground, how
//! it works, what translates, install.

use active::prelude::*;

use super::{arrow, bolt, emphasized, kicker, rich, split};
use crate::highlight::{Language, highlight, shell};
use crate::i18n::t;
use crate::islands::playground;
use crate::routes::Lang;

const INSTALL_CLI: &str = "npx solidify-rs main.ts -o app.rs     # follows the imports of main.ts, one Rust file out\n\
npx solidify-rs < app.ts > app.rs     # or through the standard input\n\
rustc --edition 2024 app.rs && ./app";

const INSTALL_CHECK: &str =
    "npx solidify-rs --check app.ts        # errors with line and column, nothing written";

const INSTALL_LIB: &str = "let options = solidify::Options::default();\n\
let rust: String = solidify::transpile(typescript, &options)?;\n\
// A program of several files, from its entry file:\n\
let rust: String = solidify::transpile_file(Path::new(\"main.ts\"), &options)?;";

const CARGO_LIB: &str =
    "[dependencies]\nsolidify = { git = \"https://github.com/maxlestage/Solidify\" }";

/// The `<main>` of the home page.
pub fn content(lang: Lang) -> Element {
    let d = t(lang);
    main()
        .id("main")
        .child(hero(lang))
        .child(marquee())
        .child(
            section()
                .id("playground")
                .class("block")
                .child(kicker(1, d.nav.playground))
                .child(split(h2(), d.play.title))
                .child(p().class("lead").attr("data-reveal", "").text(d.play.lead))
                .child(
                    div()
                        .class("play-wrap")
                        .attr("data-reveal", "scale")
                        .child(island("playground", lang.code(), playground)),
                ),
        )
        .child(how(lang))
        .child(supported(lang))
        .child(install(lang))
}

/// The preloader, CSS only: a bolt draws itself and lights up, the letters flicker on
/// like a neon sign, the battery charges to 100, a flash, then a yellow curtain lifts.
/// The client skips it on the next pages of the visit (`intro-seen`), a click skips it.
pub fn loader(lang: Lang) -> Element {
    div()
        .class("loader")
        .attr("aria-hidden", "true")
        .child(
            div()
                .class("loader-inner")
                .child(
                    div()
                        .class("loader-bolt")
                        .child(span().class("loader-halo"))
                        .child(bolt()),
                )
                .child(
                    div().class("loader-word").children(
                        "SOLIDIFY"
                            .chars()
                            .enumerate()
                            .map(|(n, c)| span().style(format!("--i:{n}")).text(c.to_string())),
                    ),
                )
                .child(div().class("loader-battery").child(span()))
                .child(
                    p().class("loader-label").text(t(lang).hero.charging).child(
                        span()
                            .class("loader-dots")
                            .children((0..3).map(|_| el("i").text("."))),
                    ),
                ),
        )
        .child(div().class("loader-volts"))
        .child(div().class("loader-flash"))
        .child(div().class("loader-curtain"))
}

fn hero(lang: Lang) -> Element {
    let d = t(lang);
    let copy = div()
        .class("hero-copy")
        .child(
            p().class("eyebrow intro")
                .attr("style", "--d:0")
                .child(span().class("dot").attr("aria-hidden", "true"))
                .text(d.hero.eyebrow),
        )
        .child(emphasized(h1(), d.hero.title))
        .child(
            p().class("lead intro")
                .attr("style", "--d:6")
                .text(d.hero.lead),
        )
        .child(
            div()
                .class("cta intro")
                .attr("style", "--d:8")
                .child(
                    a().class("button primary magnetic")
                        .href("#playground")
                        .attr("data-cursor", "Go")
                        .child(span().text(d.hero.cta))
                        .child(arrow()),
                )
                .child(
                    a().class("button magnetic")
                        .href("#install")
                        .child(span().text(d.hero.cta2))
                        .child(arrow()),
                ),
        )
        .child(ul().class("facts intro").attr("style", "--d:10").children(
            d.hero.facts.iter().map(|(value, label)| {
                // A whole number counts up as the page opens (CSS only, see
                // `.facts strong[data-count]`); the figure stays in the text.
                let figure = if value.parse::<u32>().is_ok() {
                    strong()
                        .attr("data-count", *value)
                        .attr("style", format!("--to:{value}"))
                        .child(span().class("value").text(*value))
                } else {
                    strong().text(*value)
                };
                li().child(figure).child(span().text(*label))
            }),
        ));
    section()
        .class("hero")
        .child(aurora())
        .child(circuit())
        .child(floaters())
        .child(copy)
        .child(
            div()
                .class("hero-visual intro")
                .attr("style", "--d:3")
                .attr("role", "img")
                .attr("aria-label", d.hero.scene)
                .child(scene()),
        )
        .child(
            a().class("scroll-cue intro")
                .attr("style", "--d:12")
                .href("#playground")
                .attr("aria-label", d.nav.playground)
                .child(span().attr("aria-hidden", "true")),
        )
}

/// The scene beside the title, CSS only: a dashed TypeScript cube turns; the charge
/// crackles around it, lightning strikes, and it turns into a solid Rust cube, sparks
/// orbiting it; then it fades back, and again. It leans towards the pointer (`--hx`,
/// `--hy`, set by the client).
fn scene() -> Element {
    // Each face: its dashed edge (seen through the cube), the TypeScript label (only
    // from the front), and the solid Rust side.
    let face = |side: &'static str| {
        div()
            .class("face")
            .class(side)
            .child(span().class("edge"))
            .child(span().class("wire").text(".ts"))
            .child(span().class("solid").text(".rs"))
    };
    // Jagged paths in a 300 × 300 box (the cube is 100 wide, its top near y = 100): the
    // strike in three shapes shown in turn, coming from above the box, its branches, and
    // the arcs of the charge.
    const STRIKE: [&str; 3] = [
        "M150 -64 L161 -42 L149 -26 L156 -8 L141 22 L160 34 L136 58 L155 66 L144 88 L150 104",
        "M140 -64 L152 -44 L139 -26 L146 -8 L158 18 L138 36 L161 52 L140 70 L156 84 L150 104",
        "M158 -64 L145 -40 L157 -22 L150 -8 L136 20 L156 32 L144 54 L164 64 L146 86 L150 104",
    ];
    const BRANCHES: [&str; 2] = [
        "M141 22 L120 30 L112 48 L96 54",
        "M161 52 L182 58 L188 76 L206 84",
    ];
    const CHARGE: [&str; 4] = [
        "M62 214 L80 198 L72 186 L94 176 L88 162 L104 154",
        "M238 214 L220 196 L230 186 L206 174 L214 160 L196 154",
        "M70 112 L86 120 L80 132 L98 138",
        "M232 112 L214 122 L222 132 L202 140",
    ];
    let arc = |d: &'static str, class: String| {
        el("path")
            .attr("d", d)
            .attr("pathLength", "100")
            .attr("class", class)
    };
    let arcs = svg()
        .class("scene-arcs")
        .attr("viewBox", "0 0 300 300")
        .attr("aria-hidden", "true")
        .children(
            STRIKE
                .iter()
                .enumerate()
                .map(|(n, d)| arc(d, format!("strike s{n}"))),
        )
        .children(
            BRANCHES
                .iter()
                .enumerate()
                .map(|(n, d)| arc(d, format!("branch b{n}"))),
        )
        .children(
            CHARGE
                .iter()
                .enumerate()
                .map(|(n, d)| arc(d, format!("charge c{n}"))),
        );
    div()
        .class("scene")
        .attr("aria-hidden", "true")
        .child(div().class("scene-halo"))
        .child(
            div().class("scene-tilt").child(
                div().class("scene-rig").child(
                    div().class("cube").children(
                        [
                            "f-front", "f-back", "f-right", "f-left", "f-top", "f-bottom",
                        ]
                        .map(face),
                    ),
                ),
            ),
        )
        .child(arcs)
        .child(
            div()
                .class("scene-sparks")
                .children((0..6).map(|n| span().style(format!("--i:{n}")))),
        )
        .child(div().class("scene-flash"))
        .child(div().class("scene-floor"))
        .child(
            ul().class("scene-labels")
                .child(li().class("ts").child(el("i")).text("TypeScript"))
                .child(li().class("to").child(bolt()))
                .child(li().class("rs").child(el("i")).text("Rust")),
        )
}

/// Traces of a circuit behind the hero, a pulse of current running along each.
fn circuit() -> Element {
    // In a 1200 × 700 box; each ends on a pad.
    const TRACES: [(&str, (u32, u32)); 8] = [
        ("M-10 120 H220 L280 180 H500", (500, 180)),
        ("M-10 430 H150 L230 350 H400 L440 310", (440, 310)),
        ("M1210 90 H990 L930 150 H780", (780, 150)),
        ("M1210 370 H1050 L990 430 H880", (880, 430)),
        ("M1210 620 H1020 L960 560 H820", (820, 560)),
        ("M300 710 V610 L360 550 H540", (540, 550)),
        ("M900 -10 V70 L860 110 H700", (700, 110)),
        ("M110 710 V650 L170 590 H290", (290, 590)),
    ];
    let traces = TRACES.iter().enumerate().flat_map(|(n, (d, (x, y)))| {
        [
            el("path").attr("d", *d).attr("class", "trace"),
            el("path")
                .attr("d", *d)
                .attr("pathLength", "100")
                .attr("class", "current halo")
                .attr("style", format!("--i:{n}")),
            el("path")
                .attr("d", *d)
                .attr("pathLength", "100")
                .attr("class", "current")
                .attr("style", format!("--i:{n}")),
            el("circle")
                .attr("cx", x.to_string())
                .attr("cy", y.to_string())
                .attr("r", "4")
                .attr("class", "pad")
                .attr("style", format!("--i:{n}")),
        ]
    });
    svg()
        .class("circuit")
        .attr("viewBox", "0 0 1200 700")
        .attr("preserveAspectRatio", "xMidYMid slice")
        .attr("aria-hidden", "true")
        .children(traces)
}

/// Bolts floating behind the hero, each at its own depth (`data-speed`: parallax).
fn floaters() -> Element {
    // (left %, top %, size px, parallax speed, outline only)
    const BOLTS: [(u32, u32, u32, &str, bool); 7] = [
        (3, 14, 30, "0.16", true),
        (44, 4, 18, "-0.12", false),
        (95, 10, 26, "0.22", true),
        (62, 90, 34, "0.1", true),
        (1, 72, 20, "-0.18", false),
        (90, 70, 22, "0.28", false),
        (30, 96, 16, "-0.08", true),
    ];
    div()
        .class("floaters")
        .attr("aria-hidden", "true")
        .children(
            BOLTS
                .iter()
                .enumerate()
                .map(|(n, (x, y, size, speed, outline))| {
                    div()
                        .class(if *outline {
                            "floater outline"
                        } else {
                            "floater"
                        })
                        .attr("data-speed", *speed)
                        .attr("style", format!("left:{x}%;top:{y}%;--s:{size}px;--i:{n}"))
                        .child(bolt())
                }),
        )
}

/// The light behind the hero: blurred shapes drifting slowly, over a fading grid.
fn aurora() -> Element {
    div()
        .class("aurora")
        .attr("aria-hidden", "true")
        .child(span().class("blob b1"))
        .child(span().class("blob b2"))
        .child(span().class("blob b3"))
        .child(span().class("grid"))
}

/// What becomes what, on two bands sliding in opposite directions. The same in every
/// language: code reads the same everywhere.
const PAIRS: [(&str, &str); 16] = [
    ("T | null", "Option<T>"),
    ("throw", "Result<T, E>"),
    ("interface", "struct"),
    ("A | B", "enum"),
    ("async / await", "Promise<T>"),
    ("xs.map(f)", "iter().map(f)"),
    ("Map<K, V>", "HashMap<K, V>"),
    ("extends", "trait"),
    ("bigint", "JsBigInt"),
    ("switch", "match"),
    ("a?.b", "map(|a| a.b)"),
    ("function*", "JsGenerator<T>"),
    ("for … of", "for x in &v"),
    ("number", "i64 · f64"),
    ("JSON.parse", "JsFromJson"),
    ("import", "mod"),
];

fn marquee() -> Element {
    let band = |pairs: &[(&'static str, &'static str)], reverse: bool| {
        let chips = |hidden: bool| {
            let mut group = div().class("marquee-group");
            if hidden {
                group = group.attr("aria-hidden", "true");
            }
            group.children(pairs.iter().map(|(ts, rs)| {
                span()
                    .class("chip")
                    .child(code().class("ts").text(*ts))
                    .child(span().class("to").attr("aria-hidden", "true").text("→"))
                    .child(code().class("rs").text(*rs))
            }))
        };
        div()
            .class(if reverse {
                "marquee-row reverse"
            } else {
                "marquee-row"
            })
            .child(chips(false))
            .child(chips(true))
    };
    div()
        .class("marquee")
        .attr("data-reveal", "")
        .child(band(&PAIRS[..8], false))
        .child(band(&PAIRS[8..], true))
}

fn how(lang: Lang) -> Element {
    let d = t(lang);
    section()
        .id("how")
        .class("block")
        .child(kicker(2, d.nav.how))
        .child(split(h2(), d.how.title))
        .child(p().class("lead").attr("data-reveal", "").text(d.how.lead))
        .child(
            // The current runs along the wire as the steps scroll into view (`--p`), and
            // lights each step it reaches (`.lit`).
            ol().class("steps")
                .children(d.how.steps.iter().enumerate().map(|(i, (title, text))| {
                    li().class("spot charged")
                        .attr("data-reveal", "")
                        .attr("data-tilt", "8")
                        .attr("style", format!("--i:{i}"))
                        .child(span().class("node").attr("aria-hidden", "true"))
                        .child(span().class("num").text((i + 1).to_string()))
                        .child(h3().text(*title))
                        .child(p().children(rich(text)))
                })),
        )
        .child(
            h3().class("sub")
                .attr("data-reveal", "")
                .text(d.how.rules_title),
        )
        .child(
            div()
                .class("rules")
                .children(d.how.rules.iter().enumerate().map(|(i, (title, text))| {
                    div()
                        .class("rule spot charged")
                        .attr("data-reveal", "")
                        .attr("data-tilt", "6")
                        .attr("style", format!("--i:{i}"))
                        .child(h4().text(*title))
                        .child(p().children(rich(text)))
                })),
        )
}

fn supported(lang: Lang) -> Element {
    let d = t(lang);
    section()
        .id("supported")
        .class("block")
        .child(kicker(3, d.nav.supported))
        .child(split(h2(), d.supported.title))
        .child(
            p().class("lead")
                .attr("data-reveal", "")
                .text(d.supported.lead),
        )
        .child(
            table()
                .class("mapping")
                .attr("data-reveal", "rows")
                .child(
                    thead().child(
                        tr().child(th().text(d.supported.head.0))
                            .child(th().text(d.supported.head.1)),
                    ),
                )
                .child(tbody().children(d.supported.rows.iter().map(|(ts, rs)| {
                    tr().child(td().child(code().text(*ts)))
                        .child(td().text(*rs))
                }))),
        )
        .child(
            div()
                .class("limits spot charged")
                .attr("data-reveal", "")
                .child(h3().text(d.supported.limits_title))
                .child(p().children(rich(d.supported.limits))),
        )
}

fn install(lang: Lang) -> Element {
    let d = t(lang);
    let block = |title: &'static str, code_text: &str, language: Option<Language>| {
        let html = match language {
            Some(l) => highlight(code_text, l),
            None => shell(code_text),
        };
        div()
            .class("snippet")
            .attr("data-reveal", "")
            .child(h3().text(title))
            .child(pre().class("spot").child(code().html(html)))
    };
    section()
        .id("install")
        .class("block")
        .child(kicker(4, d.nav.install))
        .child(split(h2(), d.install.title))
        .child(
            p().class("lead")
                .attr("data-reveal", "")
                .text(d.install.lead),
        )
        .child(block(d.install.cli, INSTALL_CLI, None))
        .child(block(d.install.check, INSTALL_CHECK, None))
        .child(
            div()
                .class("snippet")
                .attr("data-reveal", "")
                .child(h3().text(d.install.library))
                .child(pre().class("spot").child(code().html(shell(CARGO_LIB))))
                .child(
                    pre()
                        .class("spot")
                        .child(code().html(highlight(INSTALL_LIB, Language::Rust))),
                ),
        )
}
