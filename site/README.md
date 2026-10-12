# The Solidify website

The presentation site of Solidify, written with [active](https://github.com/maxlestage/Active). It is published at https://maxlestage.github.io/Solidify/.

- **Three languages:** English at `/`, French at `/fr/`, Spanish at `/es/`. Every page is rendered on the server with all its SEO tags, and links to the same page in the other languages.
- **The home page**, and **the documentation**: its home at `/docs/` (`/fr/docs/`, `/es/docs/`) and 16 pages (`/docs/quick-start/`…), written in Markdown in [`../docs`](../docs), with a menu, a search and "On this page".
- **Playground:** an island that runs Solidify, compiled to WebAssembly, in the browser. Its examples are the transpiler's own test programs (`crates/solidify/tests/cases`).
- **Installable app (PWA):** the site works offline, playground included.
- **Animations:** many, on an electric theme (Solidify's yellow is a spark), and none of them is needed for the page to work.
- **No JavaScript of its own.** Everything is Rust: the pages are rendered by `active`, and the browser runs two WebAssembly modules.
  - The page's client (`src/client/`): language, playground, motion, menu, search, documentation, app.
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
| `src/pages/mod.rs` | What every page shares: the head and its SEO tags, the header and its menu (the sections, the documentation's mega menu, GitHub; on small screens every page, the resources and the languages), the search dialog, the footer, the cursor, the button back to the top |
| `src/pages/home.rs` | The home page: preloader, hero (with its scene, circuit and floating bolts), marquee, playground, how it works, what translates, install |
| `src/pages/docs.rs` | The documentation: its home (emblem, search, where to start, every page) and its pages (the list of pages, the page, "On this page", previous and next) |
| `src/catalog.rs` | The documentation's categories and pages, in reading order, with their titles and summaries in the three languages |
| `src/docs.rs` | The Markdown pages of `../docs`, built in with `include_str!` and rendered once; the tests that hold them to what Solidify does |
| `src/markdown.rs` | A dependency-free Markdown renderer: anchors as on GitHub, highlighted code with a copy button, "this becomes that" pairs, callouts, tables |
| `src/search.rs` | What the search compares (words without case or accents) and how it ranks, the same on the server and in the browser |
| `src/client/mod.rs` | The page's WebAssembly client: starts everything below, and small helpers over `web-sys` |
| `src/client/language.rs` | Opens the visitor's language, keeps the one picked |
| `src/client/motion.rs` | What the motion needs from the scroll position, the pointer, the elements in view or the visit (preloader, cursor, tilt, sparks); the menu; copy buttons |
| `src/client/search.rs` | The search dialog: the `/` key and Ctrl K, ranking the results written in the page, the arrow keys |
| `src/client/docs.rs` | The list of pages as a drawer on small screens; "On this page" following the reading |
| `src/client/app.rs` | Registers the service worker; the "Install the app" button |
| `worker/` | The service worker, a crate of its own (`solidify-site-worker`) compiled to WebAssembly |
| `src/islands.rs` | The playground island: rendered on the server, then translating live in the browser |
| `src/i18n.rs` | Every text, in English, French and Spanish (`Dict`, one per language) |
| `src/examples.rs` | The playground's examples, read from the test programs |
| `src/highlight.rs` | A small syntax highlighter for TypeScript and Rust, used on both sides |
| `src/pwa.rs` | The web manifest (`active::pwa::Manifest`), `sw.js` (the worker inlined) and the list of files to keep offline |
| `src/server.rs` | A dependency-free server and the static export (every page, `manifest.webmanifest`, `sw.js`, `sitemap.xml` with each page's address in the three languages, `static/`, `pkg/`) |
| `src/routes.rs` | The URLs of every page in each language (`Route`: home, documentation, a page of it), behind the base path |
| `static/app.css` | The styles, the animations and the small-screen layout |
| `static/` | Fonts (OFL), favicon, app icons (192, 512, maskable, Apple touch icon) |

## Languages

- **Automatic.** The first thing the WebAssembly client does (`client/language.rs`), before anything else starts, is to choose the language.
  - If the visitor has picked one, it opens that one.
  - Otherwise, on an English page, it opens the first of `navigator.languages` that the site offers (`fr-CA` gives `/fr/`, `es-MX` gives `/es/`).
  - Either way the same page opens in that language: `/docs/cli/` becomes `/fr/docs/cli/`.
  - A link straight to `/fr/…` or `/es/…` is never redirected.
- **Chosen.** The picker in the header (a `<details>`, so it works without WebAssembly), the menu on small screens and the "Language" column of the footer list English, Français, Español and *Automatic*, each leading to the same page. The client keeps the choice in `localStorage` (`solidify-lang`). *Automatic* forgets it and lets the browser decide again.
- **Offline.** Every page is kept by the service worker, so switching language works offline too.

**To add a language:**
1. Add a variant to `Lang` in `routes.rs`, with its code and path.
2. Add a `Dict` in `i18n.rs`; the compiler lists every text to write.
3. Add the titles and summaries of the documentation's pages in `catalog.rs`, and its pages in `../docs/{code}/` (then their paths in `SOURCES`, `docs.rs`).

## The documentation

The pages are the Markdown files of [`../docs`](../docs): `docs/{en,fr,es}/{slug}.md`, the same names in every language. `catalog.rs` gives their order, categories, titles and summaries; `docs.rs` builds them in (`include_str!`), so the server and the static export need nothing beside them, and renders each once with `markdown.rs`.

- **Its home** (`/docs/`): an emblem, the search, three pages to start with, then every page by category.
- **A page** (`/docs/{slug}/`): the list of pages on the left (a drawer on small screens), the page, "On this page" on the right, its current section marked as the page is read, a link to edit the page on GitHub, the previous and next pages.
- **Its examples are checked.** The tests of `docs.rs` translate every TypeScript example, compare it with the Rust shown under it (exactly, or piece by piece for an excerpt marked `fragment`), check the error messages, and check that every link to a section of a page leads to a heading, in each language. [`../docs/README.md`](../docs/README.md) tells how to write a page.

## The menu and the search

- **The menu**, on wide screens: the sections of the home page (from another page, they lead to it), *Docs* and GitHub. Pointing at *Docs* (or focusing it) opens a panel of every page by category, with a card that leads to the playground (CSS only).
- **On small screens** the menu button opens a whole screen: the sections in large letters, then every page of the documentation, the resources and the languages.
- **The search** is in every page: the button in the header, the `/` key or Ctrl K (⌘ K) open it. Its results are written in the page by the server (the sections of the home page, every page of the documentation, and every section of each): the client shows those matching what is typed, best first, and moves through them with the arrow keys. Without the client the button leads to the documentation's home.

## Animations

The visuals are CSS (`static/app.css`). What needs the scroll position, the pointer, the elements in view or the visit is the WebAssembly client (`src/client/motion.rs`). The theme is electricity: the preloader powers the page up, the cursor is a spark, a current runs along circuits, cards and the steps, lightning turns the hero's TypeScript cube into Rust, clicks throw sparks, the wordmark is a neon sign.

The page never waits for the client:
- The preloader, the hero's entrance and scene, and the counting figures are CSS animations: they play as the page opens, with or without WebAssembly.
- The preloader plays once per visit: on the next pages the client hides it (`intro-seen`, kept in `sessionStorage` under `solidify-intro`), and a click lifts it (`intro-skip`). The hero waits for it through `--hero-delay`.
- What appears on scroll is hidden only once the client runs: it adds `.motion` to `<html>`, then `.in` to each element that comes into view.
- If the client never loads, nothing stays hidden.
- When the system asks for less motion (`prefers-reduced-motion: reduce`), nothing moves and everything is shown at once: no preloader, no cursor, no sparks, the scene's solid Rust cube still, the steps lit.

| Where | What | How |
| --- | --- | --- |
| Preloader | A bolt draws itself and lights up; the letters of SOLIDIFY flicker on like neon; a battery charges cell by cell while large hollow figures count to 100 %; a flash, then a yellow curtain lifts | `loader()` in `pages.rs`; CSS only (`--pct`, an integer declared with `@property`, shown through a counter) |
| Header | Slides in when the page opens; turns opaque and gets a line once the page scrolls (`.scrolled`); the link of the section being read is underlined (`.on`) | CSS transitions; `IntersectionObserver` for the current section |
| Top edge | A glowing bar shows how far the page has been read, a spark crackling at its end | `.progress`, `transform: scaleX()` from the scroll position; `.progress-head` follows it |
| Cursor (fine pointer) | A spark, and a dashed ring of charge that catches up with it; the ring grows over links and buttons and shows a label (`Go`) on the main button; hidden over text fields | `.cursor-dot`, `.cursor-ring`, `data-cursor` |
| Clicks and taps | Sparks fly: a flash, a ring of charge, eight rays of random length | `.sparkburst`, made by the client and removed after 0.7 s |
| Hero title | Each word rises out of a mask, one after another | `emphasized()` wraps every word (`.w > .w-in`, `--i` orders them) |
| Hero accent | `TypeScript` and `Rust` shimmer | an animated gradient clipped to the text |
| Hero | The eyebrow, text, buttons and figures arrive in turn, from a blur | `.intro`, `--d` orders them (CSS) |
| Hero background | Three blurred lights drift slowly over a fading grid, and move slower than the page (parallax); a pulse of current runs along each trace of a circuit and lights the pad it reaches; bolts float at their own depths and flicker now and then | `.aurora`, `--py` set on scroll; `circuit()` (SVG, `stroke-dashoffset`); `floaters()`, `data-speed` |
| Hero scene | A dashed TypeScript cube turns in 3D; the charge crackles around it, lightning strikes, a flash, and it becomes a solid Rust cube with sparks orbiting it; then it fades back. It leans towards the pointer | `scene()`: CSS 3D, one 7 s cycle (`--cycle`); `--hx`, `--hy` set by the client |
| Hero figures | Count up from 0 | CSS only: `--n`, an integer declared with `@property`, animated from 0 to `--to` and shown through a counter; the figure stays in the text for screen readers (`.value`) |
| Buttons | Drawn a little towards the pointer; a light sweeps across them, the arrow moves on; the main one crackles | `.magnetic`, `::after`, `.arrow` |
| Marquee | Two bands of "what becomes what" slide in opposite directions; pointing at one stops it | `.marquee-row`, an infinite translation over two copies |
| Sections | The number flickers on and a line draws itself; the title's words rise in turn; the text arrives from a blur | `.kicker`, `data-reveal="words"`, `data-reveal` |
| Cards | A light follows the pointer; they lean in 3D towards it and lift a little; a current runs around the edge of the one pointed at; the step numbers turn | `.spot` (`--mx`, `--my`), `data-tilt` (`--rx`, `--ry`), `.charged` (a conic gradient turning, `--a`) |
| How it works | A wire runs above the steps (down their side on small screens): the current fills it as they come up the screen and lights each step it reaches | `--p` and `.lit` set on scroll; lit at once without the client |
| Table | Rows arrive one after another; the row pointed at gets a bar | `data-reveal="rows"`, `--i` on each row |
| Playground | Grows into place; every translation (a keystroke, an example) sends a discharge along the line between the panes and the Rust tag flickers | `data-reveal="scale"`; `.surge`, restarted by the client |
| Page | A soft light follows the pointer (desktop) | `.glow` |
| Language | The page fades out when it changes, and the browser can cross-fade the next one | `.leaving`, `@view-transition` |
| Menu (small screens) | The sections, in large letters, then the pages, the resources and the languages slide in one after another; the three bars become a cross | `.menu-open` |
| Mega menu | Pointing at *Docs*: the panel drops in, a current runs along its top edge, its columns arrive in turn; the playground's bolt crackles | `.mega`, `:hover` and `:focus-within` (CSS only) |
| Search | The dialog rises from a blur, a current runs to and fro under the field, the results arrive in turn | `.search[open]`, `--r` orders the results |
| Hero accents | Now and then, interference crosses `TypeScript` and `Rust`: the word jolts, split in blue and yellow | `em::before`, `em::after` (`data-text`), clipped in slices |
| Install | The commands power up: a bright line scans down and the text appears behind it | `.snippet.in pre::after`, `clip-path` |
| Sections | As each number arrives, a current runs along the section's top line | `.block:has(.kicker.in)::before` |
| Logo | Its yellow square buzzes now and then, like a sign | `logo-idle` |
| Back to the top | Once the page has scrolled, a button appears; its ring fills with the reading | `.to-top-ring`, `--p` set on scroll |
| Documentation's home | A bolt at the heart of three turning rings of charge, sparks orbiting it; the cards lean and carry a current | `core()`, `data-tilt`, `.charged` |
| Documentation's pages | A line charges under each section's title; "this becomes that": a spark runs down from the TypeScript to the Rust, which lights up; the current page is a lit node in the list; a current runs down "On this page" to the section being read | `data-reveal="line"`, `.pair.in`, `--wire` |
| From page to page | The old page dims, the new one is wiped in from the top (where the browser cross-fades pages) | `::view-transition-old(root)`, `::view-transition-new(root)` |
| Footer | The columns arrive in turn; links underline from the left; the status light pulses; the large wordmark rises as the end of the page comes, and lights up like a neon sign, letter by letter, one of them buzzing now and then | `data-reveal`, `--seen` set on scroll; `data-reveal="neon"` |
| Page | A fine grain moves over everything | `.noise` |

**To animate a new element:**
- Give it `data-reveal`, and optionally `style="--i:n"` to order it among its neighbours.
- Use `data-reveal="words"` (or `split()` in `pages.rs`) for a heading.
- Use `data-reveal="scale"` for a block that should grow.
- Add the class `charged` to a card (with `.spot`) for the current along its edge, and `data-tilt="8"` for it to lean (the most it turns, in degrees).

## No horizontal scrolling

Nothing on the site scrolls sideways, on any screen:
- **The page:** `overflow-x: clip` on `<html>`, `<body>` and `<main>`, so the hero's light and the marquee never widen it.
- **Code** (install snippets, the playground's editor and output, inline code) wraps at the edge of its block: `white-space: pre-wrap` and `overflow-wrap: anywhere`. Code blocks only grow down.
- **The table** of what translates wraps its cells. Below 640 px each row becomes a block, the TypeScript above its Rust.
- **The menu, without WebAssembly:** on small screens the sections are not shown in the header; the footer links them, and the documentation's home links every page.
- **The documentation:** code wraps; its tables wrap their cells, and below 640 px each row becomes a block, each cell under its column's title.

The test `nothing_scrolls_sideways` (`src/server.rs`) fails if a rule of `app.css` makes an element scroll horizontally again. Checked in a browser at 320, 390, 768 and 1440 px wide, in the three languages, with the menu open and a very long line in the playground.

## Installable app

`src/pwa.rs` writes two files.

- `manifest.webmanifest`: built with `active::pwa::Manifest` (name, colours, icons).
- `sw.js`: the service worker, written in Rust (`worker/`).
  - Browsers only start a service worker from a script, and only listen to the events it registers while that script first runs. So `sw.js` holds three things: the loader `wasm-bindgen` generates (`--target no-modules`), the worker's WebAssembly inlined (42 KB), and the call to `serve`.
  - `serve` receives this version's cache name and the files to keep: every page (the home page and the documentation, in the three languages), `static/` and `pkg/`. The cache is named after a hash of all of them, so a new version of the site replaces the old one.
  - Its strategy: pages from the network first, the saved copy offline; other files from the cache first.

The client (`client/app.rs`) registers the worker once the page has loaded. It shows "Install the app" when the browser offers it (`beforeinstallprompt`). On iPhone, it is *Share → Add to Home Screen*.
