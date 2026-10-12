# Errors

Two kinds of errors meet here: the ones the program throws while it runs, which become Rust's `Result`, and the ones Solidify reports when it cannot translate a program, with the file, the line and the column.

## throw becomes Result

A function that throws, or calls one that does outside a `try`, returns `Result<T, String>`: `throw new Error("…")` becomes `return Err(…)`, and its callers pass the error on with `?`. A `try` block becomes a closure whose error `catch` receives, as the message:

```ts
function withdraw(balance: number, amount: number): number {
  if (amount > balance) {
    throw new Error(`only ${balance} left`);
  }
  return balance - amount;
}

try {
  console.log(withdraw(10, 4));
  console.log(withdraw(10, 40));
} catch (e) {
  console.log(`refused: ${e.message}`);
}
```
```rust
fn withdraw(balance: i64, amount: i64) -> Result<i64, String> {
    if amount > balance {
        return Err(format!("only {balance} left"));
    }
    Ok(balance - amount)
}

fn main() {
    let result_: Result<(), String> = (|| {
        println!("{}", withdraw(10, 4)?);
        println!("{}", withdraw(10, 40)?);
        Ok(())
    })();
    if let Err(e) = result_ {
        println!("refused: {e}");
    }
}
```

An error nothing catches reaches `main`, which returns it: the program stops with the exit code 1 and prints the message on the standard error (`Error: "only 10 left"`), where Node.js would print a stack trace.

## Error classes

A class that extends `Error` keeps its fields, its message and its name through the `Result`; `instanceof` in `catch` finds it again:

```ts
class NotFound extends Error {
  constructor(public key: string) {
    super(`no ${key}`);
    this.name = "NotFound";
  }
}

function lookup(key: string): string {
  if (key !== "a") {
    throw new NotFound(key);
  }
  return "found";
}

try {
  lookup("b");
} catch (e) {
  if (e instanceof NotFound) {
    console.log(`${e.name}: ${e.key}`);
  }
}
```
```rust fragment
#[derive(Debug, Clone, PartialEq)]
struct NotFound {
    pub key: String,
    pub message: String,
    pub name: String,
}

impl NotFound {
    pub fn new(key: String) -> Self {
        Self { key: key.clone(), message: format!("no {key}"), name: "NotFound".to_string() }
    }
}

fn lookup(key: &str) -> Result<String, String> {
    if key != "a" {
        return Err(js_error_encode(&NotFound::new(key.to_string()).js_value(), "NotFound,Error"));
    }
    Ok("found".to_string())
}

fn main() {
    let result_: Result<(), String> = (|| {
        lookup("b")?;
        Ok(())
    })();
    if let Err(e) = result_ {
        if let Some(e) = js_error_as::<NotFound>(&e, "NotFound") {
            println!("{}: {}", e.name, e.key);
        }
    }
}

// …
```

`JSON.parse` throws the same `SyntaxError` messages as V8, so a program that prints them prints the same text.

## Solidify's messages

When a program cannot be translated, Solidify says why and where: the message, then the file, the line and the column, the line itself and a mark under the place. `npx solidify-rs --check` only reports them, without writing anything.

### A value that may be null

Reading a value that may be `null` or `undefined` is an error, as in TypeScript's strict mode:

```ts error
const names: string[] = ["Ada"];
const first = names.find((n) => n.startsWith("A"));
console.log(first.length);
```
```text
error: `first` may be null here (string | null): test it with `if (… !== null)`, or use `?.` or `!`
  --> app.ts:3:13
   |
  3| console.log(first.length);
   |             ^^^^^^^^^^^^
```

Test it first (`if (first !== undefined)`), give a default (`first ?? ""`), read it with `?.`, or assert it with `!` when you know better.

### A type that does not fit

A variable keeps one type. Giving it a value of another is an error:

```ts error
let count = 0;
count = "many";
```
```text
error: expected number (integer), found string
  --> app.ts:2:9
   |
  2| count = "many";
   |         ^^^^^^
```

Use two variables, or declare the union you mean (`let count: number | string = 0`).

### What is not supported

A few things are refused, each with a message that names it:

```ts error
async function* ticks() {
  yield 1;
}
```
```text
error: async generators are not supported
  --> app.ts:1:7
   |
  1| async function* ticks() {
   |       ^^^^^^^^^^^^^^^^^^^
```

The [limits](limits.md) list them all.

## In the Rust library

`solidify::transpile` returns a `solidify::Error`: its `message`, and a `span` (byte offsets in the source). `pretty(source)` formats it as above; the [Rust library](library.md#errors) page shows how.
