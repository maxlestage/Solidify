# Introduction

Solidify lit un programme TypeScript ou JavaScript, fichiers importés compris, et écrit le Rust que vous auriez écrit à la main : structs, enums, `Option`, `Result`, itérateurs et propriété. La sortie est un seul fichier Rust ordinaire, sans machine virtuelle ni ramasse-miettes, et elle affiche ce qu'affiche Node.js.

## Ce qu'il fait

Donnez-lui ce TypeScript :

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

Chaque choix de cette sortie vient du programme lui-même :

- `Point` est une interface, il devient donc un `struct` ; le champ optionnel `label` devient un `Option<String>`.
- `x` et `y` ne contiennent jamais que des nombres entiers, ce sont donc des `i64`. Un littéral flottant, une division ou du JSON en auraient fait des `f64`.
- `describe` ne fait que lire son argument, il l'emprunte donc (`&Point`) au lieu de le prendre.
- `p.label !== undefined` est un test de l'`Option` : il devient `if let Some(label)`.
- Les instructions au niveau supérieur du fichier deviennent `main`.

## À quoi il sert

- **Passer un programme à Rust** sans le réécrire de zéro : traduisez-le, puis lisez et gardez le Rust.
- **Apprendre Rust à partir d'un code que vous connaissez :** écrivez ce que vous voulez dire en TypeScript et voyez comment Rust le dit.
- **Livrer un seul binaire** là où vous aviez un script Node.js : `rustc` compile la sortie, aucun environnement d'exécution à installer.

> [!NOTE]
> Solidify traduit des programmes, pas des bibliothèques pour le navigateur : il n'y a ni DOM, ni `fetch`, ni serveur `http`. Les [limites](limits.md) listent ce qui reste différent de Node.js, et ce qui est refusé avec un message.

## Comment c'est vérifié

Le dépôt contient 54 programmes de test. Chacun est traduit, compilé avec `rustc` et exécuté ; sa sortie doit être exactement ce qu'affiche Node.js pour le même programme. Chaque exemple de ces pages est vérifié de la même façon avec le transpileur : ce que vous lisez ici est ce qu'écrit Solidify.

## Comment lire ces pages

- **Démarrer :** [installez-le](installation.md), puis suivez le [démarrage rapide](quick-start.md), d'un fichier `.ts` jusqu'à un binaire qui tourne.
- **La traduction :** [le fonctionnement](how-it-works.md), puis une page par sujet : [types](types.md), [fonctions et propriété](ownership.md), [classes et interfaces](classes.md), [erreurs](errors.md), [async et minuteries](async.md), [modules et programmes](modules.md), et [la bibliothèque JavaScript](runtime.md) (`console.log`, chaînes, tableaux, JSON, expressions régulières, dates).
- **Référence :** la [ligne de commande](cli.md), la [bibliothèque Rust](library.md), le tableau complet de [ce qui se traduit en quoi](mapping.md), les [limites](limits.md) et la [FAQ](faq.md).

Appuyez sur `/` n'importe où sur le site pour chercher dans ces pages.
