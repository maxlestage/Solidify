# Qué se traduce en qué

Cada construcción de TypeScript y el Rust en que se convierte, por tema. Cada línea está cubierta por los programas de prueba, traducidos, compilados y ejecutados frente a Node.js.

## Tipos

| TypeScript | Rust |
| --- | --- |
| `` number `` | i64, o f64 cuando interviene un decimal o el valor puede superar i64 |
| `` bigint, 10n ** 30n `` | JsBigInt, enteros de cualquier tamaño |
| `` string, template strings, tag`…` `` | String, &str para los parámetros, format!, una llamada a la etiqueta |
| `` boolean `` | bool |
| `` T[], [A, B] `` | Vec\<T>, &[T] para los parámetros, tuplas |
| `` T \| null, x?: T, ??, ?. `` | Option\<T>, if let, unwrap_or, map |
| `` interface, object literal `` | struct con Debug, Clone, PartialEq |
| `` enum, "a" \| "b" `` | enum con Display, match |
| `` A \| B, discriminated unions, typeof, "kind" in x `` | enum con una variante por miembro, match |
| `` A & B, keyof T, T["k"], typeof x, `id-${number}` `` | un struct con los campos de ambos, string, el tipo del campo, el tipo de la variable |
| `` Map, Set, Record<K, V>, { [k: string]: V } `` | HashMap, HashSet, o JsMap, JsSet en orden de inserción |
| `` any, unknown `` | JsValue: un valor conocido en tiempo de ejecución, con los operadores de JavaScript |
| `` recursive classes `` | Option\<Box\<T>> |

## Funciones

| TypeScript | Rust |
| --- | --- |
| `` function f(s: string, xs: T[], o: T) `` | fn f(s: &str, xs: &[T], o: &T), &mut cuando se modifican |
| `` function f<T>(x: T), f<number>(x) `` | fn f\<T: Clone + PartialEq>(x: T), límites deducidos del uso |
| `` (a, b) => a + b, callbacks `` | closures, impl Fn para los parámetros |
| `` f?.(), obj?.method(), fields of function type `` | if let Some(f) = …, .map(\|o\| o.method()), campos Rc\<dyn Fn(…)> |
| `` ...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) => `` | slices, vec!, literales de struct campo a campo, patrones anidados descompuestos |

## Clases e interfaces

| TypeScript | Rust |
| --- | --- |
| `` class, get, set, static, static { … } `` | struct, impl, new(), &self o &mut self, statics del módulo |
| `` const A = class { … }, function F() { this.x = … }, F.prototype.m `` | clases, reconstruidas como structs y bloques impl |
| `` class B extends A, abstract, super, instanceof `` | structs aplanados, un trait por clase base, Box\<dyn ALike> |
| `` interface Shape { area(): number }, implements `` | un trait por interfaz, impl para cada clase, Box\<dyn ShapeLike> |
| `` class Stack<T>, interface Page<T>, <T extends HasId> `` | struct Stack\<T> + impl\<T: …>, límites hallados en todo el programa |
| `` an object stored in two places, === `` | Shared\<T> (Rc\<RefCell\<T>>): un único objeto, comparado por identidad |

## Flujo de control

| TypeScript | Rust |
| --- | --- |
| `` for … of, for (;;), while, labels, for await `` | for x in &v, for i in a..b, loop, 'label, .await por elemento |
| `` switch, case without break `` | match con casos agrupados, bloques etiquetados para los casos encadenados |
| `` map, filter, reduce, find, some, forEach… `` | cadenas de iteradores |
| `` function*, yield, yield* `` | JsGenerator\<T>, for x in gen |

## Errores

| TypeScript | Rust |
| --- | --- |
| `` throw, try / catch / finally `` | Result\<T, String>, ?, una closure para el bloque try |
| `` class NotFound extends Error, instanceof in catch `` | errores que conservan su clase, sus campos, su mensaje y su nombre |

## Async

| TypeScript | Rust |
| --- | --- |
| `` async, await, new Promise `` | fn … -> Promise\<T>: una tarea lanzada de inmediato, .await |
| `` then, catch, finally, Promise.all, allSettled, race, any `` | métodos de Promise\<T>, microtareas en el orden de Node |
| `` setTimeout, setInterval `` | un bucle de eventos ejecutado por main, con los tiempos de Node |

## Módulos

| TypeScript | Rust |
| --- | --- |
| `` import, export, import * as ns, require, module.exports `` | un mod por archivo, pub para los exports |
| `` import fs from "node:fs", path, os, process, node_modules `` | módulos incluidos en Solidify, paquetes traducidos desde sus fuentes |
| `` top-level statements, const, let `` | main, const, static |

## La biblioteca

| TypeScript | Rust |
| --- | --- |
| `` console.log of objects, arrays, maps, sets `` | el texto exacto que imprime Node.js |
| `` /regex/flags, lookbehind, test, exec, match, replace, split `` | JsRegex, un motor con retroceso incluido en el archivo |
| `` JSON.stringify, JSON.parse, replacer, reviver `` | el JsValue del valor, JsFromJson para el tipo leído |
| `` new Date(…), getFullYear, toISOString, later - earlier `` | JsDate en milisegundos (UTC), PartialOrd |
| `` eval, new Function `` | el código integrado cuando se conoce, un pequeño intérprete si no |
| `` Proxy, Reflect, Symbol, arguments `` | JsProxy con sus trampas, JsSymbol, un parámetro rest |

Las páginas de [la traducción](how-it-works.md) muestran cada uno de estos casos con un ejemplo.
