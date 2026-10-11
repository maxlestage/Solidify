# solidify

**Write TypeScript or JavaScript, get Rust.** A transpiler with no dependency: one file in, one Rust file out, with structs, enums, `Option`, `Result`, iterators and ownership inferred from the source.

```sh
cargo install --git https://github.com/maxlestage/Solidify solidify
solidify app.ts -o app.rs
rustc --edition 2024 app.rs && ./app
```

```rust
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

- `transpile` returns the Rust source, or an [`Error`] with a byte span; `Error::pretty` formats it with the line, the column and a caret.
- Numbers are `i64` until a float literal or a division makes them `f64`; `T | null` is `Option<T>`; a function that throws returns `Result<T, String>`; parameters are borrowed unless the function changes or keeps them.
- The supported subset, the mapping table and the limits are described in the [repository's README](https://github.com/maxlestage/Solidify#readme), and a playground running this crate as WebAssembly is at https://maxlestage.github.io/Solidify/.

Proprietary software, © 2026 Maxime Nathan Lestage, all rights reserved; free to use as a dependency of your programs. See [LICENSE](LICENSE).
