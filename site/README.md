# The Solidify website

The presentation site of Solidify, written with [active](https://github.com/maxlestage/Active). It is published at https://maxlestage.github.io/Solidify/.

- **One page in three languages:** English at `/`, French at `/fr/`, Spanish at `/es/`. Every page is rendered on the server with all its SEO tags.
- **Playground:** an island that runs Solidify, compiled to WebAssembly, in the browser. Its examples are the transpiler's own test programs (`crates/solidify/tests/cases`).
- **Installable app (PWA):** the site works offline, playground included.
- **Animations:** many, and none of them is needed for the page to work.
- **No JavaScript of its own.** Everything is Rust: the pages are rendered by `active`, and the browser runs two WebAssembly modules.
  - The page's client (`src/client/`): language, playground, motion, app.
  - The service worker (`worker/`).

  The only JavaScript is what `wasm-bindgen` generates to load them, and `active`'s one-line loader.

```sh
# The browser bundle (once per change of the Rust sources).
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p solidify-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/solidify_site.wasm
# The service worker (offline use), Rust too.
cargo build -p solidify-site-worker --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target no-modules --no-typescript --out-dir site/pkg/worker target/wasm32-unknown-unknown/wasm-release/solidify_site_worker.wasm

cargo run -p solidify-site                 # http://localhost:3000
cargo run -p solidify-site -- export dist  # static files for GitHub Pages or any host
```

Environment variables:
- `PORT`: the port to serve on (3000).
- `BASE_PATH`: the path the site lives under (`/Solidify` on GitHub Pages).
- `SITE_URL`: the public address, for the canonical and Open Graph URLs.
- `SITE_DIR`: this folder, when the binary runs elsewhere.

## Files

| File | Contents |
| --- | --- |
| `src/pages.rs` | The page: header (sections, language picker, mobile menu), hero, marquee, playground, how it works, what translates, install, footer |
| `src/client/mod.rs` | The page's WebAssembly client: starts everything below, and small helpers over `web-sys` |
| `src/client/language.rs` | Opens the visitor's language, keeps the one picked |
| `src/client/motion.rs` | What the motion needs from the scroll position, the pointer or the elements in view; the menu; copy buttons |
| `src/client/app.rs` | Registers the service worker; the "Install the app" button |
| `worker/` | The service worker, a crate of its own (`solidify-site-worker`) compiled to WebAssembly |
| `src/islands.rs` | The playground island: rendered on the server, then translating live in the browser |
| `src/i18n.rs` | Every text, in English, French and Spanish (`Dict`, one per language) |
| `src/examples.rs` | The playground's examples, read from the test programs |
| `src/highlight.rs` | A small syntax highlighter for TypeScript and Rust, used on both sides |
| `src/pwa.rs` | The web manifest (`active::pwa::Manifest`), `sw.js` (the worker inlined) and the list of files to keep offline |
| `src/server.rs` | A dependency-free server and the static export (pages, `manifest.webmanifest`, `sw.js`, `static/`, `pkg/`) |
| `src/routes.rs` | The URLs of each language, behind the base path |
| `static/app.css` | The styles, the animations and the small-screen layout |
| `static/` | Fonts (OFL), favicon, app icons (192, 512, maskable, Apple touch icon) |

## Languages

- **Automatic.** The first thing the WebAssembly client does (`client/language.rs`), before anything else starts, is to choose the language.
  - If the visitor has picked one, it opens that one.
  - Otherwise, on the English home page only, it opens the first of `navigator.languages` that the site offers (`fr-CA` gives `/fr/`, `es-MX` gives `/es/`).
  - A link straight to `/fr/` or `/es/` is never redirected.
- **Chosen.** The picker in the header (a `<details>`, so it works without WebAssembly) and the "Language" column of the footer list English, Français, Español and *Automatic*. The client keeps the choice in `localStorage` (`solidify-lang`). *Automatic* forgets it and lets the browser decide again.
- **Offline.** The three pages are kept by the service worker, so switching language works offline too.

**To add a language:**
1. Add a variant to `Lang` in `routes.rs`, with its code and path.
2. Add a `Dict` in `i18n.rs`; the compiler lists every text to write.
3. Add a README link in `DOCS` (`pages.rs`).

## Animations

The visuals are CSS (`static/app.css`). What needs the scroll position, the pointer or the elements in view is the WebAssembly client (`src/client/motion.rs`).

The page never waits for the client:
- The hero's entrance and the counting figures are CSS animations: they play as the page opens, with or without WebAssembly.
- What appears on scroll is hidden only once the client runs: it adds `.motion` to `<html>`, then `.in` to each element that comes into view.
- If the client never loads, nothing stays hidden.
- When the system asks for less motion (`prefers-reduced-motion: reduce`), nothing moves and everything is shown at once.

| Where | What | How |
| --- | --- | --- |
| Header | Slides in when the page opens; turns opaque and gets a line once the page scrolls (`.scrolled`); the link of the section being read is underlined (`.on`) | CSS transitions; `IntersectionObserver` for the current section |
| Top edge | A bar shows how far the page has been read | `.progress`, `transform: scaleX()` from the scroll position |
| Hero title | Each word rises out of a mask, one after another | `emphasized()` wraps every word (`.w > .w-in`, `--i` orders them) |
| Hero accent | `TypeScript` and `Rust` shimmer | an animated gradient clipped to the text |
| Hero | The eyebrow, text, buttons and figures arrive in turn, from a blur | `.intro`, `--d` orders them (CSS) |
| Hero background | Three blurred lights drift slowly over a fading grid, and move slower than the page (parallax) | `.aurora`, `--py` set on scroll |
| Hero figures | Count up from 0 | CSS only: `--n`, an integer declared with `@property`, animated from 0 to `--to` and shown through a counter; the figure stays in the text for screen readers (`.value`) |
| Buttons | Drawn a little towards the pointer; a light sweeps across them, the arrow moves on | `.magnetic`, `::after`, `.arrow` |
| Marquee | Two bands of "what becomes what" slide in opposite directions; pointing at one stops it | `.marquee-row`, an infinite translation over two copies |
| Sections | The number and a line that draws itself; the title's words rise in turn; the text arrives from a blur | `.kicker`, `data-reveal="words"`, `data-reveal` |
| Cards | A light follows the pointer; they lift a little; the step numbers turn | `.spot` (`--mx`, `--my` set by the client) |
| Table | Rows arrive one after another; the row pointed at gets a bar | `data-reveal="rows"`, `--i` on each row |
| Playground | Grows into place | `data-reveal="scale"` |
| Page | A soft light follows the pointer (desktop) | `.glow` |
| Language | The page fades out when it changes, and the browser can cross-fade the next one | `.leaving`, `@view-transition` |
| Menu (small screens) | The sections, in large letters, slide in one after another; the three bars become a cross | `.menu-open` |
| Footer | The columns arrive in turn; links underline from the left; the status light pulses; the large wordmark rises as the end of the page comes | `data-reveal`, `--seen` set on scroll |

**To animate a new element:**
- Give it `data-reveal`, and optionally `style="--i:n"` to order it among its neighbours.
- Use `data-reveal="words"` (or `split()` in `pages.rs`) for a heading.
- Use `data-reveal="scale"` for a block that should grow.

## No horizontal scrolling

Nothing on the site scrolls sideways, on any screen:
- **The page:** `overflow-x: clip` on `<html>`, `<body>` and `<main>`, so the hero's light and the marquee never widen it.
- **Code** (install snippets, the playground's editor and output, inline code) wraps at the edge of its block: `white-space: pre-wrap` and `overflow-wrap: anywhere`. Code blocks only grow down.
- **The table** of what translates wraps its cells. Below 640 px each row becomes a block, the TypeScript above its Rust.
- **The menu, without WebAssembly:** on small screens the sections are not shown in the header; the footer's "Product" column links them.

The test `nothing_scrolls_sideways` (`src/server.rs`) fails if a rule of `app.css` makes an element scroll horizontally again. Checked in a browser at 320, 390, 768 and 1440 px wide, in the three languages, with the menu open and a very long line in the playground.

## Installable app

`src/pwa.rs` writes two files.

- `manifest.webmanifest`: built with `active::pwa::Manifest` (name, colours, icons).
- `sw.js`: the service worker, written in Rust (`worker/`).
  - Browsers only start a service worker from a script, and only listen to the events it registers while that script first runs. So `sw.js` holds three things: the loader `wasm-bindgen` generates (`--target no-modules`), the worker's WebAssembly inlined (42 KB), and the call to `serve`.
  - `serve` receives this version's cache name and the files to keep: the three pages, `static/` and `pkg/`. The cache is named after a hash of all of them, so a new version of the site replaces the old one.
  - Its strategy: pages from the network first, the saved copy offline; other files from the cache first.

The client (`client/app.rs`) registers the worker once the page has loaded. It shows "Install the app" when the browser offers it (`beforeinstallprompt`). On iPhone, it is *Share → Add to Home Screen*.
