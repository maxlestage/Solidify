# Command line

`npx solidify-rs` runs Solidify on your files: the npm package holds the transpiler compiled to WebAssembly, run by Node.js 20 or later. Built from the sources, the `solidify` binary takes the same options.

## Usage

```sh
npx solidify-rs [input.ts] [-o output.rs] [--check] [--no-header]
```

It translates `input.ts` and every file it imports to one Rust file. Without an input file it reads the standard input; without `-o` it writes to the standard output.

```sh
npx solidify-rs main.ts -o app.rs     # follows the imports of main.ts, one Rust file out
npx solidify-rs < app.ts > app.rs     # one file through the standard input
npx solidify-rs --check main.ts       # only report errors
```

## Options

| Option | What it does |
| --- | --- |
| `input.ts` | The entry file of the program, TypeScript or JavaScript. `-` or nothing reads the standard input, where imports are not followed. |
| `-o file`, `--output file` | Writes the Rust to `file` instead of the standard output. |
| `--check` | Translates without writing anything: the exit code and the messages say whether the program translates. |
| `--no-header` | Leaves out the comment that names the source file at the top of the output. |
| `-h`, `--help` | Prints the usage. |
| `-V`, `--version` | Prints the version. |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The program was translated (or checked, with `--check`). |
| `1` | The program does not translate: the message, the file, the line and the column are on the standard error. |
| `2` | The command itself failed: an unknown option, `-o` without a file, a file that cannot be read or written. |

## Errors

Messages go to the standard error, one per problem found, with the line and a mark under the place:

```text
error: `name` may be null here: test it with `if (… !== null)` first
  --> app.ts:2:10
   |
  2|   return name.charAt(0);
   |          ^^^^
```

[Errors](errors.md#solidifys-messages) explains the common ones.

## In a build

`--check` makes a quick test for continuous integration, and the exit codes let a script stop at the first program that does not translate:

```sh
for f in scripts/*.ts; do
  npx solidify-rs --check "$f" || exit 1
done
```

To keep a translated program up to date, translate it then compile it in the same step:

```sh
npx solidify-rs main.ts -o src/main.rs && cargo build --release
```

## Compiling the output

```sh
rustc --edition 2024 -O app.rs -o app
```

The output needs no crate. Put it in a Cargo project as `src/main.rs` to use Cargo's tools (`cargo build`, `cargo clippy`); set `edition = "2024"` in its `Cargo.toml`.
