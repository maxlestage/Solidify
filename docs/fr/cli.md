# Ligne de commande

`npx solidify-rs` lance Solidify sur vos fichiers : le paquet npm contient le transpileur compilé en WebAssembly, exécuté par Node.js 20 ou plus récent. Construit depuis les sources, le binaire `solidify` prend les mêmes options.

## Utilisation

```sh
npx solidify-rs [input.ts] [-o output.rs] [--check] [--no-header]
```

Il traduit `input.ts` et chaque fichier qu'il importe en un seul fichier Rust. Sans fichier d'entrée, il lit l'entrée standard ; sans `-o`, il écrit sur la sortie standard.

```sh
npx solidify-rs main.ts -o app.rs     # follows the imports of main.ts, one Rust file out
npx solidify-rs < app.ts > app.rs     # one file through the standard input
npx solidify-rs --check main.ts       # only report errors
```

## Options

| Option | Ce qu'elle fait |
| --- | --- |
| `input.ts` | Le fichier d'entrée du programme, TypeScript ou JavaScript. `-` ou rien lit l'entrée standard, où les imports ne sont pas suivis. |
| `-o file`, `--output file` | Écrit le Rust dans `file` au lieu de la sortie standard. |
| `--check` | Traduit sans rien écrire : le code de sortie et les messages disent si le programme se traduit. |
| `--no-header` | Omet le commentaire qui nomme le fichier source en haut de la sortie. |
| `-h`, `--help` | Affiche l'utilisation. |
| `-V`, `--version` | Affiche la version. |

## Codes de sortie

| Code | Signification |
| --- | --- |
| `0` | Le programme a été traduit (ou vérifié, avec `--check`). |
| `1` | Le programme ne se traduit pas : le message, le fichier, la ligne et la colonne sont sur la sortie d'erreur standard. |
| `2` | La commande elle-même a échoué : une option inconnue, `-o` sans fichier, un fichier qui ne peut pas être lu ou écrit. |

## Erreurs

Les messages vont sur la sortie d'erreur standard, un par problème trouvé, avec la ligne et une marque sous l'endroit :

```text
error: `name` may be null here: test it with `if (… !== null)` first
  --> app.ts:2:10
   |
  2|   return name.charAt(0);
   |          ^^^^
```

[Erreurs](errors.md#les-messages-de-solidify) explique les plus courants.

## Dans un build

`--check` fait un test rapide pour l'intégration continue, et les codes de sortie permettent à un script de s'arrêter au premier programme qui ne se traduit pas :

```sh
for f in scripts/*.ts; do
  npx solidify-rs --check "$f" || exit 1
done
```

Pour garder un programme traduit à jour, traduisez-le puis compilez-le dans la même étape :

```sh
npx solidify-rs main.ts -o src/main.rs && cargo build --release
```

## Compiler la sortie

```sh
rustc --edition 2024 -O app.rs -o app
```

La sortie n'a besoin d'aucun crate. Mettez-la dans un projet Cargo en tant que `src/main.rs` pour utiliser les outils de Cargo (`cargo build`, `cargo clippy`) ; indiquez `edition = "2024"` dans son `Cargo.toml`.
