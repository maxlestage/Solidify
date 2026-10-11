# Solidify

[English](README.md) · **Français**

**Écrivez du TypeScript ou du JavaScript, obtenez du Rust.** Solidify est un transpileur écrit en Rust, sans dépendance, qui lit un fichier TypeScript (ou JavaScript) et écrit le Rust que vous auriez écrit à la main : structs, enums, `Option`, `Result`, itérateurs et propriété. Ni machine virtuelle ni ramasse-miettes en sortie, un programme Rust ordinaire.

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
solidify app.ts -o app.rs      # un fichier en entrée, un fichier en sortie
solidify < app.ts > app.rs     # ou par l'entrée standard
solidify --check app.ts        # seulement signaler les erreurs
rustc --edition 2024 app.rs && ./app
```

Comme bibliothèque :

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

```rust
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

Les erreurs viennent avec une ligne, une colonne et la ligne fautive :

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
2. **Inférer.** Les types viennent des annotations et de l'usage, par unification. Les nombres restent des entiers (`i64`) tant qu'un littéral flottant ou une division ne dit pas le contraire (`f64`). `T | null` devient `Option<T>`. Une fonction qui lève une erreur, ou qui en appelle une qui le fait hors d'un `try`, renvoie `Result<T, String>`. Les paramètres que rien ne contraint deviennent des paramètres de type.
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
| `Map`, `Set`, `Record<K, V>` | `HashMap`, `HashSet` |
| `throw`, `try` / `catch` / `finally` | `Result<T, String>`, `?`, une closure pour le bloc `try` |
| `map`, `filter`, `reduce`, `find`, `some`… | chaînes d'itérateurs |
| `for … of`, `for (;;)`, `while`, étiquettes | `for x in &v`, `for i in a..b`, `loop`, `'label` |
| `switch` | `match`, avec regroupement des cas |
| `async`, `await` | `async fn`, `.await`, un petit exécuteur pour `main` |
| `function f<T>(x: T)` | `fn f<T: Clone + PartialEq>(x: T)`, bornes déduites de l'usage |
| `console.log`, `Math`, `parseInt`… | `println!`, méthodes de `f64`, petits utilitaires ajoutés au fichier |
| classes récursives | `Option<Box<T>>` |

Pas encore : l'héritage (`extends`), `any`, les expressions régulières, JSON, les minuteurs, les imports entre fichiers, les unions de types différents, les tests `typeof` et `instanceof`, les générateurs. Chacun est refusé avec un message qui dit quoi écrire à la place.

Quatre règles à connaître :

- **Les entiers d'abord.** `let n = 0` est un `i64`. Écrivez `2.0`, ou divisez, et toute la famille de ce nombre devient `f64`. `parseInt`, `parseFloat` et `Number()` renvoient un `f64`, parce qu'ils peuvent renvoyer `NaN`.
- **Null est `Option`.** Testez les valeurs nullables avec `!== null`, `!== undefined` ou `??` ; le Rust généré utilise `if let`, `let … else` et `unwrap_or`. Lire un champ d'une valeur qui peut être nulle est une erreur, comme en mode strict de TypeScript.
- **Throw est `Result`.** `throw new Error("…")` devient `return Err("…".to_string())` ; `catch (e)` reçoit le message dans une `String`.
- **Un fichier à la fois.** Les instructions de premier niveau deviennent `main` ; les `const` de premier niveau qui tiennent un littéral deviennent des `const` Rust ; les autres variables de premier niveau ne peuvent pas être lues depuis une fonction.

`console.log` d'un objet imprime la forme `Debug` de Rust, l'itération d'une `Map` suit l'ordre de `HashMap`, et `arr.sort()` sans comparateur trie les nombres comme des chaînes, comme en JavaScript.

## Dépôt

| Dossier | Contenu |
| --- | --- |
| [`crates/solidify`](crates/solidify) | Le transpileur : lexeur, parseur, inférence de types, génération de code, ligne de commande |
| [`crates/solidify/tests/cases`](crates/solidify/tests/cases) | Des programmes traduits, compilés avec `rustc` et comparés à ce que Node.js imprime |
| [`site`](site) | Le site web : un bac à sable qui exécute Solidify en WebAssembly, écrit avec [active](https://github.com/maxlestage/Active) |

## Développement

```sh
cargo test --workspace                     # le transpileur et le site
cargo run -p solidify -- crates/solidify/tests/cases/fib.ts
cargo run -p solidify-site                 # le site sur http://localhost:3000
```

La suite de tests traduit chaque `tests/cases/*.ts`, compare le résultat au `.rs` voisin, le compile avec `rustc` et l'exécute : la sortie doit correspondre au fichier `.out`, qui contient ce que Node.js imprime. `UPDATE_CASES=1 cargo test -p solidify --test cases` réécrit les fichiers attendus (les `.out` quand `node` est installé).

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

1. Sur GitHub, ouvrez **Settings → Pages** et réglez **Source** sur **GitHub Actions** (une fois).
2. Après chaque exécution verte de la CI sur `master`, le workflow [`pages.yml`](.github/workflows/pages.yml) exporte le site avec `solidify-site export` et le publie sur `https://maxlestage.github.io/Solidify/`. Pour publier tout de suite : **Actions → Pages → Run workflow**.

## Licence

Solidify est un logiciel propriétaire : © 2026 Maxime Nathan Lestage, tous droits réservés. Il n'est pas open source. Le code source est publié sur GitHub pour pouvoir être lu, audité et compilé, et vous pouvez l'utiliser librement comme dépendance de vos propres programmes, y compris commerciaux. Le modifier ou en redistribuer la source demande l'accord écrit de l'auteur. Texte complet : [LICENSE](LICENSE).

## Crédits

Conçu et développé par [Maxime Nathan Lestage](https://github.com/maxlestage), au sein d'une même famille de projets Rust : [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) et [Solidify](https://maxlestage.github.io/Solidify/).
