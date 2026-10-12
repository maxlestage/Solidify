# FAQ

Des réponses courtes aux questions que l'on pose sur Solidify.

## La sortie est-elle du Rust lisible ?

Oui : elle est faite pour être lue et gardée. Les structs, les enums, `Option`, `Result`, les paramètres empruntés et les chaînes d'itérateurs sont ce que vous écririez à la main ; les noms suivent les conventions de Rust et les commentaires sont conservés. Les fonctions utilitaires écrites après `main` sont elles aussi de simples fonctions Rust.

## La sortie a-t-elle besoin d'un environnement d'exécution ou d'un crate ?

Non. La sortie est un seul fichier que `rustc` compile seul, pour l'édition 2024. Ce qui manque à la bibliothèque standard de Rust (afficher les nombres comme JavaScript, expressions régulières, dates, promesses et minuteries) est écrit dans le fichier, et seulement ce dont le programme se sert.

## Le programme traduit est-il plus rapide ?

En général, oui : il est compilé à l'avance, sans ramasse-miettes, et les nombres qui restent entiers sont des entiers machine. Le code construit à l'exécution avec `eval` tourne dans un interpréteur, et reste lent.

## Puis-je modifier la sortie ?

Oui. C'est votre programme, en Rust : traduisez une fois, puis travaillez sur le Rust. Ou gardez le TypeScript comme source et traduisez à chaque build (voir la [ligne de commande](cli.md#dans-un-build)).

## Quel TypeScript lit-il ?

Le TypeScript et le JavaScript modernes : classes, interfaces, génériques, unions, enums, `async`/`await`, générateurs, déstructuration, chaînage optionnel, modules (ES et CommonJS). Il ne lance pas le vérificateur de TypeScript lui-même : il infère ses propres types, plus stricts sur `null`.

## Pourquoi un nombre est-il un i64 et pas un f64 ?

Parce que la plupart des nombres d'un programme sont des compteurs et des indices. Un nombre reste un entier jusqu'à ce que le programme lui donne un flottant, le divise ou le lise depuis du JSON ; c'est alors un `f64`, comme tout nombre qui en reçoit la valeur. La sortie affiche les deux comme le ferait JavaScript.

## Est-ce que ça marche dans le navigateur ?

Le transpileur, oui : le bac à sable de la page d'accueil le fait tourner, compilé en WebAssembly, et le site marche hors ligne une fois installé comme app. Les programmes qu'il traduit sont des programmes en ligne de commande : ils n'ont pas de DOM.

## Mon code est-il envoyé quelque part ?

Non. `npx solidify-rs` tourne sur votre machine, et le bac à sable tourne dans votre navigateur.

## Puis-je l'utiliser dans mon projet, ou dans mon entreprise ?

Oui : vous pouvez utiliser Solidify librement, projets commerciaux compris, et le Rust qu'il écrit vous appartient. Solidify lui-même est un logiciel propriétaire : le modifier ou en redistribuer la source demande l'accord écrit de l'auteur. La [licence](../../LICENSE) en donne les conditions.

## Comment signaler un bug ?

Ouvrez une issue sur [GitHub](https://github.com/maxlestage/Solidify/issues) avec le programme, ce qu'affiche Node.js et ce qu'affiche le Rust (ou le message de Solidify). Les programmes qui ont échoué un jour deviennent des programmes de test.
