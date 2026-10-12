# Ce qui se traduit en quoi

Chaque construction de TypeScript et le Rust qu'elle devient, par sujet. Chaque ligne est couverte par les programmes de test, traduits, compilés et exécutés contre Node.js.

## Types

| TypeScript | Rust |
| --- | --- |
| `` number `` | i64, ou f64 dès qu'un flottant intervient ou que la valeur peut dépasser i64 |
| `` bigint, 10n ** 30n `` | JsBigInt, entiers de toute taille |
| `` string, template strings, tag`…` `` | String, &str pour les paramètres, format!, un appel du tag |
| `` boolean `` | bool |
| `` T[], [A, B] `` | Vec\<T>, &[T] pour les paramètres, tuples |
| `` T \| null, x?: T, ??, ?. `` | Option\<T>, if let, unwrap_or, map |
| `` interface, object literal `` | struct avec Debug, Clone, PartialEq |
| `` enum, "a" \| "b" `` | enum avec Display, match |
| `` A \| B, discriminated unions, typeof, "kind" in x `` | enum avec une variante par membre, match |
| `` A & B, keyof T, T["k"], typeof x, `id-${number}` `` | un seul struct avec les champs des deux, string, le type du champ, le type de la variable |
| `` Map, Set, Record<K, V>, { [k: string]: V } `` | HashMap, HashSet, ou JsMap, JsSet dans l'ordre d'insertion |
| `` any, unknown `` | JsValue : une valeur connue à l'exécution, avec les opérateurs de JavaScript |
| `` recursive classes `` | Option\<Box\<T>> |

## Fonctions

| TypeScript | Rust |
| --- | --- |
| `` function f(s: string, xs: T[], o: T) `` | fn f(s: &str, xs: &[T], o: &T), &mut quand ils sont modifiés |
| `` function f<T>(x: T), f<number>(x) `` | fn f\<T: Clone + PartialEq>(x: T), bornes déduites de l'usage |
| `` (a, b) => a + b, callbacks `` | closures, impl Fn pour les paramètres |
| `` f?.(), obj?.method(), fields of function type `` | if let Some(f) = …, .map(\|o\| o.method()), champs Rc\<dyn Fn(…)> |
| `` ...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) => `` | slices, vec!, littéraux de struct champ par champ, motifs imbriqués décomposés |

## Classes et interfaces

| TypeScript | Rust |
| --- | --- |
| `` class, get, set, static, static { … } `` | struct, impl, new(), &self ou &mut self, statics du module |
| `` const A = class { … }, function F() { this.x = … }, F.prototype.m `` | des classes, reconstruites en structs et blocs impl |
| `` class B extends A, abstract, super, instanceof `` | structs aplatis, un trait par classe de base, Box\<dyn ALike> |
| `` interface Shape { area(): number }, implements `` | un trait par interface, impl pour chaque classe, Box\<dyn ShapeLike> |
| `` class Stack<T>, interface Page<T>, <T extends HasId> `` | struct Stack\<T> + impl\<T: …>, bornes trouvées dans tout le programme |
| `` an object stored in two places, === `` | Shared\<T> (Rc\<RefCell\<T>>) : un seul objet, comparé par identité |

## Flux de contrôle

| TypeScript | Rust |
| --- | --- |
| `` for … of, for (;;), while, labels, for await `` | for x in &v, for i in a..b, loop, 'label, .await par élément |
| `` switch, case without break `` | match avec cas groupés, blocs étiquetés pour les cas qui s'enchaînent |
| `` map, filter, reduce, find, some, forEach… `` | chaînes d'itérateurs |
| `` function*, yield, yield* `` | JsGenerator\<T>, for x in gen |

## Erreurs

| TypeScript | Rust |
| --- | --- |
| `` throw, try / catch / finally `` | Result\<T, String>, ?, une closure pour le bloc try |
| `` class NotFound extends Error, instanceof in catch `` | des erreurs qui gardent leur classe, leurs champs, leur message et leur nom |

## Async

| TypeScript | Rust |
| --- | --- |
| `` async, await, new Promise `` | fn … -> Promise\<T> : une tâche lancée tout de suite, .await |
| `` then, catch, finally, Promise.all, allSettled, race, any `` | méthodes de Promise\<T>, microtâches dans l'ordre de Node |
| `` setTimeout, setInterval `` | une boucle d'événements lancée par main, aux délais de Node |

## Modules

| TypeScript | Rust |
| --- | --- |
| `` import, export, import * as ns, require, module.exports `` | un mod par fichier, pub pour les exports |
| `` import fs from "node:fs", path, os, process, node_modules `` | modules livrés avec Solidify, paquets traduits depuis leurs sources |
| `` top-level statements, const, let `` | main, const, static |

## La bibliothèque

| TypeScript | Rust |
| --- | --- |
| `` console.log of objects, arrays, maps, sets `` | le texte exact qu'affiche Node.js |
| `` /regex/flags, lookbehind, test, exec, match, replace, split `` | JsRegex, un moteur à retour arrière livré dans le fichier |
| `` JSON.stringify, JSON.parse, replacer, reviver `` | le JsValue de la valeur, JsFromJson pour le type lu |
| `` new Date(…), getFullYear, toISOString, later - earlier `` | JsDate en millisecondes (UTC), PartialOrd |
| `` eval, new Function `` | le code intégré quand il est connu, un petit interpréteur sinon |
| `` Proxy, Reflect, Symbol, arguments `` | JsProxy et ses pièges, JsSymbol, un paramètre rest |

Les pages de [la traduction](how-it-works.md) montrent chacune de ces lignes avec un exemple.
