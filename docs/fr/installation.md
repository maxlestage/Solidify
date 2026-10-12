# Installation

Rien à installer d'autre que Node.js : `npx` lance Solidify, compilé en WebAssembly, sur vos fichiers. Vous pouvez aussi l'utiliser dans le navigateur, hors ligne, ou comme bibliothèque Rust.

## Avec npx

Il vous faut Node.js 20 ou plus récent. Ensuite, dans le dossier de votre programme :

```sh
npx solidify-rs main.ts -o app.rs
```

Le premier lancement télécharge le paquet `solidify-rs` (environ 2 Mo, le transpileur lui-même) ; les suivants démarrent tout de suite. Pour le garder plutôt dans un projet :

```sh
npm install --save-dev solidify-rs
npx solidify-rs main.ts -o app.rs
```

La page [Ligne de commande](cli.md) liste toutes les options.

## Compiler la sortie

La sortie est un seul fichier Rust pour l'édition 2024. Installez Rust avec [rustup](https://rustup.rs), puis :

```sh
rustc --edition 2024 -O app.rs -o app
./app
```

Elle n'a besoin d'aucun crate : les quelques fonctions utilitaires dont un programme se sert (afficher les nombres comme JavaScript, expressions régulières, dates…) sont écrites dans le fichier lui-même.

## Dans le navigateur

Le bac à sable de la page d'accueil fait tourner Solidify dans votre navigateur, compilé en WebAssembly : rien ne quitte votre machine. Le site est aussi une app : ouvrez-le dans Chrome, Edge ou Safari et choisissez **Installer l'app** (sur iPhone, *Partager → Sur l'écran d'accueil*). Il marche ensuite hors ligne, bac à sable compris.

## Comme bibliothèque Rust

Ajoutez-la à votre `Cargo.toml` :

```toml
[dependencies]
solidify = { git = "https://github.com/maxlestage/Solidify" }
```

Puis appelez-la depuis Rust :

```rust
let rust: String = solidify::transpile(typescript, &solidify::Options::default())?;
```

La page [Bibliothèque Rust](library.md) montre toute l'API.

## Depuis les sources

```sh
git clone https://github.com/maxlestage/Solidify
cd Solidify
cargo build --release -p solidify
./target/release/solidify main.ts -o app.rs
```

Le binaire `solidify` prend les mêmes options que `npx solidify-rs`. Solidify n'a aucune dépendance : pour le construire, il suffit de Rust.
