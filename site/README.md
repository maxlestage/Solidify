# The Solidify website

The presentation site of Solidify, written with [active](https://github.com/maxlestage/Active). It is published at https://maxlestage.github.io/Solidify/.

- **One page in three languages:** English at `/`, French at `/fr/`, Spanish at `/es/`. Every page is rendered on the server with all its SEO tags.
- **Playground:** an island that runs Solidify, compiled to WebAssembly, in the browser. Its examples are the transpiler's own test programs (`crates/solidify/tests/cases`).
- **Installable app (PWA):** the site works offline, playground included.
- **Animations:** many, and none of them is needed for the page to work.

```sh
# The browser bundle (once per change of the Rust sources).
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p solidify-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/solidify_site.wasm

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
| `src/pages.rs` | The page: header (sections, language picker, mobile menu), hero, marquee, playground, how it works, what translates, install, footer. The script in `<head>` that picks the language is here too |
| `src/islands.rs` | The playground island: rendered on the server, then translating live in the browser |
| `src/i18n.rs` | Every text, in English, French and Spanish (`Dict`, one per language) |
| `src/examples.rs` | The playground's examples, read from the test programs |
| `src/highlight.rs` | A small syntax highlighter for TypeScript and Rust, used on both sides |
| `src/pwa.rs` | The web manifest, the service worker and its list of files to keep offline |
| `src/server.rs` | A dependency-free server and the static export (pages, `manifest.webmanifest`, `sw.js`, `static/`, `pkg/`) |
| `src/routes.rs` | The URLs of each language, behind the base path |
| `static/app.css` | The styles, the animations and the small-screen layout |
| `static/app.js` | The motion and the small conveniences, without dependencies (see below) |
| `static/` | Fonts (OFL), favicon, app icons (192, 512, maskable, Apple touch icon) |

## Languages

- **Automatic.** A short script in `<head>`, `LANGUAGE_SCRIPT` in `pages.rs`, runs before the page is drawn. If the visitor has picked a language, it opens that one. Otherwise, on the English home page only, it opens the first of `navigator.languages` that the site offers (`fr-CA` gives `/fr/`, `es-MX` gives `/es/`). A link straight to `/fr/` or `/es/` is never redirected.
- **Chosen.** The picker in the header (a `<details>`, so it works without scripts) and the "Language" column of the footer list English, Français, Español and *Automatic*. `app.js` keeps the choice in `localStorage` (`solidify-lang`). *Automatic* forgets it and lets the browser decide again.
- **Offline.** The three pages are kept by the service worker, so switching language works offline too.

**To add a language:**
1. Add a variant to `Lang` in `routes.rs`, with its code and path.
2. Add a `Dict` in `i18n.rs`; the compiler lists every text to write.
3. Add its code to the two lists of codes in `LANGUAGE_SCRIPT`.
4. Add a README link in `DOCS` (`pages.rs`).

## Animations

Everything below lives in `static/app.css` and `static/app.js`.

The page never waits for these files:
- Every link, the language picker and the playground work without them.
- The hidden starting states apply only once a script marks the page:
  - `.js` (the `<head>` script) for the hero;
  - `.motion` (`app.js` itself) for what appears on scroll.
- If `app.js` fails to load, nothing stays hidden, and the hero shows after 2.5 s at most.
- When the system asks for less motion (`prefers-reduced-motion: reduce`), nothing moves and everything is shown at once.

| Where | What | How |
| --- | --- | --- |
| Header | Slides in when the page opens; turns opaque and gets a line once the page scrolls (`.scrolled`); the link of the section being read is underlined (`.on`) | CSS transitions; `IntersectionObserver` for the current section |
| Top edge | A bar shows how far the page has been read | `.progress`, `transform: scaleX()` from the scroll position |
| Hero title | Each word rises out of a mask, one after another | `emphasized()` wraps every word (`.w > .w-in`, `--i` orders them) |
| Hero accent | `TypeScript` and `Rust` shimmer | an animated gradient clipped to the text |
| Hero | The eyebrow, text, buttons and figures arrive in turn, from a blur | `.intro`, `--d` orders them |
| Hero background | Three blurred lights drift slowly over a fading grid, and move slower than the page (parallax) | `.aurora`, `--py` set on scroll |
| Hero figures | Count up from 0 | `data-count`, eased over 1.4 s |
| Buttons | Drawn a little towards the pointer; a light sweeps across them, the arrow moves on | `.magnetic`, `::after`, `.arrow` |
| Marquee | Two bands of "what becomes what" slide in opposite directions; pointing at one stops it | `.marquee-row`, an infinite translation over two copies |
| Sections | The number and a line that draws itself; the title's words rise in turn; the text arrives from a blur | `.kicker`, `data-reveal="words"`, `data-reveal` |
| Cards | A light follows the pointer; they lift a little; the step numbers turn | `.spot` (`--mx`, `--my` set by `app.js`) |
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

## Installable app

`src/pwa.rs` writes two files:
- `manifest.webmanifest`: name, colours, icons, shortcuts to the playground in each language.
- `sw.js`: when it is installed, the service worker keeps the three pages, `static/` and `pkg/`. Its cache is named after a hash of all of them, so a new version of the site replaces the old one.

The header shows "Install the app" when the browser offers it (`beforeinstallprompt`). On iPhone, it is *Share → Add to Home Screen*.
