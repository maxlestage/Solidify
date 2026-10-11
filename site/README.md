# The Solidify website

The presentation site of Solidify, written with [active](https://github.com/maxlestage/Active): one page in English (`/`) and French (`/fr/`), rendered on the server with every SEO tag, and a playground island that runs Solidify compiled to WebAssembly in the browser. The examples of the playground are the transpiler's own test programs (`crates/solidify/tests/cases`).

```sh
# The browser bundle (once per change of the Rust sources).
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p solidify-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/solidify_site.wasm

cargo run -p solidify-site                 # http://localhost:3000
cargo run -p solidify-site -- export dist  # static files for GitHub Pages or any host
```

Environment variables: `PORT` (3000), `BASE_PATH` (`/Solidify` on GitHub Pages), `SITE_URL` (the public address, for canonical and Open Graph URLs), `SITE_DIR` (this folder, when the binary runs elsewhere).

| File | Contents |
| --- | --- |
| `src/pages.rs` | The page: hero, playground, how it works, what translates, install, footer |
| `src/islands.rs` | The playground island: server render, then live translation in the browser |
| `src/i18n.rs` | Every text, in English and French |
| `src/highlight.rs` | A small syntax highlighter for TypeScript and Rust, used on both sides |
| `src/server.rs` | A dependency-free server and the static export |
| `static/` | Stylesheet, fonts (OFL), icon |
