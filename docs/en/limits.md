# Limits

The generated program prints what Node.js prints; a few things cannot be the same, and a few are refused. Each refusal comes with a message that says what, and where.

## What stays different

- **Uncaught errors.** An error printed whole shows `Error: message`, without JavaScript's stack trace. An error nothing catches stops the program with the exit code 1.
- **Timers** use the real clock: two timers a few milliseconds apart may swap under heavy load, as they can in Node.js.
- **Code built at run time** for `eval` or `new Function` runs in a small interpreter written into the file, slower than translated code.
- **Dates** are read and shown in UTC.
- **Numbers** that stay whole are `i64`: a program that relies on integers above 2⁵³ losing precision, as JavaScript's doubles do, gets exact results instead.

## What is refused

| What | Why |
| --- | --- |
| `async function*` | Async generators are not supported. |
| `http`, `net`, `child_process` and Node's other modules | Only `fs`, `fs/promises`, `path`, `os` and `process` are shipped. |
| Native add-ons in `node_modules` | They are compiled code, not sources to translate. |
| The DOM, `window`, `fetch` | Solidify translates programs, not pages for the browser. |
| `Date.UTC` | Write the date as an ISO string instead. |
| A variable that changes type | Use two variables, or declare the union you mean. |

## Strictness

Solidify reads TypeScript as its strict mode does: reading a value that may be `null` or `undefined` is an error until it is tested, and a variable keeps the type of its first value unless its annotation says otherwise. [Errors](errors.md#solidifys-messages) shows the messages and what to change.

## Reporting a difference

A program that translates but prints something else than Node.js is a bug. Report it with the program and both outputs on [GitHub](https://github.com/maxlestage/Solidify/issues): it becomes a test program once fixed.
