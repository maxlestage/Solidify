# FAQ

Short answers to the questions people ask about Solidify.

## Is the output readable Rust?

Yes: it is meant to be read and kept. Structs, enums, `Option`, `Result`, borrowed parameters and iterator chains are what you would write by hand; the names follow Rust's conventions and the comments are kept. The helpers written after `main` are plain Rust functions too.

## Does the output need a runtime or a crate?

No. The output is one file that `rustc` compiles alone, for the 2024 edition. What Rust's standard library lacks (printing numbers as JavaScript does, regular expressions, dates, promises and timers) is written into the file, and only what the program uses.

## Is the translated program faster?

Usually: it is compiled ahead of time, without a garbage collector, and numbers that stay whole are machine integers. Code built at run time with `eval` runs in an interpreter, and stays slow.

## Can I edit the output?

Yes. It is your program, in Rust: translate once, then work on the Rust. Or keep the TypeScript as the source and translate on every build (see the [command line](cli.md#in-a-build)).

## Which TypeScript does it read?

Modern TypeScript and JavaScript: classes, interfaces, generics, unions, enums, `async`/`await`, generators, destructuring, optional chaining, modules (ES and CommonJS). It does not run TypeScript's own checker: it infers its own types, which are stricter about `null`.

## Why is a number an i64 and not an f64?

Because most numbers in programs are counts and indices. A number stays an integer until the program gives it a float, divides it or reads it from JSON; then it is an `f64`, and so is every number it flows into. The output prints both as JavaScript would.

## Does it work in the browser?

The transpiler does: the playground on the home page runs it, compiled to WebAssembly, and the site works offline once installed as an app. The programs it translates are command-line programs: they have no DOM.

## Does it send my code anywhere?

No. `npx solidify-rs` runs on your machine, and the playground runs in your browser.

## Can I use it in my project, or my company?

Yes: you may use Solidify freely, commercial projects included, and the Rust it writes is yours. Solidify itself is proprietary software: modifying it or redistributing its source needs the author's written permission. The [license](../../LICENSE) gives the terms.

## How do I report a bug?

Open an issue on [GitHub](https://github.com/maxlestage/Solidify/issues) with the program, what Node.js prints and what the Rust prints (or Solidify's message). Programs that once failed become test programs.
