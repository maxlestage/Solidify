# Solidify

[English](README.md) · **Français** · [Español](README.es.md)

**Écrivez du TypeScript ou du JavaScript, obtenez du Rust.** Solidify est un transpileur écrit en Rust, sans dépendance, qui lit un programme TypeScript (ou JavaScript), fichiers importés compris, et écrit le Rust que vous auriez écrit à la main : structs, enums, `Option`, `Result`, itérateurs et propriété. Ni machine virtuelle ni ramasse-miettes en sortie, un programme Rust ordinaire dont la sortie est celle de Node.js.

```ts
interface Point { x: number; y: number; label?: string }

function describe(p: Point): string {
  if (p.label !== undefined) {
    return p.label;
  }
  return `(${p.x}, ${p.y})`;
}

const points: Point[] = [{ x: 1, y: 2 }, { x: 3, y: 4, label: "far" }];
for (const p of points) {
  console.log(describe(p));
}
```

devient

```rust
#[derive(Debug, Clone, PartialEq)]
struct Point {
    pub x: i64,
    pub y: i64,
    pub label: Option<String>,
}

fn describe(p: &Point) -> String {
    if let Some(label) = &p.label {
        return label.clone();
    }
    format!("({}, {})", p.x, p.y)
}

fn main() {
    let points = vec![Point { x: 1, y: 2, label: None }, Point { x: 3, y: 4, label: Some("far".to_string()) }];
    for p in &points {
        println!("{}", describe(p));
    }
}
```

📚 **Site web, avec un bac à sable qui exécute Solidify dans le navigateur** : le site dans [`site/`](site), lui-même écrit avec [active](https://github.com/maxlestage/Active) et publié sur https://maxlestage.github.io/Solidify/fr/. Lancez-le avec `cargo run -p solidify-site`.

## Installer

Rien à installer d'autre que Node.js 20 ou plus récent : `npx` lance Solidify, compilé en WebAssembly, sur vos fichiers.

```sh
npx solidify-rs main.ts -o app.rs     # suit les imports de main.ts, un seul fichier Rust en sortie
npx solidify-rs < app.ts > app.rs     # ou un fichier par l'entrée standard
npx solidify-rs --check main.ts       # signale seulement les erreurs
rustc --edition 2024 app.rs && ./app
```

Le site est aussi une application : ouvrez-le dans Chrome, Edge ou Safari et installez-le (le bouton **Installer l'app**, ou *Sur l'écran d'accueil*) ; le bac à sable marche alors hors ligne.

Comme bibliothèque Rust :

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

```rust
let options = solidify::Options::default();
let rust: String = solidify::transpile(typescript, &options)?;
// Un programme de plusieurs fichiers, depuis son fichier d'entrée :
let rust: String = solidify::transpile_file(Path::new("main.ts"), &options)?;
```

Les erreurs viennent avec le fichier, une ligne, une colonne et la ligne fautive :

```text
error: `x` may be null here (number | null): test it with `if (… !== null)`, or use `?.` or `!`
  --> app.ts:4:15
   |
  4|   return x.toFixed(2);
   |          ^^^^^^^^^
```

## Fonctionnement

Trois passes, toutes déterministes :

1. **Analyser.** Un parseur écrit à la main lit le TypeScript moderne : classes, interfaces, enums, fonctions fléchées, gabarits de chaînes, déstructuration, async/await. Le JavaScript est du TypeScript sans annotations.
2. **Inférer.** Les types viennent des annotations et de l'usage, par unification. Les nombres restent des entiers (`i64`) tant qu'un littéral flottant, une division ou du JSON ne dit pas le contraire (`f64`). `T | null` devient `Option<T>`, `A | B` un enum. Une fonction qui lève une erreur, ou qui en appelle une qui le fait hors d'un `try`, renvoie `Result<T, String>`. Les paramètres que rien ne contraint deviennent des paramètres de type ; `any` et `unknown` sont des valeurs connues à l'exécution (`JsValue`).
3. **Générer.** Chaque valeur reçoit une propriété : les paramètres sont empruntés (`&str`, `&[T]`, `&T`) sauf si la fonction les modifie ou les conserve ; une variable est déplacée à son dernier usage et clonée avant. Les boucles deviennent des intervalles et des itérateurs, `switch` devient `match`, `if (x !== null)` devient `if let Some(x)`, `if (x === null) return` devient `let Some(x) = x else { return }`.

## Ce qui se traduit, et en quoi

| TypeScript | Rust |
| --- | --- |
| `` number `` | i64, ou f64 dès qu'un flottant intervient ou que la valeur peut dépasser i64 |
| `` bigint, 10n ** 30n `` | JsBigInt, entiers de toute taille |
| `` string, template strings, tag`…` `` | String, &str en paramètre, format!, un appel du tag |
| `` T[], [A, B] `` | Vec\<T>, &[T] en paramètre, tuples |
| `` T \| null, x?: T, ??, ?. `` | Option\<T>, if let, unwrap_or, map |
| `` interface, object literal `` | struct avec Debug, Clone, PartialEq |
| `` an object stored in two places, === `` | Shared\<T> (Rc\<RefCell\<T>>) : un seul objet, comparé par identité |
| `` class, get, set, static, static { … } `` | struct, impl, new(), &self ou &mut self, statics du module |
| `` const A = class { … }, function F() { this.x = … }, F.prototype.m `` | des classes, reconstruites en structs et blocs impl |
| `` enum, "a" \| "b" `` | enum avec Display, match |
| `` Map, Set, Record<K, V>, { [k: string]: V } `` | HashMap, HashSet, ou JsMap, JsSet dans l'ordre d'insertion |
| `` A & B, keyof T, T["k"], typeof x, `id-${number}` `` | un struct avec les champs des deux, string, le type du champ, celui de la variable |
| `` throw, try / catch / finally `` | Result\<T, String>, ?, une closure pour le bloc try |
| `` class NotFound extends Error, instanceof in catch `` | des erreurs qui gardent leur classe, leurs champs, leur message et leur nom |
| `` map, filter, reduce, find, some, forEach… `` | chaînes d'itérateurs |
| `` for … of, for (;;), while, labels, for await `` | for x in &v, for i in a..b, loop, 'label, .await par élément |
| `` switch, case without break `` | match avec cas groupés, blocs étiquetés pour les cas qui s'enchaînent |
| `` async, await, new Promise `` | fn … -> Promise\<T> : une tâche lancée tout de suite, .await |
| `` then, catch, finally, Promise.all, allSettled, race, any `` | méthodes de Promise\<T>, microtâches dans l'ordre de Node |
| `` setTimeout, setInterval `` | une boucle d'événements lancée par main, aux délais de Node |
| `` function f<T>(x: T), f<number>(x) `` | fn f\<T: Clone + PartialEq>(x: T), bornes déduites de l'usage |
| `` class Stack<T>, interface Page<T>, <T extends HasId> `` | struct Stack\<T> + impl\<T: …>, bornes trouvées dans tout le programme |
| `` class B extends A, abstract, super, instanceof `` | structs aplatis, un trait par classe de base, Box\<dyn ALike> |
| `` interface Shape { area(): number }, implements `` | un trait par interface, impl pour chaque classe, Box\<dyn ShapeLike> |
| `` A \| B, discriminated unions, typeof, "kind" in x `` | enum avec une variante par membre, match |
| `` any, unknown `` | JsValue : une valeur connue à l'exécution, avec les opérateurs de JavaScript |
| `` eval, new Function `` | le code intégré quand il est connu, un petit interpréteur sinon |
| `` Proxy, Reflect, Symbol, arguments `` | JsProxy et ses pièges, JsSymbol, un paramètre rest |
| `` function*, yield, yield* `` | JsGenerator\<T>, for x in gen |
| `` /regex/flags, lookbehind, test, exec, match, replace, split `` | JsRegex, un moteur à retour arrière livré dans le fichier |
| `` JSON.stringify, JSON.parse, replacer, reviver `` | le JsValue de la valeur, JsFromJson pour le type lu |
| `` import, export, import * as ns, require, module.exports `` | un mod par fichier, pub pour les exports |
| `` import fs from "node:fs", path, os, process, node_modules `` | modules fournis avec Solidify, paquets traduits depuis leurs sources |
| `` console.log of objects, arrays, maps, sets `` | le texte exact qu'affiche Node.js |
| `` ...rest, f(...xs), { ...base, z }, { a, ...rest }, ({ a: { b } }) => `` | slices, vec!, littéraux de struct champ par champ, motifs imbriqués décomposés |
| `` new Date(…), getFullYear, toISOString, later - earlier `` | JsDate en millisecondes (UTC), PartialOrd |
| `` f?.(), obj?.method(), fields of function type `` | if let Some(f) = …, .map(\|o\| o.method()), champs Rc\<dyn Fn(…)> |
| `` recursive classes `` | Option\<Box\<T>> |

**Ce qui reste différent.** Le programme généré affiche ce qu'affiche Node.js ; quelques points ne peuvent pas être identiques. Une erreur affichée en entier donne `Error: message`, sans la pile d'appels de JavaScript. Les minuteries suivent l'horloge réelle : deux minuteries à quelques millisecondes d'écart peuvent s'inverser sous forte charge, comme dans Node. Le code construit à l'exécution pour `eval` ou `new Function` tourne dans un petit interpréteur livré dans le fichier, plus lent que le code traduit. Parmi les modules propres à Node, `fs`, `fs/promises`, `path`, `os` et `process` sont là ; les paquets de `node_modules` sont traduits depuis leurs sources, donc les modules natifs et des modules comme `http` ou `child_process` sont refusés, comme les générateurs asynchrones (`async function*`), avec un message qui dit pourquoi.

Quatre règles à connaître :

- **Les entiers d'abord.** `let n = 0` est un `i64`. Écrivez `2.0`, divisez, ou lisez le nombre depuis du JSON, et toute la famille de ce nombre devient `f64`. `parseFloat` et `Number()` renvoient un `f64`, parce qu'ils peuvent renvoyer `NaN` ; `parseInt` renvoie un `Option<i64>`.
- **Null est `Option`.** Testez les valeurs nullables avec `!== null`, `!== undefined` ou `??` ; le Rust généré utilise `if let`, `let … else` et `unwrap_or`. Lire un champ d'une valeur qui peut être nulle est une erreur, comme en mode strict de TypeScript. `console.log` imprime `undefined` ou `null` comme Node.js, selon d'où vient la valeur absente.
- **Throw est `Result`.** `throw new Error("…")` devient `return Err("…".to_string())` ; `catch (e)` reçoit le message dans une `String`. `JSON.parse` lève les mêmes messages de `SyntaxError` que V8.
- **Les fichiers sont des modules.** Les instructions de premier niveau deviennent `main`, fichiers importés d'abord, comme Node.js les exécute ; les `const` de premier niveau qui tiennent un littéral deviennent des `const` Rust ; les autres variables de premier niveau lues depuis une fonction deviennent des `static` (`LazyLock`, lues et écrites sur place quand elles changent). Chaque fichier importé devient un `mod`, ses exports `pub` ; un nom déclaré par deux fichiers est renommé dans l'un d'eux.

`console.log` imprime les objets, tableaux, maps et sets exactement comme Node.js, `Map` et `Set` gardent leur ordre d'insertion quand le programme l'observe, et `arr.sort()` sans comparateur trie les nombres comme des chaînes, comme en JavaScript.

## Dépôt

| Dossier | Contenu |
| --- | --- |
| [`crates/solidify`](crates/solidify) | Le transpileur : lexeur, parseur, inférence de types, génération de code, ligne de commande |
| [`crates/solidify/tests/cases`](crates/solidify/tests/cases) | 54 programmes traduits, compilés avec `rustc` et comparés à ce que Node.js imprime (un dossier contient un programme de plusieurs fichiers) |
| [`npm`](npm) | Le paquet `solidify-rs` pour `npx` : un lanceur Node.js du transpileur compilé en WebAssembly (WASI), vérifié contre les programmes de test par `node npm/test.js` |
| [`site`](site) | Le site web : un bac à sable qui exécute Solidify en WebAssembly, écrit avec [active](https://github.com/maxlestage/Active) |

## Développement

```sh
cargo test --workspace                     # le transpileur et le site
cargo run -p solidify -- crates/solidify/tests/cases/fib.ts
cargo run -p solidify-site                 # le site sur http://localhost:3000
```

La suite de tests traduit chaque `tests/cases/*.ts` (et chaque `tests/cases/*/main.ts` avec les fichiers qu'il importe), compare le résultat au `.rs` voisin, le compile avec `rustc` et l'exécute : la sortie doit correspondre au fichier `.out`, qui contient ce que Node.js imprime. `UPDATE_CASES=1 cargo test -p solidify --test cases` réécrit les fichiers attendus (les `.out` quand `node` est installé).

Le bundle navigateur du site se construit ainsi (le [`Dockerfile`](Dockerfile) et [`ci.yml`](.github/workflows/ci.yml) font pareil) :

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p solidify-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/solidify_site.wasm
cargo run -p solidify-site                 # http://localhost:3000
cargo run -p solidify-site -- export dist  # des fichiers statiques pour n'importe quel hébergeur
```

## Publier le site sur GitHub Pages

Chaque branche dont la CI est verte est fusionnée dans `master` par le workflow [`ci.yml`](.github/workflows/ci.yml) (sauf les branches nommées `wip/…`). Après chaque exécution verte de la CI sur `master`, le workflow [`pages.yml`](.github/workflows/pages.yml) active GitHub Pages si besoin, exporte le site avec `solidify-site export` et le publie sur `https://maxlestage.github.io/Solidify/`. Pour publier tout de suite : **Actions → Pages → Run workflow**.

## Publier sur npm

Le workflow [`npm.yml`](.github/workflows/npm.yml) construit le paquet après chaque fusion dans `master` et le publie quand sa version (dans [`npm/package.json`](npm/package.json)) n'est pas encore sur npm. Il lui faut un jeton npm dans le secret du dépôt `NPM_TOKEN` (**Settings → Secrets and variables → Actions**). Pour essayer le paquet en local :

```sh
cargo build -p solidify --bin solidify --target wasm32-wasip1 --profile wasm-release
cp target/wasm32-wasip1/wasm-release/solidify.wasm LICENSE npm/
node npm/test.js                     # chaque programme de test, comme le traduit la version native
node npm/bin/solidify.js main.ts     # ce que lance `npx solidify-rs main.ts`
```

## Licence

Solidify est un logiciel propriétaire : © 2026 Maxime Nathan Lestage, tous droits réservés. Il n'est pas open source. Le code source est publié sur GitHub pour pouvoir être lu, audité et compilé, et vous pouvez l'utiliser librement comme dépendance de vos propres programmes, y compris commerciaux. Le modifier ou en redistribuer la source demande l'accord écrit de l'auteur. Texte complet : [LICENSE](LICENSE).

## Crédits

Conçu et développé par [Maxime Nathan Lestage](https://github.com/maxlestage), au sein d'une même famille de projets Rust : [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) et [Solidify](https://maxlestage.github.io/Solidify/).
