# Fonctionnement

Solidify traduit en trois passes : il analyse le programme, infère un type pour chaque valeur, puis génère le Rust. Chaque passe est déterministe : le même programme donne toujours le même Rust, et aucune passe ne devine.

## 1. Analyser

Un parseur écrit à la main lit le TypeScript moderne : classes, interfaces, enums, alias de types, génériques, fonctions fléchées, gabarits de chaînes, déstructuration, spread, chaînage optionnel, `async`/`await`, générateurs, étiquettes, expressions régulières. Le JavaScript est du TypeScript sans les annotations, et il est lu par le même parseur.

Quand le programme importe d'autres fichiers, ils sont lus aussi, dans l'ordre où Node.js les exécuterait : les fichiers importés d'abord. Une erreur de syntaxe arrête tout à cette étape, avec le fichier, la ligne et la colonne.

## 2. Inférer

Les annotations sont un point de départ, pas toute l'histoire : chaque valeur reçoit un type, de son annotation quand elle en a une, et de son usage partout ailleurs. Les types sont résolus ensemble, par unification, sur tout le programme.

- **Les nombres restent des entiers** (`i64`) jusqu'à ce qu'un littéral flottant, une division, `Math`, `parseFloat` ou du JSON dise le contraire ; alors le nombre, et tout nombre qui en reçoit la valeur, est un `f64`.
- **`T | null`** et les champs optionnels deviennent `Option<T>` ; **`A | B`** devient un enum.
- **Une fonction qui lève une erreur**, ou qui en appelle une qui le fait hors d'un `try`, renvoie `Result<T, String>`.
- **Un paramètre que rien ne contraint** devient un paramètre de type, avec les bornes dont ses usages ont besoin.
- **`any` et `unknown`** sont des valeurs connues à l'exécution (`JsValue`).

Ici, `total` n'additionne que des nombres entiers, tandis que `half` divise :

```ts
function total(values: number[]): number {
  let sum = 0;
  for (const v of values) {
    sum += v;
  }
  return sum;
}

function half(n: number): number {
  return n / 2;
}

console.log(total([1, 2, 3]), half(5));
```
```rust fragment
fn total(values: &[i64]) -> i64 {
    let mut sum = 0;
    for &v in values {
        sum += v;
    }
    sum
}

fn half(n: i64) -> f64 {
    n as f64 / 2.0
}

fn main() {
    println!("{} {}", total(&[1, 2, 3]), js_number_text(half(5)));
}

// …
```

`js_number_text`, l'une des fonctions utilitaires écrites après `main`, affiche `2.5` comme le fait JavaScript.

## 3. Générer

La dernière passe écrit le Rust, en décidant de la propriété de chaque valeur au fur et à mesure :

- **Les paramètres sont empruntés** (`&str`, `&[T]`, `&T`) sauf si la fonction les modifie (`&mut`) ou les conserve.
- **Une variable est déplacée à son dernier usage**, et clonée avant cela quand on en a encore besoin.
- **Les boucles** deviennent des intervalles (`for i in 1..=10`) et des itérateurs ; `map`, `filter`, `reduce` deviennent des chaînes d'itérateurs.
- **`switch`** devient `match` ; **`if (x !== null)`** devient `if let Some(x)` ; **`if (x === null) return`** devient `let Some(x) = x else { return }`.

```ts
function find(names: string[], prefix: string): string | null {
  for (const name of names) {
    if (name.startsWith(prefix)) {
      return name;
    }
  }
  return null;
}

const found = find(["Ada", "Grace", "Linus"], "Gr");
console.log(found ?? "nobody");
```
```rust
fn find(names: &[String], prefix: &str) -> Option<String> {
    for name in names {
        if name.starts_with(prefix) {
            return Some(name.clone());
        }
    }
    None
}

fn main() {
    let found = find(&["Ada".to_string(), "Grace".to_string(), "Linus".to_string()], "Gr");
    println!("{}", found.unwrap_or("nobody".to_string()));
}
```

## La sortie

- **Un seul fichier**, pour l'édition 2024 de Rust, que `rustc` compile seul : aucun crate à ajouter.
- **Les noms suivent les conventions de Rust :** `minMax` devient `min_max`, un `const line` de niveau supérieur devient `const LINE`, une variable qui porte le nom d'un mot-clé de Rust est échappée (`r#match`).
- **Les commentaires sont conservés**, là où ils étaient.
- **Les fonctions utilitaires viennent en dernier.** Ce qui manque à la bibliothèque standard de Rust (afficher les nombres comme JavaScript, `console.log` des objets, expressions régulières, dates, promesses et minuteries) est écrit après `main`, et seulement ce dont le programme se sert.
- **Un en-tête** nomme le fichier source ; `--no-header` l'omet.

La page [Types](types.md) passe en revue chaque sorte de valeur, et [Fonctions et propriété](ownership.md) l'emprunt, les déplacements et les closures.
