# Solidify

[English](README.md) · **Français**

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

```sh
cargo install --git https://github.com/maxlestage/Solidify solidify
solidify main.ts -o app.rs     # suit les imports de main.ts, un seul fichier Rust en sortie
solidify < app.ts > app.rs     # ou un fichier par l'entrée standard
solidify --check main.ts       # seulement signaler les erreurs
rustc --edition 2024 app.rs && ./app
```

Comme bibliothèque :

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
2. **Inférer.** Les types viennent des annotations et de l'usage, par unification. Les nombres restent des entiers (`i64`) tant qu'un littéral flottant, une division ou du JSON ne dit pas le contraire (`f64`). `T | null` devient `Option<T>`, `A | B` un enum. Une fonction qui lève une erreur, ou qui en appelle une qui le fait hors d'un `try`, renvoie `Result<T, String>`. Les paramètres que rien ne contraint deviennent des paramètres de type ; `any` est déduit de l'usage.
3. **Générer.** Chaque valeur reçoit une propriété : les paramètres sont empruntés (`&str`, `&[T]`, `&T`) sauf si la fonction les modifie ou les conserve ; une variable est déplacée à son dernier usage et clonée avant. Les boucles deviennent des intervalles et des itérateurs, `switch` devient `match`, `if (x !== null)` devient `if let Some(x)`, `if (x === null) return` devient `let Some(x) = x else { return }`.

## Ce qui se traduit, et en quoi

| TypeScript | Rust |
| --- | --- |
| `number` | `i64`, ou `f64` dès qu'un flottant intervient |
| `string`, gabarits de chaînes | `String`, `&str` pour les paramètres, `format!` |
| `T[]`, `[A, B]` | `Vec<T>`, `&[T]` pour les paramètres, tuples |
| `T \| null`, `x?: T`, `??`, `?.` | `Option<T>`, `if let`, `unwrap_or`, `map` |
| `interface`, objets littéraux | `struct` avec `Debug`, `Clone`, `PartialEq` |
| `class`, `get`, `set`, `static` | `struct` + `impl`, `new()`, `&self` ou `&mut self`, consts |
| `enum`, `"a" \| "b"` | `enum` avec `Display`, `match` |
| `Map`, `Set`, `Record<K, V>` | `HashMap`, `HashSet`, ou `JsMap`, `JsSet` quand le programme observe l'ordre d'insertion |
| `throw`, `try` / `catch` / `finally` | `Result<T, String>`, `?`, une closure pour le bloc `try` |
| `map`, `filter`, `reduce`, `find`, `some`… | chaînes d'itérateurs |
| `for … of`, `for (;;)`, `while`, étiquettes | `for x in &v`, `for i in a..b`, `loop`, `'label` |
| `switch` | `match`, avec regroupement des cas |
| `async`, `await` | `async fn`, `.await`, un petit exécuteur pour `main` |
| `function f<T>(x: T)`, `f<number>(x)` | `fn f<T: Clone + PartialEq>(x: T)`, bornes déduites de l'usage |
| `class Stack<T>`, `interface Page<T>`, `type Entry<K, V>`, `extends Stack<number>`, `<T extends HasId>` | `struct Stack<T>` + `impl<T: …>`, bornes trouvées dans tout le programme, un trait `HasIdLike` pour les contraintes |
| `interface Listener<T> { notify(v: T): void }`, `abstract class Shape<T>` | `trait ListenerLike<T>`, `Box<dyn ListenerLike<String>>` |
| méthodes qui renvoient `this`, `a.add(1).add(2)` | `&mut Self` : les appels s'enchaînent sur un même objet |
| `enum Level { … }`, `Level[Level.High]` | `enum` avec `Display`, `Level::High.name()` |
| `console.log`, `Math`, `parseInt`… | `println!`, méthodes de `f64`, petits utilitaires ajoutés au fichier |
| classes récursives | `Option<Box<T>>` |
| `class B extends A`, `abstract`, `super`, `protected`, `instanceof` | structs aplaties, un trait par classe de base, `Box<dyn ALike>` là où un `A` est attendu |
| `interface Shape { area(): number }`, `class C implements Shape`, `interface A extends B` | un trait par interface, `impl ShapeLike for C`, `Box<dyn ShapeLike>` / `&dyn ShapeLike` là où un `Shape` est attendu |
| `A \| B`, `[1, "two"]`, unions discriminées, `typeof x === "string"`, `"kind" in x` | `enum` avec une variante par membre, `match` |
| `function*`, `yield`, `yield*` | `JsGenerator<T>`, `for x in gen` |
| `setTimeout`, `setInterval`, `clearTimeout`, `new Promise`, `Promise.all` | une boucle d'événements lancée par `main`, `JsPromise` |
| `/regex/flags`, `new RegExp`, `test`, `exec`, `match`, `matchAll`, `replace`, `split`, `search`, `lastIndex`, `while ((m = re.exec(s)) !== null)` | `JsRegex`, un moteur à retour arrière livré dans le fichier généré, `while let Some(m) = re.exec(&s)` |
| `JSON.stringify(v, null, 2)`, `JSON.parse` vers un type annoté | la `JsValue` de la valeur, un `JsFromJson` généré pour le type lu |
| `import`, `export`, `export default`, `import * as ns`, `export … from` | un `mod` par fichier importé, `pub` pour les exports |
| `console.log` d'objets, tableaux, maps, sets, valeurs imbriquées | le texte exact qu'imprime Node.js (règles de `util.inspect`) |
| `any`, `unknown` | déduits de l'usage |
| paramètres `...rest`, `f(...xs)`, `{ ...base, z }`, `[a, ...rest] = xs`, `({ x, y }: Point) =>` | slices, `vec!`, littéraux de struct construits champ par champ, `let rest = xs[1..].to_vec()` |
| `parseInt` | `Option<i64>` (`NaN` est `None`), affiné par `isNaN` |
| `new Date(…)`, `getTime`, `getFullYear`…, `toISOString`, `plus_tard - plus_tôt`, `Date.now()` | `JsDate` (millisecondes, UTC), `PartialOrd`, différences de `get_time()` |
| `f?.()`, `obj?.method()`, champs de type fonction, `onClick?: () => void` | `if let Some(f) = …`, `.map(\|o\| o.method())`, champs `Rc<dyn Fn(…)>` |

Ce qui reste différent, parce que les valeurs Rust ne sont pas des références JavaScript : un objet rangé à deux endroits est copié, et modifier une copie ne touche pas l'autre (les variables modifiées dans des closures qui survivent à leur portée sont partagées via `Rc<RefCell<T>>`). Les nombres sont des `i64` tant qu'un flottant n'intervient pas : un entier qui déborde fait paniquer là où JavaScript perdrait de la précision. Les imports vont entre vos propres fichiers, pas vers des paquets. Les replacers et revivers de `JSON`, `Symbol`, `Proxy`, `eval`, la manipulation des prototypes et `arguments` n'ont pas d'équivalent Rust et sont refusés avec un message qui dit quoi écrire à la place.

Quatre règles à connaître :

- **Les entiers d'abord.** `let n = 0` est un `i64`. Écrivez `2.0`, divisez, ou lisez le nombre depuis du JSON, et toute la famille de ce nombre devient `f64`. `parseFloat` et `Number()` renvoient un `f64`, parce qu'ils peuvent renvoyer `NaN` ; `parseInt` renvoie un `Option<i64>`.
- **Null est `Option`.** Testez les valeurs nullables avec `!== null`, `!== undefined` ou `??` ; le Rust généré utilise `if let`, `let … else` et `unwrap_or`. Lire un champ d'une valeur qui peut être nulle est une erreur, comme en mode strict de TypeScript. `console.log` imprime `undefined` ou `null` comme Node.js, selon d'où vient la valeur absente.
- **Throw est `Result`.** `throw new Error("…")` devient `return Err("…".to_string())` ; `catch (e)` reçoit le message dans une `String`. `JSON.parse` lève les mêmes messages de `SyntaxError` que V8.
- **Les fichiers sont des modules.** Les instructions de premier niveau deviennent `main`, fichiers importés d'abord, comme Node.js les exécute ; les `const` de premier niveau qui tiennent un littéral deviennent des `const` Rust ; les autres variables de premier niveau lues depuis une fonction deviennent des `static` (`LazyLock`, avec un `Mutex` quand elles changent). Chaque fichier importé devient un `mod`, ses exports `pub` ; les fichiers partagent un même espace de noms, deux fichiers ne peuvent donc pas déclarer le même nom de premier niveau.

`console.log` imprime les objets, tableaux, maps et sets exactement comme Node.js, `Map` et `Set` gardent leur ordre d'insertion quand le programme l'observe, et `arr.sort()` sans comparateur trie les nombres comme des chaînes, comme en JavaScript.

## Dépôt

| Dossier | Contenu |
| --- | --- |
| [`crates/solidify`](crates/solidify) | Le transpileur : lexeur, parseur, inférence de types, génération de code, ligne de commande |
| [`crates/solidify/tests/cases`](crates/solidify/tests/cases) | 33 programmes traduits, compilés avec `rustc` et comparés à ce que Node.js imprime (un dossier contient un programme de plusieurs fichiers) |
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

## Licence

Solidify est un logiciel propriétaire : © 2026 Maxime Nathan Lestage, tous droits réservés. Il n'est pas open source. Le code source est publié sur GitHub pour pouvoir être lu, audité et compilé, et vous pouvez l'utiliser librement comme dépendance de vos propres programmes, y compris commerciaux. Le modifier ou en redistribuer la source demande l'accord écrit de l'auteur. Texte complet : [LICENSE](LICENSE).

## Crédits

Conçu et développé par [Maxime Nathan Lestage](https://github.com/maxlestage), au sein d'une même famille de projets Rust : [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) et [Solidify](https://maxlestage.github.io/Solidify/).
