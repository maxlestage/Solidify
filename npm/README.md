# solidify-rs

**Write TypeScript or JavaScript, get Rust.** Solidify reads a TypeScript (or JavaScript) program, the files it imports included, and writes the Rust you would have written by hand: structs, enums, `Option`, `Result`, iterators and ownership. The output prints what Node.js prints.

```sh
npx solidify-rs main.ts -o app.rs     # follows the imports of main.ts, one Rust file out
npx solidify-rs < app.ts > app.rs     # or one file through the standard input
npx solidify-rs --check main.ts       # only report errors
rustc --edition 2024 app.rs && ./app
```

Nothing to install but Node.js 20 or later: the transpiler is written in Rust and runs here as WebAssembly, with access to your files.

Documentation, the full list of what translates, and a playground running in the browser: https://maxlestage.github.io/Solidify/ · Source: https://github.com/maxlestage/Solidify

© 2026 Maxime Nathan Lestage, all rights reserved. Free to use; see LICENSE.
