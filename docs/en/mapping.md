# What translates to what

Every construct of TypeScript and the Rust it becomes, by subject. Each line is covered by the test programs, translated, compiled and run against Node.js.

## Types

| TypeScript | Rust |
| --- | --- |
| `` number `` | i64, or f64 when a float is involved or the value can outgrow i64 |
| `` bigint, 10n ** 30n `` | JsBigInt, integers of any size |
| `` string, template strings, tag`…` `` | String, &str for parameters, format!, a call of the tag |
| `` boolean `` | bool |
| `` T[], [A, B] `` | Vec\<T>, &[T] for parameters, tuples |
| `` T \| null, x?: T, ??, ?. `` | Option\<T>, if let, unwrap_or, map |
| `` interface, object literal `` | struct with Debug, Clone, PartialEq |
| `` enum, "a" \| "b" `` | enum with Display, match |
| `` A \| B, discriminated unions, typeof, "kind" in x `` | enum with one variant per member, match |
| `` A & B, keyof T, T["k"], typeof x, `id-${number}` `` | one struct with the fields of both, string, the field's type, the variable's type |
| `` Map, Set, Record<K, V>, { [k: string]: V } `` | HashMap, HashSet, or JsMap, JsSet in insertion order |
| `` any, unknown `` | JsValue: a value known at run time, with JavaScript's operators |
| `` recursive classes `` | Option\<Box\<T>> |

## Functions

| TypeScript | Rust |
| --- | --- |
| `` function f(s: string, xs: T[], o: T) `` | fn f(s: &str, xs: &[T], o: &T), &mut when they change |
| `` function f<T>(x: T), f<number>(x) `` | fn f\<T: Clone + PartialEq>(x: T), bounds from use |
| `` (a, b) => a + b, callbacks `` | closures, impl Fn for parameters |
| `` f?.(), obj?.method(), fields of function type `` | if let Some(f) = …, .map(\|o\| o.method()), Rc\<dyn Fn(…)> fields |
| `` ...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) => `` | slices, vec!, struct literals field by field, nested patterns taken apart |

## Classes and interfaces

| TypeScript | Rust |
| --- | --- |
| `` class, get, set, static, static { … } `` | struct, impl, new(), &self or &mut self, statics of the module |
| `` const A = class { … }, function F() { this.x = … }, F.prototype.m `` | classes, rebuilt as structs and impl blocks |
| `` class B extends A, abstract, super, instanceof `` | flattened structs, a trait per base class, Box\<dyn ALike> |
| `` interface Shape { area(): number }, implements `` | a trait per interface, impl for each class, Box\<dyn ShapeLike> |
| `` class Stack<T>, interface Page<T>, <T extends HasId> `` | struct Stack\<T> + impl\<T: …>, bounds found across the program |
| `` an object stored in two places, === `` | Shared\<T> (Rc\<RefCell\<T>>): one object, compared by identity |

## Control flow

| TypeScript | Rust |
| --- | --- |
| `` for … of, for (;;), while, labels, for await `` | for x in &v, for i in a..b, loop, 'label, .await per item |
| `` switch, case without break `` | match with grouped cases, labeled blocks for the fallthrough |
| `` map, filter, reduce, find, some, forEach… `` | iterator chains |
| `` function*, yield, yield* `` | JsGenerator\<T>, for x in gen |

## Errors

| TypeScript | Rust |
| --- | --- |
| `` throw, try / catch / finally `` | Result\<T, String>, ?, a closure for the try block |
| `` class NotFound extends Error, instanceof in catch `` | errors that keep their class, fields, message and name |

## Async

| TypeScript | Rust |
| --- | --- |
| `` async, await, new Promise `` | fn … -> Promise\<T>: a task started at once, .await |
| `` then, catch, finally, Promise.all, allSettled, race, any `` | methods of Promise\<T>, microtasks in Node's order |
| `` setTimeout, setInterval `` | an event loop run by main, with Node's timings |

## Modules

| TypeScript | Rust |
| --- | --- |
| `` import, export, import * as ns, require, module.exports `` | one mod per file, pub for exports |
| `` import fs from "node:fs", path, os, process, node_modules `` | modules shipped with Solidify, packages translated from their sources |
| `` top-level statements, const, let `` | main, const, static |

## The library

| TypeScript | Rust |
| --- | --- |
| `` console.log of objects, arrays, maps, sets `` | the exact text Node.js prints |
| `` /regex/flags, lookbehind, test, exec, match, replace, split `` | JsRegex, a backtracking engine shipped in the file |
| `` JSON.stringify, JSON.parse, replacer, reviver `` | the value's JsValue, JsFromJson for the parsed type |
| `` new Date(…), getFullYear, toISOString, later - earlier `` | JsDate in milliseconds (UTC), PartialOrd |
| `` eval, new Function `` | the code inlined when it is known, a small interpreter otherwise |
| `` Proxy, Reflect, Symbol, arguments `` | JsProxy with its traps, JsSymbol, a rest parameter |

The pages of [the translation](how-it-works.md) show each of these with an example.
